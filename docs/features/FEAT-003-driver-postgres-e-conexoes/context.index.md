<!-- FEAT_EM_ANDAMENTO — docs e driver em 21/10/2026; próximo: engine (store, keyring, session) -->
<!-- PRIO_ALTA -->
# @file: docs/features/FEAT-003-driver-postgres-e-conexoes/context.index.md

# Contexto — FEAT-003 driver-postgres-e-conexoes

> **Para Claude Code:** ponto de entrada da feature. Ler inteiro antes de agir.

## Identificação

| Campo | Valor |
|-------|-------|
| **ID** | FEAT-003 |
| **Slug** | driver-postgres-e-conexoes |
| **Origem** | BRIEF §7, marco M1 |
| **Status** | marcador da linha 1 deste arquivo |
| **Criado em** | 2026-10-21 |
| **Atualizado em** | 2026-10-21 |

## TL;DR

O M1 do BRIEF: o `driver` PostgreSQL (tokio-postgres sobre rustls com ring), o `ConnectionStore` do `engine`
(TOML sem senha + Keychain por `keyring`), a `Session` assíncrona com eventos para a UI, e as telas 3.1 (lista
de conexões, formulário) e 3.2 (conexão aberta, versão do servidor, bancos). Aceite: criar `pg-localhost`,
senha no Keychain, conectar e listar bancos.

## Leia nesta ordem

```
1. 00-brief/brief.md          → problema, objetivo, escopo, o que fica de fora
2. 10-prd/acceptance.md       → o que precisa ser verdade, e o que é do Jean
3. 50-dev/impl-notes.md       → estado atual, decisões (TLS, keyring, runtime, eventos) e log
4. docs/BRIEF.md §3.1, §3.2   → as telas; §5 e §6 → arquitetura e stack
5. CLAUDE.md (raiz)           → regras de engenharia (segredos, never panic, async fora da UI)
```

## Onde está o quê (quando existir)

```
crates/driver/src/config.rs       ConnectionConfig, TlsMode, Secret
crates/driver/src/error.rs        DriverError
crates/driver/src/lib.rs          traits Driver (connect) e Connection (ping, server_version, list_databases; query no M3)
crates/driver/src/postgres.rs     implementação tokio-postgres + rustls
crates/engine/src/store.rs        ConnectionStore (TOML) e Connection (id, nome, tag, host, porta, usuário, banco)
crates/engine/src/secrets.rs      trait SecretStore; keyring e memória
crates/engine/src/session.rs      Session, Event, canal para a UI
crates/ui-egui/src/connections.rs lista, busca, formulário, conectar
crates/ui-egui/src/workspace.rs   barra superior do ambiente e lista de bancos (tela 3.2)
apps/hy-dbplus/src/main.rs        runtime tokio entregue ao engine
```

## Workflow Claude Code

```
1. Ler este arquivo, o brief, o acceptance e o "Estado atual" do impl-notes.
2. driver, credenciais/sessão do engine, apps/*/src, Cargo.toml e Cargo.lock são sensíveis: reviewer antes de cada commit.
3. Senha nunca em log, erro, evento, arquivo ou tela. Teste nunca usa o Keychain real.
4. Teste que precisa do PostgreSQL local fica atrás de HY_DBPLUS_TEST_PG; a suíte padrão não depende dele.
5. Commit só com o "pode commitar" do Jean, na ordem: docs; driver; engine; ui e app.
6. Registrar no impl-notes.md (log + Estado atual) a cada passo.
```
