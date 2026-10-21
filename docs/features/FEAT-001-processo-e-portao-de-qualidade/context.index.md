<!-- FEAT_EM_ANDAMENTO — arquivos criados em 21/10/2026; falta o Jean ligar plugin, pre-commit, ferramentas e fazer o primeiro commit -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-001-processo-e-portao-de-qualidade/context.index.md

# Contexto — FEAT-001 processo-e-portao-de-qualidade

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-001 |
| **Slug** | processo-e-portao-de-qualidade |
| **Origem** | hyberica-claude-kit (plugin `process@hyberica` 0.4.0) e portão do HyVox (FEAT-219) |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-21 |
| **Atualizado em** | 2026-10-21 |

## TL;DR

Antes de qualquer código Rust, o hy-dbplus nasce com o processo da Hyberica: o plugin `process@hyberica`
(trava de comandos, "pode commitar", revisor obrigatório em mudança sensível), um portão de qualidade para Rust
(fmt, clippy com lints de "never panic", testes, cargo-deny, gitleaks, coerência dos CLAUDE.md), o pre-commit do
kit e o CI no GitHub Actions. Sem linha de base: o projeto é estrito desde o primeiro commit.

## Leia nesta ordem

```
1. 00-brief/brief.md          → problema, objetivo, escopo
2. 10-prd/acceptance.md       → o que precisa ser verdade, e o que é do Jean
3. 50-dev/impl-notes.md       → estado atual, decisões e log
4. CLAUDE.md (raiz)           → seção Processo (dona das regras de commit e revisão)
5. ~/dados/workspaces-clientes/hyberica-claude-kit/README.md → como o plugin funciona
```

## Onde está o quê

```
.claude/settings.json                 marketplace hyberica + plugin ligado + allow de cargo
.claude/process.json                  o que é protegido e sensível neste projeto
.githooks/pre-commit                  cópia de templates/githooks/pre-commit do kit (não editar aqui)
scripts/quality-gate.sh               o portão (--fast, --full, --all --base)
scripts/quality-gate-tools.sh         instala cargo-deny e gitleaks em tmp/tools/bin
scripts/quality_claude_docs.py        verificador de coerência dos CLAUDE.md (cópia do kit, raízes Rust) + _test.py
.gitignore                            target/, tmp/, .env*, chaves, settings.local.json
deny.toml                             licenças permitidas, crates proibidas, fontes
.github/workflows/quality-gate.yml    CI: portão em máquina limpa + build macOS/Windows
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. Defeito no plugin ou no pre-commit: descrever e parar; o conserto é no kit, nunca aqui.
3. Mudança no portão ou no process.json é arquivo protegido: caixa de aprovação, e dizer antes o que ela faz.
4. Commit só com o "pode commitar" do Jean; push, hooksPath, ferramentas e repositório no GitHub são dele.
5. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
