# @file: docs/features/FEAT-012-autocomplete/00-brief/brief.md

# FEAT-012 — Autocomplete por schema no editor SQL

## Problema

O editor SQL (FEAT-010) executa, colore e formata, mas não sugere nada: o usuário digita nome de
tabela e de coluna de cabeça ou olha na árvore. "Escrever SQL com autocomplete" é um dos dois usos
principais do produto (BRIEF §1, §3.7, M6).

## Objetivo

Ao digitar no editor, um popup abaixo do cursor sugere palavras-chave, schemas, tabelas, views,
colunas e aliases do statement sob o cursor, com o catálogo já carregado na árvore e pedindo ao
engine o que falta. `select * from cl` sugere `clientes`; `c.` lista as colunas de `clientes c`.
Tab ou Enter aceitam; Esc fecha; ⌃Space abre à força. Desenho fechado com o Jean em 23/10/2026.

## Escopo (Must)

1. **`sql::complete`** (camada 2, puro, testado): `complete(buffer, cursor_byte, catalog: &CatalogView)
   -> Option<Completion { replace: Range<usize>, items: Vec<Item { label, kind, detail }> }>`, onde
   `CatalogView` é uma visão emprestada do que a UI tem (`current_schema`, schemas, tabelas por schema,
   colunas por `(schema, tabela)`) e `kind` é `Keyword | Schema | Table | View | Column | Alias |
   Function`. Contexto pelos tokens do statement sob o cursor (`statement_at` + tokenizador):
   - prefixo = identificador em digitação (pode ser vazio logo depois de `.` ou de espaço após
     `FROM`/`JOIN`);
   - `alias.` ou `tabela.` → colunas da tabela; alias resolvido em `FROM`, `JOIN`, `UPDATE`, `INSERT
     INTO` e `DELETE FROM` do mesmo statement (`clientes c`, `clientes as c`, `public.clientes c`);
   - `schema.` → tabelas e views do schema;
   - depois de `FROM`, `JOIN`, `INTO`, `UPDATE`, `TABLE` → tabelas e views do schema atual e schemas;
   - resto → colunas das tabelas do statement, aliases, keywords, tabelas do schema atual, funções;
   - dentro de string, comentário ou identificador entre aspas: `None`;
   - ordem: começa com o prefixo antes de contém; empate por nome; caixa ignorada; no máximo 50;
   - keyword segue a caixa do prefixo (`sel` → `select`, `SEL` → `SELECT`); prefixo vazio → minúsculas;
   - nome que precisa de aspas entra com aspas (`quote_ident` do catálogo).
   Lista de keywords curada (~80 do PostgreSQL: DML, DDL comum, cláusulas, operadores por extenso,
   funções de agregação mais usadas), em `keywords.rs`.
2. **Cache de tokenização** (pendência do M5): `sql::Analysis::new(buffer)` guarda tokens, statements e
   spans uma vez e os expõe (`statements`, `spans`, `statement_at`, `tokens`); as funções livres
   `split`/`highlight`/`statement_at` continuam com o mesmo contrato por cima das mesmas partes;
   `complete` recebe a análise. A `QueryTab` guarda a análise do buffer atual e refaz só quando o texto
   muda (`Analysis::refresh`).
3. **`ui-egui`** (`complete.rs`): popup ancorado no retângulo do cursor (galley do `TextEdit`), aberto
   automaticamente ao digitar o primeiro caractere de identificador ou logo depois de `.`, ou por
   ⌃Space; ↑/↓ movem, Tab e Enter aceitam (substituem `replace` pelo `label`, movem o cursor para o fim e
   fecham), Esc fecha, continuar digitando filtra, qualquer outra tecla de edição fecha. Teclas
   consumidas antes do `TextEdit` como os atalhos de hoje. Tabela do statement sem colunas no
   `Workspace` → `load_columns`; `schema.` sem objetos → `load_schema`; o evento reabre a lista. Até 10
   itens visíveis com rolagem; cada item mostra o tipo (ícone ou texto curto) e o detalhe (tipo da
   coluna, schema da tabela).
4. **Testes**: `sql` puro (cada contexto; alias com e sem `AS`; schema qualificado; string aberta; caixa;
   aspas; limite de 50; buffer com erro de tokenização antes do cursor); UI com `egui_kittest`: digitar
   `sel` + Enter vira `select`; `from cl` sugere `clientes` e Tab completa; `c.` lista colunas e pede ao
   engine quando faltam; Esc fecha e Enter volta a inserir nova linha; dentro de string não abre; ⌃Space
   abre com prefixo vazio.

## Won't

- Snippets e geração de SQL por objeto (FEAT-013, menu da árvore); `JOIN … ON` pela FK (M7, com a
  navegação por FK no grid); documentação da keyword; sugestão dentro de corpo de função `$$…$$`;
  parâmetros de função; `search_path` real do servidor (aproximado pelo schema selecionado na árvore);
  objetos de `pg_catalog` e `information_schema`.

## Restrições

- Sem dependência nova: o tokenizador do `sqlparser` já está na crate. `crates/sql/src/complete.rs` não
  é sensível pelos padrões, mas `split.rs` e `statement_at` são, e o cache os toca: reviewer. UI pelo
  tamanho e pelas teclas consumidas (Enter e Tab no editor).
- Commits: `docs`; `refactor(sql,ui)` cache de tokenização; `feat(sql)` motor; `feat(ui)` popup.
