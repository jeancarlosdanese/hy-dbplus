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
    // Workspace (FEAT-015 fase C).
    go_to_database => "Ir para banco", "Go to database";
    record_panel_show => "Mostrar o painel de registro", "Show the record panel";
    record_panel_hide => "Esconder o painel de registro", "Hide the record panel";
    open_another_database => "Abrir outro banco", "Open another database";
    close_database => "Fechar este banco", "Close this database";
    search_tree => "Buscar tabela ou função", "Search tables and functions";
    tables => "Tabelas", "Tables";
    functions => "Funções", "Functions";
    show_columns => "Mostrar colunas", "Show columns";
    hide_columns => "Esconder colunas", "Hide columns";
    loading_columns => "carregando…", "loading…";
    reload_catalog => "Recarregar o catálogo", "Reload catalog";
    no_database_title => "Nenhum banco aberto", "No database open";
    no_database_hint => "Escolha um banco para ver as tabelas.", "Pick a database to see its tables.";
    opening_database_generic => "Abrindo banco…", "Opening database…";
    nothing_open => "Nada aberto", "Nothing open";
    no_table_open => "Nenhuma tabela aberta", "No table open";
    no_visible_schema => "Banco sem schema visível.", "Database with no visible schema.";
    no_data => "Sem dados.", "No data.";
    new_query => "Nova consulta SQL", "New SQL query";
    close_tab => "Fechar a aba", "Close tab";
    databases => "Bancos", "Databases";
    search_databases => "Buscar banco", "Search databases";
    open_hint => "abrir", "open";
    no_database_match => "Nenhum banco com esse nome.", "No database with that name.";
    review_and_commit => "Revisar e gravar", "Review and commit";
    discard_and_close => "Descartar e fechar", "Discard and close";
    discard_and_disconnect => "Descartar e desconectar", "Discard and disconnect";
    // Aba de tabela (FEAT-015 fase D).
    view_data => "Dados", "Data";
    view_structure => "Estrutura", "Structure";
    view_indexes => "Índices", "Indexes";
    reload => "Recarregar", "Reload";
    view_ddl => "Ver DDL", "View DDL";
    read_only_tag => "só leitura: sem chave primária", "read-only: no primary key";
    lock_hint => "Grave ou descarte as alterações antes", "Commit or discard the changes first";
    lock_detail => "Ordenar, paginar, recarregar e trocar de tabela esperam a gravação ou o descarte",
        "Sorting, paging, reloading and switching tables wait for the commit or the discard";
    double_click_hint => "Duplo clique edita a célula", "Double-click to edit a cell";
    discard => "Descartar", "Discard";
    delete_rows => "Apagar linhas", "Delete rows";
    delete_rows_hint => "Marca as linhas selecionadas para apagar; de novo desmarca",
        "Marks the selected rows for deletion; again to unmark";
    rows_per_page => "Linhas por página", "Rows per page";
    previous_page => "Página anterior", "Previous page";
    next_page => "Próxima página", "Next page";
    // Painel de registro.
    no_row_selected => "Nenhuma linha selecionada", "No row selected";
    fields => "Campos", "Fields";
    marked_for_deletion => "marcada para apagar", "marked for deletion";
    changed => "alterado", "changed";
    multiple_values => "Múltiplos valores", "Multiple values";
    set_null => "Definir NULL", "Set NULL";
    set_null_hint => "Põe NULL nas linhas selecionadas", "Sets NULL on the selected rows";
    revert_field => "Reverter campo", "Revert field";
    show_all => "mostrar tudo", "show all";
    show_less => "mostrar menos", "show less";
    copy => "Copiar", "Copy";
    // Gravação.
    commit_subtitle => "Roda numa transação: ou grava tudo, ou não grava nada.",
        "Runs in one transaction: everything is written, or nothing is.";
    copy_sql => "Copiar SQL", "Copy SQL";
    commit_to_database => "Gravar no banco", "Commit to database";
    committing => "Gravando…", "Committing…";
    // Estrutura e Índices.
    columns_title => "Colunas", "Columns";
    constraints_title => "Restrições", "Constraints";
    definition => "Definição", "Definition";
    no_constraints => "Nenhuma restrição.", "No constraints.";
    column => "Coluna", "Column";
    data_type => "Tipo", "Type";
    nullable => "Aceita nulo", "Nullable";
    default_value => "Padrão", "Default";
    comment => "Comentário", "Comment";
    name => "Nome", "Name";
    yes => "sim", "yes";
    no => "não", "no";
    primary_key => "Chave primária", "Primary key";
    foreign_key => "Chave estrangeira", "Foreign key";
    unique_constraint => "Única", "Unique";
    check_constraint => "Check", "Check";
    exclusion_constraint => "Exclusão", "Exclusion";
    loading_structure => "Carregando estrutura…", "Loading structure…";
    loading_indexes => "Carregando índices…", "Loading indexes…";
    no_indexes => "Nenhum índice.", "No indexes.";
    method => "Método", "Method";
    unique => "Único", "Unique";
    condition => "Condição", "Condition";
    include => "Include", "Include";
    parameters => "Parâmetros", "Parameters";
    ddl_subtitle => "Montado a partir do catálogo. Só leitura.", "Built from the catalog. Read-only.";
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

impl Tr {
    /// "Abrindo loja…".
    pub fn opening_database(self, database: &str) -> String {
        match self.0 {
            Language::PtBr => format!("Abrindo {database}…"),
            Language::En => format!("Opening {database}…"),
        }
    }

    /// O convite da área central sem aba: com banco aberto só falta a tabela ou a consulta.
    pub fn nothing_open_hint(self, database_open: bool, palette: &str, new_query: &str) -> String {
        match (self.0, database_open) {
            (Language::PtBr, true) => {
                format!("Clique numa tabela na árvore ou crie uma consulta com {new_query}.")
            }
            (Language::PtBr, false) => format!(
                "Abra um banco com {palette}. Depois, clique numa tabela ou crie uma consulta com {new_query}."
            ),
            (Language::En, true) => {
                format!("Click a table in the tree or start a query with {new_query}.")
            }
            (Language::En, false) => format!(
                "Open a database with {palette}. Then click a table or start a query with {new_query}."
            ),
        }
    }

    /// O título da confirmação de fechar uma aba.
    pub fn close_tab_title(self, name: &str) -> String {
        match self.0 {
            Language::PtBr => format!("Fechar a aba {name}?"),
            Language::En => format!("Close the {name} tab?"),
        }
    }

    pub fn close_database_title(self, database: &str) -> String {
        match self.0 {
            Language::PtBr => format!("Fechar o banco {database}?"),
            Language::En => format!("Close the {database} database?"),
        }
    }

    pub fn disconnect_title(self, connection: &str) -> String {
        match self.0 {
            Language::PtBr => format!("Desconectar {connection}?"),
            Language::En => format!("Disconnect {connection}?"),
        }
    }

    /// "Há 2 alterações e 1 exclusão que ainda não foram gravadas. Se fechar, elas se perdem."
    pub fn pending_body(self, updates: u64, deletes: u64) -> String {
        let parts = self.change_parts(updates, deletes);
        let many = updates + deletes > 1;
        match self.0 {
            Language::PtBr => {
                let what = parts.join(" e ");
                if many {
                    format!("Há {what} que ainda não foram gravadas. Se fechar, elas se perdem.")
                } else {
                    format!("Há {what} que ainda não foi gravada. Se fechar, ela se perde.")
                }
            }
            Language::En => {
                let what = parts.join(" and ");
                if many {
                    format!("{what} have not been committed. Closing discards them.")
                } else {
                    format!("{what} has not been committed. Closing discards it.")
                }
            }
        }
    }

    /// Fechar com uma célula aberta e nada mais pendente.
    pub fn cell_edit_body(self) -> &'static str {
        match self.0 {
            Language::PtBr => "Uma célula está em edição. Se fechar, o valor digitado se perde.",
            Language::En => "A cell is being edited. Closing discards the typed value.",
        }
    }

    /// ["2 alterações", "1 exclusão"]: só as partes que não são zero.
    fn change_parts(self, updates: u64, deletes: u64) -> Vec<String> {
        let mut parts = Vec::new();
        if updates > 0 {
            parts.push(self.changes(updates));
        }
        if deletes > 0 {
            parts.push(self.deletions(deletes));
        }
        parts
    }

    /// "1.284 linhas · 312 kB".
    pub fn table_summary(self, rows: u64, size: &str) -> String {
        format!("{} · {size}", self.rows(rows))
    }

    /// O contador do rodapé: "1–100 de 1.284"; sem o total ainda, "…" no lugar dele.
    pub fn page_range(self, first: u64, last: u64, total: Option<u64>) -> String {
        let total = total.map_or_else(|| "…".to_owned(), |t| self.number(t));
        let of = match self.0 {
            Language::PtBr => "de",
            Language::En => "of",
        };
        if last == 0 {
            format!("0 {of} {total}")
        } else {
            format!("{}–{} {of} {total}", self.number(first), self.number(last))
        }
    }

    /// "2 selecionadas de 100 linhas".
    pub fn selected_rows(self, selected: u64, shown: u64) -> String {
        match self.0 {
            Language::PtBr => format!(
                "{} selecionada{} de {}",
                self.number(selected),
                if selected == 1 { "" } else { "s" },
                self.rows(shown)
            ),
            Language::En => format!("{} selected of {}", self.number(selected), self.rows(shown)),
        }
    }

    /// O chip das pendências: "2 alterações, 1 exclusão".
    pub fn pending_chip(self, updates: u64, deletes: u64) -> String {
        let parts = self.change_parts(updates, deletes);
        parts.join(", ")
    }

    /// "Linha 3".
    pub fn row_title(self, n: u64) -> String {
        match self.0 {
            Language::PtBr => format!("Linha {}", self.number(n)),
            Language::En => format!("Row {}", self.number(n)),
        }
    }

    /// "Gravar 2 alterações e 1 exclusão em pedidos".
    pub fn commit_title(self, updates: u64, deletes: u64, table: &str) -> String {
        let parts = self.change_parts(updates, deletes);
        match self.0 {
            Language::PtBr => format!("Gravar {} em {table}", parts.join(" e ")),
            Language::En => format!("Commit {} to {table}", parts.join(" and ")),
        }
    }

    /// "6 colunas · 3 restrições".
    pub fn structure_summary(self, columns: u64, constraints: u64) -> String {
        format!(
            "{} · {}",
            self.count(columns, ("coluna", "colunas"), ("column", "columns")),
            self.count(
                constraints,
                ("restrição", "restrições"),
                ("constraint", "constraints")
            )
        )
    }

    pub fn indexes_count(self, n: u64) -> String {
        self.count(n, ("índice", "índices"), ("index", "indexes"))
    }

    /// "Nenhuma linha selecionada. ⌘-clique numa linha do resultado."
    pub fn no_row_selected_sql(self, command: &str) -> String {
        match self.0 {
            Language::PtBr => {
                format!("Nenhuma linha selecionada. {command}-clique numa linha do resultado.")
            }
            Language::En => format!("No row selected. {command}-click a row in the result."),
        }
    }

    /// O texto da confirmação de fechar uma aba SQL com texto no editor.
    pub fn query_text_body(self) -> &'static str {
        match self.0 {
            Language::PtBr => "O texto do editor desta consulta se perde se fechar.",
            Language::En => "The text in this query editor is lost if you close it.",
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

    /// `1 exclusão`, `2 exclusões`; `1 deletion`, `2 deletions`.
    pub fn deletions(self, n: u64) -> String {
        self.count(n, ("exclusão", "exclusões"), ("deletion", "deletions"))
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

/// A tecla de comando para texto: "⌘" no macOS e "Ctrl" fora dele.
pub fn command_label(ctx: &egui::Context) -> &'static str {
    if ctx.os() == egui::os::OperatingSystem::Mac {
        "⌘"
    } else {
        "Ctrl"
    }
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
        assert_eq!(PT.deletions(2), "2 exclusões");
        assert_eq!(PT.pending_chip(2, 1), "2 alterações, 1 exclusão");
        assert_eq!(EN.pending_chip(0, 3), "3 deletions");
        assert_eq!(
            PT.commit_title(2, 1, "pedidos"),
            "Gravar 2 alterações e 1 exclusão em pedidos"
        );
        assert_eq!(
            PT.commit_title(1, 0, "pedidos"),
            "Gravar 1 alteração em pedidos"
        );
        assert_eq!(
            EN.commit_title(1, 2, "pedidos"),
            "Commit 1 change and 2 deletions to pedidos"
        );
        assert_eq!(EN.commit_title(2, 0, "t"), "Commit 2 changes to t");
        assert_eq!(PT.page_range(1, 100, Some(1284)), "1–100 de 1.284");
        assert_eq!(EN.page_range(1, 100, None), "1–100 of …");
        assert_eq!(PT.page_range(0, 0, Some(0)), "0 de 0");
        assert_eq!(PT.selected_rows(1, 3), "1 selecionada de 3 linhas");
        assert_eq!(PT.selected_rows(2, 3), "2 selecionadas de 3 linhas");
        assert_eq!(PT.structure_summary(6, 1), "6 colunas · 1 restrição");
        assert_eq!(EN.indexes_count(1), "1 index");
        assert_eq!(EN.deletions(1), "1 deletion");
        assert_eq!(
            PT.pending_body(2, 1),
            "Há 2 alterações e 1 exclusão que ainda não foram gravadas. Se fechar, elas se perdem."
        );
        assert_eq!(
            PT.pending_body(1, 0),
            "Há 1 alteração que ainda não foi gravada. Se fechar, ela se perde."
        );
        assert_eq!(
            EN.pending_body(0, 2),
            "2 deletions have not been committed. Closing discards them."
        );
        assert_eq!(
            EN.pending_body(1, 0),
            "1 change has not been committed. Closing discards it."
        );
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
