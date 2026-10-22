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
}
