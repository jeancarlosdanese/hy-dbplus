# @file: docs/features/FEAT-010-editor-sql/10-prd/acceptance.md

# FEAT-010 — Critérios de aceite

## `sql`

- [ ] `split` separa por `;` ignorando `;` em string, `$$…$$`, comentários e identificador entre aspas; vazio
      não entra; o último sem `;` entra; `range` e `line` batem com o buffer.
- [ ] `statement_at` devolve o statement sob o cursor, o anterior no espaço entre dois, `None` em buffer vazio.
- [ ] `highlight` classifica keyword, string, número, comentário, identificador e operador; buffer incompleto
      (string sem fechar) não falha.
- [ ] `beautify` formata um SELECT com join em várias linhas e keywords em maiúsculas; SQL inválido volta como
      estava.

## `driver`

- [ ] `execute` devolve `rows` para SELECT (com `truncated` quando o limite corta), `affected` para
      UPDATE/DELETE/DDL, erro do servidor para SQL inválido; a conexão segue servindo depois do erro.
- [ ] `cancel` interrompe um `pg_sleep(5)` em menos de 1 s e o statement volta com erro de cancelamento.

## `engine`

- [ ] `run_sql` emite `SqlStarted`, um `SqlResult` por statement (na ordem, com `elapsed`) e `SqlFinished`;
      para no primeiro erro; segunda execução no mesmo banco enquanto a primeira roda é recusada com aviso.
- [ ] `cancel_sql` cancela o statement em curso e não roda os seguintes; `SqlFinished { cancelled: true }`.
- [ ] Histórico em memória guarda as últimas 200 execuções com sql, instante, duração e resultado.

## `ui-egui`

- [ ] **+ SQL** (⌘T) abre `SQL Query N` no banco ativo; × e ⌘W fecham; buffer não vazio pede confirmação.
- [ ] Editor com highlight, números de linha, `linha N, coluna M`; o statement sob o cursor fica destacado.
- [ ] Run Current (⌘↩) roda só o statement do cursor; Run All (⌘⇧↩) roda todos em ordem e para no erro;
      Cancelar interrompe; Beautify (⌘I) formata o buffer; o limite trunca e o rodapé avisa.
- [ ] Resultados em sub-abas `Query N`: grid só leitura com `N rows · tempo`, mensagem `OK, N linhas
      afetadas · tempo`, ou erro em vermelho; ⌘-clique numa linha mostra o JSON no painel direito.
- [ ] `BEGIN` sem `COMMIT`/`ROLLBACK` deixa o aviso "transação aberta" na barra do editor.
- [ ] Teste `egui_kittest` cobre abrir aba, Run Current, Run All com erro, Beautify, limite e fechar com
      confirmação.

## Portão e CI

- [ ] `scripts/quality-gate.sh --full` verde (com `HY_DBPLUS_TEST_PG`); workflow verde.

## Do Jean

- [ ] Abrir uma aba SQL, escrever duas queries, rodar a do cursor e as duas, cancelar um `pg_sleep`, formatar,
      ver o resultado truncado e o JSON de uma linha.
