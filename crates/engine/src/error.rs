//! Erros do engine. Caminho e motivo entram como texto; senha nunca.

use std::path::PathBuf;

use thiserror::Error;

/// Por que um lote do grid não gravou: o SQL não pôde ser montado (sem chave) ou o banco recusou.
/// Vai tipado no `Event::ChangesFailed` para a interface escrever a mensagem no idioma dela.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ChangeError {
    #[error("{0}")]
    Edit(#[from] hy_dbplus_catalog::EditError),
    #[error("{0}")]
    Driver(#[from] hy_dbplus_driver::DriverError),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EngineError {
    #[error("configuração: {0}")]
    Config(String),
    #[error("{path}: {reason}")]
    Io { path: PathBuf, reason: String },
    #[error("{path}: arquivo de conexões inválido: {reason}")]
    Parse { path: PathBuf, reason: String },
    #[error("senha no chaveiro do sistema: {0}")]
    Secret(String),
    #[error("conexão desconhecida: {0}")]
    UnknownConnection(String),
}

impl EngineError {
    pub(crate) fn io(path: &std::path::Path, e: &std::io::Error) -> Self {
        EngineError::Io {
            path: path.to_path_buf(),
            reason: e.to_string(),
        }
    }
}
