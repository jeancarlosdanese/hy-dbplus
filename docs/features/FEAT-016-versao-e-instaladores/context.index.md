<!-- FEAT_CONCLUIDA — 18/11/2026: entregue em fee3ae1 (v0.2.0); aceite inteiro comprovado no CI da tag e na entrega da v0.2.1 -->
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
entrega faz o merge e a tag `v0.2.0` no commit de merge). Correções da revisão da branch inteira em 379c4eb.
Fechada na branch em 18/11/2026, com o portão completo verde.

Entregue em 18/11/2026: merge `fee3ae1` e tag `v0.2.0`; o CI do push montou os quatro pacotes e criou a Release em
rascunho (AC-006, AC-007, AC-010), e a entrega do fix seguinte (`v0.2.1`) provou o commit da versão pela trava
(AC-004). O Portão de qualidade do mesmo push teve uma falha única de teste no Windows, que passou ao reexecutar
(diagnóstico no `wait_for`, `3a521d1`).

**Cabe ao mantenedor:** revisar e publicar as Releases em rascunho.
