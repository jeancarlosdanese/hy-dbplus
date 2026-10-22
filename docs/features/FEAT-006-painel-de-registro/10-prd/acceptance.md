# @file: docs/features/FEAT-006-painel-de-registro/10-prd/acceptance.md

# FEAT-006 — Critérios de aceite

## `engine::record`

- [x] `to_json` com uma linha devolve um objeto; com duas, uma lista; chaves na ordem das colunas.
- [x] `NULL` vira `null`; coluna `integer`/`numeric`/`double precision` vira número; `boolean` vira `true`;
      `jsonb` válido entra cru e inválido vira string; coluna `text` com "123" fica `"123"`.

## `ui-egui`

- [x] Clique seleciona uma linha; ⌘-clique alterna; ⇧-clique estende; rodapé `N of M rows selected`;
      ordenar, paginar ou recarregar limpa a seleção.
- [x] Tamanhos, contagem e comentário aparecem numa linha acima do grid, com quebra; o painel não os repete.
- [x] Painel recolhido ao abrir a tabela; abre ao selecionar a primeira linha; botão "Painel" esconde e mostra.
- [x] Form: nome, tipo e valor por campo; valor longo truncado com "mostrar tudo"; `NULL` em itálico; com
      seleção múltipla, "Múltiplos valores" só nos campos que diferem.
- [x] JSON: objeto ou lista, indentado, tipos pelo catálogo; botão "Copiar" põe o texto no clipboard.
- [x] Teste `egui_kittest` cobre seleção múltipla, "Múltiplos valores", aba JSON e Copiar.

## Portão e CI

- [x] `scripts/quality-gate.sh --full` verde (22/10/2026, com `HY_DBPLUS_TEST_PG`).
- [x] Workflow verde (push do Jean).

## Do Jean

- [ ] Selecionar uma linha, ver o Form com os tipos; selecionar várias com ⌘ e ⇧; ver o JSON e copiar.
