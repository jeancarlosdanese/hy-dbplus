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
                .small_button("↻")
                .on_hover_text("Recarregar o catálogo")
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
                .load_columns(id, &database, &schema, &table);
        }
        if let Some(table) = to_open {
            app.open_table(id, &schema, &table);
        }
    });
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use hy_dbplus_driver::{
        Connection as DriverConnection, ConnectionConfig, DatabaseInfo, Driver, DriverError,
        Secret, TextRows,
    };
    use hy_dbplus_engine::{Connection, Direction, Tag};

    use crate::app::{App, View, tests::deps};

    /// Driver falso com o catálogo de um banco "loja": schemas app e public, tabela pedido (PK id)
    /// e view v_pedido, função dobro.
    #[derive(Default)]
    struct FakeDriver {
        queries: Arc<AtomicUsize>,
    }
    struct FakeConn {
        queries: Arc<AtomicUsize>,
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
                vec![
                    row(&[
                        Some("id"),
                        Some("integer"),
                        Some("f"),
                        None,
                        Some("1"),
                        Some("t"),
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
        assert_eq!(
            harness.get_all_by_label("Múltiplos valores").count(),
            2,
            "id e nome diferem"
        );
        // "ok" nas três células do grid e uma vez no Form (campo igual nas duas linhas).
        assert_eq!(harness.get_all_by_label("ok").count(), 4);
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
        harness.get_by_label("Painel ▸").click_accesskit();
        harness.step();
        harness.get_by_label("102").click_accesskit();
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
