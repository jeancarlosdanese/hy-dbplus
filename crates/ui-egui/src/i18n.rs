//! Os textos da interface em português do Brasil e inglês (FEAT-015). Catálogo próprio, sem crate
//! de tradução: cada texto é um método de [`Tr`] com os dois idiomas lado a lado, e a macro não
//! compila um texto sem os dois. Nome de objeto do banco, tipo, SQL e erro do servidor nunca
//! passam por aqui: aparecem como o banco devolve.

use egui::{KeyboardShortcut, ModifierNames};
pub use hy_dbplus_engine::Language;

/// O idioma do sistema operacional; locale ilegível ou ausente vira inglês.
pub fn system_language() -> Language {
    Language::from_locale(sys_locale::get_locale().as_deref())
}

/// O idioma da interface: a escolha guardada, ou o do sistema.
pub fn resolve(preference: Option<Language>, system: impl FnOnce() -> Language) -> Language {
    preference.unwrap_or_else(system)
}

/// Os textos num idioma. `Copy`: as telas passam adiante sem empréstimo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tr(pub Language);

/// Um método por texto, com o português e o inglês. Texto sem um dos dois não compila.
macro_rules! texts {
    ($($(#[$doc:meta])* $name:ident => $pt:literal, $en:literal;)*) => {
        // As telas passam a usar o catálogo nas fases B a E da FEAT-015; a fase F tira esta licença.
        #[allow(dead_code)]
        impl Tr {
            $(
                $(#[$doc])*
                pub fn $name(self) -> &'static str {
                    match self.0 {
                        Language::PtBr => $pt,
                        Language::En => $en,
                    }
                }
            )*
        }
    };
}

texts! {
    // Ações comuns.
    ok => "OK", "OK";
    cancel => "Cancelar", "Cancel";
    save => "Salvar", "Save";
    close => "Fechar", "Close";
    delete => "Remover", "Delete";
    retry => "Tentar de novo", "Try again";
    connect => "Conectar", "Connect";
    disconnect => "Desconectar", "Disconnect";
    connections => "Conexões", "Connections";
    loading => "Carregando…", "Loading…";
    // Cabeçalho das conexões e barra superior.
    back_to_connections => "← Conexões", "← Connections";
    /// O botão da paleta, seguido do atalho.
    database => "Banco", "Database";
    panel_show => "◂ Painel", "◂ Panel";
    panel_hide => "Painel ▸", "Panel ▸";
    panel_hint => "Mostrar ou esconder o painel direito", "Show or hide the right panel";
    choose_database => "Escolher banco", "Choose database";
    no_match => "Nada com esse nome.", "Nothing with that name.";
    wait_for_run => "Aguarde a execução terminar", "Wait for the run to finish";
    // Tela de conexões (FEAT-015 fase B).
    new_connection_full => "Nova conexão", "New connection";
    search_connections => "Buscar conexão", "Search connections";
    no_connections_title => "Nenhuma conexão ainda", "No connections yet";
    no_connections_hint => "Cadastre o primeiro servidor PostgreSQL. A senha fica no Keychain, nunca em arquivo.",
        "Add your first PostgreSQL server. The password is stored in the Keychain, never in a file.";
    open => "Abrir", "Open";
    edit => "Editar", "Edit";
    connected => "conectado", "connected";
    connecting => "conectando…", "connecting…";
    hint_navigate => "navegar", "navigate";
    hint_connect => "conectar", "connect";
    hint_new => "nova conexão", "new connection";
    hint_edit => "editar", "edit";
    // Formulário de conexão.
    edit_connection => "Editar conexão", "Edit connection";
    form_subtitle => "A senha fica no Keychain do sistema.", "The password is stored in the system Keychain.";
    field_name => "Nome", "Name";
    field_environment => "Ambiente", "Environment";
    field_port => "Porta", "Port";
    field_user => "Usuário", "User";
    field_password => "Senha", "Password";
    field_database => "Banco inicial", "Initial database";
    password_hint_new => "Em branco usa a autenticação trust.", "Leave blank for trust authentication.";
    password_hint_edit => "Em branco mantém a senha atual.", "Leave blank to keep the current password.";
    error_name => "Dê um nome à conexão.", "Give the connection a name.";
    error_host => "Informe o host.", "Enter the host.";
    error_port => "Use um número de 1 a 65535.", "Use a number from 1 to 65535.";
    error_user => "Informe o usuário.", "Enter the user.";
    test_connection => "Testar conexão", "Test connection";
    testing => "Testando…", "Testing…";
    // Remover conexão.
    remove_connection => "Remover conexão", "Remove connection";
    remove_connection_body => "A senha guardada no Keychain e as consultas salvas desta conexão também são apagadas.",
        "The password stored in the Keychain and the saved queries of this connection are deleted too.";
    // Seletor de idioma.
    /// A dica do seletor PT | EN.
    language_hint => "Idioma da interface", "Interface language";
    /// O erro ao gravar a escolha de idioma (a escolha vale na sessão).
    language_save_failed => "Não consegui guardar o idioma escolhido; ele vale até fechar o app",
        "Couldn't save the chosen language; it applies until you close the app";
}

impl Tr {
    /// "Conectou em 84 ms · PostgreSQL 16.4".
    pub fn test_ok(self, elapsed_ms: u64, server_version: &str) -> String {
        match self.0 {
            Language::PtBr => format!("Conectou em {elapsed_ms} ms · PostgreSQL {server_version}"),
            Language::En => format!("Connected in {elapsed_ms} ms · PostgreSQL {server_version}"),
        }
    }

    /// O título do diálogo de remover, com o nome da conexão.
    pub fn remove_connection_title(self, name: &str) -> String {
        match self.0 {
            Language::PtBr => format!("Remover a conexão {name}?"),
            Language::En => format!("Remove the {name} connection?"),
        }
    }
}

// Plural: o português e o inglês têm a mesma regra (1 é singular; 0 e o resto, plural).
#[allow(dead_code)]
impl Tr {
    /// `1 linha`, `12.345 linhas`; `1 row`, `12,345 rows`.
    pub fn rows(self, n: u64) -> String {
        self.count(n, ("linha", "linhas"), ("row", "rows"))
    }

    /// `1 alteração`, `3 alterações`; `1 change`, `3 changes`.
    pub fn changes(self, n: u64) -> String {
        self.count(n, ("alteração", "alterações"), ("change", "changes"))
    }

    /// O número com o separador de milhar do idioma e a palavra no singular ou no plural.
    pub fn count(self, n: u64, pt: (&str, &str), en: (&str, &str)) -> String {
        let (one, many) = match self.0 {
            Language::PtBr => pt,
            Language::En => en,
        };
        let word = if n == 1 { one } else { many };
        format!("{} {word}", self.number(n))
    }

    /// `12.345` em português, `12,345` em inglês.
    pub fn number(self, n: u64) -> String {
        let sep = match self.0 {
            Language::PtBr => '.',
            Language::En => ',',
        };
        let digits = n.to_string();
        let mut out = String::with_capacity(digits.len() + digits.len() / 3);
        for (i, ch) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i).is_multiple_of(3) {
                out.push(sep);
            }
            out.push(ch);
        }
        out
    }
}

/// O rótulo de um atalho na plataforma: `⌘K` no macOS, `Ctrl+K` fora dele.
pub fn shortcut(shortcut: &KeyboardShortcut, mac: bool) -> String {
    let names = if mac {
        ModifierNames::SYMBOLS
    } else {
        ModifierNames::NAMES
    };
    shortcut.format(&names, mac)
}

/// [`shortcut`] na plataforma em que o egui está rodando.
#[allow(dead_code)]
pub fn shortcut_label(ctx: &egui::Context, sc: &KeyboardShortcut) -> String {
    shortcut(sc, ctx.os() == egui::os::OperatingSystem::Mac)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Key, Modifiers};

    const PT: Tr = Tr(Language::PtBr);
    const EN: Tr = Tr(Language::En);

    #[test]
    fn os_textos_mudam_com_o_idioma() {
        assert_eq!(PT.cancel(), "Cancelar");
        assert_eq!(EN.cancel(), "Cancel");
        assert_eq!(PT.connections(), "Conexões");
        assert_eq!(EN.connections(), "Connections");
    }

    #[test]
    fn plural_e_milhar_nos_dois_idiomas() {
        assert_eq!(PT.rows(0), "0 linhas");
        assert_eq!(PT.rows(1), "1 linha");
        assert_eq!(PT.rows(12_345), "12.345 linhas");
        assert_eq!(EN.rows(1), "1 row");
        assert_eq!(EN.rows(1_234_567), "1,234,567 rows");
        assert_eq!(PT.changes(3), "3 alterações");
        assert_eq!(EN.changes(1), "1 change");
        assert_eq!(PT.number(999), "999");
        assert_eq!(EN.number(1000), "1,000");
        assert_eq!(PT.number(u64::MAX), "18.446.744.073.709.551.615");
    }

    #[test]
    fn atalho_mostra_ctrl_fora_do_macos() {
        let k = KeyboardShortcut::new(Modifiers::COMMAND, Key::K);
        assert_eq!(shortcut(&k, true), "⌘K");
        assert_eq!(shortcut(&k, false), "Ctrl+K");
        let run_all = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Enter);
        assert!(shortcut(&run_all, false).starts_with("Ctrl+Shift+"));
        assert!(!shortcut(&run_all, false).contains('⌘'));
    }

    #[test]
    fn preferencia_vence_o_sistema_e_sem_ela_vale_o_sistema() {
        assert_eq!(resolve(Some(Language::En), || Language::PtBr), Language::En);
        assert_eq!(resolve(None, || Language::PtBr), Language::PtBr);
        assert_eq!(resolve(None, || Language::En), Language::En);
    }

    #[test]
    fn idioma_do_sistema_nao_entra_em_panico() {
        let _ = system_language();
    }
}
