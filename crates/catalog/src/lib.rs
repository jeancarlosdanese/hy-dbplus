//! Modelo neutro do catálogo (`Schema`, `Table`, `Column`, `Function`; índices e FKs no M4), os
//! carregadores por driver a partir de `pg_catalog`, os dados de uma tabela para a aba Data
//! ([`data`]: tamanhos, contagem, página ordenada) e o SQL do lote de edição ([`edit`]: `UPDATE`
//! e `DELETE` pela chave primária).
//!
//! Camada 1: depende só do `driver`. Sem cache aqui: o cache é do engine. Tudo pelo protocolo
//! simples (`Connection::query_text`): os carregadores do catálogo escapam os nomes como literal
//! (`E'...'`); os da aba Data usam identificador entre aspas duplas.

pub mod data;
pub mod edit;
pub mod model;
pub mod postgres;

pub use data::{Direction, Order, TableStats, count, count_sql, page, page_sql, stats};
pub use edit::{ChangeSet, EditError, RowUpdate, Statement, changes_sql};
pub use model::{Column, Function, Schema, Table, TableKind};
pub use postgres::{columns, functions, schemas, tables};

#[cfg(test)]
mod tests {
    #[test]
    fn a_crate_compila_com_os_lints_do_workspace() {}
}
