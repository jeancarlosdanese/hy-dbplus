//! Os dados de uma tabela para a aba Data (tela 3.4): tamanhos, contagem e comentário, e o SQL
//! da página (`LIMIT/OFFSET`, BRIEF decisão 3) com ordenação por uma lista de colunas.
//!
//! Identificadores entram sempre entre aspas duplas, com as aspas internas dobradas; `limit` e
//! `offset` são inteiros formatados pelo Rust. Nada cru chega ao SQL.

use hy_dbplus_driver::{Connection, DriverError, TextRows};

use crate::postgres::{literal, text};

/// Tamanhos legíveis (`pg_size_pretty`), contagem exata e comentário de uma tabela.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TableStats {
    pub total_size: String,
    pub data_size: String,
    pub index_size: String,
    pub row_count: u64,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Asc,
    Desc,
}

impl Direction {
    pub fn sql(self) -> &'static str {
        match self {
            Direction::Asc => "ASC",
            Direction::Desc => "DESC",
        }
    }
}

/// Ordenação por uma coluna.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub column: String,
    pub direction: Direction,
}

/// Identificador SQL: aspas duplas, com as internas dobradas.
pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// `"schema"."tabela"`.
pub fn qualified(schema: &str, table: &str) -> String {
    format!("{}.{}", quote_ident(schema), quote_ident(table))
}

/// `SELECT * FROM "schema"."tabela" [ORDER BY "col" ASC|DESC, ...] LIMIT n OFFSET m`. A lista de
/// ordenação leva a coluna escolhida e, depois dela, o desempate (a chave primária, quando o engine
/// a conhece): sem desempate, valores repetidos embaralham as páginas.
pub fn page_sql(schema: &str, table: &str, order: &[Order], limit: u32, offset: u64) -> String {
    let mut sql = format!("SELECT * FROM {}", qualified(schema, table));
    if !order.is_empty() {
        let terms: Vec<String> = order
            .iter()
            .map(|o| format!("{} {}", quote_ident(&o.column), o.direction.sql()))
            .collect();
        sql.push_str(&format!(" ORDER BY {}", terms.join(", ")));
    }
    sql.push_str(&format!(" LIMIT {limit} OFFSET {offset}"));
    sql
}

/// `SELECT count(*) FROM "schema"."tabela"`.
pub fn count_sql(schema: &str, table: &str) -> String {
    format!("SELECT count(*) FROM {}", qualified(schema, table))
}

/// Uma página de linhas, como texto.
pub async fn page(
    conn: &impl Connection,
    schema: &str,
    table: &str,
    order: &[Order],
    limit: u32,
    offset: u64,
) -> Result<TextRows, DriverError> {
    conn.query_text(&page_sql(schema, table, order, limit, offset))
        .await
}

/// Contagem exata de linhas. Resposta sem número é erro, não zero.
pub async fn count(conn: &impl Connection, schema: &str, table: &str) -> Result<u64, DriverError> {
    let rows = conn.query_text(&count_sql(schema, table)).await?;
    rows.cell(0, 0).and_then(|v| v.parse().ok()).ok_or_else(|| {
        DriverError::Db(format!(
            "count(*) de {} sem resultado",
            qualified(schema, table)
        ))
    })
}

/// Tamanhos, contagem e comentário. Duas queries: `pg_class` para tamanhos e comentário,
/// `count(*)` para a contagem (tabela inexistente falha no `count`).
pub async fn stats(
    conn: &impl Connection,
    schema: &str,
    table: &str,
) -> Result<TableStats, DriverError> {
    let sql = format!(
        "SELECT pg_catalog.pg_size_pretty(pg_catalog.pg_total_relation_size(c.oid)), \
                pg_catalog.pg_size_pretty(pg_catalog.pg_table_size(c.oid)), \
                pg_catalog.pg_size_pretty(pg_catalog.pg_indexes_size(c.oid)), \
                pg_catalog.obj_description(c.oid, 'pg_class') \
         FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         WHERE n.nspname = {} AND c.relname = {}",
        literal(schema),
        literal(table)
    );
    let rows = conn.query_text(&sql).await?;
    let row_count = count(conn, schema, table).await?;
    Ok(TableStats {
        total_size: text(&rows, 0, 0),
        data_size: text(&rows, 0, 1),
        index_size: text(&rows, 0, 2),
        row_count,
        comment: rows.cell(0, 3).map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_sql_da_pagina_tem_ordem_com_desempate_limite_e_offset() {
        let order = [
            Order {
                column: "total".into(),
                direction: Direction::Desc,
            },
            Order {
                column: "id".into(),
                direction: Direction::Asc,
            },
        ];
        assert_eq!(
            page_sql("public", "pedido", &order, 100, 200),
            "SELECT * FROM \"public\".\"pedido\" ORDER BY \"total\" DESC, \"id\" ASC LIMIT 100 OFFSET 200"
        );
        assert_eq!(
            page_sql("public", "pedido", &[], 50, 0),
            "SELECT * FROM \"public\".\"pedido\" LIMIT 50 OFFSET 0"
        );
        assert_eq!(count_sql("app", "x"), "SELECT count(*) FROM \"app\".\"x\"");
    }

    #[test]
    fn identificadores_com_aspas_saem_dobradas() {
        assert_eq!(quote_ident("a\"b"), "\"a\"\"b\"");
        assert_eq!(qualified("s\"1", "t; drop"), "\"s\"\"1\".\"t; drop\"");
        let order = [Order {
            column: "x\"; drop".into(),
            direction: Direction::Asc,
        }];
        assert!(page_sql("s", "t", &order, 1, 0).contains("ORDER BY \"x\"\"; drop\" ASC"));
    }

    mod with_server {
        use super::*;
        use crate::postgres::test_support::{connect, schema_name};

        #[tokio::test]
        async fn stats_e_pagina_de_uma_tabela_de_teste() {
            let Some(conn) = connect().await else { return };
            let schema = schema_name();
            let setup = format!(
                "CREATE SCHEMA {schema}; \
                 CREATE TABLE {schema}.item (id serial PRIMARY KEY, nome text, preco numeric(10,2)); \
                 COMMENT ON TABLE {schema}.item IS 'itens de teste'; \
                 INSERT INTO {schema}.item (nome, preco) VALUES ('b', 2), ('a', 1), ('c', NULL);"
            );
            conn.query_text(&setup).await.unwrap();
            let result = async {
                let table_stats = stats(&conn, &schema, "item").await?;
                let order = [Order {
                    column: "nome".into(),
                    direction: Direction::Desc,
                }];
                let first = page(&conn, &schema, "item", &order, 2, 0).await?;
                let second = page(&conn, &schema, "item", &order, 2, 2).await?;
                let total = count(&conn, &schema, "item").await?;
                let missing = stats(&conn, &schema, "nao_existe").await;
                Ok::<_, DriverError>((table_stats, first, second, total, missing))
            }
            .await;
            conn.query_text(&format!("DROP SCHEMA {schema} CASCADE"))
                .await
                .unwrap();
            let (stats, first, second, total, missing) = result.unwrap();
            assert_eq!(stats.row_count, 3);
            assert_eq!(stats.comment.as_deref(), Some("itens de teste"));
            assert!(
                stats.total_size.ends_with("bytes") || stats.total_size.ends_with("kB"),
                "{stats:?}"
            );
            assert!(!stats.data_size.is_empty() && !stats.index_size.is_empty());
            assert_eq!(first.columns, ["id", "nome", "preco"]);
            assert_eq!(
                first.rows.iter().map(|r| r[1].clone()).collect::<Vec<_>>(),
                [Some("c".into()), Some("b".into())]
            );
            assert_eq!(first.cell(0, 2), None, "preco NULL");
            assert_eq!(second.rows.len(), 1);
            assert_eq!(second.cell(0, 1), Some("a"));
            assert_eq!(total, 3);
            // Só a categoria: a mensagem depende do lc_messages do servidor.
            assert!(matches!(missing, Err(DriverError::Db(_))), "{missing:?}");
        }
    }
}
