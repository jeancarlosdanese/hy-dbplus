//! As preferências do usuário (FEAT-015): `preferences.toml` ao lado do `connections.toml`, hoje só
//! com o idioma da interface. Nunca guarda segredo. Arquivo ausente, ilegível ou com valor
//! desconhecido vale como "seguir o sistema": preferência nunca impede o app de abrir.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::EngineError;
use crate::store::{config_dir_from, save_toml};

/// O idioma da interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    /// Português do Brasil.
    #[default]
    PtBr,
    /// Inglês.
    En,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::PtBr, Language::En];

    /// O código guardado no arquivo.
    pub fn code(self) -> &'static str {
        match self {
            Language::PtBr => "pt-BR",
            Language::En => "en",
        }
    }

    /// O código lido do arquivo; qualquer outro valor é desconhecido.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.code() == code)
    }

    /// O idioma para um locale do sistema (`pt-BR`, `pt_PT`, `en-US`, `es`…): qualquer português
    /// vira português do Brasil; o resto, e locale ausente, vira inglês.
    pub fn from_locale(locale: Option<&str>) -> Self {
        let lang = locale
            .and_then(|l| l.split(['-', '_', '.', '@']).next())
            .unwrap_or_default();
        if lang.eq_ignore_ascii_case("pt") {
            Language::PtBr
        } else {
            Language::En
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    language: Option<String>,
}

/// As preferências e o arquivo delas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferenceStore {
    path: PathBuf,
    language: Option<Language>,
}

impl PreferenceStore {
    /// `$XDG_CONFIG_HOME/hy-dbplus/preferences.toml`, ou `~/.config/hy-dbplus/preferences.toml`.
    pub fn default_path() -> Result<PathBuf, EngineError> {
        Ok(
            config_dir_from(std::env::var_os("XDG_CONFIG_HOME"), std::env::home_dir())?
                .join("preferences.toml"),
        )
    }

    /// Lê o arquivo. Nunca falha: ausente, ilegível, TOML inválido ou idioma desconhecido viram
    /// "sem preferência".
    pub fn load(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let language = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| toml::from_str::<File>(&text).ok())
            .and_then(|file| file.language)
            .and_then(|code| Language::from_code(&code));
        Self { path, language }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// O idioma escolhido pelo usuário; `None` segue o sistema.
    pub fn language(&self) -> Option<Language> {
        self.language
    }

    /// Guarda a escolha e grava o arquivo. Em erro de gravação a escolha vale na sessão e o erro
    /// sobe para a UI avisar.
    pub fn set_language(&mut self, language: Language) -> Result<(), EngineError> {
        self.language = Some(language);
        let file = File {
            language: Some(language.code().to_owned()),
        };
        let text = toml::to_string_pretty(&file).map_err(|e| EngineError::Config(e.to_string()))?;
        save_toml(&self.path, &text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("hy-dbplus-prefs-{}", uuid::Uuid::new_v4())))
        }
        fn file(&self) -> PathBuf {
            self.0.join("hy-dbplus").join("preferences.toml")
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn locale_do_sistema_vira_idioma() {
        for pt in ["pt-BR", "pt_BR", "pt", "pt-PT", "PT_br.UTF-8"] {
            assert_eq!(Language::from_locale(Some(pt)), Language::PtBr, "{pt}");
        }
        for en in ["en-US", "es", "es-AR", "fr_FR", "", "português", "ptx"] {
            assert_eq!(Language::from_locale(Some(en)), Language::En, "{en}");
        }
        assert_eq!(Language::from_locale(None), Language::En);
    }

    #[test]
    fn arquivo_ausente_e_sem_preferencia_e_a_escolha_persiste() {
        let root = TempRoot::new();
        let mut prefs = PreferenceStore::load(root.file());
        assert_eq!(prefs.language(), None);
        prefs.set_language(Language::En).unwrap();
        assert_eq!(
            PreferenceStore::load(root.file()).language(),
            Some(Language::En)
        );
        prefs.set_language(Language::PtBr).unwrap();
        let text = std::fs::read_to_string(root.file()).unwrap();
        assert_eq!(text.trim(), r#"language = "pt-BR""#);
        assert_eq!(
            PreferenceStore::load(root.file()).language(),
            Some(Language::PtBr)
        );
    }

    #[test]
    fn arquivo_invalido_ou_idioma_desconhecido_segue_o_sistema() {
        let root = TempRoot::new();
        std::fs::create_dir_all(root.file().parent().unwrap()).unwrap();
        for text in [
            "language = \"es\"",
            "language = 3",
            "não é toml [",
            "\u{0}\u{ff}",
            "",
        ] {
            std::fs::write(root.file(), text).unwrap();
            assert_eq!(
                PreferenceStore::load(root.file()).language(),
                None,
                "{text:?}"
            );
        }
        // Um diretório no lugar do arquivo (ilegível) também.
        std::fs::remove_file(root.file()).unwrap();
        std::fs::create_dir_all(root.file()).unwrap();
        assert_eq!(PreferenceStore::load(root.file()).language(), None);
    }

    #[test]
    fn falha_ao_gravar_e_erro_e_a_escolha_vale_na_sessao() {
        let root = TempRoot::new();
        // O diretório de configuração é um arquivo: a gravação falha nas três plataformas.
        std::fs::create_dir_all(&root.0).unwrap();
        std::fs::write(root.0.join("hy-dbplus"), "x").unwrap();
        let mut prefs = PreferenceStore::load(root.file());
        assert!(prefs.set_language(Language::En).is_err());
        assert_eq!(prefs.language(), Some(Language::En));
    }

    #[cfg(unix)]
    #[test]
    fn o_arquivo_e_privado() {
        use std::os::unix::fs::PermissionsExt;
        let root = TempRoot::new();
        PreferenceStore::load(root.file())
            .set_language(Language::En)
            .unwrap();
        let mode = std::fs::metadata(root.file()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
