# @file: docs/features/FEAT-003-driver-postgres-e-conexoes/00-brief/brief.md

# FEAT-003 — Driver PostgreSQL e conexões (M1)

## Problema

O app abre uma janela vazia. Para ser um cliente de banco ele precisa guardar conexões, pedir a senha uma vez
e guardá-la no Keychain, conectar ao PostgreSQL por TLS sem bloquear a interface e mostrar o que há do outro
lado. Tudo isso é a superfície mais sensível do produto (credencial, rede, execução remota) e precisa nascer com
a forma certa: `driver` sem UI, `engine` como fachada, segredos fora de arquivo, erro como `Result`.

## Objetivo

Entregar o M1 do [BRIEF §7](../../../BRIEF.md): criar a conexão `pg-localhost` na lista, senha no Keychain,
conectar e ver os bancos do servidor. Telas 3.1 (lista de conexões) e 3.2 (conexão aberta, sem banco).

## Escopo (Must)

1. **`driver`** (camada 0): `ConnectionConfig` (host, porta, usuário, banco inicial, modo TLS), trait
   `Driver` assíncrona (`connect`, `ping`, `list_databases`, mais o `query` com stream e cancelamento no M3),
   enum `DriverError` (`thiserror`), implementação `postgres` com `tokio-postgres` sobre `rustls` com `ring` e
   os certificados do sistema. A senha chega por parâmetro (`Secret`, sem `Debug` legível) e nunca é guardada.
2. **`engine`** (camada 3): `ConnectionStore` com o TOML em `~/.config/hy-dbplus/connections.toml` (host,
   porta, usuário, banco, nome, tag de ambiente, id) e a senha no Keychain por `keyring` (serviço `hy-dbplus`,
   conta = id da conexão), atrás da trait `SecretStore` (implementação em memória para teste). `Session` que
   conecta num `tokio` cujo handle o app entrega, e devolve `Event`s por canal (`Connecting`, `Connected {
   versão, bancos }`, `Failed { erro }`) para a UI ler a cada frame.
3. **`ui-egui`** (camada 4): tela 3.1 com lista (tag de ambiente colorida, nome, `host : database`), busca,
   `+` e formulário (nome, tag, host, porta, usuário, banco, senha, modo TLS) para criar, editar e remover;
   botão conectar; tela 3.2 com a barra superior `AMBIENTE | PostgreSQL <versão> : conexão` e, até o ⌘K do M2,
   a lista de bancos do servidor no painel esquerdo. Erro de conexão aparece na tela, sem a senha.
4. **`apps/hy-dbplus`**: cria o runtime `tokio` e o entrega ao `engine`; `app id` corrigido para
   `br.com.hy.dbplus` (BRIEF decisão 2).
5. **Testes**: `driver` contra o PostgreSQL local atrás da variável `HY_DBPLUS_TEST_PG` (URL sem senha no
   repositório; fora da suíte padrão); `engine` com diretório temporário e `SecretStore` em memória (nunca o
   Keychain real em teste); parse e escrita do TOML; eventos da `Session` com um `Driver` falso.

## Won't

- Catálogo, árvore de objetos, seletor ⌘K de banco e schema: M2.
- Execução de query com stream e cancelamento: a assinatura entra na trait, a implementação é do M3 e M5.
- Túnel SSH, outros bancos, importação do `~/.pgpass` ou de conexões do TablePlus.
- CLI `connections list`: M8 (a `automation` continua vazia).
- Testes de UI com `egui_kittest`: M2, junto com a árvore, quando a tela tiver mais de um widget.

## Restrições

- Dependências novas, cada uma uma decisão registrada no impl-notes: `tokio`, `tokio-postgres`,
  `tokio-postgres-rustls`, `rustls` (sem features padrão: `ring`, não `aws-lc-rs`, cuja licença não está no
  `deny.toml`), `rustls-native-certs`, `keyring`, `thiserror`, `serde`, `toml`, `uuid`. Sem `webpki-roots`
  (licença CDLA).
- `crates/driver/src/`, credenciais e sessão no `engine`, `apps/*/src/`, `Cargo.toml` e `Cargo.lock` são
  sensíveis: `process:reviewer` antes de cada commit.
- Senha nunca em log, erro, evento, TOML ou mensagem na tela; `Secret` sem `Display` e com `Debug` oculto.
- Commits pequenos, nesta ordem: docs; `driver` (tipos, trait, erro, postgres, teste com variável); `engine`
  (store, secret store, session, eventos); `ui-egui` e app (lista, formulário, conectar, bancos).
