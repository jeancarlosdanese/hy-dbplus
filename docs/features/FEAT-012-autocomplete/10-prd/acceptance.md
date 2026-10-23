# @file: docs/features/FEAT-012-autocomplete/10-prd/acceptance.md

# FEAT-012 — Critérios de aceite

## `sql::complete`

- [x] `from cl` sugere `clientes` (schema atual); `public.` lista as tabelas e views do schema; `c.` com
      `clientes c` ou `clientes as c` no `FROM`/`JOIN`/`UPDATE`/`INSERT INTO` lista as colunas com o tipo.
- [x] Fora de `FROM`/`JOIN`, as colunas das tabelas do statement vêm antes de keywords e tabelas; o que
      começa com o prefixo vem antes do que só contém; no máximo 50 itens.
- [x] Dentro de string, comentário ou identificador entre aspas não sugere; keyword segue a caixa do prefixo;
      nome que precisa de aspas sai com aspas.
- [x] Highlight, Run Current e autocomplete tokenizam o buffer uma vez por mudança de texto.

## `ui-egui`

- [x] Popup abre ao digitar identificador ou após `.`, e por ⌃Space; ↑/↓, Tab/Enter aceitam, Esc fecha;
      com o popup fechado, Enter insere nova linha e Tab tabulação, como hoje.
- [x] Tabela sem colunas carregadas: o editor pede ao engine e a lista aparece quando chegam.
- [x] Teste `egui_kittest` cobre aceitar keyword, tabela e coluna, pedir colunas, ↑/↓, Tab, Enter, Esc
      (foco fica e o Enter seguinte é nova linha), string e ⌃Space.

## Portão e CI

- [x] `scripts/quality-gate.sh --full` verde; [ ] workflow verde (push do Jean).

## Do Jean

- [ ] Num banco real: `select * from ` + duas letras de uma tabela, aceitar com Tab; `alias.` listar colunas
      com tipo; digitar normalmente sem o popup atrapalhar o Enter; ⌃Space no meio de um `WHERE`.
