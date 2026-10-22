//! Divisão do buffer em statements e o statement sob o cursor. Decide o que o Run Current executa:
//! `;` só conta fora de string, `$$…$$`, comentário e identificador entre aspas, porque o
//! tokenizador do `sqlparser` é quem vê os `;`. Buffer que não tokeniza até o fim (string, `$$`
//! ou `/*` sem fechar, operador que o tokenizador não conhece): o que vem antes do erro divide
//! normalmente e a cauda, do statement em curso até o fim, vira o último statement, marcado
//! `complete = false` (o servidor recusa só ele; a UI pode avisar).

use std::ops::Range;

use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::tokenizer::{Location, Token, TokenWithSpan, Tokenizer};

/// Um statement do buffer: o texto sem o `;`, onde está (bytes), em que linha começa (1) e se o
/// tokenizador o leu inteiro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub sql: String,
    pub range: Range<usize>,
    pub line: usize,
    pub complete: bool,
}

/// Começo de cada linha, em bytes, para traduzir `Location` (linha e coluna em caracteres) em
/// posição no buffer.
pub(crate) struct Lines {
    starts: Vec<usize>,
}

impl Lines {
    pub(crate) fn new(buffer: &str) -> Self {
        let mut starts = vec![0];
        for (i, c) in buffer.char_indices() {
            if c == '\n' {
                starts.push(i + 1);
            }
        }
        Self { starts }
    }

    /// Byte do caractere em `location` (coluna a partir de 1). A coluna logo depois do último
    /// caractere da linha também vale (é o `end` dos spans do tokenizador, exclusivo, e onde
    /// ele aponta o erro de `$$`/`/*` sem fechar); fora disso, `None`.
    pub(crate) fn byte_at(&self, buffer: &str, location: Location) -> Option<usize> {
        let line = usize::try_from(location.line).ok()?.checked_sub(1)?;
        let column = usize::try_from(location.column).ok()?.checked_sub(1)?;
        let start = *self.starts.get(line)?;
        let rest = buffer.get(start..)?;
        let text = rest.split('\n').next().unwrap_or_default();
        let mut chars = text.char_indices();
        match chars.nth(column) {
            Some((offset, _)) => Some(start + offset),
            None if column == text.chars().count() => Some(start + text.len()),
            None => None,
        }
    }
}

/// Os tokens com posição até onde o tokenizador chegou, e o byte do erro quando ele não chegou
/// ao fim. Nunca falha: um buffer que erra no primeiro caractere devolve lista vazia e erro em 0.
pub(crate) fn tokens(buffer: &str) -> (Vec<TokenWithSpan>, Option<usize>) {
    let mut buf = Vec::new();
    match Tokenizer::new(&PostgreSqlDialect {}, buffer).tokenize_with_location_into_buf(&mut buf) {
        Ok(()) => (buf, None),
        Err(error) => {
            let at = Lines::new(buffer)
                .byte_at(buffer, error.location)
                .unwrap_or(buffer.len());
            (buf, Some(at))
        }
    }
}

/// Os statements do buffer, na ordem; vazio (só espaço e comentário) não entra; o último sem `;`
/// entra.
pub fn split(buffer: &str) -> Vec<Statement> {
    let (tokens, error_at) = tokens(buffer);
    let lines = Lines::new(buffer);
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut end = 0;
    for t in &tokens {
        match &t.token {
            Token::SemiColon => {
                if let Some(s) = start.take() {
                    push(&mut out, buffer, s, end, true);
                }
            }
            Token::Whitespace(_) | Token::EOF => {}
            _ => {
                // O `end` do span é exclusivo (a posição depois do token), apesar da doc.
                let (Some(s), Some(e)) = (
                    lines.byte_at(buffer, t.span.start),
                    lines.byte_at(buffer, t.span.end),
                ) else {
                    continue;
                };
                if start.is_none() {
                    start = Some(s);
                }
                end = e.max(end);
            }
        }
    }
    match error_at {
        // A cauda que não tokenizou: do statement em curso (ou do fim do último token lido, que
        // é 0 num buffer que erra logo no começo) até o fim do buffer.
        Some(_) => {
            let s = start.unwrap_or(end);
            push(&mut out, buffer, s, buffer.len(), false);
        }
        None => {
            if let Some(s) = start {
                push(&mut out, buffer, s, end, true);
            }
        }
    }
    out
}

fn push(out: &mut Vec<Statement>, buffer: &str, start: usize, end: usize, complete: bool) {
    let Some(raw) = buffer.get(start..end) else {
        return;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }
    // Espaço em branco no começo (a cauda incompleta vem do fim do token anterior) fica de fora.
    let start = start + raw.len() - raw.trim_start().len();
    let sql = trimmed;
    let line = buffer
        .get(..start)
        .map(|before| before.matches('\n').count() + 1)
        .unwrap_or(1);
    out.push(Statement {
        sql: sql.to_owned(),
        range: start..start + sql.len(),
        line,
        complete,
    });
}

/// O statement sob o cursor (byte): o que o contém, senão o anterior (cursor no espaço entre
/// dois, ou depois do último), senão o primeiro (cursor antes de tudo). O cursor é em bytes, como
/// `range`; quem tem o cursor em caracteres (o egui) converte antes:
/// `buffer.char_indices().nth(i).map_or(buffer.len(), |(b, _)| b)`.
pub fn statement_at(buffer: &str, cursor: usize) -> Option<Statement> {
    let statements = split(buffer);
    if let Some(s) = statements
        .iter()
        .find(|s| s.range.start <= cursor && cursor <= s.range.end)
    {
        return Some(s.clone());
    }
    statements
        .iter()
        .rev()
        .find(|s| s.range.end < cursor)
        .or_else(|| statements.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sqls(buffer: &str) -> Vec<String> {
        split(buffer).into_iter().map(|s| s.sql).collect()
    }

    #[test]
    fn divide_por_ponto_e_virgula_fora_de_string_dollar_comentario_e_aspas() {
        let buffer = "select 'a;b', \"x;y\" from t; -- fim; não\n\
                      /* bloco; */ update t set f = $$a;b$$ where id = 1;\n\
                      \n\
                      delete from t";
        assert_eq!(
            sqls(buffer),
            [
                "select 'a;b', \"x;y\" from t",
                "update t set f = $$a;b$$ where id = 1",
                "delete from t",
            ]
        );
        let statements = split(buffer);
        assert_eq!(statements[0].range, 0..26);
        assert_eq!(statements[0].line, 1);
        assert_eq!(statements[1].line, 2);
        assert_eq!(statements[2].line, 4);
        assert_eq!(&buffer[statements[2].range.clone()], "delete from t");
        assert!(statements.iter().all(|s| s.complete));
    }

    #[test]
    fn vazio_e_so_comentario_nao_entram_e_acentos_nao_deslocam_as_posicoes() {
        assert!(split("").is_empty());
        assert!(split(" ; ;\n-- nada\n").is_empty());
        let buffer = "select 'ação' as x;\nselect 2";
        let statements = split(buffer);
        assert_eq!(&buffer[statements[0].range.clone()], "select 'ação' as x");
        assert_eq!(&buffer[statements[1].range.clone()], "select 2");
    }

    #[test]
    fn o_que_vem_antes_do_erro_divide_e_a_cauda_vira_o_ultimo_statement_incompleto() {
        // String aberta na 2ª linha: a 1ª sai normal; só a cauda é incompleta (o Run Current não
        // manda o delete junto).
        let buffer = "delete from pedido where id = 7;\nselect 'ab";
        let statements = split(buffer);
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].sql, "delete from pedido where id = 7");
        assert!(statements[0].complete);
        assert_eq!(statements[1].sql, "select 'ab");
        assert_eq!(statements[1].range, 33..43);
        assert!(!statements[1].complete);
        // `$$`, `/*` e operador desconhecido sem fechar: a cauda começa no statement em curso.
        for buffer in [
            "select 1;\nselect $$ab",
            "select 1;\nselect /* ab",
            "select 1 |& 2",
        ] {
            let last = split(buffer).pop().unwrap();
            assert!(!last.complete, "{buffer}");
            assert!(buffer.ends_with(&last.sql), "{buffer}: {last:?}");
        }
        // Só a string aberta, com espaço nas pontas.
        let statements = split("  select 'aberta; select 2  ");
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].sql, "select 'aberta; select 2");
        assert_eq!(statements[0].range, 2..26);
        assert!(!statements[0].complete);
        assert!(split("$$").len() == 1 && split("|&").len() == 1);
    }

    #[test]
    fn statement_sob_o_cursor_ou_o_anterior_ou_o_primeiro() {
        let buffer = "select 1;\n\nselect 2;\n";
        let at = |cursor: usize| statement_at(buffer, cursor).map(|s| s.sql);
        assert_eq!(at(3).as_deref(), Some("select 1"));
        assert_eq!(at(8).as_deref(), Some("select 1"), "no fim do texto");
        assert_eq!(at(10).as_deref(), Some("select 1"), "no espaço entre dois");
        assert_eq!(at(14).as_deref(), Some("select 2"));
        assert_eq!(at(21).as_deref(), Some("select 2"), "depois do último");
        assert_eq!(
            statement_at("  select 1", 0).map(|s| s.sql).as_deref(),
            Some("select 1")
        );
        assert!(statement_at("", 0).is_none());
    }
}
