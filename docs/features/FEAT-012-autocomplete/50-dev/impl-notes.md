# @file: docs/features/FEAT-012-autocomplete/50-dev/impl-notes.md

# Notas de Implementação — FEAT-012 autocomplete

## Estado atual

_Atualizado em 23/10/2026._

- **Feito:** docs (`a810dc7`); passo 1, cache de tokenização (`sql::Analysis`; `split`/`highlight`/
  `statement_at` por cima das mesmas partes; `QueryTab.analysis` refeita só quando o texto muda; teste
  comparando com as funções livres) commitado em `31b3eb8`; passo 2, `sql::complete` (`CatalogView`,
  `Completion { replace, items, needs }`, `Item`, `ItemKind`, `Need`, `ident`; 5 testes) commitado em
  `12f5def`; consertos da revisão dele (`fix(sql)`) e passo 3, o popup na UI (`CompletionState`,
  `accept_completion`, teclas, `show_popup`, pedidos ao engine; kittest
  `autocomplete_sugere_keyword_tabela_e_coluna_e_as_teclas_aceitam_ou_fecham`) na árvore, portão `--full`
  verde, aguardando reviewer e "pode commitar".
- **Falta:** push e conferência do Jean (aceite "Do Jean").
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
2. `feat(sql)`: `complete.rs` + `CatalogView` (keywords do `highlight.rs`) → testes puros. Reviewer.
3. `feat(ui)`: popup em `editor.rs`, estado em `app.rs` (teclas, inserção, pedidos ao engine) → kittest. Reviewer.
4. Docs: aceite, notas, índice; BRIEF §3.7 já cita.

## Riscos e lições a conferir

- Retângulo do cursor: `TextEditOutput.galley` + `cursor_range` dão a posição; o popup é uma `Area` com
  `Order::Foreground` ancorada ali, dentro da `ScrollArea` do editor (conferir o offset do scroll).
- Consumir Enter/Tab só com o popup aberto: `lock_focus(true)` já segura o Tab no `TextEdit`; o Enter
  precisa ser consumido em `input_mut` antes do widget, como ⌘↩ hoje.
- `egui_kittest`: texto por `Event::Text`, popup por `click_accesskit()` (lições da FEAT-010).

## Log

### 23/10/2026 — Revisão do passo 3 (`process:reviewer`): aplicada

- [x] **Média.** O popup sobrevivia à perda do foco do editor (clique na busca da árvore): invisível, mas
  seguia consumindo ↑/↓/Tab/Enter/Esc do app inteiro, e o Enter dado em outro widget aceitava a sugestão
  no buffer. Agora as teclas só são consumidas com o editor focado no frame anterior
  (`had_focus_last_frame`, porque o Esc já tirou o foco neste), e sem cursor na tela o popup fecha. Teste.
- [x] **Baixa.** `Need` duplicado no ramo do `.` (`app.x a where app.|` pedia o schema duas vezes):
  `push_need` com `contains`. Teste.
- [x] **Baixa.** Tab/Enter com o popup em "carregando colunas…" sumiam sem efeito: só são consumidos com
  itens; `CatalogFailed` fecha os popups do banco.
- [x] **Baixa.** Buffer mudado fora do TextEdit (Beautify, SQL gerado) deixava `replace` velho no popup:
  `completion = None` nos dois.
- [x] **Baixa.** Shift+↑/↓/Tab/Enter eram do popup: com Shift nada é consumido.
- [x] **Baixa.** Backspace, colar e ⌘Z abriam o popup: só texto inserido (buffer cresceu, sem `Paste` e
  sem ⌘) abre. Brief ajustado.
- [x] **Baixa.** Aceite pedia Tab com o popup fechado e clique no item: no teste.

### 23/10/2026 — Passo 3: o popup

- `QueryTab.completion: Option<CompletionState { replace, items, selected, requested }>`; o estado é o do
  frame anterior: as teclas (↑/↓/Tab/Enter/Esc, consumidas antes do TextEdit só com o popup aberto; ⌃Space
  sempre) agem sobre ele no começo do frame, e `accept_completion` troca o prefixo pelo rótulo com o cursor
  pelo `pending_cursor` (o mesmo caminho do SQL gerado da FEAT-013). Depois do TextEdit (`show`, pelo galley
  e pelo cursor), `keep_open` decide: aberto e sem tecla de fechar, ou identificador/`.` digitado, ou
  ⌃Space; cursor que anda sem texto fecha; texto que muda por outro caractere fecha. `complete` roda com a
  análise do texto final e a visão dos mapas do `Workspace`; o item escolhido sobrevive se a lista é igual.
  `needs` ainda não pedidos vão ao engine e entram em `requested` (morre com o popup; o engine responde
  do cache na reabertura).
- `show_popup`: `Area` em `Order::Foreground` ancorada embaixo do cursor (`galley.pos_from_cursor` +
  `galley_pos`), `Frame::popup`, até dez linhas com rolagem, rótulo monoespaçado + tipo + detalhe, clique
  aceita; sem itens e com pedido em voo, "carregando colunas…".
- Lição: o egui solta o foco no Esc em `Focus::begin_pass`, antes de qualquer widget ver a tecla; com o
  popup aberto o editor pede o foco de volta no mesmo frame. `consume_key` basta para Tab/Enter/setas: o
  TextEdit só vê o que sobrou no `InputState`.

### 23/10/2026 — Revisão do passo 2 (`process:reviewer`): aplicada

- [x] **Média.** `Need::Columns` nascia do prefixo em digitação (`from cl|` → colunas de `cl`) e de palavras
  tomadas por tabela (`ONLY`, `generate_series`): uma query ao servidor por tecla, com resposta vazia
  cacheada. Agora só de tabela que existe em `objects[schema]`; schema conhecido sem objetos vira
  `Need::Schema`; schema desconhecido, nada. Dedup por `contains`. Testes.
- [x] **Média.** Identificador sem aspas não era dobrado para minúsculas: `FROM Clientes c` nunca listava
  coluna (e pedia `Clientes` ao servidor). `plain()` dobra schema e tabela sem aspas; com aspas fica como
  está. Testes.
- [ ] **Baixa (fica).** `ident` usa a lista de pintura como "reservadas": `count`, `type`, `key`, `start`,
  `comment` saem com aspas desnecessárias (SQL válido, só feio). Lista de reservadas à parte quando
  incomodar.
- [x] **Baixa.** Brief com `needs` no contrato, ordem estável por grupo, e keywords do `highlight.rs`
  (plano e índice também).

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
