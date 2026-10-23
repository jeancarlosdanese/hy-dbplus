//! A árvore de objetos (tela 3.3): grupos **Functions** e **Tables**, views com ícone próprio,
//! tabela expansível até as colunas `nome  tipo`, chave primária com ícone, busca e o seletor de
//! schema no rodapé do painel.

use hy_dbplus_engine::{Driver, TableKind};

use crate::app::{App, DbKey};
use crate::theme;

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, key: &DbKey)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let Some(ws) = app.workspaces.get(key) else {
        return;
    };
    let (id, database) = key.clone();
    let schemas: Vec<String> = ws.schemas.iter().map(|s| s.name.clone()).collect();
    let current = ws.current_schema.clone();
    // Com edição pendente em alguma aba, recarregar esvaziaria as colunas (e a chave primária) dela:
    // o ↻ fica desabilitado com a dica.
    let locked = app.has_pending_changes(key);

    // Rodapé: seletor de schema.
    let mut chosen_schema: Option<String> = None;
    egui::Panel::bottom("tree-schema").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Schema")
                    .small()
                    .color(tokens.text_muted),
            );
            egui::ComboBox::from_id_salt("schema")
                .selected_text(current.clone().unwrap_or_else(|| "—".into()))
                .show_ui(ui, |ui| {
                    for name in &schemas {
                        if ui
                            .selectable_label(current.as_deref() == Some(name), name)
                            .clicked()
                        {
                            chosen_schema = Some(name.clone());
                        }
                    }
                });
            if ui
                .add_enabled(!locked, egui::Button::new("↻").small())
                .on_hover_text("Recarregar o catálogo")
                .on_disabled_hover_text(crate::data::LOCK_HINT)
                .clicked()
            {
                app.deps.sessions.refresh(&id, &database);
                if let Some(schema) = &current {
                    if let Some(ws) = app.workspaces.get_mut(key) {
                        ws.objects.remove(schema);
                        ws.columns.clear();
                        ws.expanded.clear(); // tabela aberta pede as colunas de novo
                        // A estrutura cacheada sai; a aba que está em Structure/Index pede de novo
                        // na hora, senão ficaria em branco até trocar de sub-aba.
                        let mut again = Vec::new();
                        for tab in ws.tabs.iter_mut().filter_map(crate::app::Tab::table_mut) {
                            tab.structure = None;
                            tab.structure_error = None;
                            // Pedido já em voo não se repete: a resposta dele chega depois do refresh.
                            let in_flight = tab.structure_requested;
                            tab.structure_requested = tab.view != crate::app::TabView::Data;
                            if tab.structure_requested && !in_flight {
                                again.push((tab.schema.clone(), tab.table.clone()));
                            }
                        }
                        ws.loading = true;
                        for (schema, table) in again {
                            app.deps
                                .sessions
                                .load_structure(&id, &database, &schema, &table);
                        }
                    }
                    app.deps.sessions.load_schema(&id, &database, schema);
                }
            }
        });
    });
    if let Some(schema) = chosen_schema {
        if let Some(ws) = app.workspaces.get_mut(key) {
            ws.current_schema = Some(schema.clone());
            ws.expanded.clear();
            ws.loading = !ws.objects.contains_key(&schema);
        }
        app.deps.sessions.load_schema(&id, &database, &schema);
    }

    // Busca e a árvore.
    egui::CentralPanel::default().show(ui, |ui| {
        let Some(ws) = app.workspaces.get_mut(key) else {
            return;
        };
        ui.add(
            egui::TextEdit::singleline(&mut ws.search)
                .hint_text("Buscar")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(4.0);
        if let Some(error) = ws.error.clone() {
            ui.colored_label(theme::error_color(), error);
            if ui.small_button("Tentar de novo").clicked()
                && let Some(schema) = ws.current_schema.clone()
            {
                ws.error = None;
                ws.expanded.clear();
                ws.loading = true;
                app.deps.sessions.load_schema(&id, &database, &schema);
            }
            return;
        }
        let Some(schema) = ws.current_schema.clone() else {
            ui.label(egui::RichText::new("Banco sem schema visível.").color(tokens.text_muted));
            return;
        };
        let Some((tables, functions)) = ws.objects.get(&schema).cloned() else {
            ui.label(
                egui::RichText::new(if ws.loading {
                    "Carregando…"
                } else {
                    "Sem dados."
                })
                .color(tokens.text_muted),
            );
            return;
        };
        let search = ws.search.trim().to_lowercase();
        let tables: Vec<_> = tables
            .into_iter()
            .filter(|t| search.is_empty() || t.name.to_lowercase().contains(&search))
            .collect();
        let functions: Vec<_> = functions
            .into_iter()
            .filter(|f| search.is_empty() || f.name.to_lowercase().contains(&search))
            .collect();
        let mut to_load: Vec<String> = Vec::new();
        let mut to_open: Option<String> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::CollapsingHeader::new(format!("Functions ({})", functions.len()))
                .default_open(false)
                .show(ui, |ui| {
                    for f in &functions {
                        ui.label(format!("ƒ {}", f.signature()));
                    }
                });
            egui::CollapsingHeader::new(format!("Tables ({})", tables.len()))
                .default_open(true)
                .show(ui, |ui| {
                    for t in &tables {
                        let icon = match t.kind {
                            TableKind::View | TableKind::MaterializedView => "◫",
                            TableKind::Foreign => "⇄",
                            TableKind::Table => "▦",
                        };
                        let key = (schema.clone(), t.name.clone());
                        // A seta expande as colunas (botão com rótulo, acessível); o nome abre a aba Data (tela 3.4).
                        let state_id = ui.make_persistent_id(&key);
                        let mut state =
                            egui::collapsing_header::CollapsingState::load_with_default_open(
                                ui.ctx(),
                                state_id,
                                false,
                            );
                        let was_open = state.is_open();
                        let header = ui
                            .horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(if was_open { "▾" } else { "▸" })
                                            .frame(false)
                                            .small(),
                                    )
                                    .clicked()
                                {
                                    state.toggle(ui);
                                }
                                let selected = ws
                                    .data()
                                    .is_some_and(|d| d.schema == schema && d.table == t.name);
                                // Abre numa aba nova ou foca a da tabela: nada trava a troca.
                                if ui
                                    .selectable_label(selected, format!("{icon} {}", t.name))
                                    .clicked()
                                {
                                    to_open = Some(t.name.clone());
                                }
                            })
                            .response;
                        state.show_body_indented(&header, ui, |ui| match ws.columns.get(&key) {
                            Some(columns) => {
                                for c in columns {
                                    let marker = if c.is_primary_key { "🔑" } else { "  " };
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{marker} {}", c.name));
                                        ui.label(
                                            egui::RichText::new(&c.data_type)
                                                .small()
                                                .color(tokens.text_muted),
                                        );
                                    });
                                }
                            }
                            None => {
                                ui.label(
                                    egui::RichText::new("carregando…")
                                        .small()
                                        .color(tokens.text_muted),
                                );
                            }
                        });
                        if was_open {
                            if ws.expanded.insert(key.clone()) {
                                to_load.push(t.name.clone());
                            }
                        } else {
                            // Fechada: reabrir pede de novo (o engine responde do cache sem query).
                            ws.expanded.remove(&key);
                        }
                    }
                });
        });
        for table in to_load {
            app.deps
                .sessions
                .load_columns(&id, &database, &schema, &table);
        }
        if let Some(table) = to_open {
            app.open_table(key, &schema, &table);
        }
    });
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use hy_dbplus_driver::{
        BatchStatement, Connection as DriverConnection, ConnectionConfig, DatabaseInfo, Driver,
        DriverError, Outcome, Secret, TextRows,
    };
    use hy_dbplus_engine::{Connection, Direction, Tag};

    use crate::app::{App, View, tests::deps};

    /// Driver falso com o catálogo de um banco "loja": schemas app e public, tabela pedido (PK id)
    /// e view v_pedido, função dobro.
    #[derive(Default)]
    struct FakeDriver {
        queries: Arc<AtomicUsize>,
        batches: Arc<Mutex<Vec<Vec<BatchStatement>>>>,
        /// Ligado, a query de cabeçalho da estrutura falha (FEAT-009, "Tentar de novo").
        fail_structure: Arc<std::sync::atomic::AtomicBool>,
    }
    struct FakeConn {
        queries: Arc<AtomicUsize>,
        batches: Arc<Mutex<Vec<Vec<BatchStatement>>>>,
        fail_structure: Arc<std::sync::atomic::AtomicBool>,
        cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    }

    fn row(cells: &[Option<&str>]) -> Vec<Option<String>> {
        cells.iter().map(|c| c.map(str::to_owned)).collect()
    }

    impl DriverConnection for FakeConn {
        async fn ping(&self) -> Result<(), DriverError> {
            Ok(())
        }
        async fn server_version(&self) -> Result<String, DriverError> {
            Ok("15.15".into())
        }
        async fn list_databases(&self) -> Result<Vec<DatabaseInfo>, DriverError> {
            Ok(vec![
                DatabaseInfo {
                    name: "postgres".into(),
                },
                DatabaseInfo {
                    name: "loja".into(),
                },
                DatabaseInfo {
                    name: "template0".into(),
                },
            ])
        }
        /// Grava o lote; um statement com `erro` no texto é o que o servidor recusaria.
        async fn execute_batch(
            &self,
            statements: &[BatchStatement],
        ) -> Result<Vec<u64>, DriverError> {
            if let Ok(mut batches) = self.batches.lock() {
                batches.push(statements.to_vec());
            }
            if let Some(i) = statements.iter().position(|s| s.sql.contains("erro")) {
                return Err(DriverError::Statement {
                    position: i + 1,
                    reason: "invalid input syntax".into(),
                });
            }
            Ok(statements
                .iter()
                .map(|s| s.expected_rows.unwrap_or(0))
                .collect())
        }
        /// Editor (FEAT-010): "select N" devolve N linhas (truncadas pelo limite), "update" 3
        /// afetadas, "erro" é erro do servidor, "sleep" espera até 2 s ou o cancel.
        async fn execute(&self, sql: &str, limit: Option<usize>) -> Result<Outcome, DriverError> {
            self.queries.fetch_add(1, Ordering::SeqCst);
            if sql.contains("erro") {
                return Err(DriverError::Db("syntax error at or near \"erro\"".into()));
            }
            if sql.contains("sleep") {
                self.cancel_flag.store(false, Ordering::SeqCst);
                for _ in 0..200 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    if self.cancel_flag.load(Ordering::SeqCst) {
                        return Err(DriverError::Cancelled(
                            "canceling statement due to user request".into(),
                        ));
                    }
                }
                return Ok(Outcome::default());
            }
            if sql.to_ascii_lowercase().starts_with("update") {
                return Ok(Outcome {
                    affected: Some(3),
                    ..Outcome::default()
                });
            }
            let n: usize = sql
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);
            let keep = limit.map_or(n, |l| l.min(n));
            Ok(Outcome {
                rows: Some(TextRows {
                    columns: vec!["n".into()],
                    rows: (1..=keep).map(|i| vec![Some(i.to_string())]).collect(),
                }),
                affected: None,
                truncated: keep < n,
            })
        }
        async fn cancel(&self) -> Result<(), DriverError> {
            self.cancel_flag.store(true, Ordering::SeqCst);
            Ok(())
        }
        async fn query_text(&self, sql: &str) -> Result<TextRows, DriverError> {
            self.queries.fetch_add(1, Ordering::SeqCst);
            let rows = if sql.starts_with("SELECT count(*)") {
                vec![row(&[Some("250")])]
            } else if sql.contains("pg_total_relation_size") {
                vec![row(&[
                    Some("24 kB"),
                    Some("8192 bytes"),
                    Some("16 kB"),
                    Some("pedidos"),
                ])]
            } else if sql.starts_with("SELECT * FROM") {
                let desc = sql.contains("DESC");
                let first = if desc { "b" } else { "a" };
                let second = if desc { "a" } else { "b" };
                return Ok(TextRows {
                    columns: vec!["id".into(), "nome".into(), "status".into()],
                    rows: vec![
                        row(&[Some("1"), Some(first), Some("ok")]),
                        row(&[Some("2"), Some(second), Some("ok")]),
                        row(&[Some("3"), None, Some("ok")]),
                    ],
                });
            } else if sql.contains("pg_get_viewdef") {
                // Cabeçalho da estrutura (FEAT-009): tabela com comentário; a view com definição.
                if self.fail_structure.load(Ordering::SeqCst) {
                    return Err(DriverError::Db("permission denied".into()));
                }
                if sql.contains("E'v_pedido'") {
                    vec![row(&[
                        Some("v"),
                        None,
                        Some(" SELECT id, nome FROM public.pedido;"),
                    ])]
                } else {
                    vec![row(&[Some("r"), Some("pedidos"), None])]
                }
            } else if sql.contains("pg_am ") {
                vec![
                    row(&[
                        Some("pedido_obs_idx"),
                        Some("btree"),
                        Some("f"),
                        Some("f"),
                        Some(
                            "CREATE INDEX pedido_obs_idx ON public.pedido USING btree (nome) INCLUDE (status) WHERE (nome IS NOT NULL)",
                        ),
                        Some("nome"),
                        Some("nome IS NOT NULL"),
                        Some("status"),
                        Some("fillfactor=70"),
                        Some("parcial"),
                        Some("f"),
                    ]),
                    row(&[
                        Some("pedido_pkey"),
                        Some("btree"),
                        Some("t"),
                        Some("t"),
                        Some("CREATE UNIQUE INDEX pedido_pkey ON public.pedido USING btree (id)"),
                        Some("id"),
                        None,
                        None,
                        None,
                        None,
                        Some("t"),
                    ]),
                ]
            } else if sql.contains("pg_get_constraintdef") {
                vec![
                    row(&[
                        Some("pedido_cliente_fkey"),
                        Some("f"),
                        Some("FOREIGN KEY (cliente) REFERENCES cliente(id) ON DELETE CASCADE"),
                        Some("cliente"),
                        Some("public"),
                        Some("cliente"),
                        Some("id"),
                        Some("a"),
                        Some("c"),
                    ]),
                    row(&[
                        Some("pedido_pkey"),
                        Some("p"),
                        Some("PRIMARY KEY (id)"),
                        Some("id"),
                        None,
                        None,
                        None,
                        None,
                        None,
                    ]),
                ]
            } else if sql.contains("pg_attribute") {
                // A view v_pedido tem as mesmas colunas, sem chave primária (só leitura).
                let pk = if sql.contains("E'v_pedido'") {
                    "f"
                } else {
                    "t"
                };
                vec![
                    row(&[
                        Some("id"),
                        Some("integer"),
                        Some("f"),
                        None,
                        Some("1"),
                        Some(pk),
                    ]),
                    row(&[
                        Some("cliente"),
                        Some("character varying(80)"),
                        Some("f"),
                        None,
                        Some("2"),
                        Some("f"),
                    ]),
                    row(&[
                        Some("status"),
                        Some("text"),
                        Some("f"),
                        None,
                        Some("3"),
                        Some("f"),
                    ]),
                ]
            } else if sql.contains("pg_proc") {
                vec![row(&[
                    Some("dobro"),
                    Some("x integer"),
                    Some("integer"),
                    Some("f"),
                ])]
            } else if sql.contains("pg_class") && sql.contains("E'quebrado'") {
                return Err(DriverError::Db(
                    "permission denied for schema quebrado".into(),
                ));
            } else if sql.contains("pg_class") {
                vec![
                    row(&[Some("pedido"), Some("r"), None]),
                    row(&[Some("v_pedido"), Some("v"), None]),
                ]
            } else if sql.contains("pg_namespace") {
                vec![
                    row(&[Some("app")]),
                    row(&[Some("public")]),
                    row(&[Some("quebrado")]),
                ]
            } else {
                return Err(DriverError::Db(format!("sql inesperado: {sql}")));
            };
            Ok(TextRows {
                columns: Vec::new(),
                rows,
            })
        }
    }

    impl Driver for FakeDriver {
        type Conn = FakeConn;
        async fn connect(
            &self,
            config: &ConnectionConfig,
            _password: &Secret,
        ) -> Result<FakeConn, DriverError> {
            tokio::time::sleep(Duration::from_millis(5)).await;
            if config.database == "template0" {
                return Err(DriverError::Db(
                    "database \"template0\" is not currently accepting connections".into(),
                ));
            }
            Ok(FakeConn {
                queries: Arc::clone(&self.queries),
                batches: Arc::clone(&self.batches),
                fail_structure: Arc::clone(&self.fail_structure),
                cancel_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            })
        }
    }

    /// Roda frames até a condição valer (os eventos chegam de outra thread).
    fn wait_for(
        harness: &mut Harness<'_, App<FakeDriver>>,
        what: &str,
        cond: impl Fn(&App<FakeDriver>) -> bool,
    ) {
        let started = Instant::now();
        while !cond(harness.state()) {
            assert!(
                started.elapsed() < Duration::from_secs(3),
                "esperando: {what}"
            );
            std::thread::sleep(Duration::from_millis(10));
            harness.step();
        }
        harness.step();
    }

    #[test]
    fn abrir_o_banco_pela_paleta_e_expandir_uma_tabela_ate_as_colunas() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let driver = FakeDriver::default();
        let queries = Arc::clone(&driver.queries);
        let mut app = App::with_deps(deps(driver, &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let id = c.id.clone();
        let mut harness = Harness::new_ui_state(|ui, app: &mut App<FakeDriver>| app.ui(ui), app);
        wait_for(&mut harness, "Connected", |app| app.view == View::Workspace);
        harness.get_by_label("Nenhum banco selecionado.");

        // ⌘K abre a paleta com os bancos; escolher "loja" abre o banco e carrega public.
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.objects.contains_key("public"))
        });
        harness.get_by_label_contains(": loja");
        harness.get_by_label("Tables (2)");
        harness.get_by_label("Functions (1)");
        harness.get_by_label("◫ v_pedido");
        assert_eq!(
            queries.load(Ordering::SeqCst),
            3,
            "schemas, tabelas e funções"
        );

        // Expandir a tabela pede as colunas uma vez; a chave primária tem o ícone.
        harness.get_all_by_label("▸").next().unwrap().click();
        wait_for(&mut harness, "ColumnsLoaded", |app| {
            app.workspace_of(&id).is_some_and(|ws| {
                ws.columns
                    .contains_key(&("public".to_owned(), "pedido".to_owned()))
            })
        });
        harness.get_by_label("🔑 id");
        harness.get_by_label("integer");
        harness.get_by_label("character varying(80)");
        assert_eq!(queries.load(Ordering::SeqCst), 4);
        harness.step();
        assert_eq!(
            queries.load(Ordering::SeqCst),
            4,
            "expandida continua sem nova query"
        );

        // Trocar o schema recarrega a árvore do outro schema.
        harness.get_by_role(egui::accesskit::Role::ComboBox).click();
        for _ in 0..5 {
            harness.step();
            if harness.query_by_label("app").is_some() {
                break;
            }
        }
        harness.get_by_label("app").click_accesskit();
        harness.step();
        assert_eq!(
            harness
                .state()
                .workspace_of(&id)
                .unwrap()
                .current_schema
                .as_deref(),
            Some("app"),
            "clique no schema"
        );
        wait_for(&mut harness, "schema app", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.objects.contains_key("app"))
        });
        assert_eq!(
            harness
                .state()
                .workspace_of(&id)
                .unwrap()
                .current_schema
                .as_deref(),
            Some("app")
        );
        assert_eq!(queries.load(Ordering::SeqCst), 6);
    }

    #[test]
    fn erro_do_catalogo_aparece_na_arvore_e_tentar_de_novo_refaz_a_query() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let driver = FakeDriver::default();
        let queries = Arc::clone(&driver.queries);
        let mut app = App::with_deps(deps(driver, &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let id = c.id.clone();
        let mut harness = Harness::new_ui_state(|ui, app: &mut App<FakeDriver>| app.ui(ui), app);
        wait_for(&mut harness, "Connected", |app| app.view == View::Workspace);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.objects.contains_key("public"))
        });

        // Schema que o servidor recusa: erro na árvore, sem derrubar a sessão nem o banco.
        harness.get_by_role(egui::accesskit::Role::ComboBox).click();
        for _ in 0..5 {
            harness.step();
            if harness.query_by_label("quebrado").is_some() {
                break;
            }
        }
        harness.get_by_label("quebrado").click_accesskit();
        wait_for(&mut harness, "CatalogFailed", |app| {
            app.workspace_of(&id).is_some_and(|ws| ws.error.is_some())
        });
        harness.get_by_label_contains("permission denied");
        assert!(harness.state().deps.sessions.is_open(&id));
        assert_eq!(
            harness.state().active.as_ref().map(|k| k.1.as_str()),
            Some("loja")
        );
        let before = queries.load(Ordering::SeqCst);

        // Tentar de novo refaz a query (e falha de novo, de propósito: o erro é do servidor).
        harness.get_by_label("Tentar de novo").click();
        harness.step();
        wait_for(&mut harness, "nova tentativa", |app| {
            app.workspace_of(&id).is_some_and(|ws| ws.error.is_some())
        });
        assert!(
            queries.load(Ordering::SeqCst) > before,
            "tentar de novo não consultou o servidor"
        );
    }

    #[test]
    fn falha_ao_abrir_o_primeiro_banco_aparece_no_painel() {
        // Antes: a paleta fechava, o painel seguia em "Nenhum banco selecionado." e nada avisava.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = App::with_deps(deps(FakeDriver::default(), &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let id = c.id.clone();
        let mut harness = Harness::new_ui_state(|ui, app: &mut App<FakeDriver>| app.ui(ui), app);
        wait_for(&mut harness, "Connected", |app| app.view == View::Workspace);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("template0").click();
        wait_for(&mut harness, "CatalogFailed", |app| {
            app.conn_state.get(&id).is_some_and(|s| s.error.is_some())
        });
        harness.get_by_label_contains("template0: ");
        harness.get_by_label_contains("not currently accepting");
        harness.get_by_label("Nenhum banco selecionado.");
        assert!(harness.state().deps.sessions.is_open(&id));
    }

    #[test]
    fn recarregar_pede_as_colunas_da_tabela_aberta_de_novo() {
        // Antes: ↻ esvaziava as colunas mas não a lista de expandidas, e a tabela ficava em "carregando…".
        let rt = tokio::runtime::Runtime::new().unwrap();
        let driver = FakeDriver::default();
        let queries = Arc::clone(&driver.queries);
        let mut app = App::with_deps(deps(driver, &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let id = c.id.clone();
        let mut harness = Harness::new_ui_state(|ui, app: &mut App<FakeDriver>| app.ui(ui), app);
        wait_for(&mut harness, "Connected", |app| app.view == View::Workspace);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.objects.contains_key("public"))
        });
        harness.get_all_by_label("▸").next().unwrap().click();
        wait_for(&mut harness, "ColumnsLoaded", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| !ws.columns.is_empty())
        });
        harness.get_by_label("🔑 id");
        let before = queries.load(Ordering::SeqCst);
        harness.get_by_label("↻").click();
        wait_for(&mut harness, "cache esvaziado", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.columns.is_empty())
        });
        wait_for(&mut harness, "colunas de volta", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| !ws.columns.is_empty())
        });
        harness.get_by_label("🔑 id");
        assert_eq!(
            queries.load(Ordering::SeqCst),
            before + 3,
            "tabelas, funções e colunas de novo"
        );
    }

    #[test]
    fn a_paleta_abre_com_o_foco_na_busca_e_filtra_ao_digitar() {
        // Antes: `focus_pending` nascia false e o campo nunca recebia o foco; digitar não filtrava.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = App::with_deps(deps(FakeDriver::default(), &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let mut harness = Harness::new_ui_state(|ui, app: &mut App<FakeDriver>| app.ui(ui), app);
        wait_for(&mut harness, "Connected", |app| app.view == View::Workspace);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("postgres");
        harness.get_by_label("template0");
        // Texto cru, como o teclado manda: só chega ao campo se ele estiver com o foco.
        harness
            .input_mut()
            .events
            .push(egui::Event::Text("lo".into()));
        harness.step();
        assert_eq!(
            harness.state().palette.as_ref().map(|p| p.query.as_str()),
            Some("lo")
        );
        harness.step(); // a lista filtra no frame seguinte ao da digitação
        harness.get_by_label("loja");
        assert!(
            harness.query_by_label("postgres").is_none(),
            "postgres deveria ter sido filtrado"
        );
        assert!(harness.query_by_label("template0").is_none());
        harness.key_press(egui::Key::Enter);
        wait_for(&mut harness, "DatabaseSelected", |app| {
            app.active.as_ref().is_some_and(|k| k.1 == "loja")
        });
    }

    /// Harness com janela larga: a faixa de bancos, a árvore, o grid e o painel direito não cabem nos
    /// 800 px padrão, e um clique numa célula escondida sob o painel não chega ao grid.
    fn wide_harness<'a>(app: App<FakeDriver>) -> Harness<'a, App<FakeDriver>> {
        Harness::builder()
            .with_size(egui::vec2(1400.0, 800.0))
            .build_ui_state(|ui, app: &mut App<FakeDriver>| app.ui(ui), app)
    }

    #[test]
    fn abrir_a_tabela_mostra_o_grid_ordena_pagina_e_seleciona() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let driver = FakeDriver::default();
        let queries = Arc::clone(&driver.queries);
        let mut app = App::with_deps(deps(driver, &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let id = c.id.clone();
        let mut harness = wide_harness(app);
        wait_for(&mut harness, "Connected", |app| app.view == View::Workspace);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.objects.contains_key("public"))
        });
        harness.get_by_label("Escolha uma tabela na árvore.");

        // O nome abre a aba Data: página, contagem e tamanhos.
        harness.get_by_label("▦ pedido").click();
        wait_for(&mut harness, "PageLoaded", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.data().is_some_and(|d| d.rows.is_some()))
        });
        wait_for(&mut harness, "StatsLoaded", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.data().is_some_and(|d| d.stats.is_some()))
        });
        harness.get_by_label("public.pedido");
        harness.get_by_label("1-3 of 250 rows");
        // Tamanhos e comentário numa linha acima do grid (o painel começa fechado).
        harness.get_by_label_contains("250 rows · total 24 kB");
        harness.get_by_label("pedidos");
        let names = |app: &App<FakeDriver>| -> Vec<String> {
            app.workspace_of(&id)
                .unwrap()
                .data()
                .unwrap()
                .rows
                .as_ref()
                .unwrap()
                .rows
                .iter()
                .map(|r| r[1].clone().unwrap_or_default())
                .collect()
        };
        assert_eq!(names(harness.state()), ["a", "b", ""]);

        // Clique no cabeçalho ordena: asc, depois desc.
        harness.get_by_label("nome").click();
        wait_for(&mut harness, "asc", |app| {
            app.workspace_of(&id).unwrap().data().is_some_and(|d| {
                !d.loading
                    && d.page
                        .order
                        .as_ref()
                        .is_some_and(|o| o.direction == Direction::Asc)
            })
        });
        harness.get_by_label("nome ▲").click();
        wait_for(&mut harness, "desc", |app| {
            app.workspace_of(&id).unwrap().data().is_some_and(|d| {
                !d.loading
                    && d.page
                        .order
                        .as_ref()
                        .is_some_and(|o| o.direction == Direction::Desc)
            })
        });
        assert_eq!(names(harness.state()), ["b", "a", ""]);

        // Próxima página: offset 100.
        // Botão no rodapé: a ação do AccessKit (o clique por ponteiro não alcança o rodapé no harness).
        harness.get_by_label("›").click_accesskit();
        wait_for(&mut harness, "página 2", |app| {
            app.workspace_of(&id)
                .unwrap()
                .data()
                .is_some_and(|d| !d.loading && d.page.offset == 100)
        });
        harness.get_by_label("101-103 of 250 rows");

        // Selecionar a primeira linha mostra os campos no painel direito.
        harness.get_by_label("101").click_accesskit();
        harness.step();
        harness.step(); // o painel direito é desenhado antes do grid: lê a seleção no frame seguinte
        let selected = |app: &App<FakeDriver>| {
            app.workspace_of(&id)
                .unwrap()
                .data()
                .unwrap()
                .selected_rows
                .iter()
                .copied()
                .collect::<Vec<_>>()
        };
        assert_eq!(selected(harness.state()), [0]);
        // O painel começa fechado e abre na primeira seleção, em Form, com nome, tipo e valor.
        harness.get_by_label("Linha 101");
        // O tipo aparece no cabeçalho do grid e no Form.
        assert!(
            harness.get_all_by_label("integer").count() >= 2,
            "tipo da coluna no cabeçalho e no Form"
        );
        harness.get_by_label("1 of 3 rows selected");
        // ⌘-clique na terceira linha: duas selecionadas; campo igual mostra o valor, diferente "Múltiplos valores".
        harness
            .input_mut()
            .events
            .push(egui::Event::ModifiersChanged(egui::Modifiers::COMMAND));
        harness.get_by_label("103").click_accesskit();
        harness.step();
        harness
            .input_mut()
            .events
            .push(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        harness.step();
        assert_eq!(selected(harness.state()), [0, 2]);
        harness.get_by_label("2 of 3 rows selected");
        harness.get_by_label("2 linhas");
        // A tabela tem chave primária: o Form é editável, um campo por coluna (FEAT-007); os campos
        // que diferem (id, nome) ficam vazios com a dica "Múltiplos valores", o igual (status) traz "ok".
        assert_eq!(
            harness
                .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
                .count(),
            3
        );
        assert_eq!(
            harness.get_all_by_label("ok").count(),
            3,
            "as três células do grid"
        );
        assert!(
            harness.get_all_by_value("ok").count() >= 1,
            "o campo status do Form"
        );
        // JSON: lista com os dois objetos, tipos pelo catálogo; Copiar não quebra.
        harness.get_by_label("JSON").click_accesskit();
        harness.step();
        harness.step();
        harness.get_by_label_contains("\"id\": 1");
        harness.get_by_label_contains("\"status\": \"ok\"");
        harness.get_by_label("Copiar").click_accesskit();
        harness.step();
        // O botão da barra esconde e mostra o painel.
        harness.get_by_label("Painel ▸").click_accesskit();
        harness.step();
        harness.step();
        assert!(
            harness.query_by_label("2 linhas").is_none(),
            "painel ainda visível"
        );
        harness.get_by_label("◂ Painel").click_accesskit();
        harness.step();
        harness.step();
        harness.get_by_label("2 linhas");
        // Fechado pelo botão, o painel não reabre num clique de linha: só a primeira seleção abre.
        // O clique é numa célula de valor (nome "a" da linha 102), não no número: o texto da célula não
        // pode engolir o clique da linha (o Jean só conseguia selecionar pelo número, 22/10/2026).
        harness.get_by_label("Painel ▸").click_accesskit();
        harness.step();
        harness.get_by_label("a").click();
        harness.step();
        harness.step();
        assert_eq!(selected(harness.state()), [1]);
        assert!(
            harness.query_by_label("Linha 102").is_none(),
            "o clique numa linha reabriu o painel fechado pelo botão"
        );
        // schemas, tabelas e funções (3), stats (tamanhos + count = 2), colunas antes da primeira página (1),
        // quatro páginas (1 cada): sem count a cada página.
        assert_eq!(queries.load(Ordering::SeqCst), 3 + 2 + 1 + 4);
    }

    /// Duplo clique num nó: os dois pares press/release no mesmo frame, na mesma posição (o egui
    /// exige menos de 6 px e 0,3 s entre os cliques; o primeiro clique abre o painel e mexeria no
    /// layout se os cliques fossem em frames separados).
    fn double_click(harness: &mut Harness<'_, App<FakeDriver>>, label: &str) {
        let pos = harness.get_by_label(label).rect().center();
        let events = &mut harness.input_mut().events;
        events.push(egui::Event::PointerMoved(pos));
        for _ in 0..2 {
            for pressed in [true, false] {
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                });
            }
        }
        harness.step();
        harness.step();
    }

    /// FEAT-007: duplo clique edita a célula, ⌫ marca a seleção, Commit mostra o SQL exato, Executar
    /// manda o lote e recarrega; erro do servidor fica na caixa com as pendências; Descartar limpa; o
    /// Form edita com Reverter; com pendência a ordenação trava; tabela sem chave primária não edita.
    #[test]
    fn editar_celula_apagar_commit_executar_e_descartar() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let driver = FakeDriver::default();
        let queries = Arc::clone(&driver.queries);
        let batches = Arc::clone(&driver.batches);
        let mut app = App::with_deps(deps(driver, &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let id = c.id.clone();
        let mut harness = wide_harness(app);
        wait_for(&mut harness, "Connected", |app| app.view == View::Workspace);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.objects.contains_key("public"))
        });
        // Abre a tabela pelo rótulo da árvore e espera a aba NOVA (não a anterior, que já estava
        // carregada) com página, stats e colunas (a chave primária vem delas).
        let open = |harness: &mut Harness<'_, App<FakeDriver>>, label: &str, table: &str| {
            harness.get_by_label(label).click();
            wait_for(harness, "tabela aberta e carregada", |app| {
                app.workspace_of(&id).is_some_and(|ws| {
                    ws.data().is_some_and(|d| {
                        d.table == table && d.rows.is_some() && d.stats.is_some() && !d.loading
                    }) && ws
                        .columns
                        .contains_key(&("public".to_owned(), table.to_owned()))
                })
            });
        };
        open(&mut harness, "▦ pedido", "pedido");
        let tab = |app: &App<FakeDriver>| app.workspace_of(&id).unwrap().data().cloned().unwrap();

        // Duplo clique na célula "a" (nome da linha 1) abre o campo; digitar e Enter deixa pendente.
        double_click(&mut harness, "a");
        assert!(
            tab(harness.state()).editing.is_some(),
            "duplo clique não abriu a edição"
        );
        harness
            .input_mut()
            .events
            .push(egui::Event::Text("novo".into()));
        harness.step();
        harness.key_press(egui::Key::Enter);
        harness.step();
        harness.step();
        let t = tab(harness.state());
        assert!(t.editing.is_none());
        let nome = t
            .pending
            .get(&0)
            .and_then(|m| m.get("nome"))
            .cloned()
            .flatten();
        assert!(
            nome.as_deref().is_some_and(|v| v.contains("novo")),
            "pendente: {nome:?}"
        );
        harness.get_by_label("1 alteração");
        // Com pendência, ordenar não faz nada e a dica aparece.
        harness.get_by_label("nome").click();
        harness.step();
        assert!(tab(harness.state()).page.order.is_none());
        harness.get_by_label(crate::data::LOCK_HINT);

        // Selecionar a linha 3 e ⌫: marcada para apagar.
        harness.get_all_by_label("3").next().unwrap().click();
        harness.step();
        harness.step();
        harness.key_press(egui::Key::Backspace);
        harness.step();
        harness.step();
        assert!(tab(harness.state()).deleted.contains(&2));
        harness.get_by_label("2 alterações");

        // Commit mostra o SQL exato; Executar manda o lote e recarrega.
        harness.get_by_label("Commit").click_accesskit();
        harness.step();
        harness.step();
        harness.get_by_label("1 updates, 1 deletes");
        harness.get_by_label_contains("UPDATE \"public\".\"pedido\" SET \"nome\" = E'");
        harness
            .get_by_label_contains("DELETE FROM \"public\".\"pedido\" WHERE (\"id\") IN ((E'3'));");
        let before = queries.load(Ordering::SeqCst);
        harness.get_by_label("Executar").click_accesskit();
        wait_for(&mut harness, "ChangesApplied", |app| {
            app.workspace_of(&id)
                .unwrap()
                .data()
                .is_some_and(|d| d.commit.is_none() && !d.has_changes() && !d.loading)
        });
        let sent = batches.lock().unwrap().clone();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].len(), 2);
        assert!(sent[0][0].sql.starts_with("UPDATE"));
        assert_eq!(sent[0][1].expected_rows, Some(1));
        assert!(
            queries.load(Ordering::SeqCst) >= before + 3,
            "página e stats recarregados"
        );
        assert!(harness.query_by_label("2 alterações").is_none());

        // Erro do servidor: a caixa mostra, nada foi gravado e a pendência continua; Cancelar fecha.
        double_click(&mut harness, "a");
        harness
            .input_mut()
            .events
            .push(egui::Event::Text("erro".into()));
        harness.step();
        harness.key_press(egui::Key::Enter);
        harness.step();
        harness.step();
        harness.get_by_label("1 alteração");
        // Fora da sub-aba Data, ⌘S não abre o Commit (nem é consumido); de volta em Data, abre.
        harness.get_by_label("Structure").click_accesskit();
        harness.step();
        harness.step();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
        harness.step();
        harness.step();
        assert!(harness.query_by_label("1 updates, 0 deletes").is_none());
        harness.get_by_label("Data").click_accesskit();
        harness.step();
        harness.step();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
        harness.step();
        harness.step();
        harness.get_by_label("1 updates, 0 deletes");
        harness.get_by_label("Executar").click_accesskit();
        wait_for(&mut harness, "ChangesFailed", |app| {
            app.workspace_of(&id)
                .unwrap()
                .data()
                .is_some_and(|d| d.commit.as_ref().is_some_and(|c| c.error.is_some()))
        });
        harness.get_by_label_contains("statement 1 do lote falhou");
        assert_eq!(tab(harness.state()).commit.unwrap().failed, Some(1));
        assert_eq!(batches.lock().unwrap().len(), 2);
        harness.get_by_label("Cancelar").click_accesskit();
        harness.step();
        harness.step();
        let t = tab(harness.state());
        assert!(t.commit.is_none() && t.has_changes());
        harness.get_by_label("1 alteração");

        // Descartar limpa sem ir ao banco; ⌘Z faz o mesmo.
        harness.get_by_label("Descartar").click_accesskit();
        harness.step();
        harness.step();
        assert!(!tab(harness.state()).has_changes());
        assert!(harness.query_by_label("1 alteração").is_none());
        assert_eq!(batches.lock().unwrap().len(), 2);
        double_click(&mut harness, "a");
        harness
            .input_mut()
            .events
            .push(egui::Event::Text("x".into()));
        harness.step();
        harness.key_press(egui::Key::Enter);
        harness.step();
        harness.step();
        harness.get_by_label("1 alteração");
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        harness.step();
        harness.step();
        assert!(!tab(harness.state()).has_changes(), "⌘Z não descartou");
        assert_eq!(batches.lock().unwrap().len(), 2);

        // Form: o campo "nome" da linha selecionada é editável; Reverter desfaz.
        harness.get_all_by_label("2").next().unwrap().click();
        harness.step();
        harness.step();
        harness.get_by_label("Linha 2");
        let field = harness
            .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
            .nth(1)
            .unwrap();
        field.focus();
        harness.step();
        harness
            .input_mut()
            .events
            .push(egui::Event::Text("z".into()));
        harness.step();
        harness.step();
        let t = tab(harness.state());
        assert!(t.pending.get(&1).is_some_and(|m| m.contains_key("nome")));
        harness.get_by_label("alterado");
        harness.get_by_label("Reverter").click_accesskit();
        harness.step();
        harness.step();
        assert!(!tab(harness.state()).has_changes());

        // Tabela sem chave primária: aviso, e o duplo clique não abre nada.
        open(&mut harness, "◫ v_pedido", "v_pedido");
        harness.get_by_label(crate::data::READ_ONLY_HINT);
        double_click(&mut harness, "a");
        assert!(tab(harness.state()).editing.is_none());
    }

    /// Um `App` com a tabela `public.pedidos` aberta (chave `id`), página de três linhas
    /// (`id`, `nome`), sem página em voo: o mínimo para exercitar a edição sem harness.
    fn app_with_editable_table() -> (App<FakeDriver>, crate::app::DbKey) {
        use crate::app::{DataTab, Workspace};
        use hy_dbplus_engine::{Column, Page};

        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = App::with_deps(deps(FakeDriver::default(), &rt));
        std::mem::forget(rt); // o teste é síncrono; o runtime só precisa existir
        let id: hy_dbplus_engine::ConnectionId = "c1".into();
        let key: crate::app::DbKey = (id.clone(), "loja".into());
        let mut tab = DataTab::new("public", "pedidos");
        tab.requested = None;
        tab.loading = false;
        tab.page = Page::default();
        tab.rows = Some(Arc::new(TextRows {
            columns: vec!["id".into(), "nome".into()],
            rows: vec![
                vec![Some("1".into()), Some("a".into())],
                vec![Some("2".into()), None],
                vec![Some("3".into()), Some("c".into())],
            ],
        }));
        let mut ws = Workspace {
            tabs: vec![crate::app::Tab::Table(Box::new(tab))],
            active_tab: Some(0),
            ..Default::default()
        };
        let column = |name: &str, pk: bool, position: i32| Column {
            name: name.into(),
            data_type: if pk { "integer" } else { "text" }.into(),
            nullable: !pk,
            default: None,
            position,
            is_primary_key: pk,
            ..Default::default()
        };
        ws.columns.insert(
            ("public".into(), "pedidos".into()),
            vec![column("id", true, 1), column("nome", false, 2)],
        );
        app.workspaces.insert(key.clone(), ws);
        app.rail.push(key.clone());
        app.active = Some(key.clone());
        app.focus = Some(id);
        (app, key)
    }

    fn tab_of(app: &App<FakeDriver>, key: &crate::app::DbKey) -> crate::app::DataTab {
        app.workspaces[key].data().cloned().unwrap()
    }

    /// Abrir uma célula NULL e sair sem digitar não vira pendência `''` (achado do revisor); digitar
    /// e apagar tudo sobre NULL volta a NULL; valor igual ao da página some da pendência.
    #[test]
    fn celula_null_aberta_e_fechada_sem_digitar_nao_vira_pendencia() {
        let (mut app, key) = app_with_editable_table();
        assert!(app.editable(&key));
        app.start_edit(&key, 1, 1);
        assert_eq!(tab_of(&app, &key).editing.unwrap().text, "");
        app.finish_edit(&key, true);
        assert!(
            !tab_of(&app, &key).has_changes(),
            "NULL aberto e fechado virou ''"
        );
        // Digitar sobre NULL vira texto; apagar tudo sobre o texto vira '' (string vazia; NULL é
        // só pelo botão do Form), e o NULL pelo Form devolve a célula ao valor da página.
        app.start_edit(&key, 1, 1);
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .data_mut()
            .unwrap()
            .editing
            .as_mut()
            .unwrap()
            .text = "x".into();
        app.finish_edit(&key, true);
        assert_eq!(tab_of(&app, &key).cell_value(1, 1), Some(Some("x")));
        app.start_edit(&key, 1, 1);
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .data_mut()
            .unwrap()
            .editing
            .as_mut()
            .unwrap()
            .text = String::new();
        app.finish_edit(&key, true);
        assert!(tab_of(&app, &key).cell_value(1, 1) == Some(Some("")));
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .data_mut()
            .unwrap()
            .set_cell(1, 1, None);
        assert!(!tab_of(&app, &key).has_changes());
        // Célula com texto: voltar ao valor da página também some da pendência; Esc descarta.
        app.start_edit(&key, 0, 1);
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .data_mut()
            .unwrap()
            .editing
            .as_mut()
            .unwrap()
            .text = "b".into();
        app.finish_edit(&key, true);
        assert!(tab_of(&app, &key).is_pending(0, "nome"));
        app.start_edit(&key, 0, 1);
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .data_mut()
            .unwrap()
            .editing
            .as_mut()
            .unwrap()
            .text = "a".into();
        app.finish_edit(&key, true);
        assert!(!tab_of(&app, &key).has_changes());
        app.start_edit(&key, 0, 1);
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .data_mut()
            .unwrap()
            .editing
            .as_mut()
            .unwrap()
            .text = "zzz".into();
        app.finish_edit(&key, false);
        assert!(!tab_of(&app, &key).has_changes());
    }

    /// Com página em voo não se edita (a pendência é por índice), e uma página nova descarta a
    /// pendência com aviso em vez de deixar os índices apontarem outras linhas (achado do revisor).
    #[test]
    fn pagina_em_voo_trava_a_edicao_e_pagina_nova_descarta_a_pendencia() {
        use hy_dbplus_engine::Page;

        let (mut app, key) = app_with_editable_table();
        {
            let tab = app.workspaces.get_mut(&key).unwrap().data_mut().unwrap();
            tab.set_cell(0, 1, Some("b".into()));
            tab.deleted.insert(2);
            assert!(tab.has_changes());
            tab.requested = Some(Page::default());
        }
        assert!(!app.editable(&key), "editável com página em voo");
        app.start_edit(&key, 0, 1);
        assert!(tab_of(&app, &key).editing.is_none());
        let tab = app.workspaces.get_mut(&key).unwrap().data_mut().unwrap();
        let dropped = tab.accept_page(
            Page::default(),
            TextRows {
                columns: vec!["id".into(), "nome".into()],
                rows: vec![vec![Some("9".into()), Some("z".into())]],
            },
        );
        assert!(dropped);
        assert!(!tab.has_changes() && tab.requested.is_none() && !tab.loading);
        assert_eq!(tab.error.as_deref(), Some(crate::app::DROPPED_HINT));
        // Sem pendência, a página nova chega limpa, sem aviso.
        assert!(!tab.accept_page(Page::default(), TextRows::default()));
        assert!(tab.error.is_none());
    }

    /// O campo do Form vale para as linhas selecionadas vivas: a marcada para apagar fica de fora,
    /// e o lote leva só as vivas como UPDATE e a marcada como DELETE.
    #[test]
    fn campo_do_form_edita_so_as_linhas_vivas_da_selecao() {
        let (mut app, key) = app_with_editable_table();
        {
            let tab = app.workspaces.get_mut(&key).unwrap().data_mut().unwrap();
            tab.selected_rows.extend([0, 1, 2]);
        }
        app.toggle_delete(&key); // marca as três
        app.toggle_delete(&key); // todas marcadas: desmarca
        assert!(tab_of(&app, &key).deleted.is_empty());
        {
            let tab = app.workspaces.get_mut(&key).unwrap().data_mut().unwrap();
            tab.selected_rows.clear();
            tab.selected_rows.insert(2);
        }
        app.toggle_delete(&key);
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .data_mut()
            .unwrap()
            .selected_rows
            .extend([0, 1]);
        app.set_field(&key, "nome", Some("z".into()));
        let tab = tab_of(&app, &key);
        assert!(tab.is_pending(0, "nome") && tab.is_pending(1, "nome"));
        assert!(!tab.is_pending(2, "nome"), "linha marcada recebeu edição");
        let changes = tab.change_set(&["id".to_owned()]).unwrap();
        assert_eq!(changes.updates.len(), 2);
        assert_eq!(changes.deletes, vec![vec!["3".to_owned()]]);
        app.set_field(&key, "nome", None);
        assert_eq!(tab_of(&app, &key).cell_value(0, 1), Some(None));
        app.revert_field(&key, "nome");
        assert!(tab_of(&app, &key).pending.is_empty());
    }

    /// FEAT-008 passo 1: ⌘K abre um segundo banco sem fechar o primeiro; a faixa mostra os dois e
    /// trocar nela traz a árvore e a aba (com a pendência) de cada um; × com pendência pede
    /// confirmação e fechar tira o banco da faixa.
    #[test]
    fn dois_bancos_na_faixa_trocam_arvore_e_aba_e_fecham_com_confirmacao() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let driver = FakeDriver::default();
        let mut app = App::with_deps(deps(driver, &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let id = c.id.clone();
        let mut harness = wide_harness(app);
        wait_for(&mut harness, "Connected", |app| app.view == View::Workspace);
        let open_db = |harness: &mut Harness<'_, App<FakeDriver>>, db: &str| {
            harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
            harness.step();
            // O nome do banco também está na faixa: o da paleta (janela, por cima) é o último nó.
            harness.get_all_by_label(db).last().unwrap().click();
            wait_for(harness, "DatabaseSelected", |app| {
                app.active.as_ref().is_some_and(|k| k.1 == db)
                    && app
                        .workspace_of(&id)
                        .is_some_and(|ws| ws.objects.contains_key("public"))
            });
        };
        open_db(&mut harness, "loja");
        harness.get_by_label("▦ pedido").click();
        wait_for(&mut harness, "tabela carregada", |app| {
            app.workspace_of(&id).is_some_and(|ws| {
                ws.data().is_some_and(|d| d.rows.is_some() && !d.loading)
                    && ws
                        .columns
                        .contains_key(&("public".to_owned(), "pedido".to_owned()))
            })
        });
        double_click(&mut harness, "a");
        harness
            .input_mut()
            .events
            .push(egui::Event::Text("x".into()));
        harness.step();
        harness.key_press(egui::Key::Enter);
        harness.step();
        harness.step();
        harness.get_by_label("1 alteração");

        // Segundo banco: entra na faixa e vira o ativo, com árvore própria e sem aba.
        open_db(&mut harness, "postgres");
        assert_eq!(
            harness.state().rail,
            vec![
                (id.clone(), "loja".to_owned()),
                (id.clone(), "postgres".to_owned())
            ]
        );
        harness.get_by_label("Escolha uma tabela na árvore.");
        assert!(harness.query_by_label("1 alteração").is_none());
        harness.get_by_label_contains(": postgres");

        // Volta ao primeiro pela faixa: aba e pendência intactas.
        harness.get_all_by_label("loja").next().unwrap().click();
        harness.step();
        harness.step();
        assert_eq!(
            harness.state().active.as_ref().map(|k| k.1.as_str()),
            Some("loja")
        );
        harness.get_by_label("1 alteração");
        harness.get_by_label("public.pedido");

        // ⌘K num banco já na faixa só o seleciona: nada se perde (a pendência da FEAT-007 fica).
        open_db(&mut harness, "postgres");
        open_db(&mut harness, "loja");
        assert_eq!(harness.state().rail.len(), 2);
        harness.get_by_label("1 alteração");
        // ⌘K num banco que não abre, com outro ativo: o erro aparece numa faixa, não some.
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("template0").click();
        wait_for(&mut harness, "CatalogFailed", |app| {
            app.conn_state.get(&id).is_some_and(|s| s.error.is_some())
        });
        harness.get_by_label_contains("template0: ");
        harness.get_by_label("OK").click();
        harness.step();
        harness.step();
        assert!(harness.query_by_label_contains("template0: ").is_none());
        assert_eq!(harness.state().rail.len(), 2);

        // × no banco com pendência, com clique de ponteiro (o × fica dentro do item clicável):
        // confirmação; Cancelar mantém; Descartar e fechar tira da faixa.
        let close_first = |harness: &mut Harness<'_, App<FakeDriver>>| {
            harness.get_all_by_label("×").next().unwrap().click();
            harness.step();
            harness.step();
        };
        close_first(&mut harness);
        harness.get_by_label("Fechar o banco?");
        harness.get_by_label("Cancelar").click_accesskit();
        harness.step();
        harness.step();
        assert!(harness.query_by_label("Fechar o banco?").is_none());
        assert_eq!(harness.state().rail.len(), 2);
        close_first(&mut harness);
        harness.get_by_label("Descartar e fechar").click_accesskit();
        wait_for(&mut harness, "DatabaseClosed", |app| app.rail.len() == 1);
        assert_eq!(
            harness.state().active.as_ref().map(|k| k.1.as_str()),
            Some("postgres")
        );
        assert!(harness.query_by_label("1 alteração").is_none());
        harness.get_by_label("Escolha uma tabela na árvore.");

        // Fechar o único banco: faixa vazia, conexão em foco, convite ao ⌘K.
        close_first(&mut harness);
        wait_for(&mut harness, "faixa vazia", |app| app.rail.is_empty());
        assert!(harness.state().active.is_none());
        assert_eq!(harness.state().view, View::Workspace);
        harness.get_by_label("Nenhum banco selecionado.");
    }

    /// FEAT-008 passo 2: cada tabela abre numa aba; clicar na tabela já aberta foca a aba; trocar de
    /// aba mantém a pendência de cada uma; ⌘W e × fecham (com confirmação se há pendência);
    /// desconectar com pendência também confirma.
    #[test]
    fn duas_abas_no_mesmo_banco_focam_fecham_e_confirmam() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = App::with_deps(deps(FakeDriver::default(), &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let id = c.id.clone();
        let mut harness = wide_harness(app);
        wait_for(&mut harness, "Connected", |app| app.view == View::Workspace);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.objects.contains_key("public"))
        });
        let open = |harness: &mut Harness<'_, App<FakeDriver>>, label: &str, table: &str| {
            harness.get_by_label(label).click();
            wait_for(harness, "aba carregada", |app| {
                app.workspace_of(&id).is_some_and(|ws| {
                    ws.data().is_some_and(|d| {
                        d.table == table && d.rows.is_some() && d.stats.is_some() && !d.loading
                    }) && ws
                        .columns
                        .contains_key(&("public".to_owned(), table.to_owned()))
                })
            });
        };
        let ws = |app: &App<FakeDriver>| -> (usize, Option<usize>) {
            let ws = app.workspace_of(&id).unwrap();
            (ws.tabs.len(), ws.active_tab)
        };
        open(&mut harness, "▦ pedido", "pedido");
        double_click(&mut harness, "a");
        harness
            .input_mut()
            .events
            .push(egui::Event::Text("x".into()));
        harness.step();
        harness.key_press(egui::Key::Enter);
        harness.step();
        harness.step();
        harness.get_by_label("1 alteração");

        // Segunda tabela: segunda aba, ativa, sem pendência; a primeira segue com a dela.
        open(&mut harness, "◫ v_pedido", "v_pedido");
        assert_eq!(ws(harness.state()), (2, Some(1)));
        assert!(harness.query_by_label("1 alteração").is_none());
        harness.get_by_label("● pedido").click_accesskit();
        harness.step();
        harness.step();
        assert_eq!(ws(harness.state()), (2, Some(0)));
        harness.get_by_label("1 alteração");
        // Clicar na árvore numa tabela já aberta só foca a aba dela.
        harness.get_by_label("◫ v_pedido").click();
        harness.step();
        harness.step();
        assert_eq!(ws(harness.state()), (2, Some(1)));

        // ⌘W na aba sem pendência fecha na hora; na aba com pendência pede confirmação.
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::W);
        harness.step();
        harness.step();
        assert_eq!(ws(harness.state()), (1, Some(0)));
        harness.get_by_label("1 alteração");
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::W);
        harness.step();
        harness.step();
        harness.get_by_label("Fechar a aba?");
        harness.get_by_label("Cancelar").click_accesskit();
        harness.step();
        harness.step();
        assert_eq!(ws(harness.state()), (1, Some(0)));
        // × da aba (o último ×: o primeiro é o do banco na faixa) com "Descartar e fechar".
        harness
            .get_all_by_label("×")
            .last()
            .unwrap()
            .click_accesskit();
        harness.step();
        harness.step();
        harness.get_by_label("Fechar a aba?");
        harness.get_by_label("Descartar e fechar").click_accesskit();
        harness.step();
        harness.step();
        assert_eq!(ws(harness.state()), (0, None));
        harness.get_by_label("Escolha uma tabela na árvore.");

        // Desconectar com pendência: confirmação; Cancelar mantém; Descartar e fechar desconecta.
        open(&mut harness, "▦ pedido", "pedido");
        double_click(&mut harness, "a");
        harness
            .input_mut()
            .events
            .push(egui::Event::Text("y".into()));
        harness.step();
        harness.key_press(egui::Key::Enter);
        harness.step();
        harness.step();
        harness.get_by_label("1 alteração");
        harness.get_by_label("Desconectar").click_accesskit();
        harness.step();
        harness.step();
        harness.get_by_label("Desconectar?");
        harness.get_by_label("Cancelar").click_accesskit();
        harness.step();
        harness.step();
        assert!(harness.state().deps.sessions.is_open(&id));
        harness.get_by_label("Desconectar").click_accesskit();
        harness.step();
        harness.step();
        harness.get_by_label("Descartar e fechar").click_accesskit();
        wait_for(&mut harness, "Disconnected", |app| {
            app.view == View::Connections
        });
        assert!(!harness.state().deps.sessions.is_open(&id));
        assert!(harness.state().rail.is_empty());
    }

    /// `close_tab` é pura: [A, B, C] com cada ativa e cada índice, inclusive fora do vetor.
    #[test]
    fn fechar_aba_escolhe_a_ativa_certa() {
        use crate::app::{DataTab, Workspace};
        let make = |active: Option<usize>| Workspace {
            tabs: vec![
                crate::app::Tab::Table(Box::new(DataTab::new("public", "a"))),
                crate::app::Tab::Table(Box::new(DataTab::new("public", "b"))),
                crate::app::Tab::Table(Box::new(DataTab::new("public", "c"))),
            ],
            active_tab: active,
            ..Default::default()
        };
        let names = |ws: &Workspace| {
            ws.tabs
                .iter()
                .filter_map(|t| t.table().map(|t| t.table.clone()))
                .collect::<Vec<_>>()
        };
        // Fecha a ativa do meio: a da direita assume.
        let mut ws = make(Some(1));
        ws.close_tab(1);
        assert_eq!(
            (names(&ws), ws.active_tab),
            (vec!["a".into(), "c".into()], Some(1))
        );
        // Fecha à esquerda da ativa: a ativa só muda de índice.
        let mut ws = make(Some(2));
        ws.close_tab(0);
        assert_eq!(
            (names(&ws), ws.active_tab),
            (vec!["b".into(), "c".into()], Some(1))
        );
        // Fecha à direita da ativa: nada muda para ela.
        let mut ws = make(Some(0));
        ws.close_tab(2);
        assert_eq!(
            (names(&ws), ws.active_tab),
            (vec!["a".into(), "b".into()], Some(0))
        );
        // Fecha a ativa que era a última: a da esquerda assume.
        let mut ws = make(Some(2));
        ws.close_tab(2);
        assert_eq!(ws.active_tab, Some(1));
        // Índice fora do vetor: nada; fechar tudo: nenhuma.
        let mut ws = make(Some(0));
        ws.close_tab(7);
        assert_eq!(ws.tabs.len(), 3);
        ws.close_tab(0);
        ws.close_tab(0);
        ws.close_tab(0);
        assert_eq!((ws.tabs.len(), ws.active_tab), (0, None));
    }

    /// Fechar com o Commit rodando não fecha nem pergunta (o lote ainda grava); célula aberta com
    /// texto conta como pendência; a confirmação some se o alvo sumir por evento.
    #[test]
    fn fechar_com_commit_rodando_nao_fecha_e_celula_aberta_pede_confirmacao() {
        use crate::app::{CloseTarget, CommitDialog};
        let (mut app, key) = app_with_editable_table();
        {
            let tab = app.workspaces.get_mut(&key).unwrap().data_mut().unwrap();
            tab.set_cell(0, 1, Some("b".into()));
            tab.commit = Some(CommitDialog {
                running: true,
                ..Default::default()
            });
        }
        app.request_close_tab(&key, 0);
        app.request_close_database(&key);
        app.request_disconnect(&key.0);
        assert!(app.confirm_close.is_none());
        assert_eq!(app.workspaces[&key].tabs.len(), 1);
        assert!(app.rail.contains(&key));
        // Commit terminou: fechar a aba com pendência pede confirmação.
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .data_mut()
            .unwrap()
            .commit = None;
        app.request_close_tab(&key, 0);
        assert_eq!(app.confirm_close, Some(CloseTarget::Tab(key.clone(), 0)));
        // O banco some por evento com a confirmação na tela: a confirmação vai junto.
        app.forget_database(&key);
        assert!(app.confirm_close.is_none());
        // Célula aberta com texto digitado, sem pendência gravada, também pede confirmação.
        let (mut app, key) = app_with_editable_table();
        app.start_edit(&key, 0, 1);
        app.request_close_tab(&key, 0);
        assert_eq!(app.confirm_close, Some(CloseTarget::Tab(key.clone(), 0)));
        app.confirm_close_now();
        assert!(app.workspaces[&key].tabs.is_empty());
    }

    /// Um app conectado ao driver falso com o banco "loja" aberto, sem harness: os eventos chegam
    /// por `drain_events`.
    fn connected_app() -> (App<FakeDriver>, tokio::runtime::Runtime, crate::app::DbKey) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = App::with_deps(deps(FakeDriver::default(), &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let key: crate::app::DbKey = (c.id.clone(), "loja".into());
        let started = Instant::now();
        let mut opened = false;
        while started.elapsed() < Duration::from_secs(3) {
            app.drain_events();
            if !opened && app.view == View::Workspace {
                app.open_database(&c.id, "loja");
                opened = true;
            }
            if app
                .workspaces
                .get(&key)
                .is_some_and(|ws| ws.objects.contains_key("public"))
            {
                return (app, rt, key);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("banco não abriu");
    }

    fn drain_until(app: &mut App<FakeDriver>, what: &str, cond: impl Fn(&App<FakeDriver>) -> bool) {
        let started = Instant::now();
        while !cond(app) {
            assert!(
                started.elapsed() < Duration::from_secs(3),
                "esperando: {what}"
            );
            std::thread::sleep(Duration::from_millis(10));
            app.drain_events();
        }
    }

    /// Eventos de dados vão à aba da tabela, ativa ou não: a segunda aba carrega em segundo plano e o
    /// `ChangesApplied` recarrega a aba do lote, não a ativa.
    #[test]
    fn eventos_de_dados_chegam_a_aba_de_fundo() {
        let (mut app, _rt, key) = connected_app();
        app.open_table(&key, "public", "pedido");
        app.open_table(&key, "public", "v_pedido");
        // A segunda aba acabou de pedir a página; a primeira volta a ser a ativa antes de ela chegar.
        app.workspaces.get_mut(&key).unwrap().active_tab = Some(0);
        drain_until(&mut app, "duas abas carregadas", |app| {
            app.workspaces[&key]
                .tabs
                .iter()
                .filter_map(|t| t.table())
                .all(|t| t.rows.is_some() && t.stats.is_some() && !t.loading)
        });
        let ws = &app.workspaces[&key];
        assert_eq!(ws.active_tab, Some(0));
        let second = ws.tabs[1].table().unwrap();
        assert_eq!(second.table, "v_pedido");
        assert!(second.rows.as_ref().is_some_and(|r| r.rows.len() == 3));

        // Lote na aba 1 (v_pedido não tem chave; o lote vai pela aba 0): edita em "pedido", abre o
        // Commit, executa, troca para a outra aba antes do resultado.
        drain_until(&mut app, "colunas", |app| {
            app.workspaces[&key]
                .columns
                .contains_key(&("public".to_owned(), "pedido".to_owned()))
        });
        app.workspaces.get_mut(&key).unwrap().active_tab = Some(0);
        assert!(app.editable(&key));
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .data_mut()
            .unwrap()
            .set_cell(0, 1, Some("z".into()));
        app.open_commit(&key);
        app.execute_changes(&key);
        assert!(app.workspaces[&key].tabs[0].busy());
        // Editor com Commit em curso no mesmo banco (revisão de 22/10/2026): recusa com aviso,
        // senão um Cancelar do editor derrubaria o lote no servidor.
        app.open_query_tab(&key);
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .query_mut()
            .unwrap()
            .buffer = "select 1".into();
        app.run_all(&key);
        let q = app.workspaces[&key].query().unwrap();
        assert!(!q.running);
        assert!(
            q.notice
                .as_deref()
                .is_some_and(|n| n.contains("Commit em curso")),
            "{:?}",
            q.notice
        );
        {
            let ws = app.workspaces.get_mut(&key).unwrap();
            ws.tabs.pop();
        }
        app.workspaces.get_mut(&key).unwrap().active_tab = Some(1);
        let other_requested_before = app.workspaces[&key].tabs[1]
            .table()
            .unwrap()
            .requested
            .clone();
        drain_until(&mut app, "ChangesApplied e recarga", |app| {
            let t = app.workspaces[&key].tabs[0].table().unwrap();
            t.commit.is_none() && !t.has_changes() && !t.loading && t.requested.is_none()
        });
        let ws = &app.workspaces[&key];
        assert_eq!(ws.active_tab, Some(1), "a troca de aba não foi desfeita");
        assert_eq!(
            ws.tabs[1].table().unwrap().requested,
            other_requested_before,
            "a aba ativa (outra) não foi recarregada no lugar da do lote"
        );

        // Espelho (revisão de 22/10/2026): Commit do grid com SQL em execução no banco é recusado
        // com erro na aba, senão um Cancelar do editor derrubaria o lote no servidor.
        app.open_query_tab(&key);
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .query_mut()
            .unwrap()
            .buffer = "select sleep".into();
        app.run_all(&key);
        drain_until(&mut app, "SQL rodando", |app| {
            app.workspaces[&key].tabs[2].query().unwrap().running
        });
        app.workspaces.get_mut(&key).unwrap().active_tab = Some(0);
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .data_mut()
            .unwrap()
            .set_cell(0, 1, Some("w".into()));
        app.open_commit(&key);
        let t = app.workspaces[&key].tabs[0].table().unwrap();
        assert!(
            t.commit.is_none(),
            "o Commit não pode abrir com SQL rodando"
        );
        assert!(
            t.error.as_deref().is_some_and(|e| e.contains("editor SQL")),
            "{:?}",
            t.error
        );
        app.workspaces.get_mut(&key).unwrap().active_tab = Some(2);
        app.cancel_sql(&key);
        drain_until(&mut app, "SQL cancelado", |app| {
            !app.workspaces[&key].tabs[2].query().unwrap().running
        });
    }

    /// FEAT-009: as sub-abas Structure e Index mostram colunas, restrições (FK apontando) e índices;
    /// o DDL abre numa caixa com Copiar; voltar a Data traz o grid.
    #[test]
    fn structure_index_e_ddl_de_uma_tabela() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let driver = FakeDriver::default();
        let queries = Arc::clone(&driver.queries);
        let fail_structure = Arc::clone(&driver.fail_structure);
        let mut app = App::with_deps(deps(driver, &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let id = c.id.clone();
        let mut harness = wide_harness(app);
        wait_for(&mut harness, "Connected", |app| app.view == View::Workspace);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.objects.contains_key("public"))
        });
        harness.get_by_label("▦ pedido").click();
        wait_for(&mut harness, "tabela carregada", |app| {
            app.workspace_of(&id).is_some_and(|ws| {
                ws.data()
                    .is_some_and(|d| d.rows.is_some() && d.stats.is_some() && !d.loading)
            })
        });
        let before = queries.load(Ordering::SeqCst);
        harness.get_by_label("Structure").click_accesskit();
        wait_for(&mut harness, "StructureLoaded", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.data().is_some_and(|d| d.structure.is_some()))
        });
        assert_eq!(
            queries.load(Ordering::SeqCst),
            before + 4,
            "cabeçalho, colunas, restrições, índices"
        );
        harness.get_by_label("🔑 id");
        harness.get_by_label("character varying(80)");
        harness.get_by_label("pedido_cliente_fkey");
        harness
            .get_by_label_contains("→ public.cliente(id) on update no action / on delete cascade");
        // DDL: caixa com o CREATE TABLE, Copiar e Fechar.
        harness.get_by_label("DDL").click_accesskit();
        harness.step();
        harness.step();
        harness.get_by_label_contains("CREATE TABLE \"public\".\"pedido\"");
        harness.get_by_label_contains("CREATE INDEX pedido_obs_idx");
        harness.get_by_label("Copiar").click_accesskit();
        harness.step();
        harness.get_by_label("Fechar").click_accesskit();
        harness.step();
        harness.step();
        assert!(harness.query_by_label_contains("CREATE TABLE").is_none());
        // Index: do cache (sem query nova), com include, condição e parâmetros.
        harness.get_by_label("Index").click_accesskit();
        harness.step();
        harness.step();
        harness.get_by_label("pedido_obs_idx");
        harness.get_by_label("nome IS NOT NULL");
        harness.get_by_label("fillfactor=70");
        harness.get_by_label("parcial");
        assert_eq!(queries.load(Ordering::SeqCst), before + 4);
        // Data de volta: o grid continua como estava.
        harness.get_by_label("Data").click_accesskit();
        harness.step();
        harness.step();
        harness.get_by_label("1-3 of 250 rows");

        // Painel direito em Structure não mostra o Form: linha selecionada, mas grid escondido.
        harness.get_all_by_label("1").next().unwrap().click();
        harness.step();
        harness.step();
        harness.get_by_label("Linha 1");
        harness.get_by_label("Structure").click_accesskit();
        harness.step();
        harness.step();
        harness.get_by_label("Nenhuma linha selecionada");
        assert!(harness.query_by_label("Linha 1").is_none());

        // ↻ zera a estrutura; falha ao reler mostra o erro com "Tentar de novo" (sem "Carregando…"),
        // e o botão pede de novo quando o servidor volta.
        fail_structure.store(true, Ordering::SeqCst);
        harness.get_by_label("↻").click();
        wait_for(&mut harness, "StructureFailed", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.data().is_some_and(|d| d.structure_error.is_some()))
        });
        harness.get_by_label_contains("permission denied");
        assert!(harness.query_by_label("Carregando estrutura…").is_none());
        fail_structure.store(false, Ordering::SeqCst);
        harness.get_by_label("Tentar de novo").click();
        wait_for(&mut harness, "StructureLoaded de novo", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.data().is_some_and(|d| d.structure.is_some()))
        });
        harness.get_by_label("🔑 id");
        assert!(
            harness
                .query_by_label_contains("permission denied")
                .is_none()
        );

        // View: Structure mostra a definição.
        harness.get_by_label("◫ v_pedido").click();
        wait_for(&mut harness, "aba da view", |app| {
            app.workspace_of(&id).is_some_and(|ws| {
                ws.data()
                    .is_some_and(|d| d.table == "v_pedido" && !d.loading)
            })
        });
        harness.get_by_label("Structure").click_accesskit();
        wait_for(&mut harness, "estrutura da view", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.data().is_some_and(|d| d.structure.is_some()))
        });
        harness.get_by_label("Definição");
        harness.get_by_label_contains("SELECT id, nome FROM public.pedido");
    }

    /// FEAT-010: + SQL abre a aba; Run Current roda só o statement do cursor; Run All roda todos e
    /// para no erro, com uma sub-aba por statement; o limite trunca; Beautify formata; o JSON da
    /// linha vai ao painel; fechar a aba com texto pede confirmação.
    #[test]
    fn editor_sql_run_current_run_all_beautify_limite_e_fechar() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let driver = FakeDriver::default();
        let queries = Arc::clone(&driver.queries);
        let mut app = App::with_deps(deps(driver, &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let id = c.id.clone();
        let mut harness = wide_harness(app);
        wait_for(&mut harness, "Connected", |app| app.view == View::Workspace);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspace_of(&id)
                .is_some_and(|ws| ws.objects.contains_key("public"))
        });
        let query = |app: &App<FakeDriver>| app.workspace_of(&id).unwrap().query().cloned();

        // + SQL abre "SQL Query 1" ativa, com o editor vazio.
        harness.get_by_label("+ SQL").click_accesskit();
        harness.step();
        harness.step();
        assert_eq!(
            query(harness.state()).map(|q| q.title),
            Some("SQL Query 1".into())
        );
        harness.get_by_label(
            "Nenhum resultado ainda. ⌘↩ roda o statement sob o cursor; ⌘⇧↩ roda todos.",
        );
        // Digita três statements (o cursor fica no fim, no terceiro).
        harness
            .get_by_role(egui::accesskit::Role::MultilineTextInput)
            .focus();
        harness.step();
        harness.input_mut().events.push(egui::Event::Text(
            "select 5;\nselect erro;\nselect 3".into(),
        ));
        harness.step();
        harness.step();
        assert_eq!(
            query(harness.state()).unwrap().buffer,
            "select 5;\nselect erro;\nselect 3"
        );
        harness.get_by_label("linha 3, coluna 9");

        // Run Current (⌘↩): só o statement do cursor, 3 linhas, uma sub-aba.
        let before = queries.load(Ordering::SeqCst);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
        wait_for(&mut harness, "resultado", |app| {
            query(app).is_some_and(|q| !q.running && q.results.len() == 1)
        });
        assert_eq!(queries.load(Ordering::SeqCst), before + 1);
        harness.get_by_label("Query 1");
        harness.get_by_label_contains("3 rows ·");
        assert!(harness.query_by_label("Query 2").is_none());

        // Limite 2 (o ComboBox é o padrão do egui; o estado basta) e Run All (⌘⇧↩): o 1º trunca,
        // o 2º erra, o 3º não roda.
        harness
            .state_mut()
            .workspaces
            .get_mut(&(id.clone(), "loja".to_owned()))
            .and_then(|ws| ws.query_mut())
            .unwrap()
            .limit = Some(2);
        harness.key_press_modifiers(
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            egui::Key::Enter,
        );
        wait_for(&mut harness, "run all", |app| {
            query(app).is_some_and(|q| !q.running && q.results.len() == 2)
        });
        let q = query(harness.state()).unwrap();
        assert!(q.results[0].result.as_ref().unwrap().truncated);
        assert!(q.results[1].result.is_err());
        assert_eq!(q.active_result, 1);
        harness.get_by_label_contains("syntax error");
        harness.get_by_label("Query 1").click_accesskit();
        harness.step();
        harness.step();
        harness.get_by_label_contains("2 rows ·");
        harness.get_by_label_contains("truncado em 2");

        // ⌘-clique no número da linha 2: JSON no painel direito (modificador por evento, clique
        // por acessibilidade, como a seleção do grid de dados).
        harness
            .input_mut()
            .events
            .push(egui::Event::ModifiersChanged(egui::Modifiers::COMMAND));
        harness
            .get_all_by_label("2")
            .next()
            .unwrap()
            .click_accesskit();
        harness.step();
        harness
            .input_mut()
            .events
            .push(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        harness.step();
        harness.step();
        assert!(harness.state().side_panel_open);
        harness.get_by_label_contains("\"n\": \"2\"");

        // Beautify (⌘I) formata o buffer inteiro.
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::I);
        harness.step();
        harness.step();
        assert_eq!(
            query(harness.state()).unwrap().buffer,
            "SELECT\n  5;\nSELECT\n  erro;\nSELECT\n  3"
        );

        // Cancelar pela UI: "select sleep" roda, Cancelar interrompe, o aviso aparece; enquanto roda,
        // × da aba e do banco ficam travados e um segundo ⌘↩ avisa.
        {
            let q = harness
                .state_mut()
                .workspaces
                .get_mut(&(id.clone(), "loja".to_owned()))
                .and_then(|ws| ws.query_mut())
                .unwrap();
            q.buffer = "select sleep".into();
            q.cursor = 12;
        }
        harness.step();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
        wait_for(&mut harness, "rodando", |app| {
            query(app).is_some_and(|q| q.running)
        });
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
        harness.step();
        harness.get_by_label_contains("Já há uma execução em curso");
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::W);
        harness.step();
        assert!(
            harness.query_by_label("Fechar a aba?").is_none(),
            "⌘W com SQL rodando"
        );
        harness.get_by_label("Cancelar").click_accesskit();
        wait_for(&mut harness, "cancelada", |app| {
            query(app).is_some_and(|q| !q.running)
        });
        harness.get_by_label("Execução cancelada");
        // "transação aberta" por banco: BEGIN sem fim acende o aviso; COMMIT apaga. Cada bloco zera
        // `results` antes do ⌘↩: a espera "um resultado e parado" só vale com o resultado novo (com o
        // da execução anterior ainda lá, ela voltava antes de o driver responder sob carga).
        {
            let q = harness
                .state_mut()
                .workspaces
                .get_mut(&(id.clone(), "loja".to_owned()))
                .and_then(|ws| ws.query_mut())
                .unwrap();
            q.buffer = "begin".into();
            q.cursor = 5;
            q.results.clear();
        }
        harness.step();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
        wait_for(&mut harness, "begin", |app| {
            query(app).is_some_and(|q| !q.running && q.results.len() == 1)
        });
        harness.get_by_label("transação aberta");
        // Erro dentro da transação: o servidor a abortou e o COMMIT vai desfazer tudo; a barra avisa
        // (revisão de 22/10/2026). COMMIT (que no servidor vira ROLLBACK) fecha e limpa o aviso.
        {
            let q = harness
                .state_mut()
                .workspaces
                .get_mut(&(id.clone(), "loja".to_owned()))
                .and_then(|ws| ws.query_mut())
                .unwrap();
            q.buffer = "select erro".into();
            q.cursor = 11;
            q.results.clear();
        }
        harness.step();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
        wait_for(&mut harness, "erro na transação", |app| {
            query(app).is_some_and(|q| !q.running && q.results.len() == 1)
        });
        harness.get_by_label("transação abortada");
        assert!(harness.query_by_label("transação aberta").is_none());
        // ROLLBACK TO SAVEPOINT recupera a transação (6ª revisão): o aviso volta a "aberta".
        for (sql, running_label) in [
            ("rollback to savepoint a", "transação aberta"),
            ("select erro", "transação abortada"),
        ] {
            {
                let q = harness
                    .state_mut()
                    .workspaces
                    .get_mut(&(id.clone(), "loja".to_owned()))
                    .and_then(|ws| ws.query_mut())
                    .unwrap();
                q.buffer = sql.into();
                q.cursor = sql.len();
                q.results.clear();
            }
            harness.step();
            harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
            wait_for(&mut harness, sql, |app| {
                query(app).is_some_and(|q| !q.running && q.results.len() == 1)
            });
            harness.get_by_label(running_label);
        }
        assert!(harness.query_by_label("transação aberta").is_none());
        // COMMIT da transação abortada: o servidor faz ROLLBACK e responde OK; a aba avisa.
        {
            let q = harness
                .state_mut()
                .workspaces
                .get_mut(&(id.clone(), "loja".to_owned()))
                .and_then(|ws| ws.query_mut())
                .unwrap();
            q.buffer = "commit".into();
            q.cursor = 6;
            q.results.clear();
        }
        harness.step();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
        wait_for(&mut harness, "commit", |app| {
            query(app).is_some_and(|q| !q.running && q.results.len() == 1)
        });
        harness.step();
        assert!(harness.query_by_label("transação aberta").is_none());
        assert!(harness.query_by_label("transação abortada").is_none());
        harness.get_by_label_contains("o COMMIT virou ROLLBACK");
        // Um SqlResult em voo de uma aba que não está rodando (banco fechado e reaberto, aba
        // fechada) não mexe na transação nem nos resultados (revisão de 22/10/2026).
        let tab_id = query(harness.state()).unwrap().id;
        for tab in [tab_id, 999] {
            harness
                .state_mut()
                .apply_event(hy_dbplus_engine::Event::SqlResult {
                    connection: id.clone(),
                    database: "loja".into(),
                    tab,
                    index: 0,
                    sql: "begin".into(),
                    result: Ok(Outcome::default()),
                    elapsed: Duration::ZERO,
                });
        }
        harness.step();
        assert!(!harness.state().workspaces[&(id.clone(), "loja".to_owned())].open_transaction);
        assert_eq!(query(harness.state()).unwrap().results.len(), 1);
        assert!(harness.query_by_label("transação aberta").is_none());
        // Run Current com o tokenizador perdido e `;` depois: não roda (levaria o DROP junto).
        {
            let q = harness
                .state_mut()
                .workspaces
                .get_mut(&(id.clone(), "loja".to_owned()))
                .and_then(|ws| ws.query_mut())
                .unwrap();
            q.buffer = "select (r)._id from t;\ndrop table x;".into();
            q.cursor = 3;
        }
        harness.step();
        let before = queries.load(std::sync::atomic::Ordering::SeqCst);
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
        harness.step();
        harness.step();
        harness.get_by_label_contains("não sabe onde ele termina");
        assert!(!query(harness.state()).unwrap().running);
        assert_eq!(
            queries.load(std::sync::atomic::Ordering::SeqCst),
            before,
            "nada foi ao servidor"
        );

        // Fechar o banco com texto no editor não pergunta: o texto é salvo em arquivo (FEAT-011,
        // teste `queries_salvas_voltam_ao_abrir_o_banco`). Fechar a aba com texto pede confirmação;
        // "Descartar e fechar" fecha.
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::W);
        harness.step();
        harness.step();
        harness.get_by_label("Fechar a aba?");
        harness.get_by_label("Descartar e fechar").click_accesskit();
        harness.step();
        harness.step();
        assert!(harness.state().workspace_of(&id).unwrap().tabs.is_empty());
        harness.get_by_label("Escolha uma tabela na árvore.");
        // ⌘T abre outra.
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::T);
        harness.step();
        harness.step();
        assert_eq!(
            query(harness.state()).map(|q| q.title),
            Some("SQL Query 2".into())
        );
    }

    /// `select_row` sem harness: ⇧ estende da âncora (para frente e para trás, ou da própria linha sem
    /// âncora), o clique simples na única linha selecionada desmarca, e o painel só abre na primeira seleção.
    #[test]
    fn selecionar_com_shift_e_clique_simples() {
        use crate::app::{DataTab, Workspace};
        use hy_dbplus_engine::ConnectionId;

        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = App::with_deps(deps(FakeDriver::default(), &rt));
        let id: ConnectionId = "c1".into();
        let key: crate::app::DbKey = (id.clone(), "loja".into());
        let mut tab = DataTab::new("public", "pedidos");
        tab.rows = Some(Arc::new(TextRows {
            columns: vec!["id".into()],
            rows: (1..=6).map(|i| vec![Some(i.to_string())]).collect(),
        }));
        app.workspaces.insert(
            key.clone(),
            Workspace {
                tabs: vec![crate::app::Tab::Table(Box::new(tab))],
                active_tab: Some(0),
                ..Default::default()
            },
        );
        app.rail.push(key.clone());
        app.active = Some(key.clone());
        let selected = |app: &App<FakeDriver>| {
            app.workspaces[&key]
                .data()
                .unwrap()
                .selected_rows
                .iter()
                .copied()
                .collect::<Vec<_>>()
        };
        assert!(!app.side_panel_open);
        // ⇧ sem âncora: só a linha clicada; o painel abre.
        app.select_row(&key, 3, egui::Modifiers::SHIFT);
        assert_eq!(selected(&app), [3]);
        assert!(app.side_panel_open);
        // ⇧ para trás e para frente a partir da âncora 3.
        app.select_row(&key, 1, egui::Modifiers::SHIFT);
        assert_eq!(selected(&app), [1, 2, 3]);
        app.select_row(&key, 5, egui::Modifiers::SHIFT);
        assert_eq!(selected(&app), [1, 2, 3, 4, 5]);
        // Clique simples: só ela; de novo na única selecionada: nenhuma. O painel fechado fica fechado.
        app.side_panel_open = false;
        app.select_row(&key, 4, egui::Modifiers::NONE);
        assert_eq!(selected(&app), [4]);
        assert!(!app.side_panel_open);
        app.select_row(&key, 4, egui::Modifiers::NONE);
        assert!(selected(&app).is_empty());
        assert!(!app.side_panel_open);
        // Da seleção vazia, a próxima abre o painel de novo.
        app.select_row(&key, 0, egui::Modifiers::COMMAND);
        assert_eq!(selected(&app), [0]);
        assert!(app.side_panel_open);
    }

    /// FEAT-011: o texto da aba SQL vai para `<queries>/<conexão>/<banco>/<n>.sql` ao perder o foco;
    /// fechar o banco com texto não pergunta e salva; abrir o banco de novo traz a aba com o texto e o
    /// mesmo número, sem resultados; a numeração segue dali; "Descartar e fechar" apaga o arquivo.
    #[test]
    fn queries_salvas_voltam_ao_abrir_o_banco() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let driver = FakeDriver::default();
        let mut app = App::with_deps(deps(driver, &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        crate::connections::connect(&mut app, &c);
        let id = c.id.clone();
        let file = app.deps.queries.dir().join(&id).join("loja").join("1.sql");
        let mut harness = wide_harness(app);
        wait_for(&mut harness, "Connected", |app| app.view == View::Workspace);
        let open_loja = |harness: &mut Harness<'_, App<FakeDriver>>| {
            harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
            harness.step();
            harness.get_by_label("loja").click();
            wait_for(harness, "SchemaLoaded", |app| {
                app.workspace_of(&id)
                    .is_some_and(|ws| ws.objects.contains_key("public"))
            });
        };
        open_loja(&mut harness);
        let query = |app: &App<FakeDriver>| app.workspace_of(&id).unwrap().query().cloned();

        let type_sql = |harness: &mut Harness<'_, App<FakeDriver>>, text: &str| {
            harness
                .get_by_role(egui::accesskit::Role::MultilineTextInput)
                .focus();
            harness.step();
            harness
                .input_mut()
                .events
                .push(egui::Event::Text(text.into()));
            harness.step();
        };
        // Digita na aba 1 e abre outra com ⌘T: a troca de aba salva a 1 (o editor dela some da
        // tela, então é o varrimento por frame que grava).
        harness.get_by_label("+ SQL").click_accesskit();
        harness.step();
        type_sql(&mut harness, "select 1");
        assert!(!file.exists(), "salvou com o editor em foco");
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::T);
        harness.step();
        harness.step();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "select 1");
        // Digita na aba 2 e tira o foco (Esc): salva também.
        let file2 = file.with_file_name("2.sql");
        type_sql(&mut harness, "select 2");
        assert!(!file2.exists());
        harness.key_press(egui::Key::Escape);
        harness.step();
        harness.step();
        assert_eq!(std::fs::read_to_string(&file2).unwrap(), "select 2");
        assert_eq!(query(harness.state()).unwrap().saved, "select 2");

        // × do banco com texto: fecha sem perguntar; os arquivos ficam.
        harness.get_all_by_label("×").next().unwrap().click();
        wait_for(&mut harness, "DatabaseClosed", |app| app.rail.is_empty());
        assert!(harness.query_by_label("Fechar o banco?").is_none());
        assert!(file.exists() && file2.exists());

        // Abrir de novo: as abas voltam na ordem, a primeira ativa, com o texto e sem resultados;
        // ⌘T continua a numeração.
        open_loja(&mut harness);
        harness.step();
        let q = query(harness.state()).unwrap();
        assert_eq!(
            (q.title.as_str(), q.buffer.as_str()),
            ("SQL Query 1", "select 1")
        );
        assert!(q.results.is_empty());
        harness.get_by_label("SQL Query 2");
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::T);
        harness.step();
        harness.step();
        assert_eq!(
            query(harness.state()).map(|q| q.title),
            Some("SQL Query 3".into())
        );
        // ⌘W na aba vazia fecha direto e não deixa arquivo; a da esquerda (2) assume; na aba com
        // texto pergunta, e "Descartar e fechar" apaga o arquivo.
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::W);
        harness.step();
        harness.step();
        assert!(!file.with_file_name("3.sql").exists());
        assert_eq!(
            query(harness.state()).map(|q| q.title),
            Some("SQL Query 2".into())
        );
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::W);
        harness.step();
        harness.step();
        harness.get_by_label("Fechar a aba?");
        harness.get_by_label("Descartar e fechar").click_accesskit();
        harness.step();
        harness.step();
        assert!(!file2.exists(), "Descartar não apagou o arquivo");
        assert!(file.exists(), "apagou o arquivo da outra aba");
        assert_eq!(harness.state().workspace_of(&id).unwrap().tabs.len(), 1);
        // A numeração não volta atrás dentro do banco aberto.
        harness.get_by_label("+ SQL").click_accesskit();
        harness.step();
        harness.step();
        assert_eq!(
            query(harness.state()).map(|q| q.title),
            Some("SQL Query 4".into())
        );
    }

    /// FEAT-011: o `Disconnected` de uma conexão que ainda está na lista salva o texto das abas SQL
    /// (conexão caída); o de uma conexão removida não (o diretório dela acabou de ser apagado e
    /// voltaria como lixo órfão).
    #[test]
    fn desconectar_salva_as_queries_so_de_conexao_que_ainda_existe() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = App::with_deps(deps(FakeDriver::default(), &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        let id = c.id.clone();
        let dir = app.deps.queries.dir().join(&id);
        let open_with_text = |app: &mut App<FakeDriver>| {
            let key = (id.clone(), "loja".to_owned());
            let mut ws = crate::app::Workspace::default();
            let mut q = crate::app::QueryTab::new(1);
            q.buffer = "select 1".into();
            ws.tabs.push(crate::app::Tab::Query(q));
            ws.next_query_id = 1;
            app.workspaces.insert(key.clone(), ws);
            app.rail.push(key);
        };
        open_with_text(&mut app);
        app.apply_event(hy_dbplus_engine::Event::Disconnected {
            connection: id.clone(),
        });
        assert_eq!(
            std::fs::read_to_string(dir.join("loja").join("1.sql")).unwrap(),
            "select 1"
        );
        assert!(app.workspaces.is_empty());

        // Removida da lista: o diretório some e o Disconnected não o recria.
        open_with_text(&mut app);
        app.deps.store.remove(&id);
        app.deps.queries.remove_connection(&id).unwrap();
        app.apply_event(hy_dbplus_engine::Event::Disconnected {
            connection: id.clone(),
        });
        assert!(!dir.exists(), "recriou o diretório da conexão removida");
    }

    /// FEAT-011: "Descartar e fechar" do banco ou da conexão (a caixa só aparece por pendência em aba
    /// de tabela) não apaga as queries salvas; só o descarte da própria aba SQL apaga. E uma falha
    /// persistente ao salvar não é tentada de novo a cada frame nem sobrescreve outro aviso.
    #[test]
    fn descartar_banco_ou_conexao_mantem_as_queries_e_falha_nao_repete_a_cada_frame() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = App::with_deps(deps(FakeDriver::default(), &rt));
        let c = Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default());
        app.deps.store.upsert(c.clone());
        let id = c.id.clone();
        let key = (id.clone(), "loja".to_owned());
        let file = app.deps.queries.dir().join(&id).join("loja").join("1.sql");
        let open_with_text = |app: &mut App<FakeDriver>| {
            let mut ws = crate::app::Workspace::default();
            let mut q = crate::app::QueryTab::new(1);
            q.buffer = "select 1".into();
            ws.tabs.push(crate::app::Tab::Query(q));
            ws.active_tab = Some(0);
            ws.next_query_id = 1;
            app.workspaces.insert(key.clone(), ws);
            app.rail.push(key.clone());
        };
        open_with_text(&mut app);
        app.confirm_close = Some(crate::app::CloseTarget::Database(key.clone()));
        app.confirm_close_now();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "select 1");
        app.workspaces.remove(&key);
        app.rail.clear();
        open_with_text(&mut app);
        app.confirm_close = Some(crate::app::CloseTarget::Connection(id.clone()));
        app.confirm_close_now();
        assert!(file.exists(), "descartar a conexão apagou a query salva");
        app.workspaces.remove(&key);
        app.rail.clear();
        // Descartar a própria aba apaga.
        open_with_text(&mut app);
        app.confirm_close = Some(crate::app::CloseTarget::Tab(key.clone(), 0));
        app.confirm_close_now();
        assert!(!file.exists());
        app.workspaces.remove(&key);
        app.rail.clear();

        // Falha persistente: o diretório da conexão virou arquivo. O varrimento tenta uma vez, põe o
        // aviso, e não insiste enquanto o texto não mudar (outro aviso sobrevive).
        std::fs::remove_dir_all(app.deps.queries.dir().join(&id)).unwrap();
        std::fs::write(app.deps.queries.dir().join(&id), "x").unwrap();
        open_with_text(&mut app);
        let ctx = egui::Context::default();
        app.autosave_unfocused_queries(&ctx);
        let q = app.workspaces[&key].tabs[0].query().unwrap();
        assert!(
            q.notice
                .as_deref()
                .is_some_and(|n| n.contains("não foi possível salvar")),
            "{:?}",
            q.notice
        );
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .query_mut()
            .unwrap()
            .notice = Some("outro aviso".into());
        app.autosave_unfocused_queries(&ctx);
        assert_eq!(
            app.workspaces[&key].tabs[0]
                .query()
                .unwrap()
                .notice
                .as_deref(),
            Some("outro aviso"),
            "tentou de novo com o mesmo texto"
        );
        // Texto mudou: tenta de novo (e falha de novo, com o aviso).
        app.workspaces
            .get_mut(&key)
            .unwrap()
            .query_mut()
            .unwrap()
            .buffer = "select 2".into();
        app.autosave_unfocused_queries(&ctx);
        assert!(
            app.workspaces[&key].tabs[0]
                .query()
                .unwrap()
                .notice
                .as_deref()
                .is_some_and(|n| n.contains("não foi possível salvar"))
        );
        // Aba restaurada, texto apagado, ⌘W: fecha direto, mas apagar o arquivo falha e o erro não
        // pode sumir com a aba (vai ao aviso do app). O alvo é um diretório chamado `1.sql`: `remove_file`
        // recusa nas três plataformas. Com `<id>` arquivo, como acima, o Windows daria
        // `ERROR_PATH_NOT_FOUND`, que é `NotFound`, e o remove passaria como "ausente" (CI de 23/10).
        std::fs::remove_file(app.deps.queries.dir().join(&id)).unwrap();
        std::fs::create_dir_all(&file).unwrap();
        {
            let q = app.workspaces.get_mut(&key).unwrap().query_mut().unwrap();
            q.buffer.clear();
            q.saved = "select 1".into();
        }
        app.notice = None;
        app.request_close_tab(&key, 0);
        assert!(
            app.workspaces[&key].tabs.is_empty(),
            "a aba vazia fecha mesmo assim"
        );
        assert!(
            app.notice
                .as_deref()
                .is_some_and(|n| n.contains("não foi possível salvar")),
            "{:?}",
            app.notice
        );
    }
}
