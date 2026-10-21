//! Modelo neutro do catálogo (`Schema`, `Table`, `Column`, `Index`, `ForeignKey`,
//! `Function`) e os carregadores por driver a partir de `pg_catalog` (M2).
//!
//! Camada 1: depende só do `driver`.

#[cfg(test)]
mod tests {
    #[test]
    fn a_crate_compila_com_os_lints_do_workspace() {}
}
