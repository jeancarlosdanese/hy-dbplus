//! A estrutura de uma tabela ou view para as abas Structure e Index e para o DDL (FEAT-009):
//! colunas (com comentário, identidade e expressão gerada), restrições de `pg_constraint`, índices
//! de `pg_index` e a definição da view. Quatro queries, nomes como literal escapado.
//!
//! Listas vêm do servidor como texto separado por `\x1f` (unit separator): nome de coluna pode
//! ter vírgula; `\x1f` não.

use hy_dbplus_driver::{Connection, DriverError};

use crate::model::{Constraint, ConstraintKind, Index, Reference, Table, TableStructure};
use crate::postgres::{columns, literal, table_kind, text};

/// O mesmo byte nos dois lados: `SEP` ao ler, `SEP_SQL` no `string_agg`.
const SEP: char = '\u{1f}';
const SEP_SQL: &str = "E'\\x1f'";

fn list(value: Option<&str>) -> Vec<String> {
    value
        .filter(|v| !v.is_empty())
        .map(|v| v.split(SEP).map(str::to_owned).collect())
        .unwrap_or_default()
}

fn action(code: Option<&str>) -> String {
    match code {
        Some("c") => "cascade",
        Some("n") => "set null",
        Some("d") => "set default",
        Some("r") => "restrict",
        _ => "no action",
    }
    .to_owned()
}

/// A tabela, as colunas, as restrições, os índices e, para view, a definição.
pub async fn structure(
    conn: &impl Connection,
    schema: &str,
    table: &str,
) -> Result<TableStructure, DriverError> {
    let (s, t) = (literal(schema), literal(table));
    let head = conn
        .query_text(&format!(
            "SELECT c.relkind, pg_catalog.obj_description(c.oid, 'pg_class'), \
                    CASE WHEN c.relkind IN ('v', 'm') THEN pg_catalog.pg_get_viewdef(c.oid, true) END \
             FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
             WHERE n.nspname = {s} AND c.relname = {t}"
        ))
        .await?;
    if head.rows.is_empty() {
        // Tabela apagada entre a árvore e o clique: erro, não uma estrutura vazia com DDL inválido.
        return Err(DriverError::Db(format!("{schema}.{table} não existe")));
    }
    let table_info = Table {
        schema: schema.to_owned(),
        name: table.to_owned(),
        kind: table_kind(head.cell(0, 0)),
        comment: head.cell(0, 1).map(str::to_owned),
    };
    let partitioned = head.cell(0, 0) == Some("p");
    let view_definition = head
        .cell(0, 2)
        .map(|v| v.trim().trim_end_matches(';').to_owned());

    let columns = columns(conn, schema, table).await?;

    let rows = conn
        .query_text(&format!(
            "SELECT con.conname, con.contype, pg_catalog.pg_get_constraintdef(con.oid, true), \
                    (SELECT string_agg(a.attname, {SEP_SQL} ORDER BY k.ord) \
                       FROM unnest(con.conkey) WITH ORDINALITY k(attnum, ord) \
                       JOIN pg_catalog.pg_attribute a ON a.attrelid = con.conrelid AND a.attnum = k.attnum), \
                    fn.nspname, fc.relname, \
                    (SELECT string_agg(a.attname, {SEP_SQL} ORDER BY k.ord) \
                       FROM unnest(con.confkey) WITH ORDINALITY k(attnum, ord) \
                       JOIN pg_catalog.pg_attribute a ON a.attrelid = con.confrelid AND a.attnum = k.attnum), \
                    con.confupdtype, con.confdeltype \
             FROM pg_catalog.pg_constraint con \
             JOIN pg_catalog.pg_class c ON c.oid = con.conrelid \
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
             LEFT JOIN pg_catalog.pg_class fc ON fc.oid = con.confrelid \
             LEFT JOIN pg_catalog.pg_namespace fn ON fn.oid = fc.relnamespace \
             WHERE n.nspname = {s} AND c.relname = {t} AND con.contype IN ('p', 'u', 'c', 'f', 'x') \
             ORDER BY con.contype, con.conname"
        ))
        .await?;
    let constraints = (0..rows.rows.len())
        .map(|i| {
            let kind = match rows.cell(i, 1) {
                Some("p") => ConstraintKind::PrimaryKey,
                Some("u") => ConstraintKind::Unique,
                Some("f") => ConstraintKind::ForeignKey,
                Some("x") => ConstraintKind::Exclusion,
                _ => ConstraintKind::Check,
            };
            let references = (kind == ConstraintKind::ForeignKey).then(|| Reference {
                schema: text(&rows, i, 4),
                table: text(&rows, i, 5),
                columns: list(rows.cell(i, 6)),
                on_update: action(rows.cell(i, 7)),
                on_delete: action(rows.cell(i, 8)),
            });
            Constraint {
                name: text(&rows, i, 0),
                kind,
                columns: list(rows.cell(i, 3)),
                definition: text(&rows, i, 2),
                references,
            }
        })
        .collect();

    let rows = conn
        .query_text(&format!(
            "SELECT i.relname, am.amname, ix.indisunique, ix.indisprimary, \
                    pg_catalog.pg_get_indexdef(ix.indexrelid, 0, true), \
                    (SELECT string_agg(pg_catalog.pg_get_indexdef(ix.indexrelid, k.ord::int, true), {SEP_SQL} ORDER BY k.ord) \
                       FROM generate_series(1, ix.indnkeyatts) k(ord)), \
                    pg_catalog.pg_get_expr(ix.indpred, ix.indrelid, true), \
                    (SELECT string_agg(a.attname, {SEP_SQL} ORDER BY k.ord) \
                       FROM unnest(ix.indkey) WITH ORDINALITY k(attnum, ord) \
                       JOIN pg_catalog.pg_attribute a ON a.attrelid = ix.indrelid AND a.attnum = k.attnum \
                       WHERE k.ord > ix.indnkeyatts), \
                    array_to_string(i.reloptions, {SEP_SQL}), pg_catalog.obj_description(i.oid, 'pg_class'), \
                    EXISTS (SELECT 1 FROM pg_catalog.pg_constraint cn WHERE cn.conindid = ix.indexrelid \
                            AND cn.contype IN ('p', 'u', 'x')) \
             FROM pg_catalog.pg_index ix \
             JOIN pg_catalog.pg_class i ON i.oid = ix.indexrelid \
             JOIN pg_catalog.pg_class t ON t.oid = ix.indrelid \
             JOIN pg_catalog.pg_namespace n ON n.oid = t.relnamespace \
             JOIN pg_catalog.pg_am am ON am.oid = i.relam \
             WHERE n.nspname = {s} AND t.relname = {t} ORDER BY i.relname"
        ))
        .await?;
    let indexes = (0..rows.rows.len())
        .map(|i| Index {
            name: text(&rows, i, 0),
            algorithm: text(&rows, i, 1),
            is_unique: rows.cell(i, 2) == Some("t"),
            is_primary: rows.cell(i, 3) == Some("t"),
            definition: text(&rows, i, 4),
            columns: list(rows.cell(i, 5)),
            condition: rows.cell(i, 6).map(str::to_owned),
            include: list(rows.cell(i, 7)),
            storage_parameters: list(rows.cell(i, 8)),
            comment: rows.cell(i, 9).map(str::to_owned),
            is_constraint: rows.cell(i, 10) == Some("t"),
        })
        .collect();

    Ok(TableStructure {
        table: table_info,
        partitioned,
        columns,
        constraints,
        indexes,
        view_definition,
    })
}

#[cfg(test)]
mod tests {
    mod with_server {
        use hy_dbplus_driver::Connection;

        use crate::ddl::table_ddl;
        use crate::model::{ConstraintKind, TableKind};
        use crate::postgres::test_support::{connect, schema_name};
        use crate::structure::structure;

        /// Lê a estrutura de uma tabela com tudo o que o brief pede e prova o DDL: rodado num schema
        /// novo, produz a mesma estrutura (menos o nome do schema). Sem `serial` de propósito: o
        /// default apontaria para a sequência do schema antigo.
        #[tokio::test]
        async fn estrutura_completa_e_ddl_que_recria_a_tabela() {
            let Some(conn) = connect().await else { return };
            let a = schema_name();
            let setup = format!(
                "CREATE SCHEMA {a}; \
                 CREATE TABLE {a}.cliente (id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY, nome text); \
                 CREATE TABLE {a}.pedido (\"cliente id\" integer NOT NULL REFERENCES {a}.cliente(id) ON DELETE CASCADE, \
                     seq integer NOT NULL, obs text DEFAULT 's/ obs', \
                     total numeric(10,2) GENERATED ALWAYS AS (seq * 2) STORED, \
                     PRIMARY KEY (\"cliente id\", seq), CONSTRAINT obs_unica UNIQUE (obs), CHECK (seq > 0)); \
                 CREATE INDEX pedido_obs_idx ON {a}.pedido (obs) INCLUDE (total) WITH (fillfactor = 70) WHERE obs IS NOT NULL; \
                 COMMENT ON TABLE {a}.pedido IS 'pedidos d''água'; \
                 COMMENT ON COLUMN {a}.pedido.obs IS 'livre'; \
                 COMMENT ON INDEX {a}.pedido_obs_idx IS 'parcial'; \
                 CREATE VIEW {a}.pedido_v AS SELECT seq, obs FROM {a}.pedido;"
            );
            conn.query_text(&setup).await.unwrap();
            // Carrega tudo e roda o DDL num schema novo; apaga os dois schemas antes de conferir
            // (asserção que falha não pode deixar schema para trás).
            let b = schema_name();
            let loaded = async {
                let s = structure(&conn, &a, "pedido").await?;
                let cliente = structure(&conn, &a, "cliente").await?;
                let view = structure(&conn, &a, "pedido_v").await?;
                let missing = structure(&conn, &a, "nao_existe").await;
                let ddl = format!(
                    "CREATE SCHEMA {b}; {} {} {}",
                    table_ddl(&cliente),
                    table_ddl(&s),
                    table_ddl(&view)
                )
                .replace(&a, &b);
                conn.query_text(&ddl).await?;
                let mut pairs = Vec::new();
                for name in ["cliente", "pedido", "pedido_v"] {
                    let original = format!("{:?}", structure(&conn, &a, name).await?);
                    let recreated =
                        format!("{:?}", structure(&conn, &b, name).await?).replace(&b, &a);
                    pairs.push((name, original, recreated));
                }
                Ok::<_, hy_dbplus_driver::DriverError>((s, cliente, view, missing, pairs))
            }
            .await;
            conn.query_text(&format!(
                "DROP SCHEMA {a} CASCADE; DROP SCHEMA IF EXISTS {b} CASCADE"
            ))
            .await
            .unwrap();
            let (s, cliente, view, missing, pairs) = loaded.unwrap();

            assert_eq!(s.table.kind, TableKind::Table);
            assert!(!s.partitioned);
            assert_eq!(s.table.comment.as_deref(), Some("pedidos d'água"));
            let names: Vec<&str> = s.columns.iter().map(|c| c.name.as_str()).collect();
            assert_eq!(names, ["cliente id", "seq", "obs", "total"]);
            let obs = &s.columns[2];
            assert_eq!(obs.default.as_deref(), Some("'s/ obs'::text"));
            assert_eq!(obs.comment.as_deref(), Some("livre"));
            let total = &s.columns[3];
            assert_eq!(total.generated.as_deref(), Some("(seq * 2)"));
            assert!(total.default.is_none() && !total.generated_virtual);
            assert!(s.columns[0].is_primary_key && s.columns[1].is_primary_key);
            let kinds: Vec<ConstraintKind> = s.constraints.iter().map(|c| c.kind).collect();
            assert_eq!(
                kinds,
                [
                    ConstraintKind::Check,
                    ConstraintKind::ForeignKey,
                    ConstraintKind::PrimaryKey,
                    ConstraintKind::Unique
                ]
            );
            let fk = &s.constraints[1];
            assert_eq!(fk.columns, ["cliente id"]);
            let r = fk.references.as_ref().unwrap();
            assert_eq!(
                (r.schema.as_str(), r.table.as_str()),
                (a.as_str(), "cliente")
            );
            assert_eq!(r.columns, ["id"]);
            assert_eq!(
                (r.on_update.as_str(), r.on_delete.as_str()),
                ("no action", "cascade")
            );
            let pk = &s.constraints[2];
            assert_eq!(pk.columns, ["cliente id", "seq"]);
            let idx = s
                .indexes
                .iter()
                .find(|i| i.name == "pedido_obs_idx")
                .unwrap();
            assert_eq!(idx.algorithm, "btree");
            assert!(!idx.is_unique && !idx.is_primary && !idx.is_constraint);
            assert_eq!(idx.columns, ["obs"]);
            assert_eq!(idx.include, ["total"]);
            assert_eq!(idx.condition.as_deref(), Some("obs IS NOT NULL"));
            assert_eq!(idx.storage_parameters, ["fillfactor=70"]);
            assert_eq!(idx.comment.as_deref(), Some("parcial"));
            assert!(s.indexes.iter().any(|i| i.is_primary && i.is_constraint));
            assert!(
                s.indexes
                    .iter()
                    .any(|i| i.name == "obs_unica" && i.is_unique && i.is_constraint)
            );
            assert_eq!(cliente.columns[0].identity.as_deref(), Some("ALWAYS"));
            assert_eq!(view.table.kind, TableKind::View);
            assert!(
                view.view_definition
                    .as_deref()
                    .is_some_and(|d| d.contains("SELECT"))
            );
            assert!(missing.is_err(), "tabela inexistente tem de ser erro");
            for (name, original, recreated) in pairs {
                assert_eq!(recreated, original, "{name}");
            }
        }
    }
}
