# @file: docs/features/FEAT-004-catalogo-arvore-e-seletor/10-prd/acceptance.md

# FEAT-004 — Critérios de aceite

## `driver` e `catalog`

- [x] `query_text("select 1 as a, null as b")` devolve colunas `a`, `b` e a linha `["1", None]` (teste com
      servidor); erro de SQL vira `DriverError::Db` com a mensagem do servidor, sem a senha.
- [x] `catalog::schemas` lista os schemas sem `pg_*` nem `information_schema` (sem `LIKE`: independe de
      `standard_conforming_strings`); `tables` separa tabela, view e
      view materializada; `columns` traz tipo legível (`format_type`), nulabilidade, default, posição e PK;
      `functions` traz nome e assinatura (teste com servidor, num schema criado e apagado pelo teste).
- [x] Nenhum carregador depende de `egui`, `tokio` ou do engine (`cargo xtask layers`; `tokio` só como dev-dep).

## `engine`

- [x] `select_database` abre a conexão do banco e emite `DatabaseSelected { database, schemas, current_schema }`
      (`public` se existir, senão o primeiro); banco inválido emite `CatalogFailed` e a sessão continua aberta.
- [x] `load_schema` e `load_columns` emitem os eventos com os dados; segunda chamada sai do cache sem query;
      `refresh` invalida tabelas, funções e colunas (schemas novos ao reselecionar o banco; teste com driver falso
      contando as queries).
- [x] A senha guardada na sessão nunca aparece em evento, `Debug` ou erro (teste); é descartada no `disconnect`.

## `ui-egui`

- [x] ⌘K abre a paleta com a lista de bancos e busca; Enter/clique seleciona; Esc fecha; a barra superior
      mostra `: banco` (teste de interação; Enter e Esc conferidos a olho).
- [x] Árvore com **Functions** e **Tables**; views com ícone próprio; tabela expande até as colunas `nome
      tipo`; PK com ícone de chave; seletor de schema no rodapé troca a árvore; busca filtra por nome.
- [x] Teste `egui_kittest`: com um engine falso, abrir ⌘K, escolher o banco, ver os dois grupos, expandir uma
      tabela e encontrar a coluna com a chave; e trocar o schema.
- [x] Erro de carga aparece na árvore e um botão "tentar de novo" refaz a query (teste de interação).

## Portão e CI

- [x] `scripts/quality-gate.sh --full` verde; cargo-deny verde com `egui_kittest`. [ ] Workflow verde (push).

## Do Jean

- [ ] Conectar em `pg-localhost`, ⌘K, escolher um banco, ver as tabelas e expandir uma até as colunas.
- [ ] Trocar o schema no rodapé e ver a árvore mudar.
