//! Beautify (⌘I): formatação pelo `sqlformat` com indentação de 2, keywords em maiúsculas e o
//! dialeto PostgreSQL. O resultado substitui o buffer, então só vale se não mudou nada além do
//! espaço em branco: a saída é tokenizada e comparada com a entrada (o `sqlformat` reescreve o
//! conteúdo de `$$…$$` e de string aberta); se difere, ou se a entrada não tokeniza, a entrada
//! volta intacta.

use sqlformat::{Dialect, FormatOptions, Indent, QueryParams};
use sqlparser::tokenizer::{Token, Whitespace};

use crate::split::tokens;

pub fn beautify(sql: &str) -> String {
    if sql.trim().is_empty() {
        return sql.to_owned();
    }
    let options = FormatOptions {
        indent: Indent::Spaces(2),
        uppercase: Some(true),
        dialect: Dialect::PostgreSql,
        ..FormatOptions::default()
    };
    let formatted = sqlformat::format(sql, &QueryParams::None, &options);
    if same_tokens(sql, &formatted) {
        formatted
    } else {
        sql.to_owned()
    }
}

/// Os mesmos tokens, fora espaço em branco e caixa de palavra não citada; comentários contam.
fn same_tokens(a: &str, b: &str) -> bool {
    let (ta, ea) = tokens(a);
    let (tb, eb) = tokens(b);
    if ea.is_some() || eb.is_some() {
        return false;
    }
    let meaningful = |ts: Vec<sqlparser::tokenizer::TokenWithSpan>| {
        ts.into_iter()
            .filter_map(|t| match t.token {
                Token::Whitespace(Whitespace::Space | Whitespace::Newline | Whitespace::Tab)
                | Token::EOF => None,
                Token::Whitespace(Whitespace::SingleLineComment { comment, prefix }) => {
                    Some(Token::Whitespace(Whitespace::SingleLineComment {
                        comment: comment.trim().to_owned(),
                        prefix,
                    }))
                }
                Token::Word(w) if w.quote_style.is_none() => {
                    Some(Token::make_word(&w.value.to_ascii_uppercase(), None))
                }
                other => Some(other),
            })
            .collect::<Vec<_>>()
    };
    meaningful(ta) == meaningful(tb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formata_com_indentacao_e_keywords_em_maiusculas() {
        let got = beautify(
            "select p.id, c.nome from pedido p join cliente c on c.id = p.cliente_id where p.total > 10 order by p.id",
        );
        assert_eq!(
            got,
            "SELECT\n  p.id,\n  c.nome\nFROM\n  pedido p\n  JOIN cliente c ON c.id = p.cliente_id\nWHERE\n  p.total > 10\nORDER BY\n  p.id"
        );
    }

    #[test]
    fn varios_statements_ficam_separados_e_vazio_volta_vazio() {
        let got = beautify("select 1; select 2;");
        assert_eq!(got, "SELECT\n  1;\nSELECT\n  2;");
        assert_eq!(beautify("   "), "   ");
    }

    /// Antes: o `sqlformat` reescrevia o conteúdo de `$$…$$` (espaços, `;`, quebras) e inseria
    /// espaço dentro de string aberta; o buffer era substituído pelo texto corrompido.
    #[test]
    fn literal_dollar_e_string_aberta_voltam_intactos() {
        for sql in [
            "select $$a  b;c$$",
            "insert into t values ($$a\n  b$$)",
            "DO $body$\nBEGIN\n  PERFORM 1;\nEND\n$body$;",
            "select 'ab",
            "select \"ab",
        ] {
            assert_eq!(beautify(sql), sql, "{sql:?}");
        }
        // String fechada com espaços e `;` dentro sai intacta no meio do SQL formatado.
        assert_eq!(beautify("select 'a  b;c' as x"), "SELECT\n  'a  b;c' AS x");
    }
}
