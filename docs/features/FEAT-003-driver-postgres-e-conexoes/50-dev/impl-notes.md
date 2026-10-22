# @file: docs/features/FEAT-003-driver-postgres-e-conexoes/50-dev/impl-notes.md

# Notas de Implementação — FEAT-003 driver-postgres-e-conexoes

## Estado atual

_Atualizado em 21/10/2026._

- **Feito:** documentação; `driver` PostgreSQL (config, erro, trait, implementação, 11 testes, 2 deles contra o
  servidor local); portão `--full` verde com 369 pacotes no lock.
- **Em curso:** nada.
- **Falta:** `engine`; `ui-egui` e app; verificação do Jean.
- **Depende de:** FEAT-002 concluída (M0); PostgreSQL local `pg-localhost` para os testes com variável.
- **Para retomar:** esta pasta, o `CLAUDE.md` da raiz e o BRIEF §3.1, §3.2, §5 e §6.

## Decisões

- **[2026-10-21]** TLS por `rustls` 0.23 com `default-features = false` e `ring`: o provedor padrão
  (`aws-lc-rs`) traz `aws-lc-sys` com licença OpenSSL/ISC fora do `deny.toml`. Raízes de confiança do sistema
  por `rustls-native-certs` (Keychain no macOS), não `webpki-roots` (CDLA-Permissive, fora da lista).
- **[2026-10-21]** `tokio-postgres-rustls` 0.14 (MIT) faz a ponte `tokio-postgres` ↔ `rustls`; feature
  `native-certs`, sem `webpki-roots`.
- **[2026-10-21]** `keyring` 4 com as features padrão (`v1`: Keychain no macOS, Credential Manager no Windows,
  Secret Service por zbus no Linux). Serviço `hy-dbplus`, conta = id da conexão: UUID v4 pela crate `uuid`
  (MIT OR Apache-2.0; `getrandom` já está no grafo via `ring`). Renomear a conexão não troca o id, então a
  senha segue a conexão.
- **[2026-10-21]** `tokio` sem features padrão: `rt-multi-thread`, `sync`, `time`, `net`. O runtime nasce no
  app e o `engine` recebe um `tokio::runtime::Handle`; a UI nunca chama `block_on`.
- **[2026-10-21]** Eventos do `engine` para a UI por `std::sync::mpsc` (a UI lê sem bloquear a cada frame e
  pede repaint por `egui::Context::request_repaint` passado ao engine como callback), não por canal `tokio`:
  a UI não é assíncrona.
- **[2026-10-21]** `TlsMode` com a semântica do libpq: `disable`; `prefer` e `require` cifram sem verificar
  o certificado (verificador permissivo próprio, que ainda confere as assinaturas do handshake pelo `ring`);
  `verify-full` verifica com as raízes do sistema e o nome do host. Sem o modo permissivo, todo servidor local
  com certificado autoassinado falharia. Padrão `prefer`, como o libpq.
- **[2026-10-21]** Trait `Driver`/`Connection` com `async fn` nativo (`impl Future + Send`), sem a crate
  `async-trait`: a `Session` do engine é genérica no driver, e o driver falso dos testes entra por aí.
- **[2026-10-21]** `Secret(String)` com `Debug` fixo `Secret(***)` e sem `Display`; limpeza de memória
  (zeroize) fica para depois, se virar requisito.
- **[2026-10-21]** Caminho da configuração: `$XDG_CONFIG_HOME/hy-dbplus/` ou `~/.config/hy-dbplus/` em todo
  sistema, como o BRIEF §6, via `std::env::var_os` e `std::env::home_dir` (sem crate `dirs`). Revisar no M8
  se o empacotamento `.app` pedir `~/Library/Application Support`.
- **[2026-10-21]** Dependências novas (versão mínima; licença): `tokio` 1 (MIT), `tokio-postgres` 0.7
  (MIT OR Apache-2.0), `tokio-postgres-rustls` 0.14 (MIT), `rustls` 0.23 (Apache-2.0 OR ISC OR MIT),
  `rustls-native-certs` 0.8 (Apache-2.0 OR ISC OR MIT), `keyring` 4 (MIT OR Apache-2.0), `thiserror` 2
  (MIT OR Apache-2.0), `serde` 1 (MIT OR Apache-2.0), `toml` 1 (MIT OR Apache-2.0), `uuid` 1 com `v4` (MIT OR Apache-2.0).

## Log

### 21/10/2026 — Criação

- [x] Leitura do BRIEF §3.1, §3.2, §4, §5 e §6; versões e licenças no crates.io (`cargo info`); projeto de
  teste no scratchpad com as dependências candidatas: `cargo fetch`, `cargo tree` (ring, sem aws-lc, sem
  openssl) e `cargo-deny check licenses bans sources` verde com o `deny.toml` do projeto.
- [x] Brief, aceite, notas e índice escritos antes do código (`d71c07b`).

### 21/10/2026 — `driver`

- [x] `config.rs`: `ConnectionConfig` (host, porta, usuário, banco, `tls`, com `Default` para localhost),
  `TlsMode` em kebab-case no TOML com `prefer` como padrão, `Secret` sem `Display` e com `Debug` `Secret(***)`.
- [x] `error.rs`: `DriverError` (`Config`, `Connect`, `Auth`, `Tls`, `Timeout`, `Closed`, `Db`); nenhuma variante
  carrega a senha.
- [x] `postgres.rs`: `pg_config` (testável sem rede: hosts, porta, usuário, banco, senha só se houver,
  `application_name`, timeout de 10 s, `ssl_mode`), `tls_config` por modo, `AcceptAnyServerCert` para
  `prefer`/`require`, `map_error` (28P01 e 28000 viram `Auth`; io vira `Connect`/`Timeout`), `Driver` e
  `Connection` para PostgreSQL (`SHOW server_version`; `pg_database` sem templates e sem `datallowconn = false`).
- [x] Testes: 11, sendo 2 atrás de `HY_DBPLUS_TEST_PG` (URL do libpq; `postgres://postgres@localhost/postgres`
  basta no Postgres.app, que autentica `trust`). Provado que a variável liga os testes: com porta 1 na URL os
  dois falham; sem a variável, saem cedo. Usuário inexistente vem como 28000 e vira `Auth`, sem a senha no texto.
- [x] Dependências do passo no `Cargo.toml`: `tokio`, `tokio-postgres`, `tokio-postgres-rustls`, `rustls`,
  `rustls-native-certs`, `thiserror`, `serde`, `toml` (dev no driver, para a ida e volta do TOML). Lock de 299
  para 369 pacotes; cargo-deny verde. `keyring` e `uuid` entram com o `engine`.
- [x] Revisor (um médio, vários baixos, todos aplicados antes do commit): o `connect_timeout` do tokio-postgres
  cobre só o TCP, então servidor que aceita a conexão e cala deixava o `connect` pendente para sempre; agora
  `tokio::time::timeout` envolve DNS, TCP, TLS e autenticação, o timeout é campo do `PostgresDriver` (10 s por
  padrão) e um teste com `TcpListener` mudo prova o `Timeout` em 300 ms. `map_error` decide pelo `kind()` do
  `io::Error` (e `rustls::Error` dentro de `InvalidData`) e pelas três categorias sem causa io do
  tokio-postgres, em vez de substring; "invalid configuration" (senha exigida e não informada) vira `Auth`.
  `pg_config` virou `pub(crate)` (o `Config` do tokio-postgres entrega a senha por `get_password`). Sem runtime
  o `connect` devolve `Config` em vez de o tokio entrar em pânico (teste com `Waker::noop`). Banco vazio cai
  para o nome do usuário, como o libpq. Um só `open` genérico para `NoTls` e rustls. Docs da trait alinhados
  (`server_version` é para exibir; `list_databases` são os bancos que aceitam conexão); BRIEF §5 descreve
  `Driver` e `Connection`. Identificadores de teste em inglês (`with_server`, `blank_host`, `no_user`).
  Pendência para o M3: a tarefa de rede engole o erro da conexão (`let _ = connection.await`); guardar o
  último erro para a UI dizer por que caiu.
