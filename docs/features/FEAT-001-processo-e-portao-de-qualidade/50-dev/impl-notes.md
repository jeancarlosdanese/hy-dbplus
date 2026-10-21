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

### 21/10/2026 — Primeiro commit e `fix(process)`

- [x] Primeiro commit (`854dd01`) só saiu depois de o plugin process ser corrigido para repositório sem commit
  (`git rev-parse HEAD` falhava e o registro do revisor não era gravado; process 0.4.1, lido da pasta do kit
  em modo dev, usa a árvore vazia como HEAD). Ficou para a 0.4.2: repositório com `--object-format=sha256`.
- [x] Achados do `process:reviewer` (2ª e 3ª rodadas) corrigidos: regex de sensível aceita módulo em diretório
  (`query/mod.rs`, `statements/mod.rs`) e cobre `crates/*/CLAUDE.md`; `deny.toml` com exceção por crate para a
  fonte padrão do egui (`epaint_default_fonts`: OFL-1.1 e Ubuntu-font-1.0, que reprovariam no M0), `OFL-1.1`
  fora do allow global, `pq-sys` nos bans (o `CLAUDE.md` já prometia "sem libpq") e comentário do `ring`
  reescrito; verificador dos CLAUDE.md enxerga `crates/*/CLAUDE.md` e `xtask/CLAUDE.md` (teste novo falha
  com o `docs()` antigo); `rust-cache` do job `gate` guardado por `Cargo.toml` como no `build`; comentários
  do portão (gatilho da autochecagem), do workflow (cálculo da base, macOS como alvo principal) e do brief
  desta FEAT; `AGENTS.md` fora do BRIEF (`CLAUDE.md` é o dono único); índice sem caminho local da máquina.
- [x] Decisão do Jean: o BRIEF citava um banco e uma tabela de outro projeto como referência de aceite;
  trocados por "um banco local" e "uma tabela". Portão do CI passou do Linux para o macOS (alvo principal e
  máquina de desenvolvimento); Linux e Windows ficaram no job `build`, com as bibliotecas do egui só no Linux.
- [x] 3ª rodada do revisor neste diff: `libpq-sys` e `libpq` juntaram-se ao `pq-sys` nos bans; a base de
  sensível do plugin descrita por inteiro no `CLAUDE.md` (app, regras, skills, agentes e migrations).
- [ ] Teste do portão shell (`scripts/quality_gate_test.py`) para os consertos de gitleaks sem base e
  `--base ""`: tarefa própria antes do M0.
