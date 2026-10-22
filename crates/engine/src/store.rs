//! As conexões guardadas: `~/.config/hy-dbplus/connections.toml` (ou `$XDG_CONFIG_HOME`), com
//! host, porta, usuário, banco, modo TLS, nome e tag de ambiente. A senha nunca entra aqui: fica
//! no chaveiro do sistema ([`crate::secrets`]), indexada pelo `id` da conexão.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::EngineError;
use hy_dbplus_driver::ConnectionConfig;

/// UUID v4 em texto. Renomear a conexão não troca o id, então a senha a acompanha.
pub type ConnectionId = String;

/// Tag de ambiente (tela 3.1 do BRIEF): colore a lista e a barra superior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tag {
    #[default]
    Local,
    Dev,
    Staging,
    Prod,
}

impl Tag {
    pub const ALL: [Tag; 4] = [Tag::Local, Tag::Dev, Tag::Staging, Tag::Prod];

    pub fn label(self) -> &'static str {
        match self {
            Tag::Local => "local",
            Tag::Dev => "dev",
            Tag::Staging => "staging",
            Tag::Prod => "prod",
        }
    }
}

/// Uma conexão da lista.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    pub id: ConnectionId,
    pub name: String,
    #[serde(default)]
    pub tag: Tag,
    #[serde(flatten)]
    pub config: ConnectionConfig,
}

impl Connection {
    /// Conexão nova, com id gerado.
    pub fn new(name: impl Into<String>, tag: Tag, config: ConnectionConfig) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            tag,
            config,
        }
    }

    /// Subtítulo da lista: `host : database`.
    pub fn subtitle(&self) -> String {
        format!("{} : {}", self.config.host, self.config.database)
    }
}

/// O arquivo inteiro. `connections` ausente = lista vazia.
#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    #[serde(default)]
    connections: Vec<Connection>,
}

/// A lista de conexões e o arquivo de onde veio.
#[derive(Debug)]
pub struct ConnectionStore {
    path: PathBuf,
    connections: Vec<Connection>,
}

impl ConnectionStore {
    /// `$XDG_CONFIG_HOME/hy-dbplus/connections.toml`, ou `~/.config/hy-dbplus/connections.toml`.
    pub fn default_path() -> Result<PathBuf, EngineError> {
        path_from(std::env::var_os("XDG_CONFIG_HOME"), std::env::home_dir())
    }

    /// Lê o arquivo; ausente = lista vazia (o arquivo nasce no primeiro `save`). Id repetido
    /// (arquivo editado à mão) é recusado: `get`, `upsert` e `remove` agem por id.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, EngineError> {
        let path = path.into();
        let connections = match std::fs::read_to_string(&path) {
            Ok(text) => {
                let file: File = toml::from_str(&text).map_err(|e| EngineError::Parse {
                    path: path.clone(),
                    reason: e.to_string(),
                })?;
                file.connections
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(EngineError::io(&path, &e)),
        };
        let mut seen = std::collections::HashSet::new();
        if let Some(dup) = connections.iter().find(|c| !seen.insert(c.id.as_str())) {
            return Err(EngineError::Parse {
                path,
                reason: format!("id duplicado: {}", dup.id),
            });
        }
        Ok(Self { path, connections })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn list(&self) -> &[Connection] {
        &self.connections
    }

    pub fn get(&self, id: &str) -> Option<&Connection> {
        self.connections.iter().find(|c| c.id == id)
    }

    /// Insere ou substitui pela mesma `id`, mantendo a posição na lista.
    pub fn upsert(&mut self, connection: Connection) {
        match self.connections.iter_mut().find(|c| c.id == connection.id) {
            Some(slot) => *slot = connection,
            None => self.connections.push(connection),
        }
    }

    /// Remove e devolve a conexão; quem chama também remove a senha do chaveiro.
    pub fn remove(&mut self, id: &str) -> Option<Connection> {
        let pos = self.connections.iter().position(|c| c.id == id)?;
        Some(self.connections.remove(pos))
    }

    /// Escreve atômico: arquivo temporário ao lado, depois `rename`. No Unix o diretório nasce
    /// 0700 e o arquivo 0600 (não tem senha, mas tem host e usuário). Um `.tmp` de uma falha
    /// anterior é apagado antes, e o desta falha, depois.
    pub fn save(&self) -> Result<(), EngineError> {
        let file = File {
            connections: self.connections.clone(),
        };
        let text = toml::to_string_pretty(&file).map_err(|e| EngineError::Config(e.to_string()))?;
        if let Some(dir) = self.path.parent() {
            create_private_dir(dir).map_err(|e| EngineError::io(dir, &e))?;
        }
        let tmp = self.path.with_extension("toml.tmp");
        remove_if_exists(&tmp).map_err(|e| EngineError::io(&tmp, &e))?;
        if let Err(e) = write_private(&tmp, text.as_bytes()) {
            let _ = std::fs::remove_file(&tmp);
            return Err(EngineError::io(&tmp, &e));
        }
        std::fs::rename(&tmp, &self.path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            EngineError::io(&self.path, &e)
        })
    }
}

/// O caminho do arquivo a partir do ambiente. `XDG_CONFIG_HOME` vazio conta como ausente.
fn path_from(
    xdg_config_home: Option<OsString>,
    home: Option<PathBuf>,
) -> Result<PathBuf, EngineError> {
    let base = match xdg_config_home.filter(|v| !v.is_empty()) {
        Some(xdg) => PathBuf::from(xdg),
        None => home
            .ok_or_else(|| EngineError::Config("sem diretório home".into()))?
            .join(".config"),
    };
    Ok(base.join("hy-dbplus").join("connections.toml"))
}

fn remove_if_exists(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(unix)]
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Diretório próprio por teste; apagado no fim.
    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("hy-dbplus-store-{}", uuid::Uuid::new_v4())))
        }
        fn file(&self) -> PathBuf {
            self.0.join("hy-dbplus").join("connections.toml")
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn sample() -> Connection {
        Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default())
    }

    #[test]
    fn arquivo_ausente_e_lista_vazia_e_o_save_cria_o_diretorio() {
        let root = TempRoot::new();
        let path = root.file();
        let mut store = ConnectionStore::load(&path).unwrap();
        assert!(store.list().is_empty());
        store.upsert(sample());
        store.save().unwrap();
        assert!(path.exists());
        assert!(
            !path.with_extension("toml.tmp").exists(),
            "temporário ficou para trás"
        );
        let again = ConnectionStore::load(&path).unwrap();
        assert_eq!(again.list(), store.list());
    }

    #[test]
    fn o_arquivo_nao_tem_senha_e_tem_o_que_a_lista_mostra() {
        let root = TempRoot::new();
        let path = root.file();
        let mut store = ConnectionStore::load(&path).unwrap();
        let mut c = sample();
        c.config.user = "ana".into();
        store.upsert(c.clone());
        store.save().unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("name = \"pg-localhost\""), "{text}");
        assert!(text.contains("tag = \"local\""), "{text}");
        assert!(text.contains("user = \"ana\""), "{text}");
        assert!(text.contains("tls = \"prefer\""), "{text}");
        assert!(
            !text.to_lowercase().contains("password") && !text.to_lowercase().contains("senha"),
            "{text}"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                std::fs::metadata(path.parent().unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
    }

    #[test]
    fn um_tmp_de_falha_anterior_nao_atrapalha_o_save() {
        let root = TempRoot::new();
        let path = root.file();
        let mut store = ConnectionStore::load(&path).unwrap();
        store.upsert(sample());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path.with_extension("toml.tmp"), "lixo").unwrap();
        store.save().unwrap();
        assert!(!path.with_extension("toml.tmp").exists());
        assert_eq!(ConnectionStore::load(&path).unwrap().list().len(), 1);
    }

    #[test]
    fn upsert_mantem_a_posicao_e_remove_devolve_a_conexao() {
        let root = TempRoot::new();
        let mut store = ConnectionStore::load(root.file()).unwrap();
        let a = sample();
        let mut b = Connection::new("outra", Tag::Prod, ConnectionConfig::default());
        store.upsert(a.clone());
        store.upsert(b.clone());
        b.name = "renomeada".into();
        store.upsert(b.clone());
        assert_eq!(
            store
                .list()
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["pg-localhost", "renomeada"]
        );
        assert_eq!(store.get(&b.id).map(|c| c.tag), Some(Tag::Prod));
        assert_eq!(store.remove(&a.id), Some(a));
        assert_eq!(store.remove("nao-existe"), None);
        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn arquivo_invalido_ou_com_id_duplicado_e_erro_de_parse_com_o_caminho() {
        let root = TempRoot::new();
        let path = root.file();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "connections = 42\n").unwrap();
        let err = ConnectionStore::load(&path).unwrap_err();
        assert!(
            matches!(&err, EngineError::Parse { path: p, .. } if p == &path),
            "{err:?}"
        );
        let one = toml::to_string(&File {
            connections: vec![sample()],
        })
        .unwrap();
        let body = one.trim_start_matches("[[connections]]\n");
        std::fs::write(
            &path,
            format!("[[connections]]\n{body}\n[[connections]]\n{body}"),
        )
        .unwrap();
        let err = ConnectionStore::load(&path).unwrap_err();
        assert!(
            matches!(&err, EngineError::Parse { reason, .. } if reason.contains("id duplicado")),
            "{err:?}"
        );
    }

    #[test]
    fn caminho_padrao_usa_xdg_config_home_ou_home_config() {
        let home = Some(PathBuf::from("/home/ana"));
        assert_eq!(
            path_from(Some("/tmp/xdg".into()), home.clone()).unwrap(),
            PathBuf::from("/tmp/xdg/hy-dbplus/connections.toml")
        );
        assert_eq!(
            path_from(Some("".into()), home.clone()).unwrap(),
            PathBuf::from("/home/ana/.config/hy-dbplus/connections.toml")
        );
        assert_eq!(
            path_from(None, home).unwrap(),
            PathBuf::from("/home/ana/.config/hy-dbplus/connections.toml")
        );
        assert!(matches!(path_from(None, None), Err(EngineError::Config(_))));
    }
}
