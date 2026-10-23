//! Modelos de SQL a partir de uma tabela ou view do catálogo (FEAT-013): o menu de contexto da
//! árvore os escreve no editor para o usuário completar. `SELECT`, `SELECT count(*)`, `INSERT`,
//! `UPDATE`, `DELETE`, `TRUNCATE` e `DROP … CASCADE`; o `CREATE` é o DDL da aba Structure
//! ([`crate::table_ddl`]) e precisa da estrutura, não só das colunas.
//!
//! Nada aqui executa: o texto vai para o editor e o usuário decide. Identificadores sempre por
//! `quote_ident`, como o resto do catálogo; cada modelo termina em `;` e o `UPDATE`/`DELETE` de
//! tabela sem chave primária avisa em comentário.

use crate::data::{qualified, quote_ident};
use crate::model::{Column, Table, TableKind};

/// Um modelo do menu. `Create` fica fora: é o DDL (`table_ddl`), que precisa da estrutura.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    Select,
    Count,
    Insert,
    Update,
    Delete,
    Truncate,
    Drop,
}

impl Template {
    /// Os modelos na ordem do menu.
    pub const ALL: [Template; 7] = [
        Template::Select,
        Template::Count,
        Template::Insert,
        Template::Update,
        Template::Delete,
        Template::Truncate,
        Template::Drop,
    ];

    /// O rótulo do item do menu.
    pub fn label(self) -> &'static str {
        match self {
            Template::Select => "SELECT",
            Template::Count => "SELECT count(*)",
            Template::Insert => "INSERT",
            Template::Update => "UPDATE",
            Template::Delete => "DELETE",
            Template::Truncate => "TRUNCATE",
            Template::Drop => "DROP … CASCADE",
        }
    }

    /// View e view materializada só leem (e caem): sem `INSERT`, `UPDATE`, `DELETE` nem `TRUNCATE`.
    pub fn applies_to(self, kind: TableKind) -> bool {
        match self {
            Template::Select | Template::Count | Template::Drop => true,
            Template::Insert | Template::Update | Template::Delete | Template::Truncate => {
                !kind.is_view()
            }
        }
    }

    /// Precisa das colunas da tabela (o resto só do nome).
    pub fn needs_columns(self) -> bool {
        matches!(
            self,
            Template::Select | Template::Insert | Template::Update | Template::Delete
        )
    }
}

/// O SQL do modelo para a tabela, com as colunas na ordem do catálogo (podem faltar para os modelos
/// que não precisam delas).
pub fn template_sql(template: Template, table: &Table, columns: &[Column]) -> String {
    let target = qualified(&table.schema, &table.name);
    match template {
        Template::Select => select_sql(&target, columns),
        Template::Count => format!("SELECT count(*) FROM {target};"),
        Template::Insert => insert_sql(&target, columns),
        Template::Update => update_sql(&target, columns),
        Template::Delete => delete_sql(&target, columns),
        Template::Truncate => format!("TRUNCATE TABLE {target};"),
        Template::Drop => {
            let object = match table.kind {
                TableKind::View => "VIEW",
                TableKind::MaterializedView => "MATERIALIZED VIEW",
                TableKind::Foreign => "FOREIGN TABLE",
                TableKind::Table => "TABLE",
            };
            format!("DROP {object} {target} CASCADE;")
        }
    }
}

/// Um valor de exemplo pelo tipo (`format_type`, em minúsculas): o usuário troca. Tipo sem typmod
/// casa por igualdade (`date` não pode pegar `daterange`, nem `json` o `jsonpath`); os que levam
/// typmod ou qualificador (`character varying(80)`, `numeric(10,2)`, `time with time zone`,
/// `timestamp without time zone`) casam pelo começo. Array vira `'{}'`; o resto, `NULL`.
pub fn example_value(data_type: &str) -> &'static str {
    let t = data_type.trim().to_ascii_lowercase();
    if t.ends_with("[]") {
        return "'{}'";
    }
    const EXACT: [(&str, &str); 15] = [
        ("text", "''"),
        ("citext", "''"),
        ("name", "''"),
        ("integer", "0"),
        ("bigint", "0"),
        ("smallint", "0"),
        ("real", "0"),
        ("double precision", "0"),
        ("money", "0"),
        ("boolean", "false"),
        ("date", "now()"),
        ("uuid", "gen_random_uuid()"),
        ("json", "'{}'"),
        ("jsonb", "'{}'"),
        ("bytea", "'\\x'"),
    ];
    const PREFIXES: [(&str, &str); 5] = [
        ("character", "''"),
        ("varchar", "''"),
        ("numeric", "0"),
        ("decimal", "0"),
        ("time", "now()"),
    ];
    if let Some((_, value)) = EXACT.iter().find(|(name, _)| t == *name) {
        return value;
    }
    PREFIXES
        .iter()
        .find(|(prefix, _)| t.starts_with(prefix))
        .map_or("NULL", |(_, value)| value)
}

fn select_sql(target: &str, columns: &[Column]) -> String {
    if columns.is_empty() {
        return format!("SELECT *\nFROM {target}\nLIMIT 100;");
    }
    let list: Vec<String> = columns
        .iter()
        .map(|c| format!("  {}", quote_ident(&c.name)))
        .collect();
    format!("SELECT\n{}\nFROM {target}\nLIMIT 100;", list.join(",\n"))
}

/// Coluna que o usuário pode escrever: nem gerada nem identidade `ALWAYS`.
fn writable(c: &Column) -> bool {
    c.generated.is_none() && !c.identity.as_deref().is_some_and(|i| i == "ALWAYS")
}

/// O valor do `INSERT`/`UPDATE` de uma coluna: `DEFAULT` quando ela tem, exemplo pelo tipo senão.
fn value_for(c: &Column) -> &'static str {
    if c.default.is_some() || c.identity.is_some() {
        "DEFAULT"
    } else {
        example_value(&c.data_type)
    }
}

fn insert_sql(target: &str, columns: &[Column]) -> String {
    let writable: Vec<&Column> = columns.iter().filter(|c| writable(c)).collect();
    if writable.is_empty() {
        return format!("INSERT INTO {target} DEFAULT VALUES;");
    }
    let names: Vec<String> = writable.iter().map(|c| quote_ident(&c.name)).collect();
    let values: Vec<&str> = writable.iter().map(|c| value_for(c)).collect();
    format!(
        "INSERT INTO {target} ({})\nVALUES ({});",
        names.join(", "),
        values.join(", ")
    )
}

/// O `WHERE` pela chave primária, ou pela primeira coluna com aviso quando não há chave; sem
/// coluna nenhuma, `false` (não há como apontar uma linha). O aviso é um comentário na linha de
/// cima do `WHERE`, dentro do statement: na primeira linha do texto ele ficaria fora do range do
/// `split`, e o Run Current com o cursor nele rodaria o statement anterior do buffer.
fn key_where(columns: &[Column]) -> String {
    let keys: Vec<&Column> = columns.iter().filter(|c| c.is_primary_key).collect();
    if !keys.is_empty() {
        return where_of(&keys);
    }
    match columns.first() {
        Some(first) => format!(
            "-- sem chave primária: confira o WHERE\n{}",
            where_of(&[first])
        ),
        None => "-- tabela sem colunas: confira o WHERE\nWHERE false".to_owned(),
    }
}

fn where_of(columns: &[&Column]) -> String {
    let terms: Vec<String> = columns
        .iter()
        .map(|c| format!("{} = {}", quote_ident(&c.name), example_value(&c.data_type)))
        .collect();
    format!("WHERE {}", terms.join(" AND "))
}

fn update_sql(target: &str, columns: &[Column]) -> String {
    let where_clause = key_where(columns);
    // As colunas graváveis fora da chave; se só há chave (tabela de ligação), a própria chave; se
    // nada é gravável (só geradas e identidade ALWAYS), todas, com aviso: o usuário decide.
    let mut set: Vec<&Column> = columns
        .iter()
        .filter(|c| writable(c) && !c.is_primary_key)
        .collect();
    let mut warning = "";
    if set.is_empty() {
        set = columns.iter().filter(|c| writable(c)).collect();
    }
    if set.is_empty() {
        set = columns.iter().collect();
        warning = "-- nenhuma coluna gravável (geradas ou identidade ALWAYS): confira o SET\n";
    }
    if set.is_empty() {
        return format!(
            "UPDATE {target}\n-- tabela sem colunas: confira o SET\nSET\n{where_clause};"
        );
    }
    let assignments: Vec<String> = set
        .iter()
        .map(|c| format!("{} = {}", quote_ident(&c.name), value_for(c)))
        .collect();
    format!(
        "UPDATE {target}\n{warning}SET {}\n{where_clause};",
        assignments.join(",\n    ")
    )
}

fn delete_sql(target: &str, columns: &[Column]) -> String {
    format!("DELETE FROM {target}\n{};", key_where(columns))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(kind: TableKind) -> Table {
        Table {
            schema: "public".into(),
            name: "Clientes".into(),
            kind,
            comment: None,
        }
    }

    fn col(name: &str, data_type: &str) -> Column {
        Column {
            name: name.into(),
            data_type: data_type.into(),
            nullable: true,
            position: 1,
            ..Column::default()
        }
    }

    fn columns() -> Vec<Column> {
        vec![
            Column {
                is_primary_key: true,
                identity: Some("ALWAYS".into()),
                ..col("id", "integer")
            },
            col("nome", "character varying(80)"),
            Column {
                default: Some("now()".into()),
                ..col("criado em", "timestamp with time zone")
            },
            Column {
                generated: Some("lower(nome)".into()),
                ..col("nome_lower", "text")
            },
            col("ativo", "boolean"),
            col("saldo", "numeric(10,2)"),
            col("ref", "uuid"),
            col("tags", "text[]"),
            col("ponto", "point"),
        ]
    }

    #[test]
    fn select_count_truncate_e_drop_pelo_tipo_do_objeto() {
        let t = table(TableKind::Table);
        assert_eq!(
            template_sql(Template::Select, &t, &columns()[..2]),
            "SELECT\n  \"id\",\n  \"nome\"\nFROM \"public\".\"Clientes\"\nLIMIT 100;"
        );
        assert_eq!(
            template_sql(Template::Select, &t, &[]),
            "SELECT *\nFROM \"public\".\"Clientes\"\nLIMIT 100;"
        );
        assert_eq!(
            template_sql(Template::Count, &t, &[]),
            "SELECT count(*) FROM \"public\".\"Clientes\";"
        );
        assert_eq!(
            template_sql(Template::Truncate, &t, &[]),
            "TRUNCATE TABLE \"public\".\"Clientes\";"
        );
        assert_eq!(
            template_sql(Template::Drop, &t, &[]),
            "DROP TABLE \"public\".\"Clientes\" CASCADE;"
        );
        assert_eq!(
            template_sql(Template::Drop, &table(TableKind::View), &[]),
            "DROP VIEW \"public\".\"Clientes\" CASCADE;"
        );
        assert_eq!(
            template_sql(Template::Drop, &table(TableKind::MaterializedView), &[]),
            "DROP MATERIALIZED VIEW \"public\".\"Clientes\" CASCADE;"
        );
        assert_eq!(
            template_sql(Template::Drop, &table(TableKind::Foreign), &[]),
            "DROP FOREIGN TABLE \"public\".\"Clientes\" CASCADE;"
        );
    }

    #[test]
    fn insert_pula_identidade_always_e_gerada_e_poe_default_e_exemplo_pelo_tipo() {
        let sql = template_sql(Template::Insert, &table(TableKind::Table), &columns());
        assert_eq!(
            sql,
            "INSERT INTO \"public\".\"Clientes\" (\"nome\", \"criado em\", \"ativo\", \"saldo\", \
             \"ref\", \"tags\", \"ponto\")\nVALUES ('', DEFAULT, false, 0, gen_random_uuid(), '{}', NULL);"
        );
        // Identidade BY DEFAULT entra como DEFAULT; só colunas não graváveis → DEFAULT VALUES.
        let by_default = [Column {
            identity: Some("BY DEFAULT".into()),
            ..col("id", "bigint")
        }];
        assert_eq!(
            template_sql(Template::Insert, &table(TableKind::Table), &by_default),
            "INSERT INTO \"public\".\"Clientes\" (\"id\")\nVALUES (DEFAULT);"
        );
        let only_generated = [Column {
            generated: Some("1".into()),
            ..col("x", "integer")
        }];
        assert_eq!(
            template_sql(Template::Insert, &table(TableKind::Table), &only_generated),
            "INSERT INTO \"public\".\"Clientes\" DEFAULT VALUES;"
        );
    }

    #[test]
    fn update_e_delete_pela_chave_e_aviso_sem_chave() {
        let t = table(TableKind::Table);
        assert_eq!(
            template_sql(Template::Update, &t, &columns()),
            "UPDATE \"public\".\"Clientes\"\nSET \"nome\" = '',\n    \"criado em\" = DEFAULT,\n    \
             \"ativo\" = false,\n    \"saldo\" = 0,\n    \"ref\" = gen_random_uuid(),\n    \
             \"tags\" = '{}',\n    \"ponto\" = NULL\nWHERE \"id\" = 0;"
        );
        assert_eq!(
            template_sql(Template::Delete, &t, &columns()),
            "DELETE FROM \"public\".\"Clientes\"\nWHERE \"id\" = 0;"
        );
        // Chave composta.
        let link = [
            Column {
                is_primary_key: true,
                ..col("cliente_id", "integer")
            },
            Column {
                is_primary_key: true,
                ..col("tag", "text")
            },
        ];
        assert_eq!(
            template_sql(Template::Delete, &t, &link),
            "DELETE FROM \"public\".\"Clientes\"\nWHERE \"cliente_id\" = 0 AND \"tag\" = '';"
        );
        // Só chave: o SET usa a própria chave.
        assert_eq!(
            template_sql(Template::Update, &t, &link),
            "UPDATE \"public\".\"Clientes\"\nSET \"cliente_id\" = 0,\n    \"tag\" = ''\n\
             WHERE \"cliente_id\" = 0 AND \"tag\" = '';"
        );
        // Sem chave: primeira coluna e aviso dentro do statement (nunca na primeira linha).
        let no_key = [col("nome", "text"), col("ativo", "boolean")];
        assert_eq!(
            template_sql(Template::Delete, &t, &no_key),
            "DELETE FROM \"public\".\"Clientes\"\n-- sem chave primária: confira o WHERE\nWHERE \"nome\" = '';"
        );
        assert_eq!(
            template_sql(Template::Update, &t, &no_key),
            "UPDATE \"public\".\"Clientes\"\nSET \"nome\" = '',\n    \"ativo\" = false\n\
             -- sem chave primária: confira o WHERE\nWHERE \"nome\" = '';"
        );
        // Sem coluna nenhuma: WHERE false; só geradas: SET com aviso.
        assert_eq!(
            template_sql(Template::Delete, &t, &[]),
            "DELETE FROM \"public\".\"Clientes\"\n-- tabela sem colunas: confira o WHERE\nWHERE false;"
        );
        assert_eq!(
            template_sql(Template::Update, &t, &[]),
            "UPDATE \"public\".\"Clientes\"\n-- tabela sem colunas: confira o SET\nSET\n\
             -- tabela sem colunas: confira o WHERE\nWHERE false;"
        );
        let only_generated = [Column {
            generated: Some("1".into()),
            is_primary_key: true,
            ..col("x", "integer")
        }];
        assert_eq!(
            template_sql(Template::Update, &t, &only_generated),
            "UPDATE \"public\".\"Clientes\"\n-- nenhuma coluna gravável (geradas ou identidade ALWAYS): \
             confira o SET\nSET \"x\" = 0\nWHERE \"x\" = 0;"
        );
        for sql in [
            template_sql(Template::Update, &t, &no_key),
            template_sql(Template::Delete, &t, &[]),
        ] {
            assert!(!sql.starts_with("--"), "{sql}");
        }
    }

    #[test]
    fn exemplo_pelo_tipo_aspas_dobradas_e_modelos_por_tipo_de_objeto() {
        assert_eq!(example_value("character varying(80)"), "''");
        assert_eq!(example_value("TIMESTAMP WITHOUT TIME ZONE"), "now()");
        assert_eq!(example_value("double precision"), "0");
        assert_eq!(example_value("integer[]"), "'{}'");
        assert_eq!(example_value("jsonb"), "'{}'");
        assert_eq!(example_value("bytea"), "'\\x'");
        assert_eq!(example_value("tsvector"), "NULL");
        assert_eq!(example_value("time with time zone"), "now()");
        assert_eq!(example_value("date"), "now()");
        assert_eq!(example_value("daterange"), "NULL");
        assert_eq!(example_value("datemultirange"), "NULL");
        assert_eq!(example_value("jsonpath"), "NULL");
        assert_eq!(example_value("interval"), "NULL");
        assert_eq!(example_value("\"char\""), "NULL");
        let weird = Table {
            schema: "s\"q".into(),
            name: "t\"n".into(),
            kind: TableKind::Table,
            comment: None,
        };
        assert_eq!(
            template_sql(Template::Count, &weird, &[]),
            "SELECT count(*) FROM \"s\"\"q\".\"t\"\"n\";"
        );
        for kind in [TableKind::View, TableKind::MaterializedView] {
            let allowed: Vec<Template> = Template::ALL
                .into_iter()
                .filter(|t| t.applies_to(kind))
                .collect();
            assert_eq!(allowed, [Template::Select, Template::Count, Template::Drop]);
        }
        assert!(Template::ALL.iter().all(|t| t.applies_to(TableKind::Table)));
        assert!(
            Template::ALL
                .iter()
                .all(|t| t.applies_to(TableKind::Foreign))
        );
        assert_eq!(
            Template::ALL.iter().filter(|t| t.needs_columns()).count(),
            4
        );
    }
}
