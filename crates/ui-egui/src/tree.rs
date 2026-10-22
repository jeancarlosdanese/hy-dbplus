//! A árvore de objetos (tela 3.3): grupos **Functions** e **Tables**, views com ícone próprio,
//! tabela expansível até as colunas `nome  tipo`, chave primária com ícone, busca e o seletor de
//! schema no rodapé do painel.

use hy_dbplus_engine::{ConnectionId, Driver, TableKind};

use crate::app::App;
use crate::theme;

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, id: &ConnectionId)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let Some(ws) = app.workspaces.get(id) else {
        return;
    };
    let Some(database) = ws.database.clone() else {
        return;
    };
    let schemas: Vec<String> = ws.schemas.iter().map(|s| s.name.clone()).collect();
    let current = ws.current_schema.clone();
    // Com edição pendente, trocar de tabela perderia o lote e recarregar esvaziaria as colunas (e a
    // chave primária) da tabela aberta: os dois ficam desabilitados com a dica.
    let locked = app.has_pending_changes(id);

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
                app.deps.sessions.refresh(id, &database);
                if let Some(schema) = &current {
                    if let Some(ws) = app.workspaces.get_mut(id) {
                        ws.objects.remove(schema);
                        ws.columns.clear();
                        ws.expanded.clear(); // tabela aberta pede as colunas de novo
                        ws.loading = true;
                    }
                    app.deps.sessions.load_schema(id, &database, schema);
                }
            }
        });
    });
    if let Some(schema) = chosen_schema {
        if let Some(ws) = app.workspaces.get_mut(id) {
            ws.current_schema = Some(schema.clone());
            ws.expanded.clear();
            ws.loading = !ws.objects.contains_key(&schema);
        }
        app.deps.sessions.load_schema(id, &database, &schema);
    }

    // Busca e a árvore.
    egui::CentralPanel::default().show(ui, |ui| {
        let Some(ws) = app.workspaces.get_mut(id) else {
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
                app.deps.sessions.load_schema(id, &database, &schema);
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
                                    .data
                                    .as_ref()
                                    .is_some_and(|d| d.schema == schema && d.table == t.name);
                                if ui
                                    .add_enabled(
                                        !locked || selected,
                                        egui::Button::selectable(
                                            selected,
                                            format!("{icon} {}", t.name),
                                        ),
                                    )
                                    .on_disabled_hover_text(crate::data::LOCK_HINT)
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
                .load_columns(id, &database, &schema, &table);
        }
        if let Some(table) = to_open {
            app.open_table(id, &schema, &table);
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
        DriverError, Secret, TextRows,
    };
    use hy_dbplus_engine::{Connection, Direction, Tag};

    use crate::app::{App, View, tests::deps};

    /// Driver falso com o catálogo de um banco "loja": schemas app e public, tabela pedido (PK id)
    /// e view v_pedido, função dobro.
    #[derive(Default)]
    struct FakeDriver {
        queries: Arc<AtomicUsize>,
        batches: Arc<Mutex<Vec<Vec<BatchStatement>>>>,
    }
    struct FakeConn {
        queries: Arc<AtomicUsize>,
        batches: Arc<Mutex<Vec<Vec<BatchStatement>>>>,
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
        wait_for(&mut harness, "Connected", |app| {
            matches!(app.view, View::Workspace { .. })
        });
        harness.get_by_label("Nenhum banco selecionado.");

        // ⌘K abre a paleta com os bancos; escolher "loja" abre o banco e carrega public.
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspaces
                .get(&id)
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
            app.workspaces.get(&id).is_some_and(|ws| {
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
            harness.state().workspaces[&id].current_schema.as_deref(),
            Some("app"),
            "clique no schema"
        );
        wait_for(&mut harness, "schema app", |app| {
            app.workspaces
                .get(&id)
                .is_some_and(|ws| ws.objects.contains_key("app"))
        });
        assert_eq!(
            harness.state().workspaces[&id].current_schema.as_deref(),
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
        wait_for(&mut harness, "Connected", |app| {
            matches!(app.view, View::Workspace { .. })
        });
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspaces
                .get(&id)
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
            app.workspaces.get(&id).is_some_and(|ws| ws.error.is_some())
        });
        harness.get_by_label_contains("permission denied");
        assert!(harness.state().deps.sessions.is_open(&id));
        assert_eq!(
            harness.state().workspaces[&id].database.as_deref(),
            Some("loja")
        );
        let before = queries.load(Ordering::SeqCst);

        // Tentar de novo refaz a query (e falha de novo, de propósito: o erro é do servidor).
        harness.get_by_label("Tentar de novo").click();
        harness.step();
        wait_for(&mut harness, "nova tentativa", |app| {
            app.workspaces.get(&id).is_some_and(|ws| ws.error.is_some())
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
        wait_for(&mut harness, "Connected", |app| {
            matches!(app.view, View::Workspace { .. })
        });
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("template0").click();
        wait_for(&mut harness, "CatalogFailed", |app| {
            app.workspaces.get(&id).is_some_and(|ws| ws.error.is_some())
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
        wait_for(&mut harness, "Connected", |app| {
            matches!(app.view, View::Workspace { .. })
        });
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspaces
                .get(&id)
                .is_some_and(|ws| ws.objects.contains_key("public"))
        });
        harness.get_all_by_label("▸").next().unwrap().click();
        wait_for(&mut harness, "ColumnsLoaded", |app| {
            app.workspaces
                .get(&id)
                .is_some_and(|ws| !ws.columns.is_empty())
        });
        harness.get_by_label("🔑 id");
        let before = queries.load(Ordering::SeqCst);
        harness.get_by_label("↻").click();
        wait_for(&mut harness, "cache esvaziado", |app| {
            app.workspaces
                .get(&id)
                .is_some_and(|ws| ws.columns.is_empty())
        });
        wait_for(&mut harness, "colunas de volta", |app| {
            app.workspaces
                .get(&id)
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
        let id = c.id.clone();
        let mut harness = Harness::new_ui_state(|ui, app: &mut App<FakeDriver>| app.ui(ui), app);
        wait_for(&mut harness, "Connected", |app| {
            matches!(app.view, View::Workspace { .. })
        });
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
            app.workspaces
                .get(&id)
                .is_some_and(|ws| ws.database.as_deref() == Some("loja"))
        });
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
        let mut harness = Harness::new_ui_state(|ui, app: &mut App<FakeDriver>| app.ui(ui), app);
        wait_for(&mut harness, "Connected", |app| {
            matches!(app.view, View::Workspace { .. })
        });
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspaces
                .get(&id)
                .is_some_and(|ws| ws.objects.contains_key("public"))
        });
        harness.get_by_label("Escolha uma tabela na árvore.");

        // O nome abre a aba Data: página, contagem e tamanhos.
        harness.get_by_label("▦ pedido").click();
        wait_for(&mut harness, "PageLoaded", |app| {
            app.workspaces
                .get(&id)
                .is_some_and(|ws| ws.data.as_ref().is_some_and(|d| d.rows.is_some()))
        });
        wait_for(&mut harness, "StatsLoaded", |app| {
            app.workspaces
                .get(&id)
                .is_some_and(|ws| ws.data.as_ref().is_some_and(|d| d.stats.is_some()))
        });
        harness.get_by_label("public.pedido");
        harness.get_by_label("1-3 of 250 rows");
        // Tamanhos e comentário numa linha acima do grid (o painel começa fechado).
        harness.get_by_label_contains("250 rows · total 24 kB");
        harness.get_by_label("pedidos");
        let names = |app: &App<FakeDriver>| -> Vec<String> {
            app.workspaces[&id]
                .data
                .as_ref()
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
            app.workspaces[&id].data.as_ref().is_some_and(|d| {
                !d.loading
                    && d.page
                        .order
                        .as_ref()
                        .is_some_and(|o| o.direction == Direction::Asc)
            })
        });
        harness.get_by_label("nome ▲").click();
        wait_for(&mut harness, "desc", |app| {
            app.workspaces[&id].data.as_ref().is_some_and(|d| {
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
            app.workspaces[&id]
                .data
                .as_ref()
                .is_some_and(|d| !d.loading && d.page.offset == 100)
        });
        harness.get_by_label("101-103 of 250 rows");

        // Selecionar a primeira linha mostra os campos no painel direito.
        harness.get_by_label("101").click_accesskit();
        harness.step();
        harness.step(); // o painel direito é desenhado antes do grid: lê a seleção no frame seguinte
        let selected = |app: &App<FakeDriver>| {
            app.workspaces[&id]
                .data
                .as_ref()
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
        let mut harness = Harness::new_ui_state(|ui, app: &mut App<FakeDriver>| app.ui(ui), app);
        wait_for(&mut harness, "Connected", |app| {
            matches!(app.view, View::Workspace { .. })
        });
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
        harness.step();
        harness.get_by_label("loja").click();
        wait_for(&mut harness, "SchemaLoaded", |app| {
            app.workspaces
                .get(&id)
                .is_some_and(|ws| ws.objects.contains_key("public"))
        });
        // Abre a tabela pelo rótulo da árvore e espera a aba NOVA (não a anterior, que já estava
        // carregada) com página, stats e colunas (a chave primária vem delas).
        let open = |harness: &mut Harness<'_, App<FakeDriver>>, label: &str, table: &str| {
            harness.get_by_label(label).click();
            wait_for(harness, "tabela aberta e carregada", |app| {
                app.workspaces.get(&id).is_some_and(|ws| {
                    ws.data.as_ref().is_some_and(|d| {
                        d.table == table && d.rows.is_some() && d.stats.is_some() && !d.loading
                    }) && ws
                        .columns
                        .contains_key(&("public".to_owned(), table.to_owned()))
                })
            });
        };
        open(&mut harness, "▦ pedido", "pedido");
        let tab = |app: &App<FakeDriver>| app.workspaces[&id].data.clone().unwrap();

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
            app.workspaces[&id]
                .data
                .as_ref()
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
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
        harness.step();
        harness.step();
        harness.get_by_label("1 updates, 0 deletes");
        harness.get_by_label("Executar").click_accesskit();
        wait_for(&mut harness, "ChangesFailed", |app| {
            app.workspaces[&id]
                .data
                .as_ref()
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
    fn app_with_editable_table() -> (App<FakeDriver>, hy_dbplus_engine::ConnectionId) {
        use crate::app::{DataTab, Workspace};
        use hy_dbplus_engine::{Column, Page};

        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = App::with_deps(deps(FakeDriver::default(), &rt));
        std::mem::forget(rt); // o teste é síncrono; o runtime só precisa existir
        let id: hy_dbplus_engine::ConnectionId = "c1".into();
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
            database: Some("loja".into()),
            data: Some(tab),
            ..Default::default()
        };
        let column = |name: &str, pk: bool, position: i32| Column {
            name: name.into(),
            data_type: if pk { "integer" } else { "text" }.into(),
            nullable: !pk,
            default: None,
            position,
            is_primary_key: pk,
        };
        ws.columns.insert(
            ("public".into(), "pedidos".into()),
            vec![column("id", true, 1), column("nome", false, 2)],
        );
        app.workspaces.insert(id.clone(), ws);
        (app, id)
    }

    fn tab_of(app: &App<FakeDriver>, id: &str) -> crate::app::DataTab {
        app.workspaces[id].data.clone().unwrap()
    }

    /// Abrir uma célula NULL e sair sem digitar não vira pendência `''` (achado do revisor); digitar
    /// e apagar tudo sobre NULL volta a NULL; valor igual ao da página some da pendência.
    #[test]
    fn celula_null_aberta_e_fechada_sem_digitar_nao_vira_pendencia() {
        let (mut app, id) = app_with_editable_table();
        assert!(app.editable(&id));
        app.start_edit(&id, 1, 1);
        assert_eq!(tab_of(&app, &id).editing.unwrap().text, "");
        app.finish_edit(&id, true);
        assert!(
            !tab_of(&app, &id).has_changes(),
            "NULL aberto e fechado virou ''"
        );
        // Digitar sobre NULL vira texto; apagar tudo sobre o texto vira '' (string vazia; NULL é
        // só pelo botão do Form), e o NULL pelo Form devolve a célula ao valor da página.
        app.start_edit(&id, 1, 1);
        app.workspaces
            .get_mut(&id)
            .unwrap()
            .data
            .as_mut()
            .unwrap()
            .editing
            .as_mut()
            .unwrap()
            .text = "x".into();
        app.finish_edit(&id, true);
        assert_eq!(tab_of(&app, &id).cell_value(1, 1), Some(Some("x")));
        app.start_edit(&id, 1, 1);
        app.workspaces
            .get_mut(&id)
            .unwrap()
            .data
            .as_mut()
            .unwrap()
            .editing
            .as_mut()
            .unwrap()
            .text = String::new();
        app.finish_edit(&id, true);
        assert!(tab_of(&app, &id).cell_value(1, 1) == Some(Some("")));
        app.workspaces
            .get_mut(&id)
            .unwrap()
            .data
            .as_mut()
            .unwrap()
            .set_cell(1, 1, None);
        assert!(!tab_of(&app, &id).has_changes());
        // Célula com texto: voltar ao valor da página também some da pendência; Esc descarta.
        app.start_edit(&id, 0, 1);
        app.workspaces
            .get_mut(&id)
            .unwrap()
            .data
            .as_mut()
            .unwrap()
            .editing
            .as_mut()
            .unwrap()
            .text = "b".into();
        app.finish_edit(&id, true);
        assert!(tab_of(&app, &id).is_pending(0, "nome"));
        app.start_edit(&id, 0, 1);
        app.workspaces
            .get_mut(&id)
            .unwrap()
            .data
            .as_mut()
            .unwrap()
            .editing
            .as_mut()
            .unwrap()
            .text = "a".into();
        app.finish_edit(&id, true);
        assert!(!tab_of(&app, &id).has_changes());
        app.start_edit(&id, 0, 1);
        app.workspaces
            .get_mut(&id)
            .unwrap()
            .data
            .as_mut()
            .unwrap()
            .editing
            .as_mut()
            .unwrap()
            .text = "zzz".into();
        app.finish_edit(&id, false);
        assert!(!tab_of(&app, &id).has_changes());
    }

    /// Com página em voo não se edita (a pendência é por índice), e uma página nova descarta a
    /// pendência com aviso em vez de deixar os índices apontarem outras linhas (achado do revisor).
    #[test]
    fn pagina_em_voo_trava_a_edicao_e_pagina_nova_descarta_a_pendencia() {
        use hy_dbplus_engine::Page;

        let (mut app, id) = app_with_editable_table();
        {
            let tab = app.workspaces.get_mut(&id).unwrap().data.as_mut().unwrap();
            tab.set_cell(0, 1, Some("b".into()));
            tab.deleted.insert(2);
            assert!(tab.has_changes());
            tab.requested = Some(Page::default());
        }
        assert!(!app.editable(&id), "editável com página em voo");
        app.start_edit(&id, 0, 1);
        assert!(tab_of(&app, &id).editing.is_none());
        let tab = app.workspaces.get_mut(&id).unwrap().data.as_mut().unwrap();
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
        let (mut app, id) = app_with_editable_table();
        {
            let tab = app.workspaces.get_mut(&id).unwrap().data.as_mut().unwrap();
            tab.selected_rows.extend([0, 1, 2]);
        }
        app.toggle_delete(&id); // marca as três
        app.toggle_delete(&id); // todas marcadas: desmarca
        assert!(tab_of(&app, &id).deleted.is_empty());
        {
            let tab = app.workspaces.get_mut(&id).unwrap().data.as_mut().unwrap();
            tab.selected_rows.clear();
            tab.selected_rows.insert(2);
        }
        app.toggle_delete(&id);
        app.workspaces
            .get_mut(&id)
            .unwrap()
            .data
            .as_mut()
            .unwrap()
            .selected_rows
            .extend([0, 1]);
        app.set_field(&id, "nome", Some("z".into()));
        let tab = tab_of(&app, &id);
        assert!(tab.is_pending(0, "nome") && tab.is_pending(1, "nome"));
        assert!(!tab.is_pending(2, "nome"), "linha marcada recebeu edição");
        let changes = tab.change_set(&["id".to_owned()]).unwrap();
        assert_eq!(changes.updates.len(), 2);
        assert_eq!(changes.deletes, vec![vec!["3".to_owned()]]);
        app.set_field(&id, "nome", None);
        assert_eq!(tab_of(&app, &id).cell_value(0, 1), Some(None));
        app.revert_field(&id, "nome");
        assert!(tab_of(&app, &id).pending.is_empty());
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
        let mut tab = DataTab::new("public", "pedidos");
        tab.rows = Some(Arc::new(TextRows {
            columns: vec!["id".into()],
            rows: (1..=6).map(|i| vec![Some(i.to_string())]).collect(),
        }));
        app.workspaces.insert(
            id.clone(),
            Workspace {
                data: Some(tab),
                ..Default::default()
            },
        );
        let selected = |app: &App<FakeDriver>| {
            app.workspaces[&id]
                .data
                .as_ref()
                .unwrap()
                .selected_rows
                .iter()
                .copied()
                .collect::<Vec<_>>()
        };
        assert!(!app.side_panel_open);
        // ⇧ sem âncora: só a linha clicada; o painel abre.
        app.select_row(&id, 3, egui::Modifiers::SHIFT);
        assert_eq!(selected(&app), [3]);
        assert!(app.side_panel_open);
        // ⇧ para trás e para frente a partir da âncora 3.
        app.select_row(&id, 1, egui::Modifiers::SHIFT);
        assert_eq!(selected(&app), [1, 2, 3]);
        app.select_row(&id, 5, egui::Modifiers::SHIFT);
        assert_eq!(selected(&app), [1, 2, 3, 4, 5]);
        // Clique simples: só ela; de novo na única selecionada: nenhuma. O painel fechado fica fechado.
        app.side_panel_open = false;
        app.select_row(&id, 4, egui::Modifiers::NONE);
        assert_eq!(selected(&app), [4]);
        assert!(!app.side_panel_open);
        app.select_row(&id, 4, egui::Modifiers::NONE);
        assert!(selected(&app).is_empty());
        assert!(!app.side_panel_open);
        // Da seleção vazia, a próxima abre o painel de novo.
        app.select_row(&id, 0, egui::Modifiers::COMMAND);
        assert_eq!(selected(&app), [0]);
        assert!(app.side_panel_open);
    }
}
