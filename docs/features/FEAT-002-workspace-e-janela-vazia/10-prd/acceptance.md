# @file: docs/features/FEAT-002-workspace-e-janela-vazia/10-prd/acceptance.md

# FEAT-002 — Critérios de aceite

## Workspace e lints

- [x] `cargo build --workspace --locked` e `cargo test --workspace --locked` verdes em macOS.
- [x] Toda crate e app tem `license.workspace = true` e `[lints] workspace = true`; `cargo deny check licenses
      bans sources` verde.
- [x] Um `unwrap()` fora de teste em qualquer crate faz `cargo clippy --workspace --all-targets -- -D warnings`
      reprovar; o mesmo `unwrap()` dentro de `#[cfg(test)]` passa (provado uma vez, registrado no impl-notes).
- [x] `unsafe` em qualquer crate não compila (`unsafe_code = "forbid"`).

## `xtask layers`

- [x] `cargo xtask layers` imprime a tabela de crates e camadas e sai com 0.
- [x] Testes do motor de regras (sem cargo): dependência para cima reprova; crate fora da tabela reprova; `egui`
      em `engine` reprova; `egui` em `ui-egui` e nos apps passa; dependência de teste segue a mesma regra.
- [x] `scripts/quality-gate.sh --fast` roda `cargo xtask layers` e fica vermelho quando ele reprova (pelo `run()`
      do portão; ver impl-notes sobre a prova).

## Janela

- [x] `cargo run -p hy-dbplus` abre uma janela com título "hy-dbplus", tamanho inicial definido e fundo no tema
      (claro ou escuro conforme o sistema), sem painel nem widget além de um texto de boas-vindas.
- [x] Nenhum `unwrap`/`expect` no caminho de inicialização: erro do eframe vira mensagem no stderr e saída 1.
- [x] `hy-dbplus-cli` sem argumentos imprime uso e versão e sai com 0.

## Portão e CI

- [x] `scripts/quality-gate.sh --full` verde na máquina do Jean (fmt, clippy, testes, cargo-deny, gitleaks,
      CLAUDE.md, layers).
- [ ] Workflow verde no push: `gate` em macOS, `build` em Linux (com as bibliotecas do egui) e Windows.

## Do Jean

- [ ] Rodar `cargo run -p hy-dbplus` e confirmar a janela e o tema.
- [ ] Marcar como feitos os itens de "Verificação real (depois do M0)" do aceite da FEAT-001 que o M0 prova.
