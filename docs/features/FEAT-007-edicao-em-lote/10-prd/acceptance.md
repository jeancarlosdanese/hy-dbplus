# @file: docs/features/FEAT-007-edicao-em-lote/10-prd/acceptance.md

# FEAT-007 — Critérios de aceite

## `catalog::edit`

- [ ] `UPDATE` só com as colunas alteradas, `WHERE` por todas as colunas da chave, valores por `literal`
      (E'…', `'` e `\` dobrados) e `NULL` literal; identificador com aspas e maiúsculas preservado.
- [ ] `DELETE … WHERE (pk) IN (…)` com chave simples e composta; sem linha para apagar, sem `DELETE`.
- [ ] `ChangeSet` sem chave primária é erro, não SQL.

## `driver`

- [ ] `execute_batch` roda `BEGIN`, os statements e `COMMIT`; erro no meio faz `ROLLBACK` e o erro traz o
      índice do statement (teste com servidor: a tabela fica como estava).
- [ ] Contagem esperada diferente da real faz `ROLLBACK` sem `COMMIT` (teste com servidor).

## `engine`

- [ ] `apply_changes` emite `ChangesApplied` com as contagens e esvazia o cache de stats da tabela (a UI
      recarrega página e stats); erro emite `ChangesFailed` com a mensagem do servidor, sem segredo; tabela
      sem chave primária é recusada antes de ir ao driver.
- [ ] `preview_sql` devolve exatamente os statements que `apply_changes` manda ao driver (teste com driver
      falso que grava o SQL).
- [ ] Eventos passam por `commit_if_current` (sessão reconectada não recebe o resultado).

## `ui-egui`

- [ ] Tabela sem chave primária: aviso na linha de tamanhos; duplo clique e ⌫ não fazem nada.
- [ ] Duplo clique na célula abre a edição; Enter confirma (célula destacada, chip `1 alteração`); Esc cancela.
- [ ] Form: campo editável com NULL e Reverter; com várias linhas selecionadas, a edição vale para todas.
- [ ] ⌫ marca as linhas selecionadas (riscadas); ⌫ de novo desmarca; o chip conta updates e deletes.
- [ ] Com pendência, ordenar, paginar, recarregar e trocar de tabela ficam desabilitados com a dica.
- [ ] Commit (botão ou ⌘S) abre a caixa com o SQL e a contagem; Executar manda o lote; sucesso fecha, limpa
      e recarrega; erro fica na caixa com o statement que falhou e as pendências continuam.
- [ ] Descartar limpa tudo sem tocar no banco.
- [ ] Teste `egui_kittest` cobre edição na célula, ⌫, Commit com o SQL, Executar com recarga e Descartar.

## Portão e CI

- [ ] `scripts/quality-gate.sh --full` verde (com `HY_DBPLUS_TEST_PG`); workflow verde.

## Do Jean

- [ ] Editar uma célula e um campo do Form, apagar duas linhas, ver o SQL, executar, ver a página recarregada;
      provocar um erro (texto numa coluna `integer`) e ver que nada mudou no banco.
