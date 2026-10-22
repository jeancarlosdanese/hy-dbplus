# @file: docs/features/FEAT-008-bancos-e-abas/00-brief/brief.md

# FEAT-008 — Vários bancos abertos e abas por tabela

## Problema

A UI mostra uma conexão por vez, com um banco e uma tabela: trocar de banco pelo ⌘K zera a árvore e a aba,
e abrir outra tabela fecha a anterior. O Jean quer o modelo do TablePlus (captura de 22/10): uma faixa à
esquerda com os bancos abertos (cada um é conexão + banco, com sua árvore), e, no banco selecionado, abas
de tabela (e de SQL, no M5) lado a lado. O engine já sustenta isso: guarda várias conexões e, por conexão,
vários bancos, cada um com a sua conexão PostgreSQL. A limitação é só da UI.

## Objetivo

Uma janela com três colunas: a **faixa de bancos** (ícone, nome do banco e a cor do ambiente; clicar troca
árvore e abas), a **árvore** do banco selecionado e o **centro com abas** (uma por tabela aberta, × para
fechar). ⌘K abre um banco a mais na faixa, sem fechar nada. Cada aba guarda a sua página, seleção e
pendência de edição (FEAT-007), e a trava de "trocar de tabela" deixa de existir: abrir outra tabela é
abrir outra aba.

## Escopo (Must)

1. **Estado da UI** (`app.rs`): `Workspace` passa a ser por (conexão, banco), com árvore, caches e a lista de
   abas (`tabs: Vec<DataTab>`, `active_tab`); `rail: Vec<(ConnectionId, String)>` é a ordem dos bancos
   abertos e `active: Option<(ConnectionId, String)>` o selecionado. `View::Workspace` deixa de carregar a
   conexão: mostra o banco ativo. Todo evento do engine traz conexão e banco e vai ao workspace certo; os de
   dados (`ColumnsLoaded`, `StatsLoaded`, `PageLoaded`, `DataFailed`, `ChangesApplied`, `ChangesFailed`)
   vão à aba daquela tabela.
2. **Faixa de bancos** (`rail.rs`, 84 px à esquerda): um item por banco aberto, ícone de banco, nome do
   banco em duas linhas se precisar, cor do ambiente da conexão na borda; o ativo em destaque; clique troca;
   × fecha o banco: fecha as abas dele (confirmação se alguma tem pendência) e pede ao engine
   `close_database`. "← Conexões" no topo volta à lista sem fechar nada; "Desconectar" na lista fecha os
   bancos daquela conexão (com a mesma confirmação).
3. **⌘K** abre o banco escolhido: se já está na faixa, só seleciona; senão o engine abre (`select_database`
   já faz) e ele entra no fim da faixa, selecionado. A barra superior mostra `AMBIENTE | PostgreSQL versão :
   conexão : banco` do banco ativo.
4. **Abas** (`tabs.rs`): barra acima do grid, uma aba por tabela aberta do banco ativo, rótulo
   `schema.tabela` (só `tabela` quando o schema é o atual), × em cada aba, ⌘W fecha a ativa; fechar aba com
   pendência pede confirmação (Descartar / Cancelar). Clicar numa tabela da árvore abre uma aba nova ou foca
   a já aberta (uma aba por tabela). A árvore destaca a tabela da aba ativa. Sem aba aberta, o convite
   "Escolha uma tabela na árvore." como hoje.
5. **Edição por aba** (FEAT-007): pendência, caixa de Commit e travas valem só para a aba; trocar de aba,
   abrir outra tabela ou trocar de banco na faixa não mexe nela. Fechar o banco ou desconectar com pendência
   pede confirmação. Sai a trava de "trocar de tabela" na árvore e a da paleta ⌘K.
6. **Engine**: `Sessions::close_database(id, database)` fecha a conexão daquele banco e solta os caches;
   `Disconnected` continua fechando tudo. Evento `DatabaseClosed { connection, database }`.
7. **Testes**: kittest abre dois bancos pelo ⌘K (faixa com dois itens, árvore do segundo), volta ao
   primeiro pela faixa, abre duas tabelas (duas abas, clique na segunda foca), edita numa aba, troca de aba e
   de banco e volta (pendência intacta), fecha aba com pendência (confirmação), fecha banco pela faixa;
   engine: `close_database` fecha e `DatabaseClosed` sai; evento de dados vai à aba certa.

## Won't

- Várias janelas nativas (uma por conexão): fica para depois, sem desfazer isto.
- Arrastar abas para reordenar; abas de SQL (M5); fechar "todas as outras".
- Mesma tabela em duas abas (uma aba por tabela, como o TablePlus faz por padrão).
- Persistir bancos e abas abertos entre execuções (M8, junto com o histórico).

## Restrições

- `crates/engine/src/session.rs` é sensível: reviewer antes do commit do engine. A UI não é, mas o diff é
  grande: reviewer também.
- Commits: `docs`; `feat(engine)` (`close_database`); `feat(ui)` em dois passos (workspace por banco + faixa;
  abas por tabela), cada um com a suíte verde.
