<!-- FEAT_IMPLEMENTADA — na árvore em 23/10/2026 (docs a810dc7; catalog e ui aguardando commit); falta push e conferência do Jean -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-013-gerar-sql-pela-arvore/context.index.md

# Contexto — FEAT-013 gerar-sql-pela-arvore

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-013 |
| **Slug** | gerar-sql-pela-arvore |
| **Origem** | pedido do Jean (23/10/2026): botão direito na tabela com modelos de SQL; decidido antes do autocomplete (FEAT-012) |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-23 |
| **Atualizado em** | 2026-10-23 |

## TL;DR

Botão direito numa tabela ou view da árvore: SELECT, SELECT count, INSERT, UPDATE, DELETE, TRUNCATE, DROP
CASCADE e CREATE, montados pelo `catalog` com as colunas e a chave primária, entram no fim da aba SQL ativa
(ou numa nova) com o cursor no statement. Nada roda pelo menu.

## Leia nesta ordem

```
1. 00-brief/brief.md          → os modelos, o destino do texto, o que fica de fora
2. 10-prd/acceptance.md       → o que precisa ser verdade, e o que é do Jean
3. 50-dev/impl-notes.md       → decisões (catalog monta; nada roda; colunas sob demanda) e plano
4. docs/features/FEAT-009-structure-e-index/ e FEAT-010-editor-sql/ → colunas, DDL e abas SQL que isto usa
```

## Onde está o quê (quando existir)

```
crates/catalog/src/generate.rs    Template, template_sql, valores de exemplo por tipo
crates/ui-egui/src/tree.rs        context_menu na linha da tabela/view
crates/ui-egui/src/app.rs         generate_sql, Workspace.pending_sql, conclusão em ColumnsLoaded/StructureLoaded, insert_sql
crates/ui-egui/src/editor.rs      QueryTab.pending_cursor aplicado ao TextEdit com foco
crates/sql/src/lib.rs             teste: cada modelo é um statement só a partir do cursor
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. Reviewer antes de cada commit (o SQL gerado inclui DROP CASCADE).
3. Commit só com o "pode commitar" do Jean, na ordem: docs; catalog; ui.
4. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
