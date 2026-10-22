# @file: docs/features/FEAT-011-queries-salvas/50-dev/impl-notes.md

# Notas de Implementação — FEAT-011 queries-salvas

## Estado atual

_Atualizado em 22/10/2026._

- **Feito:** documentação.
- **Em curso:** nada.
- **Falta:** `engine::queries` (`QueryStore`); `ui-egui` (restaurar, salvar ao perder o foco e ao fechar,
  apagar ao descartar, `on_exit`); `main.rs` com o `QueryStore` no `Deps`; verificação do Jean.
- **Depende de:** FEAT-010 (abas SQL), que está na árvore aguardando os commits.
- **Para retomar:** esta pasta e o `ConnectionStore` (`crates/engine/src/store.rs`) como molde.

## Decisões

- **[2026-10-22] Um arquivo por aba, salvo ao perder o foco** (decisão do Jean): sobrevive a uma queda do app,
  dá para abrir num editor de texto e não exige formato. TOML único por banco ficou de fora.
- **[2026-10-22] Caminho pelo id da conexão**, não pelo nome: o nome muda na edição da conexão; o UUID não.
  O nome do banco entra escapado (`%xx` fora de `[A-Za-z0-9_-]`) para não sair do diretório.
- **[2026-10-22] Numeração por banco.** `SQL Query n` e `n.sql` casam; o id da aba só precisa ser único por
  banco porque os eventos do engine trazem conexão e banco. Sai o `next_query_id` global da FEAT-010.
- **[2026-10-22] Aba restaurada não está `running`.** A UI só aplica `SqlResult` quando a aba existe e está
  `running` (4ª revisão da FEAT-010): com a numeração por banco, um resultado em voo do banco fechado não
  cai na aba restaurada com o mesmo número.
- **[2026-10-22] Só o texto.** Resultados, cursor e limite ficam em memória; o histórico é do M7.

## Plano

1. `engine::queries`: `QueryStore::new(dir)`/`default_dir()`, `list`, `save`, `remove`, `remove_connection`,
   escape do nome do banco → testes com diretório temporário. Reviewer. Commit `feat(engine)`.
2. `ui-egui` + `main.rs`: `Deps.queries`, restaurar no `DatabaseSelected`, `QueryTab.saved`, salvar em
   `lost_focus`/fechar/`on_exit`, apagar no descartar, `remove_connection` na lista → kittest com diretório
   temporário. Reviewer. Commit `feat(ui)`.
3. Docs: aceite, notas, índice; BRIEF §3.7 (abas SQL salvas).

## Log

### 22/10/2026 — Criação

- [x] Brief, aceite, notas e índice a partir do pedido do Jean ("podemos guardar as queries?") e das
  decisões dele: arquivos soltos, salvos ao perder o foco.
