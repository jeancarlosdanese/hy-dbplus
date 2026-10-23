# @file: docs/features/FEAT-010-editor-sql/00-brief/brief.md

# FEAT-010 — Editor SQL em abas: highlight, Run Current, Run All, limite, Beautify e resultado

## Problema

O app navega e edita dados, mas não executa SQL escrito pelo usuário, que é o segundo uso principal do
produto (BRIEF §1 e §3.7, marco M5). O TablePlus abre uma aba "SQL Query" ao lado das abas de tabela, com
highlight, várias queries no buffer, **Run Current** (a query sob o cursor), limite de linhas, **Beautify**
e o resultado em grid com uma aba por query executada.

## Objetivo

Uma aba de SQL por banco (quantas quiser), ao lado das abas de tabela: editor com highlight e numeração de
linhas, barra de status `linha, coluna`, controles **limite** (No limit / 100 / 500 / 1000), **Beautify
⌘I**, **Run Current ⌘↩**, **Run All ⌘⇧↩** e **Cancelar**; resultado abaixo do editor, uma sub-aba por
statement executado (`Query 1`, `Query 2`, …) com grid ou mensagem (`OK, 3 linhas afetadas, 12 ms`) e o
erro do servidor no lugar certo. Executa qualquer statement (BRIEF §4: o editor é a exceção ao "só
leitura"). Autocomplete é o M6 (FEAT-012); histórico persistido é o M7.

## Escopo (Must)

1. **`sql`** (camada 2, puro, testado), com `sqlparser` (dialeto PostgreSQL) para tokenizar:
   - `split(buffer) -> Vec<Statement { sql, range: Range<usize> (bytes), line: usize, complete: bool }>`:
     divide por `;` fora de string, `$$…$$`, comentário `--` e `/* */` e identificador entre aspas;
     statement vazio não entra; o último sem `;` entra. O que vem antes de um erro do tokenizador (string,
     `$$` ou `/*` sem fechar) divide normalmente; a cauda vira o último statement com `complete = false`.
   - `statement_at(buffer, cursor em bytes) -> Option<Statement>`: o statement que contém o cursor, senão o
     anterior (cursor no espaço entre dois, como o TablePlus), senão o primeiro.
   - `highlight(buffer) -> Vec<Span { range, kind: Keyword | String | Number | Comment | Identifier |
     Operator }>` pelo tokenizador, tolerante a buffer incompleto (nunca falha: texto que não tokeniza vira
     `Plain`).
   - `beautify(sql) -> String` com `sqlformat` (indentação de 2, keywords em maiúsculas), com guarda de ida e
     volta: a saída só vale se tem os mesmos tokens da entrada (fora espaço e caixa); senão a entrada volta
     intacta (o `sqlformat` reescreve `$$…$$` e string aberta).
2. **`driver`**: `Connection::execute(sql, limit: Option<usize>) -> Result<Outcome, DriverError>` com
   `Outcome { rows: Option<TextRows>, affected: Option<u64>, truncated: bool }`: um statement pelo protocolo
   simples via `simple_query_raw` (stream), lendo no máximo `limit` linhas e descartando o resto
   (`truncated = true`) sem cancelar no servidor (dentro de uma transação o cancel a abortaria; 5ª revisão); `rows` para quem devolve `RowDescription`, `affected` para o resto.
   `Connection::cancel() -> Result<(), DriverError>` pelo `CancelToken` do tokio-postgres (abre uma conexão
   nova só para o cancel, com o mesmo TLS). O `RwLock` do lote (FEAT-007) vale: a query do editor pega a
   leitura.
3. **`engine`**: `Sessions::run_sql(id, database, tab: QueryTabId, statements: Vec<String>, limit)` executa
   em sequência numa tarefa, emitindo `SqlStarted { tab, total }`, por statement `SqlResult { tab, index, sql,
   result: Ok(Outcome) | Err(DriverError), elapsed }` e `SqlFinished { tab, cancelled }`; erro num statement
   interrompe os seguintes (Run All para no erro, como o `psql`). `Sessions::cancel_sql(id, database)` chama
   o `cancel` do driver e marca a tarefa para parar entre statements. Uma execução por banco de cada vez.
   Histórico em memória por sessão (`Vec<HistoryEntry { sql, started, elapsed, ok }>`, últimas 200) para o
   M7 persistir.
4. **`ui-egui`**:
   - `Workspace.tabs` vira `Vec<Tab>` com `Tab::Table(DataTab) | Tab::Query(QueryTab)`; a barra de abas
     ganha **+ SQL** (⌘T) que abre `SQL Query N`; ×/⌘W fecham (buffer não vazio pede confirmação).
   - Editor (`editor.rs`): `TextEdit::multiline` com `layouter` que pinta pelo `sql::highlight` (cores do
     tema), gutter com números de linha, fonte monoespaçada, Tab insere tabulação; barra de status
     `linha N, coluna M`; o statement sob o cursor fica com fundo leve (Run Current mostra o que vai rodar).
   - Controles: limite (`No limit`, 100, 500, 1000; padrão 100), **Beautify ⌘I** (todo o buffer),
     **Run Current ⌘↩**, **Run All ⌘⇧↩**, **Cancelar** (só com execução em curso); "executando…" com o
     tempo decorrido; "transação aberta" (por banco: a transação é da conexão) depois de um `BEGIN` sem
     fim, e o Commit do grid espera ela fechar.
   - Resultados (`results.rs`): painel inferior redimensionável com sub-abas `Query 1 … N` (a última
     executada ativa) e, em cada uma, o grid virtualizado só leitura (colunas e linhas em texto; `NULL`
     em itálico) com rodapé `N rows · 12 ms` (`· truncado em 100` quando o limite cortou), ou a mensagem
     `OK, 3 linhas afetadas · 12 ms`, ou o erro em vermelho com o statement. Ctrl/⌘-clique numa linha abre o
     JSON no painel direito (reaproveita `record::to_json`); sem edição.
   - `StructureLoaded`, pendências e tudo da aba de tabela continuam iguais; `has_pending_changes` olha só as
     abas de tabela; execução em curso trava fechar a aba de SQL e o banco; fechar o banco ou desconectar com
     texto numa aba SQL pede confirmação, como fechar a aba (decisão do Jean, 22/10). Superado pela FEAT-011:
     o texto passou a ser salvo em arquivo e fechar o banco ou desconectar não pergunta mais por ele.
5. **Testes**: `sql` puro (split com string/`$$`/comentários/aspas, cursor entre statements, highlight de
   buffer incompleto, beautify); `driver` com servidor (`execute` de SELECT com limite e truncado, de
   UPDATE com `affected`, erro de sintaxe; `cancel` de `pg_sleep(5)` volta em menos de 2 s com erro de
   cancelamento); `engine` com driver falso (eventos em ordem, parada no erro, cancel, histórico); UI com
   `egui_kittest` (abrir aba SQL, digitar duas queries, Run Current roda só a do cursor, Run All roda as
   duas com duas sub-abas, erro na segunda, Beautify muda o buffer, limite truncando, fechar aba com texto
   pede confirmação).

## Won't

- Autocomplete (M6, FEAT-012); histórico persistido e tela History (M7); Export; gráfico (Chart).
- Editar o resultado no grid (o resultado de SQL livre não tem chave conhecida).
- Transação explícita por aba (`BEGIN` sem `COMMIT` fica aberto na conexão do banco até o usuário mandar
  `COMMIT`/`ROLLBACK`; o editor avisa "transação aberta" quando o último statement foi `BEGIN` sem fim).
- Vários statements em paralelo; Run Current com seleção de texto (roda o statement do cursor).

## Restrições

- Dependências novas (decisão): `sqlparser` (Apache-2.0; já previsto no BRIEF §6), sem a feature
  `recursive-protection` (é do parser, que o tokenizador não usa, e puxaria `stacker`/`psm` com C), e
  `sqlformat` (MIT) com `futures-util` já transitivo; licenças das transitivas permitidas no `deny.toml`;
  `Cargo.lock` muda.
- `crates/driver/src/`, `crates/engine/src/session.rs` e o módulo novo do engine com "quer"/"run" no nome
  são sensíveis: reviewer antes de cada commit deles. `crates/sql/src/` com `split`/`statement` no nome
  também é sensível (decide o que o Run Current executa). UI passa pelo reviewer pelo tamanho.
- Commits: `docs`; `feat(sql)`; `feat(driver)`; `feat(engine)`; `feat(ui)` em dois passos (aba de SQL com
  editor e execução; resultados com sub-abas e JSON).
