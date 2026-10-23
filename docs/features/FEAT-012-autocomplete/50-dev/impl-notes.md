# @file: docs/features/FEAT-012-autocomplete/50-dev/impl-notes.md

# Notas de Implementação — FEAT-012 autocomplete

## Estado atual

_Atualizado em 23/10/2026._

- **Feito:** docs (`a810dc7`); passo 1, cache de tokenização (`sql::Analysis`; `split`/`highlight`/
  `statement_at` por cima das mesmas partes; `QueryTab.analysis` refeita só quando o texto muda; teste
  comparando com as funções livres) commitado em `31b3eb8`; passo 2, `sql::complete` (`CatalogView`,
  `Completion { replace, items, needs }`, `Item`, `ItemKind`, `Need`, `ident`; 5 testes) na árvore,
  aguardando reviewer e "pode commitar".
- **Falta:** popup na UI (passo 3); testes; docs.
- **Depende de:** FEAT-010 (editor, `statement_at`, tokenizador) e FEAT-004/009 (catálogo na UI).
- **Para retomar:** este arquivo, o brief, `crates/sql/src/{split,highlight}.rs` e
  `crates/ui-egui/src/editor.rs` (atalhos consumidos antes do `TextEdit`, cursor lido do estado).

## Decisões (fechadas com o Jean em 23/10/2026)

- **Disparo automático** ao digitar, mais ⌃Space; alternativa "só ⌃Space" descartada.
- **Tab e Enter aceitam** (padrão TablePlus e VS Code); Esc fecha e devolve o Enter ao editor.
- **Keyword segue a caixa do prefixo.**
- **Tabela sem schema = schema selecionado na árvore**; outro schema só qualificado.
- **Funções entram** como Should, depois de keywords, tabelas e colunas.
- **Motor puro na `sql`**, com visão emprestada do catálogo da UI: nada de clonar o catálogo por tecla.
- **Cache de tokenização entra junto**: o popup roda a cada tecla e já havia duas tokenizações por frame.

## Plano

1. `refactor(sql,ui)`: `Analysis` com tokens/statements/spans; `highlight`/`statement_at` por cima;
   `QueryTab.analysis` refeita só quando o buffer muda. Testes existentes continuam. Reviewer (split).
2. `feat(sql)`: `complete.rs` + `keywords.rs` + `CatalogView` → testes puros. Reviewer.
3. `feat(ui)`: `complete.rs` (popup, teclas, inserção, pedidos ao engine) → kittest. Reviewer.
4. Docs: aceite, notas, índice; BRIEF §3.7 já cita.

## Riscos e lições a conferir

- Retângulo do cursor: `TextEditOutput.galley` + `cursor_range` dão a posição; o popup é uma `Area` com
  `Order::Foreground` ancorada ali, dentro da `ScrollArea` do editor (conferir o offset do scroll).
- Consumir Enter/Tab só com o popup aberto: `lock_focus(true)` já segura o Tab no `TextEdit`; o Enter
  precisa ser consumido em `input_mut` antes do widget, como ⌘↩ hoje.
- `egui_kittest`: texto por `Event::Text`, popup por `click_accesskit()` (lições da FEAT-010).

## Log

### 23/10/2026 — Passo 2: `sql::complete`

- `complete(&Analysis, cursor, &CatalogView) -> Option<Completion>`: a visão emprestada do catálogo tem a
  forma dos mapas do `Workspace` (`objects` por schema, `columns` por `(schema, tabela)`), nada é clonado
  por tecla. `Completion.needs` diz à UI o que pedir ao engine (`Need::Columns` das tabelas do statement
  sem colunas na visão; `Need::Schema` de um `schema.` ainda sem objetos).
- Contexto: tokens do statement sob o cursor (só os dele: `select c.` depois de um `;` não vê o `FROM` do
  statement anterior); prefixo = palavra sem aspas que contém o cursor; `q.` resolve alias → schema → tabela
  do statement sem alias → tabela do schema atual; `FROM`/`JOIN`/`INTO`/`UPDATE`/`TABLE` → tabelas, views e
  schemas; resto → colunas das tabelas do statement, aliases, keywords, tabelas e funções do schema atual.
  `table_refs` lê `[schema.]tabela [AS] [alias]` com vírgulas depois de `FROM`/`JOIN`/`UPDATE`/`INTO`;
  keyword no lugar do alias não é alias; `schema.` sem tabela ainda não é referência.
- Ordem: começa com o prefixo antes de contém (caixa ignorada), estável dentro do grupo, rótulo repetido
  sai uma vez, no máximo `MAX_ITEMS` (50). Keyword na caixa do prefixo. `ident` põe aspas quando o nome não
  é `[a-z_][a-z0-9_$]*` ou é keyword (`order` → `"order"`).
- A lista de keywords é a `KEYWORDS` do `highlight.rs` (não há `keywords.rs` separado): uma lista só para
  pintar e sugerir.

### 23/10/2026 — Passo 1: `Analysis`

- `Analysis::new(text)` guarda tokens com posição em bytes (`byte_ranges`), statements (`split_tokens`) e
  spans (`spans_of`); `statement_of` é a regra do `statement_at` sobre statements prontos. `Analysis::refresh`
  compara o texto e só refaz quando mudou. No editor a análise sai da aba no começo do frame, o layouter a
  refaz com o texto que o TextEdit passa (já com a tecla do frame) e ela volta à aba no fim;
  `QueryTab::statement_at` a reaproveita no Run Current. `Analysis::tokens()` público, para o motor.
- A lista `KEYWORDS` do `highlight.rs` é a base da lista do autocomplete (passo 2).

### 23/10/2026 — Criação

- Brief, aceite, notas e índice a partir da conversa com o Jean (cinco recomendações aceitas; ordem:
  depois da FEAT-013).
