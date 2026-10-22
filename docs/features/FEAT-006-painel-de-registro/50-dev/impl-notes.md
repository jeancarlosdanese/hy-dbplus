# @file: docs/features/FEAT-006-painel-de-registro/50-dev/impl-notes.md

# Notas de Implementação — FEAT-006 painel-de-registro

## Estado atual

_Atualizado em 22/10/2026._

- **Feito:** documentação; `engine::record` (`to_json`, 3 testes); `load_page` emitindo `ColumnsLoaded`
  quando ele mesmo lê as colunas; UI com seleção múltipla, linha de tamanhos, painel Form/JSON com Copiar
  (teste `egui_kittest` do grid estendido); portão `--full` verde, janela viva.
- **Em curso:** revisão (`process:reviewer`) e commits, na ordem engine, ui, docs.
- **Falta:** push, CI e conferência do Jean.
- **Depende de:** FEAT-005 (aba Data) e do fix `7717fe6`.
- **Para retomar:** esta pasta, as capturas do TablePlus citadas no brief (painel de campo com tipo; "Multiple
  values") e o `CLAUDE.md`.

## Decisões

- **[2026-10-22]** JSON montado no engine (`record::to_json`), não na UI: a CLI e o servidor MCP (M8) devolvem
  linhas em JSON e precisam da mesma conversão por tipo. `serde_json` entra na `engine`.
- **[2026-10-22]** Tipos pelo catálogo, não por inspeção do texto: "123" numa coluna `text` é string. Sem o
  cache de colunas (não deveria acontecer: a página já carrega as colunas), tudo é string.
- **[2026-10-22]** Seleção por índice da página (`BTreeSet<usize>`), limpa a cada `PageLoaded`: a FEAT-007 vai
  precisar da chave primária por linha e troca isso por identidade; até lá, índice basta.
- **[2026-10-22]** `serde_json` com `preserve_order` (chaves na ordem das colunas) e `arbitrary_precision`
  (`numeric` do PostgreSQL não cabe em `f64`: o texto do número vai como está, sem arredondar). Texto que
  não é número válido em JSON (`NaN`, `Infinity`) ou JSON inválido numa coluna `jsonb` vira string.
- **[2026-10-22]** Quem lê as colunas avisa: `load_page` passa a emitir `ColumnsLoaded` antes de `PageLoaded`
  quando leu as colunas para o desempate pela chave primária. A UI deixou de pedir `load_columns` à parte em
  `open_table` (o pedido corria com a página em voo e lia as colunas duas vezes; o teste do grid conta as
  queries e pegou isso).
- **[2026-10-22]** O Form mostra o tipo de cada campo pelo cache de colunas do workspace (`ColumnsLoaded`),
  o mesmo que o cabeçalho do grid usa; valor acima de 300 caracteres vem truncado com "mostrar tudo".
- **[2026-10-22]** Painel recolhido por padrão e aberto na primeira seleção: o Jean pediu menos espaço sem
  seleção; os tamanhos vão para uma linha acima do grid.

## Log

### 22/10/2026 — Criação

- [x] Brief, aceite, notas e índice a partir da conferência do Jean e das capturas do TablePlus.

### 22/10/2026 — Implementação (no scratchpad, depois trazida para a árvore)

- [x] `engine::record`: `row_to_value` e `to_json`; tipo pelo catálogo (`kind`): inteiros, `numeric`,
  `real`, `double precision`, `oid` → número; `boolean` → `true`/`false`; `json`/`jsonb` → cru; resto string.
- [x] `Sessions::load_page`: `ColumnsLoaded` antes de `PageLoaded` quando leu as colunas (teste ajustado).
- [x] UI: `DataTab.selected_rows`/`anchor`/`expanded_fields`; `App::select_row` (⇧ estende, ⌘ alterna);
  rodapé "N of M rows selected"; linha de tamanhos/contagem/comentário acima do grid; `ui-egui/src/record.rs`
  com Form e JSON; painel direito fechado por padrão, abre na seleção, botão "Painel".
- [x] Teste do grid: seleção, tipos no Form, ⌘-clique, "Múltiplos valores", JSON, Copiar, Painel; contagem de
  queries `3 + 2 + 1 + 4` (sem o pedido de colunas à parte).
- [x] Lições do kittest: modificadores por `Event::ModifiersChanged`; rótulos repetidos por `get_all_by_label`.

### 22/10/2026 — Revisão (`process:reviewer`, 1ª rodada)

- [x] Painel reabria em todo clique de linha, contra o brief ("o botão Painel continua mandando"):
  `select_row` abre só na transição de nenhuma linha para alguma; teste do grid estendido (fecha pelo botão,
  clica outra linha, painel segue fechado).
- [x] Busca "coluna do catálogo pelo nome" escrita duas vezes (cabeçalho do grid e painel):
  `Workspace::catalog_columns`, usada pelos dois.
- [x] Teste sem harness de `select_row`: ⇧ sem âncora, para trás e para frente; clique simples na única
  selecionada desmarca; painel só na primeira seleção. O teste pegou um bug: o ⇧-clique sem âncora não a
  definia, e o ⇧ seguinte estendia da própria linha em vez da primeira clicada (`anchor.get_or_insert`).
- [x] BRIEF §3.4 alinhado (tamanhos acima do grid; painel Form/JSON).
- [ ] Anotado para o M8: `to_json` devolve `"null"` se a serialização falhar (hoje não há caminho de erro);
  quando a CLI/MCP usar a função, devolver `Result` e deixar o chamador avisar.
- Conferido pelo revisor, sem achado: `arbitrary_precision` com `NaN`/`Infinity`/`+1`/`1.`/`.5`/`00012`
  vira string, `12.50` e inteiros longos preservados, `1e999` passa como número (válido na gramática JSON;
  consumidor sem `arbitrary_precision` rejeita); cache de colunas sob `commit_if_current`; índices de
  seleção sempre por `get`.
