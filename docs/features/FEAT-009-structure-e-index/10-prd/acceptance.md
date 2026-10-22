# @file: docs/features/FEAT-009-structure-e-index/10-prd/acceptance.md

# FEAT-009 — Critérios de aceite

## `catalog`

- [x] `structure` devolve colunas (tipo, nulo, default, identidade, gerada, comentário, posição, chave),
      restrições (PK, unique, check, FK com referência e ações) e índices (algoritmo, único, colunas,
      condição, include, parâmetros, comentário, definição) de uma tabela de teste; view traz a definição.
- [x] `table_ddl` gera um `CREATE TABLE` que, rodado num schema novo, produz a mesma estrutura (menos o nome do
      schema); índice da chave primária e das restrições únicas não se repetem; comentários saem em
      `COMMENT ON`.
- [x] Fixtures puras cobrem: identificador com aspas, default com `'`, check, FK composta, índice parcial com
      include, view.

## `engine`

- [x] `load_structure` emite `StructureLoaded` e responde do cache na segunda vez; `refresh` esvazia o cache;
      erro vira `StructureFailed`.

## `ui-egui`

- [x] Rodapé: Data, Structure e Index selecionam a sub-aba da aba de tabela; Data é o padrão; a sub-aba fica
      por aba.
- [x] Structure: colunas com 🔑 na chave, tipo, nulo, default (identidade/gerada), comentário; restrições com
      FK `→ schema.tabela(colunas)`; botão DDL abre a caixa com o texto e Copiar; view mostra a definição.
- [x] Index: índices com algoritmo, único, colunas, condição, include, parâmetros, comentário.
- [x] Falha ao carregar a estrutura mostra o erro com "Tentar de novo" nas sub-abas (sem "Carregando…"
      eterno); o painel direito em Structure/Index diz "Nenhuma linha selecionada"; ⌘S/⌘Z/⌫ só em Data.
- [x] Teste `egui_kittest` cobre Structure (coluna, FK, DDL com Copiar) e Index (include).

## Portão e CI

- [x] `scripts/quality-gate.sh --full` verde (22/10/2026, com `HY_DBPLUS_TEST_PG`).
- [ ] Workflow verde (push do Jean).

## Do Jean

- [ ] Abrir uma tabela com FK e índice, conferir Structure, Index e o DDL contra o TablePlus.
