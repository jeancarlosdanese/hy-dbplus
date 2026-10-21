//! Fachada usada por todos os frontends (app, CLI, MCP): `Session`, `Workspace` (abas),
//! `ConnectionStore` (TOML + Keychain), histórico de queries e eventos assíncronos (M1+).
//!
//! Camada 3: depende de `driver`, `catalog` e `sql`. Nada aqui conhece `egui`.

#[cfg(test)]
mod tests {
    #[test]
    fn a_crate_compila_com_os_lints_do_workspace() {}
}
