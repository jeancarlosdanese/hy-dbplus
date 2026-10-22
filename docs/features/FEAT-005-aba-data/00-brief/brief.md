# @file: docs/features/FEAT-005-aba-data/00-brief/brief.md

# FEAT-005 — Aba Data: grid, paginação, ordenação e painel lateral (M3)

## Problema

A árvore mostra tabelas e colunas, mas nada dos dados. O uso principal do produto (BRIEF §1) é navegar dados:
abrir uma tabela e ver as linhas num grid rápido, paginado e ordenável, com os tamanhos e o comentário ao lado
(tela 3.4). Tabelas grandes não cabem na memória nem na tela: o grid precisa ser virtualizado e a leitura
paginada no servidor.

## Objetivo

Entregar o M3 do [BRIEF §7](../../../BRIEF.md): abrir uma tabela e ver as linhas, os tamanhos e o comentário.
Tela 3.4 sem edição: `+ Row`, Filters e Columns ficam desenhados e desativados até o M7.

## Escopo (Must)

1. **`catalog`**: `TableStats` (`total_size`, `data_size`, `index_size` já formatados por `pg_size_pretty`,
   `row_count` exato, `comment`) por `pg_total_relation_size`, `pg_table_size`, `pg_indexes_size` e
   `count(*)`; construtores puros de SQL `page_sql(schema, table, order, limit, offset)` e `count_sql`, com
   identificadores entre aspas duplas (dobradas), testados sem servidor.
2. **`engine`**: `load_stats(id, banco, schema, tabela)` emite `StatsLoaded` (cache por tabela, invalidado no
   `refresh`); `load_page(id, banco, schema, tabela, Page { order, limit, offset })` emite `PageLoaded { rows,
   total, page }` sem cache; `CatalogFailed` para erro. Tudo pela conexão do banco, conferindo a geração.
3. **`ui-egui`**: na árvore, clicar no nome da tabela abre a aba Data (a seta continua expandindo as
   colunas). Central: grid virtualizado com `egui_extras::TableBuilder` (cabeçalho por coluna com o tipo em
   cinza, número da linha, valores monoespaçados, `NULL` em itálico, colunas redimensionáveis), clique no
   cabeçalho ordena (asc, desc, sem ordem), linha selecionada em destaque. Rodapé: abas **Data / Structure /
   Index** (as duas últimas desativadas até o M4), `+ Row`, contador `1-24 of 24 rows`, **Filters** e
   **Columns** desativados, paginação `‹ ›` e tamanho da página (100, 500, 1000). Painel direito: tamanhos e
   comentário; com uma linha selecionada, os campos dela (`nome: valor`). Estados "carregando" e erro no grid.
4. **Testes**: `catalog` (SQL puro e `stats` contra o servidor local no schema de teste); engine com driver
   falso (`StatsLoaded`, `PageLoaded`, cache de stats, página com ordem e offset no SQL); UI com
   `egui_kittest` (abrir tabela pela árvore, ver cabeçalho e células, ordenar, avançar página, selecionar linha
   e ver os campos no painel).

## Won't

- Edição de células, `+ Row`, commit/rollback (fora do MVP). Filtros e seleção de colunas: M7.
- Navegação por FK (seta na célula): M7 (o `catalog` ainda não lê FKs: M4).
- Keyset pagination: decisão 3 do BRIEF, `LIMIT/OFFSET` no MVP.
- Contagem estimada para tabelas enormes: `count(*)` exato; se doer, `reltuples` entra depois com aviso.
- Tipos nativos no grid (número à direita, data formatada): texto do `query_text` por enquanto.

## Restrições

- Dependência nova: `egui_extras` 0.36 (MIT OR Apache-2.0), sem features (`default-features = false`), só na
  `ui-egui`. Já prevista no BRIEF §6.
- Identificadores no SQL sempre entre aspas duplas com aspas dobradas; nunca concatenar nome cru. Sem
  parâmetros no protocolo simples, `limit` e `offset` são números formatados pelo Rust.
- `crates/engine/src/session.rs`, `Cargo.toml` e `Cargo.lock` são sensíveis.
- Commits pequenos, nesta ordem: docs; `catalog` (stats e SQL); `engine`; `ui-egui`.
