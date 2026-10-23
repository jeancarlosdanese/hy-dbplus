# @file: docs/features/FEAT-013-gerar-sql-pela-arvore/50-dev/impl-notes.md

# Notas de Implementação — FEAT-013 gerar-sql-pela-arvore

## Estado atual

_Atualizado em 23/10/2026._

- **Feito:** nada além destes docs.
- **Falta:** `catalog::generate`; menu na árvore e `App::generate_sql` com pedido pendente; testes; docs.
- **Depende de:** FEAT-009 (colunas e estrutura no catálogo, `table_ddl`), FEAT-010 (abas SQL) e
  FEAT-011 (autosave), todas commitadas.
- **Para retomar:** este arquivo, o brief, `crates/catalog/src/{edit,data,ddl}.rs` como molde de montagem
  de SQL, `crates/ui-egui/src/tree.rs` (linha da tabela) e `App::open_query_tab`.

## Decisões

- **[2026-10-23] Modelos montados no `catalog`, não na UI**: é SQL a partir do modelo do catálogo, como
  `page_sql`, `changes_sql` e `table_ddl`; puro e testável sem egui, e o CLI/MCP (M8) pode reaproveitar.
- **[2026-10-23] Nada roda pelo menu** (decisão do Jean no desenho): o menu só escreve no editor. O
  `DROP CASCADE` e o `TRUNCATE` ficam a um ⌘↩ de distância, mas é o usuário que aperta.
- **[2026-10-23] Destino do texto**: aba SQL ativa → última aba SQL do banco → aba nova. Sempre no fim do
  buffer com uma linha em branco antes, nunca no cursor: inserir no meio de um statement em edição faria
  o Run Current rodar uma mistura.
- **[2026-10-23] Colunas sob demanda** pelo mesmo `load_columns` da árvore, com um pedido pendente por
  banco (`Workspace.pending_sql: Option<PendingSql { schema, table, template, tab }>`); pedido novo
  substitui o anterior. CREATE usa `load_structure`; o `StructureLoaded` hoje só alimenta a aba Data da
  tabela (`app.rs:854`), passa a conferir o pendente também.
- **[2026-10-23] Valores de exemplo por tipo** (`data_type` do `format_type`): `character`, `text`,
  `varchar` → `''`; `integer`, `bigint`, `smallint`, `numeric`, `real`, `double`, `serial` → `0`;
  `boolean` → `false`; `date`, `time`, `timestamp` → `now()`; `uuid` → `gen_random_uuid()`; `json`,
  `jsonb` → `'{}'`; resto → `NULL`. Comparação pelo começo do tipo, em minúsculas.
- **[2026-10-23] Lista do menu fechada com o Jean** (23/10): SELECT, SELECT count, INSERT, UPDATE, DELETE,
  TRUNCATE, DROP CASCADE, CREATE; sem `SELECT … WHERE pk`, sem `COPY`.

## Plano

1. `catalog::generate`: `Template`, `template_sql`, valores por tipo, aspas, view → testes. Reviewer.
   Commit `feat(catalog)`.
2. `ui-egui`: `context_menu` na linha da tabela/view; `App::generate_sql` + `pending_sql` + conclusão em
   `ColumnsLoaded`/`StructureLoaded`/falhas; inserir no fim, cursor (gravar no `TextEdit::State` do
   editor, como o `cursor` é lido hoje) e foco → kittest. Reviewer. Commit `feat(ui)`.
3. Docs: aceite, notas, índice; BRIEF §3.3.

## Riscos e lições a conferir

- `egui_kittest`: o clique secundário no nó (`click_secondary`/evento `PointerButton::Secondary`) e o
  menu de contexto como `Area` — conferir como os itens aparecem na árvore de acessibilidade antes de
  escrever o teste (lições da FEAT-010: botões de popup por `click_accesskit()`).
- Pôr o cursor programaticamente: `TextEdit::load_state` + `cursor.set_char_range` + `store`, e
  `ctx.memory_mut(|m| m.request_focus(editor_id))`; o `QueryTab.cursor` é copiado do estado a cada frame,
  então os dois precisam andar juntos no mesmo frame.

## Log

### 23/10/2026 — Criação

- Brief, aceite, notas e índice a partir da conversa com o Jean (menu de contexto; ordem: FEAT-013 antes
  da FEAT-012 autocomplete).
