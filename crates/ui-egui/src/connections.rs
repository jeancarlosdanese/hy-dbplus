//! Tela 3.1: lista de conexões (tag, nome, `host : database`), busca, `+`, formulário de criar
//! e editar, remover com confirmação, conectar.

use hy_dbplus_engine::{Connection, ConnectionConfig, ConnectionId, Driver, Secret, Tag, TlsMode};

use crate::app::{App, Status};
use crate::theme;

/// Estado da lista entre frames.
#[derive(Debug, Default)]
pub struct Connections {
    pub search: String,
    pub selected: Option<ConnectionId>,
    pub confirm_remove: Option<ConnectionId>,
}

/// O formulário. Sem `Debug`: carrega a senha digitada até salvar.
pub struct ConnectionForm {
    /// `None` = nova conexão; `Some` = edição (o id não muda).
    pub id: Option<ConnectionId>,
    pub name: String,
    pub tag: Tag,
    pub host: String,
    pub port: String,
    pub user: String,
    pub database: String,
    pub password: String,
    pub tls: TlsMode,
    pub error: Option<String>,
}

impl ConnectionForm {
    pub fn new() -> Self {
        let d = ConnectionConfig::default();
        Self {
            id: None,
            name: String::new(),
            tag: Tag::Local,
            host: d.host,
            port: d.port.to_string(),
            user: d.user,
            database: d.database,
            password: String::new(),
            tls: d.tls,
            error: None,
        }
    }

    pub fn edit(c: &Connection) -> Self {
        Self {
            id: Some(c.id.clone()),
            name: c.name.clone(),
            tag: c.tag,
            host: c.config.host.clone(),
            port: c.config.port.to_string(),
            user: c.config.user.clone(),
            database: c.config.database.clone(),
            password: String::new(),
            tls: c.config.tls,
            error: None,
        }
    }

    /// A conexão pronta para guardar, ou o que está errado.
    pub fn build(&self) -> Result<Connection, String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("dê um nome à conexão".into());
        }
        if self.host.trim().is_empty() {
            return Err("informe o host".into());
        }
        let port: u16 = self
            .port
            .trim()
            .parse()
            .map_err(|_| "porta inválida (1 a 65535)".to_owned())?;
        if port == 0 {
            return Err("porta inválida (1 a 65535)".into());
        }
        if self.user.trim().is_empty() {
            return Err("informe o usuário".into());
        }
        let config = ConnectionConfig {
            host: self.host.trim().into(),
            port,
            user: self.user.trim().into(),
            database: if self.database.trim().is_empty() {
                "postgres".into()
            } else {
                self.database.trim().into()
            },
            tls: self.tls,
        };
        Ok(match &self.id {
            Some(id) => Connection {
                id: id.clone(),
                name: name.into(),
                tag: self.tag,
                config,
            },
            None => Connection::new(name, self.tag, config),
        })
    }
}

impl Default for ConnectionForm {
    fn default() -> Self {
        Self::new()
    }
}

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    egui::Panel::top("connections-top").show(ui, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Conexões").size(18.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("＋ Nova").clicked() {
                    app.form = Some(ConnectionForm::new());
                }
                ui.add(
                    egui::TextEdit::singleline(&mut app.connections.search)
                        .hint_text("Buscar")
                        .desired_width(220.0),
                );
            });
        });
        ui.add_space(6.0);
    });
    if let Some(notice) = app.notice.clone() {
        egui::Panel::bottom("connections-notice").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(theme::error_color(), notice);
                if ui.small_button("fechar").clicked() {
                    app.notice = None;
                }
            });
        });
    }
    egui::CentralPanel::default_margins().show(ui, |ui| {
        let search = app.connections.search.trim().to_lowercase();
        let visible: Vec<Connection> = app
            .deps
            .store
            .list()
            .iter()
            .filter(|c| {
                search.is_empty()
                    || c.name.to_lowercase().contains(&search)
                    || c.subtitle().to_lowercase().contains(&search)
            })
            .cloned()
            .collect();
        if app.deps.store.list().is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Nenhuma conexão ainda. Crie a primeira em ＋ Nova.")
                        .color(tokens.text_muted),
                );
            });
            return;
        }
        if visible.is_empty() {
            ui.label(egui::RichText::new("Nada com esse nome.").color(tokens.text_muted));
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for c in &visible {
                row(app, ui, c);
            }
        });
    });
    form_window(app, ui.ctx());
    confirm_remove_window(app, ui.ctx());
}

fn row<D>(app: &mut App<D>, ui: &mut egui::Ui, c: &Connection)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let status = app.status_of(&c.id);
    let selected = app.connections.selected.as_deref() == Some(c.id.as_str());
    let fill = if selected {
        tokens.accent_soft
    } else {
        tokens.card
    };
    egui::Frame::new()
        .fill(fill)
        .corner_radius(tokens.radius)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 36.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 4.0, theme::tag_color(c.tag));
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&c.name).strong());
                        ui.label(
                            egui::RichText::new(c.tag.label())
                                .small()
                                .color(theme::tag_color(c.tag)),
                        );
                    });
                    ui.label(
                        egui::RichText::new(c.subtitle())
                            .small()
                            .color(tokens.text_muted),
                    );
                    match &status {
                        Status::Failed(e) => {
                            ui.label(egui::RichText::new(e).small().color(theme::error_color()));
                        }
                        Status::Connecting => {
                            ui.label(
                                egui::RichText::new("conectando…")
                                    .small()
                                    .color(tokens.text_muted),
                            );
                        }
                        _ => {}
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    match status {
                        Status::Connecting => {
                            ui.add_enabled(false, egui::Button::new("Conectar"));
                        }
                        Status::Connected { .. } => {
                            if ui.button("Abrir").clicked() {
                                app.focus = Some(c.id.clone());
                                app.active = app.key_of(&c.id);
                                app.view = crate::app::View::Workspace;
                            }
                        }
                        _ => {
                            if ui.button("Conectar").clicked() {
                                connect(app, c);
                            }
                        }
                    }
                    if ui.button("Editar").clicked() {
                        app.form = Some(ConnectionForm::edit(c));
                    }
                    if ui.button("Remover").clicked() {
                        app.connections.confirm_remove = Some(c.id.clone());
                    }
                });
            });
        });
    ui.add_space(6.0);
}

/// Conecta com a senha do chaveiro (ou vazia, para autenticação `trust`).
pub fn connect<D>(app: &mut App<D>, c: &Connection)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    app.connections.selected = Some(c.id.clone());
    // Sem senha guardada (`None`) vale `trust`; chaveiro indisponível não conecta, senão o erro
    // de autenticação do servidor esconderia a causa real.
    let secret = match app.deps.secrets.get(&c.id) {
        Ok(s) => s.unwrap_or_default(),
        Err(e) => {
            app.notice = Some(e.to_string());
            return;
        }
    };
    app.deps.sessions.connect(c, secret);
}

fn form_window<D>(app: &mut App<D>, ctx: &egui::Context)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let Some(form) = app.form.as_mut() else {
        return;
    };
    let mut open = true;
    let mut save = false;
    let mut cancel = false;
    let title = if form.id.is_some() {
        "Editar conexão"
    } else {
        "Nova conexão"
    };
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            egui::Grid::new("connection-form")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Nome");
                    ui.text_edit_singleline(&mut form.name);
                    ui.end_row();
                    ui.label("Ambiente");
                    egui::ComboBox::from_id_salt("tag")
                        .selected_text(form.tag.label())
                        .show_ui(ui, |ui| {
                            for tag in Tag::ALL {
                                ui.selectable_value(&mut form.tag, tag, tag.label());
                            }
                        });
                    ui.end_row();
                    ui.label("Host");
                    ui.text_edit_singleline(&mut form.host);
                    ui.end_row();
                    ui.label("Porta");
                    ui.text_edit_singleline(&mut form.port);
                    ui.end_row();
                    ui.label("Usuário");
                    ui.text_edit_singleline(&mut form.user);
                    ui.end_row();
                    ui.label("Senha");
                    ui.add(
                        egui::TextEdit::singleline(&mut form.password)
                            .password(true)
                            .hint_text(if form.id.is_some() {
                                "em branco mantém a atual"
                            } else {
                                "em branco para trust"
                            }),
                    );
                    ui.end_row();
                    ui.label("Banco");
                    ui.text_edit_singleline(&mut form.database);
                    ui.end_row();
                    ui.label("TLS");
                    egui::ComboBox::from_id_salt("tls")
                        .selected_text(form.tls.label())
                        .show_ui(ui, |ui| {
                            for mode in TlsMode::ALL {
                                ui.selectable_value(&mut form.tls, mode, mode.label());
                            }
                        });
                    ui.end_row();
                });
            if let Some(e) = &form.error {
                ui.colored_label(theme::error_color(), e);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Salvar").clicked() {
                    save = true;
                }
                if ui.button("Cancelar").clicked() {
                    cancel = true;
                }
            });
        });
    if save {
        save_form(app);
    } else if cancel || !open {
        app.form = None;
    }
}

fn save_form<D>(app: &mut App<D>)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let Some(form) = app.form.as_mut() else {
        return;
    };
    let connection = match form.build() {
        Ok(c) => c,
        Err(e) => {
            form.error = Some(e);
            return;
        }
    };
    app.deps.store.upsert(connection.clone());
    if let Err(e) = app.deps.store.save() {
        // O formulário fica aberto, com a senha digitada, para tentar de novo.
        app.notice = Some(e.to_string());
        return;
    }
    let password = std::mem::take(&mut form.password);
    if !password.is_empty()
        && let Err(e) = app.deps.secrets.set(&connection.id, &Secret::new(password))
    {
        app.notice = Some(e.to_string());
    }
    // A configuração mudou: a sessão aberta (ou a tentativa em voo) com a antiga cai.
    app.deps.sessions.disconnect(&connection.id);
    app.status.remove(&connection.id);
    app.form = None;
}

fn confirm_remove_window<D>(app: &mut App<D>, ctx: &egui::Context)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let Some(id) = app.connections.confirm_remove.clone() else {
        return;
    };
    let Some(name) = app.deps.store.get(&id).map(|c| c.name.clone()) else {
        app.connections.confirm_remove = None;
        return;
    };
    let mut decision: Option<bool> = None;
    egui::Window::new("Remover conexão")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!("Remover \"{name}\" e a senha guardada?"));
            ui.horizontal(|ui| {
                if ui.button("Remover").clicked() {
                    decision = Some(true);
                }
                if ui.button("Cancelar").clicked() {
                    decision = Some(false);
                }
            });
        });
    match decision {
        Some(true) => {
            app.deps.sessions.disconnect(&id);
            app.deps.store.remove(&id);
            // A senha só sai do chaveiro se o arquivo foi gravado: senão a conexão voltaria sem senha.
            let saved = app.deps.store.save();
            let secret = match &saved {
                Ok(()) => app.deps.secrets.remove(&id),
                Err(_) => Ok(()),
            };
            let errors: Vec<String> = [saved.err(), secret.err()]
                .into_iter()
                .flatten()
                .map(|e| e.to_string())
                .collect();
            if !errors.is_empty() {
                app.notice = Some(errors.join("; "));
            }
            app.status.remove(&id);
            app.connections.confirm_remove = None;
        }
        Some(false) => app.connections.confirm_remove = None,
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_formulario_valida_e_monta_a_conexao() {
        let mut f = ConnectionForm::new();
        assert_eq!(f.build().unwrap_err(), "dê um nome à conexão");
        f.name = " pg-localhost ".into();
        f.port = "abc".into();
        assert!(f.build().unwrap_err().contains("porta"));
        f.port = "0".into();
        assert!(f.build().unwrap_err().contains("porta"));
        f.port = "5432".into();
        f.database = "  ".into();
        let c = f.build().unwrap();
        assert_eq!(c.name, "pg-localhost");
        assert_eq!(c.config.database, "postgres");
        assert_eq!(c.config.port, 5432);
        assert_eq!(c.tag, Tag::Local);
        let mut edit = ConnectionForm::edit(&c);
        edit.name = "outro".into();
        let again = edit.build().unwrap();
        assert_eq!(again.id, c.id, "editar não troca o id");
        assert_eq!(again.name, "outro");
    }
}
