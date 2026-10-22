//! O que descreve uma conexão (sem a senha) e o tipo que carrega a senha.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Modo TLS, com a semântica do libpq: `prefer` e `require` cifram sem verificar o
/// certificado (todo servidor local tem certificado autoassinado); `verify-full` verifica com as
/// raízes do sistema e o nome do host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TlsMode {
    Disable,
    #[default]
    Prefer,
    Require,
    VerifyFull,
}

impl TlsMode {
    pub const ALL: [TlsMode; 4] = [
        TlsMode::Disable,
        TlsMode::Prefer,
        TlsMode::Require,
        TlsMode::VerifyFull,
    ];

    /// Nome como aparece na UI e no TOML.
    pub fn label(self) -> &'static str {
        match self {
            TlsMode::Disable => "disable",
            TlsMode::Prefer => "prefer",
            TlsMode::Require => "require",
            TlsMode::VerifyFull => "verify-full",
        }
    }
}

/// Como chegar ao servidor. Vai para o TOML de conexões; a senha fica fora, em [`Secret`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    /// Banco inicial da conexão (`postgres` por padrão).
    pub database: String,
    #[serde(default)]
    pub tls: TlsMode,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            port: 5432,
            user: "postgres".into(),
            database: "postgres".into(),
            tls: TlsMode::Prefer,
        }
    }
}

/// A senha. Sem `Display`, `Debug` fixo e sem `Serialize`: só sai por [`Secret::expose`], no
/// ponto em que o driver a entrega ao servidor.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_senha_nao_aparece_no_debug() {
        let s = Secret::new("hunter2");
        assert_eq!(format!("{s:?}"), "Secret(***)");
        assert_eq!(s.expose(), "hunter2");
        assert!(Secret::default().is_empty());
    }

    #[test]
    fn o_modo_tls_vai_e_volta_em_kebab_case_e_prefer_e_o_padrao() {
        for mode in TlsMode::ALL {
            let cfg = ConnectionConfig {
                tls: mode,
                ..ConnectionConfig::default()
            };
            let text = toml::to_string(&cfg).unwrap();
            assert!(
                text.contains(&format!("tls = \"{}\"", mode.label())),
                "{text}"
            );
            let back: ConnectionConfig = toml::from_str(&text).unwrap();
            assert_eq!(back, cfg);
        }
        let no_tls: ConnectionConfig =
            toml::from_str("host = \"db\"\nport = 5433\nuser = \"u\"\ndatabase = \"d\"\n").unwrap();
        assert_eq!(no_tls.tls, TlsMode::Prefer);
        assert_eq!(no_tls.port, 5433);
    }
}
