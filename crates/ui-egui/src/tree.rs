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
                        ws.loading = true;
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
            tabs: vec![tab],
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
                DataTab::new("public", "a"),
                DataTab::new("public", "b"),
                DataTab::new("public", "c"),
            ],
            active_tab: active,
            ..Default::default()
        };
        let names = |ws: &Workspace| ws.tabs.iter().map(|t| t.table.clone()).collect::<Vec<_>>();
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
                .all(|t| t.rows.is_some() && t.stats.is_some() && !t.loading)
        });
        let ws = &app.workspaces[&key];
        assert_eq!(ws.active_tab, Some(0));
        assert_eq!(ws.tabs[1].table, "v_pedido");
        assert!(ws.tabs[1].rows.as_ref().is_some_and(|r| r.rows.len() == 3));

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
        assert!(app.workspaces[&key].tabs[0].commit_running());
        app.workspaces.get_mut(&key).unwrap().active_tab = Some(1);
        let other_requested_before = app.workspaces[&key].tabs[1].requested.clone();
        drain_until(&mut app, "ChangesApplied e recarga", |app| {
            let t = &app.workspaces[&key].tabs[0];
            t.commit.is_none() && !t.has_changes() && !t.loading && t.requested.is_none()
        });
        let ws = &app.workspaces[&key];
        assert_eq!(ws.active_tab, Some(1), "a troca de aba não foi desfeita");
        assert_eq!(
            ws.tabs[1].requested, other_requested_before,
            "a aba ativa (outra) não foi recarregada no lugar da do lote"
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
                tabs: vec![tab],
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
}
