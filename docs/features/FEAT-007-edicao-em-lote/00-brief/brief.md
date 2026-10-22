# @file: docs/features/FEAT-007-edicao-em-lote/00-brief/brief.md

# FEAT-007 — Edição em lote: Update e Delete numa transação, com o SQL à vista

## Problema

A aba Data mostra e seleciona linhas, mas não muda nada. O Jean pediu (22/10, capturas do TablePlus) Delete
sobre a seleção e Update direto na célula ou no painel. O BRIEF (decisão 6) fixou o recorte: Update e Delete
entram no MVP, só em tabela com chave primária, como edições pendentes confirmadas em lote numa transação
com o SQL à vista; `+ Row` continua fora.

## Objetivo

Editar como no TablePlus: duplo clique na célula ou campo do Form abre a edição; a mudança fica pendente
(célula marcada), nada vai ao banco; ⌫ marca as linhas selecionadas para exclusão (riscadas); **Commit**
abre o SQL exato que vai rodar, com a contagem (`3 updates, 2 deletes`), e **Executar** roda tudo numa
transação; erro desfaz tudo e mantém as pendências para corrigir; sucesso recarrega a página. **Descartar**
limpa as pendências. Tabela sem chave primária continua só leitura, com aviso.

## Escopo (Must)

1. **`catalog::edit`** (camada 1, puro, testado): SQL de uma `ChangeSet` para uma tabela com chave primária.
   - `UPDATE "s"."t" SET "col" = E'…', "outra" = NULL WHERE "pk" = E'…' [AND "pk2" = E'…']`, um por linha
     alterada, só com as colunas que mudaram.
   - `DELETE FROM "s"."t" WHERE ("pk"[, "pk2"]) IN ((E'…'[, E'…']), …)`, um só para todas as linhas.
   - Identificadores por `quote_ident`, valores por `literal` (E'…'), `NULL` literal. O valor vai como texto
     e o servidor converte para o tipo da coluna; texto que não cabe no tipo é erro do servidor, mostrado.
2. **`driver`**: `Connection::execute_batch(&[BatchStatement { sql, expected_rows: Option<u64> }]) ->
   Result<Vec<u64>, DriverError>`: `BEGIN`, cada statement pelo protocolo simples, `COMMIT`, numa transação
   só do lote (nenhuma outra query da conexão corre no meio). Erro de um statement ou contagem de linhas
   diferente da esperada faz `ROLLBACK` e devolve `DriverError::Statement`/`RowCount` com a posição (a
   partir de 1). Devolve as linhas afetadas de cada statement.
3. **`engine`**: `Sessions::apply_changes(id, database, schema, table, ChangeSet)`; monta o SQL pelo
   catálogo (`BatchStatement` com a contagem esperada: 1 por `UPDATE`, N no `DELETE`) e roda
   `execute_batch`; contagem diferente (linha mudou ou sumiu por outra sessão) é o driver que desfaz.
   Eventos: `ChangesApplied { schema, table, updated, deleted }` e `ChangesFailed { schema, table, error,
   position }` (`position` é o statement que falhou, para a caixa destacar). `preview_sql(schema, table,
   &ChangeSet) -> Result<Vec<String>, EditError>` puro, para a caixa de Commit (o mesmo texto que roda).
4. **`ui-egui`**:
   - Só com chave primária (`Column::is_primary_key` no cache): sem ela, a linha de tamanhos mostra "sem
     chave primária: só leitura" e nada entra em edição.
   - Célula: duplo clique abre um `TextEdit` no lugar do valor; Enter confirma, Esc cancela; a célula com
     pendência ganha fundo de destaque e o valor novo. `NULL` só pelo Form.
   - Form: cada campo vira `TextEdit` com botões **NULL** e **Reverter**; com seleção múltipla, o campo
     editado vale para todas as linhas selecionadas (é o "lote" do Update).
   - Delete: ⌫ (ou botão **Delete** no rodapé) marca as linhas selecionadas (riscadas em vermelho);
     ⌫ de novo desmarca.
   - Rodapé: chip `N alterações` com **Commit** (⌘S) e **Descartar** (⌘Z). Com pendência, ordenar, paginar,
     recarregar e trocar de tabela ficam desabilitados com a dica "commit ou descarte as alterações".
   - Caixa de Commit: título `3 updates, 2 deletes`, o SQL completo (monoespaçado, rolável, botão Copiar),
     **Executar** e **Cancelar**. Durante a execução, botões desabilitados. Erro aparece na caixa, com o
     statement que falhou; a transação já foi desfeita e as pendências continuam. Sucesso fecha a caixa,
     limpa as pendências e recarrega a página.
5. **Testes**: `catalog::edit` (escape de `'` e `\`, identificador com aspas, PK composta, `NULL`, só as
   colunas alteradas, ordem estável); `driver` atrás de `HY_DBPLUS_TEST_PG` (erro no 2º statement deixa a
   tabela como estava; contagem esperada errada faz rollback); `engine` com driver falso (eventos, SQL do
   preview igual ao executado, sem chave recusado antes do driver, lote vazio); UI com `egui_kittest` (duplo
   clique, digitar, Enter →
   `1 alteração`; ⌫ marca; Commit mostra o SQL; Executar → o driver falso recebe `BEGIN`, `UPDATE`,
   `DELETE`, `COMMIT`, a página recarrega; Descartar limpa; ordenar desabilitado com pendência; tabela sem
   PK não entra em edição).

## Won't

- `+ Row` (INSERT): fora do MVP (decisão 6).
- Tabela sem chave primária: só leitura (sem `ctid`, sem `WHERE` por todas as colunas).
- Verificação otimista dos valores antigos no `WHERE`: o lote confere só a contagem de linhas afetadas pela
  chave; conflito com outra sessão que mudou o mesmo valor vence o último (como o TablePlus).
- Editor por tipo (data, booleano, json): tudo é texto; fica para quando o grid tiver tipos nativos.
- Seleção de texto na célula (saiu na FEAT-006): o duplo clique agora é a edição.
- Pendências que sobrevivem a ordenar/paginar: as ações ficam desabilitadas até commit ou descarte.

## Restrições

- Nada de SQL com valor interpolado sem `literal`/`quote_ident`; o texto mostrado na caixa é o que roda.
- `crates/driver/src/`, `crates/engine/src/session.rs` e um módulo novo com "exec" no nome no engine são
  sensíveis: `process:reviewer` antes de cada commit deles. O `catalog::edit` e a UI não são.
- Commits, com o docs primeiro: `docs`; `feat(catalog)`; `feat(driver)`; `feat(engine)`; `feat(ui)`.
