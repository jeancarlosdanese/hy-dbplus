# @file: docs/features/FEAT-011-queries-salvas/50-dev/impl-notes.md

# Notas de Implementação — FEAT-011 queries-salvas

## Estado atual

_Atualizado em 22/10/2026._

- **Feito:** documentação (`5125df8`); `engine::queries` (`QueryStore`; 7 testes, revisado); `ui-egui`
  (`Deps.queries`, numeração por banco, restaurar no `DatabaseSelected`, salvar ao perder o foco, ao fechar
  aba/banco/conexão, no `forget_database` e no `on_exit`, apagar no "Descartar", `remove_connection` ao
  remover a conexão; kittest `queries_salvas_voltam_ao_abrir_o_banco`); `main.rs` com o `QueryStore`.
  Tudo na árvore, portão `--full` verde.
- **Commitado:** `2ff102d` engine, `62b62ae` UI (+ `main.rs`, BRIEF, docs). Na árvore: `fix(ui)` do último
  achado baixo (erro ao apagar o arquivo de aba vazia fechada ia embora com a aba), aguardando "pode commitar".
- **Falta:** push e verificação do Jean (aceite "Do Jean").
- **Depende de:** FEAT-010 (abas SQL), commitada (`7e68ca4`).
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
- **[2026-10-22] Fechar o banco ou desconectar com texto no editor não pergunta mais.** O texto é salvo
  antes de fechar, então não é trabalho perdido; `Workspace::would_lose_work` só olha abas de tabela. Fechar
  a aba em si continua pedindo confirmação, e "Descartar e fechar" apaga o arquivo (a aba fechada não volta).
  Substitui a decisão "1. Alinhar" da FEAT-010, que era anterior a este cache.
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

### 22/10/2026 — `engine::queries`

- [x] `QueryStore` no molde do `ConnectionStore`: `store.rs` passou a expor (na crate) `config_dir_from`,
  `create_private_dir`, `write_private` e `remove_if_exists`, para os dois usarem o mesmo diretório e a
  mesma escrita atômica com permissões. `SavedQuery { id, sql }`; `list` ignora o que não é `<n>.sql`
  (`7.SQL`, `abc.sql`, `notas.txt`); `save` de texto só de espaços apaga.
- [x] `safe_segment`: `%XX` por byte UTF-8 fora de `[A-Za-z0-9_-]`, aplicado ao id da conexão e ao nome do
  banco. Teste prova que `../fora/da pasta\x` fica dentro do diretório.
- [x] Testes: ordem e substituição, vazio apaga, remove e remove_connection idempotentes, nomes perigosos,
  permissões Unix (`0700`/`0600`), erro de leitura vira `Io` com caminho, `default_dir` ao lado do
  `connections.toml`.

### 22/10/2026 — Revisão do `QueryStore` (`process:reviewer`): aplicada

- [x] **Média.** Nome vazio virava componente vazio: `remove_connection("")` apagaria o diretório inteiro
  e `save("c1", "", …)` gravaria dentro do diretório da conexão. `safe_segment("")` devolve `%`, que nenhum
  outro nome produz (`%` real vira `%25`). Teste `nome_vazio_nao_sai_do_lugar`.
- [x] **Baixa.** `007.sql`/`+7.sql` editados à mão viravam um segundo `7`: `query_id` só aceita a forma
  canônica (`id.to_string() == stem`). Teste.
- [x] **Baixa.** "Nada de segredo" dizia menos do que parecia: doc do módulo, brief e índice agora dizem
  que o arquivo é o texto do usuário como está (inclusive senha dentro de um `CREATE ROLE`), `0600`.
- Anotado, sem conserto: sistema de arquivos sem distinção de maiúsculas junta `Loja` e `loja` (o volume
  do Jean distingue); dois saves simultâneos da mesma aba por dois processos disputam o `.tmp` (duas
  instâncias do app não são suportadas, como no `ConnectionStore`); `rename` no Windows substitui mas não
  é atômico (fora do MVP).

### 22/10/2026 — `ui-egui` e `main.rs`

- [x] `Deps.queries`; `QueryTab.saved` (grava só se o buffer mudou) e `QueryTab::restored`;
  `Workspace.next_query_id` no lugar do contador global do `App`.
- [x] Restaurar no `DatabaseSelected` ao criar o workspace: abas na ordem, `next_query_id = maior n`, a
  primeira ativa; erro do `list` vai ao erro da conexão.
- [x] Salvar: `editor.rs` ao `lost_focus`; `close_database`, `request_disconnect`, `forget_database`
  (fechamento pelo engine, conexão caída inclusive) e `on_exit`. Falha vira aviso na aba.
- [x] Apagar: "Descartar e fechar" de aba, banco ou conexão (`discard_*` apaga o arquivo e esvazia o buffer,
  para o fechamento não salvar de novo); ⌘W/× de aba vazia apaga o arquivo que houvesse; remover a conexão
  chama `remove_connection`.
- [x] Fechar banco/desconectar com texto não pergunta (decisão acima); a caixa da faixa fala só de pendência.
- [x] Kittest: digitar + Esc → arquivo; × do banco sem pergunta e arquivo fica; reabrir → `SQL Query 1` com o
  texto, sem resultados; ⌘T → 2; ⌘W vazia sem arquivo; Descartar apaga; numeração segue (3). Provado
  falhando sem o salvar e sem o restaurar.

### 22/10/2026 — Revisão da FEAT inteira (`process:reviewer`): aplicada

- [x] **Média.** Trocar de aba ou de banco não salvava: o `lost_focus` só é lido no frame em que o `TextEdit`
  daquela aba é desenhado, e a troca desenha o da aba nova. Agora `App::ui` faz um varrimento por frame
  (`autosave_unfocused_queries`): toda aba SQL com `buffer != saved` cujo editor não tem o foco é salva
  (custo: comparar o texto; escrita só quando mudou). O hook do editor saiu. Kittest: digitar e ⌘T salva a
  aba 1; digitar e Esc salva a 2.
- [x] **Média.** Remover a conexão apagava o diretório e o `Disconnected` seguinte podia recriá-lo pelo
  `forget_database` (lixo órfão): só salva se a conexão ainda está na lista. Teste
  `desconectar_salva_as_queries_so_de_conexao_que_ainda_existe` (os dois lados).
- [x] **Média.** Falha ao salvar no fechamento era engolida (a aba com o aviso sumia). `save_query_tab`
  devolve `Result`; × do banco e Desconectar não seguem para o engine se algo falhou (erro no cabeçalho da
  conexão, a aba fica com o aviso); fechamento pelo engine põe o erro em `conn_state.error` e `App.notice`;
  `on_exit` escreve no stderr.
- [x] **Baixa.** `forget_connection` usa `workspace_keys_of`; TL;DR e Objetivo do brief dizem que fechar a
  aba pergunta (e Descartar apaga), em vez de "salvo ao fechar aba".

### 22/10/2026 — Última revisão (`process:reviewer`, sobre os consertos): aplicada

- [x] **Média (produto).** "Descartar e fechar" do banco ou da conexão só aparece por pendência em aba de
  tabela, mas apagava todas as queries já salvas daquele banco. Agora só o descarte da própria aba SQL apaga
  o arquivo; banco e conexão fecham salvando, como sem pendência. Teste.
- [x] **Baixa.** Falha persistente ao salvar refazia a escrita a cada repaint e sobrescrevia qualquer outro
  aviso da aba: `QueryTab.save_failed` guarda o texto que falhou e o varrimento só tenta de novo quando ele
  muda. Teste.
- [x] **Baixa.** `discard_query_tab` só limpa `saved` quando o `remove` deu certo (senão a aba voltaria com
  o texto descartado sem ninguém notar).
- [x] **Baixa.** Ids egui da toolbar, do limite, do status, do scroll e dos resultados levam a chave do banco
  (com a numeração por banco, `SQL Query 1` de dois bancos compartilhava estado). Comentário órfão removido.

### 22/10/2026 — Passada de portão da UI: aplicada depois do commit

- [x] **Baixa.** `request_close_tab` de aba SQL vazia descartava o `Err` do `save` (que apaga o arquivo); com
  falha de I/O a aba fechava, o aviso dela sumia e o arquivo voltava na próxima abertura. O erro vai para a
  conexão e para o aviso do app; a aba fecha mesmo assim. Teste.
