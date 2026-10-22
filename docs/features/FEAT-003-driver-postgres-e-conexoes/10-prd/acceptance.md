# @file: docs/features/FEAT-003-driver-postgres-e-conexoes/10-prd/acceptance.md

# FEAT-003 — Critérios de aceite

## `driver`

- [ ] `ConnectionConfig` e `TlsMode` (`Disable`, `Prefer`, `Require`) serializáveis; `Secret` sem `Display`,
      com `Debug` que imprime `Secret(***)` (teste).
- [ ] `Driver::connect` falha com `DriverError` legível (host inválido, senha errada, TLS exigido sem suporte),
      sem a senha no texto do erro (teste com senha errada contra o PostgreSQL local, atrás de `HY_DBPLUS_TEST_PG`).
- [ ] `list_databases` devolve os bancos do servidor (sem templates) e `server_version` a versão (teste com a
      variável).
- [ ] Nenhuma crate abaixo da `ui-egui` ganhou dependência de UI (`cargo xtask layers`).

## `engine`

- [ ] `ConnectionStore::load` lê `connections.toml` (ausente = lista vazia); `save` escreve atômico (arquivo
      temporário + rename) e sem senha (teste procura `senha` e o valor no arquivo).
- [ ] Senha gravada e lida pelo `SecretStore` por id da conexão; remover a conexão remove a senha (teste com a
      implementação em memória).
- [ ] `Session::connect` emite `Connecting` e depois `Connected { server_version, databases }` ou `Failed`,
      sem bloquear o chamador (teste com `Driver` falso e runtime `tokio` de teste).

## `ui-egui` e app

- [ ] Lista de conexões vazia mostra o convite para criar a primeira; `+` abre o formulário; salvar grava o
      TOML e a senha no Keychain; a conexão aparece com tag colorida, nome e `host : database`.
- [ ] Editar e remover funcionam; remover pede confirmação.
- [ ] Conectar mostra "conectando", depois a barra `LOCAL | PostgreSQL 15.x : pg-localhost` e a lista de bancos;
      senha errada mostra o erro na tela e volta para a lista.
- [ ] A interface não congela durante a conexão (host inexistente com timeout de 10 s).
- [ ] `app id` é `br.com.hy.dbplus`.

## Portão e CI

- [ ] `scripts/quality-gate.sh --full` verde; cargo-deny verde com as dependências novas; workflow verde.
- [ ] Testes que dependem do PostgreSQL local ficam atrás de `HY_DBPLUS_TEST_PG` e são pulados sem ela.

## Do Jean

- [ ] Criar a conexão `pg-localhost` no app, digitar a senha uma vez, conectar e ver os bancos.
- [ ] Conferir no Acesso às Chaves (Keychain) que o item `hy-dbplus` existe e que o TOML não tem a senha.
- [ ] Fechar o app, abrir de novo: a conexão continua na lista e conecta sem pedir a senha.
