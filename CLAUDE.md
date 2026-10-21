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
scripts/quality-gate.sh --fast          # fmt + clippy + layers + coerência dos CLAUDE.md
scripts/quality-gate.sh --full          # + testes, cargo-deny, gitleaks
scripts/quality-gate-tools.sh           # instala cargo-deny e gitleaks (uma vez por máquina)
```

## Processo (plugin `process@hyberica`, FEAT-001)

Dona das regras de commit e revisão neste repositório. Vale aqui:

- Commit só depois do resumo do diff, da mensagem sugerida e de uma mensagem só do Jean com "pode commitar": vale
  para o que estava alterado naquele instante, por 30 minutos, em quantos commits forem; arquivo mudado depois
  volta a pedir caixa. Não vale para `--amend`.
- Tag e os comandos que criam commit sem `git commit` (merge, rebase, revert, cherry-pick, pull) pedem aprovação
  na tela, com ou sem "pode commitar". `git push` é recusado: é do Jean. Versão só quando o Jean pedir.
- Mudança sensível pendente na árvore não commita sem o agente `process:reviewer` rodar depois do último commit
  (`/code-review` não conta). O que é sensível aqui está em `.claude/process.json`, somado à base do plugin
  (este arquivo e o `CLAUDE.md` de cada app, regras, skills, agentes e migrations).
- A caixa de aprovação é rede de segurança, não caminho do commit. Sempre que ela for aparecer, dizer antes ao
  Jean o que ela faz e se é para aprovar.
- **Portão de qualidade** (`scripts/quality-gate.sh`): no fim de cada resposta (`--fast`), no pre-commit
  (`--full`) e no GitHub Actions (`--full --all`). Não há linha de base: o projeto nasceu sem passivo e o portão
  é estrito desde o primeiro commit. Nunca `git commit --no-verify`.
- Padrão de engenharia completo: skill `/process:quality`.
