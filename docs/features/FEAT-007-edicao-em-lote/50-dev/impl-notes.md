# @file: docs/features/FEAT-007-edicao-em-lote/50-dev/impl-notes.md

# Notas de Implementação — FEAT-007 edicao-em-lote

## Estado atual

_Atualizado em 22/10/2026._

- **Feito:** documentação (desenho aceito pelo Jean em 22/10); `catalog::edit` (`ChangeSet`, `RowUpdate`,
  `Statement { sql, expected_rows }`, `changes_sql`, `EditError`; 4 testes).
- **Em curso:** `driver::execute_batch`.
- **Falta:** `engine::apply_changes`; `ui-egui`; verificação do Jean.
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
- **[2026-10-22] Camadas.** SQL no `catalog` (puro, testado), transação no `driver`, orquestração e eventos
  no `engine`, estado pendente na UI (`DataTab.pending`, `DataTab.deleted`, `DataTab.editing`).

## Plano

1. `catalog::edit`: `ChangeSet`, `RowUpdate`, `changes_sql` → testes puros. Commit `feat(catalog)`.
2. `driver`: `execute_batch(statements, expected: &[Option<u64>])` → testes com servidor atrás de
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
