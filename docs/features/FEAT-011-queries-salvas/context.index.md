<!-- FEAT_EM_ANDAMENTO — implementada em 22/10/2026 (engine + UI na árvore), aguardando revisão, commits e conferência do Jean -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-011-queries-salvas/context.index.md

# Contexto — FEAT-011 queries-salvas

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-011 |
| **Slug** | queries-salvas |
| **Origem** | pedido do Jean (22/10/2026): guardar as queries das abas SQL; decisões: arquivos soltos, salvos ao perder o foco |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-22 |
| **Atualizado em** | 2026-10-22 |

## TL;DR

Cada aba SQL vira `~/.config/hy-dbplus/queries/<id da conexão>/<banco>/<n>.sql`, salvo sempre que o texto
mudou e o editor não tem o foco (Esc, clique fora, troca de aba ou de banco) e ao fechar banco, conexão ou
app; ao abrir o banco as abas voltam com o texto e o mesmo número. Fechar a aba com texto pergunta, e
"Descartar e fechar" apaga o arquivo. Só o texto digitado, como está (o store não acrescenta host nem
senha da conexão; arquivo `0600`).

## Leia nesta ordem

```
1. 00-brief/brief.md          → QueryStore, restaurar, salvar, apagar, o que fica de fora
2. 10-prd/acceptance.md       → o que precisa ser verdade, e o que é do Jean
3. 50-dev/impl-notes.md       → decisões (arquivo por aba, id da conexão, numeração por banco) e plano
4. docs/features/FEAT-010-editor-sql/ → as abas SQL que passam a ser salvas
```

## Onde está o quê (quando existir)

```
crates/engine/src/queries.rs      QueryStore: list, save, remove, remove_connection, escape do banco
crates/ui-egui/src/app.rs         Deps.queries; restaurar no DatabaseSelected; QueryTab.saved; salvar/apagar
crates/ui-egui/src/editor.rs      salvar ao perder o foco
crates/ui-egui/src/connections.rs remove_connection ao remover a conexão
apps/hy-dbplus/src/main.rs        QueryStore no Deps
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. apps/hy-dbplus/src/main.rs é sensível; engine/queries.rs escreve em ~/.config: reviewer nos dois.
3. Commit só com o "pode commitar" do Jean, na ordem: docs; engine; ui (com main.rs).
4. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
