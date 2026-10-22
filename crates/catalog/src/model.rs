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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Column {
    pub name: String,
    /// Tipo legível (`format_type`): `integer`, `character varying(80)`, `numeric(10,2)`.
    pub data_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    /// Posição na tabela, a partir de 1.
    pub position: i32,
    pub is_primary_key: bool,
    /// `COMMENT ON COLUMN`.
    pub comment: Option<String>,
    /// Coluna de identidade: `ALWAYS` ou `BY DEFAULT` (FEAT-009).
    pub identity: Option<String>,
    /// Coluna gerada (`GENERATED ALWAYS AS (expr) STORED`): a expressão.
    pub generated: Option<String>,
    /// Coluna gerada virtual (PostgreSQL 18, `attgenerated = 'v'`): o DDL escreve `VIRTUAL`.
    pub generated_virtual: bool,
}

/// Uma restrição da tabela, como `pg_constraint` a descreve (FEAT-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintKind {
    PrimaryKey,
    Unique,
    Check,
    ForeignKey,
    Exclusion,
}

impl ConstraintKind {
    pub fn label(self) -> &'static str {
        match self {
            ConstraintKind::PrimaryKey => "primary key",
            ConstraintKind::Unique => "unique",
            ConstraintKind::Check => "check",
            ConstraintKind::ForeignKey => "foreign key",
            ConstraintKind::Exclusion => "exclusion",
        }
    }
}

/// Para onde uma chave estrangeira aponta e o que faz em UPDATE e DELETE (`no action`,
/// `restrict`, `cascade`, `set null`, `set default`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub schema: String,
    pub table: String,
    pub columns: Vec<String>,
    pub on_update: String,
    pub on_delete: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    pub name: String,
    pub kind: ConstraintKind,
    pub columns: Vec<String>,
    /// Como o servidor a escreve (`pg_get_constraintdef`): `PRIMARY KEY (id)`, `CHECK ((x > 0))`.
    pub definition: String,
    pub references: Option<Reference>,
}

/// Um índice, como `pg_index` o descreve (FEAT-009).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    pub name: String,
    /// `btree`, `hash`, `gin`, `gist`, `brin`, `spgist`.
    pub algorithm: String,
    pub is_unique: bool,
    pub is_primary: bool,
    /// O índice implementa uma restrição (PK, UNIQUE, EXCLUDE): o DDL não o repete.
    pub is_constraint: bool,
    /// Colunas ou expressões, como o servidor as escreve.
    pub columns: Vec<String>,
    /// `WHERE` de índice parcial.
    pub condition: Option<String>,
    pub include: Vec<String>,
    /// `reloptions`: `fillfactor=70`.
    pub storage_parameters: Vec<String>,
    pub comment: Option<String>,
    /// `CREATE [UNIQUE] INDEX …` completo (`pg_get_indexdef`).
    pub definition: String,
}

/// Tudo o que a aba Structure, a aba Index e o DDL precisam de uma tabela ou view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableStructure {
    pub table: Table,
    /// `relkind = 'p'`: o DDL sai sem `PARTITION BY` e avisa.
    pub partitioned: bool,
    pub columns: Vec<Column>,
    pub constraints: Vec<Constraint>,
    pub indexes: Vec<Index>,
    /// `pg_get_viewdef` para view e view materializada.
    pub view_definition: Option<String>,
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
