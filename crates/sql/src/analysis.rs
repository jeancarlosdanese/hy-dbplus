//! A análise do buffer feita uma vez por texto (FEAT-012): os tokens com posição em bytes, os
//! statements (o que o `split` devolve) e os spans de cor (o que o `highlight` devolve). O editor
//! precisa das três coisas a cada frame (highlight), a cada ⌘↩ (Run Current) e, com o
//! autocomplete, a cada tecla; antes cada um tokenizava o buffer de novo. A aba guarda a última
//! análise e [`Analysis::refresh`] só refaz quando o texto mudou.

use std::ops::Range;

use sqlparser::tokenizer::{Token, TokenWithSpan};

use crate::highlight::{Span, spans_of};
use crate::split::{Statement, byte_ranges, split_tokens, tokens};

#[derive(Debug, Clone)]
pub struct Analysis {
    text: String,
    tokens: Vec<TokenWithSpan>,
    /// A posição em bytes de cada token, na mesma ordem (`None` quando o `Location` do
    /// tokenizador não cai no texto, o que não deve acontecer; o token é ignorado).
    ranges: Vec<Option<Range<usize>>>,
    error_at: Option<usize>,
    statements: Vec<Statement>,
    spans: Vec<Span>,
}

impl Analysis {
    pub fn new(text: &str) -> Self {
        let (tokens, error_at) = tokens(text);
        let ranges = byte_ranges(text, &tokens);
        let statements = split_tokens(text, &tokens, &ranges, error_at);
        let spans = spans_of(text, &tokens, &ranges, error_at);
        Self {
            text: text.to_owned(),
            tokens,
            ranges,
            error_at,
            statements,
            spans,
        }
    }

    /// A análise de `text`: a que está em `slot`, se é do mesmo texto, senão uma nova, guardada.
    pub fn refresh<'a>(slot: &'a mut Option<Analysis>, text: &str) -> &'a Analysis {
        if slot.as_ref().is_none_or(|a| a.text != text) {
            *slot = Some(Analysis::new(text));
        }
        // O `if` acima garante `Some`: a closure nunca roda (devolver o empréstimo condicional
        // direto do `if let` não passa no verificador de empréstimos).
        slot.get_or_insert_with(|| Analysis::new(text))
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }

    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// Onde o tokenizador parou, quando não leu o texto inteiro.
    pub fn error_at(&self) -> Option<usize> {
        self.error_at
    }

    /// O statement sob o cursor (byte), com a regra de [`crate::statement_at`].
    pub fn statement_at(&self, cursor: usize) -> Option<&Statement> {
        crate::split::statement_of(&self.statements, cursor)
    }

    /// Os tokens com a posição em bytes, na ordem; espaço em branco e comentário incluídos.
    pub fn tokens(&self) -> impl Iterator<Item = (&Token, &Range<usize>)> {
        self.tokens
            .iter()
            .zip(&self.ranges)
            .filter_map(|(t, r)| Some((&t.token, r.as_ref()?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{highlight, split, statement_at};

    #[test]
    fn a_analise_bate_com_split_highlight_e_statement_at_e_so_refaz_quando_o_texto_muda() {
        let text = "select 1;\nselect 'a -- x";
        let a = Analysis::new(text);
        assert_eq!(a.statements(), &split(text)[..]);
        assert_eq!(a.spans(), &highlight(text)[..]);
        assert_eq!(a.error_at(), Some(17));
        for cursor in [0, 5, 9, 10, 15, text.len()] {
            assert_eq!(a.statement_at(cursor).cloned(), statement_at(text, cursor));
        }
        assert_eq!(a.tokens().count(), 7, "select, ws, 1, ;, ws, select, ws");

        let mut slot = None;
        let first = Analysis::refresh(&mut slot, text) as *const Analysis;
        assert_eq!(Analysis::refresh(&mut slot, text) as *const Analysis, first);
        assert_eq!(Analysis::refresh(&mut slot, "select 2").text(), "select 2");
        assert_eq!(slot.as_ref().map(|a| a.statements().len()), Some(1));
    }
}
