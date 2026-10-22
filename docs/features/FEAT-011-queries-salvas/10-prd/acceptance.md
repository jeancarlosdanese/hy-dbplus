# @file: docs/features/FEAT-011-queries-salvas/10-prd/acceptance.md

# FEAT-011 — Critérios de aceite

## `engine::queries`

- [ ] `save` grava `<dir>/queries/<conexão>/<banco>/<n>.sql` com `0600` (diretórios `0700`) e escrita atômica;
      texto vazio apaga; `list` devolve em ordem de `n`; `remove` e `remove_connection` apagam.
- [ ] Nome de banco com `/`, `..` ou espaço vira caminho seguro dentro do diretório.
- [ ] Erro de leitura ou escrita vira `EngineError::Io` com o caminho, sem segredo.

## `ui-egui`

- [ ] Ao abrir um banco com queries salvas, as abas SQL voltam com o texto, na ordem e com o número original;
      sem aba de tabela, a primeira fica ativa.
- [ ] Digitar e tirar o foco do editor salva; o aviso da aba mostra falha ao salvar.
- [ ] Fechar aba, banco ou conexão salva o texto antes; "Descartar e fechar" apaga o arquivo; texto vazio não
      deixa arquivo.
- [ ] Remover a conexão da lista apaga o diretório dela.
- [ ] Teste `egui_kittest` cobre salvar ao perder o foco, restaurar ao reabrir o banco e apagar ao descartar.

## Portão e CI

- [ ] `scripts/quality-gate.sh --full` verde; workflow verde.

## Do Jean

- [ ] Escrever uma query, fechar o banco, abrir de novo e vê-la de volta; fechar o app e abrir de novo idem;
      descartar uma aba e conferir que o arquivo sumiu em `~/.config/hy-dbplus/queries/`.
