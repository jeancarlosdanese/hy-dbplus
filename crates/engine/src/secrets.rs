//! A senha de cada conexão, fora de arquivo: no chaveiro do sistema por `keyring` (Keychain no
//! macOS, Credential Manager no Windows, Secret Service no Linux), serviço `hy-dbplus`, conta =
//! id da conexão. Em teste, [`MemorySecrets`]: o chaveiro real nunca é tocado.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::EngineError;
use hy_dbplus_driver::Secret;

pub trait SecretStore: Send + Sync {
    fn set(&self, id: &str, secret: &Secret) -> Result<(), EngineError>;
    /// `None` quando não há senha guardada para a conexão.
    fn get(&self, id: &str) -> Result<Option<Secret>, EngineError>;
    /// Remover o que não existe não é erro.
    fn remove(&self, id: &str) -> Result<(), EngineError>;
}

/// O chaveiro do sistema.
#[derive(Debug, Clone)]
pub struct KeyringSecrets {
    service: String,
}

impl KeyringSecrets {
    pub const SERVICE: &'static str = "hy-dbplus";

    pub fn new() -> Self {
        Self {
            service: Self::SERVICE.into(),
        }
    }

    fn entry(&self, id: &str) -> Result<keyring::Entry, EngineError> {
        keyring::Entry::new(&self.service, id).map_err(|e| EngineError::Secret(e.to_string()))
    }
}

impl Default for KeyringSecrets {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for KeyringSecrets {
    fn set(&self, id: &str, secret: &Secret) -> Result<(), EngineError> {
        self.entry(id)?
            .set_password(secret.expose())
            .map_err(|e| EngineError::Secret(e.to_string()))
    }

    fn get(&self, id: &str) -> Result<Option<Secret>, EngineError> {
        match self.entry(id)?.get_password() {
            Ok(p) => Ok(Some(Secret::new(p))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(EngineError::Secret(e.to_string())),
        }
    }

    fn remove(&self, id: &str) -> Result<(), EngineError> {
        match self.entry(id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(EngineError::Secret(e.to_string())),
        }
    }
}

/// Senhas em memória: testes e, se o chaveiro faltar, uma sessão sem persistência.
#[derive(Debug, Default)]
pub struct MemorySecrets {
    map: Mutex<HashMap<String, Secret>>,
}

impl SecretStore for MemorySecrets {
    fn set(&self, id: &str, secret: &Secret) -> Result<(), EngineError> {
        self.map
            .lock()
            .map_err(|_| poisoned())?
            .insert(id.to_owned(), secret.clone());
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Option<Secret>, EngineError> {
        Ok(self.map.lock().map_err(|_| poisoned())?.get(id).cloned())
    }

    fn remove(&self, id: &str) -> Result<(), EngineError> {
        self.map.lock().map_err(|_| poisoned())?.remove(id);
        Ok(())
    }
}

fn poisoned() -> EngineError {
    EngineError::Config("armazenamento de senhas em memória indisponível".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memoria_guarda_le_e_remove_sem_erro_no_ausente() {
        let s = MemorySecrets::default();
        assert_eq!(s.get("x").unwrap(), None);
        s.set("x", &Secret::new("p")).unwrap();
        assert_eq!(
            s.get("x").unwrap().map(|p| p.expose().to_owned()),
            Some("p".into())
        );
        s.remove("x").unwrap();
        s.remove("x").unwrap();
        assert_eq!(s.get("x").unwrap(), None);
    }
}
