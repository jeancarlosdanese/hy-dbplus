<!-- FEAT_EM_ANDAMENTO — fases A a C commitadas; falta o CI no pull request (C.0, do Jean) e o fechamento -->
# @file: docs/features/FEAT-016-versao-e-instaladores/context.index.md

# FEAT-016: versão e instaladores

- Brief: `00-brief/brief.md`
- Aceite: `10-prd/acceptance.md`
- Plano e notas: `50-dev/impl-notes.md`

## Status da implementação

Aberta em 18/11/2026 na branch `feat/FEAT-016-versao-e-instaladores`; plano em três fases (A versão, B pacotes do
Linux e do Windows, C release no CI e a versão ligada na entrega) aprovado no mesmo dia. **Fase A** em a4ad25c: `VERSION` em 0.1.0 e `cargo xtask version <x.y.z>` / `--check <tag>` (`xtask/src/version.rs`), revisada e
com o portão completo verde. A chave `version` do `process.json` só entra na fase C. **Fase B** em 0175a8e e 1f79a9f: `cargo xtask bundle` por sistema (`.deb` e `.tar.gz` no
Linux, `.zip` no Windows, em `xtask/src/package.rs`; `.dmg` com o nome novo no macOS), app do Windows sem console e
com `error.log`; revisada, portão completo verde. `.deb` e `.zip` só se montam no CI (fase C). **Fase C**
em c7db0e0, 6c40bb4 e no commit de docs seguinte: `release.yml` (pacotes nos três sistemas, `.deb` instalado e aberto num Debian limpo, Release em
rascunho na tag), `version` e `bump` no `process.json`, regra no `CLAUDE.md` e a versão já em 0.2.0 na branch (a
entrega faz o merge e a tag `v0.2.0` no commit de merge). Falta o CI no pull request (C.0) e o fechamento.

**Cabe ao mantenedor:** push da branch e o pull request (C.0), para o `release.yml` montar os pacotes nos três
sistemas; push da tag `v0.2.0` depois da entrega, que cria a Release em rascunho; revisar e publicar a Release.
