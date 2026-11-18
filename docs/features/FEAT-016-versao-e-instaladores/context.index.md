<!-- FEAT_EM_ANDAMENTO — fase A (versão) commitada; próxima, fase B (pacotes do Linux e do Windows) -->
# @file: docs/features/FEAT-016-versao-e-instaladores/context.index.md

# FEAT-016: versão e instaladores

- Brief: `00-brief/brief.md`
- Aceite: `10-prd/acceptance.md`
- Plano e notas: `50-dev/impl-notes.md`

## Status da implementação

Aberta em 18/11/2026 na branch `feat/FEAT-016-versao-e-instaladores`; plano em três fases (A versão, B pacotes do
Linux e do Windows, C release no CI e a versão ligada na entrega) aprovado no mesmo dia. **Fase A** em a4ad25c: `VERSION` em 0.1.0 e `cargo xtask version <x.y.z>` / `--check <tag>` (`xtask/src/version.rs`), revisada e
com o portão completo verde. A chave `version` do `process.json` só entra na fase C.

**Cabe ao mantenedor:** push da branch e o pull request (C.0), para o `release.yml` montar os pacotes nos três
sistemas; push da tag `v0.2.0` depois da entrega, que cria a Release em rascunho; revisar e publicar a Release.
