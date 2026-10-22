//! O app egui: estado, roteamento entre as telas e leitura dos eventos do engine a cada frame.
//! Genérico no driver para o teste de interação rodar com um engine falso; o binário usa
//! [`PgApp`].

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use hy_dbplus_engine::{
    Column, ConnectionId, ConnectionStore, DatabaseInfo, Driver, Event, Function, Page,
    PostgresDriver, Schema, SecretStore, Sessions, Table, TableStats, TextRows,
};

use crate::connections::{ConnectionForm, Connections};
use crate::theme;

/// O app com o driver PostgreSQL, como o binário o cria.
pub type PgApp = App<PostgresDriver>;

/// O que o app recebe pronto do binário: o engine é dele, a UI só usa.
pub struct Deps<D: Driver> {
    pub store: ConnectionStore,
    pub secrets: Box<dyn SecretStore>,
    pub sessions: Sessions<D>,
    pub events: Receiver<Event>,
}

/// Estado de uma conexão do ponto de vista da UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Idle,
    Connecting,
    Connected {
        server_version: String,
        databases: Vec<DatabaseInfo>,
    },
    Failed(String),
}

/// O formato do painel de registro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelMode {
    #[default]
    Form,
    Json,
}

/// Qual tela está na frente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Connections,
    Workspace { connection: ConnectionId },
}

/// O banco selecionado numa conexão e o catálogo que a UI já recebeu (tela 3.3).
#[derive(Debug, Default)]
pub struct Workspace {
    pub database: Option<String>,
    pub schemas: Vec<Schema>,
    pub current_schema: Option<String>,
    /// Tabelas e funções por schema, como chegaram do engine.
    pub objects: HashMap<String, (Vec<Table>, Vec<Function>)>,
    pub columns: HashMap<(String, String), Vec<Column>>,
    /// Tabelas abertas na árvore, por (schema, tabela).
    pub expanded: HashSet<(String, String)>,
    pub loading: bool,
    pub error: Option<String>,
    pub search: String,
    /// A tabela aberta na aba Data, se houver.
    pub data: Option<DataTab>,
}

impl Workspace {
    /// As colunas do catálogo (nome, tipo) na ordem das colunas de uma página de dados, casadas pelo
    /// nome; `None` onde o cache não tem a coluna. Cabeçalho do grid, Form e JSON usam esta busca.
    pub fn catalog_columns(
        &self,
        schema: &str,
        table: &str,
        names: &[String],
    ) -> Vec<Option<&Column>> {
        let cache = self.columns.get(&(schema.to_owned(), table.to_owned()));
        names
            .iter()
            .map(|name| cache.and_then(|c| c.iter().find(|c| &c.name == name)))
            .collect()
    }
}

/// A aba Data de uma tabela (tela 3.4): página atual, ordenação, contagem, tamanhos e seleção.
#[derive(Debug, Clone)]
pub struct DataTab {
    pub schema: String,
    pub table: String,
    /// A página confirmada (a das `rows`); `requested` é a pedida e ainda não chegada.
    pub page: Page,
    pub requested: Option<Page>,
    /// `Arc`: a aba é clonada a cada frame para desenhar; as linhas não.
    pub rows: Option<Arc<TextRows>>,
    pub stats: Option<TableStats>,
    /// Linhas selecionadas (índices da página); `anchor` é a última clicada, para ⇧ estender.
    pub selected_rows: BTreeSet<usize>,
    pub anchor: Option<usize>,
    /// Campos do Form abertos por "mostrar tudo".
    pub expanded_fields: HashSet<String>,
    pub loading: bool,
    pub error: Option<String>,
}

impl DataTab {
    pub fn new(schema: &str, table: &str) -> Self {
        Self {
            schema: schema.into(),
            table: table.into(),
            page: Page::default(),
            requested: Some(Page::default()),
            rows: None,
            stats: None,
            selected_rows: BTreeSet::new(),
            anchor: None,
            expanded_fields: HashSet::new(),
            loading: true,
            error: None,
        }
    }
}

/// A paleta ⌘K (tela 3.2): busca entre os bancos da conexão.
#[derive(Debug, Default)]
pub struct Palette {
    pub query: String,
    pub selected: usize,
    /// Pede o foco do campo uma vez, ao abrir; depois o usuário manda.
    pub focus_pending: bool,
}

pub struct App<D: Driver> {
    pub(crate) deps: Deps<D>,
    pub(crate) view: View,
    pub(crate) status: HashMap<ConnectionId, Status>,
    pub(crate) workspaces: HashMap<ConnectionId, Workspace>,
    pub(crate) palette: Option<Palette>,
    /// Painel direito visível (botão na barra superior; abre sozinho na primeira seleção).
    pub(crate) side_panel_open: bool,
    pub(crate) panel_mode: PanelMode,
    pub(crate) connections: Connections,
    pub(crate) form: Option<ConnectionForm>,
    /// Último erro de arquivo ou chaveiro, mostrado no rodapé até o próximo sucesso.
    pub(crate) notice: Option<String>,
}

impl<D> App<D>
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    /// Cria o app e aplica o tema ao contexto do egui.
    pub fn new(cc: &eframe::CreationContext<'_>, deps: Deps<D>) -> Self {
        theme::apply(&cc.egui_ctx);
        Self::with_deps(deps)
    }

    /// Sem o `CreationContext`: para testes e para o `egui_kittest`.
    pub fn with_deps(deps: Deps<D>) -> Self {
        Self {
            deps,
            view: View::Connections,
            status: HashMap::new(),
            workspaces: HashMap::new(),
            palette: None,
            side_panel_open: false,
            panel_mode: PanelMode::default(),
            connections: Connections::default(),
            form: None,
            notice: None,
        }
    }

    /// Lê o que o engine mandou desde o último frame.
    pub(crate) fn drain_events(&mut self) {
        while let Ok(event) = self.deps.events.try_recv() {
            let id = event.connection().clone();
            match event {
                Event::Connecting { .. } => {
                    self.status.insert(id, Status::Connecting);
                }
                Event::Connected {
                    server_version,
                    databases,
                    ..
                } => {
                    self.status.insert(
                        id.clone(),
                        Status::Connected {
                            server_version,
                            databases,
                        },
                    );
                    self.workspaces.insert(id.clone(), Workspace::default());
                    self.view = View::Workspace { connection: id };
                }
                Event::Failed { error, .. } => {
                    self.status.insert(id, Status::Failed(error.to_string()));
                }
                Event::Disconnected { .. } => {
                    // Conexão já removida da lista não volta ao mapa.
                    if self.deps.store.get(&id).is_some() {
                        self.status.insert(id.clone(), Status::Idle);
                    }
                    self.workspaces.remove(&id);
                    if self.view == (View::Workspace { connection: id }) {
                        self.view = View::Connections;
                    }
                }
                Event::DatabaseSelected {
                    database,
                    schemas,
                    current_schema,
                    ..
                } => {
                    let ws = self.workspaces.entry(id.clone()).or_default();
                    *ws = Workspace {
                        database: Some(database.clone()),
                        schemas,
                        current_schema: current_schema.clone(),
                        ..Default::default()
                    };
                    if let Some(schema) = current_schema {
                        ws.loading = true;
                        self.deps.sessions.load_schema(&id, &database, &schema);
                    }
                }
                Event::SchemaLoaded {
                    database,
                    schema,
                    tables,
                    functions,
                    ..
                } => {
                    if let Some(ws) = self
                        .workspaces
                        .get_mut(&id)
                        .filter(|ws| ws.database.as_deref() == Some(database.as_str()))
                    {
                        ws.objects.insert(schema, (tables, functions));
                        ws.loading = false;
                        ws.error = None;
                    }
                }
                Event::ColumnsLoaded {
                    database,
                    schema,
                    table,
                    columns,
                    ..
                } => {
                    if let Some(ws) = self
                        .workspaces
                        .get_mut(&id)
                        .filter(|ws| ws.database.as_deref() == Some(database.as_str()))
                    {
                        ws.columns.insert((schema, table), columns);
                    }
                }
                Event::StatsLoaded {
                    database,
                    schema,
                    table,
                    stats,
                    ..
                } => {
                    if let Some(tab) = self.data_tab_mut(&id, &database, &schema, &table) {
                        tab.stats = Some(stats);
                    }
                }
                Event::PageLoaded {
                    database,
                    schema,
                    table,
                    page,
                    rows,
                    ..
                } => {
                    // Só a página pedida por último: resposta atrasada de outra ordem ou offset é descartada.
                    if let Some(tab) = self
                        .data_tab_mut(&id, &database, &schema, &table)
                        .filter(|t| t.requested.as_ref() == Some(&page))
                    {
                        tab.page = page;
                        tab.requested = None;
                        tab.rows = Some(Arc::new(rows));
                        tab.loading = false;
                        tab.error = None;
                        tab.selected_rows.clear();
                        tab.anchor = None;
                    }
                }
                Event::DataFailed {
                    database,
                    schema,
                    table,
                    error,
                    ..
                } => {
                    // Página ou stats: o grid já carregado continua na tela, com o erro em cima. A página pedida

                    // fica: se a falha foi dos stats, a página em voo ainda chega; se foi dela, "Tentar de novo" a repete.
                    if let Some(tab) = self.data_tab_mut(&id, &database, &schema, &table) {
                        tab.loading = false;
                        tab.error = Some(error.to_string());
                    }
                }
                // FEAT-007 passo 4: recarga da página e dos stats, e a caixa de Commit com o erro.
                Event::ChangesApplied { .. } | Event::ChangesFailed { .. } => {}
                Event::CatalogFailed {
                    database, error, ..
                } => {
                    // Só do banco selecionado, ou da tentativa de abrir o primeiro.
                    if let Some(ws) = self
                        .workspaces
                        .get_mut(&id)
                        .filter(|ws| ws.database.as_deref().is_none_or(|d| d == database))
                    {
                        ws.loading = false;
                        ws.error = Some(if ws.database.is_none() {
                            format!("{database}: {error}")
                        } else {
                            error.to_string()
                        });
                    }
                }
            }
        }
    }

    fn data_tab_mut(
        &mut self,
        id: &str,
        database: &str,
        schema: &str,
        table: &str,
    ) -> Option<&mut DataTab> {
        self.workspaces
            .get_mut(id)
            .filter(|ws| ws.database.as_deref() == Some(database))
            .and_then(|ws| ws.data.as_mut())
            .filter(|tab| tab.schema == schema && tab.table == table)
    }

    /// Abre (ou troca) a tabela da aba Data e pede a primeira página e os tamanhos.
    pub(crate) fn open_table(&mut self, id: &ConnectionId, schema: &str, table: &str) {
        let Some(database) = self.workspaces.get(id).and_then(|ws| ws.database.clone()) else {
            return;
        };
        let tab = DataTab::new(schema, table);
        let page = tab.page.clone();
        if let Some(ws) = self.workspaces.get_mut(id) {
            ws.data = Some(tab);
        }
        self.deps
            .sessions
            .load_page(id, &database, schema, table, page);
        self.deps.sessions.load_stats(id, &database, schema, table);
    }

    /// Recarrega a aba Data com a página pedida (ordem, limite, offset).
    pub(crate) fn load_data_page(&mut self, id: &ConnectionId, page: Page) {
        let Some(ws) = self.workspaces.get_mut(id) else {
            return;
        };
        let Some(database) = ws.database.clone() else {
            return;
        };
        let Some(tab) = ws.data.as_mut() else { return };
        tab.requested = Some(page.clone());
        tab.loading = true;
        tab.error = None;
        tab.selected_rows.clear();
        tab.anchor = None;
        let (schema, table) = (tab.schema.clone(), tab.table.clone());
        self.deps
            .sessions
            .load_page(id, &database, &schema, &table, page);
    }

    /// Repete a página pedida (ou a atual) e, se os tamanhos ainda não vieram, os stats.
    pub(crate) fn retry_data(&mut self, id: &ConnectionId) {
        let Some(ws) = self.workspaces.get(id) else {
            return;
        };
        let Some(database) = ws.database.clone() else {
            return;
        };
        let Some(tab) = ws.data.as_ref() else { return };
        let (schema, table, stats_missing) =
            (tab.schema.clone(), tab.table.clone(), tab.stats.is_none());
        let page = tab.requested.clone().unwrap_or_else(|| tab.page.clone());
        self.load_data_page(id, page);
        if stats_missing {
            self.deps
                .sessions
                .load_stats(id, &database, &schema, &table);
        }
    }

    /// Clique numa linha do grid: simples seleciona só ela; ⌘ (Ctrl fora do macOS) alterna; ⇧ estende
    /// da última clicada. Só a primeira seleção (de nenhuma linha para alguma) abre o painel: depois, o
    /// botão "Painel" manda, e o clique numa linha não o reabre.
    pub(crate) fn select_row(
        &mut self,
        id: &ConnectionId,
        index: usize,
        modifiers: egui::Modifiers,
    ) {
        let Some(tab) = self.workspaces.get_mut(id).and_then(|ws| ws.data.as_mut()) else {
            return;
        };
        let was_empty = tab.selected_rows.is_empty();
        if modifiers.shift {
            // Sem âncora (primeiro clique já com ⇧), a linha clicada vira a âncora dos próximos.
            let from = *tab.anchor.get_or_insert(index);
            let (lo, hi) = if from <= index {
                (from, index)
            } else {
                (index, from)
            };
            tab.selected_rows.extend(lo..=hi);
        } else if modifiers.command {
            if !tab.selected_rows.remove(&index) {
                tab.selected_rows.insert(index);
            }
            tab.anchor = Some(index);
        } else {
            let only_this = tab.selected_rows.len() == 1 && tab.selected_rows.contains(&index);
            tab.selected_rows.clear();
            if !only_this {
                tab.selected_rows.insert(index);
            }
            tab.anchor = Some(index);
        }
        if was_empty && !tab.selected_rows.is_empty() {
            self.side_panel_open = true;
        }
    }

    pub(crate) fn status_of(&self, id: &str) -> Status {
        self.status.get(id).cloned().unwrap_or(Status::Idle)
    }

    /// O conteúdo da janela, separado do `eframe::App` para ser testável. O `ui` raiz do eframe
    /// vem sem margem nem fundo; cada tela abre seus painéis.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.drain_events();
        match self.view.clone() {
            View::Connections => crate::connections::show(self, ui),
            View::Workspace { connection } => crate::workspace::show(self, ui, &connection),
        }
    }
}

impl<D> eframe::App for App<D>
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui(ui);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::{Duration, Instant};

    use hy_dbplus_engine::{Connection, ConnectionConfig, MemorySecrets, Secret, Tag};

    use super::*;
    use crate::connections::ConnectionForm;

    pub(crate) fn deps<D: Driver + 'static>(driver: D, rt: &tokio::runtime::Runtime) -> Deps<D>
    where
        D::Conn: 'static,
    {
        let path = std::env::temp_dir()
            .join(format!("hy-dbplus-ui-{}", uuid_like()))
            .join("connections.toml");
        let store = ConnectionStore::load(path).unwrap();
        let (sessions, events) = Sessions::new(driver, rt.handle().clone(), || {});
        Deps {
            store,
            secrets: Box::new(MemorySecrets::default()),
            sessions,
            events,
        }
    }

    fn uuid_like() -> String {
        Connection::new("x", Tag::Local, ConnectionConfig::default()).id
    }

    /// Cadeia inteira sem renderizar: formulário → store + senha → conectar → evento → tela 3.2.
    /// Precisa do PostgreSQL local (`HY_DBPLUS_TEST_PG`: host, porta, usuário, banco e, se houver, a senha).
    #[test]
    fn criar_conectar_e_abrir_a_conexao() {
        let Some(url) = std::env::var_os("HY_DBPLUS_TEST_PG") else {
            return;
        };
        let pg = tokio_postgres_config(url);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = App::with_deps(deps(PostgresDriver::default(), &rt));
        let mut form = ConnectionForm::new();
        form.name = "pg-localhost".into();
        form.host = pg.0;
        form.port = pg.1.to_string();
        form.user = pg.2;
        form.database = pg.3;
        form.password = pg.4;
        let c = form.build().unwrap();
        app.deps.store.upsert(c.clone());
        app.deps.store.save().unwrap();
        if !form.password.is_empty() {
            app.deps
                .secrets
                .set(&c.id, &Secret::new(form.password.clone()))
                .unwrap();
        }
        let text = std::fs::read_to_string(app.deps.store.path()).unwrap();
        assert!(
            !text.contains(&form.password) || form.password.is_empty(),
            "senha no TOML"
        );
        crate::connections::connect(&mut app, &c);
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(10) {
            app.drain_events();
            if matches!(
                app.status_of(&c.id),
                Status::Connected { .. } | Status::Failed(_)
            ) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let status = app.status_of(&c.id);
        assert!(
            matches!(&status, Status::Connected { databases, .. } if !databases.is_empty()),
            "{status:?}"
        );
        assert_eq!(
            app.view,
            View::Workspace {
                connection: c.id.clone()
            }
        );
        app.deps.sessions.disconnect(&c.id);
        app.drain_events();
        assert_eq!(app.view, View::Connections);
        assert_eq!(app.status_of(&c.id), Status::Idle);
        let _ = std::fs::remove_dir_all(app.deps.store.path().parent().unwrap());
    }

    /// (host, porta, usuário, banco, senha) de uma URL do libpq, sem depender do tokio-postgres aqui.
    fn tokio_postgres_config(url: std::ffi::OsString) -> (String, u16, String, String, String) {
        let url = url.to_string_lossy().into_owned();
        let rest = url
            .trim_start_matches("postgres://")
            .trim_start_matches("postgresql://");
        let (auth, hostdb) = rest.split_once('@').unwrap_or(("postgres", rest));
        let (user, password) = auth.split_once(':').unwrap_or((auth, ""));
        let (hostport, db) = hostdb.split_once('/').unwrap_or((hostdb, "postgres"));
        let (host, port) = hostport.split_once(':').unwrap_or((hostport, "5432"));
        (
            host.into(),
            port.parse().unwrap_or(5432),
            user.into(),
            db.into(),
            password.into(),
        )
    }
}
