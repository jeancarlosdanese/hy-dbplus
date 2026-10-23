//! As queries das abas SQL (FEAT-011): um arquivo por aba em
//! `~/.config/hy-dbplus/queries/<id da conexão>/<banco>/<n>.sql` (ou `$XDG_CONFIG_HOME`), só com o
//! texto. A UI salva ao perder o foco e ao fechar aba, banco, conexão ou app, e lê ao abrir o
//! banco. O store não acrescenta nada ao texto (nem host, nem a senha da conexão); o que o usuário
//! digitar fica no arquivo como está, inclusive uma senha dentro de um `CREATE ROLE`, por isso o
//! arquivo nasce `0600`.

use std::path::{Path, PathBuf};

use crate::EngineError;
use crate::session::QueryTabId;
use crate::store::{config_dir_from, create_private_dir, remove_if_exists, write_private};

/// Uma aba SQL salva: o número da aba (`SQL Query n`, arquivo `n.sql`) e o texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedQuery {
    pub id: QueryTabId,
    pub sql: String,
}

/// O diretório das queries salvas.
#[derive(Debug, Clone)]
pub struct QueryStore {
    dir: PathBuf,
}

impl QueryStore {
    /// `$XDG_CONFIG_HOME/hy-dbplus/queries`, ou `~/.config/hy-dbplus/queries`.
    pub fn default_dir() -> Result<PathBuf, EngineError> {
        Ok(
            config_dir_from(std::env::var_os("XDG_CONFIG_HOME"), std::env::home_dir())?
                .join("queries"),
        )
    }

    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// As queries salvas do banco, em ordem de número. Diretório ausente = nenhuma. Arquivos que não
    /// são `<n>.sql` são ignorados; erro de leitura vira [`EngineError::Io`] com o caminho.
    pub fn list(&self, connection: &str, database: &str) -> Result<Vec<SavedQuery>, EngineError> {
        let dir = self.database_dir(connection, database);
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(EngineError::io(&dir, &e)),
        };
        let mut queries = Vec::new();
        for entry in entries {
            let path = entry.map_err(|e| EngineError::io(&dir, &e))?.path();
            let Some(id) = query_id(&path) else { continue };
            let sql = std::fs::read_to_string(&path).map_err(|e| EngineError::io(&path, &e))?;
            queries.push(SavedQuery { id, sql });
        }
        queries.sort_by_key(|q| q.id);
        Ok(queries)
    }

    /// Grava o texto da aba (atômico: `.tmp` ao lado e `rename`; diretórios `0700`, arquivo `0600`).
    /// Texto só de espaços apaga o arquivo: aba vazia não deixa rastro.
    pub fn save(
        &self,
        connection: &str,
        database: &str,
        id: QueryTabId,
        sql: &str,
    ) -> Result<(), EngineError> {
        if sql.trim().is_empty() {
            return self.remove(connection, database, id);
        }
        let dir = self.database_dir(connection, database);
        create_private_dir(&dir).map_err(|e| EngineError::io(&dir, &e))?;
        let path = dir.join(format!("{id}.sql"));
        let tmp = dir.join(format!("{id}.sql.tmp"));
        remove_if_exists(&tmp).map_err(|e| EngineError::io(&tmp, &e))?;
        if let Err(e) = write_private(&tmp, sql.as_bytes()) {
            let _ = std::fs::remove_file(&tmp);
            return Err(EngineError::io(&tmp, &e));
        }
        std::fs::rename(&tmp, &path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            EngineError::io(&path, &e)
        })
    }

    /// Apaga o arquivo da aba; ausente não é erro.
    pub fn remove(
        &self,
        connection: &str,
        database: &str,
        id: QueryTabId,
    ) -> Result<(), EngineError> {
        let path = self
            .database_dir(connection, database)
            .join(format!("{id}.sql"));
        remove_if_exists(&path).map_err(|e| EngineError::io(&path, &e))
    }

    /// Apaga tudo o que a conexão tinha salvo (quando ela sai da lista); ausente não é erro.
    pub fn remove_connection(&self, connection: &str) -> Result<(), EngineError> {
        let dir = self.dir.join(safe_segment(connection));
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(EngineError::io(&dir, &e)),
        }
    }

    fn database_dir(&self, connection: &str, database: &str) -> PathBuf {
        self.dir
            .join(safe_segment(connection))
            .join(safe_segment(database))
    }
}

/// O número da aba a partir de `<n>.sql`; qualquer outro nome é `None`.
fn query_id(path: &Path) -> Option<QueryTabId> {
    if path.extension()? != "sql" {
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    let id: QueryTabId = stem.parse().ok()?;
    // Só a forma canônica: `007.sql` ou `+7.sql` editado à mão não vira um segundo `7`.
    (id.to_string() == stem).then_some(id)
}

/// Um nome (de conexão ou de banco) como um único componente de caminho: letras, dígitos, `_` e
/// `-` passam; cada outro byte (UTF-8) vira `%XX`. Assim `..`, `/`, `\` e espaço não saem do
/// diretório, e dois nomes diferentes nunca viram o mesmo componente. O nome vazio vira `%`
/// (nenhum outro nome produz um `%` solto): `join("")` não acrescentaria componente nenhum, e um
/// `remove_connection("")` apagaria o diretório inteiro. Em sistema de arquivos que não distingue
/// maiúsculas, `Loja` e `loja` caem na mesma pasta (PostgreSQL os distingue): limitação anotada.
fn safe_segment(name: &str) -> String {
    if name.is_empty() {
        return "%".to_owned();
    }
    let mut out = String::with_capacity(name.len());
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Diretório próprio por teste; apagado no fim.
    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("hy-dbplus-queries-{}", uuid::Uuid::new_v4())))
        }
        fn store(&self) -> QueryStore {
            QueryStore::new(self.0.join("queries"))
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn ids(queries: &[SavedQuery]) -> Vec<QueryTabId> {
        queries.iter().map(|q| q.id).collect()
    }

    #[test]
    fn salva_lista_em_ordem_substitui_e_nao_deixa_temporario() {
        let root = TempRoot::new();
        let store = root.store();
        assert!(
            store.list("c1", "loja").unwrap().is_empty(),
            "diretório ausente"
        );
        store.save("c1", "loja", 3, "select 3").unwrap();
        store.save("c1", "loja", 1, "select 1").unwrap();
        store.save("c1", "loja", 2, "select 2").unwrap();
        let queries = store.list("c1", "loja").unwrap();
        assert_eq!(ids(&queries), [1, 2, 3]);
        assert_eq!(queries[2].sql, "select 3");
        store.save("c1", "loja", 2, "select 'dois'").unwrap();
        assert_eq!(store.list("c1", "loja").unwrap()[1].sql, "select 'dois'");
        let dir = store.dir().join("c1").join("loja");
        let names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().all(|n| n.ends_with(".sql")), "{names:?}");
        // Outro banco e outra conexão não se misturam.
        assert!(store.list("c1", "outro").unwrap().is_empty());
        assert!(store.list("c2", "loja").unwrap().is_empty());
    }

    #[test]
    fn texto_vazio_apaga_remove_e_remove_connection_limpam_e_ausente_nao_e_erro() {
        let root = TempRoot::new();
        let store = root.store();
        store.save("c1", "loja", 1, "select 1").unwrap();
        store.save("c1", "loja", 2, "select 2").unwrap();
        store.save("c1", "rh", 1, "select 'rh'").unwrap();
        store.save("c1", "loja", 1, "  \n\t ").unwrap();
        assert_eq!(ids(&store.list("c1", "loja").unwrap()), [2]);
        store.remove("c1", "loja", 2).unwrap();
        store.remove("c1", "loja", 2).unwrap();
        store.remove("c1", "loja", 99).unwrap();
        assert!(store.list("c1", "loja").unwrap().is_empty());
        assert_eq!(ids(&store.list("c1", "rh").unwrap()), [1]);
        store.remove_connection("c1").unwrap();
        store.remove_connection("c1").unwrap();
        assert!(store.list("c1", "rh").unwrap().is_empty());
        assert!(!store.dir().join("c1").exists());
    }

    #[test]
    fn nomes_perigosos_ficam_dentro_do_diretorio_e_arquivos_estranhos_sao_ignorados() {
        let root = TempRoot::new();
        let store = root.store();
        let database = "../fora/da pasta\\x";
        store.save("c/1", database, 1, "select 1").unwrap();
        let conn_dir = store.dir().join("c%2F1");
        assert!(conn_dir.is_dir(), "conexão escapada");
        let dbs: Vec<PathBuf> = std::fs::read_dir(&conn_dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(dbs.len(), 1);
        assert!(dbs[0].starts_with(store.dir()), "{dbs:?}");
        assert!(!root.0.join("fora").exists());
        assert_eq!(store.list("c/1", database).unwrap()[0].sql, "select 1");
        assert_eq!(safe_segment("loja_2-x"), "loja_2-x");
        assert_eq!(safe_segment("a/b c"), "a%2Fb%20c");
        assert_eq!(safe_segment("ação"), "a%C3%A7%C3%A3o");
        assert_eq!(safe_segment(""), "%");
        assert_eq!(safe_segment("%"), "%25");
        // Arquivos que não são `<n>.sql` canônicos não viram aba.
        let dir = store.dir().join("c%2F1").join(safe_segment(database));
        std::fs::write(dir.join("notas.txt"), "x").unwrap();
        std::fs::write(dir.join("abc.sql"), "x").unwrap();
        std::fs::write(dir.join("7.SQL"), "x").unwrap();
        std::fs::write(dir.join("007.sql"), "x").unwrap();
        std::fs::write(dir.join("+7.sql"), "x").unwrap();
        assert_eq!(ids(&store.list("c/1", database).unwrap()), [1]);
    }

    /// Nome vazio não vira componente vazio: `remove_connection("")` apagaria o diretório inteiro e
    /// `save("c1", "", …)` gravaria no diretório da conexão (revisão de 22/10/2026).
    #[test]
    fn nome_vazio_nao_sai_do_lugar() {
        let root = TempRoot::new();
        let store = root.store();
        store.save("c1", "loja", 1, "select 1").unwrap();
        store.save("", "loja", 1, "select 'sem conexão'").unwrap();
        store.save("c1", "", 2, "select 'sem banco'").unwrap();
        assert_eq!(ids(&store.list("c1", "loja").unwrap()), [1]);
        assert_eq!(ids(&store.list("", "loja").unwrap()), [1]);
        assert_eq!(ids(&store.list("c1", "").unwrap()), [2]);
        store.remove_connection("").unwrap();
        assert!(store.dir().join("c1").is_dir(), "apagou a conexão errada");
        assert_eq!(ids(&store.list("c1", "loja").unwrap()), [1]);
        assert!(store.list("", "loja").unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn diretorios_0700_e_arquivo_0600() {
        use std::os::unix::fs::PermissionsExt;
        let root = TempRoot::new();
        let store = root.store();
        store.save("c1", "loja", 1, "select 1").unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(store.dir()), 0o700);
        assert_eq!(mode(&store.dir().join("c1")), 0o700);
        assert_eq!(mode(&store.dir().join("c1").join("loja")), 0o700);
        assert_eq!(
            mode(&store.dir().join("c1").join("loja").join("1.sql")),
            0o600
        );
    }

    #[test]
    fn erro_de_leitura_vira_io_com_o_caminho() {
        let root = TempRoot::new();
        let store = root.store();
        // Um arquivo onde deveria estar o diretório do banco: `read_dir` falha com `NotADirectory`
        // nas duas plataformas (Unix `ENOTDIR`, Windows `ERROR_DIRECTORY`). Com o arquivo no lugar
        // do diretório da conexão, o Windows dá `ERROR_PATH_NOT_FOUND`, que é `NotFound`, e o `list`
        // devolve a lista vazia (nada a restaurar ali mesmo); o CI do Windows pegou isso em 23/10.
        std::fs::create_dir_all(store.dir().join("c1")).unwrap();
        std::fs::write(store.dir().join("c1").join("loja"), "não sou um diretório").unwrap();
        let err = store.list("c1", "loja").unwrap_err();
        assert!(
            matches!(&err, EngineError::Io { path, .. } if path.starts_with(store.dir())),
            "{err:?}"
        );
        let err = store.save("c1", "loja", 1, "select 1").unwrap_err();
        assert!(matches!(err, EngineError::Io { .. }), "{err:?}");
    }

    #[test]
    fn default_dir_fica_ao_lado_do_arquivo_de_conexoes() {
        let dir = config_dir_from(Some("/tmp/xdg".into()), None)
            .unwrap()
            .join("queries");
        assert_eq!(dir, PathBuf::from("/tmp/xdg/hy-dbplus/queries"));
    }
}
