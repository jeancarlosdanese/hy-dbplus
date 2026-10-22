# @file: docs/features/FEAT-007-edicao-em-lote/50-dev/impl-notes.md

# Notas de Implementação — FEAT-007 edicao-em-lote

## Estado atual

_Atualizado em 22/10/2026._

- **Feito:** documentação (desenho aceito pelo Jean em 22/10); `catalog::edit` (`ChangeSet`, `RowUpdate`,
  `Statement { sql, expected_rows }`, `changes_sql`, `EditError`; 4 testes).
- **Feito também:** `driver::execute_batch` (`BatchStatement { sql, expected_rows }`, transação pela API
  `Transaction` do tokio-postgres sob `RwLock`, `DriverError::Statement` e `RowCount`; teste com servidor).
- **Feito também:** `engine::apply_changes` e `preview_sql`, eventos `ChangesApplied`/`ChangesFailed`
  (teste com driver falso que grava os lotes).
- **Em curso:** revisão e commit do engine.
- **Falta:** `ui-egui`; verificação do Jean.
- **Depende de:** FEAT-006 (seleção múltipla, painel Form, `Column::is_primary_key` no cache de colunas).
- **Para retomar:** esta pasta, o brief (Must 1–5) e o `CLAUDE.md`.

## Decisões

- **[2026-10-22] SQL com literais, não parâmetros.** O lote roda pelo protocolo simples com `literal`
  (E'…') e `quote_ident`, os mesmos da leitura: o texto da caixa de Commit é exatamente o que o servidor
  recebe, e a CLI e o MCP (M8) mostram o mesmo SQL. Parâmetros (`$1`) dariam um texto "parecido" com o que
  roda; a escolha é a honestidade do preview. Valor vai como texto e o servidor converte para o tipo da
  coluna (literal de tipo desconhecido); erro de conversão é do servidor e aparece na caixa.
- **[2026-10-22] Contagem como verificação de concorrência.** `WHERE` só pela chave primária; cada `UPDATE`
  precisa afetar 1 linha e o `DELETE`, N. Contagem diferente (linha apagada ou chave mudada por outra sessão)
  faz `ROLLBACK`. Não se confere o valor antigo (TablePlus também não); fica anotado para depois.
- **[2026-10-22] Pendência trava a navegação.** Ordenar, paginar, recarregar e trocar de tabela ficam
  desabilitados enquanto há alteração pendente: a pendência é por índice da página (igual à seleção da
  FEAT-006) e a chave de cada linha é lida da página no momento do Commit. Modal "commit, descartar ou
  cancelar" fica para quando incomodar.
- **[2026-10-22] Um `DELETE` para todas as linhas, um `UPDATE` por linha.** `UPDATE` em lote com
  `CASE`/`VALUES` esconderia o SQL; um por linha é legível e o preview é a lista. `DELETE … IN` é um só
  porque é a mesma operação em todas.
- **[2026-10-22] `NULL` só pelo Form.** Na célula, texto é texto (uma string vazia é `''`). O botão NULL no
  Form é explícito e não precisa de atalho secreto.
- **[2026-10-22] O lote roda sozinho na conexão.** O tokio-postgres enfileira as queries de várias tarefas na
  mesma conexão: com `BEGIN` por `simple_query`, a query de stats ou de página de outra tarefa entraria na
  transação e um erro dela abortaria o lote sem aviso (o `COMMIT` de transação abortada vira `ROLLBACK`
  silencioso). O `Client` fica num `tokio::sync::RwLock`: leituras pegam a leitura (seguem concorrentes) e
  `execute_batch` pega a escrita e usa `Client::transaction` (que exige `&mut`): nada mais corre no meio.
- **[2026-10-22] Camadas.** SQL no `catalog` (puro, testado), transação no `driver`, orquestração e eventos
  no `engine`, estado pendente na UI (`DataTab.pending`, `DataTab.deleted`, `DataTab.editing`).

## Plano

1. `catalog::edit`: `ChangeSet`, `RowUpdate`, `changes_sql` → `Vec<BatchStatement>` (o tipo do driver, um só
   para as duas crates) → testes puros. Commit `feat(catalog)`.
2. `driver`: `execute_batch(&[BatchStatement { sql, expected_rows: Option<u64> }])` → testes com servidor atrás de
   `HY_DBPLUS_TEST_PG`. Reviewer. Commit `feat(driver)`.
3. `engine`: `apply_changes`, `preview_sql`, eventos, recarga da página → testes com driver falso. Reviewer.
   Commit `feat(engine)`.
4. `ui-egui`: edição na célula e no Form, ⌫, chip e caixa de Commit, travas → kittest. Commit `feat(ui)`.
5. Docs: aceite marcado, notas, índice; BRIEF §3.4 (edição) se precisar.

## Log

### 22/10/2026 — Criação

- [x] Brief, aceite, notas e índice a partir da decisão 6 do BRIEF e do pedido do Jean na conferência do M3.

### 22/10/2026 — `catalog::edit`

- [x] `changes_sql(schema, table, &ChangeSet) -> Result<Vec<Statement>, EditError>`: um `UPDATE` por linha
  com colunas em `set` (linha sem coluna alterada não gera statement), `DELETE … IN` único por último;
  `Statement.expected_rows` (1 por `UPDATE`, N no `DELETE`) é o que o engine confere. Sem chave primária
  ou chave com número de valores errado → `EditError` (sem SQL). Valor `None` → `NULL` literal.

### 22/10/2026 — `driver::execute_batch`

- [x] `Connection::execute_batch(&[BatchStatement]) -> Result<Vec<u64>>`: transação só do lote; erro de um
  statement → `ROLLBACK` e `DriverError::Statement { position, reason }`; contagem diferente da esperada →
  `ROLLBACK` e `DriverError::RowCount { position, expected, got }`; sucesso → `COMMIT` e as contagens.
- [x] Teste com servidor (`HY_DBPLUS_TEST_PG`): texto em coluna `integer` no 2º statement desfaz o 1º;
  contagem errada desfaz; a conexão segue servindo depois do erro; lote bom grava e devolve `[1, 2]`.
- [x] Drivers falsos do engine e da UI implementam o método devolvendo erro (o engine ganha o seu no passo 3).

### 22/10/2026 — Revisão do driver (`process:reviewer`, 1ª rodada): nada bloqueante

- [x] Um tipo só para o statement do lote: `catalog::edit` passa a devolver `hy_dbplus_driver::BatchStatement`
  (com `Some(n)`) em vez do seu `Statement`; o engine não converte nada.
- [x] `DriverError::Statement.reason` sem o prefixo "erro do banco:" (mensagem crua do servidor); conexão
  caída no meio do lote volta como `DriverError::Closed`, não embrulhada.
- [x] Doc do trait: erro no `BEGIN`/`COMMIT` sobe como `Db`/`Closed`; cada `sql` é um statement só (a
  contagem é do último `CommandComplete`; `SELECT` conta as linhas devolvidas); sem `COMMIT`/`ROLLBACK`
  no texto. Para a UI: erro do `COMMIT` com conexão caída é resultado desconhecido; não dizer "nada gravado".
- [x] Teste da exclusividade: lote com `pg_sleep` em voo numa tarefa e `query_text` na mesma conexão; com o
  `RwLock` a leitura espera o `ROLLBACK` e vê o valor antigo; sem ele entraria na transação.
- [x] "stream e cancelamento no M3" → M5 também na doc de `PostgresConnection` e no BRIEF §5.
- Aceito sem mudar: o teste com servidor deixa o schema `hy_batch_*` para trás se uma asserção falhar antes
  do `DROP` (mesmo padrão dos testes do catálogo).

### 22/10/2026 — `engine::apply_changes`

- [x] `Sessions::apply_changes(id, database, schema, table, &ChangeSet)`: SQL por `catalog::changes_sql` (o
  mesmo de `preview_sql`); `EditError` (sem chave) vira `ChangesFailed` na hora, sem tarefa; o lote vai ao
  driver numa tarefa e o resultado passa por `commit_if_current`. Sucesso esvazia o cache de stats da
  tabela (a contagem mudou) e emite `ChangesApplied { updated, deleted }`; erro emite `ChangesFailed` com o
  texto do `DriverError`. Lote vazio sai como `ChangesApplied` com zeros, sem ir ao banco.
- [x] `ChangesFailed { error: String, position: Option<usize> }`: o texto de `EditError` ou `DriverError` para
  mostrar, e o statement que falhou (de `DriverError::Statement`/`RowCount`) para a caixa destacar no SQL.
- [x] Teste: o driver falso grava os lotes; o SQL enviado é igual ao `preview_sql`, com `Some(1)`/`Some(1)`;
  stats pedidos de novo vão ao banco; `qtd = 'x'` vira `ChangesFailed` com "statement 1"; sem chave não
  chega ao driver; disconnect com o lote em voo não entrega o resultado.
- [x] UI: braço `ChangesApplied | ChangesFailed => {}` só para o `match` compilar; o passo 4 o preenche.

### 22/10/2026 — Revisão do engine (`process:reviewer`, 1ª rodada): nada bloqueante

- [x] `position` no `ChangesFailed` (ver acima), decidido antes de a UI consumir o evento.
- [x] Lote com chave e sem statement: `ChangesApplied` com zeros na hora, sem tarefa e sem esvaziar o cache
  de stats (teste: stats seguem do cache). Sem chave continua `ChangesFailed`.
- [x] `ChangeSet::update_count()` no catálogo: a regra "linha com `set` não vazio é um UPDATE" num lugar só,
  usada por `is_empty`, por `changes_sql` (implícito) e pelo `updated` do engine.
- [x] Docs: brief Must 3 com a assinatura real de `preview_sql` e `ChangesFailed`; doc do `catalog::edit`
  dizia que o engine confere a contagem (é o driver); "rollback por contagem" saiu dos testes do engine (a
  prova real é o teste com servidor do driver).
- Para o passo 4: ao receber `ChangesApplied`, pedir os stats sempre (não reaproveitar `retry_data`, que só
  os pede quando `tab.stats` está vazio).
