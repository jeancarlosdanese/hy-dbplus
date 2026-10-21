# @file: docs/features/FEAT-001-processo-e-portao-de-qualidade/00-brief/brief.md

# FEAT-001 — Processo e portão de qualidade

## Problema

O hy-dbplus nasce num diretório vazio, para ser desenvolvido por agentes com um mantenedor que não escreve Rust.
Sem travas, a IA pode commitar sem revisão, empurrar código que entra em pânico com uma resposta inesperada do
banco, puxar uma dependência com licença incompatível ou vazar uma senha de conexão num commit. A Hyberica já
tem esse processo pronto (plugin `process@hyberica` e o portão do HyVox), mas calibrado para Go e Next.

## Objetivo

Instalar o processo antes do primeiro código: a parte genérica vem do plugin sem cópia; a parte do projeto
(portão, CI, o que é sensível) é escrita para um workspace Rust que começa sem passivo.

## Escopo (Must)

1. **Plugin `process@hyberica`** ligado no projeto (`.claude/settings.json`), com `.claude/process.json` dizendo o
   que é protegido e sensível aqui (o `comment` do arquivo explica o critério).
2. **Portão de qualidade para Rust** (`scripts/quality-gate.sh`): `--fast` com fmt, clippy (`-D warnings`, lints
   de never-panic do workspace) e coerência dos CLAUDE.md; `--full` soma testes, cargo-deny (licenças, bans,
   fontes) e gitleaks no stage; `--all` no CI soma advisories e gitleaks no intervalo do push.
3. **Pre-commit** copiado do template do kit, sem alteração; **CI** no GitHub Actions com o portão em máquina
   limpa (macOS, alvo principal) e build com testes em Linux e Windows.
4. **`CLAUDE.md`** do projeto com a seção Processo (dona das regras), as regras de engenharia e os comandos.
5. **Esta FEAT** documentada no padrão reduzido do kit (brief, aceite, notas, índice).

## Won't

- Linha de base e catracas (ratchets): o projeto não tem passivo; tudo é estrito desde o início.
- Verificador de identificador em português para Rust (o HyVox tem extratores para Go e TS): fica com o revisor;
  entra como script se virar problema recorrente.
- Regra de versão no `process.json`: só quando o workspace tiver arquivo de versão e tags (M8).
- Mudar o plugin ou o template do pre-commit: é do kit.

## Restrições

- Push, `git config core.hooksPath`, instalação das ferramentas e criação do repositório no GitHub são do Jean.
- O marketplace `hyberica` desta máquina aponta hoje para a pasta local do kit (modo dev), não para o GitHub.
  O `settings.json` do projeto declara a fonte do GitHub, como nos outros projetos; na máquina do Jean vale a
  pasta local. Não é defeito, mas convém saber.
