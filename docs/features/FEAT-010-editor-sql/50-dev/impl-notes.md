# @file: docs/features/FEAT-010-editor-sql/50-dev/impl-notes.md

# Notas de Implementação — FEAT-010 editor-sql

## Estado atual

_Atualizado em 22/10/2026._

- **Feito:** documentação.
- **Em curso:** discussão do desenho com o Jean.
- **Falta:** `sql` (split, statement_at, highlight, beautify); `driver` (execute com limite, cancel); `engine`
  (run_sql, cancel_sql, eventos, histórico em memória); `ui-egui` (aba de SQL, editor, controles,
  resultados); verificação do Jean.
- **Depende de:** FEAT-008 (abas por banco), FEAT-007 (`RwLock` da conexão).
- **Para retomar:** esta pasta, BRIEF §3.7 e a captura do TablePlus da FEAT-008 (aba SQL Query com Query 1/2/3).

## Decisões

- **[2026-10-22] Tokenizador do `sqlparser` para split e highlight.** Um só motor para dividir statements e
  colorir, tolerante a buffer incompleto; o parser completo fica para o autocomplete (M6). `sqlformat` para o
  Beautify: não vale escrever formatador.
- **[2026-10-22] Limite por leitura, não por `LIMIT` no SQL.** O editor executa o statement como o usuário
  escreveu; o driver lê no máximo N linhas do stream e descarta o resto, avisando `truncated`. Envolver em
  `SELECT * FROM (…) LIMIT n` mudaria o plano e quebraria statements que não são SELECT.
- **[2026-10-22] Um statement por vez, Run All para no erro.** Como o `psql` sem `ON_ERROR_STOP` desligado: o
  usuário vê até onde foi. Sem transação implícita: cada statement é autocommit na conexão do banco; `BEGIN`
  explícito fica aberto até o usuário fechar (aviso na barra).
- **[2026-10-22] Cancel pelo `CancelToken`.** Abre uma conexão só para o cancel, com o mesmo TLS; o statement
  em curso volta com erro `canceling statement due to user request`, que vira `SqlResult` de erro e
  `SqlFinished { cancelled: true }`.
- **[2026-10-22] Abas heterogêneas.** `Workspace.tabs: Vec<Tab>` (`Table` | `Query`); o que é de tabela
  (`data()`, pendências, estrutura) passa a filtrar `Tab::Table`; a aba de SQL tem estado próprio
  (`QueryTab`). Histórico em memória por sessão no engine, para o M7 só persistir.

## Plano

1. `sql`: `split`, `statement_at`, `highlight`, `beautify` com `sqlparser` + `sqlformat` → testes puros.
   Reviewer (split é sensível). Commit `feat(sql)`.
2. `driver`: `execute(sql, limit)` por `simple_query_raw`, `Outcome`, `cancel` → testes com servidor.
   Reviewer. Commit `feat(driver)`.
3. `engine`: `run_sql`, `cancel_sql`, `SqlStarted`/`SqlResult`/`SqlFinished`, histórico → driver falso.
   Reviewer. Commit `feat(engine)`.
4. UI passo 1: `Tab` enum, aba SQL com editor (highlight, gutter, status), controles, execução e a mensagem
   de resultado → kittest. Reviewer. Commit `feat(ui)`.
5. UI passo 2: resultados em sub-abas com grid, truncado, JSON no painel, aviso de transação aberta,
   confirmação ao fechar → kittest. Reviewer. Commit `feat(ui)`.
6. Docs: aceite, notas, índice; BRIEF §3.7 com o que ficou de fora.

## Log

### 22/10/2026 — Criação

- [x] Brief, aceite, notas e índice a partir do BRIEF §3.7, do marco M5 e da captura do TablePlus.
