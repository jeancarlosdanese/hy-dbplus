# @file: docs/features/FEAT-009-structure-e-index/50-dev/impl-notes.md

# Notas de Implementação — FEAT-009 structure-e-index

## Estado atual

_Atualizado em 22/10/2026._

- **Feito:** documentação; `catalog::structure` e `catalog::ddl` (modelo, carregador em quatro queries,
  gerador de DDL; testes puros e com servidor, inclusive o round-trip do DDL).
- **Feito também:** `engine::load_structure` com cache por tabela, `StructureLoaded`, `refresh` esvazia
  (teste com driver falso).
- **Feito também:** `ui-egui`: sub-abas Data / Structure / Index no rodapé, `structure.rs` (colunas,
  restrições, definição de view, caixa do DDL), `index.rs`; teste kittest.
- **Em curso:** revisão e commit da UI.
- **Falta:** verificação do Jean.
- **Depende de:** FEAT-008 (aba de tabela).
- **Para retomar:** esta pasta, BRIEF §3.5 e §3.6, e o `CLAUDE.md`.

## Decisões

- **[2026-10-22] DDL montado no catálogo, não pedido ao servidor.** O PostgreSQL não tem `pg_get_tabledef`;
  o `CREATE TABLE` sai do `TableStructure` (puro, testável), com as definições que o servidor escreve
  (`pg_get_constraintdef`, `pg_get_indexdef`, `pg_get_expr`, `format_type`) para não reinventar sintaxe. O
  teste com servidor roda o DDL gerado num schema novo e compara a estrutura: é a prova de que o DDL é válido
  e completo.
- **[2026-10-22] Índices de PK e UNIQUE não se repetem.** A restrição já cria o índice; o bloco de índices
  lista só os que não implementam restrição (`pg_index.indisprimary` e `pg_constraint.conindid`).
- **[2026-10-22] Sub-aba por aba, Data como padrão.** `TabView` vive no `DataTab`; trocar de sub-aba não
  recarrega dados nem mexe na pendência; Structure/Index pedem a estrutura uma vez e usam o cache do engine.
- **[2026-10-22] Somente leitura.** Editar nome, tipo, chave e índice fica fora (brief Won't).

## Plano

1. `catalog`: modelo (`Column` com comentário/identidade/gerada; `Constraint`, `Index`, `TableStructure`),
   `structure()` e `ddl::table_ddl` → testes puros e com servidor. Commit `feat(catalog)`.
2. `engine`: `load_structure`, cache em `Database.structures`, `StructureLoaded`, `refresh` esvazia → teste
   com driver falso. Reviewer. Commit `feat(engine)`.
3. `ui-egui`: `TabView`, rodapé, Structure, Index, caixa do DDL → kittest. Reviewer. Commit `feat(ui)`.
4. Docs: aceite marcado, notas, índice; BRIEF §3.5/§3.6 com o que ficou fora.

## Log

### 22/10/2026 — Criação

- [x] Brief, aceite, notas e índice a partir do BRIEF §3.5/§3.6 e do marco M4.

### 22/10/2026 — `catalog::structure` e `catalog::ddl`

- [x] `Column` ganhou `comment`, `identity` (`ALWAYS`/`BY DEFAULT`) e `generated` (a expressão; coluna gerada
  guarda a expressão em `pg_attrdef`, então ela vai em `generated` e `default` fica `None`); `Column` derive
  `Default` para os testes. `Constraint`/`ConstraintKind`/`Reference`, `Index` (com `is_constraint`),
  `TableStructure`.
- [x] `structure()`: cabeçalho (relkind, comentário, `pg_get_viewdef`), colunas, restrições (`pg_constraint`
  com colunas e referência por `unnest … WITH ORDINALITY`), índices (`pg_index` com colunas por
  `pg_get_indexdef(oid, n, true)`, include por `ord > indnkeyatts`, `reloptions`, comentário e se implementa
  restrição). Listas vêm separadas por `\x1f` (nome de coluna pode ter vírgula).
- [x] `table_ddl`: `CREATE TABLE` com colunas (NOT NULL, GENERATED … STORED, IDENTITY ou DEFAULT) e
  `CONSTRAINT` com a definição do servidor; índices que não implementam restrição pela `pg_get_indexdef`;
  `COMMENT ON` de tabela, coluna e índice; view e view materializada pela definição.
- [x] Teste com servidor: PK composta, unique nomeada, check, FK `ON DELETE CASCADE`, índice parcial com
  INCLUDE e fillfactor, comentários, identidade, gerada, view; o DDL gerado roda num schema novo (nome
  trocado no texto) e a estrutura lida de lá é igual (Debug comparado, menos o schema). `pg_get_expr(...,
  true)` escreve a condição sem os parênteses externos (`obs IS NOT NULL`).
- Sem `serial` no teste de propósito: o default `nextval('a.seq')` apontaria para o schema antigo (brief
  Won't: sequências só pelo nome).

### 22/10/2026 — Revisão do catálogo (`process:reviewer`): um médio bloqueante, aplicado

- [x] **Médio:** o teste com servidor deixava o schema para trás quando uma asserção falhava (o revisor achou
  um órfão `hy_dbplus_test_59947_…` no banco local). Agora carrega tudo, apaga os dois schemas e só então
  confere, como os outros testes da crate.
- [x] Tabela inexistente: `structure` devolve erro em vez de estrutura vazia com DDL inválido (o engine o trata
  como `DataFailed`); testado.
- [x] Particionada (`relkind = 'p'`, `TableStructure.partitioned`) e estrangeira saem com aviso em comentário no
  topo do DDL; coluna gerada virtual (PostgreSQL 18) reconhecida e escrita `VIRTUAL`; testes puros.
- [x] Separador das listas num lugar só (`SEP`/`SEP_SQL`); doc do `lib.rs`; brief com `ALWAYS`/`BY DEFAULT` e
  o Won't de collation e `WITH NO DATA`.
- Para o Jean: o órfão `hy_dbplus_test_59947_728128000_2` no banco local pode ser apagado com
  `DROP SCHEMA hy_dbplus_test_59947_728128000_2 CASCADE` (eu não apago nada no seu banco sem pedir).

### 22/10/2026 — `engine::load_structure`

- [x] `Database.structures: HashMap<(schema, tabela), TableStructure>`; `load_structure` no molde de
  `load_stats` (cache na hora, senão tarefa com `catalog::structure` e `commit_if_current`); `StructureLoaded`
  com a estrutura; erro em `DataFailed`; `refresh` esvazia junto com objetos, colunas e stats. `table_ddl` e
  os tipos novos reexportados pelo engine.
- [x] Teste: quatro queries na primeira vez, zero na segunda (cache), quatro de novo depois do ↻; tabela que o
  driver falso recusa vira `DataFailed`. O driver falso responde às três queries novas (cabeçalho, restrições,
  índices): o cabeçalho antes dos ramos de `pg_class`/`pg_namespace`, restrições e índices antes do de colunas
  (as duas também citam `pg_attribute`).
- [x] UI: só o braço `StructureLoaded => {}` para compilar.

### 22/10/2026 — Revisão do engine (`process:reviewer`): nada bloqueante

- [x] Doc do `refresh` cita as estruturas; nota do driver falso com a justificativa certa.
- [x] As colunas lidas pela estrutura abastecem `db.columns` (`entry().or_insert_with`), como `load_page` faz:
  sem duas cópias e sem query a mais ao abrir Structure depois de Data; teste cobre.

### 22/10/2026 — `ui-egui`: sub-abas, Structure, Index e DDL

- [x] `DataTab`: `view: TabView { Data, Structure, Index }`, `structure: Option<Arc<TableStructure>>`,
  `structure_requested`, `ddl_open`. `App::set_view` troca a sub-aba e pede a estrutura uma vez;
  `StructureLoaded` vai à aba da tabela (`data_tab_mut`); o ↻ da árvore zera `structure` das abas do banco
  (o engine esvaziou o cache dele).
- [x] `data.rs`: os botões Data / Structure / Index do rodapé são `selectable_label` por aba; fora de Data o
  rodapé mostra `+ Column` e Triggers desabilitados e o botão DDL (Structure), e o centro delega a
  `structure::show` / `index::show`; paginação, seleção e edição ficam na sub-aba Data.
- [x] `structure.rs`: grid de colunas (`#`, coluna com 🔑, tipo, nulo, default com identidade/gerada,
  comentário) e grid de restrições (nome, tipo, colunas, definição; FK como `→ schema.tabela(colunas) on
  update … / on delete …`); view mostra a definição. `ddl_dialog`: `Modal` com o `table_ddl`, Copiar e Fechar.
- [x] `index.rs`: grid nome, algoritmo, único, colunas, condição, include, parâmetros, comentário.
- [x] Driver falso da UI responde às queries da estrutura (tabela com FK e índice parcial com include; a view
  com definição). Teste `structure_index_e_ddl_de_uma_tabela`: Structure com quatro queries, 🔑, tipo, FK
  apontando; DDL com `CREATE TABLE` e o índice, Copiar e Fechar; Index do cache com include, condição e
  parâmetros; Data de volta com o grid.
