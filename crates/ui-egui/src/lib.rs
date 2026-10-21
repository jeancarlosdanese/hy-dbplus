//! A interface: lista de conexões, árvore, grid, abas, editor SQL, painel lateral e temas.
//! Única crate do workspace que conhece `egui` e `eframe` (M0: tema e janela vazia).
//!
//! Camada 4: depende só do `engine`.

#[cfg(test)]
mod tests {
    #[test]
    fn a_crate_compila_com_os_lints_do_workspace() {}
}
