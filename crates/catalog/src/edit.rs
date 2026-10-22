//! O SQL de um lote de Update e Delete da aba Data (FEAT-007): um `UPDATE` por linha alterada, só
//! com as colunas que mudaram, e um `DELETE … IN` para todas as linhas marcadas. Só tabela com
//! chave primária: o `WHERE` é pela chave, e cada statement traz quantas linhas deve afetar (o
//! engine confere e desfaz a transação se a contagem não bater).
//!
//! Identificador entre aspas duplas (`quote_ident`), valor como literal `E'…'` (`literal`) e
//! `NULL` literal: nada cru chega ao SQL. O texto gerado aqui é exatamente o que o driver manda
//! ao servidor e o que a caixa de Commit mostra.

use std::fmt;

use crate::data::{qualified, quote_ident};
use crate::postgres::literal;

/// Uma linha alterada: a chave (valores em texto, na ordem de `ChangeSet::key_columns`) e as
/// colunas que mudaram, com o valor novo (`None` é `NULL`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RowUpdate {
    pub key: Vec<String>,
    pub set: Vec<(String, Option<String>)>,
}

/// O lote pendente de uma tabela: as colunas da chave primária, as linhas alteradas e as chaves
/// das linhas a apagar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChangeSet {
    pub key_columns: Vec<String>,
    pub updates: Vec<RowUpdate>,
    pub deletes: Vec<Vec<String>>,
}

impl ChangeSet {
    pub fn is_empty(&self) -> bool {
        self.updates.iter().all(|u| u.set.is_empty()) && self.deletes.is_empty()
    }
}

/// Um statement do lote e quantas linhas ele deve afetar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub sql: String,
    pub expected_rows: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    /// Tabela sem chave primária: não há como apontar a linha com segurança.
    NoPrimaryKey,
    /// Uma chave com número de valores diferente do número de colunas da chave.
    KeyMismatch { expected: usize, got: usize },
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::NoPrimaryKey => write!(f, "a tabela não tem chave primária: só leitura"),
            EditError::KeyMismatch { expected, got } => {
                write!(
                    f,
                    "chave com {got} valores; a chave primária tem {expected} colunas"
                )
            }
        }
    }
}

impl std::error::Error for EditError {}

/// Os statements do lote, na ordem: um `UPDATE` por linha (as que têm alguma coluna em `set`) e,
/// por fim, um `DELETE` para todas as chaves de `deletes`, se houver.
pub fn changes_sql(
    schema: &str,
    table: &str,
    changes: &ChangeSet,
) -> Result<Vec<Statement>, EditError> {
    if changes.key_columns.is_empty() {
        return Err(EditError::NoPrimaryKey);
    }
    let target = qualified(schema, table);
    let mut statements = Vec::new();
    for update in &changes.updates {
        if update.set.is_empty() {
            continue;
        }
        let assignments: Vec<String> = update
            .set
            .iter()
            .map(|(column, value)| {
                format!("{} = {}", quote_ident(column), value_sql(value.as_deref()))
            })
            .collect();
        statements.push(Statement {
            sql: format!(
                "UPDATE {target} SET {} WHERE {}",
                assignments.join(", "),
                key_where(&changes.key_columns, &update.key)?
            ),
            expected_rows: 1,
        });
    }
    if !changes.deletes.is_empty() {
        let keys: Vec<String> = changes
            .deletes
            .iter()
            .map(|key| key_tuple(&changes.key_columns, key))
            .collect::<Result<_, _>>()?;
        let columns: Vec<String> = changes.key_columns.iter().map(|c| quote_ident(c)).collect();
        statements.push(Statement {
            sql: format!(
                "DELETE FROM {target} WHERE ({}) IN ({})",
                columns.join(", "),
                keys.join(", ")
            ),
            expected_rows: changes.deletes.len() as u64,
        });
    }
    Ok(statements)
}

fn value_sql(value: Option<&str>) -> String {
    value.map_or_else(|| "NULL".to_owned(), literal)
}

fn check_key(key_columns: &[String], key: &[String]) -> Result<(), EditError> {
    if key.len() != key_columns.len() {
        return Err(EditError::KeyMismatch {
            expected: key_columns.len(),
            got: key.len(),
        });
    }
    Ok(())
}

/// `"pk" = E'1' AND "pk2" = E'a'`.
fn key_where(key_columns: &[String], key: &[String]) -> Result<String, EditError> {
    check_key(key_columns, key)?;
    Ok(key_columns
        .iter()
        .zip(key)
        .map(|(column, value)| format!("{} = {}", quote_ident(column), literal(value)))
        .collect::<Vec<_>>()
        .join(" AND "))
}

/// `(E'1', E'a')`.
fn key_tuple(key_columns: &[String], key: &[String]) -> Result<String, EditError> {
    check_key(key_columns, key)?;
    let values: Vec<String> = key.iter().map(|v| literal(v)).collect();
    Ok(format!("({})", values.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> String {
        v.to_owned()
    }

    #[test]
    fn update_so_com_as_colunas_alteradas_e_where_pela_chave() {
        let changes = ChangeSet {
            key_columns: vec![s("id")],
            updates: vec![
                RowUpdate {
                    key: vec![s("7")],
                    set: vec![(s("nome"), Some(s("O'Hara \\ Cia"))), (s("saldo"), None)],
                },
                RowUpdate {
                    key: vec![s("8")],
                    set: vec![],
                },
            ],
            deletes: vec![],
        };
        let statements = changes_sql("public", "Clientes", &changes).unwrap();
        assert_eq!(
            statements.len(),
            1,
            "linha sem coluna alterada não gera statement"
        );
        assert_eq!(
            statements[0].sql,
            "UPDATE \"public\".\"Clientes\" SET \"nome\" = E'O''Hara \\\\ Cia', \"saldo\" = NULL WHERE \"id\" = E'7'"
        );
        assert_eq!(statements[0].expected_rows, 1);
    }

    #[test]
    fn delete_unico_com_chave_composta_e_identificador_com_aspas() {
        let changes = ChangeSet {
            key_columns: vec![s("pedido"), s("item \"n\"")],
            updates: vec![],
            deletes: vec![vec![s("1"), s("a")], vec![s("2"), s("b")]],
        };
        let statements = changes_sql("vendas", "itens", &changes).unwrap();
        assert_eq!(statements.len(), 1);
        assert_eq!(
            statements[0].sql,
            "DELETE FROM \"vendas\".\"itens\" WHERE (\"pedido\", \"item \"\"n\"\"\") IN ((E'1', E'a'), (E'2', E'b'))"
        );
        assert_eq!(statements[0].expected_rows, 2);
    }

    #[test]
    fn updates_antes_do_delete_e_lote_vazio_sem_statements() {
        let changes = ChangeSet {
            key_columns: vec![s("id")],
            updates: vec![RowUpdate {
                key: vec![s("1")],
                set: vec![(s("x"), Some(s("1")))],
            }],
            deletes: vec![vec![s("2")]],
        };
        let statements = changes_sql("s", "t", &changes).unwrap();
        assert!(statements[0].sql.starts_with("UPDATE"));
        assert!(statements[1].sql.starts_with("DELETE"));
        let empty = ChangeSet {
            key_columns: vec![s("id")],
            ..Default::default()
        };
        assert!(empty.is_empty());
        assert!(changes_sql("s", "t", &empty).unwrap().is_empty());
        assert!(!changes.is_empty());
    }

    #[test]
    fn sem_chave_primaria_ou_chave_incompleta_e_erro() {
        let no_key = ChangeSet {
            deletes: vec![vec![s("1")]],
            ..Default::default()
        };
        assert_eq!(changes_sql("s", "t", &no_key), Err(EditError::NoPrimaryKey));
        let short = ChangeSet {
            key_columns: vec![s("a"), s("b")],
            updates: vec![RowUpdate {
                key: vec![s("1")],
                set: vec![(s("x"), None)],
            }],
            deletes: vec![],
        };
        let err = changes_sql("s", "t", &short).unwrap_err();
        assert_eq!(
            err,
            EditError::KeyMismatch {
                expected: 2,
                got: 1
            }
        );
        assert_eq!(
            err.to_string(),
            "chave com 1 valores; a chave primária tem 2 colunas"
        );
    }
}
