# @file: docs/features/FEAT-008-bancos-e-abas/50-dev/impl-notes.md

# Notas de Implementação — FEAT-008 bancos-e-abas

## Estado atual

_Atualizado em 22/10/2026._

- **Feito:** documentação; `engine::close_database` + `DatabaseClosed` (teste: fecha só aquele banco, os
  outros seguem do cache, o fechado reabre pelo servidor, desconhecido não avisa).
- **Em curso:** revisão e commit do engine.
- **Falta:** UI passo 1 (workspace por banco e faixa); UI passo 2 (abas); verificação do Jean.
- **Depende de:** FEAT-007 (edição por aba).
- **Para retomar:** esta pasta, a captura do TablePlus citada no brief e o `CLAUDE.md`.

## Decisões

- **[2026-10-22] Modelo do TablePlus, numa janela.** Faixa de bancos abertos à esquerda (cada item é conexão +
  banco), árvore do banco selecionado e abas de tabela do banco selecionado. Abas são por banco, não globais:
  trocar na faixa troca o conjunto de abas. Várias janelas nativas ficam para depois.
- **[2026-10-22] Uma aba por tabela.** Clicar numa tabela já aberta foca a aba. A mesma tabela em duas abas
  não entra (o TablePlus também não abre por padrão).
- **[2026-10-22] Engine sem mudança de modelo.** `Slot.databases` já guarda um banco por conexão PostgreSQL;
  entra só `close_database` (hoje só `disconnect` fecha, e fecha tudo).
- **[2026-10-22] Chave do workspace.** `HashMap<(ConnectionId, String), Workspace>`; os eventos trazem os dois
  campos. `View::Workspace` sem conexão: o ativo vem de `App.active`. As travas de "trocar de tabela" e da
  paleta (FEAT-007) saem: a pendência vive na aba.

## Plano

1. `engine`: `close_database` + `DatabaseClosed`; teste. Reviewer. Commit `feat(engine)`.
2. UI passo 1: `Workspace` por (conexão, banco), `rail`/`active`, faixa, ⌘K acumulando, barra superior do
   ativo, roteamento dos eventos; `data: Option<DataTab>` ainda único por banco. Testes ajustados + teste de
   dois bancos. Reviewer. Commit `feat(ui)`.
3. UI passo 2: `tabs`/`active_tab`, barra de abas, ×/⌘W, confirmação ao fechar, árvore focando/abrindo aba,
   eventos por aba, saída das travas de trocar de tabela e da paleta. Teste de duas abas e pendência por aba.
   Reviewer. Commit `feat(ui)`.
4. Docs: aceite marcado, notas, índice; BRIEF §3.3/§3.4 (faixa e abas).

## Log

### 22/10/2026 — Criação

- [x] Brief, aceite, notas e índice a partir do pedido do Jean ("abrir mais de um DB e mais de uma tabela
  por DB ao mesmo tempo") e da captura do TablePlus.

### 22/10/2026 — `engine::close_database`

- [x] `Sessions::close_database(id, database)`: tira o banco de `slot.databases` (caches e a conexão própria
  somem quando o último `Arc` cai) e emite `DatabaseClosed`; a sessão e os outros bancos seguem. Tarefa em
  voo daquele banco termina e o evento dela ainda sai: a UI, sem o banco, ignora (mesmo caso de hoje para
  tabela fechada). `disconnect` continua limpando tudo.
- [x] Na UI, só o braço `DatabaseClosed => {}` para compilar; a faixa o trata no passo 1.

### 22/10/2026 — Revisão do engine (`process:reviewer`): nada bloqueante

- [x] Comentário na UI apontava "passo 2" (é o passo 1 que trata `DatabaseClosed`).
- [x] Teste do caso em voo: `load_page` pendente, `close_database`, o `PageLoaded` ainda sai e o banco não
  ressuscita (reabrir vai ao servidor); `close_database` em conexão desconhecida não avisa.
- Para o passo 1 da UI: `close_database` de um banco ainda abrindo (`select_database` em voo) é no-op e o
  banco entra aberto depois. A faixa só ganha o item no `DatabaseSelected`, então o × nunca aparece
  enquanto abre; se a UI um dia mostrar "abrindo…" com ×, tem de repetir o `close_database` ao receber o
  `DatabaseSelected` de um banco que já fechou.
