//! Conexões e execução de SQL: a trait [`Driver`] (conectar) e [`Connection`] (ping, versão do
//! servidor, listar bancos, query em texto, lote numa transação; stream e cancelamento no M5),
//! com a implementação PostgreSQL em [`postgres`].
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

/// Resultado de uma query como texto: o que o protocolo simples do PostgreSQL devolve, sem
/// conversão por tipo. Serve ao catálogo (`pg_catalog` já é texto) e, até o M3 pedir mais, ao grid.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextRows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}

impl TextRows {
    /// A célula `(linha, coluna)`, `None` se é NULL ou está fora do resultado.
    pub fn cell(&self, row: usize, col: usize) -> Option<&str> {
        self.rows.get(row)?.get(col)?.as_deref()
    }
}

/// O resultado de um statement do editor ([`Connection::execute`]): linhas (quem devolve
/// `RowDescription`, mesmo vazio) ou contagem de linhas afetadas (o resto); `truncated` quando o
/// limite cortou a leitura.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outcome {
    pub rows: Option<TextRows>,
    pub affected: Option<u64>,
    pub truncated: bool,
}

/// Um statement de um lote ([`Connection::execute_batch`]) e quantas linhas ele deve afetar;
/// `None` não confere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchStatement {
    pub sql: String,
    pub expected_rows: Option<u64>,
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
    /// Um statement pelo protocolo simples (sem prepared statement nem parâmetros), com o
    /// resultado em texto. Com vários statements no texto, todos rodam no servidor; o relato é o
    /// do primeiro que devolve um conjunto de linhas (mesmo vazio).
    fn query_text(&self, sql: &str) -> impl Future<Output = Result<TextRows, DriverError>> + Send;
    /// Os statements numa transação só deles (`BEGIN` … `COMMIT`), pelo protocolo simples, na
    /// ordem; nenhuma outra query desta conexão corre no meio. Devolve as linhas afetadas de cada
    /// um. Erro de um statement ou contagem diferente da esperada faz `ROLLBACK` e devolve
    /// [`DriverError::Statement`] ou [`DriverError::RowCount`] com a posição (a partir de 1):
    /// nada fica gravado. Erro no `BEGIN` ou no `COMMIT`, e conexão caída no meio, sobem como
    /// [`DriverError::Db`] ou [`DriverError::Closed`]; com a conexão caída no `COMMIT` o
    /// resultado é desconhecido (pode ter gravado). Cada `sql` deve ser um statement só, sem
    /// `COMMIT`/`ROLLBACK`: a contagem é a do último `CommandComplete` do texto (num `SELECT`,
    /// as linhas devolvidas).
    fn execute_batch(
        &self,
        statements: &[BatchStatement],
    ) -> impl Future<Output = Result<Vec<u64>, DriverError>> + Send;
    /// Um statement do editor pelo protocolo simples, lendo o resultado como stream: no máximo
    /// `limit` linhas ficam e o resto é descartado (`truncated = true`), mas o statement vai até
    /// o fim no servidor: nada é cancelado por causa do limite (dentro de uma transação aberta o
    /// ERROR do cancel a abortaria, e sem TLS um cancel em voo derruba o statement seguinte).
    /// O SQL roda como veio, sem `LIMIT` por fora. Erro do
    /// servidor sobe como [`DriverError::Db`]; cancelado (por [`Connection::cancel`] ou por fora,
    /// como um `statement_timeout`), como [`DriverError::Cancelled`] com o motivo do servidor.
    fn execute(
        &self,
        sql: &str,
        limit: Option<usize>,
    ) -> impl Future<Output = Result<Outcome, DriverError>> + Send;
    /// Pede ao servidor para cancelar o statement em curso nesta conexão (abre uma conexão
    /// própria só para isso). O statement cancelado volta com [`DriverError::Cancelled`]; só quem
    /// chamou sabe que o cancelamento foi dele.
    fn cancel(&self) -> impl Future<Output = Result<(), DriverError>> + Send;
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
