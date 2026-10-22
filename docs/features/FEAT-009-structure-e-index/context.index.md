<!-- FEAT_EM_ANDAMENTO — implementada em 22/10/2026; falta push, CI e conferência do Jean -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-009-structure-e-index/context.index.md

# Contexto — FEAT-009 structure-e-index

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-009 |
| **Slug** | structure-e-index |
| **Origem** | BRIEF §3.5, §3.6 e marco M4 |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-22 |
| **Atualizado em** | 2026-10-22 |

## TL;DR

Sub-abas Data / Structure / Index em cada aba de tabela. Structure: colunas (tipo, nulo, default, comentário,
chave), restrições (PK, unique, check, FK) e o botão DDL com o `CREATE TABLE` montado pelo catálogo. Index:
índices como o TablePlus. Somente leitura; engine cacheia a estrutura por tabela e o ↻ esvazia.

## Leia nesta ordem

```
1. 00-brief/brief.md          → modelo do catálogo, DDL, engine, UI, o que fica de fora
2. 10-prd/acceptance.md       → o que precisa ser verdade, e o que é do Jean
3. 50-dev/impl-notes.md       → decisões (DDL no catálogo, índices de restrição, sub-aba por aba) e plano
4. docs/features/FEAT-008-bancos-e-abas/ → a aba de tabela que ganha as sub-abas
```

## Onde está o quê

```
crates/catalog/src/model.rs       Column (comentário, identidade, gerada), Constraint, Index, TableStructure
crates/catalog/src/structure.rs   structure(conn, schema, table): colunas, restrições, índices, view
crates/catalog/src/ddl.rs         table_ddl(&TableStructure)
crates/engine/src/session.rs      load_structure, StructureLoaded, cache Database.structures
crates/ui-egui/src/structure.rs   sub-aba Structure e a caixa do DDL
crates/ui-egui/src/index.rs       sub-aba Index
crates/ui-egui/src/data.rs        rodapé com as sub-abas; TabView no DataTab (app.rs)
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. engine/session.rs é sensível: reviewer antes do commit; catálogo e UI também passam pelo reviewer.
3. Commit só com o "pode commitar" do Jean, na ordem: docs; catalog; engine; ui.
4. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
