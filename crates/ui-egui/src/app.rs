//! O app egui: estado, roteamento entre as telas e leitura dos eventos do engine a cada frame.
//! Genérico no driver para o teste de interação rodar com um engine falso; o binário usa
//! [`PgApp`].

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Receiver;

use hy_dbplus_engine::{
    Column, ConnectionId, ConnectionStore, DatabaseInfo, Driver, Event, Function, PostgresDriver,
    Schema, SecretStore, Sessions, Table,
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
