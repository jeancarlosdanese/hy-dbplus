//! A interface: lista de conexões, árvore, grid, abas, editor SQL, painel lateral e temas.
//! Única crate do workspace que conhece `egui` e `eframe`.
//!
//! Camada 4: depende só do `engine`. M1: telas 3.1 (conexões) e 3.2 (conexão aberta).

mod app;
mod connections;
pub mod theme;
mod workspace;

pub use app::{App, Deps};
