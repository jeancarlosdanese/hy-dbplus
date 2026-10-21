<!-- FEAT_EM_ANDAMENTO — docs criados em 21/10/2026; próximo: workspace com lints e cascas -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-002-workspace-e-janela-vazia/context.index.md

# Contexto — FEAT-002 workspace-e-janela-vazia

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-002 |
| **Slug** | workspace-e-janela-vazia |
| **Origem** | BRIEF §7, marco M0 |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-21 |
| **Atualizado em** | 2026-10-21 |

## TL;DR

O M0 do BRIEF: workspace Cargo com as seis crates do §5 vazias e os dois apps, lints de never-panic como regra
do compilador, `cargo xtask layers` provando que nada abaixo de `ui-egui` depende de `egui`, e o app desktop
abrindo uma janela vazia com o tema. A partir daqui o portão de qualidade roda inteiro (fmt, clippy, testes,
cargo-deny, layers).

## Leia nesta ordem

```
1. 00-brief/brief.md          → problema, objetivo, escopo, o que fica de fora
2. 10-prd/acceptance.md       → o que precisa ser verdade, e o que é do Jean
3. 50-dev/impl-notes.md       → estado atual, decisões (camadas, lints, dependências) e log
4. docs/BRIEF.md §5 a §7      → arquitetura, stack e marcos
5. CLAUDE.md (raiz)           → regras de engenharia e processo
```

## Onde está o quê (quando existir)

```
Cargo.toml                    workspace: membros, [workspace.package], [workspace.dependencies], [workspace.lints]
clippy.toml                   unwrap/expect/panic/indexação liberados só em teste
.cargo/config.toml            alias `cargo xtask`
crates/<nome>/                driver, catalog, sql, engine, automation, ui-egui (cascas no M0)
apps/hy-dbplus/               app desktop (eframe): cria a janela, aplica o tema
apps/hy-dbplus-cli/           CLI: uso e versão no M0
xtask/src/layers.rs           tabela de camadas e motor de regras (testado sem cargo)
xtask/src/main.rs             `cargo xtask layers`
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. Cargo.toml, Cargo.lock, .cargo/, xtask/ e apps/*/src/ são sensíveis: process:reviewer antes de cada commit.
3. Dependência nova: justificar no impl-notes, conferir licença no deny.toml, Cargo.lock no mesmo commit.
4. Commit só com o "pode commitar" do Jean, na ordem: docs; workspace; xtask layers; janela.
5. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
