# @file: docs/features/FEAT-009-structure-e-index/00-brief/brief.md

# FEAT-009 — Abas Structure e Index (somente leitura) e DDL

## Problema

A aba de tabela só mostra dados. Para entender uma tabela o usuário precisa das colunas com tipo,
nulabilidade, default e comentário, das restrições (chave primária, únicas, checks, chaves estrangeiras),
dos índices e do DDL, como nas abas **Structure** e **Index** do TablePlus (BRIEF §3.5 e §3.6, marco M4). O
catálogo hoje lê só colunas (com a chave primária) para a árvore e a edição.

## Objetivo

Dentro de cada aba de tabela (FEAT-008), as sub-abas **Data / Structure / Index** do rodapé passam a
funcionar. Structure lista as colunas e as restrições e tem o botão **DDL**, que abre o `CREATE TABLE`
completo (ou o `CREATE VIEW`) com Copiar. Index lista os índices como o TablePlus. Tudo somente leitura;
editar nome, tipo ou chave fica para depois.

## Escopo (Must)

1. **`catalog::structure`** (camada 1): modelo e carregadores.
   - `Column` ganha `comment: Option<String>` (`col_description`) e `identity: Option<String>`
     (`attidentity`: `always`/`by default`) e `generated: Option<String>` (`attgenerated`: expressão).
   - `Constraint { name, kind: PrimaryKey | Unique | Check | ForeignKey | Exclusion, columns: Vec<String>,
     definition: String (pg_get_constraintdef), references: Option<Reference { schema, table, columns,
     on_update, on_delete }> }` de `pg_constraint`.
   - `Index { name, algorithm (pg_am.amname), is_unique, is_primary, columns: Vec<String> (expressões como o
     servidor escreve, via pg_get_indexdef por posição), condition: Option<String> (parcial),
     include: Vec<String>, storage_parameters: Vec<String> (reloptions), comment: Option<String>,
     definition: String (pg_get_indexdef) }` de `pg_index`.
   - `TableStructure { table: Table, columns, constraints, indexes, view_definition: Option<String>
     (pg_get_viewdef para views) }`; `structure(conn, schema, table)` com quatro queries (colunas, restrições,
     índices, definição da view).
2. **`catalog::ddl`** (puro, testado): `table_ddl(&TableStructure) -> String` monta `CREATE TABLE "s"."t" (
   colunas…, CONSTRAINT…)`, depois `CREATE [UNIQUE] INDEX` (a definição do servidor), `COMMENT ON TABLE` e
   `COMMENT ON COLUMN`; para view, `CREATE VIEW … AS <definição>`. Identificadores por `quote_ident`, textos
   por `literal`. Chave primária e índices que a implementam não se repetem (o índice `indisprimary` fica de
   fora do bloco de índices; os de restrição `UNIQUE` também, porque a restrição já os cria).
3. **`engine`**: `Sessions::load_structure(id, database, schema, table)` com cache por tabela em
   `Database.structures`; evento `StructureLoaded { connection, database, schema, table, structure }`;
   falha em `DataFailed`. O ↻ da árvore (`refresh`) esvazia o cache. `ddl` é reexportado para a UI e a CLI.
4. **`ui-egui`**: `DataTab` ganha `view: TabView { Data, Structure, Index }` e `structure: Option<Arc<TableStructure>>`;
   os botões Structure e Index do rodapé passam a selecionar a sub-aba (Data continua o padrão). Ao entrar em
   Structure ou Index pela primeira vez, pede `load_structure`.
   - **Structure**: grid `# | coluna | tipo | nulo | default | comentário` (🔑 na chave primária; identidade e
     gerada aparecem na coluna default), abaixo as restrições (`nome | tipo | colunas | definição`, FK com
     `→ schema.tabela(colunas)`), e o botão **DDL** que abre uma caixa (`Modal`) com o texto monoespaçado,
     rolável, e **Copiar**. Em view, Structure mostra as colunas e a definição.
   - **Index**: grid `nome | algoritmo | único | colunas | condição | include | parâmetros | comentário`.
   - Painel direito e seleção de linhas continuam da sub-aba Data; em Structure e Index o painel direito
     mostra "Nenhuma linha selecionada".
5. **Testes**: catálogo com servidor (tabela com PK composta, unique, check, FK com ON DELETE CASCADE, índice
   parcial com INCLUDE e reloptions, comentários de tabela e coluna, coluna identity; view): o
   `TableStructure` bate; o DDL gerado roda num schema novo e a estrutura lida de lá é igual (menos nomes de
   schema). `ddl` puro com fixtures. Engine com driver falso (cache, evento, refresh esvazia). UI com
   `egui_kittest`: Structure lista colunas e a FK, DDL abre e copia, Index lista o índice com include.

## Won't

- Editar nome, tipo, nulabilidade, chave ou índice; `+ Column`; Triggers (o botão fica desabilitado).
- Tabelas particionadas e herança no DDL (as partições já aparecem nos tamanhos; o DDL da pai sai sem
  `PARTITION BY`, anotado no texto).
- Sequências e tipos próprios (enum, domínio) no DDL: só referenciados pelo nome.
- Navegar pela FK no grid de dados (seta do BRIEF §3.4): M7, com Filters.

## Restrições

- Nenhum SQL com nome interpolado sem `literal`/`quote_ident`.
- `crates/engine/src/session.rs` é sensível: reviewer antes do commit do engine. Catálogo e UI passam pelo
  reviewer também (diff grande).
- Commits: `docs`; `feat(catalog)` (structure + ddl); `feat(engine)`; `feat(ui)`.
