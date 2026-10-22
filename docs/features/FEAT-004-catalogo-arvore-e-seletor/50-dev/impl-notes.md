# @file: docs/features/FEAT-004-catalogo-arvore-e-seletor/50-dev/impl-notes.md

# Notas de Implementação — FEAT-004 catalogo-arvore-e-seletor

## Estado atual

_Atualizado em 21/10/2026._

- **Feito:** documentação (`a418d93`); `driver::query_text` e `catalog` (`f4ffd77`); `engine` com banco
  selecionado, cache do catálogo e eventos novos no repositório.
- **Em curso:** nada.
- **Falta:** trazer do scratchpad a `ui-egui` (um commit); verificação do Jean.
- **Depende de:** FEAT-003 concluída (M1).
- **Para retomar:** esta pasta, o `CLAUDE.md` da raiz e o BRIEF §3.2, §3.3 e §5.

## Decisões

- **[2026-10-21]** Catálogo como texto por `simple_query`: `pg_catalog` devolve nomes e `format_type`, que já
  são texto; sem conversão por OID no driver até o grid (M3) precisar de mais. Um `simple_query` por carga,
  sem prepared statement, o que também é o que o Run Current do M5 quer.
- **[2026-10-21]** Selecionar banco = nova conexão do driver com `database` trocado: no PostgreSQL não há
  `USE`. A sessão guarda o `Secret` em memória para isso (nunca em evento, `Debug` ou erro); a conexão do banco
  inicial fica aberta para a lista de bancos e a versão.
- **[2026-10-21]** Carga lazy em três níveis (schemas ao selecionar o banco; tabelas e funções ao escolher o
  schema; colunas ao expandir a tabela), com cache no engine por (conexão, banco, schema) e `refresh` manual,
  como o BRIEF §5 descreve para o `catalog`.
- **[2026-10-21]** `egui_kittest` 0.36 (MIT OR Apache-2.0) só como dev-dependency da `ui-egui`, sem `wgpu` nem
  `snapshot`: harness sem janela, testes de interação; screenshot fica para quando houver tema a conferir.

## Log

### 21/10/2026 — Criação

- [x] Leitura do BRIEF §3.2, §3.3, §5; API do `egui_kittest` 0.36 no registro local; brief, aceite, notas e
  índice antes do código.

### 21/10/2026 — `driver`, `catalog` e `engine` (scratchpad)

- [x] `TextRows` e `Connection::query_text` por `simple_query`: primeiro conjunto de linhas, colunas pelo
  `RowDescription`, NULL como `None`; teste com servidor (`select 1, null, 'x'; select 2`, erro de SQL como
  `Db`, statement sem linhas).
- [x] `catalog`: `model.rs` (`Schema`, `Table`/`TableKind` com view, materializada e estrangeira, `Column` com
  `format_type`, nulabilidade, default, posição e PK, `Function` com `signature()`), `postgres.rs` com um
  `literal()` (corrigido na rodada do revisor, abaixo) e quatro queries sobre `pg_catalog`. Teste com servidor cria o schema
  `hy_dbplus_test_<pid>_<nanos>` com tabela (PK, varchar(80), numeric(10,2) default 0), view e função,
  confere tudo e apaga com `CASCADE`.
- [x] `engine`: `Slot` guarda `config`, `secret` (descartada no `disconnect`; sem `zeroize`, o valor deixa de ser
  alcançável, não é sobrescrito) e `databases: HashMap<nome,
  Database { conn, schemas, current_schema, objects, columns }>`; `select_database` abre a conexão do banco e
  lê os schemas (`public` ou o primeiro), `load_schema` (tabelas + funções), `load_columns`, `refresh`; cache
  responde na hora sem query; `CatalogFailed` deixa a sessão do servidor viva; tudo conferindo a geração.
  Driver falso conta queries e responde pelo nome da tabela de `pg_catalog` no SQL (da mais específica para a
  mais geral: tabelas e funções também citam `pg_namespace`).
- [x] `ui-egui` (scratchpad): `App<D: Driver>` genérico (`PgApp` para o binário) para o teste de interação
  rodar com um engine falso; `Workspace` por conexão (banco, schemas, schema atual, objetos e colunas em cache
  local, tabelas expandidas, busca, carregando, erro); `palette.rs` (⌘K via `consume_shortcut`, janela
  ancorada no topo com busca, setas e Enter, Esc fecha, clique seleciona); `tree.rs` (seletor de schema e ↻
  no rodapé do painel por `Panel::bottom`, busca, `CollapsingHeader` Functions e Tables, ícones ▦ ◫ ⇄ ƒ, colunas
  `🔑 nome  tipo`, `load_columns` uma vez por tabela expandida, erro com "Tentar de novo"); `workspace.rs` com
  `: banco` no título e botão `Banco ⌘K`. Eventos novos tratados em `drain_events`.
- [x] Teste `egui_kittest` (`Harness::new_ui_state`, sem janela): conectar com driver falso, ⌘K, clicar em
  `loja`, ver `Tables (2)`, `Functions (1)` e `◫ v_pedido`, expandir `▦ pedido`, ver `🔑 id`, `integer` e
  `character varying(80)`, contagem de queries (3, 4, 6), trocar o schema pelo ComboBox. Aprendido: o clique
  por ponteiro do kittest não ativa item dentro do popup do ComboBox; `click_accesskit()` (ação do AccessKit)
  ativa. Dois nós com o mesmo texto fazem `get_by_label_contains` falhar: usar o rótulo exato.
- [x] `egui_kittest` 0.36 como dev-dependency da `ui-egui` com `default-features = false`.
- [x] Segundo teste `egui_kittest`: schema que o servidor recusa mostra o erro na árvore sem derrubar a sessão
  nem o banco, e "Tentar de novo" consulta o servidor outra vez (driver falso falha em `E'quebrado'`). A UI
  do scratchpad compila e passa contra o driver e o engine já commitados no repositório.

### 21/10/2026 — `driver` e `catalog` no repositório

- [x] Revisor (dois médios, cinco baixos, aplicados antes do commit): o escape só por aspas dobradas falhava
  com `standard_conforming_strings = off` (um nome de schema com `\'` fecharia o literal e o resto rodaria
  como segundo statement, com as credenciais do usuário); `literal()` agora é `E'...'` com `\` e `'`
  dobrados, válido nos dois modos, e a lista de schemas usa `left(nspname, 3) <> 'pg_'` em vez de `LIKE`
  com escape. `text_rows` devolvia o segundo statement quando o primeiro `SELECT` tinha zero linhas; agora
  para no primeiro `CommandComplete` depois do primeiro `RowDescription` (teste: `select 1 where false;
  select 2` devolve a coluna `a` e nenhuma linha; `set ...; select 3` devolve o select). O teste do
  catálogo lê tudo, apaga o schema e só então confere, para uma asserção falha não deixar schema órfão; o
  setup ganhou view materializada e procedure. `thiserror` saiu do `catalog` (não usava). BRIEF §5 diz que o
  cache do catálogo mora no engine. O parse da URL de teste está duplicado no driver e no catálogo, anotado
  nos dois.

### 21/10/2026 — `engine` no repositório

- [x] `session.rs` e `lib.rs` do scratchpad trazidos sem mudança (18 testes do engine). A UI do M1 ganhou um
  braço que ignora os quatro eventos novos, para o commit do engine ficar verde sozinho; o passo seguinte
  (ui-egui) o substitui pelo tratamento real. 53 testes no workspace com o servidor; portão `--full` verde.
- [x] Revisor (um médio, cinco baixos): o evento de uma tarefa podia sair depois do `Disconnected` que a
  superou (o lock era solto antes do `send`). Agora `commit_if_current` confere a geração, aplica a mudança
  ao `Slot` e envia o evento sob o mesmo lock, nos quatro pontos (`Connected`/`Failed`, `DatabaseSelected`,
  `SchemaLoaded`, `ColumnsLoaded` e os `CatalogFailed`); tentativa superada ou lock envenenado é descartada.
  Documentado: `refresh` mantém a conexão e a lista de schemas (schema novo aparece ao reselecionar o banco);
  `load_schema`/`load_columns` só valem entre `DatabaseSelected` e `Disconnected`; dois `select_database` do
  mesmo banco em voo abrem duas conexões e a segunda vence (a paleta fecha ao escolher, então a UI não
  dispara em dobro). A senha é descartada, não zerada (sem `zeroize`: dependência nova é decisão).
