# @file: docs/features/FEAT-002-workspace-e-janela-vazia/50-dev/impl-notes.md

# Notas de Implementação — FEAT-002 workspace-e-janela-vazia

## Estado atual

_Atualizado em 21/10/2026._

- **Feito:** documentação; workspace com lints, seis crates e dois apps como cascas; portão `--full` verde com
  as etapas Rust de verdade.
- **Em curso:** nada.
- **Falta:** `xtask layers`; janela com tema; CI verde; verificação do Jean.
- **Depende de:** nada.
- **Para retomar:** esta pasta, o `CLAUDE.md` da raiz e o BRIEF §5 a §7.

## Decisões

- **[2026-10-21]** Todas as crates do BRIEF §5 nascem no M0, vazias, em vez de uma por marco: a tabela do
  `xtask layers` e a divisão de responsabilidades ficam definidas de uma vez, e cada marco seguinte só
  preenche. Crate vazia custa um `lib.rs` com documentação e um teste.
- **[2026-10-21]** Camadas: `driver` 0, `catalog` 1, `sql` 2 (usa o modelo do `catalog` para o autocomplete),
  `engine` 3, `automation` e `ui-egui` 4 (as duas só conhecem o `engine`; sem aresta entre elas), apps e
  `xtask` isentos. Crate de UI (`egui`, `eframe`, `winit`, `egui_*`, `wgpu*`) só em `ui-egui` e nos apps.
- **[2026-10-21]** Lints de never-panic como `deny` em `[workspace.lints.clippy]`, não `warn` + `-D warnings`:
  a regra do `CLAUDE.md` é "proibido", e com `deny` o `cargo clippy` local sem flags já reprova. Em teste,
  `clippy.toml` libera (`allow-unwrap-in-tests` e afins), como o `CLAUDE.md` promete.
- **[2026-10-21]** `xtask layers` modelado no `xtask/src/layers.rs` do PdfCraft (motor puro sobre um modelo
  simples, `cargo metadata` lido por `serde_json`), sem `anyhow`: erro é `String` no xtask, que é ferramenta
  de build, não produto. Tabela e regras bem menores: seis crates, uma classe de UI.
- **[2026-10-21]** Argumento e variável de ambiente entram por `args_os` e `var_os`: a versão `String` da
  `std` faz `unwrap` interno e os lints não enxergam. Regra registrada no `CLAUDE.md`.
- **[2026-10-21]** `eframe` com `glow`, sem `wgpu`, até o M3: janela vazia não precisa de GPU moderna e o
  grafo de dependências fica menor para o cargo-deny e para o build no CI.
- **[2026-10-21]** Dependências novas do M0, cada uma uma decisão:
  - `egui` e `eframe` 0.36.2 (MIT OR Apache-2.0; mesma versão dos projetos de inspiração; a fonte padrão
    `epaint_default_fonts` já tem exceção de licença no `deny.toml`).
  - `serde_json` 1 (MIT OR Apache-2.0) só no `xtask`, para ler o `cargo metadata`.

## Log

### 21/10/2026 — Criação

- [x] Leitura do BRIEF §5 a §7, do workspace do PdfCraft (`Cargo.toml`, `clippy.toml`, `xtask/src/layers.rs`,
  `apps/pdfcraft`) e das versões no cache local do cargo (egui, eframe, egui_kittest 0.36.2; tokio 1.53.1).
- [x] Brief, aceite, notas e índice escritos antes do código (`adb0dfd`).

### 21/10/2026 — Workspace, lints e cascas

- [x] `Cargo.toml` do workspace (membros `crates/*` e `apps/*`; `xtask` entra no passo seguinte), `clippy.toml`,
  seis crates e dois apps com `license`, `edition`, `rust-version` e `repository` herdados e `[lints]
  workspace = true`. Cada crate declara já as dependências internas do BRIEF §5, para o `layers` ter grafo.
- [x] Provas dos lints (uma vez, na máquina): `unwrap()` fora de teste reprova o clippy ("used `unwrap()` on an
  `Option` value"); o mesmo `unwrap()` e uma indexação dentro de `#[cfg(test)]` passam; bloco `unsafe` não
  compila ("usage of an `unsafe` block"). Revertido depois de provar.
- [x] `hy-dbplus-cli`: uso e versão, saída 0; comando desconhecido sai com 1. `hy-dbplus`: só imprime a versão
  até o passo da janela.
- [x] Portão `--full` verde: fmt, clippy, testes (6 crates com um teste cada), cargo-deny (licenças, bans,
  fontes; ainda sem dependência externa), CLAUDE.md, gitleaks.
- [x] Revisor (seis achados, todos corrigidos antes do commit): a CLI entrava em pânico com argumento que não é
  UTF-8 (`std::env::args()` faz `unwrap` por dentro, invisível aos lints); virou `args_os` com `run(args)`
  testável e teste que falhava antes (`0xff` sai com 1). `version` saiu das dependências internas (o bump do
  workspace quebraria o `--locked`), o que exigiu `publish = false` herdado e `allow-wildcard-paths` no
  `deny.toml` (sem isso o cargo-deny trata `path` sem `version` como curinga). `LICENSE` MIT na raiz, que o
  metadado já declarava. Lista de lints do `CLAUDE.md` alinhada ao `Cargo.toml` (`unimplemented!`, `dbg!`,
  regra do `args_os`). Uso da CLI não anuncia mais comandos que ela recusa.
