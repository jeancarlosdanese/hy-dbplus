//! Sessões abertas e os eventos para a UI. Conectar roda no runtime `tokio` do app; a UI recebe
//! [`Event`]s por um canal síncrono que lê a cada frame, e um `repaint` avisa que há evento novo.
//!
//! Cada conexão tem uma geração: `connect` e `disconnect` a incrementam, e uma tentativa em voo
//! só vira sessão se a geração ainda for a dela. Assim "desconectar" durante "conectando" não
//! deixa a sessão abrir depois, e reconectar derruba a sessão antiga antes.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use hy_dbplus_driver::{Connection as DriverConnection, DatabaseInfo, Driver, DriverError, Secret};

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
}

impl Event {
    pub fn connection(&self) -> &ConnectionId {
        match self {
            Event::Connecting { connection }
            | Event::Connected { connection, .. }
            | Event::Failed { connection, .. }
            | Event::Disconnected { connection } => connection,
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

/// O estado de uma conexão: a geração corrente e a sessão aberta, se houver.
struct Slot<C> {
    generation: u64,
    session: Option<Session<C>>,
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
            let slot = map.entry(id.clone()).or_insert(Slot {
                generation: 0,
                session: None,
            });
            slot.generation += 1;
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
            let event = match result {
                Ok(session) => settle(&slots, &id, generation, session),
                Err(error) if is_current(&slots, &id, generation) => Some(Event::Failed {
                    connection: id,
                    error,
                }),
                Err(_) => None, // tentativa superada: o evento daquela geração já foi emitido
            };
            if let Some(event) = event {
                // Receptor fechado = UI já foi embora; nada a fazer.
                let _ = events.send(event);
                repaint();
            }
        });
    }

    /// Fecha a sessão (a conexão do driver cai quando o último `Arc` some) e invalida uma
    /// tentativa em voo. Avisa só se havia sessão aberta.
    pub fn disconnect(&self, id: &str) {
        let had_session = match self.slots.lock() {
            Ok(mut map) => match map.get_mut(id) {
                Some(slot) => {
                    slot.generation += 1;
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

    fn emit(&self, event: Event) {
        let _ = self.events.send(event);
        (self.repaint)();
    }
}

/// Guarda a sessão recém-aberta se a geração ainda é a dela; superada por um `disconnect` ou
/// outro `connect`, descarta em silêncio (o evento daquela geração já foi emitido).
fn settle<C>(
    slots: &Slots<C>,
    id: &ConnectionId,
    generation: u64,
    session: Session<C>,
) -> Option<Event> {
    let Ok(mut map) = slots.lock() else {
        return Some(Event::Failed {
            connection: id.clone(),
            error: DriverError::Closed,
        });
    };
    let slot = map.get_mut(id)?;
    if slot.generation != generation {
        return None;
    }
    let event = Event::Connected {
        connection: id.clone(),
        server_version: session.server_version.clone(),
        databases: session.databases.clone(),
    };
    slot.session = Some(session);
    Some(event)
}

/// Se a geração ainda é a da tentativa (lock envenenado conta como superada).
fn is_current<C>(slots: &Slots<C>, id: &str, generation: u64) -> bool {
    slots
        .lock()
        .ok()
        .and_then(|map| map.get(id).map(|slot| slot.generation == generation))
        .unwrap_or(false)
}

async fn open_session<D: Driver>(
    driver: &D,
    id: ConnectionId,
    config: &hy_dbplus_driver::ConnectionConfig,
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
    use hy_dbplus_driver::ConnectionConfig;

    /// Driver falso: demora 20 ms; falha quando a senha é "errada".
    struct FakeDriver;
    struct FakeConn;

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
        async fn query_text(&self, _sql: &str) -> Result<hy_dbplus_driver::TextRows, DriverError> {
            Ok(hy_dbplus_driver::TextRows::default())
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
            Ok(FakeConn)
        }
    }

    fn setup() -> (
        tokio::runtime::Runtime,
        Sessions<FakeDriver>,
        Receiver<Event>,
        Arc<AtomicUsize>,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let repaints = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&repaints);
        let (sessions, rx) = Sessions::new(FakeDriver, rt.handle().clone(), move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        (rt, sessions, rx, repaints)
    }

    fn conn() -> Connection {
        Connection::new("pg-localhost", Tag::Local, ConnectionConfig::default())
    }

    fn next(rx: &Receiver<Event>) -> Event {
        rx.recv_timeout(Duration::from_secs(2)).unwrap()
    }

    #[test]
    fn conectar_emite_connecting_e_connected_sem_bloquear() {
        let (_rt, sessions, rx, repaints) = setup();
        let c = conn();
        let before = std::time::Instant::now();
        sessions.connect(&c, Secret::new("ok"));
        assert!(
            before.elapsed() < Duration::from_millis(15),
            "connect bloqueou"
        );
        assert_eq!(
            next(&rx),
            Event::Connecting {
                connection: c.id.clone()
            }
        );
        let connected = next(&rx);
        assert!(
            matches!(&connected, Event::Connected { connection, server_version, databases }
                if connection == &c.id && server_version == "15.15" && databases.len() == 2),
            "{connected:?}"
        );
        assert!(sessions.is_open(&c.id));
        assert_eq!(sessions.session(&c.id).unwrap().databases[1].name, "loja");
        assert!(repaints.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn senha_errada_emite_failed_sem_a_senha_e_nao_abre_sessao() {
        let (_rt, sessions, rx, _) = setup();
        let c = conn();
        sessions.connect(&c, Secret::new("errada"));
        let _ = next(&rx);
        let failed = next(&rx);
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
        assert!(!sessions.is_open(&c.id));
    }

    #[test]
    fn desconectar_fecha_e_avisa_so_se_estava_aberta() {
        let (_rt, sessions, rx, _) = setup();
        let c = conn();
        sessions.disconnect(&c.id);
        assert!(rx.try_recv().is_err());
        sessions.connect(&c, Secret::new("ok"));
        let _ = next(&rx);
        let _ = next(&rx);
        sessions.disconnect(&c.id);
        assert_eq!(
            next(&rx),
            Event::Disconnected {
                connection: c.id.clone()
            }
        );
        assert!(!sessions.is_open(&c.id));
    }

    #[test]
    fn desconectar_durante_o_conectando_nao_deixa_a_sessao_abrir_depois() {
        // Antes: a task terminava, inseria a sessão e emitia Connected depois do disconnect.
        let (_rt, sessions, rx, _) = setup();
        let c = conn();
        sessions.connect(&c, Secret::new("ok"));
        assert_eq!(
            next(&rx),
            Event::Connecting {
                connection: c.id.clone()
            }
        );
        sessions.disconnect(&c.id);
        std::thread::sleep(Duration::from_millis(80));
        assert!(rx.try_recv().is_err(), "nenhum evento depois do disconnect");
        assert!(!sessions.is_open(&c.id));
    }

    #[test]
    fn falha_de_tentativa_superada_nao_chega_a_ui() {
        // Antes: connect (gen 1) com senha errada, disconnect (gen 2), e o Failed da gen 1 chegava.
        let (_rt, sessions, rx, _) = setup();
        let c = conn();
        sessions.connect(&c, Secret::new("errada"));
        assert_eq!(
            next(&rx),
            Event::Connecting {
                connection: c.id.clone()
            }
        );
        sessions.disconnect(&c.id);
        std::thread::sleep(Duration::from_millis(80));
        assert!(rx.try_recv().is_err(), "Failed fora de ordem");
    }

    #[test]
    fn reconectar_com_senha_errada_derruba_a_sessao_antiga() {
        // Antes: Failed era emitido com a sessão antiga ainda aberta por baixo.
        let (_rt, sessions, rx, _) = setup();
        let c = conn();
        sessions.connect(&c, Secret::new("ok"));
        let _ = next(&rx);
        let _ = next(&rx);
        assert!(sessions.is_open(&c.id));
        sessions.connect(&c, Secret::new("errada"));
        assert_eq!(
            next(&rx),
            Event::Disconnected {
                connection: c.id.clone()
            }
        );
        assert_eq!(
            next(&rx),
            Event::Connecting {
                connection: c.id.clone()
            }
        );
        assert!(matches!(next(&rx), Event::Failed { .. }));
        assert!(!sessions.is_open(&c.id));
    }
}
