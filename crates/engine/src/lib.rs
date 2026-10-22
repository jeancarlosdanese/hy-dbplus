//! Fachada usada por todos os frontends (app, CLI, MCP): conexões guardadas ([`store`]), senhas
//! no Keychain ([`secrets`]), sessões abertas e eventos assíncronos para a UI ([`session`]).
//!
//! Camada 3: depende de `driver`, `catalog` e `sql`. Nada aqui conhece `egui`. O runtime `tokio`
//! é do app: o engine recebe um `Handle` e nunca bloqueia quem o chama.

pub mod error;
pub mod secrets;
pub mod session;
pub mod store;

pub use error::EngineError;
pub use hy_dbplus_driver::postgres::PostgresDriver;
pub use hy_dbplus_driver::{ConnectionConfig, DatabaseInfo, DriverError, Secret, TlsMode};
pub use secrets::{KeyringSecrets, MemorySecrets, SecretStore};
pub use session::{Event, Session, Sessions};

/// As sessões PostgreSQL: o único driver do MVP, pronto para a UI e os apps.
pub type PgSessions = Sessions<PostgresDriver>;
pub use store::{Connection, ConnectionId, ConnectionStore, Tag};

#[cfg(test)]
mod tests {
    #[test]
    fn a_crate_compila_com_os_lints_do_workspace() {}
}
