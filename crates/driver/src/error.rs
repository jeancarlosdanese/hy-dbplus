//! Erros do driver. Nenhuma variante carrega a senha; a mensagem do servidor entra como texto.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DriverError {
    #[error("configuração inválida: {0}")]
    Config(String),
    #[error("não conectou a {host}:{port}: {reason}")]
    Connect {
        host: String,
        port: u16,
        reason: String,
    },
    #[error("autenticação recusada para o usuário {user}: {reason}")]
    Auth { user: String, reason: String },
    #[error("TLS: {0}")]
    Tls(String),
    #[error("tempo esgotado ao conectar a {host}:{port}")]
    Timeout { host: String, port: u16 },
    #[error("o servidor fechou a conexão")]
    Closed,
    #[error("erro do banco: {0}")]
    Db(String),
    /// Um statement do lote falhou; a transação foi desfeita. `position` a partir de 1.
    #[error("statement {position} do lote falhou e nada foi gravado: {reason}")]
    Statement { position: usize, reason: String },
    /// Um statement do lote afetou outro número de linhas; a transação foi desfeita.
    #[error(
        "statement {position} do lote afetou {got} linha(s), esperava {expected}; nada foi gravado"
    )]
    RowCount {
        position: usize,
        expected: u64,
        got: u64,
    },
}
