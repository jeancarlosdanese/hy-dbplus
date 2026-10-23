//! Modelo neutro do catálogo (`Schema`, `Table`, `Column`, `Function`, `Constraint`, `Index`), os
//! carregadores por driver a partir de `pg_catalog`, os dados de uma tabela para a aba Data
//! ([`data`]: tamanhos, contagem, página ordenada), o SQL do lote de edição ([`edit`]: `UPDATE`
//! e `DELETE` pela chave primária), a estrutura de uma tabela ([`structure`]: colunas, restrições,
//! índices, view), o DDL montado a partir dela ([`ddl`]) e os modelos de SQL do menu da árvore
//! ([`generate`]: `SELECT`, `INSERT`, `UPDATE`, `DELETE`, `TRUNCATE`, `DROP … CASCADE`).
//!
//! Camada 1: depende só do `driver`. Sem cache aqui: o cache é do engine. Tudo pelo protocolo
//! simples (`Connection::query_text`): os carregadores do catálogo escapam os nomes como literal
//! (`E'...'`); os da aba Data usam identificador entre aspas duplas.

pub mod data;
pub mod ddl;
pub mod edit;
pub mod generate;
pub mod model;
pub mod postgres;
pub mod structure;

pub use data::{Direction, Order, TableStats, count, count_sql, page, page_sql, stats};
pub use ddl::table_ddl;
pub use edit::{ChangeSet, EditError, RowUpdate, changes_sql};
pub use generate::{Template, example_value, template_sql};
pub use model::{
    Column, Constraint, ConstraintKind, Function, Index, Reference, Schema, Table, TableKind,
    TableStructure,
};
pub use postgres::{columns, functions, schemas, tables};
pub use structure::structure;

#[cfg(test)]
mod tests {
    #[test]
    fn a_crate_compila_com_os_lints_do_workspace() {}
}
