<!-- FEAT_EM_ANDAMENTO — implementada em 22/10/2026; falta push, CI e conferência do Jean -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-006-painel-de-registro/context.index.md

# Contexto — FEAT-006 painel-de-registro

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-006 |
| **Slug** | painel-de-registro |
| **Origem** | conferência do Jean no M3 (22/10/2026); tela do TablePlus com Form e "Multiple values" |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-22 |
| **Atualizado em** | 2026-10-22 |

## TL;DR

Painel de registro da aba Data: seleção múltipla no grid, tamanhos numa linha compacta acima do grid, painel
recolhido por padrão que abre na seleção, com abas Form (nome, tipo, valor; "Múltiplos valores") e JSON
(tipos pelo catálogo, botão Copiar). Só leitura; a edição é a FEAT-007.

## Leia nesta ordem

```
1. 00-brief/brief.md          → problema, objetivo, escopo, o que fica de fora
2. 10-prd/acceptance.md       → o que precisa ser verdade, e o que é do Jean
3. 50-dev/impl-notes.md       → estado atual, decisões (JSON no engine, tipos pelo catálogo, seleção) e log
4. docs/features/FEAT-005-aba-data/ → a aba Data que este painel completa
```

## Onde está o quê

```
crates/engine/src/record.rs       to_json(columns, types, rows)
crates/ui-egui/src/data.rs        seleção múltipla, linha de tamanhos, rodapé "N of M rows selected"
crates/ui-egui/src/record.rs      o painel: abas Form e JSON, "Múltiplos valores", Copiar
crates/ui-egui/src/app.rs         DataTab.selected_rows, PanelMode, select_row, abertura automática do painel
crates/engine/src/session.rs      load_page emite ColumnsLoaded antes de PageLoaded
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. Cargo.toml/Cargo.lock (serde_json na engine) são sensíveis: reviewer antes do commit do engine.
3. Commit só com o "pode commitar" do Jean, na ordem: engine; ui; docs (uma rodada do reviewer a menos).
4. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
