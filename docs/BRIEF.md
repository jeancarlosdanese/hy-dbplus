# hy-dbplus — Brief do produto

> Status: aprovado · Revisão 3 · 2026-10-21

## 1. Objetivo

Um cliente de banco de dados desktop, leve e simples, no espírito do TablePlus,
escrito inteiramente em Rust. O uso principal é **acessar dados** (navegar
tabelas e linhas) e **escrever SQL nativo com autocomplete**.

Princípios:

- **Leve.** Abre em menos de um segundo, binário pequeno, consumo de memória
  baixo mesmo com várias conexões abertas.
- **Simples.** Poucas telas, poucos cliques. Tudo que o TablePlus faz bem na
  navegação diária está aqui; o que é raro fica fora.
- **Rust puro.** Sem WebView, sem JavaScript. Interface em `egui`/`eframe`,
  roda em macOS, Linux e Windows com o mesmo código, seguindo o modelo de [PdfCraft](https://github.com/storytold/pdfcraft) e
  [VectorCraft](https://github.com/storytold/vectorcraft).
- **Local e privado.** Sem conta, sem telemetria, sem nuvem. Credenciais no
  Keychain do sistema.
- **Feito para agentes também.** O engine é uma API headless usada pela
  interface, por uma CLI e por um servidor MCP opcional.

## 2. Inspiração

| Projeto | O que copiamos |
|---|---|
| TablePlus | Fluxo de uso, layout de três painéis, abas Data/Structure/Index, editor SQL com "Run Current" |
| PdfCraft / VectorCraft | Workspace Cargo em camadas, engine independente da UI, `egui`, regra "never panic", CLI + MCP, docs para agentes (aqui só `CLAUDE.md`, dono único das instruções) |

## 3. Referência de telas (TablePlus)

Capturas feitas em 2026-10-21 no TablePlus 27.0.2, conexão `pg-localhost`,
num banco de desenvolvimento local. Servem como especificação visual e funcional.

### 3.1 Lista de conexões
- Lista vertical com ícone do tipo de banco (Pg, Ms, Re, Sl, Mg), nome,
  **tag de ambiente** (local, dev, prod) e subtítulo `host : database`.
- Busca por nome, botão `+` para nova conexão.
- Ações laterais: Create Connection, Backup, Restore (os dois últimos fora do MVP).

### 3.2 Conexão aberta, sem banco selecionado
- Barra superior colorida pelo ambiente com `AMBIENTE | PostgreSQL 15.15 : conexão`.
- Painel esquerdo com abas **Items / Queries / History** e busca.
- Mensagem "No database selected, press ⌘K to select a database".
- Painel direito "No row selected".

### 3.3 Banco selecionado
- Barra superior passa a mostrar `: database`.
- Árvore com grupos **Functions** e **Tables**; views com ícone distinto.
- Seletor de **schema** no rodapé do painel esquerdo (`public`).
- Cada tabela expande até suas **colunas com tipo**; PK com ícone de chave.

### 3.4 Aba Data
- Grid com cabeçalho por coluna, número da linha, valores monoespaçados.
- Colunas com **FK** mostram seta para navegar ao registro referenciado.
- Rodapé: abas **Data / Structure / Index**, `+ Row`, contador
  `1-24 of 24 rows`, **Filters**, **Columns**, paginação.
- Tamanhos (`total_size`, `data_size`, `index_size`), contagem e `comment` da tabela numa linha acima do
  grid (FEAT-006; o painel não os repete).
- Painel direito, recolhido por padrão: abre ao selecionar a primeira linha e mostra os campos dela em
  **Form** (nome, tipo, valor) ou **JSON** com botão Copiar; seleção múltipla mostra "Múltiplos valores"
  nos campos que diferem (FEAT-006).
- Edição (FEAT-007, só tabela com chave primária): duplo clique na célula ou campo do Form deixa a mudança
  pendente; ⌫ marca a seleção para apagar; **Commit** (⌘S) mostra o SQL exato e roda tudo numa transação;
  **Descartar** (⌘Z) limpa. Com pendência, ordenar, paginar e recarregar ficam travados.
- Vários bancos e abas (FEAT-008): faixa à esquerda com os bancos abertos (conexão + banco, cor do
  ambiente; ⌘K abre mais um; × fecha); cada tabela aberta é uma aba do banco, com página, seleção e
  pendência próprias (× e ⌘W fecham; fechar aba, banco ou conexão com pendência pede confirmação).

### 3.5 Aba Structure
- Somente leitura no MVP (FEAT-009): colunas `#`, nome (🔑 na chave), tipo, nulo, default (identidade e
  gerada), comentário; restrições (chave primária, únicas, checks, chaves estrangeiras apontando para a
  tabela referenciada); view mostra a definição.
- Rodapé: `+ Column` e **Triggers** desabilitados; **DDL** abre o `CREATE TABLE` montado pelo catálogo
  (particionada e estrangeira com aviso; sem `PARTITION BY`, collation e sequências).
- Editar **Name**, **Primary**, tipo e chave fica para depois do MVP.

### 3.6 Aba Index
- Somente leitura (FEAT-009): `index_name`, `index_algorithm`, `is_unique`, `column_name`,
  `condition`, `include`, `storage_parameters`, `comment`.

### 3.7 Editor SQL
- Aba própria ao lado das abas de tabela (`<tabela> | SQL Query N`; **+ SQL** ou ⌘T), uma por banco,
  quantas quiser (FEAT-010).
- Syntax highlight, numeração de linhas, várias queries no mesmo buffer; o statement sob o cursor fica
  destacado.
- Barra de status `linha, coluna`.
- Controles: limite de linhas (**No limit**, 100, 500, 1000; por leitura, sem envolver o SQL em
  `LIMIT`), **Beautify ⌘I**, **Run Current ⌘↩** (a query sob o cursor), **Run All ⌘⇧↩** (para no
  primeiro erro), **Cancelar**; aviso "transação aberta" depois de um `BEGIN` sem fim.
- Resultado em sub-abas `Query N` abaixo do editor: grid só leitura, `OK, N linhas afetadas` ou o erro;
  ⌘-clique numa linha mostra o JSON no painel direito.
- **Autocomplete** de palavras-chave, tabelas, colunas e aliases (M6, FEAT-012); histórico persistido (M7).
- **Queries salvas** (FEAT-011): cada aba SQL num arquivo em `~/.config/hy-dbplus/queries/<conexão>/<banco>/<n>.sql`, salvo ao perder o foco e ao fechar (fechar o banco ou desconectar com texto não pergunta: salva), restaurado ao abrir o banco com o mesmo número; "Descartar e fechar" de uma aba apaga.

## 4. Escopo do MVP (v0.1)

Somente **PostgreSQL** e somente **leitura**, exceto o editor SQL, que executa
qualquer statement.

| # | Funcionalidade | Tela ref. |
|---|---|---|
| 1 | Lista de conexões com tag de ambiente, criar/editar/remover, senha no Keychain | 3.1 |
| 2 | Conectar, listar bancos, selecionar banco (⌘K) e schema | 3.2, 3.3 |
| 3 | Árvore de Tables, Views e Functions, expansível até colunas com tipo e PK | 3.3 |
| 4 | Aba Data: grid virtualizado, paginação, ordenação por coluna, filtros simples, contador de linhas | 3.4 |
| 5 | Painel lateral com tamanhos, comentário e campos da linha selecionada | 3.4 |
| 6 | Aba Structure (somente leitura) com colunas, tipos, nulabilidade, default, check e DDL | 3.5 |
| 7 | Aba Index (somente leitura) | 3.6 |
| 8 | Editor SQL em abas: highlight, Run Current, Run All, limite de linhas, Beautify, resultado em grid, histórico | 3.7 |
| 9 | Autocomplete por schema: keywords, tabelas, colunas, aliases do statement atual | 3.7 |
| 10 | Navegação por FK no grid (clique na seta abre o registro referenciado) | 3.4 |
| 11 | Tema claro e escuro | todas |
| 12 | CLI headless: `connections list`, `query`, `tables`; servidor MCP opt-in | — |

### Fora do MVP (próximas versões)
- `+ Row` (inserção) e commit/rollback manual. Update e Delete entraram pela decisão 6.
- Edição de estrutura: `+ Column`, renomear, Primary, Triggers.
- MySQL, SQLite, Redis, MongoDB.
- Backup e restore.
- Importação e exportação (CSV, JSON, SQL dump).
- Túnel SSH.
- Build para web (WASM): descartado (decisão 4).

## 5. Arquitetura

Workspace Cargo em camadas. **Nada abaixo da UI depende de `egui`.**

```
hy-dbplus/
├── Cargo.toml                 # workspace
├── crates/
│   ├── driver/                # hy-dbplus-driver: conexões e execução de SQL
│   ├── catalog/               # hy-dbplus-catalog: schemas, tabelas, colunas, índices, FKs
│   ├── sql/                   # hy-dbplus-sql: split de statements, beautify, autocomplete
│   ├── engine/                # hy-dbplus-engine: fachada; sessões, abas, histórico, credenciais
│   ├── automation/            # hy-dbplus-automation: tabela de tools headless + MCP
│   └── ui-egui/               # hy-dbplus-ui-egui: a interface
├── apps/
│   ├── hy-dbplus/             # app desktop (eframe)
│   └── hy-dbplus-cli/         # CLI + `mcp`
├── xtask/                     # tarefas de build e checks (layers, fmt, clippy)
└── docs/
```

| Crate | Responsabilidade | Dependências principais |
|---|---|---|
| `driver` | Trait `Driver` (conectar) e `Connection` (ping, versão, listar bancos; query em texto e lote numa transação; stream de linhas e cancelamento no M5). Implementação `postgres` no MVP. | `tokio`, `tokio-postgres`, `rustls` |
| `catalog` | Modelo neutro (`Schema`, `Table`, `Column`, `Index`, `ForeignKey`, `Function`) e carregadores por driver a partir de `pg_catalog`/`information_schema`. O cache do catálogo (por conexão e banco, com invalidação manual) mora no `engine`. | `driver` |
| `sql` | Divisão do buffer em statements com posição; statement sob o cursor; formatação; motor de autocomplete (contexto do cursor × catálogo). | `sqlparser`, `sqlformat`, `catalog` |
| `engine` | API usada por todos os frontends: `Session`, `Workspace` (abas), `ConnectionStore` (arquivo TOML + Keychain), histórico de queries, eventos assíncronos para a UI. | `driver`, `catalog`, `sql`, `keyring`, `serde`, `toml` |
| `automation` | Lista de tools (`connections.list`, `query.run`, `catalog.tables`, …) com schema JSON, usada pela CLI e exposta como servidor MCP opt-in. | `engine`, `serde_json` |
| `ui-egui` | Widgets: lista de conexões, árvore, grid, abas, editor SQL com autocomplete, painel lateral, temas. Testável com `egui_kittest`. | `egui`, `eframe`, `egui_extras`, `engine` |

Regras de engenharia (never panic, layering, async fora da UI, segredos, Rust only, dependências) e o
portão de qualidade têm um dono só: o [CLAUDE.md](../CLAUDE.md) da raiz. Este brief não as repete.

## 6. Stack

| Área | Escolha | Observação |
|---|---|---|
| Linguagem | Rust stable (1.99 instalado) | edição 2024 |
| UI | `egui` 0.36 + `eframe` + `egui_extras` | mesma versão dos projetos de inspiração |
| Async | `tokio` | runtime único, criado pelo app |
| PostgreSQL | `tokio-postgres` + `rustls` | sem `libpq`, binário estático |
| Parser SQL | `sqlparser` | dialeto PostgreSQL; usado para split e autocomplete |
| Formatação SQL | `sqlformat` | Beautify |
| Credenciais | `keyring` | Keychain no macOS, Credential Manager no Windows, Secret Service no Linux |
| Config | `serde` + `toml` | `~/.config/hy-dbplus/connections.toml` |
| Testes de UI | `egui_kittest` | screenshots e interação headless |
| Banco de teste | PostgreSQL local já existente (`pg-localhost`) | sem Docker obrigatório |

## 7. Marcos

| Marco | Entrega | Critério de aceite |
|---|---|---|
| **M0** | Workspace com lints de never-panic, `xtask layers`, app abre uma janela vazia com tema | `cargo run -p hy-dbplus` abre; `scripts/quality-gate.sh --full` verde |
| **M1** | `driver` PostgreSQL + lista de conexões + conectar | Cria conexão `pg-localhost`, senha no Keychain, conecta e lista bancos |
| **M2** | `catalog` + árvore de objetos + seletor de banco e schema | Abre um banco local, vê tabelas, views, functions e colunas |
| **M3** | Aba Data: grid virtualizado, paginação, ordenação, painel lateral | Abre uma tabela, vê as linhas, os tamanhos e o comentário |
| **M4** | Abas Structure e Index somente leitura, DDL | Estrutura e índices de uma tabela batem com o TablePlus |
| **M5** | Editor SQL: abas, highlight, Run Current, limite, Beautify, resultado | Executa as queries da tela 3.7 e vê o resultado |
| **M6** | Autocomplete por schema | Digita `select * from` e o prefixo de uma tabela e recebe o nome; `alias.` lista as colunas |
| **M7** | Filtros, navegação por FK, histórico de queries, busca na árvore | Fluxo diário completo sem sair do app |
| **M8** | CLI + MCP opt-in, empacotamento macOS (`.app`), README | Um agente lista tabelas e roda uma query via MCP |

Cada marco é um ou mais commits pequenos. Documentação de cada feature em
`docs/features/FEAT-NNN-slug/` (brief, aceite, notas de implementação e `context.index.md`),
como a FEAT-001.

## 8. Riscos e mitigações

| Risco | Impacto | Mitigação |
|---|---|---|
| Editor SQL com autocomplete em `egui` não existe pronto | Alto | Construir sobre `TextEdit` com layouter customizado para highlight e popup próprio. Começar simples (keywords + tabelas), evoluir. Prototipar em M5/M6 cedo. |
| Grid com milhares de linhas e colunas largas | Médio | `egui_extras::TableBuilder` virtualizado; paginação server-side (`LIMIT/OFFSET` ou keyset); truncar células longas. |
| Query longa travando a UI | Médio | Execução em `tokio`, cancelamento via `pg_cancel_backend`/`CancelToken`, UI por eventos. |
| Usuário não conhece Rust | Médio | Projeto desenvolvido por agentes com `CLAUDE.md` detalhado; usuário valida comportamento, não código. Commits pequenos. |
| Visual não nativo do macOS | Baixo | Aceito. Tema inspirado no TablePlus, atalhos de teclado iguais (⌘K, ⌘↩, ⌘I). |

## 9. Decisões tomadas

| # | Decisão | Escolha | Data |
|---|---|---|---|
| 1 | Licença | **MIT** | 2026-10-21 |
| 2 | Nome do binário / bundle id | `hy-dbplus` / `br.com.hy.dbplus` | 2026-10-21 |
| 3 | Paginação do grid | **`LIMIT/OFFSET`** no MVP, como o TablePlus. Keyset fica como evolução futura; a interface do `driver` não muda. | 2026-10-21 |
| 4 | Web (WASM) | **Ignorar.** O navegador não abre conexão TCP com o banco; uma versão web exigiria um servidor intermediário, contra o princípio de ferramenta simples e local. | 2026-10-21 |
| 5 | Plataformas | **macOS** é o alvo principal. **Linux** e **Windows** são builds secundários na CI, mesmo código via `egui`. | 2026-10-21 |
| 6 | Edição de dados | **Update e Delete entram no MVP** (FEAT-007): só tabelas com chave primária; edições pendentes no grid, confirmadas em lote numa transação com o SQL à vista; Delete sobre a seleção, com confirmação. `+ Row` continua fora. | 2026-10-22 |

## 10. Próximos passos

1. ~~Aprovar este brief e responder as decisões em aberto.~~ Feito em 2026-10-21.
2. ~~Processo, portão de qualidade e `CLAUDE.md`.~~ FEAT-001, 2026-10-21.
3. ~~M0: FEAT-002.~~ Concluída em 2026-10-21.
4. M1: FEAT-003 (`docs/features/FEAT-003-driver-postgres-e-conexoes/`), iniciada em 2026-10-21.
