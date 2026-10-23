<!-- FEAT_EM_ANDAMENTO — docs e packaging/ criados em 23/10/2026; xtask bundle não iniciado -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-014-empacotamento-macos/context.index.md

# Contexto — FEAT-014 empacotamento-macos

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-014 |
| **Slug** | empacotamento-macos |
| **Origem** | pedido do Jean (23/10/2026): "gerar um instalador para o Mac"; M8 do BRIEF |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-23 |
| **Atualizado em** | 2026-10-23 |

## TL;DR

`cargo xtask bundle` monta `hy-dbplus.app` (build release, ícone `.icns` a partir de `packaging/icon.svg`,
`Info.plist` com `br.com.hy.dbplus` e a versão do workspace), assina (ad hoc por padrão; Developer ID e
notarização por variáveis de ambiente) e gera o `.dmg` com o atalho para Applications. Só ferramentas do
macOS, nenhuma dependência nova.

## Leia nesta ordem

```
1. 00-brief/brief.md          → o que o bundle faz, assinatura, o que fica de fora
2. 10-prd/acceptance.md       → o que precisa ser verdade, e o que é do Jean
3. 50-dev/impl-notes.md       → decisões (xtask sem cargo-bundle; ad hoc agora) e plano
4. xtask/src/main.rs          → a tabela de comandos onde `bundle` entra
```

## Onde está o quê (quando existir)

```
packaging/icon.svg            ícone do app (sem texto)
packaging/logo.svg, logo.png  logomarca y-dbplus (texto em Inter; PNG renderizado)
xtask/src/bundle.rs           plist, iconset, layout, codesign, dmg, notarização
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. xtask/ é sensível e packaging/ é protegido: reviewer antes de cada commit; packaging só com o Jean.
3. Commit só com o "pode commitar" do Jean, na ordem: docs (+ packaging); feat(xtask).
4. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
