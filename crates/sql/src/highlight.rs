//! Coloração por token para o editor: cada span diz onde (bytes) e o que é. Buffer incompleto
//! (string, `$$` ou `/*` sem fechar) não falha: colore os tokens lidos até o erro e o resto fica
//! `Plain`. Sem recursão: roda a cada frame no `layouter`.

use std::ops::Range;

use sqlparser::tokenizer::{Token, TokenWithSpan, Whitespace};

use crate::split::{byte_ranges, tokens};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Keyword,
    String,
    Number,
    Comment,
    Identifier,
    Operator,
    /// Texto que o tokenizador não entendeu (string sem fechar): sem cor.
    Plain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub kind: Kind,
}

/// Os spans do buffer, em ordem, sem sobreposição; espaço em branco fica de fora.
pub fn highlight(buffer: &str) -> Vec<Span> {
    let (tokens, error_at) = tokens(buffer);
    let ranges = byte_ranges(buffer, &tokens);
    spans_of(buffer, &tokens, &ranges, error_at)
}

/// O `highlight` a partir dos tokens já lidos (a [`crate::Analysis`] os guarda).
pub(crate) fn spans_of(
    buffer: &str,
    tokens: &[TokenWithSpan],
    ranges: &[Option<Range<usize>>],
    error_at: Option<usize>,
) -> Vec<Span> {
    let mut out: Vec<Span> = tokens
        .iter()
        .zip(ranges)
        .filter_map(|(t, range)| {
            let kind = kind_of(&t.token)?;
            let range = range.clone()?;
            (range.start < range.end).then_some(Span { range, kind })
        })
        .collect();
    if let Some(at) = error_at {
        // O trecho que não tokenizou: o erro de string aponta para o começo dela; o de `$$`,
        // `/*` e operador inacabado aponta para o fim do buffer, e aí vale o fim do último token
        // lido. Espaço em branco no começo fica de fora.
        let mut from = if at < buffer.len() {
            at
        } else {
            out.last().map_or(0, |s| s.range.end)
        };
        if let Some(rest) = buffer.get(from..) {
            from += rest.len() - rest.trim_start().len();
        }
        if from < buffer.len() {
            out.push(Span {
                range: from..buffer.len(),
                kind: Kind::Plain,
            });
        }
    }
    out
}

/// Palavras que o editor pinta como keyword: as reservadas do PostgreSQL mais as de DDL e DML
/// que aparecem em todo statement. Tipos (`integer`, `text`) ficam de fora de propósito.
const KEYWORDS: &[&str] = &[
    "ABORT",
    "ADD",
    "AFTER",
    "ALL",
    "ALTER",
    "ANALYZE",
    "AND",
    "ANY",
    "ARRAY",
    "AS",
    "ASC",
    "BEFORE",
    "BEGIN",
    "BETWEEN",
    "BY",
    "CALL",
    "CASCADE",
    "CASE",
    "CAST",
    "CHECK",
    "COALESCE",
    "COLLATE",
    "COLUMN",
    "COMMENT",
    "COMMIT",
    "CONCURRENTLY",
    "CONFLICT",
    "CONSTRAINT",
    "COPY",
    "COUNT",
    "CREATE",
    "CROSS",
    "DATABASE",
    "DEALLOCATE",
    "DECLARE",
    "DEFAULT",
    "DELETE",
    "DESC",
    "DISTINCT",
    "DO",
    "DOMAIN",
    "DROP",
    "EACH",
    "ELSE",
    "END",
    "ENUM",
    "ESCAPE",
    "EXCEPT",
    "EXCEPTION",
    "EXCLUDED",
    "EXECUTE",
    "EXISTS",
    "EXPLAIN",
    "EXTENSION",
    "FALSE",
    "FETCH",
    "FILTER",
    "FIRST",
    "FOR",
    "FOREIGN",
    "FORMAT",
    "FROM",
    "FULL",
    "FUNCTION",
    "GENERATED",
    "GRANT",
    "GROUP",
    "HAVING",
    "IDENTITY",
    "IF",
    "ILIKE",
    "IN",
    "INDEX",
    "INHERITS",
    "INNER",
    "INSERT",
    "INTERSECT",
    "INTERVAL",
    "INTO",
    "IS",
    "JOIN",
    "KEY",
    "LANGUAGE",
    "LAST",
    "LATERAL",
    "LEFT",
    "LIKE",
    "LIMIT",
    "LOCK",
    "LOOP",
    "MATERIALIZED",
    "MAX",
    "MIN",
    "NATURAL",
    "NEXT",
    "NOT",
    "NOTHING",
    "NULL",
    "NULLIF",
    "NULLS",
    "OFFSET",
    "ON",
    "ONLY",
    "OR",
    "ORDER",
    "OUTER",
    "OVER",
    "OWNER",
    "PARTITION",
    "PREPARE",
    "PRIMARY",
    "PROCEDURE",
    "RAISE",
    "RECURSIVE",
    "REFERENCES",
    "REFRESH",
    "RELEASE",
    "RENAME",
    "REPLACE",
    "RETURN",
    "RETURNING",
    "RETURNS",
    "REVOKE",
    "RIGHT",
    "ROLE",
    "ROLLBACK",
    "ROW",
    "ROWS",
    "SAVEPOINT",
    "SCHEMA",
    "SELECT",
    "SEQUENCE",
    "SET",
    "SHOW",
    "SIMILAR",
    "SOME",
    "START",
    "STATEMENT",
    "SUM",
    "TABLE",
    "TEMP",
    "TEMPORARY",
    "THEN",
    "TO",
    "TRANSACTION",
    "TRIGGER",
    "TRUE",
    "TRUNCATE",
    "TYPE",
    "UNION",
    "UNIQUE",
    "UNLOGGED",
    "UPDATE",
    "USING",
    "VACUUM",
    "VALUES",
    "VARIADIC",
    "VERBOSE",
    "VIEW",
    "WHEN",
    "WHERE",
    "WINDOW",
    "WITH",
    "WORK",
    "AVG",
];

fn is_keyword(word: &str) -> bool {
    KEYWORDS.iter().any(|k| k.eq_ignore_ascii_case(word))
}

fn kind_of(token: &Token) -> Option<Kind> {
    Some(match token {
        Token::Word(w) => {
            // Lista própria: a do sqlparser cobre todos os dialetos e pintaria `id`, `name` ou
            // `value` como keyword.
            if w.quote_style.is_none() && is_keyword(&w.value) {
                Kind::Keyword
            } else {
                Kind::Identifier
            }
        }
        Token::Number(..) => Kind::Number,
        Token::SingleQuotedString(_)
        | Token::DoubleQuotedString(_)
        | Token::TripleSingleQuotedString(_)
        | Token::TripleDoubleQuotedString(_)
        | Token::DollarQuotedString(_)
        | Token::EscapedStringLiteral(_)
        | Token::UnicodeStringLiteral(_)
        | Token::NationalStringLiteral(_)
        | Token::HexStringLiteral(_)
        | Token::SingleQuotedByteStringLiteral(_)
        | Token::DoubleQuotedByteStringLiteral(_)
        | Token::TripleSingleQuotedByteStringLiteral(_)
        | Token::TripleDoubleQuotedByteStringLiteral(_)
        | Token::SingleQuotedRawStringLiteral(_)
        | Token::DoubleQuotedRawStringLiteral(_)
        | Token::TripleSingleQuotedRawStringLiteral(_)
        | Token::TripleDoubleQuotedRawStringLiteral(_) => Kind::String,
        Token::Whitespace(
            Whitespace::SingleLineComment { .. } | Whitespace::MultiLineComment(_),
        ) => Kind::Comment,
        Token::Whitespace(_) | Token::EOF => return None,
        _ => Kind::Operator,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(buffer: &str) -> Vec<(String, Kind)> {
        highlight(buffer)
            .into_iter()
            .map(|s| (buffer[s.range].to_owned(), s.kind))
            .collect()
    }

    #[test]
    fn classifica_keyword_string_numero_comentario_identificador_e_operador() {
        let got = kinds("SELECT \"Nome\", total * 2 FROM pedido -- x\nWHERE id = 'a' /* b */");
        assert_eq!(
            got,
            [
                ("SELECT".to_owned(), Kind::Keyword),
                ("\"Nome\"".to_owned(), Kind::Identifier),
                (",".to_owned(), Kind::Operator),
                ("total".to_owned(), Kind::Identifier),
                ("*".to_owned(), Kind::Operator),
                ("2".to_owned(), Kind::Number),
                ("FROM".to_owned(), Kind::Keyword),
                ("pedido".to_owned(), Kind::Identifier),
                ("-- x".to_owned(), Kind::Comment),
                ("WHERE".to_owned(), Kind::Keyword),
                ("id".to_owned(), Kind::Identifier),
                ("=".to_owned(), Kind::Operator),
                ("'a'".to_owned(), Kind::String),
                ("/* b */".to_owned(), Kind::Comment),
            ]
        );
    }

    #[test]
    fn buffer_incompleto_colore_ate_onde_deu_e_o_resto_fica_plain() {
        let got = kinds("select 1 from t where x = 'aber");
        assert_eq!(got[0], ("select".to_owned(), Kind::Keyword));
        let last = got.last().unwrap();
        assert_eq!(last.1, Kind::Plain);
        assert!(last.0.starts_with('\''), "{last:?}");
        assert!(got.iter().any(|(t, k)| t == "where" && *k == Kind::Keyword));
        // Só a string aberta: tudo Plain; vazio: nada.
        assert_eq!(kinds("'x"), [("'x".to_owned(), Kind::Plain)]);
        assert!(highlight("").is_empty());
    }

    /// Antes: recursão infinita (estouro de pilha) quando o erro do tokenizador aponta para o fim
    /// do buffer (`$$`, `/*`, operador inacabado).
    #[test]
    fn dollar_comentario_e_operador_sem_fechar_terminam_com_plain_no_fim() {
        for buffer in [
            "select $$ab",
            "select /* ab",
            "select $t$ab",
            "select U&'ab",
            "select 1 $$",
            "$$",
            "select 1 /* a /* b */",
            "|&",
            "|&$>",
            "select 1 |& 2",
        ] {
            let spans = highlight(buffer);
            let last = spans
                .last()
                .unwrap_or_else(|| panic!("{buffer}: sem spans"));
            assert_eq!(last.kind, Kind::Plain, "{buffer}");
            assert_eq!(last.range.end, buffer.len(), "{buffer}");
        }
        let got = kinds("select 1 |& 2");
        assert_eq!(got[0], ("select".to_owned(), Kind::Keyword));
        assert_eq!(got[1], ("1".to_owned(), Kind::Number));
        assert_eq!(got.last().unwrap().1, Kind::Plain);
    }

    #[test]
    fn acento_e_dollar_nao_deslocam_os_spans() {
        let buffer = "select 'ação', $$a$$ as \"ç\"";
        let got = kinds(buffer);
        assert!(got.contains(&("'ação'".to_owned(), Kind::String)));
        assert!(got.contains(&("$$a$$".to_owned(), Kind::String)));
        assert!(got.contains(&("\"ç\"".to_owned(), Kind::Identifier)));
    }
}
