<!-- FEAT_EM_ANDAMENTO — docs criados em 22/10/2026; próximo: discutir o desenho e começar pela crate sql -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-010-editor-sql/context.index.md

# Contexto — FEAT-010 editor-sql

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-010 |
| **Slug** | editor-sql |
| **Origem** | BRIEF §3.7 e marco M5; captura do TablePlus (aba SQL Query, Query 1/2/3) |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-22 |
| **Atualizado em** | 2026-10-22 |

## TL;DR

Aba "SQL Query" por banco ao lado das abas de tabela: editor com highlight e números de linha, Run Current
(⌘↩) do statement sob o cursor, Run All (⌘⇧↩) parando no erro, limite de linhas por leitura, Beautify (⌘I),
Cancelar pelo CancelToken; resultado em sub-abas `Query N` com grid só leitura, mensagem ou erro. Split e
highlight pelo tokenizador do `sqlparser`; formatação pelo `sqlformat`. Autocomplete é o M6.

## Leia nesta ordem

```
1. 00-brief/brief.md          → escopo por camada (sql, driver, engine, ui), o que fica de fora
2. 10-prd/acceptance.md       → o que precisa ser verdade por crate, e o que é do Jean
3. 50-dev/impl-notes.md       → decisões (limite por leitura, um statement por vez, cancel, abas heterogêneas) e plano
4. docs/features/FEAT-008-bancos-e-abas/ → as abas por banco que ganham a aba de SQL
```

## Onde está o quê (quando existir)

```
crates/sql/src/split.rs           split, statement_at
crates/sql/src/highlight.rs       highlight (spans por token)
crates/sql/src/format.rs          beautify (sqlformat)
crates/driver/src/lib.rs          Connection::execute(sql, limit) -> Outcome; cancel
crates/engine/src/query.rs        run_sql, cancel_sql, histórico em memória; eventos Sql*
crates/ui-egui/src/app.rs         Tab::Table | Tab::Query, QueryTab
crates/ui-egui/src/editor.rs      editor com highlight, gutter, status, controles
crates/ui-egui/src/results.rs     sub-abas Query N, grid só leitura, mensagem, erro
crates/ui-egui/src/tabs.rs        + SQL (⌘T)
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. sql/split, driver, engine/session.rs e engine/query.rs são sensíveis: reviewer antes de cada commit.
3. Commit só com o "pode commitar" do Jean, na ordem: docs; sql; driver; engine; ui (editor); ui (resultados).
4. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
