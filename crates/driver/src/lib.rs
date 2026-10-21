//! Conexões e execução de SQL: a trait `Driver` (conectar, ping, listar bancos, executar
//! query com stream de linhas e cancelamento) e a implementação PostgreSQL (M1).
//!
//! Camada 0: não conhece nenhuma outra crate do workspace.

#[cfg(test)]
mod tests {
    #[test]
    fn a_crate_compila_com_os_lints_do_workspace() {}
}
