# @file: docs/features/FEAT-010-editor-sql/50-dev/impl-notes.md

# Notas de Implementação — FEAT-010 editor-sql

## Estado atual

_Atualizado em 22/10/2026._

- **Feito:** documentação (desenho aceito pelo Jean em 22/10); `sql` (`split`, `statement_at`, `highlight`,
  `beautify`; 10 testes).
- **Feito também:** `driver` (`execute` com limite por leitura, descartando as linhas a mais sem cancelar no
  servidor; `cancel` pelo CancelToken; `Cancelled` com o motivo); `engine` (`run_sql`, `cancel_sql`, `SqlStarted`/`SqlResult`/
  `SqlFinished`/`SqlBusy`, histórico em memória); `ui-egui` (`Tab::Query`, **+ SQL**/⌘T, editor com
  highlight, gutter e status, controles, resultados em sub-abas, JSON no painel, confirmação ao fechar).
- **Commitado:** 1 (`27e92ed`, teste do engine), 2 (`fd57509`, crate `sql` e dependências), 3 (`53415ea`,
  driver) e 4 (`fbd55b3`, engine).
- **Em curso:** commit 5 (UI, com os consertos da 6ª revisão) e 6 (docs da FEAT-011), aguardando o revisor da
  UI e o "pode commitar" do Jean.
- **Falta:** push, CI e verificação do Jean.
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

### 22/10/2026 — crate `sql`

- [x] `sqlparser 0.63` (Apache-2.0) e `sqlformat 0.5` (MIT) no workspace; `cargo-deny` verde.
- [x] `split.rs`: tokens com posição do `sqlparser` (dialeto PostgreSQL); `Lines` traduz `Location` (linha,
  coluna em caracteres) para byte. O `end` do span do tokenizador é exclusivo (a posição depois do token),
  apesar da doc dizer inclusivo; a coluna logo depois do último caractere da linha vale como posição.
  `statement_at`: o que contém o cursor, senão o anterior, senão o primeiro (cursor em bytes; o egui conta
  em caracteres, a UI converte).
- [x] `highlight.rs`: spans por token; lista própria de keywords (a do `sqlparser` cobre todos os dialetos e
  pintaria `id`, `name` e `value`); buffer com erro de tokenização colore o prefixo e deixa o resto `Plain`.
- [x] `format.rs`: `sqlformat` com indentação 2, keywords em maiúsculas, dialeto PostgreSQL; vazio volta vazio.
- [x] Conserto de teste intermitente do engine (`fechar_com_pagina_em_voo_nao_ressuscita_o_banco`): os três
  eventos podem chegar em qualquer ordem sob carga; o teste colhe três e confere o conjunto.

### 22/10/2026 — Revisão da crate `sql` (`process:reviewer`): dois altos, aplicados

- [x] **Alto:** `highlight` entrava em recursão infinita (estouro de pilha, não é pânico capturável) com
  `$$`, `$tag$`, `/*`, `U&'` ou operador inacabado sem fechar: o tokenizador aponta o erro para o fim do
  buffer, o prefixo era o buffer inteiro e a função se chamava com a mesma entrada. Agora
  `tokenize_with_location_into_buf` devolve os tokens lidos até o erro, sem recursão; o resto vira um span
  `Plain` (do erro, para string, ou do fim do último token). Teste com os dez casos da sonda do revisor.
- [x] **Alto:** o Beautify corrompia `$$…$$` (espaços, `;`, quebras) e inseria espaço em string aberta, e o
  texto substituía o buffer. Guarda de ida e volta: a saída só vale se tem os mesmos tokens da entrada (fora
  espaço e caixa de palavra não citada; comentários contam); senão a entrada volta intacta. Teste.
- [x] **Médio:** o fallback "buffer inteiro vira um statement" mandava ao servidor o que o usuário não pediu
  (`delete …; select 'ab` com o cursor na 2ª linha). Agora o que vem antes do erro divide normalmente e a
  cauda, do statement em curso até o fim, é o último statement com `complete = false` (o Run Current avisa
  "Statement incompleto" e roda mesmo assim: o servidor recusa). Teste.
- [x] `sqlparser` sem `recursive-protection` (tira `stacker`/`psm` com C e mais 4 crates da cadeia);
  keywords a mais (`FOR`, `CONFLICT`, `RECURSIVE`, `INTERVAL`, `RETURN`, `RAISE`…) e `eq_ignore_ascii_case`
  sem alocar; docs alinhados (brief com `complete`, "senão o primeiro", cursor em bytes).
- Verificado pelo revisor com fuzz de 300 mil buffers: `Lines::byte_at`, `split` e `statement_at` sem
  pânico, `range` sempre em fronteira de caractere; o conserto do teste do engine é honesto (a ordem entre
  `DatabaseClosed` e a página em voo é corrida real).

### 22/10/2026 — `driver`: `execute` e `cancel`

- [x] `Outcome { rows, affected, truncated }`; `execute(sql, limit)` por `simple_query_raw` (stream, `futures-util`
  só pelo trait); passou do limite → as linhas a mais são descartadas e o statement vai até o fim (sem
  cancel no servidor, decisão da 5ª revisão); `affected` só sem `RowDescription`.
- [x] `cancel()` pelo `CancelToken` guardado no `connect` (não precisa do lock que a query em curso segura),
  com o mesmo TLS numa conexão só para isso; `db_error` mapeia `57014` para `DriverError::Cancelled`.
- [x] Teste com servidor: `generate_series(1, 100000)` com limite 5 (5 linhas, truncado), SELECT vazio ainda é
  `rows`, INSERT com `affected`, erro de sintaxe sem derrubar a conexão, `pg_sleep(5)` cancelado em menos
  de 2 s com `Cancelled`.

### 22/10/2026 — `engine`: `run_sql`, `cancel_sql`, histórico

- [x] Em `session.rs` (não num `query.rs` à parte: usa `slots` e `commit_if_current`, que são privados).
  `Database.running: Option<Arc<AtomicBool>>` é a bandeira de cancelamento e a trava "uma execução por
  banco" (`SqlBusy`); `Database.history` com as últimas 200. `run_sql` emite `SqlStarted` na hora, um
  `SqlResult` por statement (para no primeiro erro; `Cancelled` marca `cancelled`) e `SqlFinished`.
- [x] `cancel_sql` liga a bandeira e chama o `cancel` do driver numa tarefa (o statement de agora volta
  `Cancelled`; os seguintes não rodam). Driver falso: "select N", "update", "erro", "sleep" (olha a bandeira
  a cada 10 ms). O teste espera o statement começar antes de cancelar (antes disso o cancel vence a corrida
  e a execução termina sem rodar nada, o que também é correto).

### 22/10/2026 — `ui-egui`: aba de SQL, editor e resultados

- [x] `Workspace.tabs: Vec<Tab>` com `Tab::Table(Box<DataTab>)` e `Tab::Query(QueryTab)`; `data()`/`data_mut()`
  são a aba ativa se é de tabela, `query()`/`query_mut()` se é de SQL; `tab_mut` e `query_tab_mut` roteiam
  os eventos por tabela ou por id da aba; `Tab::would_lose_work` (pendência, célula aberta ou texto no
  editor) e `Tab::busy` (Commit ou SQL rodando) alimentam ×/⌘W e a confirmação.
- [x] `editor.rs`: toolbar (limite `No limit`/100/500/1000, padrão 100; Beautify ⌘I; Run Current ⌘↩; Run
  All ⌘⇧↩; Cancelar; "executando… N s"; "transação aberta"; avisos), resultados embaixo, editor com
  `TextEdit::multiline().code_editor().lock_focus(true)` e `layouter` sobre `sql::highlight` (keyword na cor
  de destaque, string verde, número laranja, comentário apagado) com fundo leve no statement sob o cursor,
  gutter de números pintado ao lado, status `linha N, coluna M`. O cursor vem do estado do `TextEdit`
  (`CharIndex`, em caracteres) e vira byte para `statement_at`. Tab insere tabulação (não dois espaços).
- [x] `results.rs`: painel inferior redimensionável com sub-abas `Query N` (erro em vermelho), grid só leitura
  (`N rows · ms · truncado em L`), mensagem `OK, N linhas afetadas · ms` ou o erro com o statement; o número
  da linha é um botão e o ⌘-clique nele (ou na linha) manda o JSON ao painel direito (`record.rs`, tipos
  desconhecidos → strings).
- [x] `tabs.rs`: **+ SQL** e ⌘T; ⌘W vale com o editor em foco (o TextEdit não usa ⌘W), não com a paleta
  aberta. `rail::confirm` usa o título da aba. ↻ só zera a estrutura das abas de tabela.
- [x] Teste kittest `editor_sql_run_current_run_all_beautify_limite_e_fechar` e unitário do aviso de
  transação. Lições: o grid dentro de um `Panel::bottom` não recebe o clique de ponteiro no kittest (o nó
  existe, mas o hover falha), por isso o número da linha é botão e o teste clica por acessibilidade com
  `ModifiersChanged`; o ComboBox do limite é padrão do egui e o teste define o limite pelo estado.

## Sequência de commits (quando o Jean voltar)

Cada um com "pode commitar"; os sensíveis com o revisor rodado depois do commit anterior:

1. [x] `27e92ed` `fix(engine): teste de fechar banco com página em voo aceita os eventos em qualquer ordem`.
2. [x] `fd57509` `feat(sql): split por statement, statement sob o cursor, highlight e beautify com sqlparser
   e sqlformat (FEAT-010)` — `Cargo.toml`, `Cargo.lock`, `crates/sql/**`.
3. [x] `53415ea` `feat(driver): execute com limite por leitura e cancel pelo CancelToken (FEAT-010)` —
   `crates/driver/**`, stubs em `engine/session.rs` (testes) e `ui-egui/tree.rs` (testes).
4. [x] `fbd55b3` `feat(engine): run_sql, cancel_sql, eventos do editor e histórico em memória (FEAT-010)` —
   `crates/engine/**`.
5. `feat(ui): aba SQL Query com editor, Run Current/All, limite, Beautify, cancel e resultados (FEAT-010)` —
   `crates/ui-egui/**`, docs da FEAT-010, BRIEF §3.7.
6. `docs(feat-011): brief, aceite, notas e índice das queries salvas em arquivo` —
   `docs/features/FEAT-011-queries-salvas/**` e a renumeração do autocomplete (FEAT-012) no BRIEF e no brief
   da FEAT-010.

### 22/10/2026 — Revisão do driver, engine e UI (`process:reviewer`): aplicada

- [x] `DriverError::Cancelled(String)` com o motivo do servidor; o engine só marca `cancelled` quando a
  bandeira do usuário está ligada (um `statement_timeout` também devolve `57014` e é erro comum); teste.
- [x] Nada de clonar os resultados por frame: `editor.rs` clona só um cabeçalho e `results.rs` empresta a aba.
- [x] "transação aberta" é do banco (`Workspace.open_transaction`), não da aba: todas as abas SQL avisam e o
  Commit do grid recusa com a razão enquanto estiver aberta; `END TRANSACTION` e `ABORT` fecham.
- [x] Engine: `running` e histórico só do banco que começou a execução (`Arc::ptr_eq` da bandeira; fechado e
  reaberto é outro); `close_database` liga a bandeira e cancela no servidor.
- [x] `truncado em N` usa as linhas que ficaram, não o limite atual; `busy`/`connection_busy` (era
  `commit_running`) e "Aguarde a execução terminar"; confirmação da aba SQL fala em texto do editor.
- [x] ⌘W só sem campo em foco ou com o foco no editor da aba ativa (`editor::editor_id`), nunca noutro campo
  (em Linux/Windows Ctrl+W do TextEdit apaga palavra); ⌘I ignorado com execução em curso; ⌘↩ repetido e
  statement incompleto avisam.
- [x] Testes a mais: cancel pela UI (botão, aviso, × travado, ⌘↩ repetido), transação aberta e fechada na
  barra, `Cancelled` sem pedido no engine, `END TRANSACTION`/`ABORT`.
- [x] Decisão do Jean (22/10): fechar o banco ou desconectar com texto numa aba SQL pede confirmação, como
  fechar a aba (`Workspace::would_lose_work`); a caixa diz "tem texto no editor SQL" quando é só isso. Teste.
- Anotado para o M6: `statement_at` e `highlight` tokenizam o buffer duas vezes por frame (cache por texto
  quando o autocomplete entrar).

### 22/10/2026 — 4ª revisão (`process:reviewer`, depois do commit 1): aplicada

- [x] **Alta.** O cancel por limite do `execute` saía na linha `limit+1` e, sem TLS, chegava depois do fim
  do statement e derrubava o seguinte da mesma conexão (Run All parava com "canceling statement due to
  user request"; 29/30 no programa de prova). Causa: sem TLS, `cancel_query_raw` só escreve o pacote e
  volta; o filho do postmaster processa depois. Com TLS o filho já leu o pedido quando `cancel` volta, e
  o cancel chega com o backend ocioso, onde o servidor o descarta (por isso o teste com a URL `Prefer`
  não pegava). Conserto: as linhas a mais são descartadas e o cancel só sai depois de 1000 linhas ou 8 MiB
  descartados; se o cancel saiu e o stream acabou sem o `Cancelled`, um `pg_sleep(0.2)` na mesma conexão
  absorve o pedido em voo. Teste com conexão `TlsMode::Disable` (150 e 1200 linhas, limite 100, o
  statement seguinte passa); 0/30 no programa de prova depois.
- [x] **Média.** `transaction_state` apagava "transação aberta" em `ROLLBACK TO SAVEPOINT` e liberava o
  Commit do grid dentro da transação do editor. `ROLLBACK … TO` mantém o estado; teste.
- [x] **Baixa.** Cancelar do editor com Commit do grid em curso cancelava o lote (o cancel é por conexão):
  `run` recusa com "Há um Commit em curso neste banco"; teste.
- [x] **Baixa.** Run Current com statement incompleto cujo texto tem `;` levaria statements inteiros que o
  usuário não pediu (tokenizador erra em `(r)._id`, sintaxe que o servidor aceita): não roda, o aviso
  diz por quê; teste (nada vai ao servidor).
- [x] **Baixa.** `disconnect` não cancelava um `run_sql` em curso (o engine é a fachada do CLI e do MCP):
  liga a bandeira e cancela, como `close_database`; teste com contador de `cancel` no driver falso.
- [x] **Baixa.** `SqlResult` em voo de um banco fechado e reaberto mexia em `open_transaction`: só é
  aplicado quando a aba existe e está `running`; `drain_events` virou `apply_event` por evento (os testes
  injetam eventos); teste.
- [x] Comentário órfão em `request_close_tab`; aceite e brief dizem "menos de 2 s" como o teste; o índice
  aponta para esta sequência de commits; a FEAT-011 entrou como commit 6.

### 22/10/2026 — 5ª revisão (`process:reviewer`, sobre os consertos da 4ª): aplicada

- [x] **Alta.** Cancelar o statement no servidor por causa do limite, dentro de uma transação aberta,
  aborta a transação (qualquer ERROR, inclusive o 57014) e o erro era engolido como `truncated`; o COMMIT
  seguinte vira ROLLBACK (o tokio-postgres não expõe o tag nem o byte de estado do `ReadyForQuery`), e a
  UI mostrava "OK, 0 linhas afetadas": perda silenciosa. Decisão: **o limite não cancela mais nada**; o
  `execute` lê o statement até o fim e descarta as linhas a mais. Sai o descarte-antes-do-cancel e o
  `pg_sleep` absorvedor da 4ª revisão (viraram desnecessários). Custo: um `SELECT` sem `LIMIT` numa tabela
  enorme é lido inteiro antes de aparecer (o usuário tem o Cancelar). Alternativa anotada para o Jean:
  embrulhar `SELECT` simples em `LIMIT n+1` como o TablePlus faz, o que contraria o "roda como veio" do
  brief. Teste com servidor: `BEGIN` + tabela + `SELECT` de 200 000 linhas com limite 100 + `COMMIT`, a
  tabela tem de existir (falhava com "relation t_tx does not exist").
- [x] **Alta (complemento).** A UI não tem como ver `COMMIT` → `ROLLBACK`: qualquer erro com transação
  aberta liga `Workspace.transaction_aborted` e a barra mostra "transação abortada" (hover: o servidor
  desfaz tudo no COMMIT; rode ROLLBACK e refaça); fechar a transação limpa (um `BEGIN` dentro da transação
  abortada é recusado pelo servidor e mantém o aviso). Teste.
- [x] **Média.** Espelho do conserto anterior: `open_commit`/`execute_changes` recusam com erro na aba
  enquanto alguma aba SQL do banco está executando (um Cancelar do editor derrubaria o lote). Teste.
- [x] **Baixa.** `transaction_state` compara por palavras (`START  TRANSACTION`, quebra de linha); teste.
- [x] **Baixa.** `let closed = true; if closed` em `close_database` removido.
- Doc do trait `Connection::execute` e comentários dizem agora que o limite não cancela.

### 22/10/2026 — 6ª revisão (`process:reviewer`, sobre os consertos da 5ª): aplicada

- Driver e engine: nada bloqueia; `feat(driver)` commitado (`53415ea`).
- [x] **Média.** `transaction_aborted` ficava preso depois de `ROLLBACK TO SAVEPOINT` recuperar a transação
  (o aviso mandava rodar ROLLBACK e perder o que o savepoint salvou). `transaction_effect` distingue
  `Opens`/`Closes`/`Recovers`/`Keeps`; recuperar limpa o aviso. Teste.
- [x] **Baixa.** `COMMIT AND CHAIN`/`ROLLBACK AND CHAIN` contam como abrir (a sessão segue em transação).
- [x] **Baixa.** `COMMIT` de uma transação abortada avisa na aba: "o COMMIT virou ROLLBACK, nada foi gravado".
- [x] **Baixa.** Três passagens destas notas descreviam o cancel por limite removido; corrigidas.

### 22/10/2026 — 7ª revisão (`process:reviewer`, UI antes do commit 5): aplicada

- Engine commitado (`fbd55b3`) depois de "nada bloqueia" na passada de portão.
- [x] **Média.** `COMMIT AND CHAIN`/`END AND CHAIN` de transação abortada não avisava a perda:
  `TxEffect` virou `Opens | Commits { chain } | RollsBack { chain } | Recovers | Keeps` e
  `undoes_commit(effect, aborted)` decide o aviso por "era COMMIT/END", sem heurística inline. Teste.
- [x] **Baixa.** `AND NO CHAIN` contava como `AND CHAIN`: só a sequência `AND CHAIN` conta. Teste.
- [x] **Baixa.** Caixa de fechar banco/conexão dizia "tem texto no editor SQL" com uma célula aberta numa
  aba de tabela: `only_text` olha `would_lose_work` das abas de tabela.
