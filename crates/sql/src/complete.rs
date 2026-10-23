//! O motor do autocomplete (FEAT-012): dado o texto analisado, o cursor e uma visão do catálogo
//! que a UI já tem, devolve o trecho a substituir e as sugestões, em ordem. Puro: a UI desenha o
//! popup e pede ao engine o que a visão não tem (`Completion::needs`).
//!
//! Contexto pelos tokens do statement sob o cursor:
//! - `alias.` ou `tabela.` → colunas da tabela (alias resolvido no `FROM`/`JOIN`/`UPDATE`/`INTO`
//!   do mesmo statement); `schema.` → tabelas e views do schema;
//! - depois de `FROM`, `JOIN`, `INTO`, `UPDATE`, `TABLE` → tabelas e views do schema atual e schemas;
//! - no resto → colunas das tabelas do statement, aliases, keywords, tabelas do schema atual e
//!   funções;
//! - dentro de string, comentário ou identificador entre aspas → nada.

use std::collections::HashMap;
use std::ops::Range;

use hy_dbplus_catalog::{Column, Function, Schema, Table, TableKind};
use sqlparser::tokenizer::{Token, Whitespace};

use crate::analysis::Analysis;
use crate::highlight::{KEYWORDS, Kind, is_keyword, kind_of};

/// O máximo de sugestões devolvidas.
pub const MAX_ITEMS: usize = 50;

/// A visão do catálogo que a UI já carregou, emprestada (nada é clonado por tecla). As chaves de
/// `columns` são `(schema, tabela)`, como no `Workspace` da UI.
#[derive(Clone, Copy)]
pub struct CatalogView<'a> {
    pub current_schema: Option<&'a str>,
    pub schemas: &'a [Schema],
    pub objects: &'a HashMap<String, (Vec<Table>, Vec<Function>)>,
    pub columns: &'a HashMap<(String, String), Vec<Column>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemKind {
    Column,
    Alias,
    Keyword,
    Table,
    View,
    Schema,
    Function,
}

/// Uma sugestão: o texto que entra no lugar do prefixo (com aspas quando o nome precisa) e o
/// detalhe para a lista (tipo da coluna, schema da tabela, assinatura da função).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub label: String,
    pub kind: ItemKind,
    pub detail: String,
}

/// O que a visão não tem e o engine pode carregar: as colunas de uma tabela do statement ou os
/// objetos de um schema citado. A UI pede e chama `complete` de novo quando o evento chega.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Need {
    Columns { schema: String, table: String },
    Schema(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    /// O trecho do texto (bytes) que a sugestão aceita substitui: o prefixo em digitação, ou vazio
    /// no cursor.
    pub replace: Range<usize>,
    pub items: Vec<Item>,
    pub needs: Vec<Need>,
}

/// Uma tabela citada no statement (`FROM`, `JOIN`, `UPDATE`, `INSERT INTO`, `DELETE FROM`), com
/// o schema resolvido e o alias, se tem.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TableRef {
    schema: String,
    table: String,
    alias: Option<String>,
}

/// Um token do statement com a posição, só os que importam para o contexto (sem espaço).
struct Tok<'a> {
    token: &'a Token,
    range: Range<usize>,
}

/// As sugestões para o cursor (byte) no texto analisado; `None` quando não cabe sugerir (string,
/// comentário, identificador entre aspas, texto que não tokenizou até o cursor).
pub fn complete(analysis: &Analysis, cursor: usize, catalog: &CatalogView) -> Option<Completion> {
    let text = analysis.text();
    if analysis.error_at().is_some_and(|at| cursor >= at) {
        return None;
    }
    // Dentro de string, comentário ou identificador entre aspas: nada.
    for (token, range) in analysis.tokens() {
        let inside = match token {
            Token::Whitespace(Whitespace::SingleLineComment { .. }) => {
                let ends_with_newline = text.get(range.clone()).is_some_and(|c| c.ends_with('\n'));
                range.start < cursor
                    && (cursor < range.end || (cursor == range.end && !ends_with_newline))
            }
            Token::Whitespace(Whitespace::MultiLineComment(_)) => {
                range.start < cursor && cursor < range.end
            }
            Token::Word(w) if w.quote_style.is_some() => range.start < cursor && cursor < range.end,
            t if kind_of(t) == Some(Kind::String) => range.start < cursor && cursor < range.end,
            _ => false,
        };
        if inside {
            return None;
        }
    }
    let statement = analysis.statement_at(cursor).map(|s| s.range.clone());
    let toks: Vec<Tok> = analysis
        .tokens()
        .filter(|(t, _)| !matches!(t, Token::Whitespace(_) | Token::EOF))
        .filter(|(_, r)| {
            statement
                .as_ref()
                .is_none_or(|s| r.start >= s.start && r.end <= s.end)
        })
        .map(|(token, range)| Tok {
            token,
            range: range.clone(),
        })
        .collect();

    // O prefixo: a palavra sem aspas que o cursor está digitando (cursor dentro ou no fim dela).
    let typing = toks.iter().position(|t| {
        matches!(t.token, Token::Word(w) if w.quote_style.is_none())
            && t.range.start < cursor
            && cursor <= t.range.end
    });
    let (replace, before): (Range<usize>, &[Tok]) =
        match typing.and_then(|i| Some((toks.get(i)?.range.start, toks.get(..i)?))) {
            Some((start, before)) => (start..cursor, before),
            None => {
                let n = toks.iter().take_while(|t| t.range.end <= cursor).count();
                (cursor..cursor, toks.get(..n).unwrap_or(&[]))
            }
        };
    let prefix = text.get(replace.clone()).unwrap_or_default();
    let refs = table_refs(&toks, catalog.current_schema);
    let mut needs: Vec<Need> = refs
        .iter()
        .filter(|r| {
            !catalog
                .columns
                .contains_key(&(r.schema.clone(), r.table.clone()))
        })
        .map(|r| Need::Columns {
            schema: r.schema.clone(),
            table: r.table.clone(),
        })
        .collect();
    needs.dedup();

    let mut items: Vec<Item> = Vec::new();
    let last = before.last().map(|t| t.token);
    if matches!(last, Some(Token::Period)) {
        // `q.`: alias ou tabela do statement → colunas; schema → tabelas; tabela do schema atual.
        let Some(Token::Word(q)) = before.iter().rev().nth(1).map(|t| t.token) else {
            return None;
        };
        let qualifier = q.value.as_str();
        let by_alias = refs.iter().find(|r| {
            r.alias
                .as_deref()
                .is_some_and(|a| a.eq_ignore_ascii_case(qualifier))
        });
        let by_table = refs
            .iter()
            .find(|r| r.alias.is_none() && r.table.eq_ignore_ascii_case(qualifier));
        if let Some(r) = by_alias {
            push_columns(&mut items, catalog, &r.schema, &r.table);
        } else if catalog.schemas.iter().any(|s| s.name == qualifier) {
            match catalog.objects.get(qualifier) {
                Some((tables, _)) => push_tables(&mut items, tables, false),
                None => needs.push(Need::Schema(qualifier.to_owned())),
            }
        } else if let Some(r) = by_table {
            push_columns(&mut items, catalog, &r.schema, &r.table);
        } else if let Some(schema) = catalog.current_schema
            && table_of(catalog, schema, qualifier).is_some()
        {
            if !catalog
                .columns
                .contains_key(&(schema.to_owned(), qualifier.to_owned()))
            {
                needs.push(Need::Columns {
                    schema: schema.to_owned(),
                    table: qualifier.to_owned(),
                });
            }
            push_columns(&mut items, catalog, schema, qualifier);
        } else {
            return None;
        }
    } else if matches!(last, Some(Token::Word(w)) if w.quote_style.is_none() && ["FROM", "JOIN", "INTO", "UPDATE", "TABLE"].iter().any(|k| k.eq_ignore_ascii_case(&w.value)))
    {
        if let Some((tables, _)) = catalog.current_schema.and_then(|s| catalog.objects.get(s)) {
            push_tables(&mut items, tables, true);
        }
        for s in catalog.schemas {
            items.push(Item {
                label: ident(&s.name),
                kind: ItemKind::Schema,
                detail: "schema".into(),
            });
        }
    } else {
        for r in &refs {
            push_columns(&mut items, catalog, &r.schema, &r.table);
            if let Some(alias) = &r.alias {
                items.push(Item {
                    label: ident(alias),
                    kind: ItemKind::Alias,
                    detail: format!("{}.{}", r.schema, r.table),
                });
            }
        }
        for k in KEYWORDS {
            items.push(Item {
                label: keyword_case(k, prefix),
                kind: ItemKind::Keyword,
                detail: String::new(),
            });
        }
        if let Some((tables, functions)) =
            catalog.current_schema.and_then(|s| catalog.objects.get(s))
        {
            push_tables(&mut items, tables, true);
            for f in functions {
                items.push(Item {
                    label: ident(&f.name),
                    kind: ItemKind::Function,
                    detail: format!("({}) → {}", f.arguments, f.returns),
                });
            }
        }
    }
    Some(Completion {
        replace,
        items: rank(items, prefix),
        needs,
    })
}

/// Filtra pelo prefixo (começa com antes de contém, caixa ignorada), mantém a ordem dos grupos e
/// corta em [`MAX_ITEMS`]; rótulos repetidos (coluna com o mesmo nome em duas tabelas) saem uma vez.
fn rank(items: Vec<Item>, prefix: &str) -> Vec<Item> {
    let p = prefix.to_lowercase();
    let mut ranked: Vec<(u8, usize, Item)> = items
        .into_iter()
        .enumerate()
        .filter_map(|(i, item)| {
            let name = item.label.trim_matches('"').to_lowercase();
            let rank = if p.is_empty() || name.starts_with(&p) {
                0
            } else if name.contains(&p) {
                1
            } else {
                return None;
            };
            Some((rank, i, item))
        })
        .collect();
    ranked.sort_by_key(|a| (a.0, a.1));
    let mut out: Vec<Item> = Vec::new();
    for (_, _, item) in ranked {
        if out.len() >= MAX_ITEMS {
            break;
        }
        if !out
            .iter()
            .any(|o| o.label == item.label && o.kind == item.kind)
        {
            out.push(item);
        }
    }
    out
}

fn push_columns(items: &mut Vec<Item>, catalog: &CatalogView, schema: &str, table: &str) {
    if let Some(columns) = catalog.columns.get(&(schema.to_owned(), table.to_owned())) {
        for c in columns {
            items.push(Item {
                label: ident(&c.name),
                kind: ItemKind::Column,
                detail: if c.is_primary_key {
                    format!("{} · chave", c.data_type)
                } else {
                    c.data_type.clone()
                },
            });
        }
    }
}

fn push_tables(items: &mut Vec<Item>, tables: &[Table], with_schema: bool) {
    for t in tables {
        let kind = if t.kind.is_view() {
            ItemKind::View
        } else {
            ItemKind::Table
        };
        let what = match t.kind {
            TableKind::Table => "tabela",
            TableKind::View => "view",
            TableKind::MaterializedView => "view materializada",
            TableKind::Foreign => "tabela estrangeira",
        };
        items.push(Item {
            label: ident(&t.name),
            kind,
            detail: if with_schema {
                format!("{what} · {}", t.schema)
            } else {
                what.to_owned()
            },
        });
    }
}

fn table_of<'a>(catalog: &'a CatalogView, schema: &str, name: &str) -> Option<&'a Table> {
    catalog
        .objects
        .get(schema)?
        .0
        .iter()
        .find(|t| t.name == name)
}

/// As tabelas citadas no statement: depois de `FROM`, `JOIN`, `UPDATE` ou `INTO`, `[schema.]tabela
/// [AS] [alias]`, com vírgulas para mais de uma. Palavra-chave no lugar do alias (`WHERE`, `ON`,
/// `SET`, `LEFT`…) não é alias.
fn table_refs(toks: &[Tok], current_schema: Option<&str>) -> Vec<TableRef> {
    let word = |i: usize| match toks.get(i).map(|t| t.token) {
        Some(Token::Word(w)) => Some(w),
        _ => None,
    };
    let mut refs = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let starts = word(i).is_some_and(|w| {
            w.quote_style.is_none()
                && ["FROM", "JOIN", "UPDATE", "INTO"]
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case(&w.value))
        });
        i += 1;
        if !starts {
            continue;
        }
        while let Some(first) = word(i) {
            let (schema, table, next) = match (toks.get(i + 1).map(|t| t.token), word(i + 2)) {
                (Some(Token::Period), Some(second)) => {
                    (first.value.clone(), second.value.clone(), i + 3)
                }
                // `schema.` ainda sem a tabela (está sendo digitada): não é referência.
                (Some(Token::Period), None) => break,
                _ => match current_schema {
                    Some(s) => (s.to_owned(), first.value.clone(), i + 1),
                    None => break,
                },
            };
            let mut j = next;
            if word(j).is_some_and(|w| w.value.eq_ignore_ascii_case("AS")) {
                j += 1;
            }
            let alias = word(j)
                .filter(|w| w.quote_style.is_some() || !is_keyword(&w.value))
                .map(|w| w.value.clone());
            if alias.is_some() {
                j += 1;
            }
            refs.push(TableRef {
                schema,
                table,
                alias,
            });
            if matches!(toks.get(j).map(|t| t.token), Some(Token::Comma)) {
                i = j + 1;
                continue;
            }
            i = j;
            break;
        }
    }
    refs
}

/// A keyword na caixa do prefixo: `sel` → `select`, `SEL` → `SELECT`; prefixo vazio → minúsculas.
fn keyword_case(keyword: &str, prefix: &str) -> String {
    if prefix.chars().next().is_some_and(char::is_uppercase) {
        keyword.to_owned()
    } else {
        keyword.to_lowercase()
    }
}

/// O nome como entra no SQL: entre aspas quando não é `[a-z_][a-z0-9_$]*` ou é keyword.
pub fn ident(name: &str) -> String {
    let plain = name.chars().enumerate().all(|(i, c)| {
        c == '_' || c.is_ascii_lowercase() || (i > 0 && (c.is_ascii_digit() || c == '$'))
    });
    if plain && !name.is_empty() && !is_keyword(name) {
        name.to_owned()
    } else {
        hy_dbplus_catalog::data::quote_ident(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, data_type: &str, pk: bool) -> Column {
        Column {
            name: name.into(),
            data_type: data_type.into(),
            is_primary_key: pk,
            ..Column::default()
        }
    }

    fn table(schema: &str, name: &str, kind: TableKind) -> Table {
        Table {
            schema: schema.into(),
            name: name.into(),
            kind,
            comment: None,
        }
    }

    struct Fixture {
        schemas: Vec<Schema>,
        objects: HashMap<String, (Vec<Table>, Vec<Function>)>,
        columns: HashMap<(String, String), Vec<Column>>,
    }

    fn fixture() -> Fixture {
        let mut objects = HashMap::new();
        objects.insert(
            "public".to_owned(),
            (
                vec![
                    table("public", "clientes", TableKind::Table),
                    table("public", "pedido", TableKind::Table),
                    table("public", "v_pedido", TableKind::View),
                    table("public", "Loja", TableKind::Table),
                ],
                vec![Function {
                    schema: "public".into(),
                    name: "dobro".into(),
                    arguments: "x integer".into(),
                    returns: "integer".into(),
                    kind: "function".into(),
                }],
            ),
        );
        let mut columns = HashMap::new();
        columns.insert(
            ("public".to_owned(), "clientes".to_owned()),
            vec![
                col("id", "integer", true),
                col("nome", "text", false),
                col("order", "integer", false),
            ],
        );
        columns.insert(
            ("public".to_owned(), "pedido".to_owned()),
            vec![
                col("id", "integer", true),
                col("cliente_id", "integer", false),
            ],
        );
        Fixture {
            schemas: vec![
                Schema {
                    name: "public".into(),
                },
                Schema { name: "app".into() },
            ],
            objects,
            columns,
        }
    }

    impl Fixture {
        fn view(&self) -> CatalogView<'_> {
            CatalogView {
                current_schema: Some("public"),
                schemas: &self.schemas,
                objects: &self.objects,
                columns: &self.columns,
            }
        }
    }

    fn labels(c: &Completion) -> Vec<&str> {
        c.items.iter().map(|i| i.label.as_str()).collect()
    }

    fn at(text: &str, catalog: &CatalogView) -> Option<Completion> {
        // O cursor é o `|` do texto.
        let cursor = text.find('|').unwrap();
        let text = text.replacen('|', "", 1);
        complete(&Analysis::new(&text), cursor, catalog)
    }

    #[test]
    fn depois_de_from_tabelas_views_e_schemas_pelo_prefixo() {
        let f = fixture();
        let c = at("select * from cl|", &f.view()).unwrap();
        assert_eq!(c.replace, 14..16);
        assert_eq!(labels(&c), ["clientes"]);
        assert_eq!(c.items[0].detail, "tabela · public");
        let c = at("select * from |", &f.view()).unwrap();
        assert_eq!(c.replace, 14..14);
        assert_eq!(
            labels(&c),
            [
                "clientes", "pedido", "v_pedido", "\"Loja\"", "public", "app"
            ]
        );
        assert_eq!(c.items[2].kind, ItemKind::View);
        // `contém` depois de `começa com`: "ped" acha pedido antes de v_pedido.
        let c = at("select * from ped|", &f.view()).unwrap();
        assert_eq!(labels(&c), ["pedido", "v_pedido"]);
        let c = at("delete from |", &f.view()).unwrap();
        assert!(labels(&c).contains(&"clientes"));
        let c = at("insert into |", &f.view()).unwrap();
        assert!(labels(&c).contains(&"clientes"));
    }

    #[test]
    fn alias_e_tabela_com_ponto_listam_as_colunas_e_schema_lista_as_tabelas() {
        let f = fixture();
        let c = at("select c.| from clientes c", &f.view()).unwrap();
        assert_eq!(labels(&c), ["id", "nome", "\"order\""]);
        assert_eq!(c.items[0].detail, "integer · chave");
        assert_eq!(c.replace, 9..9);
        let c = at(
            "select c.no| from clientes as c join pedido p on p.cliente_id = c.id",
            &f.view(),
        )
        .unwrap();
        assert_eq!(labels(&c), ["nome"]);
        assert_eq!(c.replace, 9..11);
        let c = at("select p.| from clientes c, pedido p", &f.view()).unwrap();
        assert_eq!(labels(&c), ["id", "cliente_id"]);
        let c = at("select clientes.| from clientes", &f.view()).unwrap();
        assert_eq!(labels(&c), ["id", "nome", "\"order\""]);
        // Tabela do schema atual sem estar no FROM: também vale (UPDATE/WHERE escritos antes do FROM).
        let c = at("select pedido.|", &f.view()).unwrap();
        assert_eq!(labels(&c), ["id", "cliente_id"]);
        let c = at("select * from public.|", &f.view()).unwrap();
        assert_eq!(labels(&c), ["clientes", "pedido", "v_pedido", "\"Loja\""]);
        assert_eq!(c.items[0].detail, "tabela");
        // Schema conhecido sem objetos carregados: pede ao engine.
        let c = at("select * from app.|", &f.view()).unwrap();
        assert!(c.items.is_empty());
        assert_eq!(c.needs, [Need::Schema("app".into())]);
        // Qualificador desconhecido: nada.
        assert!(at("select x.| from clientes c", &f.view()).is_none());
        // Schema qualificado no FROM resolve o alias.
        let c = at("select l.| from public.\"Loja\" l", &f.view()).unwrap();
        assert!(c.items.is_empty());
        assert_eq!(
            c.needs,
            [Need::Columns {
                schema: "public".into(),
                table: "Loja".into()
            }]
        );
    }

    #[test]
    fn no_resto_colunas_aliases_keywords_tabelas_e_funcoes_na_ordem_e_na_caixa_do_prefixo() {
        let f = fixture();
        let c = at("select | from clientes c", &f.view()).unwrap();
        let l = labels(&c);
        assert_eq!(&l[..4], ["id", "nome", "\"order\"", "c"]);
        assert_eq!(c.items[3].kind, ItemKind::Alias);
        assert_eq!(c.items[3].detail, "public.clientes");
        assert_eq!(c.items[4].kind, ItemKind::Keyword);
        assert_eq!(c.items[4].label, "abort");
        assert_eq!(c.items.len(), MAX_ITEMS);
        let c = at("select n| from clientes c", &f.view()).unwrap();
        assert_eq!(labels(&c)[..2], ["nome", "natural"]);
        let c = at("SEL|", &f.view()).unwrap();
        assert_eq!(labels(&c), ["SELECT"]);
        let c = at("sel|", &f.view()).unwrap();
        assert_eq!(labels(&c), ["select"]);
        let c = at("select dob|", &f.view()).unwrap();
        assert_eq!(labels(&c), ["dobro"]);
        assert_eq!(c.items[0].detail, "(x integer) → integer");
        // Keyword no lugar do alias não é alias; a coluna `order` sai com aspas.
        let c = at("select | from clientes where id = 1", &f.view()).unwrap();
        assert!(!c.items.iter().any(|i| i.kind == ItemKind::Alias));
        assert!(labels(&c).contains(&"\"order\""));
        // Duas tabelas com `id`: uma vez só; e as colunas da tabela do FROM que ainda não chegaram
        // viram pedido.
        let c = at("select id| from clientes c, pedido p, \"Loja\"", &f.view()).unwrap();
        assert_eq!(labels(&c).iter().filter(|l| **l == "id").count(), 1);
        assert_eq!(
            c.needs,
            [Need::Columns {
                schema: "public".into(),
                table: "Loja".into()
            }]
        );
        // Buffer vazio e cursor no começo: keywords.
        let c = at("|", &f.view()).unwrap();
        assert_eq!(c.items[0].label, "abort");
        assert_eq!(c.replace, 0..0);
    }

    #[test]
    fn dentro_de_string_comentario_aspas_ou_texto_que_nao_tokenizou_nao_sugere() {
        let f = fixture();
        assert!(at("select 'ab|c' from clientes", &f.view()).is_none());
        assert!(at("select 'abc|", &f.view()).is_none());
        assert!(at("select 1 -- co|m", &f.view()).is_none());
        assert!(at("select 1 -- com|", &f.view()).is_none());
        assert!(at("select 1 /* co|m */", &f.view()).is_none());
        assert!(at("select \"no|me\" from clientes", &f.view()).is_none());
        assert!(at("select $$ab|", &f.view()).is_none());
        // Depois da string fechada e na linha seguinte ao comentário: sugere.
        assert!(at("select 'abc' |", &f.view()).is_some());
        assert!(at("select 1 -- com\n|", &f.view()).is_some());
        // Só o statement do cursor conta para o FROM.
        let c = at("select * from clientes c;\nselect c.|", &f.view());
        assert!(c.is_none());
    }

    #[test]
    fn ident_poe_aspas_so_quando_precisa() {
        assert_eq!(ident("nome"), "nome");
        assert_eq!(ident("nome_2$"), "nome_2$");
        assert_eq!(ident("Loja"), "\"Loja\"");
        assert_eq!(ident("order"), "\"order\"");
        assert_eq!(ident("2x"), "\"2x\"");
        assert_eq!(ident("a b"), "\"a b\"");
        assert_eq!(ident(""), "\"\"");
    }
}
