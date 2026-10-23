# @file: docs/features/FEAT-012-autocomplete/50-dev/impl-notes.md

# Notas de Implementação — FEAT-012 autocomplete

## Estado atual

_Atualizado em 23/10/2026._

- **Feito:** nada além destes docs. Vem depois da FEAT-013 (decisão do Jean, 23/10).
- **Falta:** cache de tokenização; `sql::complete` + keywords; popup na UI; testes; docs.
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

### 23/10/2026 — Criação

- Brief, aceite, notas e índice a partir da conversa com o Jean (cinco recomendações aceitas; ordem:
  depois da FEAT-013).
