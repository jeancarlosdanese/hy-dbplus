//! Conexões e execução de SQL: a trait [`Driver`] (conectar) e [`Connection`] (ping, versão do
//! servidor, listar bancos; query com stream e cancelamento no M3), com a implementação
//! PostgreSQL em [`postgres`].
//!
//! Camada 0: não conhece nenhuma outra crate do workspace. Assíncrona sobre `tokio`; quem chama
//! `connect` precisa estar dentro de um runtime (a implementação PostgreSQL faz `tokio::spawn`
//! da tarefa de rede).
//!
//! Segredo: a senha entra por [`Secret`] e nunca sai em erro, log ou `Debug`.

use std::future::Future;

pub mod config;
pub mod error;
pub mod postgres;

pub use config::{ConnectionConfig, Secret, TlsMode};
pub use error::DriverError;

/// Um banco do servidor, como aparece no seletor da UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseInfo {
    pub name: String,
}

/// Uma conexão aberta com o servidor.
pub trait Connection: Send + Sync {
    /// Confirma que a conexão responde.
    fn ping(&self) -> impl Future<Output = Result<(), DriverError>> + Send;
    /// Versão do servidor para exibir, como `15.15` (pacotes Debian devolvem `16.4 (Ubuntu ...)`;
    /// para comparar versão use `server_version_num`, quando for preciso).
    fn server_version(&self) -> impl Future<Output = Result<String, DriverError>> + Send;
    /// Bancos do servidor que aceitam conexão (sem templates), em ordem alfabética.
    fn list_databases(&self)
    -> impl Future<Output = Result<Vec<DatabaseInfo>, DriverError>> + Send;
}

/// Abre conexões para um tipo de banco. Só PostgreSQL no MVP.
pub trait Driver: Send + Sync {
    type Conn: Connection;
    fn connect(
        &self,
        config: &ConnectionConfig,
        password: &Secret,
    ) -> impl Future<Output = Result<Self::Conn, DriverError>> + Send;
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_crate_compila_com_os_lints_do_workspace() {}
}
