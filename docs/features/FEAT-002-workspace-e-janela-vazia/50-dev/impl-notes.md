# @file: docs/features/FEAT-002-workspace-e-janela-vazia/50-dev/impl-notes.md

# Notas de Implementação — FEAT-002 workspace-e-janela-vazia

## Estado atual

_Atualizado em 21/10/2026._

- **Feito:** documentação; workspace com lints, seis crates e dois apps como cascas (`921208b`); `cargo xtask
  layers` (`86e3ebc`); tema claro e escuro e janela vazia com eframe; portão `--full` verde com 299 pacotes
  no lock.
- **Em curso:** nada.
- **Falta:** nada. CI verde nos três sistemas e janela conferida pelo Jean em 21/10/2026; FEAT concluída.
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
  regra do `args_os`). Uso da CLI não anuncia mais comandos que ela recusa. Commit `921208b`.

### 21/10/2026 — `xtask layers`

- [x] `xtask/src/layers.rs`: tabela (driver 0, catalog 1, sql 2, engine 3, automation 4, ui-egui 4 UI; apps e
  xtask isentos) e motor de regras sobre um modelo sem cargo; `from_metadata` lê `cargo metadata --no-deps`.
  Oito testes: grafo do BRIEF passa; para cima reprova; mesma camada reprova nos dois sentidos (automation e
  ui-egui não se conhecem); crate de UI fora da `ui-egui` reprova mesmo como dependência de teste ou de build;
  UI na `ui-egui` e nos apps passa; crate fora da tabela reprova; leitura do metadata.
- [x] `xtask/src/main.rs`: tabela de comandos (só `layers`), erro como `String`, sem `anyhow`; alias em
  `.cargo/config.toml`. `cargo xtask layers` imprime as 9 crates e sai com 0.
- [x] Portão `--fast` roda `cargo run -p xtask --locked --quiet -- layers` depois do clippy; o `run()` do portão
  pinta vermelho quando o comando sai com erro (provado em `scripts/quality_gate_test.py` para o portão em
  geral). Provar o layers reprovando no repositório real exige mudar o grafo, o que muda o `Cargo.lock` e faz
  o `--locked` falhar antes: a prova fica nos testes do motor.
- [x] Dependência nova: `serde_json` 1 (com `serde`, `serde_core`, `serde_derive`, `syn`, `quote`,
  `proc-macro2`, `unicode-ident`, `itoa`, `memchr`, `zmij` no lock, todas MIT OR Apache-2.0 ou equivalentes;
  cargo-deny verde). Primeira atualização do `Cargo.lock` feita de propósito, sem `--locked`.
- [x] Revisor (quatro achados, corrigidos antes do commit): o `main.rs` do xtask repetia o `std::env::args()`
  da CLI (pânico com byte inválido); virou `args_os` com `run(args)` e o mesmo teste de `0xff`. Crate de camada
  dependendo de app ou xtask passava calada (inalcançável hoje, só binários; abriria com um `lib.rs` num app):
  virou a violação `OnExempt`, com teste. Doc e mensagem da regra de UI dizem "ui-egui, apps e xtask", como o
  código faz. `cargo xtask layers` entrou na seção Comandos do `CLAUDE.md`. Commit `86e3ebc`.

### 21/10/2026 — Janela com tema

- [x] `crates/ui-egui/src/theme.rs`: `Tokens` por papel, indexados por `egui::Theme` (painel, cartão, campo, texto,
  borda, divisor, hover, pressionado, destaque), `visuals(kind)` monta os `Visuals` do egui a partir dos tokens,
  `apply(ctx)` registra as duas variantes e deixa a preferência em "sistema", `current(ctx)` devolve os tokens
  da variante em uso. Três testes sem janela (`egui::Context::default()`). Revisor: o enum `Kind` espelhava
  `egui::Theme` 1:1 e saiu. Nota: em Linux sem sinal de tema (X11, Wayland sem portal) o egui cai no escuro
  em silêncio; no macOS o winit informa a aparência e o `ThemeChanged` ao vivo.
- [x] `crates/ui-egui/src/app.rs`: `App` com `new(cc)` aplicando o tema e `ui(&mut Ui)` separado da trait, para
  o `egui_kittest` no M2. O `eframe` 0.36 trocou `update(ctx)` por `ui(ui)`: o `Ui` raiz vem sem margem nem
  fundo, e é o `CentralPanel` que pinta o fundo do tema.
- [x] `apps/hy-dbplus/src/main.rs`: `ViewportBuilder` com título, app id, tamanho inicial 1200×780 e mínimo
  800×500; `Renderer::Glow` explícito; erro do `run_native` vira mensagem no stderr e saída 1.
- [x] Dependências novas: `egui` e `eframe` 0.36.2 (MIT OR Apache-2.0), eframe sem as features padrão
  (`default_fonts`, `glow`, `wayland`, `x11`). Lock foi de 20 para 299 pacotes; cargo-deny verde (licenças,
  bans, fontes), com a exceção da fonte padrão do egui já prevista na FEAT-001.
- [x] Prova: janela aberta por 6 s a partir do binário do repositório, sem erro no stderr; 21 testes no
  workspace; portão `--full` verde (fmt, clippy, layers, test, cargo-deny, scripts, CLAUDE.md, gitleaks).
