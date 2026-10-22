# @file: docs/features/FEAT-009-structure-e-index/50-dev/impl-notes.md

# Notas de Implementação — FEAT-009 structure-e-index

## Estado atual

_Atualizado em 22/10/2026._

- **Feito:** documentação.
- **Em curso:** nada.
- **Falta:** `catalog::structure` e `catalog::ddl`; `engine::load_structure`; `ui-egui` (sub-abas, Structure,
  Index, caixa do DDL); verificação do Jean.
- **Depende de:** FEAT-008 (aba de tabela).
- **Para retomar:** esta pasta, BRIEF §3.5 e §3.6, e o `CLAUDE.md`.

## Decisões

- **[2026-10-22] DDL montado no catálogo, não pedido ao servidor.** O PostgreSQL não tem `pg_get_tabledef`;
  o `CREATE TABLE` sai do `TableStructure` (puro, testável), com as definições que o servidor escreve
  (`pg_get_constraintdef`, `pg_get_indexdef`, `pg_get_expr`, `format_type`) para não reinventar sintaxe. O
  teste com servidor roda o DDL gerado num schema novo e compara a estrutura: é a prova de que o DDL é válido
  e completo.
- **[2026-10-22] Índices de PK e UNIQUE não se repetem.** A restrição já cria o índice; o bloco de índices
  lista só os que não implementam restrição (`pg_index.indisprimary` e `pg_constraint.conindid`).
- **[2026-10-22] Sub-aba por aba, Data como padrão.** `TabView` vive no `DataTab`; trocar de sub-aba não
  recarrega dados nem mexe na pendência; Structure/Index pedem a estrutura uma vez e usam o cache do engine.
- **[2026-10-22] Somente leitura.** Editar nome, tipo, chave e índice fica fora (brief Won't).

## Plano

1. `catalog`: modelo (`Column` com comentário/identidade/gerada; `Constraint`, `Index`, `TableStructure`),
   `structure()` e `ddl::table_ddl` → testes puros e com servidor. Commit `feat(catalog)`.
2. `engine`: `load_structure`, cache em `Database.structures`, `StructureLoaded`, `refresh` esvazia → teste
   com driver falso. Reviewer. Commit `feat(engine)`.
3. `ui-egui`: `TabView`, rodapé, Structure, Index, caixa do DDL → kittest. Reviewer. Commit `feat(ui)`.
4. Docs: aceite marcado, notas, índice; BRIEF §3.5/§3.6 com o que ficou fora.

## Log

### 22/10/2026 — Criação

- [x] Brief, aceite, notas e índice a partir do BRIEF §3.5/§3.6 e do marco M4.
