# @file: docs/features/FEAT-005-aba-data/10-prd/acceptance.md

# FEAT-005 — Critérios de aceite

## `catalog`

- [x] `page_sql("public", "pedido", &[total desc, id asc], 100, 200)` produz `SELECT * FROM "public"."pedido"
      ORDER BY "total" DESC, "id" ASC LIMIT 100 OFFSET 200`; nomes com aspas duplas saem dobradas (teste puro).
- [x] `stats` devolve tamanhos legíveis, `row_count` exato e o comentário (teste com servidor no schema de teste).

## `engine`

- [x] `load_stats` emite `StatsLoaded` e a segunda chamada sai do cache; `refresh` invalida (teste com driver
      falso contando queries).
- [x] `load_page` emite `PageLoaded { rows, page }` com o SQL contendo `ORDER BY` (coluna pedida e a PK como
      desempate quando as colunas estão em cache), `LIMIT` e `OFFSET` (driver falso guarda o último SQL); o
      total vem de `StatsLoaded`.

## `ui-egui`

- [x] Clicar no nome de uma tabela na árvore abre a aba Data no centro com cabeçalho e linhas; a seta ao lado
      continua expandindo as colunas sem abrir a aba.
- [x] Clique no cabeçalho ordena (asc → desc → sem ordem) e recarrega a página; `‹ ›` e o tamanho da página
      recarregam; o contador mostra `início-fim of total rows` (total dos stats; `…` até chegar).
- [x] Selecionar uma linha destaca e mostra os campos no painel direito; sem seleção, o painel mostra
      `total_size`, `data_size`, `index_size` e `comment`.
- [x] `NULL` aparece em itálico e distinto de texto vazio.
- [x] Teste `egui_kittest` cobre abrir, ordenar, paginar e selecionar.
- [x] Grid com 1000 linhas por página não trava a interface (virtualizado por `body.rows`: só as linhas visíveis
      são desenhadas; conferência de desempenho real é do Jean).

## Portão e CI

- [x] `scripts/quality-gate.sh --full` verde; cargo-deny verde com `egui_extras`. [ ] Workflow verde (push).

## Do Jean

- [ ] Abrir uma tabela com dados reais, ordenar por uma coluna, avançar página, selecionar uma linha e ver os
      campos e os tamanhos no painel.
