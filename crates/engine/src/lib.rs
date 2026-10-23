//! Fachada usada por todos os frontends (app, CLI, MCP): conexões guardadas ([`store`]), senhas
//! no Keychain ([`secrets`]), queries das abas SQL em arquivo ([`queries`]), sessões abertas e
//! eventos assíncronos para a UI ([`session`]).
//!
//! Camada 3: depende de `driver`, `catalog` e `sql`. Nada aqui conhece `egui`. O runtime `tokio`
//! é do app: o engine recebe um `Handle` e nunca bloqueia quem o chama.

pub mod error;
pub mod queries;
pub mod record;
pub mod secrets;
pub mod session;
pub mod store;

pub use error::EngineError;
pub use hy_dbplus_catalog::{
    ChangeSet, Column, Constraint, ConstraintKind, Direction, EditError, Function, Index, Order,
    Reference, RowUpdate, Schema, Table, TableKind, TableStats, TableStructure, table_ddl,
};
pub use hy_dbplus_driver::postgres::PostgresDriver;
pub use hy_dbplus_driver::{
    ConnectionConfig, DatabaseInfo, Driver, DriverError, Outcome, Secret, TextRows, TlsMode,
};
pub use secrets::{KeyringSecrets, MemorySecrets, SecretStore};
pub use session::{
    Event, HISTORY_LIMIT, HistoryEntry, Page, QueryTabId, Session, Sessions, preview_sql,
};

/// As sessões PostgreSQL: o único driver do MVP, pronto para a UI e os apps.
pub type PgSessions = Sessions<PostgresDriver>;
pub use queries::{QueryStore, SavedQuery};
pub use store::{Connection, ConnectionId, ConnectionStore, Tag};

#[cfg(test)]
mod tests {
    #[test]
    fn a_crate_compila_com_os_lints_do_workspace() {}
}
