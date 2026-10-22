//! O DDL de uma tabela ou view a partir de [`TableStructure`] (FEAT-009). O PostgreSQL não tem
//! `pg_get_tabledef`: o `CREATE TABLE` é montado aqui com o que o servidor escreve para cada
//! parte (`format_type`, `pg_get_expr`, `pg_get_constraintdef`, `pg_get_indexdef`,
//! `pg_get_viewdef`). Identificadores por `quote_ident`, textos de comentário por `literal`.
//!
//! Fora: `PARTITION BY`, herança, `SERVER`/`OPTIONS` de tabela estrangeira, collation de coluna,
//! sequências e tipos próprios (só referenciados pelo nome). Particionada e estrangeira saem com um
//! aviso em comentário no topo.

use crate::data::{qualified, quote_ident};
use crate::model::{TableKind, TableStructure};
use crate::postgres::literal;

/// `CREATE TABLE` (ou `CREATE VIEW`), índices que não implementam restrição e os comentários.
pub fn table_ddl(structure: &TableStructure) -> String {
    let table = &structure.table;
    let target = qualified(&table.schema, &table.name);
    let mut out = String::new();
    match (table.kind, &structure.view_definition) {
        (TableKind::View, Some(def)) => {
            out.push_str(&format!("CREATE VIEW {target} AS\n{def};\n"));
        }
        (TableKind::MaterializedView, Some(def)) => {
            out.push_str(&format!("CREATE MATERIALIZED VIEW {target} AS\n{def};\n"));
        }
        _ => {
            if structure.partitioned {
                out.push_str("-- tabela particionada: PARTITION BY não incluído\n");
            }
            if table.kind == TableKind::Foreign {
                out.push_str("-- tabela estrangeira: SERVER e OPTIONS não incluídos\n");
            }
            let mut lines: Vec<String> = structure
                .columns
                .iter()
                .map(|c| {
                    let mut line = format!("    {} {}", quote_ident(&c.name), c.data_type);
                    if !c.nullable {
                        line.push_str(" NOT NULL");
                    }
                    if let Some(expr) = &c.generated {
                        let mode = if c.generated_virtual {
                            "VIRTUAL"
                        } else {
                            "STORED"
                        };
                        line.push_str(&format!(" GENERATED ALWAYS AS ({expr}) {mode}"));
                    } else if let Some(identity) = &c.identity {
                        line.push_str(&format!(" GENERATED {identity} AS IDENTITY"));
                    } else if let Some(default) = &c.default {
                        line.push_str(&format!(" DEFAULT {default}"));
                    }
                    line
                })
                .collect();
            for con in &structure.constraints {
                lines.push(format!(
                    "    CONSTRAINT {} {}",
                    quote_ident(&con.name),
                    con.definition
                ));
            }
            out.push_str(&format!(
                "CREATE TABLE {target} (\n{}\n);\n",
                lines.join(",\n")
            ));
        }
    }
    for index in structure.indexes.iter().filter(|i| !i.is_constraint) {
        out.push_str(&format!("{};\n", index.definition));
    }
    if let Some(comment) = &table.comment {
        let what = match table.kind {
            TableKind::View => "VIEW",
            TableKind::MaterializedView => "MATERIALIZED VIEW",
            _ => "TABLE",
        };
        out.push_str(&format!(
            "COMMENT ON {what} {target} IS {};\n",
            literal(comment)
        ));
    }
    for c in &structure.columns {
        if let Some(comment) = &c.comment {
            out.push_str(&format!(
                "COMMENT ON COLUMN {target}.{} IS {};\n",
                quote_ident(&c.name),
                literal(comment)
            ));
        }
    }
    for index in &structure.indexes {
        if let Some(comment) = &index.comment {
            out.push_str(&format!(
                "COMMENT ON INDEX {} IS {};\n",
                qualified(&table.schema, &index.name),
                literal(comment)
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Column, Constraint, ConstraintKind, Index, Reference, Table};

    fn table(kind: TableKind) -> Table {
        Table {
            schema: "app".into(),
            name: "Pedido".into(),
            kind,
            comment: None,
        }
    }

    #[test]
    fn create_table_com_colunas_restricoes_indices_e_comentarios() {
        let structure = TableStructure {
            table: Table {
                comment: Some("pedidos d'água".into()),
                ..table(TableKind::Table)
            },
            columns: vec![
                Column {
                    name: "id".into(),
                    data_type: "bigint".into(),
                    nullable: false,
                    identity: Some("ALWAYS".into()),
                    is_primary_key: true,
                    position: 1,
                    ..Default::default()
                },
                Column {
                    name: "cliente id".into(),
                    data_type: "integer".into(),
                    nullable: false,
                    position: 2,
                    ..Default::default()
                },
                Column {
                    name: "obs".into(),
                    data_type: "text".into(),
                    nullable: true,
                    default: Some("'s/ obs'::text".into()),
                    comment: Some("livre".into()),
                    position: 3,
                    ..Default::default()
                },
                Column {
                    name: "total".into(),
                    data_type: "numeric(10,2)".into(),
                    nullable: true,
                    generated: Some("(qtd * preco)".into()),
                    position: 4,
                    ..Default::default()
                },
            ],
            constraints: vec![
                Constraint {
                    name: "pedido_pkey".into(),
                    kind: ConstraintKind::PrimaryKey,
                    columns: vec!["id".into()],
                    definition: "PRIMARY KEY (id)".into(),
                    references: None,
                },
                Constraint {
                    name: "pedido_cliente_fkey".into(),
                    kind: ConstraintKind::ForeignKey,
                    columns: vec!["cliente id".into()],
                    definition: "FOREIGN KEY (\"cliente id\") REFERENCES app.cliente(id) ON DELETE CASCADE"
                        .into(),
                    references: Some(Reference {
                        schema: "app".into(),
                        table: "cliente".into(),
                        columns: vec!["id".into()],
                        on_update: "no action".into(),
                        on_delete: "cascade".into(),
                    }),
                },
            ],
            indexes: vec![
                Index {
                    name: "pedido_pkey".into(),
                    algorithm: "btree".into(),
                    is_unique: true,
                    is_primary: true,
                    is_constraint: true,
                    columns: vec!["id".into()],
                    condition: None,
                    include: vec![],
                    storage_parameters: vec![],
                    comment: None,
                    definition: "CREATE UNIQUE INDEX pedido_pkey ON app.\"Pedido\" USING btree (id)".into(),
                },
                Index {
                    name: "pedido_obs_idx".into(),
                    algorithm: "btree".into(),
                    is_unique: false,
                    is_primary: false,
                    is_constraint: false,
                    columns: vec!["obs".into()],
                    condition: Some("(obs IS NOT NULL)".into()),
                    include: vec!["total".into()],
                    storage_parameters: vec!["fillfactor=70".into()],
                    comment: Some("parcial".into()),
                    definition: "CREATE INDEX pedido_obs_idx ON app.\"Pedido\" USING btree (obs) INCLUDE (total) WITH (fillfactor='70') WHERE (obs IS NOT NULL)".into(),
                },
            ],
            view_definition: None,
            partitioned: false,
        };
        let ddl = table_ddl(&structure);
        assert_eq!(
            ddl,
            "CREATE TABLE \"app\".\"Pedido\" (\n\
             \x20   \"id\" bigint NOT NULL GENERATED ALWAYS AS IDENTITY,\n\
             \x20   \"cliente id\" integer NOT NULL,\n\
             \x20   \"obs\" text DEFAULT 's/ obs'::text,\n\
             \x20   \"total\" numeric(10,2) GENERATED ALWAYS AS ((qtd * preco)) STORED,\n\
             \x20   CONSTRAINT \"pedido_pkey\" PRIMARY KEY (id),\n\
             \x20   CONSTRAINT \"pedido_cliente_fkey\" FOREIGN KEY (\"cliente id\") REFERENCES app.cliente(id) ON DELETE CASCADE\n\
             );\n\
             CREATE INDEX pedido_obs_idx ON app.\"Pedido\" USING btree (obs) INCLUDE (total) WITH (fillfactor='70') WHERE (obs IS NOT NULL);\n\
             COMMENT ON TABLE \"app\".\"Pedido\" IS E'pedidos d''água';\n\
             COMMENT ON COLUMN \"app\".\"Pedido\".\"obs\" IS E'livre';\n\
             COMMENT ON INDEX \"app\".\"pedido_obs_idx\" IS E'parcial';\n"
        );
    }

    #[test]
    fn view_e_view_materializada_saem_com_a_definicao() {
        let view = TableStructure {
            table: table(TableKind::View),
            partitioned: false,
            columns: vec![],
            constraints: vec![],
            indexes: vec![],
            view_definition: Some(" SELECT id FROM app.pedido".into()),
        };
        assert_eq!(
            table_ddl(&view),
            "CREATE VIEW \"app\".\"Pedido\" AS\n SELECT id FROM app.pedido;\n"
        );
        let mat = TableStructure {
            table: table(TableKind::MaterializedView),
            ..view
        };
        assert!(table_ddl(&mat).starts_with("CREATE MATERIALIZED VIEW"));
    }

    #[test]
    fn particionada_estrangeira_e_gerada_virtual_saem_anotadas() {
        let mut t = TableStructure {
            table: table(TableKind::Table),
            partitioned: true,
            columns: vec![Column {
                name: "x".into(),
                data_type: "integer".into(),
                nullable: true,
                generated: Some("(y * 2)".into()),
                generated_virtual: true,
                ..Default::default()
            }],
            constraints: vec![],
            indexes: vec![],
            view_definition: None,
        };
        let ddl = table_ddl(&t);
        assert!(ddl.starts_with("-- tabela particionada: PARTITION BY não incluído\nCREATE TABLE"));
        assert!(ddl.contains("GENERATED ALWAYS AS ((y * 2)) VIRTUAL"));
        t.partitioned = false;
        t.table.kind = TableKind::Foreign;
        assert!(
            table_ddl(&t).starts_with("-- tabela estrangeira: SERVER e OPTIONS não incluídos\n")
        );
    }
}
