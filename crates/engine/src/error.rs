//! Erros do engine. Caminho e motivo entram como texto; senha nunca.

use std::path::PathBuf;

use thiserror::Error;

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
