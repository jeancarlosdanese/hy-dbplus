//! SQL do editor (FEAT-010): divisão do buffer em statements com posição ([`split`]), o statement
//! sob o cursor ([`statement_at`]), coloração por token ([`highlight`]) e formatação
//! ([`beautify`]). O motor de autocomplete (contexto do cursor × catálogo) é o M6.
//!
//! Camada 2: depende do `catalog` (autocomplete) e, aqui, do tokenizador do `sqlparser` (dialeto
//! PostgreSQL) e do `sqlformat`. Tudo puro e sem recursão: o buffer é entrada não confiável e
//! nada aqui falha, só devolve menos (o que não tokeniza vira a cauda incompleta do split, fica
//! sem cor no highlight e faz o Beautify devolver a entrada intacta).

pub mod format;
pub mod highlight;
pub mod split;

pub use format::beautify;
pub use highlight::{Kind, Span, highlight};
pub use split::{Statement, split, statement_at};

#[cfg(test)]
mod tests {
    use hy_dbplus_catalog::{Column, Table, TableKind, Template, template_sql};

    #[test]
    fn a_crate_compila_com_os_lints_do_workspace() {}

    /// FEAT-013: o texto gerado pelo menu da árvore entra no fim do buffer com o cursor no começo
    /// dele; o statement sob o cursor tem que ser o gerado, mesmo com o aviso em comentário (fora
    /// de um statement, `statement_at` devolveria o anterior do buffer).
    #[test]
    fn o_sql_gerado_e_um_statement_so_a_partir_do_cursor_no_comeco() {
        let table = Table {
            schema: "public".into(),
            name: "x".into(),
            kind: TableKind::Table,
            comment: None,
        };
        let no_key = [Column {
            name: "nome".into(),
            data_type: "text".into(),
            ..Column::default()
        }];
        let generated = [Column {
            name: "g".into(),
            data_type: "integer".into(),
            generated: Some("1".into()),
            ..Column::default()
        }];
        let cases: Vec<(Template, &[Column])> = vec![
            (Template::Select, &no_key),
            (Template::Count, &[]),
            (Template::Insert, &no_key),
            (Template::Update, &no_key),
            (Template::Update, &[]),
            (Template::Update, &generated),
            (Template::Delete, &no_key),
            (Template::Delete, &[]),
            (Template::Truncate, &[]),
            (Template::Drop, &[]),
        ];
        for (template, columns) in cases {
            let sql = template_sql(template, &table, columns);
            let buffer = format!("select 1;\n\n{sql}\n");
            let start = buffer.len() - sql.len() - 1;
            let statements = super::split(&buffer);
            assert_eq!(statements.len(), 2, "{template:?}: {buffer}");
            let current = super::statement_at(&buffer, start).expect("statement sob o cursor");
            assert_eq!(current.sql, sql.trim_end_matches(';'), "{template:?}");
            assert!(current.complete, "{template:?}");
        }
    }
}
