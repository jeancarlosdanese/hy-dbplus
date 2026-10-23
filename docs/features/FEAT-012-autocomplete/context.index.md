<!-- FEAT_PLANEJADA — docs criados em 23/10/2026; código não iniciado; vem depois da FEAT-013 -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-012-autocomplete/context.index.md

# Contexto — FEAT-012 autocomplete

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-012 |
| **Slug** | autocomplete |
| **Origem** | BRIEF M6 (§3.7, §4 item 9); desenho fechado com o Jean em 23/10/2026 |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-23 |
| **Atualizado em** | 2026-10-23 |

## TL;DR

Popup abaixo do cursor com keywords, schemas, tabelas, views, colunas, aliases e funções, pelo statement sob
o cursor e o catálogo já carregado (pedindo colunas e schemas ao engine quando faltam). Abre ao digitar e por
⌃Space; Tab/Enter aceitam; Esc fecha. Motor puro na crate `sql`, com cache de tokenização compartilhado com o
highlight e o Run Current.

## Leia nesta ordem

```
1. 00-brief/brief.md          → motor, contextos, popup, o que fica de fora
2. 10-prd/acceptance.md       → o que precisa ser verdade, e o que é do Jean
3. 50-dev/impl-notes.md       → as cinco decisões do Jean, plano e riscos
4. docs/features/FEAT-010-editor-sql/ → o editor e o tokenizador que isto estende
```

## Onde está o quê (quando existir)

```
crates/sql/src/analysis.rs        Analysis: tokens, statements e spans do buffer, uma vez
crates/sql/src/complete.rs        complete, CatalogView, Item, resolução de alias
crates/sql/src/keywords.rs        lista curada de keywords
crates/ui-egui/src/complete.rs    popup, teclas, inserção, pedidos ao engine
crates/ui-egui/src/editor.rs      chama o popup; QueryTab.analysis
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. split.rs é sensível (o cache o toca): reviewer antes de cada commit.
3. Commit só com o "pode commitar" do Jean, na ordem: docs; refactor cache; sql; ui.
4. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
