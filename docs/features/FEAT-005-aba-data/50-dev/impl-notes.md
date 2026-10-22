# @file: docs/features/FEAT-005-aba-data/50-dev/impl-notes.md

# Notas de Implementação — FEAT-005 aba-data

## Estado atual

_Atualizado em 21/10/2026._

- **Feito:** documentação.
- **Em curso:** código no scratchpad (cópia completa do workspace), enquanto o último commit da FEAT-004
  aguarda o revisor.
- **Falta:** `catalog`; `engine`; `ui-egui`; verificação do Jean.
- **Depende de:** FEAT-004 concluída (M2).
- **Para retomar:** esta pasta, o `CLAUDE.md` da raiz e o BRIEF §3.4, §5 e decisão 3.

## Decisões

- **[2026-10-21]** Paginação `LIMIT/OFFSET` (BRIEF decisão 3); ordenação por `ORDER BY "coluna" ASC|DESC`
  montada pelo `catalog` com identificadores entre aspas duplas dobradas. Sem ordem explícita o PostgreSQL
  não garante a ordem das páginas; a UI avisa no contador quando não há ordenação? Não: segue o TablePlus,
  que também não avisa. Fica anotado.
- **[2026-10-21]** `count(*)` exato por página aberta (uma query a mais por carga). Em tabela enorme isso é
  lento; a troca por `reltuples` estimado fica para quando aparecer, com aviso "~" no contador.
- **[2026-10-21]** Grid com `egui_extras::TableBuilder` e `body.rows(altura, n, ...)`: só as linhas visíveis
  são desenhadas; a página inteira (até 1000 linhas de texto) fica em memória na UI.
- **[2026-10-21]** Clicar no nome da tabela abre a aba; a seta expande. Na árvore, `CollapsingState` com
  `show_header` permite o cabeçalho customizado (seta + rótulo selecionável), em vez do `CollapsingHeader`
  que faz do cabeçalho inteiro o toggle.
- **[2026-10-21]** `egui_extras` 0.36 (MIT OR Apache-2.0) com `default-features = false`: só a tabela; sem
  carregadores de imagem.

## Log

### 21/10/2026 — Criação

- [x] Leitura do BRIEF §3.4 e decisão 3; API do `egui_extras::TableBuilder` 0.36 no registro local; brief,
  aceite, notas e índice antes do código.
