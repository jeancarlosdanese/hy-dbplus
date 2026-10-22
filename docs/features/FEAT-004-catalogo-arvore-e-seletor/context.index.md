<!-- FEAT_EM_ANDAMENTO — driver, catalog, engine e UI em 21/10/2026; falta push, CI e a conferência do Jean -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-004-catalogo-arvore-e-seletor/context.index.md

# Contexto — FEAT-004 catalogo-arvore-e-seletor

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-004 |
| **Slug** | catalogo-arvore-e-seletor |
| **Origem** | BRIEF §7, marco M2 |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-21 |
| **Atualizado em** | 2026-10-21 |

## TL;DR

O M2 do BRIEF: `query_text` no driver, a crate `catalog` lendo `pg_catalog` (schemas, tabelas, views, funções,
colunas com tipo e PK), o engine abrindo uma conexão por banco selecionado com cache do catálogo, e a UI com a
paleta ⌘K de bancos, a árvore Functions/Tables expansível até as colunas e o seletor de schema no rodapé.

## Leia nesta ordem

```
1. 00-brief/brief.md          → problema, objetivo, escopo, o que fica de fora
2. 10-prd/acceptance.md       → o que precisa ser verdade, e o que é do Jean
3. 50-dev/impl-notes.md       → estado atual, decisões (texto, conexão por banco, lazy, kittest) e log
4. docs/BRIEF.md §3.2, §3.3   → as telas; §5 → camadas
5. CLAUDE.md (raiz)           → regras de engenharia
```

## Onde está o quê

```
crates/driver/src/lib.rs          Connection::query_text, TextRows
crates/catalog/src/model.rs       Schema, Table, TableKind, Column, Function
crates/catalog/src/postgres.rs    carregadores sobre pg_catalog (schemas, tables, functions, columns)
crates/engine/src/session.rs      select_database, load_schema, load_columns, refresh; cache por banco dentro do Slot; eventos novos
crates/ui-egui/src/palette.rs     ⌘K: lista de bancos com busca
crates/ui-egui/src/tree.rs        árvore Functions/Tables, colunas, seletor de schema, busca
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. driver, sessão/credenciais do engine, Cargo.toml e Cargo.lock são sensíveis: reviewer antes de cada commit.
3. Teste com servidor fica atrás de HY_DBPLUS_TEST_PG e cria/apaga o próprio schema; nunca toca dados existentes.
4. Commit só com o "pode commitar" do Jean, na ordem: docs; driver + catalog; engine; ui.
5. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
