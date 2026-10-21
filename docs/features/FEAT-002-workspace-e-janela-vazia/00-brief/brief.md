# @file: docs/features/FEAT-002-workspace-e-janela-vazia/00-brief/brief.md

# FEAT-002 — Workspace e janela vazia (M0)

## Problema

O repositório tem processo e portão, mas nenhuma linha de Rust: o portão pula fmt, clippy, testes e cargo-deny
por falta de `Cargo.toml`, e as regras de engenharia do `CLAUDE.md` (never panic, layering) ainda não são
verificadas por ninguém. Sem o esqueleto do workspace, cada marco seguinte começaria decidindo estrutura em vez
de entregar comportamento.

## Objetivo

Entregar o M0 do [BRIEF §7](../../../BRIEF.md): o workspace Cargo com as crates do BRIEF §5 (vazias), os lints
de never-panic como regra do compilador, o `cargo xtask layers` provando que nada abaixo da UI depende de
`egui`, e o app desktop abrindo uma janela vazia com o tema. A partir daqui o portão roda inteiro.

## Escopo (Must)

1. **Workspace** (`Cargo.toml` da raiz): `resolver = "3"`, edição 2024, `license = "MIT"` em
   `[workspace.package]` herdado por toda crate (o cargo-deny reprova crate sem licença), dependências
   centralizadas em `[workspace.dependencies]`, `Cargo.lock` commitado.
2. **Lints** em `[workspace.lints]`: `unsafe_code = "forbid"`; `unwrap_used`, `expect_used`, `panic`,
   `unreachable`, `todo`, `unimplemented`, `indexing_slicing` e `dbg_macro` como `deny` no Clippy;
   `clippy.toml` libera `unwrap`, `expect`, `panic` e indexação dentro de testes. Toda crate declara
   `[lints] workspace = true`.
3. **Crates** do BRIEF §5 como cascas com `lib.rs` documentado e um teste trivial: `driver`, `catalog`, `sql`,
   `engine`, `automation`, `ui-egui`. Apps `hy-dbplus` (eframe) e `hy-dbplus-cli` (imprime uso e versão).
   Nomes de pacote com prefixo `hy-dbplus-`.
4. **`xtask layers`**: lê `cargo metadata`, classifica cada crate numa camada (tabela em
   `xtask/src/layers.rs`: driver 0, catalog 1, sql 2, engine 3, automation e ui-egui 4; apps e xtask isentos),
   reprova dependência para cima, crate fora da tabela e crate de UI (`egui`, `eframe`, `winit`, `egui_*`,
   `wgpu*`) fora de `ui-egui` e dos apps. Motor de regras testado sem o cargo; alias `cargo xtask` em
   `.cargo/config.toml`. O portão `--fast` passa a rodar `cargo xtask layers`.
5. **Janela vazia com tema**: `ui-egui` exporta `App` (implementa `eframe::App`) e `theme` (claro e escuro,
   cores inspiradas no TablePlus, seguindo o sistema); o app `hy-dbplus` cria a janela com título, tamanho
   inicial e o tema aplicado. Teste de UI headless com `egui_kittest` fica para o M2, quando houver widget.
6. **Portão inteiro verde** com Rust de verdade: fmt, clippy `-D warnings` com os lints acima, testes,
   cargo-deny (licenças, bans, fontes) e o CI nos três sistemas.

## Won't

- Qualquer comportamento de produto: conexão, catálogo, grid, editor. Cada um é um marco.
- `tokio`, `tokio-postgres`, `keyring`, `thiserror`, `serde`, `toml`, `egui_extras`, `egui_kittest`: entram no
  marco que os usa, cada um como decisão registrada.
- `wgpu`: o eframe desenha com `glow` (OpenGL), que basta para uma janela vazia e traz muito menos dependência.
  Reavaliar no M3 (grid virtualizado) se o desempenho pedir.
- Outros comandos do `xtask` (`gates`, `version`, `ci`, `screenshots` do PdfCraft): só `layers` (decisão do
  Jean em 21/10/2026); os demais entram quando houver o que verificar.
- Empacotamento `.app`, ícone, menu nativo do macOS: M8.

## Restrições

- Cada dependência nova é decisão: justificativa no `impl-notes.md`, licença na lista do `deny.toml`
  (a fonte padrão do egui já tem exceção por crate), `Cargo.lock` no commit.
- `Cargo.toml`, `Cargo.lock`, `.cargo/`, `xtask/` e `apps/*/src/` são sensíveis (`.claude/process.json`): o
  `process:reviewer` roda antes de cada commit que os toque.
- Commits pequenos, nesta ordem: docs; workspace com lints e cascas; `xtask layers`; janela com tema.
