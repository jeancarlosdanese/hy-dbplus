# hy-dbplus

Cliente de banco de dados desktop, leve e simples, no espírito do TablePlus, escrito inteiramente em Rust com
`egui`/`eframe`. Uso principal: navegar dados e escrever SQL com autocomplete. Produto, arquitetura, stack e
marcos estão em [docs/BRIEF.md](docs/BRIEF.md): leia antes de qualquer trabalho. Inspiração de estrutura e
rigor: PdfCraft e VectorCraft (storytold).

## Comece por aqui

1. [docs/BRIEF.md](docs/BRIEF.md): o que o produto é, o MVP, as crates e os marcos M0 a M8.
2. A FEAT em curso em `docs/features/`: o `context.index.md` de cada uma é o ponto de entrada (marcador de status
   na linha 1; `50-dev/impl-notes.md` tem o estado atual).
3. Este arquivo: regras de engenharia e de processo.

## Idioma e nomenclatura

- Comunicação com o mantenedor em português do Brasil, com acentuação correta.
- Identificador de código (crate, módulo, tipo, função, variável) em inglês; comentário, documentação e mensagem
  de commit em pt-BR. Nome de função de teste em pt-BR é permitido.

## Regras de engenharia (não negociáveis)

- **Never panic.** Toda resposta do banco, todo arquivo de configuração, toda query digitada e toda tecla é
  entrada não confiável. `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` e indexação
  com posição vinda de entrada são proibidos fora de teste, e `dbg!` em qualquer lugar (lints do Clippy em
  `[workspace.lints]` do `Cargo.toml`, dono da lista; reprovam o portão). `unsafe_code = "forbid"`. Erro vira
  `Result` com o enum de erro da crate e sobe com `?`. Argumento e variável de ambiente entram por `args_os`
  e `var_os`: a versão `String` da `std` entra em pânico com byte inválido.
- **Layering.** Nada abaixo da UI depende de `egui`, `eframe` ou `winit`. O engine é a fachada de todos os
  frontends (app, CLI, MCP). A divisão de crates está no BRIEF §5.
- **Async fora da UI.** Consultas rodam no runtime `tokio` e devolvem eventos por canal; nenhuma chamada de rede
  bloqueia o frame.
- **Segredos nunca em arquivo.** Senha só no Keychain via `keyring`; o TOML de conexões guarda host, porta,
  usuário, banco e tag. Nada de segredo em log, erro ou resposta do MCP.
- **Rust only.** Nenhum JS/TS no produto nem no build. Binário estático: TLS por `rustls`, sem `libpq` nem
  `openssl` (bloqueados em `deny.toml`).
- **Dependência nova é decisão.** Justificar, conferir manutenção e licença (`deny.toml` lista as permitidas);
  `Cargo.lock` é parte do código e o portão roda com `--locked`.
- **Sem refatoração estética.** Refatora-se por objetivo declarado. Formatador só nos arquivos tocados.
- **Testes.** Bug corrigido ganha teste que falharia antes do conserto (provar revertendo o conserto). Teste que
  depende do PostgreSQL local fica atrás de uma feature ou variável de ambiente e não entra na suíte padrão.

## Workflow

- Discutir o objetivo de toda feat ou fix antes de implementar. Melhor não fazer do que fazer errado.
- Nunca iniciar, reiniciar ou parar serviços: o PostgreSQL local (`pg-localhost`) já está rodando.
- Algo inesperado: avisar antes de agir.
- Mudança grande se quebra em partes pequenas e testáveis; commits incrementais (Conventional Commits).
- Documentação de feature em `docs/features/FEAT-NNN-slug/` (brief, aceite, notas de implementação,
  `context.index.md`), criada antes do código e atualizada conforme a implementação avança.
- Build e testes verdes antes de declarar algo pronto; reportar o resultado como ele é.

## Comandos

```bash
cargo run -p hy-dbplus                  # app desktop (a partir do M0)
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask layers                      # camadas do BRIEF §5 (xtask/src/layers.rs)
cargo xtask bundle                      # hy-dbplus.app e .dmg em target/bundle/ (macOS; FEAT-014)
scripts/quality-gate.sh --fast          # fmt + clippy + layers + coerência dos CLAUDE.md
scripts/quality-gate.sh --full          # + testes, cargo-deny, gitleaks
scripts/quality-gate-tools.sh           # instala cargo-deny e gitleaks (uma vez por máquina)
```

## Processo (plugin `process@hyberica`, FEAT-001)

O plugin está ligado no escopo do projeto (`.claude/settings.json`); o que é próprio daqui está em
`.claude/process.json`. Esta seção é a dona das regras de commit e entrega; como o revisor roda (passadas,
delta, gravidade) e o critério de conclusão ficam na skill `process:quality` (Revisão final e Entrega).

- **Branch:** toda mudança em branch (`feat/`, `fix/`, `docs/`, `chore/`), em passos pequenos. Todo commit, em
  branch ou na `main`, só com o "pode commitar" do Jean, sozinho numa oração de mensagem só dele (vale 30
  minutos, quantos commits forem; não vale para `--amend`): a IA mostra o diff e a mensagem sugerida e espera
  a palavra. Em branch não há revisor no commit; na `main` o commit ainda exige o revisor sobre todo código
  tocado. Sequência: diff → "pode commitar" → commit → "entrega". Troca de branch e commit, merge, reset ou
  tag vão em comandos separados.
- **Entrega:** "entrega", primeira palavra de uma mensagem só do Jean, autoriza de uma vez o portão completo
  na branch e o merge `--no-ff` na `main` (vale 30 minutos). Sem ela, o merge é recusado. A trava recusa o
  merge se a branch tem código sem revisão (ou alterado depois dela, teste incluído) ou sem o portão completo
  verde sobre a ponta da branch (comando `gate` do `.claude/process.json`, rodado pelo executor do plugin; a
  saída fica na tela e o push vem depois, do Jean). Versão e tag entram na entrega quando o workspace tiver
  `version` e `bump` no `process.json` (M8); até lá a IA não sobe versão nem cria tag na entrega (é conduta: sem
  `version` a trava não confere a tag).
- **Push e deploy:** sempre do Jean; a trava recusa.
- **Revisor:** em toda branch com código, antes do merge: primeira passada sobre o diff inteiro, as seguintes
  só sobre o delta; liberada sem achado alto ou médio (`/code-review` não conta). Achado baixo também se
  corrige: exceção deste projeto ao padrão da skill, que deixa o baixo só registrado (pedido do Jean,
  17/11/2026). Detalhe: skill `process:quality`.
- **Recusa da trava:** não há caixa de aprovação; o comando passa ou é recusado. Recusa não se contorna: a IA
  explica o motivo e o caminho que tomou, e o resumo traz a tabela de decisões (skill `process:quality`,
  Revisão final). Arquivo do portão é editável em branch e o revisor o confere; registros da trava,
  `.git/config` e `settings.json` do Claude a IA não escreve.
- **Mensagem para outra sessão:** nunca repetir "pode commitar" nem "entrega", nem para explicar algo.
- **Aviso do plugin:** depois de uma atualização, a sessão recebe a nota da versão, uma vez. Ela diz o que
  conferir neste projeto e quais comandos são do Jean. A IA apresenta a nota e pergunta; só muda o projeto se
  o Jean pedir.
- **Portão de qualidade** (`scripts/quality-gate.sh`): no fim de cada resposta (`--fast`), no pre-commit
  (`--fast` em branch, `--full` na `main` e no merge para ela), na entrega (`--full --all`, pelo `gate`) e no
  GitHub Actions (`--full --all`). Não há linha de base: o projeto nasceu sem passivo e o portão é estrito
  desde o primeiro commit. Nunca `git commit --no-verify`. O pre-commit de branch não roda testes: antes de
  declarar algo pronto ou pedir a entrega, a IA roda `scripts/quality-gate.sh --full`.
- **Segunda camada:** `.githooks/pre-commit` e `.githooks/pre-merge-commit` conferem o que vai de fato no
  commit e no merge. Ativar uma vez por clone, pelo Jean: `git config core.hooksPath .githooks`.
- **Padrão visual e protótipos:** app desktop em `egui`; o padrão das telas são os tokens de
  `crates/ui-egui/src/theme.rs`, os componentes de `widgets.rs`, os ícones de `fonts.rs` e o catálogo de textos
  `i18n.rs` (pt-BR e en). Nenhuma cor, espaçamento ou texto de tela fora deles. Protótipo de referência (HTML,
  fora do build): `docs/features/FEAT-015-redesenho-ux/20-prototipos/prototipo.html`; tela nova ganha protótipo
  na pasta `20-prototipos/` da sua FEAT.
- **Formulários:** o padrão de formulários do plugin (backend que normaliza, front que mascara) não se aplica:
  não há backend web. Siglas sempre em maiúsculas: nenhuma. Exceções: nenhuma.
- **FEATs** (skills `process:feat-new`, `feat-run` e `feat-close`; item sem valor segue o padrão do plugin):
  - **Pasta:** `docs/features/`.
  - **Arquivo das concluídas:** nenhum.
  - **Backlog:** nenhum.
  - **Passos extras ao fechar:** nenhum.
  - **Teto de FEATs em andamento:** 15.
  - **Versão:** padrão do plugin, a partir do M8 (antes disso o projeto não versiona).
- Padrão de engenharia completo: skill `/process:quality`.
