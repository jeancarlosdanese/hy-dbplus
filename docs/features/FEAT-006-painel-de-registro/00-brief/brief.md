# @file: docs/features/FEAT-006-painel-de-registro/00-brief/brief.md

# FEAT-006 — Painel de registro: Form, JSON e seleção múltipla

## Problema

O painel direito da aba Data mostra os campos da linha selecionada como texto corrido, sem o tipo, e ocupa
um painel inteiro com quatro números quando nada está selecionado (conferência do Jean em 22/10). O grid só
seleciona uma linha. Para a edição que vem na FEAT-007, o usuário precisa ver o registro inteiro com os
tipos, em formulário ou em JSON, e escolher mais de uma linha.

## Objetivo

Painel de registro como o do TablePlus: cada campo com nome, tipo e valor; alternância **Form / JSON**;
seleção múltipla no grid com "Múltiplos valores" nos campos que diferem; painel recolhido por padrão e os
tamanhos numa linha compacta acima do grid. Só leitura: a edição é a FEAT-007.

## Escopo (Must)

1. **Seleção múltipla** (`ui-egui`): clique seleciona uma linha; ⌘-clique (Ctrl fora do macOS) alterna;
   ⇧-clique estende da última selecionada; `selected_rows: BTreeSet<usize>` no `DataTab`; o rodapé mostra
   `N of M rows selected` quando há seleção; `PageLoaded` e troca de ordenação limpam a seleção.
2. **Linha de tamanhos** acima do grid: `rows · total_size · data_size · index_size` e o comentário, com
   quebra de linha; sai do painel.
3. **Painel**: recolhido por padrão (`side_panel_open = false`); abre ao selecionar a primeira linha; o botão
   "Painel" continua mandando. Duas abas no topo: **Form** e **JSON**.
   - Form: por campo, nome à esquerda e tipo à direita (do cache de colunas; vazio se ainda não carregou),
     valor abaixo com quebra de linha; valor com mais de 300 caracteres fica truncado com "mostrar tudo";
     `NULL` em itálico. Com mais de uma linha, campo igual em todas mostra o valor, diferente mostra
     "Múltiplos valores" em cinza.
   - JSON: uma linha vira um objeto, várias viram uma lista de objetos; chaves na ordem das colunas; por tipo
     do catálogo: `NULL` → `null`; inteiros, numéricos e flutuantes → número (texto que não parse vira string);
     `boolean` → `true/false`; `json`/`jsonb` → o valor cru (se parse); o resto → string. Texto
     monoespaçado com indentação de 2 e botão **Copiar**.
4. **`engine::record`**: `to_json(columns: &[String], types: &[Option<String>], rows: &[&[Option<String>]])
   -> String` puro, testado sem UI (a CLI e o MCP reutilizam no M8). Dependência `serde_json` na `engine`.
5. **Testes**: `engine::record` (tipos, NULL, texto que parece número em coluna `text` fica string, json
   inválido vira string, lista com duas linhas); UI com `egui_kittest`: ⌘-clique em duas linhas → rodapé
   `2 of N rows selected`, painel aberto com "Múltiplos valores" no campo que difere e o valor no campo
   igual; aba JSON mostra `"id": 1`; "Copiar" põe o JSON no clipboard do egui.

## Won't

- Edição de qualquer campo, Delete, Update: FEAT-007.
- Formatos por tipo no grid (datas, números alinhados à direita): fica para quando o grid tiver tipos nativos.
- Busca dentro do painel ("Search for field"): M7, junto com Filters.

## Restrições

- `serde_json` 1 (MIT OR Apache-2.0) já está no workspace (xtask); passa a ser dependência da `engine`.
- `Cargo.toml`, `Cargo.lock` e `crates/engine/src/session.rs` são sensíveis; `record.rs` novo no engine não
  é, mas vai junto no commit do engine.
- Commits: docs; `engine::record`; `ui-egui` (seleção, linha de tamanhos, painel Form/JSON).
