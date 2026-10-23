# @file: docs/features/FEAT-013-gerar-sql-pela-arvore/50-dev/impl-notes.md

# Notas de Implementação — FEAT-013 gerar-sql-pela-arvore

## Estado atual

_Atualizado em 23/10/2026._

- **Feito:** docs (`a810dc7`); `catalog::generate` (`Template`, `template_sql`, `example_value`; 4 testes +
  teste cruzado com `statement_at` na crate `sql`); UI (menu de contexto na árvore, `generate_sql`,
  `pending_sql`, conclusão em `ColumnsLoaded`/`StructureLoaded`, `insert_sql` com cursor e foco; kittest
  `menu_da_tabela_gera_sql_na_aba_sql_sem_executar`). Portão `--full` verde; duas rodadas de reviewer
  aplicadas; na árvore aguardando "pode commitar" (dois commits: catalog, ui).
- **Falta:** push e conferência do Jean (aceite "Do Jean").
- **Depende de:** FEAT-009 (colunas e estrutura no catálogo, `table_ddl`), FEAT-010 (abas SQL) e
  FEAT-011 (autosave), todas commitadas.
- **Para retomar:** este arquivo, o brief, `crates/catalog/src/{edit,data,ddl}.rs` como molde de montagem
  de SQL, `crates/ui-egui/src/tree.rs` (linha da tabela) e `App::open_query_tab`.

## Decisões

- **[2026-10-23] Modelos montados no `catalog`, não na UI**: é SQL a partir do modelo do catálogo, como
  `page_sql`, `changes_sql` e `table_ddl`; puro e testável sem egui, e o CLI/MCP (M8) pode reaproveitar.
- **[2026-10-23] Nada roda pelo menu** (decisão do Jean no desenho): o menu só escreve no editor. O
  `DROP CASCADE` e o `TRUNCATE` ficam a um ⌘↩ de distância, mas é o usuário que aperta.
- **[2026-10-23] Destino do texto**: aba SQL ativa → última aba SQL do banco → aba nova. Sempre no fim do
  buffer com uma linha em branco antes, nunca no cursor: inserir no meio de um statement em edição faria
  o Run Current rodar uma mistura.
- **[2026-10-23] Colunas sob demanda** pelo mesmo `load_columns` da árvore, com um pedido pendente por
  banco (`Workspace.pending_sql: Option<PendingSql { schema, table, item }>`); pedido novo substitui o
  anterior. O destino (aba SQL) é escolhido na inserção, não no pedido: se o usuário trocou de aba entre o
  clique e o evento, o texto vai para a aba SQL ativa naquele momento. CREATE usa `load_structure`; o `StructureLoaded` hoje só alimenta a aba Data da
  tabela (`app.rs:854`), passa a conferir o pendente também.
- **[2026-10-23] Valores de exemplo por tipo** (`data_type` do `format_type`, em minúsculas): tipo sem
  typmod casa por igualdade (`text`, `citext`, `name` → `''`; `integer`, `bigint`, `smallint`, `real`,
  `double precision`, `money` → `0`; `boolean` → `false`; `date` → `now()`; `uuid` → `gen_random_uuid()`;
  `json`, `jsonb` → `'{}'`; `bytea` → `'\x'`); prefixo só nos que levam typmod ou qualificador
  (`character…`, `varchar…`, `numeric…`, `decimal…`, `time…`/`timestamp…` → `now()`); array → `'{}'`;
  resto → `NULL` (`daterange`, `jsonpath`, `interval`, `"char"`: a 1ª revisão pegou `date` engolindo
  `daterange`).
- **[2026-10-23] Lista do menu fechada com o Jean** (23/10): SELECT, SELECT count, INSERT, UPDATE, DELETE,
  TRUNCATE, DROP CASCADE, CREATE; sem `SELECT … WHERE pk`, sem `COPY`.

- **[2026-10-23] Aspas sempre** (`quote_ident`), como `page_sql`, `changes_sql` e o DDL: "aspas só quando
  precisa" exigiria a lista de palavras reservadas, que é da crate `sql` (camada acima). O brief foi
  ajustado.
- **[2026-10-23] `Template` sem `Create`**: o `CREATE` é o `table_ddl` da estrutura; a UI o trata como
  item à parte do menu em vez de um modelo que `template_sql` não conseguiria montar só com colunas.
- **[2026-10-23] Coluna gerada fica fora** do `INSERT` e do `UPDATE` (o servidor recusa); identidade
  `BY DEFAULT` entra como `DEFAULT`.

## Plano

1. `catalog::generate`: `Template`, `template_sql`, valores por tipo, aspas, view → testes. Reviewer.
   Commit `feat(catalog)`.
2. `ui-egui`: `context_menu` na linha da tabela/view; `App::generate_sql` + `pending_sql` + conclusão em
   `ColumnsLoaded`/`StructureLoaded`/falhas; inserir no fim, cursor (gravar no `TextEdit::State` do
   editor, como o `cursor` é lido hoje) e foco → kittest. Reviewer. Commit `feat(ui)`.
3. Docs: aceite, notas, índice; BRIEF §3.3.

## Riscos e lições a conferir

- `egui_kittest`: o clique secundário no nó (`click_secondary`/evento `PointerButton::Secondary`) e o
  menu de contexto como `Area` — conferir como os itens aparecem na árvore de acessibilidade antes de
  escrever o teste (lições da FEAT-010: botões de popup por `click_accesskit()`).
- Pôr o cursor programaticamente: `TextEdit::load_state` + `cursor.set_char_range` + `store`, e
  `ctx.memory_mut(|m| m.request_focus(editor_id))`; o `QueryTab.cursor` é copiado do estado a cada frame,
  então os dois precisam andar juntos no mesmo frame.

## Log

### 23/10/2026 — Revisão do `catalog::generate` (`process:reviewer`): aplicada

- [x] **Média.** O aviso `-- sem chave primária` na primeira linha ficava fora do statement (o `split` trata
  comentário como espaço): com o cursor nele, o Run Current rodaria o statement anterior do buffer. O aviso
  passou para dentro (linha acima do `WHERE`/`SET`), e a crate `sql` ganhou um teste que passa cada modelo por
  `split`/`statement_at` com o cursor no começo do texto.
- [x] **Baixa.** `example_value` por prefixo pegava `daterange`/`datemultirange` (`now()`) e `jsonpath` (`'{}'`),
  inválidos: igualdade nos tipos sem typmod; prefixo só em `character`, `varchar`, `numeric`, `decimal`,
  `time`. Teste.
- [x] **Baixa.** Brief com a assinatura real de `template_sql` e `Create` fora do `Template`.

### 23/10/2026 — UI e revisão dela (`process:reviewer`): aplicada

- `SqlItem`/`PendingSql`/`Workspace.pending_sql`; `generate_sql` monta na hora com colunas (ou estrutura
  cacheada na aba Data) ou pede ao engine; `complete_pending_sql` no `ColumnsLoaded`/`StructureLoaded` da
  tabela certa; `StructureFailed`/`CatalogFailed` descartam. `insert_sql`: `query_target` (aba SQL ativa →
  última) ou `open_query_tab`; fim do buffer, linha em branco, `QueryTab.pending_cursor` que o editor aplica
  ao estado do `TextEdit` com `request_focus`. `Tab::Query` foi para `Box` (a enum passou do limite do clippy).
- kittest: INSERT com colunas sob demanda numa aba nova (cursor 0, foco no editor, estado do widget), DELETE
  no fim da mesma aba com o `statement_at` apontando para ele, CREATE pela estrutura, view só com os itens de
  view e `DROP VIEW`, falha da estrutura descarta com aviso na tela, nada executado (`FakeDriver.executes`).
- [x] **Média (reviewer).** O aviso da falha ia para `App.notice`, que só a tela de conexões desenha: no
  workspace o CREATE sumia em silêncio. Agora vai para o `notice` da aba SQL de destino (toolbar) ou, sem
  aba, para a faixa da conexão; o teste confere o texto na tela.
- [x] **Baixa (reviewer).** `insert_sql` repetia `open_query_tab`; docs desatualizados (campos do
  `PendingSql`, regra do `example_value`, marcador do índice, aceite com "aspas só onde precisa").
- Lições: `click_secondary()` do kittest abre o `context_menu` e os itens respondem a `click_accesskit()`;
  Esc entre um menu e outro (o editor com foco engole o clique seguinte); `queries` do driver falso conta
  catálogo também, por isso o contador `executes` à parte; `App.notice` não aparece no workspace.

### 23/10/2026 — `catalog::generate`

- `Template` (7 modelos, `label`, `applies_to` por tipo de objeto, `needs_columns`), `template_sql`,
  `example_value`. Formato de várias linhas como o Beautify deixa (`SELECT` com uma coluna por linha, `SET`
  alinhado). Testes: cada modelo, view/materializada/estrangeira no `DROP`, identidade e gerada, `DEFAULT`,
  chave composta, só chave, sem chave, sem colunas, aspas dobradas.

### 23/10/2026 — Criação

- Brief, aceite, notas e índice a partir da conversa com o Jean (menu de contexto; ordem: FEAT-013 antes
  da FEAT-012 autocomplete).
