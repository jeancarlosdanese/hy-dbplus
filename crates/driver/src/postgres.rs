//! PostgreSQL por `tokio-postgres` sobre `rustls` (`ring`, sem OpenSSL nem libpq).
//!
//! TLS como no libpq: `prefer` e `require` cifram sem verificar o certificado; `verify-full`
//! verifica com as raízes do sistema (`rustls-native-certs`). Só o verificador permissivo é
//! código nosso; o resto é configuração.

use std::sync::Arc;
use std::time::Duration;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use tokio::sync::RwLock;
use tokio_postgres::error::SqlState;
use tokio_postgres::tls::{MakeTlsConnect, TlsConnect};
use tokio_postgres::{Client, NoTls};
use tokio_postgres_rustls::MakeRustlsConnect;

use tokio_postgres::SimpleQueryMessage;

use crate::{
    BatchStatement, Connection, ConnectionConfig, DatabaseInfo, Driver, DriverError, Outcome,
    Secret, TextRows, TlsMode,
};

/// Tempo máximo padrão para abrir a conexão: DNS, TCP, TLS e autenticação, tudo junto (o
/// `connect_timeout` do tokio-postgres cobre só o TCP, por isso o `tokio::time::timeout` em volta).
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// O driver PostgreSQL. Um por processo basta.
#[derive(Debug, Clone, Copy)]
pub struct PostgresDriver {
    pub connect_timeout: Duration,
}

impl Default for PostgresDriver {
    fn default() -> Self {
        Self {
            connect_timeout: CONNECT_TIMEOUT,
        }
    }
}

/// Uma conexão aberta. A tarefa de rede do `tokio-postgres` roda em `tokio::spawn` e termina
/// quando o `Client` é descartado. O erro dela se perde (a próxima query devolve `Closed`); o M5,
/// com stream de query, precisa guardá-lo para a UI dizer por que a conexão caiu.
pub struct PostgresConnection {
    /// Leituras compartilham o `Client` (o tokio-postgres enfileira as queries na mesma conexão);
    /// um lote pega a escrita e roda a transação sozinho, senão a query de outra tarefa entraria
    /// no `BEGIN … COMMIT` dele.
    client: RwLock<Client>,
    /// Para cancelar o statement em curso sem esperar o lock (a query que roda segura a leitura).
    cancel: tokio_postgres::CancelToken,
    /// O TLS da conexão, para a conexão de cancelamento usar o mesmo.
    tls: Option<ClientConfig>,
}

impl std::fmt::Debug for PostgresConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PostgresConnection")
    }
}

/// A configuração do `tokio-postgres` a partir da nossa. Interna: `get_password()` dela
/// devolve a senha, e ela só sai daqui para o servidor.
pub(crate) fn pg_config(
    config: &ConnectionConfig,
    password: &Secret,
    timeout: Duration,
) -> tokio_postgres::Config {
    let mut pg = tokio_postgres::Config::new();
    // Banco vazio: o nome do usuário, como o libpq.
    let database = if config.database.trim().is_empty() {
        config.user.trim()
    } else {
        config.database.trim()
    };
    pg.host(config.host.trim())
        .port(config.port)
        .user(config.user.trim())
        .dbname(database)
        .application_name("hy-dbplus")
        .connect_timeout(timeout)
        .ssl_mode(match config.tls {
            TlsMode::Disable => tokio_postgres::config::SslMode::Disable,
            TlsMode::Prefer => tokio_postgres::config::SslMode::Prefer,
            TlsMode::Require | TlsMode::VerifyFull => tokio_postgres::config::SslMode::Require,
        });
    if !password.is_empty() {
        pg.password(password.expose());
    }
    pg
}

/// Aceita qualquer certificado do servidor (semântica de `prefer`/`require` do libpq): a
/// conexão fica cifrada, mas sem prova de identidade do servidor. As assinaturas do handshake
/// continuam verificadas pelo provedor.
#[derive(Debug)]
struct AcceptAnyServerCert {
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for AcceptAnyServerCert {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// A configuração TLS para um modo (`None` para `disable`).
fn tls_config(mode: TlsMode) -> Result<Option<ClientConfig>, DriverError> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = ClientConfig::builder_with_provider(Arc::clone(&provider))
        .with_safe_default_protocol_versions()
        .map_err(|e| DriverError::Tls(e.to_string()))?;
    let config = match mode {
        TlsMode::Disable => return Ok(None),
        TlsMode::Prefer | TlsMode::Require => builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyServerCert { provider }))
            .with_no_client_auth(),
        TlsMode::VerifyFull => {
            let mut roots = rustls::RootCertStore::empty();
            let native = rustls_native_certs::load_native_certs();
            if native.certs.is_empty() {
                let why = native
                    .errors
                    .first()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "nenhum certificado".into());
                return Err(DriverError::Tls(format!(
                    "raízes de confiança do sistema indisponíveis: {why}"
                )));
            }
            let (_added, _ignored) = roots.add_parsable_certificates(native.certs);
            builder.with_root_certificates(roots).with_no_client_auth()
        }
    };
    Ok(Some(config))
}

/// Traduz o erro do `tokio-postgres` para o nosso, sem a senha e sem o `Debug` interno. O
/// `Display` do erro é só a categoria; a causa (`source`) é io, rustls, o `DbError` do servidor
/// ou o texto da autenticação.
fn map_error(config: &ConnectionConfig, e: &tokio_postgres::Error) -> DriverError {
    if let Some(db) = e.as_db_error() {
        let code = db.code();
        if *code == SqlState::INVALID_PASSWORD
            || *code == SqlState::INVALID_AUTHORIZATION_SPECIFICATION
        {
            return DriverError::Auth {
                user: config.user.clone(),
                reason: db.message().to_owned(),
            };
        }
        return DriverError::Db(db.message().to_owned());
    }
    if e.is_closed() {
        return DriverError::Closed;
    }
    let kind = e.to_string();
    let source = std::error::Error::source(e);
    let detail = source.map(ToString::to_string).unwrap_or_default();
    let reason = if detail.is_empty() {
        kind.clone()
    } else {
        format!("{kind}: {detail}")
    };
    if let Some(io) = source.and_then(|s| s.downcast_ref::<std::io::Error>()) {
        return match io.kind() {
            std::io::ErrorKind::TimedOut => DriverError::Timeout {
                host: config.host.clone(),
                port: config.port,
            },
            std::io::ErrorKind::InvalidData
                if io
                    .get_ref()
                    .is_some_and(|inner| inner.is::<rustls::Error>()) =>
            {
                DriverError::Tls(detail)
            }
            _ => DriverError::Connect {
                host: config.host.clone(),
                port: config.port,
                reason,
            },
        };
    }
    // Categorias do tokio-postgres sem causa io: "error performing TLS handshake" (servidor sem
    // TLS em `require`), "invalid configuration" (senha pedida pelo servidor e não informada),
    // "timeout waiting for server".
    match kind.as_str() {
        "error performing TLS handshake" => DriverError::Tls(reason),
        "invalid configuration" => DriverError::Auth {
            user: config.user.clone(),
            reason: detail,
        },
        "timeout waiting for server" => DriverError::Timeout {
            host: config.host.clone(),
            port: config.port,
        },
        _ => DriverError::Connect {
            host: config.host.clone(),
            port: config.port,
            reason,
        },
    }
}

/// Abre o socket e autentica, com o timeout em volta de tudo, e deixa a tarefa de rede rodando.
async fn open<T>(
    config: &ConnectionConfig,
    pg: &tokio_postgres::Config,
    tls: T,
    timeout: Duration,
) -> Result<Client, DriverError>
where
    T: MakeTlsConnect<tokio_postgres::Socket> + Send + 'static,
    T::Stream: Send + 'static,
    T::TlsConnect: Send,
    <T::TlsConnect as TlsConnect<tokio_postgres::Socket>>::Future: Send,
{
    let connected = tokio::time::timeout(timeout, pg.connect(tls))
        .await
        .map_err(|_| DriverError::Timeout {
            host: config.host.clone(),
            port: config.port,
        })?;
    let (client, connection) = connected.map_err(|e| map_error(config, &e))?;
    tokio::spawn(async move {
        // Termina quando o Client cai; um erro aqui aparece na próxima query como `Closed`.
        let _ = connection.await;
    });
    Ok(client)
}

impl Driver for PostgresDriver {
    type Conn = PostgresConnection;

    async fn connect(
        &self,
        config: &ConnectionConfig,
        password: &Secret,
    ) -> Result<PostgresConnection, DriverError> {
        if config.host.trim().is_empty() {
            return Err(DriverError::Config("host vazio".into()));
        }
        if config.user.trim().is_empty() {
            return Err(DriverError::Config("usuário vazio".into()));
        }
        // Sem runtime o tokio entraria em pânico no socket e no spawn; aqui vira erro.
        tokio::runtime::Handle::try_current()
            .map_err(|_| DriverError::Config("connect exige um runtime tokio".into()))?;
        let pg = pg_config(config, password, self.connect_timeout);
        let tls = tls_config(config.tls)?;
        let client = match tls.clone() {
            None => open(config, &pg, NoTls, self.connect_timeout).await?,
            Some(tls) => {
                open(
                    config,
                    &pg,
                    MakeRustlsConnect::new(tls),
                    self.connect_timeout,
                )
                .await?
            }
        };
        Ok(PostgresConnection {
            cancel: client.cancel_token(),
            client: RwLock::new(client),
            tls,
        })
    }
}

impl Connection for PostgresConnection {
    async fn ping(&self) -> Result<(), DriverError> {
        self.client
            .read()
            .await
            .simple_query("SELECT 1")
            .await
            .map(|_| ())
            .map_err(|e| db_error(&e))
    }

    async fn server_version(&self) -> Result<String, DriverError> {
        let row = self
            .client
            .read()
            .await
            .query_one("SHOW server_version", &[])
            .await
            .map_err(|e| db_error(&e))?;
        row.try_get::<_, String>(0).map_err(|e| db_error(&e))
    }

    async fn query_text(&self, sql: &str) -> Result<TextRows, DriverError> {
        let messages = self
            .client
            .read()
            .await
            .simple_query(sql)
            .await
            .map_err(|e| db_error(&e))?;
        text_rows(messages)
    }

    async fn list_databases(&self) -> Result<Vec<DatabaseInfo>, DriverError> {
        let rows = self.client.read().await
            .query("SELECT datname FROM pg_database WHERE NOT datistemplate AND datallowconn ORDER BY datname", &[])
            .await
            .map_err(|e| db_error(&e))?;
        rows.iter()
            .map(|row| {
                row.try_get::<_, String>(0)
                    .map(|name| DatabaseInfo { name })
                    .map_err(|e| db_error(&e))
            })
            .collect()
    }

    async fn execute_batch(&self, statements: &[BatchStatement]) -> Result<Vec<u64>, DriverError> {
        let mut client = self.client.write().await;
        let tx = client.transaction().await.map_err(|e| db_error(&e))?;
        let mut counts = Vec::with_capacity(statements.len());
        for (i, statement) in statements.iter().enumerate() {
            let position = i + 1;
            let messages = match tx.simple_query(&statement.sql).await {
                Ok(messages) => messages,
                Err(e) => {
                    // O erro do statement é o que interessa; o do ROLLBACK (conexão caída) não
                    // muda nada: a transação morre com ela.
                    let _ = tx.rollback().await;
                    return Err(match db_error(&e) {
                        // Conexão caída não é "o statement falhou": sobe como está.
                        DriverError::Closed => DriverError::Closed,
                        DriverError::Db(reason) => DriverError::Statement { position, reason },
                        other => DriverError::Statement {
                            position,
                            reason: other.to_string(),
                        },
                    });
                }
            };
            let got = affected_rows(&messages);
            if let Some(expected) = statement.expected_rows
                && got != expected
            {
                let _ = tx.rollback().await;
                return Err(DriverError::RowCount {
                    position,
                    expected,
                    got,
                });
            }
            counts.push(got);
        }
        tx.commit().await.map_err(|e| db_error(&e))?;
        Ok(counts)
    }

    async fn execute(&self, sql: &str, limit: Option<usize>) -> Result<Outcome, DriverError> {
        use futures_util::StreamExt;

        let client = self.client.read().await;
        let mut stream = std::pin::pin!(
            client
                .simple_query_raw(sql)
                .await
                .map_err(|e| db_error(&e))?
        );
        let mut outcome = Outcome::default();
        let mut columns: Option<Vec<String>> = None;
        let mut rows = Vec::new();
        while let Some(message) = stream.next().await {
            match message.map_err(|e| db_error(&e))? {
                SimpleQueryMessage::RowDescription(description) => {
                    columns = Some(description.iter().map(|c| c.name().to_owned()).collect());
                }
                SimpleQueryMessage::Row(row) => {
                    if columns.is_none() {
                        columns = Some(row.columns().iter().map(|c| c.name().to_owned()).collect());
                    }
                    // Passou do limite: a linha é descartada, mas o statement vai até o fim. Nada
                    // de cancelar no servidor: dentro de uma transação aberta o ERROR do cancel a
                    // aborta e o COMMIT seguinte viraria ROLLBACK em silêncio; e sem TLS o pedido
                    // de cancel fica em voo e derruba o statement seguinte desta conexão.
                    if limit.is_some_and(|n| rows.len() >= n) {
                        outcome.truncated = true;
                        continue;
                    }
                    rows.push(
                        (0..row.len())
                            .map(|i| row.get(i).map(str::to_owned))
                            .collect(),
                    );
                }
                SimpleQueryMessage::CommandComplete(n) if columns.is_none() => {
                    outcome.affected = Some(n);
                }
                _ => {}
            }
        }
        if let Some(columns) = columns {
            outcome.rows = Some(TextRows { columns, rows });
        }
        Ok(outcome)
    }

    async fn cancel(&self) -> Result<(), DriverError> {
        match self.tls.clone() {
            None => self.cancel.cancel_query(NoTls).await,
            Some(tls) => self.cancel.cancel_query(MakeRustlsConnect::new(tls)).await,
        }
        .map_err(|e| db_error(&e))
    }
}

/// As linhas afetadas pelo último `CommandComplete` (um statement por texto; com vários, vale o
/// último). Sem `CommandComplete` (não acontece com o protocolo simples), zero.
fn affected_rows(messages: &[SimpleQueryMessage]) -> u64 {
    messages
        .iter()
        .rev()
        .find_map(|m| match m {
            SimpleQueryMessage::CommandComplete(n) => Some(*n),
            _ => None,
        })
        .unwrap_or(0)
}

/// O primeiro conjunto de linhas das mensagens do protocolo simples: do primeiro
/// `RowDescription` até o `CommandComplete` dele, mesmo sem nenhuma `Row` no meio (um `SELECT`
/// que não devolve linha ainda é o primeiro conjunto). Statements sem resultado antes dele
/// (`SET`, `CREATE`) são pulados; os depois já rodaram no servidor, só não são relatados.
fn text_rows(messages: Vec<SimpleQueryMessage>) -> Result<TextRows, DriverError> {
    let mut out = TextRows::default();
    let mut seen_description = false;
    for message in messages {
        match message {
            SimpleQueryMessage::RowDescription(columns) if !seen_description => {
                seen_description = true;
                out.columns = columns.iter().map(|c| c.name().to_owned()).collect();
            }
            SimpleQueryMessage::Row(row) => {
                if !seen_description {
                    seen_description = true;
                    out.columns = row.columns().iter().map(|c| c.name().to_owned()).collect();
                }
                let mut cells = Vec::with_capacity(out.columns.len());
                for i in 0..out.columns.len() {
                    cells.push(row.try_get(i).map_err(|e| db_error(&e))?.map(str::to_owned));
                }
                out.rows.push(cells);
            }
            SimpleQueryMessage::CommandComplete(_) if seen_description => break,
            _ => {}
        }
    }
    Ok(out)
}

/// Erro de uma query já conectada: sem host nem usuário para citar.
fn db_error(e: &tokio_postgres::Error) -> DriverError {
    if let Some(db) = e.as_db_error() {
        if *db.code() == SqlState::QUERY_CANCELED {
            return DriverError::Cancelled(db.message().to_owned());
        }
        return DriverError::Db(db.message().to_owned());
    }
    if e.is_closed() {
        return DriverError::Closed;
    }
    DriverError::Db(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_configuracao_do_tokio_postgres_segue_a_nossa() {
        let cfg = ConnectionConfig {
            host: "db.local".into(),
            port: 5433,
            user: "ana".into(),
            database: "loja".into(),
            tls: TlsMode::VerifyFull,
        };
        let pg = pg_config(&cfg, &Secret::new("s3gr3d0"), CONNECT_TIMEOUT);
        assert_eq!(
            pg.get_hosts(),
            &[tokio_postgres::config::Host::Tcp("db.local".into())]
        );
        assert_eq!(pg.get_ports(), &[5433]);
        assert_eq!(pg.get_user(), Some("ana"));
        assert_eq!(pg.get_dbname(), Some("loja"));
        assert_eq!(pg.get_password(), Some(&b"s3gr3d0"[..]));
        assert_eq!(pg.get_ssl_mode(), tokio_postgres::config::SslMode::Require);
        assert_eq!(pg.get_connect_timeout(), Some(&CONNECT_TIMEOUT));
        assert_eq!(pg.get_application_name(), Some("hy-dbplus"));
        // Sem senha não manda senha (autenticação trust/peer); banco vazio vira o usuário.
        let no_db = ConnectionConfig {
            database: " ".into(),
            tls: TlsMode::Disable,
            ..cfg
        };
        let pg = pg_config(&no_db, &Secret::default(), CONNECT_TIMEOUT);
        assert_eq!(pg.get_password(), None);
        assert_eq!(pg.get_dbname(), Some("ana"));
        assert_eq!(pg.get_ssl_mode(), tokio_postgres::config::SslMode::Disable);
    }

    #[test]
    fn o_tls_monta_para_todo_modo_menos_disable() {
        assert!(tls_config(TlsMode::Disable).unwrap().is_none());
        assert!(tls_config(TlsMode::Prefer).unwrap().is_some());
        assert!(tls_config(TlsMode::Require).unwrap().is_some());
        // verify-full depende das raízes do sistema: na máquina de desenvolvimento e no CI existem.
        assert!(tls_config(TlsMode::VerifyFull).unwrap().is_some());
    }

    #[test]
    fn sem_runtime_e_erro_e_nao_panico() {
        // Nenhum runtime tokio neste teste, de propósito: antes, o socket ou o spawn entrariam em
        // pânico. O future resolve no primeiro poll, então um waker vazio basta.
        let driver = PostgresDriver::default();
        let config = ConnectionConfig::default();
        let secret = Secret::default();
        let mut fut = Box::pin(driver.connect(&config, &secret));
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        let std::task::Poll::Ready(result) = fut.as_mut().poll(&mut cx) else {
            panic!("o connect sem runtime deveria resolver no primeiro poll");
        };
        assert!(
            matches!(result, Err(DriverError::Config(ref m)) if m.contains("runtime")),
            "{result:?}"
        );
    }

    #[tokio::test]
    async fn host_e_usuario_vazios_sao_erro_de_configuracao() {
        let d = PostgresDriver::default();
        let blank_host = ConnectionConfig {
            host: " ".into(),
            ..ConnectionConfig::default()
        };
        assert!(matches!(
            d.connect(&blank_host, &Secret::default()).await,
            Err(DriverError::Config(_))
        ));
        let no_user = ConnectionConfig {
            user: String::new(),
            ..ConnectionConfig::default()
        };
        assert!(matches!(
            d.connect(&no_user, &Secret::default()).await,
            Err(DriverError::Config(_))
        ));
    }

    #[tokio::test]
    async fn porta_fechada_e_erro_de_conexao_sem_a_senha_no_texto() {
        let cfg = ConnectionConfig {
            host: "127.0.0.1".into(),
            port: 1,
            tls: TlsMode::Disable,
            ..ConnectionConfig::default()
        };
        let err = PostgresDriver::default()
            .connect(&cfg, &Secret::new("naovaza"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, DriverError::Connect { port: 1, .. }),
            "{err:?}"
        );
        assert!(!err.to_string().contains("naovaza"));
    }

    #[tokio::test]
    async fn servidor_que_aceita_o_tcp_e_nao_responde_da_timeout() {
        // Antes: o connect_timeout do tokio-postgres cobria só o TCP, e isto pendia para sempre.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let cfg = ConnectionConfig {
            host: "127.0.0.1".into(),
            port,
            tls: TlsMode::Disable,
            ..ConnectionConfig::default()
        };
        let driver = PostgresDriver {
            connect_timeout: Duration::from_millis(300),
        };
        let started = std::time::Instant::now();
        let err = driver.connect(&cfg, &Secret::default()).await.unwrap_err();
        assert!(
            matches!(err, DriverError::Timeout { port: p, .. } if p == port),
            "{err:?}"
        );
        assert!(started.elapsed() < Duration::from_secs(3));
        drop(listener);
    }

    /// Testes contra o PostgreSQL local: `HY_DBPLUS_TEST_PG=postgres://postgres@localhost/postgres`
    /// (URL do libpq; pode levar senha, que fica só no ambiente). Sem a variável, pulados.
    mod with_server {
        use super::*;

        // O teste do catálogo repete este parse: a URL de teste é uma só; mudar lá e aqui.
        fn config_from_env() -> Option<(ConnectionConfig, Secret)> {
            let url = std::env::var_os("HY_DBPLUS_TEST_PG")?;
            let pg: tokio_postgres::Config = url
                .to_string_lossy()
                .parse()
                .expect("HY_DBPLUS_TEST_PG inválida");
            let host = match pg.get_hosts().first() {
                Some(tokio_postgres::config::Host::Tcp(h)) => h.clone(),
                _ => "localhost".into(),
            };
            let cfg = ConnectionConfig {
                host,
                port: pg.get_ports().first().copied().unwrap_or(5432),
                user: pg.get_user().unwrap_or("postgres").into(),
                database: pg.get_dbname().unwrap_or("postgres").into(),
                tls: TlsMode::Prefer,
            };
            let secret = Secret::new(String::from_utf8_lossy(
                pg.get_password().unwrap_or_default(),
            ));
            Some((cfg, secret))
        }

        #[tokio::test]
        async fn conecta_le_a_versao_e_lista_os_bancos() {
            let Some((cfg, secret)) = config_from_env() else {
                return;
            };
            let conn = PostgresDriver::default()
                .connect(&cfg, &secret)
                .await
                .unwrap();
            conn.ping().await.unwrap();
            let version = conn.server_version().await.unwrap();
            assert!(
                version.starts_with(|c: char| c.is_ascii_digit()),
                "{version}"
            );
            let dbs = conn.list_databases().await.unwrap();
            assert!(dbs.iter().any(|d| d.name == cfg.database), "{dbs:?}");
            assert!(
                dbs.iter().all(|d| !d.name.starts_with("template")),
                "{dbs:?}"
            );
        }

        /// `execute`: linhas com limite (descarta o excesso e marca truncado), contagem de
        /// linhas afetadas, erro de sintaxe sem derrubar a conexão; `cancel` interrompe um
        /// `pg_sleep` em menos de um segundo com `Cancelled`.
        #[tokio::test]
        async fn execute_com_limite_contagem_erro_e_cancel() {
            let Some((cfg, secret)) = config_from_env() else {
                return;
            };
            let conn = std::sync::Arc::new(
                PostgresDriver::default()
                    .connect(&cfg, &secret)
                    .await
                    .unwrap(),
            );
            let out = conn
                .execute("SELECT g AS n FROM generate_series(1, 100000) g", Some(5))
                .await
                .unwrap();
            let rows = out.rows.unwrap();
            assert_eq!(rows.columns, ["n"]);
            assert_eq!(rows.rows.len(), 5);
            assert_eq!(rows.cell(4, 0), Some("5"));
            assert!(out.truncated && out.affected.is_none());
            // Sem limite: tudo, sem truncar; resultado vazio ainda é `rows`.
            let out = conn.execute("SELECT 1 WHERE false", None).await.unwrap();
            assert_eq!(out.rows.unwrap().rows.len(), 0);
            assert!(!out.truncated);
            let out = conn
                .execute("CREATE TEMP TABLE t_exec (x int); INSERT INTO t_exec SELECT generate_series(1, 7)", None)
                .await
                .unwrap();
            assert_eq!((out.rows, out.affected), (None, Some(7)));
            let err = conn.execute("SELEC 1", None).await.unwrap_err();
            assert!(matches!(err, DriverError::Db(m) if m.contains("syntax error")));
            // A conexão segue servindo depois do erro.
            assert_eq!(
                conn.execute("SELECT count(*) FROM t_exec", None)
                    .await
                    .unwrap()
                    .rows
                    .unwrap()
                    .cell(0, 0),
                Some("7")
            );
            // Resultado maior que o limite, numa conexão sem TLS: o statement seguinte tem de
            // passar. Sem TLS um pedido de cancel só é escrito no socket e fica em voo; um cancel
            // por limite que chegasse depois do fim do statement derrubaria o próximo (revisão de
            // 22/10/2026). Com TLS o filho do servidor já leu o pedido quando `cancel` volta, e o
            // cancel chega com o backend ocioso, onde é descartado: por isso a conexão própria.
            let plain = PostgresDriver::default()
                .connect(
                    &ConnectionConfig {
                        tls: TlsMode::Disable,
                        ..cfg.clone()
                    },
                    &secret,
                )
                .await
                .unwrap();
            for total in [150, 1_200, 150, 1_200] {
                let out = plain
                    .execute(
                        &format!("SELECT g FROM generate_series(1, {total}) g"),
                        Some(100),
                    )
                    .await
                    .unwrap();
                assert!(out.truncated);
                assert_eq!(out.rows.unwrap().rows.len(), 100);
                let next = plain
                    .execute("SELECT count(*) FROM generate_series(1, 20000)", None)
                    .await;
                assert!(next.is_ok(), "depois de {total} linhas: {next:?}");
            }
            // Limite dentro de uma transação aberta: nada pode cancelar no servidor (um ERROR,
            // inclusive o 57014 do cancel, aborta a transação e o COMMIT viraria ROLLBACK em
            // silêncio; revisão de 22/10/2026). A tabela criada antes do SELECT tem de existir depois.
            plain.execute("BEGIN", None).await.unwrap();
            plain
                .execute("CREATE TEMP TABLE t_tx (x int)", None)
                .await
                .unwrap();
            plain
                .execute("INSERT INTO t_tx VALUES (1)", None)
                .await
                .unwrap();
            let out = plain
                .execute("SELECT g FROM generate_series(1, 200000) g", Some(100))
                .await
                .unwrap();
            assert!(out.truncated);
            plain.execute("COMMIT", None).await.unwrap();
            let count = plain
                .execute("SELECT count(*) FROM t_tx", None)
                .await
                .unwrap_or_else(|e| panic!("a transação foi abortada pelo limite: {e}"));
            assert_eq!(count.rows.unwrap().cell(0, 0), Some("1"));
            // Cancel: a query longa volta com Cancelled em menos de um segundo.
            let started = std::time::Instant::now();
            let sleeping = {
                let conn = std::sync::Arc::clone(&conn);
                tokio::spawn(async move { conn.execute("SELECT pg_sleep(5)", None).await })
            };
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            conn.cancel().await.unwrap();
            let result = sleeping.await.unwrap();
            assert!(
                matches!(result.unwrap_err(), DriverError::Cancelled(m) if m.contains("user request"))
            );
            assert!(started.elapsed() < std::time::Duration::from_secs(2));
            conn.ping().await.unwrap();
        }

        /// O lote é tudo ou nada: erro no 2º statement ou contagem diferente da esperada desfaz
        /// o 1º; depois do erro a conexão segue servindo, e o lote bom grava e devolve as contagens.
        #[tokio::test]
        async fn lote_desfaz_tudo_no_erro_ou_na_contagem_errada_e_grava_quando_bate() {
            let Some((cfg, secret)) = config_from_env() else {
                return;
            };
            let conn = PostgresDriver::default()
                .connect(&cfg, &secret)
                .await
                .unwrap();
            let schema = format!(
                "hy_batch_{}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            );
            conn.query_text(&format!(
                "CREATE SCHEMA {schema}; \
                 CREATE TABLE {schema}.t (id integer PRIMARY KEY, qtd integer NOT NULL); \
                 INSERT INTO {schema}.t VALUES (1, 1), (2, 2), (3, 3)"
            ))
            .await
            .unwrap();
            async fn state(conn: &PostgresConnection, schema: &str) -> Vec<String> {
                let rows = conn
                    .query_text(&format!("SELECT id, qtd FROM {schema}.t ORDER BY id"))
                    .await
                    .unwrap();
                rows.rows
                    .iter()
                    .map(|r| {
                        r.iter()
                            .map(|c| c.clone().unwrap_or_default())
                            .collect::<Vec<_>>()
                            .join("=")
                    })
                    .collect()
            }
            let st = |sql: String, expected: Option<u64>| BatchStatement {
                sql,
                expected_rows: expected,
            };
            let update_1 = || {
                st(
                    format!("UPDATE {schema}.t SET qtd = 10 WHERE id = 1"),
                    Some(1),
                )
            };
            let untouched = vec!["1=1".to_owned(), "2=2".to_owned(), "3=3".to_owned()];

            // Texto numa coluna integer: erro do servidor no 2º statement; o 1º é desfeito.
            let err = conn
                .execute_batch(&[
                    update_1(),
                    st(
                        format!("UPDATE {schema}.t SET qtd = 'x' WHERE id = 2"),
                        Some(1),
                    ),
                ])
                .await
                .unwrap_err();
            assert!(
                matches!(&err, DriverError::Statement { position: 2, reason } if reason.contains("integer")),
                "{err}"
            );
            assert_eq!(state(&conn, &schema).await, untouched);

            // Contagem errada (esperava apagar 3, só há 2 com essas chaves): também desfaz.
            let err = conn
                .execute_batch(&[
                    update_1(),
                    st(
                        format!("DELETE FROM {schema}.t WHERE id IN (2, 3)"),
                        Some(3),
                    ),
                ])
                .await
                .unwrap_err();
            assert_eq!(
                err,
                DriverError::RowCount {
                    position: 2,
                    expected: 3,
                    got: 2
                }
            );
            assert_eq!(state(&conn, &schema).await, untouched);

            // A transação é só do lote: uma leitura na mesma conexão, no meio dele, espera o fim
            // (aqui o ROLLBACK do 3º statement) e vê o valor antigo. Sem o RwLock ela entraria no
            // BEGIN … e leria o 10 ainda não confirmado.
            let conn = std::sync::Arc::new(conn);
            let in_flight = {
                let conn = std::sync::Arc::clone(&conn);
                let batch = vec![
                    update_1(),
                    st("SELECT pg_sleep(0.3)".to_owned(), None),
                    st(
                        format!("UPDATE {schema}.t SET qtd = 'x' WHERE id = 2"),
                        Some(1),
                    ),
                ];
                tokio::spawn(async move { conn.execute_batch(&batch).await })
            };
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let seen = conn
                .query_text(&format!("SELECT qtd FROM {schema}.t WHERE id = 1"))
                .await
                .unwrap();
            assert_eq!(
                seen.cell(0, 0),
                Some("1"),
                "a leitura entrou na transação do lote"
            );
            assert!(matches!(
                in_flight.await.unwrap(),
                Err(DriverError::Statement { position: 3, .. })
            ));
            assert_eq!(state(&conn, &schema).await, untouched);

            // Lote bom: grava e devolve as contagens.
            let counts = conn
                .execute_batch(&[
                    update_1(),
                    st(
                        format!("DELETE FROM {schema}.t WHERE id IN (2, 3)"),
                        Some(2),
                    ),
                ])
                .await
                .unwrap();
            assert_eq!(counts, [1, 2]);
            assert_eq!(state(&conn, &schema).await, ["1=10"]);

            conn.query_text(&format!("DROP SCHEMA {schema} CASCADE"))
                .await
                .unwrap();
        }

        #[tokio::test]
        async fn query_text_devolve_colunas_e_celulas_como_texto() {
            let Some((cfg, secret)) = config_from_env() else {
                return;
            };
            let conn = PostgresDriver::default()
                .connect(&cfg, &secret)
                .await
                .unwrap();
            let rows = conn
                .query_text("select 1 as a, null as b, 'x' as c; select 2")
                .await
                .unwrap();
            assert_eq!(rows.columns, ["a", "b", "c"]);
            assert_eq!(
                rows.rows,
                vec![vec![Some("1".into()), None, Some("x".into())]]
            );
            assert_eq!(rows.cell(0, 0), Some("1"));
            assert_eq!(rows.cell(0, 1), None);
            assert_eq!(rows.cell(1, 0), None);
            let err = conn
                .query_text("select * from tabela_que_nao_existe")
                .await
                .unwrap_err();
            assert!(
                matches!(err, DriverError::Db(ref m) if m.contains("tabela_que_nao_existe")),
                "{err:?}"
            );
            // Statement sem linhas: colunas e linhas vazias, sem erro.
            let none = conn
                .query_text("set application_name = 'hy-dbplus-test'")
                .await
                .unwrap();
            assert!(none.columns.is_empty() && none.rows.is_empty());
            // Antes: SELECT sem linhas era pulado e o segundo statement aparecia no lugar dele.
            let empty = conn
                .query_text("select 1 as a where false; select 2 as b")
                .await
                .unwrap();
            assert_eq!(empty.columns, ["a"]);
            assert!(empty.rows.is_empty(), "{empty:?}");
            // SET antes do SELECT é pulado; o SELECT é o primeiro conjunto.
            let after_set = conn
                .query_text("set application_name = 'x'; select 3 as c")
                .await
                .unwrap();
            assert_eq!(after_set.columns, ["c"]);
            assert_eq!(after_set.cell(0, 0), Some("3"));
        }

        #[tokio::test]
        async fn usuario_inexistente_e_erro_de_autenticacao_sem_a_senha() {
            let Some((cfg, _)) = config_from_env() else {
                return;
            };
            let cfg = ConnectionConfig {
                user: "hy_dbplus_nao_existe".into(),
                ..cfg
            };
            let err = PostgresDriver::default()
                .connect(&cfg, &Secret::new("naovaza"))
                .await
                .unwrap_err();
            assert!(matches!(err, DriverError::Auth { .. }), "{err:?}");
            assert!(!err.to_string().contains("naovaza"));
        }
    }
}
