# @file: docs/features/FEAT-011-queries-salvas/00-brief/brief.md

# FEAT-011 — Queries salvas: cada aba SQL num arquivo, de volta ao abrir o banco

## Problema

As abas SQL (FEAT-010) vivem só em memória: fechar o banco, desconectar ou sair do app perde o texto
(a confirmação só avisa). O TablePlus guarda as abas de SQL por conexão e as traz de volta. O Jean pediu um
cache das queries (22/10).

## Objetivo

Cada aba SQL de um banco vira um arquivo de texto, salvo sempre que o texto mudou e o editor não tem o
foco (Esc, clique fora, troca de aba ou de banco) e ao fechar banco, conexão ou o app, restaurado quando o
banco é aberto de novo. Só o texto: resultados ficam em memória.
"Descartar e fechar" apaga o arquivo, para o cache não acumular lixo. O store não acrescenta nada ao
texto (nem host, nem a senha da conexão); o que o usuário digitar fica no arquivo como está, inclusive
uma senha dentro de um `CREATE ROLE`, por isso o arquivo é `0600`.

## Escopo (Must)

1. **`engine::queries`** (`QueryStore`, no molde do `ConnectionStore`): diretório
   `~/.config/hy-dbplus/queries/<id da conexão>/<banco>/` (o id é o UUID da conexão, estável quando o nome
   muda), um arquivo `<n>.sql` por aba (o `n` é o número da aba, `SQL Query n`); diretório `0700`, arquivo
   `0600`, escrita atômica (tmp + rename, como o TOML de conexões). API: `list(conn, db) -> Vec<SavedQuery {
   id, sql }>` em ordem de `n`; `save(conn, db, id, sql)` (texto vazio apaga o arquivo); `remove(conn, db,
   id)`; `remove_connection(conn)` quando a conexão é removida da lista. Erro vira `EngineError::Io` com o
   caminho. Nome de banco entra no caminho só com caracteres seguros: o resto é escapado em hexadecimal
   (`%xx`), para `..`, `/` e espaço não saírem do diretório.
2. **Restaurar** (`ui-egui`): ao abrir o banco (`DatabaseSelected` que cria o workspace), as queries salvas
   viram abas SQL na ordem, com o texto e sem resultados; a primeira delas fica ativa quando não há aba de
   tabela. A numeração das abas passa a ser por banco (`Workspace.next_query_id = maior n salvo`), para
   `SQL Query n` e o arquivo `n.sql` casarem; os eventos do engine já carregam conexão e banco, então o id
   só precisa ser único por banco.
3. **Salvar** (`ui-egui`): quando o `TextEdit` da aba perde o foco e o texto mudou desde o último salvamento
   (`QueryTab.saved: String`); ao fechar a aba (×/⌘W sem confirmação, texto vazio → arquivo apagado); ao
   fechar o banco pela faixa e ao desconectar (todas as abas SQL do banco, antes de sumir); ao sair do app
   (`eframe::App::on_exit`); "Descartar e fechar" de uma aba SQL apaga o arquivo dela. O "Descartar" do
   banco ou da conexão só existe por pendência em aba de tabela e não toca nas queries salvas (revisão).
   Falha ao salvar vai ao aviso da aba ("não foi possível salvar: <erro>") e não trava nada.
4. **Lista de conexões**: remover a conexão apaga o diretório dela (`remove_connection`), com a confirmação
   que já existe para remover.
5. **Testes**: `QueryStore` com diretório temporário (salvar, listar em ordem, texto vazio apaga, remover,
   remover conexão, nome de banco com `/` e `..` escapado, permissões no Unix, erro de diretório ilegível
   vira `Io` com caminho); UI com `egui_kittest` e `QueryStore` num diretório temporário: digitar, tirar o
   foco → arquivo com o texto; fechar o banco e abrir de novo → a aba volta com o texto e o mesmo número;
   "Descartar e fechar" → arquivo some; nova aba depois da restaurada ganha o próximo número.

## Won't

- Histórico de execuções persistido e a tela History (M7): o histórico em memória da FEAT-010 continua.
- Sincronizar entre máquinas, versionar o texto, salvar resultados, renomear a aba (o nome é `SQL Query n`).
- Salvar a cada tecla (perder o foco basta; sair do app salva tudo).

## Restrições

- `apps/hy-dbplus/src/main.rs` (monta o `Deps` com o `QueryStore`) é sensível: reviewer. `engine/queries.rs` não
  casa com os padrões sensíveis, mas escreve arquivo em `~/.config`: reviewer também. UI pelo tamanho.
- Commits: `docs`; `feat(engine)` (`QueryStore`); `feat(ui)` (restaurar, salvar, apagar) com o `main.rs`.
