# @file: docs/features/FEAT-008-bancos-e-abas/50-dev/impl-notes.md

# Notas de Implementação — FEAT-008 bancos-e-abas

## Estado atual

_Atualizado em 22/10/2026._

- **Feito:** documentação; `engine::close_database` + `DatabaseClosed` (teste: fecha só aquele banco, os
  outros seguem do cache, o fechado reabre pelo servidor, desconhecido não avisa).
- **Feito também:** UI passo 1: workspace por (conexão, banco), faixa de bancos, ⌘K acumulando, foco por
  conexão, roteamento dos eventos, × com confirmação (teste de dois bancos).
- **Em curso:** revisão e commit da UI passo 1.
- **Falta:** UI passo 2 (abas por tabela); verificação do Jean.
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

### 22/10/2026 — UI passo 1: workspace por banco e faixa

- [x] `App`: `workspaces: HashMap<DbKey, Workspace>` (`DbKey = (ConnectionId, String)`), `rail` (ordem da
  faixa), `active` (banco selecionado), `focus` (conexão do workspace na tela, mesmo sem banco),
  `conn_state` (banco abrindo e erro da última tentativa, fora de qualquer banco), `confirm_close`.
  `View::Workspace` sem conexão; `Workspace` perdeu `database` (está na chave).
- [x] Eventos: `DatabaseSelected` cria o workspace e o põe na faixa só se ainda não existe (banco já aberto só
  vira o ativo, sem perder árvore e aba); `CatalogFailed` vai ao workspace do banco (schema que não carregou)
  ou ao `conn_state` (banco que não abriu); `DatabaseClosed` tira da faixa (`forget_database`, o último da
  faixa assume); `Disconnected` tira todos os bancos da conexão e, sem foco, volta à lista.
- [x] `rail.rs`: item por banco com cor do ambiente, nome do banco e da conexão, × (confirmação por `Modal`
  quando há pendência; "Descartar e fechar" chama `close_database`). Rótulos não selecionáveis para o clique
  chegar ao item.
- [x] `workspace.rs` por `focus`: barra superior com o banco ativo, faixa, árvore do ativo (ou "Abrindo
  banco…"/"Nenhum banco selecionado." com o erro do `conn_state`), centro e painel do ativo.
- [x] `tree`, `data`, `record`, `commit` recebem `&DbKey`; a paleta chama `App::open_database` (já aberto só
  seleciona) e perdeu a trava da FEAT-007 (trocar de banco não zera mais nada); "Abrir" na lista foca a
  conexão e o primeiro banco dela na faixa.
- [x] Testes: `dois_bancos_na_faixa_trocam_arvore_e_aba_e_fecham_com_confirmacao`; os demais migrados para a
  chave (`workspace_of(id)` nos testes). Os testes do grid ganharam janela de 1400 px: com a faixa, a célula
  "a" caía sob o painel direito nos 800 px padrão e o clique não chegava ao grid.
- Fica para o passo 2: "Desconectar" com pendência sem confirmação (hoje fecha tudo); `open_table` ainda troca
  a tabela da aba única e mantém a trava de trocar de tabela.
