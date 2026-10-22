# @file: docs/features/FEAT-003-driver-postgres-e-conexoes/50-dev/impl-notes.md

# Notas de Implementação — FEAT-003 driver-postgres-e-conexoes

## Estado atual

_Atualizado em 21/10/2026._

- **Feito:** documentação; conferência das dependências candidatas num projeto de teste (cargo-deny verde com o
  `deny.toml` do projeto, `ring` e não `aws-lc-rs` no grafo, sem openssl).
- **Em curso:** nada.
- **Falta:** `driver`; `engine`; `ui-egui` e app; verificação do Jean.
- **Depende de:** FEAT-002 concluída (M0); PostgreSQL local `pg-localhost` para os testes com variável.
- **Para retomar:** esta pasta, o `CLAUDE.md` da raiz e o BRIEF §3.1, §3.2, §5 e §6.

## Decisões

- **[2026-10-21]** TLS por `rustls` 0.23 com `default-features = false` e `ring`: o provedor padrão
  (`aws-lc-rs`) traz `aws-lc-sys` com licença OpenSSL/ISC fora do `deny.toml`. Raízes de confiança do sistema
  por `rustls-native-certs` (Keychain no macOS), não `webpki-roots` (CDLA-Permissive, fora da lista).
- **[2026-10-21]** `tokio-postgres-rustls` 0.14 (MIT) faz a ponte `tokio-postgres` ↔ `rustls`; feature
  `native-certs`, sem `webpki-roots`.
- **[2026-10-21]** `keyring` 4 com as features padrão (`v1`: Keychain no macOS, Credential Manager no Windows,
  Secret Service por zbus no Linux). Serviço `hy-dbplus`, conta = id da conexão: UUID v4 pela crate `uuid`
  (MIT OR Apache-2.0; `getrandom` já está no grafo via `ring`). Renomear a conexão não troca o id, então a
  senha segue a conexão.
- **[2026-10-21]** `tokio` sem features padrão: `rt-multi-thread`, `sync`, `time`, `net`. O runtime nasce no
  app e o `engine` recebe um `tokio::runtime::Handle`; a UI nunca chama `block_on`.
- **[2026-10-21]** Eventos do `engine` para a UI por `std::sync::mpsc` (a UI lê sem bloquear a cada frame e
  pede repaint por `egui::Context::request_repaint` passado ao engine como callback), não por canal `tokio`:
  a UI não é assíncrona.
- **[2026-10-21]** `Secret(String)` com `Debug` fixo `Secret(***)` e sem `Display`; limpeza de memória
  (zeroize) fica para depois, se virar requisito.
- **[2026-10-21]** Caminho da configuração: `$XDG_CONFIG_HOME/hy-dbplus/` ou `~/.config/hy-dbplus/` em todo
  sistema, como o BRIEF §6, via `std::env::var_os` e `std::env::home_dir` (sem crate `dirs`). Revisar no M8
  se o empacotamento `.app` pedir `~/Library/Application Support`.
- **[2026-10-21]** Dependências novas (versão mínima; licença): `tokio` 1 (MIT), `tokio-postgres` 0.7
  (MIT OR Apache-2.0), `tokio-postgres-rustls` 0.14 (MIT), `rustls` 0.23 (Apache-2.0 OR ISC OR MIT),
  `rustls-native-certs` 0.8 (Apache-2.0 OR ISC OR MIT), `keyring` 4 (MIT OR Apache-2.0), `thiserror` 2
  (MIT OR Apache-2.0), `serde` 1 (MIT OR Apache-2.0), `toml` 1 (MIT OR Apache-2.0), `uuid` 1 com `v4` (MIT OR Apache-2.0).

## Log

### 21/10/2026 — Criação

- [x] Leitura do BRIEF §3.1, §3.2, §4, §5 e §6; versões e licenças no crates.io (`cargo info`); projeto de
  teste no scratchpad com as dependências candidatas: `cargo fetch`, `cargo tree` (ring, sem aws-lc, sem
  openssl) e `cargo-deny check licenses bans sources` verde com o `deny.toml` do projeto.
- [x] Brief, aceite, notas e índice escritos antes do código.
