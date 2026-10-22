# @file: docs/features/FEAT-007-edicao-em-lote/10-prd/acceptance.md

# FEAT-007 — Critérios de aceite

## `catalog::edit`

- [x] `UPDATE` só com as colunas alteradas, `WHERE` por todas as colunas da chave, valores por `literal`
      (E'…', `'` e `\` dobrados) e `NULL` literal; identificador com aspas e maiúsculas preservado.
- [x] `DELETE … WHERE (pk) IN (…)` com chave simples e composta; sem linha para apagar, sem `DELETE`.
- [x] `ChangeSet` sem chave primária é erro, não SQL.

## `driver`

- [x] `execute_batch` roda `BEGIN`, os statements e `COMMIT`; erro no meio faz `ROLLBACK` e o erro traz o
      índice do statement (teste com servidor: a tabela fica como estava).
- [x] Contagem esperada diferente da real faz `ROLLBACK` sem `COMMIT` (teste com servidor).

## `engine`

- [x] `apply_changes` emite `ChangesApplied` com as contagens e esvazia o cache de stats da tabela (a UI
      recarrega página e stats); erro emite `ChangesFailed` com a mensagem do servidor, sem segredo; tabela
      sem chave primária é recusada antes de ir ao driver.
- [x] `preview_sql` devolve exatamente os statements que `apply_changes` manda ao driver (teste com driver
      falso que grava o SQL).
- [x] Eventos passam por `commit_if_current` (sessão reconectada não recebe o resultado).

## `ui-egui`

- [x] Tabela sem chave primária: aviso na linha de tamanhos; duplo clique e ⌫ não fazem nada.
- [x] Duplo clique na célula abre a edição; Enter confirma (célula destacada, chip `1 alteração`); Esc cancela.
- [x] Form: campo editável com NULL e Reverter; com várias linhas selecionadas, a edição vale para todas.
- [x] ⌫ marca as linhas selecionadas (riscadas); ⌫ de novo desmarca; o chip conta updates e deletes.
- [x] Com pendência, ordenar, paginar, recarregar e trocar de tabela ficam desabilitados com a dica.
- [x] Commit (botão ou ⌘S) abre a caixa com o SQL e a contagem; Executar manda o lote; sucesso fecha, limpa
      e recarrega; erro fica na caixa com o statement que falhou e as pendências continuam.
- [x] Descartar limpa tudo sem tocar no banco.
- [x] Teste `egui_kittest` cobre edição na célula, ⌫, Commit com o SQL, Executar com recarga e Descartar.

## Portão e CI

- [x] `scripts/quality-gate.sh --full` verde (22/10/2026, com `HY_DBPLUS_TEST_PG`).
- [ ] Workflow verde (push do Jean).

## Do Jean

- [ ] Editar uma célula e um campo do Form, apagar duas linhas, ver o SQL, executar, ver a página recarregada;
      provocar um erro (texto numa coluna `integer`) e ver que nada mudou no banco.
