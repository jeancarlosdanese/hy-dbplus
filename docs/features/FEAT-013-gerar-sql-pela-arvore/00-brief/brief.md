# @file: docs/features/FEAT-013-gerar-sql-pela-arvore/00-brief/brief.md

# FEAT-013 — Gerar SQL pelo botão direito na árvore

## Problema

Para escrever um `INSERT`, um `UPDATE` ou um `DELETE` numa tabela, o usuário digita o nome de cada
coluna e da chave primária de cabeça ou copia da aba Structure. Ferramentas como o TablePlus e o
DBeaver geram o esqueleto do statement a partir do objeto na árvore, e o usuário só preenche os
valores. Pedido do Jean (23/10/2026): "botão direito na tabela para sugestão de queries: insert,
update, delete, drop cascade, etc.".

## Objetivo

Botão direito numa tabela ou view da árvore abre um menu com modelos de SQL prontos, montados com as
colunas e a chave primária do catálogo. O texto vai para a aba SQL ativa (no fim do buffer, com uma
linha em branco antes) ou para uma aba nova se o banco não tem nenhuma, com o cursor em cima do
statement gerado: **Run Current ⌘↩** roda exatamente ele. Nada é executado pelo menu: `DROP`,
`TRUNCATE` e `DELETE` só aparecem no editor e o usuário decide.

## Escopo (Must)

1. **`catalog`** (camada 1, puro, testado): `generate.rs` com `Template` (sete modelos; o `CREATE` é item
   da UI, pelo `table_ddl`) e `template_sql(template, &Table, &[Column]) -> String`. Modelos para tabela
   (e tabela estrangeira):
   - **SELECT**: `SELECT col1, col2, … FROM schema.tabela LIMIT 100;` (colunas na ordem da tabela, uma
     linha por bloco de colunas como o Beautify deixaria).
   - **SELECT count**: `SELECT count(*) FROM schema.tabela;`
   - **INSERT**: todas as colunas menos identidade `ALWAYS` e geradas; valor de exemplo por tipo (`''`
     para texto, `0` para número, `false` para booleano, `now()` para data e hora, `gen_random_uuid()`
     para uuid, `'{}'` para json e array, `'\x'` para bytea, `NULL` para o resto); coluna com `default`
     ou identidade `BY DEFAULT` recebe `DEFAULT`; sem coluna gravável, `DEFAULT VALUES`.
   - **UPDATE**: `SET` com as colunas graváveis que não são chave (só chave: a própria chave), mesmo
     valor de exemplo, `WHERE` pela chave primária (`id = 0`, `AND` na composta); sem chave primária,
     `WHERE` pela primeira coluna e o comentário `-- sem chave primária: confira o WHERE`; sem coluna
     nenhuma, `WHERE false` e aviso. Todo aviso fica dentro do statement (linha acima do `WHERE` ou do
     `SET`), nunca na primeira linha: fora do statement, o Run Current com o cursor nele rodaria outro.
   - **DELETE**: `WHERE` pela chave primária, mesma regra sem chave.
   - **TRUNCATE**: `TRUNCATE TABLE schema.tabela;`
   - **DROP CASCADE**: `DROP TABLE schema.tabela CASCADE;`
   - **CREATE TABLE**: o DDL da aba Structure (`table_ddl`, FEAT-009), que precisa da estrutura.
   View e view materializada: SELECT, SELECT count, CREATE (DDL com `pg_get_viewdef`) e `DROP VIEW …
   CASCADE` / `DROP MATERIALIZED VIEW … CASCADE`; tabela estrangeira, `DROP FOREIGN TABLE`. Identificadores
   sempre entre aspas por `quote_ident`, como o resto do catálogo e o DDL. Todo modelo termina em `;`.
2. **`ui-egui`**:
   - A linha da tabela ou view na árvore (`tree.rs`) ganha `context_menu` com os itens acima, por tipo
     de objeto, na ordem: SELECT, SELECT count, INSERT, UPDATE, DELETE, separador, TRUNCATE, DROP …
     CASCADE, separador, CREATE.
   - `App::generate_sql(key, schema, table, item: SqlItem)` (`Template(..)` ou `Create`): se as colunas da tabela já estão no
     `Workspace.columns`, monta e insere; senão guarda o pedido em `Workspace.pending_sql` e chama
     `load_columns`; o `ColumnsLoaded` da tabela completa o pedido. CREATE pede `load_structure` e o
     `StructureLoaded` completa (o engine tem cache: a segunda vez é imediata). `CatalogFailed` ou
     `StructureFailed` da tabela descartam o pedido e avisam na toolbar da aba SQL de destino (ou na faixa da
     conexão quando o banco não tem aba SQL).
   - Destino: a aba ativa se é SQL; senão a última aba SQL do banco; senão `open_query_tab`. O texto
     entra no fim do buffer, separado por uma linha em branco quando o buffer não está vazio, com o
     cursor no começo do statement novo; a aba fica ativa e o editor recebe o foco. O autosave da
     FEAT-011 grava o buffer como qualquer edição.
3. **Testes**: `catalog` puro (um teste por modelo, com e sem chave primária, identidade, default,
   aspas, view); UI com `egui_kittest`: botão direito numa tabela com colunas carregadas, INSERT vai
   para uma aba nova com o cursor no statement; tabela sem colunas carregadas pede ao engine e insere
   quando `ColumnsLoaded` chega; segundo modelo entra no fim da mesma aba com linha em branco; CREATE
   espera o `StructureLoaded`; nada é executado (o driver falso conta os `execute` à parte, porque o menu
   dispara queries de catálogo).

## Won't

- Executar qualquer modelo pelo menu; `SELECT … WHERE pk = valor` (o SELECT já lista as colunas e o
  Data cobre a leitura); `COPY`; `ALTER TABLE`; `GRANT`; gerar a partir de uma linha do grid
  (`INSERT` com os valores da linha, "Copy as" do TablePlus) fica para depois do M7; `REFRESH
  MATERIALIZED VIEW`; funções (`SELECT fn(args)`).
- Menu de contexto nos schemas, nas funções e nas colunas.

## Restrições

- Sem dependência nova. `crates/catalog/src/generate.rs` não casa com os padrões sensíveis; o
  reviewer roda mesmo assim (monta SQL destrutivo que o usuário vai executar: `DROP CASCADE` tem que
  apontar para a tabela certa, com aspas certas).
- Commits: `docs`; `feat(catalog)`; `feat(ui)`.
