//! Sessões abertas e os eventos para a UI. Conectar roda no runtime `tokio` do app; a UI recebe
//! [`Event`]s por um canal síncrono que lê a cada frame, e um `repaint` avisa que há evento novo.
//!
//! Cada conexão tem uma geração: `connect` e `disconnect` a incrementam, e uma tentativa em voo
//! só vira sessão se a geração ainda for a dela. Assim "desconectar" durante "conectando" não
//! deixa a sessão abrir depois, e reconectar derruba a sessão antiga antes.
//!
//! Banco selecionado (M2): no PostgreSQL cada banco é uma conexão própria, aberta com a mesma
//! configuração e a senha guardada em memória na sessão. O catálogo é lido por demanda (schemas ao
//! selecionar; tabelas e funções por schema; colunas e stats por tabela) e fica em cache até `refresh`.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use hy_dbplus_catalog::{
    self as catalog, Column, Direction, Function, Order, Schema, Table, TableStats,
};
use hy_dbplus_driver::{
    Connection as DriverConnection, ConnectionConfig, DatabaseInfo, Driver, DriverError, Secret,
    TextRows,
};

use crate::store::{Connection, ConnectionId};

/// O que o engine conta à UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Connecting {
        connection: ConnectionId,
    },
    Connected {
        connection: ConnectionId,
        server_version: String,
        databases: Vec<DatabaseInfo>,
    },
    Failed {
        connection: ConnectionId,
        error: DriverError,
    },
    Disconnected {
        connection: ConnectionId,
    },
    /// Banco aberto: os schemas e o schema inicial (`public` se existir, senão o primeiro).
    DatabaseSelected {
        connection: ConnectionId,
        database: String,
        schemas: Vec<Schema>,
        current_schema: Option<String>,
    },
    SchemaLoaded {
        connection: ConnectionId,
        database: String,
        schema: String,
        tables: Vec<Table>,
        functions: Vec<Function>,
    },
    ColumnsLoaded {
        connection: ConnectionId,
        database: String,
        schema: String,
        table: String,
        columns: Vec<Column>,
    },
    /// Falha ao abrir o banco ou ao ler o catálogo; a sessão do servidor continua aberta.
    CatalogFailed {
        connection: ConnectionId,
        database: String,
        error: DriverError,
    },
    /// Tamanhos, contagem e comentário de uma tabela (aba Data).
    StatsLoaded {
        connection: ConnectionId,
        database: String,
        schema: String,
        table: String,
        stats: TableStats,
    },
    /// Uma página de linhas de uma tabela e a página pedida (o total vem dos `stats`).
    PageLoaded {
        connection: ConnectionId,
        database: String,
        schema: String,
        table: String,
        page: Page,
        rows: TextRows,
    },
    /// Falha de stats ou de página de uma tabela (a árvore e a sessão continuam como estavam).
    DataFailed {
        connection: ConnectionId,
        database: String,
        schema: String,
        table: String,
        error: DriverError,
    },
}

/// O que uma página pede: ordenação opcional, tamanho e deslocamento (`LIMIT/OFFSET`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub order: Option<Order>,
    pub limit: u32,
    pub offset: u64,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            order: None,
            limit: 100,
            offset: 0,
        }
    }
}

impl Event {
    pub fn connection(&self) -> &ConnectionId {
        match self {
            Event::Connecting { connection }
            | Event::Connected { connection, .. }
            | Event::Failed { connection, .. }
            | Event::Disconnected { connection }
            | Event::DatabaseSelected { connection, .. }
            | Event::SchemaLoaded { connection, .. }
            | Event::ColumnsLoaded { connection, .. }
            | Event::CatalogFailed { connection, .. }
            | Event::StatsLoaded { connection, .. }
            | Event::PageLoaded { connection, .. }
            | Event::DataFailed { connection, .. } => connection,
        }
    }
}

/// Uma sessão aberta: a conexão do driver e o que já se sabe do servidor.
pub struct Session<C> {
    pub connection: ConnectionId,
    pub server_version: String,
    pub databases: Vec<DatabaseInfo>,
    pub conn: Arc<C>,
}

// À mão: o derive exigiria `C: Clone`/`C: Debug`, e a conexão do driver não é nenhum dos dois.
impl<C> Clone for Session<C> {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            server_version: self.server_version.clone(),
            databases: self.databases.clone(),
            conn: Arc::clone(&self.conn),
        }
    }
}

impl<C> std::fmt::Debug for Session<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("connection", &self.connection)
            .field("server_version", &self.server_version)
            .field("databases", &self.databases.len())
            .finish_non_exhaustive()
    }
}

/// Um banco aberto dentro da sessão: a conexão dele e o catálogo em cache.
struct Database<C> {
    conn: Arc<C>,
    schemas: Vec<Schema>,
    current_schema: Option<String>,
    objects: HashMap<String, (Vec<Table>, Vec<Function>)>,
    columns: HashMap<(String, String), Vec<Column>>,
    stats: HashMap<(String, String), TableStats>,
}

/// O estado de uma conexão: a geração corrente, a sessão aberta, a senha (só em memória, para
/// abrir outros bancos) e os bancos abertos.
struct Slot<C> {
    generation: u64,
    session: Option<Session<C>>,
    config: ConnectionConfig,
    secret: Secret,
    databases: HashMap<String, Database<C>>,
}

type Slots<C> = Arc<Mutex<HashMap<ConnectionId, Slot<C>>>>;

/// Todas as sessões, mais o driver, o runtime e o canal de eventos.
pub struct Sessions<D: Driver> {
    driver: Arc<D>,
    runtime: tokio::runtime::Handle,
    events: Sender<Event>,
    repaint: Arc<dyn Fn() + Send + Sync>,
    slots: Slots<D::Conn>,
}

impl<D> Sessions<D>
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    /// `repaint` é chamado depois de cada evento (o app passa `egui::Context::request_repaint`).
    /// O `Runtime` dono do `Handle` precisa viver mais que este valor.
    pub fn new(
        driver: D,
        runtime: tokio::runtime::Handle,
        repaint: impl Fn() + Send + Sync + 'static,
    ) -> (Self, Receiver<Event>) {
        let (tx, rx) = mpsc::channel();
        let sessions = Self {
            driver: Arc::new(driver),
            runtime,
            events: tx,
            repaint: Arc::new(repaint),
            slots: Arc::new(Mutex::new(HashMap::new())),
        };
        (sessions, rx)
    }

    /// Abre a sessão em segundo plano: `Connecting` agora, `Connected` ou `Failed` depois.
    /// Nunca bloqueia. Sobre uma sessão aberta, derruba a antiga (`Disconnected`) antes.
    pub fn connect(&self, connection: &Connection, password: Secret) {
        let id = connection.id.clone();
        let generation = {
            let Ok(mut map) = self.slots.lock() else {
                self.emit(Event::Failed {
                    connection: id,
                    error: DriverError::Closed,
                });
                return;
            };
            let slot = map.entry(id.clone()).or_insert_with(|| Slot {
                generation: 0,
                session: None,
                config: connection.config.clone(),
                secret: Secret::default(),
                databases: HashMap::new(),
            });
            slot.generation += 1;
            slot.config = connection.config.clone();
            slot.secret = password.clone();
            slot.databases.clear();
            let had_session = slot.session.take().is_some();
            let generation = slot.generation;
            drop(map);
            if had_session {
                self.emit(Event::Disconnected {
                    connection: id.clone(),
                });
            }
            generation
        };
        self.emit(Event::Connecting {
            connection: id.clone(),
        });
        let config = connection.config.clone();
        let driver = Arc::clone(&self.driver);
        let events = self.events.clone();
        let repaint = Arc::clone(&self.repaint);
        let slots = Arc::clone(&self.slots);
        self.runtime.spawn(async move {
            let result = open_session(&*driver, id.clone(), &config, &password).await;
            let sent = match result {
                Ok(session) => commit_if_current(&slots, &events, &id, generation, |slot| {
                    let event = Event::Connected {
                        connection: id.clone(),
                        server_version: session.server_version.clone(),
                        databases: session.databases.clone(),
                    };
                    slot.session = Some(session);
                    event
                }),
                Err(error) => {
                    commit_if_current(&slots, &events, &id, generation, |_| Event::Failed {
                        connection: id.clone(),
                        error,
                    })
                }
            };
            if sent {
                repaint();
            }
        });
    }

    /// Fecha a sessão (a conexão do driver cai quando o último `Arc` some), descarta a senha e
    /// os bancos abertos e invalida uma tentativa em voo. Avisa só se havia sessão aberta.
    pub fn disconnect(&self, id: &str) {
        let had_session = match self.slots.lock() {
            Ok(mut map) => match map.get_mut(id) {
                Some(slot) => {
                    slot.generation += 1;
                    slot.secret = Secret::default();
                    slot.databases.clear();
                    slot.session.take().is_some()
                }
                None => false,
            },
            Err(_) => false,
        };
        if had_session {
            self.emit(Event::Disconnected {
                connection: id.to_owned(),
            });
        }
    }

    pub fn session(&self, id: &str) -> Option<Session<D::Conn>> {
        self.slots.lock().ok()?.get(id)?.session.clone()
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.session(id).is_some()
    }

    /// Abre o banco (conexão própria) e lê os schemas; `DatabaseSelected` ou `CatalogFailed`.
    /// Banco já aberto responde do cache, na hora.
    pub fn select_database(&self, id: &str, database: &str) {
        let (generation, config, secret) = {
            let Ok(map) = self.slots.lock() else { return };
            let Some(slot) = map.get(id).filter(|s| s.session.is_some()) else {
                return;
            };
            if let Some(db) = slot.databases.get(database) {
                let event = Event::DatabaseSelected {
                    connection: id.to_owned(),
                    database: database.to_owned(),
                    schemas: db.schemas.clone(),
                    current_schema: db.current_schema.clone(),
                };
                drop(map);
                self.emit(event);
                return;
            }
            let config = ConnectionConfig {
                database: database.to_owned(),
                ..slot.config.clone()
            };
            (slot.generation, config, slot.secret.clone())
        };
        let id = id.to_owned();
        let database = database.to_owned();
        let driver = Arc::clone(&self.driver);
        let events = self.events.clone();
        let repaint = Arc::clone(&self.repaint);
        let slots = Arc::clone(&self.slots);
        self.runtime.spawn(async move {
            let result = async {
                let conn = driver.connect(&config, &secret).await?;
                let schemas = catalog::schemas(&conn).await?;
                Ok::<_, DriverError>((conn, schemas))
            }
            .await;
            let sent = match result {
                Ok((conn, schemas)) => {
                    let current_schema = schemas
                        .iter()
                        .find(|s| s.name == "public")
                        .or_else(|| schemas.first())
                        .map(|s| s.name.clone());
                    commit_if_current(&slots, &events, &id, generation, |slot| {
                        slot.databases.insert(
                            database.clone(),
                            Database {
                                conn: Arc::new(conn),
                                schemas: schemas.clone(),
                                current_schema: current_schema.clone(),
                                objects: HashMap::new(),
                                columns: HashMap::new(),
                                stats: HashMap::new(),
                            },
                        );
                        Event::DatabaseSelected {
                            connection: id.clone(),
                            database: database.clone(),
                            schemas,
                            current_schema,
                        }
                    })
                }
                Err(error) => {
                    commit_if_current(&slots, &events, &id, generation, |_| Event::CatalogFailed {
                        connection: id.clone(),
                        database: database.clone(),
                        error,
                    })
                }
            };
            if sent {
                repaint();
            }
        });
    }

    /// Tabelas e funções do schema: do cache na hora, ou duas queries e `SchemaLoaded`. Só para um
    /// banco que recebeu `DatabaseSelected` e antes do `Disconnected`; fora disso, nada acontece.
    pub fn load_schema(&self, id: &str, database: &str, schema: &str) {
        let (generation, conn) = {
            let Ok(mut map) = self.slots.lock() else {
                return;
            };
            let Some(slot) = map.get_mut(id) else { return };
            let Some(db) = slot.databases.get_mut(database) else {
                return;
            };
            db.current_schema = Some(schema.to_owned());
            if let Some((tables, functions)) = db.objects.get(schema) {
                let event = Event::SchemaLoaded {
                    connection: id.to_owned(),
                    database: database.to_owned(),
                    schema: schema.to_owned(),
                    tables: tables.clone(),
                    functions: functions.clone(),
                };
                drop(map);
                self.emit(event);
                return;
            }
            (slot.generation, Arc::clone(&db.conn))
        };
        let (id, database, schema) = (id.to_owned(), database.to_owned(), schema.to_owned());
        let events = self.events.clone();
        let repaint = Arc::clone(&self.repaint);
        let slots = Arc::clone(&self.slots);
        self.runtime.spawn(async move {
            let result = async {
                let tables = catalog::tables(&*conn, &schema).await?;
                let functions = catalog::functions(&*conn, &schema).await?;
                Ok::<_, DriverError>((tables, functions))
            }
            .await;
            let sent = match result {
                Ok((tables, functions)) => {
                    commit_if_current(&slots, &events, &id, generation, |slot| {
                        if let Some(db) = slot.databases.get_mut(&database) {
                            db.objects
                                .insert(schema.clone(), (tables.clone(), functions.clone()));
                        }
                        Event::SchemaLoaded {
                            connection: id.clone(),
                            database: database.clone(),
                            schema: schema.clone(),
                            tables,
                            functions,
                        }
                    })
                }
                Err(error) => {
                    commit_if_current(&slots, &events, &id, generation, |_| Event::CatalogFailed {
                        connection: id.clone(),
                        database: database.clone(),
                        error,
                    })
                }
            };
            if sent {
                repaint();
            }
        });
    }

    /// Colunas de uma tabela: do cache na hora, ou uma query e `ColumnsLoaded`. Mesma
    /// pré-condição de `load_schema`.
    pub fn load_columns(&self, id: &str, database: &str, schema: &str, table: &str) {
        let (generation, conn) = {
            let Ok(map) = self.slots.lock() else { return };
            let Some(slot) = map.get(id) else { return };
            let Some(db) = slot.databases.get(database) else {
                return;
            };
            if let Some(columns) = db.columns.get(&(schema.to_owned(), table.to_owned())) {
                let event = Event::ColumnsLoaded {
                    connection: id.to_owned(),
                    database: database.to_owned(),
                    schema: schema.to_owned(),
                    table: table.to_owned(),
                    columns: columns.clone(),
                };
                drop(map);
                self.emit(event);
                return;
            }
            (slot.generation, Arc::clone(&db.conn))
        };
        let (id, database, schema, table) = (
            id.to_owned(),
            database.to_owned(),
            schema.to_owned(),
            table.to_owned(),
        );
        let events = self.events.clone();
        let repaint = Arc::clone(&self.repaint);
        let slots = Arc::clone(&self.slots);
        self.runtime.spawn(async move {
            let sent = match catalog::columns(&*conn, &schema, &table).await {
                Ok(columns) => commit_if_current(&slots, &events, &id, generation, |slot| {
                    if let Some(db) = slot.databases.get_mut(&database) {
                        db.columns
                            .insert((schema.clone(), table.clone()), columns.clone());
                    }
                    Event::ColumnsLoaded {
                        connection: id.clone(),
                        database: database.clone(),
                        schema: schema.clone(),
                        table: table.clone(),
                        columns,
                    }
                }),
                Err(error) => {
                    commit_if_current(&slots, &events, &id, generation, |_| Event::CatalogFailed {
                        connection: id.clone(),
                        database: database.clone(),
                        error,
                    })
                }
            };
            if sent {
                repaint();
            }
        });
    }

    /// Esvazia tabelas, funções, colunas e stats do cache do banco (a conexão e a lista de schemas ficam:
    /// schema criado fora do app aparece ao reselecionar o banco); a UI recarrega o que mostra.
    pub fn refresh(&self, id: &str, database: &str) {
        if let Ok(mut map) = self.slots.lock()
            && let Some(db) = map.get_mut(id).and_then(|s| s.databases.get_mut(database))
        {
            db.objects.clear();
            db.columns.clear();
            db.stats.clear();
        }
    }

    /// Tamanhos, contagem e comentário da tabela: do cache na hora, ou duas queries e `StatsLoaded`.
    /// Mesma pré-condição de `load_schema`.
    pub fn load_stats(&self, id: &str, database: &str, schema: &str, table: &str) {
        let (generation, conn) = {
            let Ok(map) = self.slots.lock() else { return };
            let Some(slot) = map.get(id) else { return };
            let Some(db) = slot.databases.get(database) else {
                return;
            };
            if let Some(stats) = db.stats.get(&(schema.to_owned(), table.to_owned())) {
                let event = Event::StatsLoaded {
                    connection: id.to_owned(),
                    database: database.to_owned(),
                    schema: schema.to_owned(),
                    table: table.to_owned(),
                    stats: stats.clone(),
                };
                drop(map);
                self.emit(event);
                return;
            }
            (slot.generation, Arc::clone(&db.conn))
        };
        let (id, database, schema, table) = (
            id.to_owned(),
            database.to_owned(),
            schema.to_owned(),
            table.to_owned(),
        );
        let events = self.events.clone();
        let repaint = Arc::clone(&self.repaint);
        let slots = Arc::clone(&self.slots);
        self.runtime.spawn(async move {
            let sent = match catalog::stats(&*conn, &schema, &table).await {
                Ok(stats) => commit_if_current(&slots, &events, &id, generation, |slot| {
                    if let Some(db) = slot.databases.get_mut(&database) {
                        db.stats
                            .insert((schema.clone(), table.clone()), stats.clone());
                    }
                    Event::StatsLoaded {
                        connection: id.clone(),
                        database: database.clone(),
                        schema: schema.clone(),
                        table: table.clone(),
                        stats,
                    }
                }),
                Err(error) => {
                    commit_if_current(&slots, &events, &id, generation, |_| Event::DataFailed {
                        connection: id.clone(),
                        database: database.clone(),
                        schema: schema.clone(),
                        table: table.clone(),
                        error,
                    })
                }
            };
            if sent {
                repaint();
            }
        });
    }

    /// Uma página de linhas (`LIMIT/OFFSET`), ordenada pela coluna pedida e, como desempate, pela
    /// chave primária: se as colunas da tabela ainda não estão em cache, a tarefa as lê antes da
    /// primeira página (uma query a mais, uma vez por tabela), para todas as páginas saírem sob a
    /// mesma ordem. Sem cache de página: `PageLoaded` a cada chamada; o total vem de `load_stats`.
    /// Mesma pré-condição de `load_schema`.
    pub fn load_page(&self, id: &str, database: &str, schema: &str, table: &str, page: Page) {
        let (generation, conn, cached_columns) = {
            let Ok(map) = self.slots.lock() else { return };
            let Some(slot) = map.get(id) else { return };
            let Some(db) = slot.databases.get(database) else {
                return;
            };
            (
                slot.generation,
                Arc::clone(&db.conn),
                db.columns
                    .get(&(schema.to_owned(), table.to_owned()))
                    .cloned(),
            )
        };
        let (id, database, schema, table) = (
            id.to_owned(),
            database.to_owned(),
            schema.to_owned(),
            table.to_owned(),
        );
        let events = self.events.clone();
        let repaint = Arc::clone(&self.repaint);
        let slots = Arc::clone(&self.slots);
        self.runtime.spawn(async move {
            let result = async {
                let fetched = match cached_columns {
                    Some(_) => None,
                    None => Some(catalog::columns(&*conn, &schema, &table).await?),
                };
                let columns = cached_columns
                    .as_deref()
                    .or(fetched.as_deref())
                    .unwrap_or_default();
                let order = order_with_primary_key(page.order.as_ref(), columns);
                let rows =
                    catalog::page(&*conn, &schema, &table, &order, page.limit, page.offset).await?;
                Ok::<_, DriverError>((fetched, rows))
            }
            .await;
            let sent = match result {
                Ok((fetched, rows)) => {
                    // Colunas lidas aqui vão ao cache e à UI (`ColumnsLoaded`), antes da página: quem abre a
                    // tabela recebe os tipos sem pedir à parte.
                    if let Some(columns) = fetched {
                        commit_if_current(&slots, &events, &id, generation, |slot| {
                            if let Some(db) = slot.databases.get_mut(&database) {
                                db.columns
                                    .entry((schema.clone(), table.clone()))
                                    .or_insert_with(|| columns.clone());
                            }
                            Event::ColumnsLoaded {
                                connection: id.clone(),
                                database: database.clone(),
                                schema: schema.clone(),
                                table: table.clone(),
                                columns,
                            }
                        });
                    }
                    commit_if_current(&slots, &events, &id, generation, |_| Event::PageLoaded {
                        connection: id.clone(),
                        database: database.clone(),
                        schema: schema.clone(),
                        table: table.clone(),
                        page,
                        rows,
                    })
                }
                Err(error) => {
                    commit_if_current(&slots, &events, &id, generation, |_| Event::DataFailed {
                        connection: id.clone(),
                        database: database.clone(),
                        schema: schema.clone(),
                        table: table.clone(),
                        error,
                    })
                }
            };
            if sent {
                repaint();
            }
        });
    }

    fn emit(&self, event: Event) {
        let _ = self.events.send(event);
        (self.repaint)();
    }
}

/// A coluna pedida e, depois dela, as colunas de chave primária que ainda não estão na lista.
fn order_with_primary_key(chosen: Option<&Order>, columns: &[Column]) -> Vec<Order> {
    let mut order: Vec<Order> = chosen.into_iter().cloned().collect();
    for pk in columns.iter().filter(|c| c.is_primary_key) {
        if order.iter().all(|o| o.column != pk.name) {
            order.push(Order {
                column: pk.name.clone(),
                direction: Direction::Asc,
            });
        }
    }
    order
}

/// Sob o lock: se a geração ainda é a da tentativa, aplica a mudança ao `Slot` e envia o evento
/// que ela devolve, antes de soltar o lock. Assim nenhum evento de uma tentativa chega depois
/// do `Disconnected` (ou do `Connecting` seguinte) que a superou: `disconnect` e `connect`
/// incrementam a geração com o mesmo lock. O `Sender` é ilimitado e não bloqueia. Tentativa
/// superada, ou lock envenenado, é descartada em silêncio: o evento daquela geração já saiu.
fn commit_if_current<C>(
    slots: &Slots<C>,
    events: &Sender<Event>,
    id: &str,
    generation: u64,
    apply: impl FnOnce(&mut Slot<C>) -> Event,
) -> bool {
    let Ok(mut map) = slots.lock() else {
        return false;
    };
    let Some(slot) = map.get_mut(id).filter(|s| s.generation == generation) else {
        return false;
    };
    let event = apply(slot);
    let _ = events.send(event);
    true
}

async fn open_session<D: Driver>(
    driver: &D,
    id: ConnectionId,
    config: &ConnectionConfig,
    password: &Secret,
) -> Result<Session<D::Conn>, DriverError> {
    let conn = driver.connect(config, password).await?;
    let server_version = conn.server_version().await?;
    let databases = conn.list_databases().await?;
    Ok(Session {
        connection: id,
        server_version,
        databases,
        conn: Arc::new(conn),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::*;
    use crate::store::Tag;
    use hy_dbplus_driver::TextRows;

    /// Driver falso: demora 20 ms; falha quando a senha é "errada"; conta as queries de catálogo
    /// e responde conforme a tabela de `pg_catalog` citada no SQL.
    #[derive(Default)]
    struct FakeDriver {
        queries: Arc<AtomicUsize>,
        last_sql: Arc<Mutex<String>>,
    }
    struct FakeConn {
        queries: Arc<AtomicUsize>,
        last_sql: Arc<Mutex<String>>,
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
            ])
        }
        async fn query_text(&self, sql: &str) -> Result<TextRows, DriverError> {
            self.queries.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut last) = self.last_sql.lock() {
                *last = sql.to_owned();
            }

            // Da mais específica para a mais geral: tabelas e funções também citam pg_namespace.
            let rows = if sql.contains("\"quebrada\"") {
                return Err(DriverError::Db(
                    "relation \"public.quebrada\" does not exist".into(),
                ));
            } else if sql.starts_with("SELECT count(*)") {
                vec![row(&[Some("3")])]
            } else if sql.contains("pg_total_relation_size") {
                vec![row(&[
                    Some("24 kB"),
                    Some("8192 bytes"),
                    Some("16 kB"),
                    Some("pedidos"),
                ])]
            } else if sql.starts_with("SELECT * FROM") {
                return Ok(TextRows {
                    columns: vec!["id".into(), "nome".into()],
                    rows: vec![row(&[Some("1"), Some("a")]), row(&[Some("2"), None])],
                });
            } else if sql.contains("pg_attribute") {
                vec![row(&[
                    Some("id"),
                    Some("integer"),
                    Some("f"),
                    None,
                    Some("1"),
                    Some("t"),
                ])]
            } else if sql.contains("pg_proc") {
                vec![row(&[
                    Some("dobro"),
                    Some("x integer"),
                    Some("integer"),
                    Some("f"),
                ])]
            } else if sql.contains("pg_class") {
                vec![
                    row(&[Some("pedido"), Some("r"), None]),
                    row(&[Some("v_pedido"), Some("v"), Some("resumo")]),
                ]
            } else if sql.contains("pg_namespace") {
                vec![row(&[Some("app")]), row(&[Some("public")])]
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
            password: &Secret,
        ) -> Result<FakeConn, DriverError> {
            tokio::time::sleep(Duration::from_millis(20)).await;
            if password.expose() == "errada" {
                return Err(DriverError::Auth {
                    user: config.user.clone(),
                    reason: "senha inválida".into(),
                });
            }
            if config.database == "inexistente" {
                return Err(DriverError::Db(
                    "database \"inexistente\" does not exist".into(),
                ));
            }
            Ok(FakeConn {
                queries: Arc::clone(&self.queries),
                last_sql: Arc::clone(&self.last_sql),
            })
        }
    }

    struct Harness {
        _rt: tokio::runtime::Runtime,
        sessions: Sessions<FakeDriver>,
        rx: Receiver<Event>,
        repaints: Arc<AtomicUsize>,
        queries: Arc<AtomicUsize>,
        last_sql: Arc<Mutex<String>>,
    }

    fn setup() -> Harness {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let repaints = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&repaints);
        let driver = FakeDriver::default();
        let queries = Arc::clone(&driver.queries);
        let last_sql = Arc::clone(&driver.last_sql);
        let (sessions, rx) = Sessions::new(driver, rt.handle().clone(), move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        Harness {
            _rt: rt,
            sessions,
            rx,
            repaints,
            queries,
            last_sql,
        }
    }

    fn conn() -> Connection {
        Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default())
    }

    fn next(rx: &Receiver<Event>) -> Event {
        rx.recv_timeout(Duration::from_secs(2)).unwrap()
    }

    /// Conecta e consome `Connecting` e `Connected`.
    fn connected(h: &Harness, c: &Connection) {
        h.sessions.connect(c, Secret::new("ok"));
        let _ = next(&h.rx);
        let _ = next(&h.rx);
        assert!(h.sessions.is_open(&c.id));
    }

    #[test]
    fn conectar_emite_connecting_e_connected_sem_bloquear() {
        let h = setup();
        let c = conn();
        let before = std::time::Instant::now();
        h.sessions.connect(&c, Secret::new("ok"));
        assert!(
            before.elapsed() < Duration::from_millis(15),
            "connect bloqueou"
        );
        assert_eq!(
            next(&h.rx),
            Event::Connecting {
                connection: c.id.clone()
            }
        );
        let connected = next(&h.rx);
        assert!(
            matches!(&connected, Event::Connected { connection, server_version, databases }
                if connection == &c.id && server_version == "15.15" && databases.len() == 2),
            "{connected:?}"
        );
        assert!(h.sessions.is_open(&c.id));
        assert_eq!(h.sessions.session(&c.id).unwrap().databases[1].name, "loja");
        assert!(h.repaints.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn senha_errada_emite_failed_sem_a_senha_e_nao_abre_sessao() {
        let h = setup();
        let c = conn();
        h.sessions.connect(&c, Secret::new("errada"));
        let _ = next(&h.rx);
        let failed = next(&h.rx);
        assert!(
            matches!(
                &failed,
                Event::Failed {
                    error: DriverError::Auth { .. },
                    ..
                }
            ),
            "{failed:?}"
        );
        assert!(!format!("{failed:?}").contains("errada"));
        assert!(!h.sessions.is_open(&c.id));
    }

    #[test]
    fn desconectar_fecha_e_avisa_so_se_estava_aberta() {
        let h = setup();
        let c = conn();
        h.sessions.disconnect(&c.id);
        assert!(h.rx.try_recv().is_err());
        connected(&h, &c);
        h.sessions.disconnect(&c.id);
        assert_eq!(
            next(&h.rx),
            Event::Disconnected {
                connection: c.id.clone()
            }
        );
        assert!(!h.sessions.is_open(&c.id));
    }

    #[test]
    fn desconectar_durante_o_conectando_nao_deixa_a_sessao_abrir_depois() {
        let h = setup();
        let c = conn();
        h.sessions.connect(&c, Secret::new("ok"));
        assert_eq!(
            next(&h.rx),
            Event::Connecting {
                connection: c.id.clone()
            }
        );
        h.sessions.disconnect(&c.id);
        std::thread::sleep(Duration::from_millis(80));
        assert!(
            h.rx.try_recv().is_err(),
            "nenhum evento depois do disconnect"
        );
        assert!(!h.sessions.is_open(&c.id));
    }

    #[test]
    fn falha_de_tentativa_superada_nao_chega_a_ui() {
        let h = setup();
        let c = conn();
        h.sessions.connect(&c, Secret::new("errada"));
        assert_eq!(
            next(&h.rx),
            Event::Connecting {
                connection: c.id.clone()
            }
        );
        h.sessions.disconnect(&c.id);
        std::thread::sleep(Duration::from_millis(80));
        assert!(h.rx.try_recv().is_err(), "Failed fora de ordem");
    }

    #[test]
    fn reconectar_com_senha_errada_derruba_a_sessao_antiga() {
        let h = setup();
        let c = conn();
        connected(&h, &c);
        h.sessions.connect(&c, Secret::new("errada"));
        assert_eq!(
            next(&h.rx),
            Event::Disconnected {
                connection: c.id.clone()
            }
        );
        assert_eq!(
            next(&h.rx),
            Event::Connecting {
                connection: c.id.clone()
            }
        );
        assert!(matches!(next(&h.rx), Event::Failed { .. }));
        assert!(!h.sessions.is_open(&c.id));
    }

    #[test]
    fn selecionar_banco_abre_a_conexao_le_os_schemas_e_escolhe_public() {
        let h = setup();
        let c = conn();
        connected(&h, &c);
        h.sessions.select_database(&c.id, "loja");
        let event = next(&h.rx);
        assert_eq!(
            event,
            Event::DatabaseSelected {
                connection: c.id.clone(),
                database: "loja".into(),
                schemas: vec![
                    Schema { name: "app".into() },
                    Schema {
                        name: "public".into()
                    }
                ],
                current_schema: Some("public".into()),
            }
        );
        assert_eq!(h.queries.load(Ordering::SeqCst), 1);
        // Segunda seleção do mesmo banco: do cache, sem query nem nova conexão.
        h.sessions.select_database(&c.id, "loja");
        assert!(matches!(next(&h.rx), Event::DatabaseSelected { .. }));
        assert_eq!(h.queries.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn banco_invalido_emite_catalog_failed_e_a_sessao_continua() {
        let h = setup();
        let c = conn();
        connected(&h, &c);
        h.sessions.select_database(&c.id, "inexistente");
        let event = next(&h.rx);
        assert!(
            matches!(&event, Event::CatalogFailed { database, error: DriverError::Db(_), .. } if database == "inexistente"),
            "{event:?}"
        );
        assert!(h.sessions.is_open(&c.id));
        // Sem sessão aberta, selecionar banco é ignorado.
        h.sessions.disconnect(&c.id);
        let _ = next(&h.rx);
        h.sessions.select_database(&c.id, "loja");
        assert!(h.rx.try_recv().is_err());
    }

    #[test]
    fn schema_e_colunas_vem_do_cache_ate_o_refresh() {
        let h = setup();
        let c = conn();
        connected(&h, &c);
        h.sessions.select_database(&c.id, "loja");
        let _ = next(&h.rx);
        h.sessions.load_schema(&c.id, "loja", "public");
        let event = next(&h.rx);
        let Event::SchemaLoaded {
            tables,
            functions,
            schema,
            ..
        } = &event
        else {
            panic!("{event:?}")
        };
        assert_eq!(schema, "public");
        assert_eq!(
            tables
                .iter()
                .map(|t| (t.name.as_str(), t.kind.is_view()))
                .collect::<Vec<_>>(),
            [("pedido", false), ("v_pedido", true)]
        );
        assert_eq!(functions[0].signature(), "dobro(x integer)");
        assert_eq!(
            h.queries.load(Ordering::SeqCst),
            3,
            "schemas + tables + functions"
        );
        h.sessions.load_schema(&c.id, "loja", "public");
        assert!(matches!(next(&h.rx), Event::SchemaLoaded { .. }));
        assert_eq!(h.queries.load(Ordering::SeqCst), 3, "cache");

        h.sessions.load_columns(&c.id, "loja", "public", "pedido");
        let event = next(&h.rx);
        let Event::ColumnsLoaded { columns, table, .. } = &event else {
            panic!("{event:?}")
        };
        assert_eq!(table, "pedido");
        assert!(columns[0].is_primary_key && columns[0].data_type == "integer");
        assert_eq!(h.queries.load(Ordering::SeqCst), 4);
        h.sessions.load_columns(&c.id, "loja", "public", "pedido");
        let _ = next(&h.rx);
        assert_eq!(h.queries.load(Ordering::SeqCst), 4, "cache");

        h.sessions.refresh(&c.id, "loja");
        h.sessions.load_schema(&c.id, "loja", "public");
        let _ = next(&h.rx);
        assert_eq!(h.queries.load(Ordering::SeqCst), 6, "refresh invalida");
    }

    #[test]
    fn a_senha_guardada_nunca_aparece_nos_eventos() {
        let h = setup();
        let c = conn();
        h.sessions.connect(&c, Secret::new("segredo-unico"));
        let _ = next(&h.rx);
        let _ = next(&h.rx);
        h.sessions.select_database(&c.id, "loja");
        let selected = next(&h.rx);
        h.sessions.load_schema(&c.id, "loja", "public");
        let loaded = next(&h.rx);
        for e in [selected, loaded] {
            assert!(!format!("{e:?}").contains("segredo-unico"));
        }
        h.sessions.disconnect(&c.id);
        let _ = next(&h.rx);
        // Depois do disconnect a senha sai da memória: selecionar banco não abre nada.
        h.sessions.select_database(&c.id, "loja");
        assert!(h.rx.try_recv().is_err());
    }

    #[test]
    fn stats_vem_do_cache_e_a_pagina_leva_ordem_limite_e_offset_ao_sql() {
        let h = setup();
        let c = conn();
        connected(&h, &c);
        h.sessions.select_database(&c.id, "loja");
        let _ = next(&h.rx);
        let before = h.queries.load(Ordering::SeqCst);
        h.sessions.load_stats(&c.id, "loja", "public", "pedido");
        let event = next(&h.rx);
        let Event::StatsLoaded { stats, table, .. } = &event else {
            panic!("{event:?}")
        };
        assert_eq!(table, "pedido");
        assert_eq!(
            (
                stats.row_count,
                stats.total_size.as_str(),
                stats.comment.as_deref()
            ),
            (3, "24 kB", Some("pedidos"))
        );
        assert_eq!(
            h.queries.load(Ordering::SeqCst),
            before + 2,
            "tamanhos + count"
        );
        h.sessions.load_stats(&c.id, "loja", "public", "pedido");
        assert!(matches!(next(&h.rx), Event::StatsLoaded { .. }));
        assert_eq!(h.queries.load(Ordering::SeqCst), before + 2, "cache");
        h.sessions.refresh(&c.id, "loja");
        h.sessions.load_stats(&c.id, "loja", "public", "pedido");
        let _ = next(&h.rx);
        assert_eq!(
            h.queries.load(Ordering::SeqCst),
            before + 4,
            "refresh invalida"
        );

        // Página: sem colunas em cache, a tarefa as lê antes (uma query) e a PK já entra no desempate.
        let page = Page {
            order: Some(Order {
                column: "nome".into(),
                direction: Direction::Desc,
            }),
            limit: 50,
            offset: 100,
        };
        let before = h.queries.load(Ordering::SeqCst);
        h.sessions
            .load_page(&c.id, "loja", "public", "pedido", page.clone());
        let event = next(&h.rx);
        assert!(
            matches!(event, Event::ColumnsLoaded { .. }),
            "colunas lidas pela página chegam à UI: {event:?}"
        );
        let event = next(&h.rx);
        let Event::PageLoaded {
            rows, page: got, ..
        } = &event
        else {
            panic!("{event:?}")
        };
        assert_eq!(got, &page);
        assert_eq!(rows.columns, ["id", "nome"]);
        assert_eq!(rows.cell(1, 1), None);
        assert_eq!(
            *h.last_sql.lock().unwrap(),
            "SELECT * FROM \"public\".\"pedido\" ORDER BY \"nome\" DESC, \"id\" ASC LIMIT 50 OFFSET 100"
        );
        assert_eq!(
            h.queries.load(Ordering::SeqCst),
            before + 2,
            "colunas + página"
        );
        // Segunda página: colunas do cache, só a página; e a árvore recebe as colunas sem nova query.
        h.sessions.load_page(
            &c.id,
            "loja",
            "public",
            "pedido",
            Page {
                offset: 150,
                ..page
            },
        );
        let _ = next(&h.rx);
        assert_eq!(h.queries.load(Ordering::SeqCst), before + 3);
        h.sessions.load_columns(&c.id, "loja", "public", "pedido");
        assert!(matches!(next(&h.rx), Event::ColumnsLoaded { .. }));
        assert_eq!(
            h.queries.load(Ordering::SeqCst),
            before + 3,
            "cache preenchido pela página"
        );
        // Sem banco aberto, load_page é ignorado.
        h.sessions
            .load_page(&c.id, "outro", "public", "pedido", Page::default());
        assert!(h.rx.try_recv().is_err());
    }

    #[test]
    fn falha_de_pagina_ou_stats_vem_como_data_failed_com_a_tabela() {
        let h = setup();
        let c = conn();
        connected(&h, &c);
        h.sessions.select_database(&c.id, "loja");
        let _ = next(&h.rx);
        h.sessions
            .load_page(&c.id, "loja", "public", "quebrada", Page::default());
        let event = next(&h.rx);
        assert!(
            matches!(&event, Event::DataFailed { table, error: DriverError::Db(_), .. } if table == "quebrada"),
            "{event:?}"
        );
        h.sessions.load_stats(&c.id, "loja", "public", "quebrada");
        let event = next(&h.rx);
        assert!(
            matches!(&event, Event::DataFailed { table, error: DriverError::Db(_), .. } if table == "quebrada"),
            "{event:?}"
        );
        assert!(h.sessions.is_open(&c.id));
    }
}
