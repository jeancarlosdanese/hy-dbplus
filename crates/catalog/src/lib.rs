//! Modelo neutro do catálogo (`Schema`, `Table`, `Column`, `Function`; índices e FKs no M4) e os
//! carregadores por driver a partir de `pg_catalog`.
//!
//! Camada 1: depende só do `driver`. Sem cache aqui: o cache é do engine. Cada carregador é uma
//! query pelo protocolo simples (`Connection::query_text`), com os nomes escapados como literal.

pub mod model;
pub mod postgres;

pub use model::{Column, Function, Schema, Table, TableKind};
pub use postgres::{columns, functions, schemas, tables};

#[cfg(test)]
mod tests {
    #[test]
    fn a_crate_compila_com_os_lints_do_workspace() {}
}
