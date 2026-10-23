# @file: docs/features/FEAT-012-autocomplete/50-dev/impl-notes.md

# Notas de Implementação — FEAT-012 autocomplete

## Estado atual

_Atualizado em 23/10/2026._

- **Feito:** docs (`a810dc7`); passo 1, cache de tokenização (`sql::Analysis`; `split`/`highlight`/
  `statement_at` por cima das mesmas partes; `QueryTab.analysis` refeita só quando o texto muda; teste
  comparando com as funções livres) na árvore, aguardando reviewer e "pode commitar".
- **Falta:** `sql::complete` + keywords; popup na UI; testes; docs.
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
