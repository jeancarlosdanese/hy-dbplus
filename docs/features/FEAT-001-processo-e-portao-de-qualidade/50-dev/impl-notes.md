# @file: docs/features/FEAT-001-processo-e-portao-de-qualidade/50-dev/impl-notes.md

# Notas de Implementação — FEAT-001 processo-e-portao-de-qualidade

## Estado atual

_Atualizado em 21/10/2026._

- **Feito:** todos os arquivos do processo e do portão (lista em `10-prd/acceptance.md`); portão verde em
  `--fast`; verificador dos CLAUDE.md verde; testes do verificador passando.
- **Em curso:** nada.
- **Falta:** os passos do Jean (sessão nova, `hooksPath`, ferramentas, primeiro commit, repositório e push) e a
  verificação real depois do M0.
- **Depende de:** Jean.
- **Para retomar:** esta pasta e o `CLAUDE.md` da raiz.

## Decisões

- **[2026-10-21]** Sem linha de base nem catraca. O HyVox precisa delas porque tem passivo; o hy-dbplus nasce
  vazio, então fmt, clippy e cargo-deny são estritos desde o primeiro commit. `--baseline` sai com erro 64.
- **[2026-10-21]** Never-panic é lint do Clippy, não script: `unwrap_used`, `expect_used`, `panic`,
  `unreachable`, `todo`, `indexing_slicing` e `unsafe_code = "forbid"` em `[workspace.lints]` do `Cargo.toml`
  (M0). O portão só roda `clippy -D warnings`; a regra fica onde o compilador a vê.
- **[2026-10-21]** cargo-deny em vez de cargo-audit: cobre advisories, licenças, bans e fontes num só binário
  com um só `deny.toml`. Advisories só no CI (`--all`), como o govulncheck no HyVox: vulnerabilidade publicada
  hoje não trava o commit de quem não mexeu nela.
- **[2026-10-21]** gitleaks por `go install`, como no HyVox: o Go está na máquina e no CI. Trocar por download
  do binário se o Go sair da máquina.
- **[2026-10-21]** `--locked` em clippy, test e build: `Cargo.lock` é parte do código; atualização de
  dependência é commit próprio com `cargo update`.
- **[2026-10-21]** O verificador dos CLAUDE.md é cópia do kit com as raízes trocadas (o CI não alcança o
  plugin). Qualquer outro conserto é no kit.
- **[2026-10-21]** O critério do que é sensível está no `comment` de `.claude/process.json` (dono único).
  A UI fica de fora: é coberta por testes e não decide segurança.
- **[2026-10-21]** `--fast` roda clippy no workspace inteiro em vez de só nas crates tocadas: o projeto é
  pequeno e o cache incremental do cargo basta. Revisar se o Stop hook passar de ~30 s.

## Log

### 21/10/2026 — Criação

- [x] Leitura do kit (README, CLAUDE.md, FEAT-001 do kit, hooks, templates), do portão do HyVox e do piloto no
  salao-xadrez.
- [x] `git init -b main`, pastas, `.gitignore`.
- [x] Cópias do kit: `.githooks/pre-commit`, `scripts/quality_claude_docs.py` e teste.
- [x] Escritos para Rust: `scripts/quality-gate.sh`, `scripts/quality-gate-tools.sh`, `deny.toml`,
  `.github/workflows/quality-gate.yml`, `.claude/settings.json`, `.claude/process.json`, `CLAUDE.md`.
- [x] O portão `--fast` pegou o primeiro defeito: com as raízes trocadas para Rust, a fixture do teste do
  verificador (`lib/sumiu.ts`) deixou de ser detectada como caminho morto. Fixture adaptada para `src/*.rs`;
  o resto do teste é idêntico ao kit. Regex de sensível do engine ganhou o plural (`credentials.rs`).
- [x] Revisão pelo `process:reviewer` (sete achados, todos corrigidos antes do primeiro commit): `hashFiles`
  em `jobs.if` invalidava o workflow (passou para `steps.if`); gitleaks do CI varria zero commits no primeiro
  push (base vazia agora varre o histórico inteiro); BRIEF era segundo dono das regras (agora aponta para o
  `CLAUDE.md`); regex de sensível com palavra fechada (`executor.rs` escapava; virou prefixo, e a CLI e o
  split de statements entraram); clarify do `ring` contradizia a lista de licenças (removido até o M0); lista
  do sensível em quatro redações (dono único: `process.json`); ids do workflow em português.
- [x] Verificação: `bash -n` nos shells, `python3 -m unittest` no verificador, `scripts/quality-gate.sh --fast`
  verde, `CLAUDE.md` com 80 linhas e verde no verificador.
