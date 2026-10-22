//! A interface: lista de conexões, árvore, grid, abas, editor SQL, painel lateral e temas.
//! Única crate do workspace que conhece `egui` e `eframe`.
//!
//! Camada 4: depende só do `engine`. M1: telas 3.1 (conexões) e 3.2 (conexão aberta); M2: paleta ⌘K e árvore (3.3).

mod app;
mod commit;
mod connections;
mod data;
mod palette;
mod record;
pub mod theme;
mod tree;
mod workspace;

pub use app::{App, Deps, PgApp};
