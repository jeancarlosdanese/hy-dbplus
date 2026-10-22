# @file: docs/features/FEAT-005-aba-data/50-dev/impl-notes.md

# Notas de Implementação — FEAT-005 aba-data

## Estado atual

_Atualizado em 22/10/2026._

- **Feito:** documentação (`b6e4f88`); no scratchpad (cópia completa do workspace): `catalog::data`, `engine`
  (`load_stats`, `load_page`, `Page`, eventos) e `ui-egui` (aba Data, árvore com nome e seta separados).
- **Em curso:** `engine` na árvore do repositório (este commit); `catalog` commitado (`874431f`).
- **Falta:** trazer do scratchpad a `ui-egui`; verificação do Jean.
- **Depende de:** FEAT-004 concluída (M2).
- **Para retomar:** esta pasta, o `CLAUDE.md` da raiz e o BRIEF §3.4, §5 e decisão 3.

## Decisões

- **[2026-10-21]** Paginação `LIMIT/OFFSET` (BRIEF decisão 3); ordenação por `ORDER BY "coluna" ASC|DESC`
  montada pelo `catalog` com identificadores entre aspas duplas dobradas. Sem ordem explícita o PostgreSQL
  não garante a ordem das páginas; a UI avisa no contador quando não há ordenação? Não: segue o TablePlus,
  que também não avisa. Fica anotado.
- **[2026-10-21]** `count(*)` exato, uma vez por tabela aberta, nos `stats` (cache até o `refresh`); o
  `load_page` não conta de novo (revisor do catálogo: abrir uma tabela grande contava duas vezes). Em tabela
  enorme isso é lento; a troca por `reltuples` estimado fica para quando aparecer, com aviso "~" no contador.
- **[2026-10-22]** `ORDER BY` com desempate: `page_sql` recebe uma lista de colunas; o engine põe a coluna
  escolhida pelo usuário e, depois, a chave primária (do cache de colunas, se já carregado). Sem desempate,
  valores repetidos embaralham as páginas (revisor do catálogo).
- **[2026-10-21]** Grid com `egui_extras::TableBuilder` e `body.rows(altura, n, ...)`: só as linhas visíveis
  são desenhadas; a página inteira (até 1000 linhas de texto) fica em memória na UI.
- **[2026-10-21]** Clicar no nome da tabela abre a aba; a seta expande. Na árvore, `CollapsingState` com
  `show_header` permite o cabeçalho customizado (seta + rótulo selecionável), em vez do `CollapsingHeader`
  que faz do cabeçalho inteiro o toggle.
- **[2026-10-21]** `egui_extras` 0.36 (MIT OR Apache-2.0) com `default-features = false`: só a tabela; sem
  carregadores de imagem.

## Log

### 21/10/2026 — Criação

- [x] Leitura do BRIEF §3.4 e decisão 3; API do `egui_extras::TableBuilder` 0.36 no registro local; brief,
  aceite, notas e índice antes do código.

### 21/10/2026 — `catalog`, `engine` e `ui-egui` no scratchpad (a trazer: engine e ui-egui)

- [x] `catalog::data`: `TableStats`, `Direction`, `Order`, `quote_ident`/`qualified` (aspas duplas dobradas),
  `page_sql`, `count_sql`, `page`, `count`, `stats` (`pg_size_pretty` de `pg_total_relation_size`,
  `pg_table_size`, `pg_indexes_size`, `obj_description`, mais `count(*)`). Testes puros do SQL (ordem, limite,
  offset, nomes com aspas e `; drop`) e com servidor (3 linhas, ordem desc, página 2, NULL, comentário).
  O suporte de teste com servidor virou `postgres::test_support` (`connect`, `schema_name`), reusado.
- [x] (superado em 22/10, abaixo) `engine`: `Page { order, limit: 100, offset }`, eventos `StatsLoaded` e `PageLoaded { page, rows, total }`,
  `load_stats` (cache por tabela no `Database`, invalidado no `refresh`) e `load_page` (sem cache: página +
  `count(*)`), ambos por `commit_if_current`. Driver falso guarda o último SQL; teste de cache, refresh, ordem
  e offset no SQL, e `load_page` ignorado sem banco aberto (19 testes).
- [x] `ui-egui`: `DataTab` dentro do `Workspace` (schema, tabela, página, linhas, total, stats, linha
  selecionada, carregando, erro); `open_table` pede página e stats; `load_data_page` recarrega. `data.rs`:
  `TableBuilder` com `Sense::click`, coluna `#` (selectable label, alvo de seleção), uma coluna por campo com
  tipo em cinza (do cache de colunas, se houver), cabeçalho clicável (asc → desc → sem ordem), `NULL` em
  itálico, `body.rows` virtualizado; rodapé com Data/Structure/Index (as duas desativadas), `+ Row`, Filters e
  Columns desativados, contador `início-fim of total rows`, `‹ ›` e tamanho da página (100/500/1000); painel
  direito com tamanhos, contagem e comentário, ou os campos da linha selecionada. Na árvore, o cabeçalho da
  tabela é próprio: botão `▸`/`▾` (acessível) expande e o nome selecionável abre a aba (`CollapsingState`
  com `show_body_indented`). `egui_extras` 0.36 sem features. Teste `egui_kittest`: abrir pela árvore,
  contador, NULL, stats, ordenar duas vezes (dados trocam de ordem), página seguinte, selecionar linha e ver
  "Linha 101". Aprendido: no harness, botão do rodapé e selectable label do grid só respondem por
  `click_accesskit()`; o painel direito lê a seleção no frame seguinte.

### 22/10/2026 — `catalog` no repositório

- [x] `data.rs`, `postgres::test_support` e os re-exports trazidos do scratchpad. Revisor (seis baixos): `count`
  sem número vira erro, não zero; `stats` usa o `text()` do catálogo; re-exports `page_sql` e `count_sql` (o
  par de construtores puros), `quote_ident`/`qualified` ficam em `data::`; doc do `lib.rs` diz que a aba Data
  usa identificador entre aspas duplas; `page_sql` recebe `&[Order]` para o desempate pela chave primária
  (decisão acima); a contagem dupla sai no passo do engine (decisão acima). `schema_name()` dos testes ganhou
  um contador: dois testes no mesmo microssegundo colidiam no nome do schema (falha intermitente vista aqui).
  O engine e a UI do scratchpad precisam acompanhar a assinatura nova de `page`/`page_sql` e o `PageLoaded`
  sem `total`. Commit `874431f`.

### 22/10/2026 — `engine` no repositório

- [x] `session.rs` e `lib.rs` do scratchpad, já adaptados: `PageLoaded { page, rows }` sem total; `load_page`
  monta a ordem com a coluna pedida e, se as colunas da tabela estão em cache, as de chave primária como
  desempate (teste confere o SQL antes e depois de `load_columns`); `load_stats` com cache por tabela e
  `refresh` invalidando; sem `count(*)` por página. A UI do M2 ganhou um braço que ignora `StatsLoaded` e
  `PageLoaded` até o passo seguinte, como na FEAT-004.
- [x] Revisor (um médio, três baixos): o desempate dependia de as colunas estarem em cache, e expandir a tabela
  na árvore entre duas páginas trocava a ordem (linhas repetidas ou puladas). Agora `load_page` lê as colunas
  antes da primeira página quando não estão em cache (uma query a mais, uma vez por tabela), guarda no cache
  e monta a ordem com a PK desde a primeira página; a árvore recebe as colunas do cache sem query (teste).
  Falha de página ou de stats sai como `DataFailed { schema, table, error }`, para a aba Data saber que foi
  dela (antes era `CatalogFailed` sem tabela; teste). Docs de `refresh` e do módulo citam os stats. Pendência
  registrada: o esqueleto lock → cache → task → `commit_if_current` → falha se repete em cinco métodos; um
  `refactor(engine)` com helper fica para depois do M3, separado das feats.
