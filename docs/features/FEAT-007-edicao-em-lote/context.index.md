<!-- FEAT_EM_ANDAMENTO — implementada em 22/10/2026; falta push, CI e conferência do Jean -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-007-edicao-em-lote/context.index.md

# Contexto — FEAT-007 edicao-em-lote

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-007 |
| **Slug** | edicao-em-lote |
| **Origem** | decisão 6 do BRIEF (22/10/2026); conferência do Jean no M3 ("Delete sobre a seleção; Update na tabela ou no painel") |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-22 |
| **Atualizado em** | 2026-10-22 |

## TL;DR

Update e Delete na aba Data, só em tabela com chave primária: edição pendente na célula (duplo clique) e no
Form, ⌫ marca linhas para apagar, Commit mostra o SQL exato e roda tudo numa transação; erro desfaz e mantém
as pendências; Descartar limpa. `+ Row` fica fora.

## Leia nesta ordem

```
1. 00-brief/brief.md          → problema, objetivo, escopo por camada, o que fica de fora
2. 10-prd/acceptance.md       → o que precisa ser verdade por crate, e o que é do Jean
3. 50-dev/impl-notes.md       → decisões (literais, contagem, trava de navegação), plano e log
4. docs/features/FEAT-006-painel-de-registro/ → seleção e painel que esta feature torna editáveis
```

## Onde está o quê

```
crates/catalog/src/edit.rs        ChangeSet, RowUpdate, changes_sql (UPDATE por linha, DELETE … IN)
crates/driver/src/lib.rs          Connection::execute_batch (BEGIN … COMMIT, ROLLBACK em erro ou contagem)
crates/engine/src/session.rs      apply_changes, preview_sql, ChangesApplied / ChangesFailed
crates/ui-egui/src/app.rs         DataTab.pending/deleted/editing/commit, change_set, start_edit, toggle_delete, open_commit
crates/ui-egui/src/data.rs        edição na célula, ⌫, chip "N alterações", travas de navegação
crates/ui-egui/src/record.rs      campos editáveis no Form, NULL, Reverter
crates/ui-egui/src/commit.rs      caixa de Commit: SQL, Executar, erro
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. driver e engine/session.rs são sensíveis: reviewer antes de cada commit deles.
3. Commit só com o "pode commitar" do Jean, na ordem: docs; catalog; driver; engine; ui.
4. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
