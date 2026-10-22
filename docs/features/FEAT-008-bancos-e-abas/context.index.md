<!-- FEAT_EM_ANDAMENTO — docs criados em 22/10/2026; próximo: engine::close_database -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-008-bancos-e-abas/context.index.md

# Contexto — FEAT-008 bancos-e-abas

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-008 |
| **Slug** | bancos-e-abas |
| **Origem** | pedido do Jean (22/10/2026): "abrir mais de um DB e mais de uma tabela por DB ao mesmo tempo"; captura do TablePlus com a faixa de bancos e as abas |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-22 |
| **Atualizado em** | 2026-10-22 |

## TL;DR

Faixa de bancos abertos à esquerda (conexão + banco, com a cor do ambiente), árvore do banco selecionado e
abas por tabela no centro, cada aba com página, seleção e pendência próprias. ⌘K abre um banco a mais; ×
fecha aba ou banco, com confirmação se houver pendência. Engine só ganha `close_database`.

## Leia nesta ordem

```
1. 00-brief/brief.md          → problema, objetivo, escopo (estado, faixa, ⌘K, abas, edição por aba, engine)
2. 10-prd/acceptance.md       → o que precisa ser verdade, e o que é do Jean
3. 50-dev/impl-notes.md       → decisões (modelo do TablePlus, uma aba por tabela, chave do workspace) e plano
4. docs/features/FEAT-007-edicao-em-lote/ → a edição que passa a viver por aba
```

## Onde está o quê (quando existir)

```
crates/engine/src/session.rs      close_database, DatabaseClosed
crates/ui-egui/src/app.rs         Workspace por (conexão, banco), rail/active, tabs/active_tab, roteamento
crates/ui-egui/src/rail.rs        a faixa de bancos abertos
crates/ui-egui/src/tabs.rs        a barra de abas, ×, ⌘W, confirmação ao fechar
crates/ui-egui/src/workspace.rs   três colunas: faixa, árvore, centro com abas
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. engine/session.rs é sensível: reviewer antes do commit; a UI também passa pelo reviewer (diff grande).
3. Commit só com o "pode commitar" do Jean, na ordem: docs; engine; ui (faixa); ui (abas).
4. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
