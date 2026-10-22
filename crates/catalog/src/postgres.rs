//! Carregadores sobre `pg_catalog`. Uma query cada; nomes de schema e tabela entram como literal
//! escapado (o protocolo simples não tem parâmetros).

use hy_dbplus_driver::{Connection, DriverError, TextRows};

use crate::model::{Column, Function, Schema, Table, TableKind};

/// Literal SQL válido nos dois modos de `standard_conforming_strings`: sempre `E'...'`, com
/// `\` e `'` dobrados. Os nomes vêm do próprio catálogo, e um nome com aspa ou barra é um
/// nome válido, não um erro.
fn literal(value: &str) -> String {
    format!("E'{}'", value.replace('\\', "\\\\").replace('\'', "''"))
}

/// A célula como texto ou vazio: quem lê decide o que fazer com o NULL.
fn text(rows: &TextRows, row: usize, col: usize) -> String {
    rows.cell(row, col).unwrap_or_default().to_owned()
}

/// Schemas do banco, sem os internos (`pg_*`, `information_schema`), em ordem alfabética. Sem
/// `LIKE`: o escape do `_` dependeria de `standard_conforming_strings`.
pub async fn schemas(conn: &impl Connection) -> Result<Vec<Schema>, DriverError> {
    let rows = conn
        .query_text(
            "SELECT nspname FROM pg_catalog.pg_namespace \
             WHERE left(nspname, 3) <> 'pg_' AND nspname <> 'information_schema' ORDER BY nspname",
        )
        .await?;
    Ok((0..rows.rows.len())
        .map(|i| Schema {
            name: text(&rows, i, 0),
        })
        .collect())
}

/// Tabelas, views, views materializadas, particionadas e estrangeiras de um schema, com comentário.
pub async fn tables(conn: &impl Connection, schema: &str) -> Result<Vec<Table>, DriverError> {
    let sql = format!(
        "SELECT c.relname, c.relkind, pg_catalog.obj_description(c.oid, 'pg_class') \
         FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         WHERE n.nspname = {} AND c.relkind IN ('r', 'p', 'v', 'm', 'f') ORDER BY c.relname",
        literal(schema)
    );
    let rows = conn.query_text(&sql).await?;
    Ok((0..rows.rows.len())
        .map(|i| Table {
            schema: schema.to_owned(),
            name: text(&rows, i, 0),
            kind: match rows.cell(i, 1) {
                Some("v") => TableKind::View,
                Some("m") => TableKind::MaterializedView,
                Some("f") => TableKind::Foreign,
                _ => TableKind::Table,
            },
            comment: rows.cell(i, 2).map(str::to_owned),
        })
        .collect())
}

/// Colunas de uma tabela ou view, na ordem da tabela, com tipo legível, default e chave primária.
pub async fn columns(
    conn: &impl Connection,
    schema: &str,
    table: &str,
) -> Result<Vec<Column>, DriverError> {
    let sql = format!(
        "SELECT a.attname, pg_catalog.format_type(a.atttypid, a.atttypmod), NOT a.attnotnull, \
                pg_catalog.pg_get_expr(d.adbin, d.adrelid), a.attnum, \
                EXISTS (SELECT 1 FROM pg_catalog.pg_index i \
                        WHERE i.indrelid = a.attrelid AND i.indisprimary AND a.attnum = ANY (i.indkey)) \
         FROM pg_catalog.pg_attribute a \
         JOIN pg_catalog.pg_class c ON c.oid = a.attrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         LEFT JOIN pg_catalog.pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
         WHERE n.nspname = {} AND c.relname = {} AND a.attnum > 0 AND NOT a.attisdropped \
         ORDER BY a.attnum",
        literal(schema),
        literal(table)
    );
    let rows = conn.query_text(&sql).await?;
    Ok((0..rows.rows.len())
        .map(|i| Column {
            name: text(&rows, i, 0),
            data_type: text(&rows, i, 1),
            nullable: rows.cell(i, 2) == Some("t"),
            default: rows.cell(i, 3).map(str::to_owned),
            position: rows.cell(i, 4).and_then(|v| v.parse().ok()).unwrap_or(0),
            is_primary_key: rows.cell(i, 5) == Some("t"),
        })
        .collect())
}

/// Funções, procedures, agregados e funções de janela de um schema.
pub async fn functions(conn: &impl Connection, schema: &str) -> Result<Vec<Function>, DriverError> {
    let sql = format!(
        "SELECT p.proname, pg_catalog.pg_get_function_identity_arguments(p.oid), \
                pg_catalog.pg_get_function_result(p.oid), p.prokind \
         FROM pg_catalog.pg_proc p JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace \
         WHERE n.nspname = {} ORDER BY p.proname, 2",
        literal(schema)
    );
    let rows = conn.query_text(&sql).await?;
    Ok((0..rows.rows.len())
        .map(|i| Function {
            schema: schema.to_owned(),
            name: text(&rows, i, 0),
            arguments: text(&rows, i, 1),
            returns: text(&rows, i, 2),
            kind: match rows.cell(i, 3) {
                Some("p") => "procedure",
                Some("a") => "aggregate",
                Some("w") => "window",
                _ => "function",
            }
            .to_owned(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_literal_dobra_aspas_e_barras_e_vale_nos_dois_modos() {
        assert_eq!(literal("public"), "E'public'");
        assert_eq!(literal("o'reilly"), "E'o''reilly'");
        assert_eq!(literal("a'; drop table x; --"), "E'a''; drop table x; --'");
        // Com standard_conforming_strings = off, `\'` fecharia o literal; dobrada, não.
        assert_eq!(literal("\\'; SELECT 1; --"), "E'\\\\''; SELECT 1; --'");
        assert_eq!(literal("c:\\temp"), "E'c:\\\\temp'");
    }

    /// Contra o PostgreSQL local (`HY_DBPLUS_TEST_PG`): cria um schema próprio, lê tudo, apaga o
    /// schema e só então confere (uma asserção falha não deixa o schema para trás).
    mod with_server {
        use super::*;
        use hy_dbplus_driver::postgres::PostgresDriver;
        use hy_dbplus_driver::{ConnectionConfig, Driver, Secret, TlsMode};

        // Igual ao helper dos testes do driver: a URL de teste é uma só; mudar lá e aqui.
        async fn connect() -> Option<impl Connection> {
            let url = std::env::var_os("HY_DBPLUS_TEST_PG")?;
            let pg: tokio_postgres::Config = url
                .to_string_lossy()
                .parse()
                .expect("HY_DBPLUS_TEST_PG inválida");
            let host = match pg.get_hosts().first() {
                Some(tokio_postgres::config::Host::Tcp(h)) => h.clone(),
                _ => "localhost".into(),
            };
            let cfg = ConnectionConfig {
                host,
                port: pg.get_ports().first().copied().unwrap_or(5432),
                user: pg.get_user().unwrap_or("postgres").into(),
                database: pg.get_dbname().unwrap_or("postgres").into(),
                tls: TlsMode::Prefer,
            };
            let secret = Secret::new(String::from_utf8_lossy(
                pg.get_password().unwrap_or_default(),
            ));
            Some(
                PostgresDriver::default()
                    .connect(&cfg, &secret)
                    .await
                    .unwrap(),
            )
        }

        fn schema_name() -> String {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0);
            format!("hy_dbplus_test_{}_{nanos}", std::process::id())
        }

        struct Loaded {
            schemas: Vec<Schema>,
            tables: Vec<Table>,
            columns: Vec<Column>,
            functions: Vec<Function>,
            missing: Vec<Column>,
        }

        async fn load_all(conn: &impl Connection, schema: &str) -> Result<Loaded, DriverError> {
            Ok(Loaded {
                schemas: schemas(conn).await?,
                tables: tables(conn, schema).await?,
                columns: columns(conn, schema, "pedido").await?,
                functions: functions(conn, schema).await?,
                missing: columns(conn, schema, "nao_existe").await?,
            })
        }

        #[tokio::test]
        async fn le_schemas_tabelas_views_colunas_e_funcoes() {
            let Some(conn) = connect().await else { return };
            let schema = schema_name();
            let setup = format!(
                "CREATE SCHEMA {schema}; \
                 CREATE TABLE {schema}.pedido (id serial PRIMARY KEY, cliente varchar(80) NOT NULL, \
                                               total numeric(10,2) DEFAULT 0, obs text); \
                 COMMENT ON TABLE {schema}.pedido IS 'pedidos de teste'; \
                 CREATE VIEW {schema}.pedido_resumo AS SELECT id, total FROM {schema}.pedido; \
                 CREATE MATERIALIZED VIEW {schema}.pedido_total AS SELECT sum(total) AS total FROM {schema}.pedido; \
                 CREATE FUNCTION {schema}.dobro(x integer) RETURNS integer LANGUAGE sql AS 'select x * 2'; \
                 CREATE PROCEDURE {schema}.limpar() LANGUAGE sql AS 'delete from {schema}.pedido';"
            );
            conn.query_text(&setup).await.unwrap();
            let loaded = load_all(&conn, &schema).await;
            conn.query_text(&format!("DROP SCHEMA {schema} CASCADE"))
                .await
                .unwrap();
            let loaded = loaded.unwrap();

            assert!(
                loaded.schemas.iter().any(|s| s.name == schema),
                "{:?}",
                loaded.schemas
            );
            assert!(
                loaded
                    .schemas
                    .iter()
                    .all(|s| !s.name.starts_with("pg_") && s.name != "information_schema")
            );

            let names: Vec<(&str, TableKind)> = loaded
                .tables
                .iter()
                .map(|t| (t.name.as_str(), t.kind))
                .collect();
            assert_eq!(
                names,
                [
                    ("pedido", TableKind::Table),
                    ("pedido_resumo", TableKind::View),
                    ("pedido_total", TableKind::MaterializedView)
                ]
            );
            assert_eq!(
                loaded.tables[0].comment.as_deref(),
                Some("pedidos de teste")
            );
            assert!(loaded.tables[1].kind.is_view() && loaded.tables[2].kind.is_view());

            let view: Vec<(&str, &str, bool, bool)> = loaded
                .columns
                .iter()
                .map(|c| {
                    (
                        c.name.as_str(),
                        c.data_type.as_str(),
                        c.nullable,
                        c.is_primary_key,
                    )
                })
                .collect();
            assert_eq!(
                view,
                [
                    ("id", "integer", false, true),
                    ("cliente", "character varying(80)", false, false),
                    ("total", "numeric(10,2)", true, false),
                    ("obs", "text", true, false),
                ]
            );
            assert_eq!(loaded.columns[2].default.as_deref(), Some("0"));
            assert_eq!(
                loaded
                    .columns
                    .iter()
                    .map(|c| c.position)
                    .collect::<Vec<_>>(),
                [1, 2, 3, 4]
            );

            let funcs: Vec<(String, &str, &str)> = loaded
                .functions
                .iter()
                .map(|f| (f.signature(), f.returns.as_str(), f.kind.as_str()))
                .collect();
            assert_eq!(
                funcs,
                [
                    ("dobro(x integer)".to_owned(), "integer", "function"),
                    ("limpar()".to_owned(), "", "procedure")
                ]
            );

            assert!(loaded.missing.is_empty());
        }
    }
}
