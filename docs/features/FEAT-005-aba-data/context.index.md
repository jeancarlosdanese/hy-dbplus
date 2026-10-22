<!-- FEAT_EM_ANDAMENTO — docs criados em 21/10/2026; próximo: catalog (stats e SQL da página) -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-005-aba-data/context.index.md

# Contexto — FEAT-005 aba-data

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-005 |
| **Slug** | aba-data |
| **Origem** | BRIEF §7, marco M3 |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-21 |
| **Atualizado em** | 2026-10-21 |

## TL;DR

O M3 do BRIEF: abrir uma tabela pela árvore e ver as linhas num grid virtualizado (`egui_extras`), paginado por
`LIMIT/OFFSET`, ordenável pelo cabeçalho, com o número da linha, `NULL` em itálico e a linha selecionada
destacada; rodapé com contador e paginação; painel direito com tamanhos, comentário e os campos da linha.

## Leia nesta ordem

```
1. 00-brief/brief.md          → problema, objetivo, escopo, o que fica de fora
2. 10-prd/acceptance.md       → o que precisa ser verdade, e o que é do Jean
3. 50-dev/impl-notes.md       → estado atual, decisões (LIMIT/OFFSET, count, grid, árvore) e log
4. docs/BRIEF.md §3.4         → a tela; §9 decisão 3 → paginação
5. CLAUDE.md (raiz)           → regras de engenharia
```

## Onde está o quê (quando existir)

```
crates/catalog/src/data.rs        TableStats, Order, page_sql, count_sql, stats
crates/engine/src/session.rs      load_stats, load_page, Page; eventos StatsLoaded e PageLoaded
crates/ui-egui/src/data.rs        a aba Data: grid, rodapé, painel direito
crates/ui-egui/src/tree.rs        nome da tabela abre a aba; seta expande
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. session.rs, Cargo.toml e Cargo.lock são sensíveis: reviewer antes de cada commit.
3. Identificador no SQL sempre entre aspas duplas dobradas; limit/offset são inteiros formatados.
4. Commit só com o "pode commitar" do Jean, na ordem: docs; catalog; engine; ui.
5. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
