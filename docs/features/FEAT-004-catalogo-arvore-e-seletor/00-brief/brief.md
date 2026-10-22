# @file: docs/features/FEAT-004-catalogo-arvore-e-seletor/00-brief/brief.md

# FEAT-004 — Catálogo, árvore de objetos e seletor de banco e schema (M2)

## Problema

Conectado, o app mostra só os nomes dos bancos. Navegar dados exige escolher o banco (⌘K) e o schema, e ver a
árvore de tabelas, views e funções até as colunas com tipo e chave primária (telas 3.2 e 3.3 do BRIEF). No
PostgreSQL cada banco é uma conexão própria, e `pg_catalog` é a única fonte confiável de metadados.

## Objetivo

Entregar o M2 do [BRIEF §7](../../../BRIEF.md): abrir um banco local e ver tabelas, views, funções e colunas,
com seletor de banco (⌘K) e de schema.

## Escopo (Must)

1. **`driver`**: `Connection::query_text(sql)` com `simple_query` (tudo como texto, sem prepared statement):
   `TextRows { columns, rows: Vec<Vec<Option<String>>> }`. Base também do grid (M3) e do Run Current (M5).
2. **`catalog`** (camada 1): modelo neutro (`Schema`, `Table` com `TableKind` tabela/view/materializada,
   `Column` com tipo, nulabilidade, default, posição e `is_primary_key`, `Function` com assinatura) e os
   carregadores `schemas`, `tables`, `functions`, `columns` sobre `pg_catalog`, cada um uma query. Sem cache
   na crate: o cache é do engine.
3. **`engine`**: a sessão guarda a senha em memória (só para abrir a conexão de outro banco) e tem uma conexão
   por banco selecionado; `select_database(id, db)` abre a conexão, lê os schemas e emite
   `DatabaseSelected { schemas, current_schema }`; `load_schema(id, banco, schema)` emite `SchemaLoaded { tables,
   functions }`; `load_columns(id, banco, schema, table)` emite `ColumnsLoaded`. Cache por (conexão, banco, schema) com
   invalidação manual (`refresh`).
4. **`ui-egui`**: paleta ⌘K (busca e lista de bancos; Enter seleciona; Esc fecha); barra superior ganha
   `: banco`; painel esquerdo com a árvore em grupos **Functions** e **Tables** (views com ícone próprio),
   tabela expansível até as colunas `nome  tipo`, chave primária com ícone de chave; seletor de schema no
   rodapé do painel; busca na árvore (tela 3.2). Estados "carregando" e erro na própria árvore.
5. **Testes**: `catalog` contra o PostgreSQL local atrás de `HY_DBPLUS_TEST_PG` (cria e apaga um schema
   próprio `hy_dbplus_test_<id>` com tabela, view, PK, função); engine com driver falso; UI com `egui_kittest`
   (harness sem janela): abrir ⌘K, escolher banco, ver grupos e expandir uma tabela.

## Won't

- Aba Data, Structure e Index (M3, M4); clique em tabela só seleciona e expande.
- Índices, FKs, triggers e DDL no catálogo: entram com as abas que os mostram (M4, M10).
- Tipos nativos no `driver` (`Value` por OID): o texto do `simple_query` basta até o grid pedir mais.
- Busca global de objetos (M7).

## Restrições

- `crates/driver/src/`, sessão e credenciais do `engine`, `Cargo.toml` e `Cargo.lock` são sensíveis.
- Dependência nova: `egui_kittest` 0.36 (MIT OR Apache-2.0) como dev-dependency da `ui-egui`, sem as
  features `wgpu`/`snapshot`. O `catalog` não ganha dependência externa.
- Commits pequenos, nesta ordem: docs; `driver` (`query_text`) e `catalog`; `engine`; `ui-egui`.
