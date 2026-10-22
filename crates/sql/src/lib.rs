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
    #[test]
    fn a_crate_compila_com_os_lints_do_workspace() {}
}
