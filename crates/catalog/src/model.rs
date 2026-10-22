//! O que a árvore mostra. Tudo texto: nomes e tipos como o servidor os escreve.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    Table,
    View,
    MaterializedView,
    /// Tabela estrangeira (FDW): lida como tabela, com ícone próprio um dia.
    Foreign,
}

impl TableKind {
    pub fn is_view(self) -> bool {
        matches!(self, TableKind::View | TableKind::MaterializedView)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub schema: String,
    pub name: String,
    pub kind: TableKind,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    /// Tipo legível (`format_type`): `integer`, `character varying(80)`, `numeric(10,2)`.
    pub data_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    /// Posição na tabela, a partir de 1.
    pub position: i32,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub schema: String,
    pub name: String,
    /// Argumentos como o servidor escreve: `integer, text`.
    pub arguments: String,
    pub returns: String,
    /// `function`, `procedure`, `aggregate` ou `window`.
    pub kind: String,
}

impl Function {
    /// `nome(argumentos)`, como aparece na árvore.
    pub fn signature(&self) -> String {
        format!("{}({})", self.name, self.arguments)
    }
}
