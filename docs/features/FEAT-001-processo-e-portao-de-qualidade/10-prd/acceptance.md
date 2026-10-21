# @file: docs/features/FEAT-001-processo-e-portao-de-qualidade/10-prd/acceptance.md

# FEAT-001 — Critérios de aceite

## Feito pela IA (21/10/2026)

- [x] `git init` com branch `main`; `.gitignore` com `target/`, `tmp/`, `.env*`, chaves e `settings.local.json`.
- [x] `.claude/settings.json`: marketplace `hyberica`, `process@hyberica` ligado, `allow` para `cargo *` e o
  portão; sem `Bash(git commit *)` em `ask`.
- [x] `.claude/process.json` com protegido e sensível do projeto.
- [x] `.githooks/pre-commit` idêntico a `templates/githooks/pre-commit` do kit 0.4.0, executável.
- [x] `scripts/quality-gate.sh` (`--fast`, `--full`, `--all --base`), compatível com bash 3.2, sintaxe conferida,
  verde em `--fast` no estado atual (etapas Rust puladas com aviso até o M0).
- [x] `scripts/quality-gate-tools.sh` com cargo-deny 0.20.2 e gitleaks v8.30.1 fixados.
- [x] `scripts/quality_claude_docs.py` e teste copiados do kit, raízes trocadas para `crates/`, `apps/`,
  `xtask/`; testes passando; `CLAUDE.md` verde no verificador.
- [x] `deny.toml`: licenças permitivas, `openssl`/`native-tls` proibidos, só crates.io como fonte.
- [x] `.github/workflows/quality-gate.yml`: portão em ubuntu com base de comparação; build em macOS e Windows
  quando houver `Cargo.toml`.
- [x] `CLAUDE.md` da raiz com seção Processo, regras de engenharia e comandos, até 200 linhas.

## Do Jean

- [ ] Abrir uma sessão nova no projeto e conferir `claude plugin list`: `process@hyberica` ligado no escopo
  project (a sessão grava `.git/claude-plugin-root`, que o pre-commit usa).
- [ ] `git config core.hooksPath .githooks` (a trava recusa esse comando vindo da IA).
- [ ] `scripts/quality-gate-tools.sh` (instala cargo-deny e gitleaks em `tmp/tools/bin`).
- [ ] Primeiro commit com "pode commitar" (a IA sugere a mensagem); conferir que o pre-commit rodou o portão.
- [ ] Criar o repositório `jeancarlosdanese/hy-dbplus` no GitHub, push e conferir o workflow verde.

## Verificação real (depois do M0)

- [ ] `cargo clippy` reprova um `unwrap()` fora de teste (lints de never-panic em `[workspace.lints]`).
- [ ] Commit que toca `crates/driver/src/` é recusado sem o `process:reviewer` e passa depois dele.
- [ ] `git push` pela IA é recusado pela trava.
- [ ] Dependência com licença GPL reprova o `cargo deny check licenses`.
- [ ] Segredo no stage reprova o gitleaks no `--full`.
