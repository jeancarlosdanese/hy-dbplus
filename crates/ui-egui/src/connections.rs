//! Telas 1 a 4 do protótipo da FEAT-015: lista de conexões (linha inteira clicável, teclado,
//! estado de cada conexão na própria linha), estado vazio, formulário de criar e editar com
//! "Testar conexão" e erro junto do campo, e remover com confirmação.

use hy_dbplus_engine::{
    Connection, ConnectionConfig, ConnectionId, Driver, Secret, Tag, TestOk, TlsMode,
};

use crate::app::{App, Status};
use crate::fonts::icon;
use crate::i18n::{self, Tr};
use crate::theme::{self, radius, size, space, text_size};
use crate::widgets::{self, ButtonKind, DialogAction, DialogOutcome, DialogParts};

/// Estado da lista entre frames.
#[derive(Debug, Default)]
pub struct Connections {
    pub search: String,
    pub selected: Option<ConnectionId>,
    pub confirm_remove: Option<ConnectionId>,
    /// Numera os pedidos de "Testar conexão": resposta de um formulário já fechado não vale.
    pub test_seq: u64,
}

/// Campo do formulário com erro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Host,
    Port,
    User,
}

impl Field {
    /// A mensagem do erro, no idioma da tela.
    pub fn message(self, tr: Tr) -> &'static str {
        match self {
            Field::Name => tr.error_name(),
            Field::Host => tr.error_host(),
            Field::Port => tr.error_port(),
            Field::User => tr.error_user(),
        }
    }
}

/// O "Testar conexão" do formulário.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FormTest {
    #[default]
    Idle,
    /// Em curso, com a etiqueta do pedido.
    Running(String),
    Done(Result<TestOk, String>),
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
    /// O campo com erro da última tentativa de salvar.
    pub error: Option<Field>,
    pub test: FormTest,
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
            test: FormTest::Idle,
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
            test: FormTest::Idle,
        }
    }

    /// A configuração digitada, ou o primeiro campo errado. O nome não entra (o teste não precisa).
    pub fn config(&self) -> Result<ConnectionConfig, Field> {
        if self.host.trim().is_empty() {
            return Err(Field::Host);
        }
        let port: u16 = self.port.trim().parse().map_err(|_| Field::Port)?;
        if port == 0 {
            return Err(Field::Port);
        }
        if self.user.trim().is_empty() {
            return Err(Field::User);
        }
        Ok(ConnectionConfig {
            host: self.host.trim().into(),
            port,
            user: self.user.trim().into(),
            database: if self.database.trim().is_empty() {
                "postgres".into()
            } else {
                self.database.trim().into()
            },
            tls: self.tls,
        })
    }

    /// A conexão pronta para guardar, ou o primeiro campo errado.
    pub fn build(&self) -> Result<Connection, Field> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(Field::Name);
        }
        let config = self.config()?;
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

/// As conexões que a busca deixa ver, na ordem do arquivo.
fn visible<D: Driver>(app: &App<D>) -> Vec<Connection> {
    let search = app.connections.search.trim().to_lowercase();
    app.deps
        .store
        .list()
        .iter()
        .filter(|c| {
            search.is_empty()
                || c.name.to_lowercase().contains(&search)
                || c.subtitle().to_lowercase().contains(&search)
        })
        .cloned()
        .collect()
}

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let tr = app.tr();
    let list = visible(app);
    if app.form.is_none() && app.connections.confirm_remove.is_none() {
        keyboard(app, ui.ctx(), &list);
    }
    egui::Panel::top("connections-top")
        .frame(
            egui::Frame::new()
                .fill(tokens.panel)
                .inner_margin(egui::Margin {
                    left: 32,
                    right: 32,
                    top: 20,
                    bottom: 16,
                }),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                mark(ui);
                ui.label(
                    egui::RichText::new(tr.connections())
                        .size(text_size::TITLE)
                        .family(crate::fonts::strong(ui.ctx())),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::button(
                        ui,
                        ButtonKind::Primary,
                        Some(icon::ADD),
                        tr.new_connection_full(),
                    )
                    .clicked()
                    {
                        app.form = Some(ConnectionForm::new());
                    }
                    app.language_switch(ui);
                    ui.add(
                        egui::TextEdit::singleline(&mut app.connections.search)
                            .id(search_id())
                            .hint_text(format!("{} {}", icon::SEARCH, tr.search_connections()))
                            .desired_width(240.0)
                            .min_size(egui::vec2(0.0, size::CONTROL)),
                    );
                });
            });
        });
    if let Some(notice) = app.notice.clone() {
        egui::Panel::bottom("connections-notice")
            .frame(
                egui::Frame::new()
                    .fill(tokens.panel)
                    .inner_margin(egui::Margin::symmetric(32, 8)),
            )
            .show(ui, |ui| {
                if widgets::banner(ui, widgets::BannerKind::Error, &notice, Some(tr.close())) {
                    app.notice = None;
                }
            });
    }
    if !app.deps.store.list().is_empty() {
        egui::Panel::bottom("connections-hints")
            .frame(
                egui::Frame::new()
                    .fill(tokens.panel)
                    .inner_margin(egui::Margin::symmetric(32, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 18.0;
                    let new = i18n::shortcut_label(ui.ctx(), &new_shortcut());
                    let edit = i18n::shortcut_label(ui.ctx(), &edit_shortcut());
                    for hint in [
                        format!("↑↓ {}", tr.hint_navigate()),
                        format!("↩ {}", tr.hint_connect()),
                        format!("{new} {}", tr.hint_new()),
                        format!("{edit} {}", tr.hint_edit()),
                    ] {
                        ui.label(
                            egui::RichText::new(hint)
                                .size(text_size::SMALL)
                                .color(tokens.text_faint),
                        );
                    }
                });
            });
    }
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(tokens.panel)
                .inner_margin(egui::Margin::symmetric(32, 0)),
        )
        .show(ui, |ui| {
            if app.deps.store.list().is_empty() {
                ui.add_space(ui.available_height() / 4.0);
                widgets::empty_state(
                    ui,
                    icon::CONNECT,
                    tr.no_connections_title(),
                    tr.no_connections_hint(),
                );
                ui.add_space(space::M);
                ui.vertical_centered(|ui| {
                    if widgets::button(
                        ui,
                        ButtonKind::Primary,
                        Some(icon::ADD),
                        tr.new_connection_full(),
                    )
                    .clicked()
                    {
                        app.form = Some(ConnectionForm::new());
                    }
                });
                return;
            }
            if list.is_empty() {
                ui.label(egui::RichText::new(tr.no_match()).color(tokens.text_muted));
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                for c in &list {
                    row(app, ui, c);
                }
            });
        });
    form_window(app, ui.ctx());
    confirm_remove_window(app, ui.ctx());
}

/// A marca do app no cabeçalho: o quadrado azul-petróleo com "hy".
fn mark(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 7.0, egui::Color32::from_rgb(0x1C, 0x4F, 0x5E));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "hy",
        egui::FontId::new(text_size::BODY, crate::fonts::strong(ui.ctx())),
        egui::Color32::WHITE,
    );
    ui.add_space(space::XS);
}

fn search_id() -> egui::Id {
    egui::Id::new("connections-search")
}

pub(crate) fn new_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::N)
}

pub(crate) fn edit_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::E)
}

/// ↑↓ movem a seleção pela lista visível, Enter conecta (ou abre) a selecionada, ⌘N abre o
/// formulário novo e ⌘E edita a selecionada. Só com nenhum diálogo aberto.
fn keyboard<D>(app: &mut App<D>, ctx: &egui::Context, list: &[Connection])
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    // Setas e Enter só com o foco livre ou na busca: num botão focado (Remover, Conectar de
    // outra linha, PT | EN) o Enter é dele.
    let free = ctx
        .memory(|m| m.focused())
        .is_none_or(|id| id == search_id());
    let (new, edit, up, down, enter) = ctx.input_mut(|i| {
        let mut key = |k| free && i.consume_key(egui::Modifiers::NONE, k);
        let up = key(egui::Key::ArrowUp);
        let down = key(egui::Key::ArrowDown);
        let enter = key(egui::Key::Enter);
        (
            i.consume_shortcut(&new_shortcut()),
            i.consume_shortcut(&edit_shortcut()),
            up,
            down,
            enter,
        )
    });
    if new {
        app.form = Some(ConnectionForm::new());
        return;
    }
    let position = app
        .connections
        .selected
        .as_ref()
        .and_then(|id| list.iter().position(|c| &c.id == id));
    let target = match (up, down, position) {
        (true, _, Some(p)) => Some(p.saturating_sub(1)),
        (_, true, Some(p)) => Some((p + 1).min(list.len().saturating_sub(1))),
        (true, _, None) => list.len().checked_sub(1),
        (_, true, None) => (!list.is_empty()).then_some(0),
        _ => None,
    };
    if let Some(c) = target.and_then(|t| list.get(t)) {
        app.connections.selected = Some(c.id.clone());
        return;
    }
    let Some(c) = position.and_then(|p| list.get(p)).cloned() else {
        return;
    };
    if edit {
        app.form = Some(ConnectionForm::edit(&c));
    } else if enter {
        activate(app, &c);
    }
}

/// Conecta, ou abre o workspace da conexão já conectada. Conectando: nada.
fn activate<D>(app: &mut App<D>, c: &Connection)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    match app.status_of(&c.id) {
        Status::Connecting => {}
        Status::Connected { .. } => {
            app.connections.selected = Some(c.id.clone());
            app.focus = Some(c.id.clone());
            app.active = app.key_of(&c.id);
            app.view = crate::app::View::Workspace;
        }
        _ => connect(app, c),
    }
}

/// Uma linha: a faixa do ambiente, o selo Pg, nome, etiqueta e estado, `host:porta / banco`,
/// os ícones de editar e remover (na selecionada ou com erro) e Conectar ou Abrir. A linha inteira
/// é clicável: clique seleciona, duplo clique conecta ou abre.
fn row<D>(app: &mut App<D>, ui: &mut egui::Ui, c: &Connection)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let tr = app.tr();
    let status = app.status_of(&c.id);
    let selected = app.connections.selected.as_deref() == Some(c.id.as_str());
    let failed = matches!(status, Status::Failed(_));
    let width = ui.available_width();
    // A resposta da linha nasce antes dos botões: eles ficam por cima e recebem o próprio clique.
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 56.0), egui::Sense::click());
    let stroke = if selected {
        egui::Stroke::new(1.5, tokens.accent)
    } else {
        egui::Stroke::new(1.0, tokens.border)
    };
    let fill = if response.hovered() && !selected {
        tokens.hover
    } else {
        tokens.card
    };
    ui.painter()
        .rect(rect, radius::CARD, fill, stroke, egui::StrokeKind::Inside);
    let bar = egui::Rect::from_min_size(rect.min, egui::vec2(4.0, rect.height()));
    ui.painter().rect_filled(
        bar,
        egui::CornerRadius {
            nw: radius::CARD,
            sw: radius::CARD,
            ne: 0,
            se: 0,
        },
        theme::tag_color(c.tag),
    );
    let mut clicked_button = false;
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(
                rect.shrink2(egui::vec2(space::M, 0.0))
                    .translate(egui::vec2(4.0, 0.0)),
            )
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            // Texto selecionável pega o clique: a linha inteira precisa recebê-lo.
            ui.style_mut().interaction.selectable_labels = false;
            // Nome e endereço longos (janela estreita, host da nuvem) truncam: a linha tem altura fixa
            // e os botões ficam à direita.
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            // Os botões primeiro, da direita: o texto fica com a largura que sobra e não passa por baixo deles.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let kind = if selected {
                    ButtonKind::Primary
                } else {
                    ButtonKind::Normal
                };
                match status {
                    Status::Connecting => {
                        widgets::button_enabled(
                            ui,
                            false,
                            kind,
                            None,
                            tr.connect(),
                            tr.connecting(),
                        );
                    }
                    Status::Connected { .. } => {
                        if widgets::button(ui, kind, None, tr.open()).clicked() {
                            clicked_button = true;
                            activate(app, c);
                        }
                    }
                    _ => {
                        if widgets::button(ui, kind, None, tr.connect()).clicked() {
                            clicked_button = true;
                            connect(app, c);
                        }
                    }
                }
                if selected || failed {
                    if widgets::icon_button(ui, icon::DELETE, tr.delete()).clicked() {
                        clicked_button = true;
                        app.connections.confirm_remove = Some(c.id.clone());
                    }
                    if widgets::icon_button(ui, icon::EDIT, tr.edit()).clicked() {
                        clicked_button = true;
                        app.form = Some(ConnectionForm::edit(c));
                    }
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let (badge, _) =
                        ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
                    ui.painter().rect_filled(badge, 7.0, tokens.hover);
                    ui.painter().text(
                        badge.center(),
                        egui::Align2::CENTER_CENTER,
                        "Pg",
                        egui::FontId::new(text_size::SMALL, crate::fonts::strong(ui.ctx())),
                        tokens.text_muted,
                    );
                    ui.add_space(space::XS);
                    // A largura que sobra à esquerda dos botões: os textos não passam dela.
                    let width = ui.available_width().max(0.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(width);
                        ui.add_space(space::S);
                        ui.horizontal(|ui| {
                            // O nome fica com até 60% para a etiqueta e o estado caberem depois.
                            ui.allocate_ui(egui::vec2(width * 0.6, 20.0), |ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&c.name)
                                            .size(14.0)
                                            .family(crate::fonts::strong(ui.ctx())),
                                    )
                                    .truncate(),
                                );
                            });
                            widgets::env_tag(ui, c.tag);
                            match &status {
                                Status::Connected { .. } => {
                                    ui.label(
                                        egui::RichText::new(format!("● {}", tr.connected()))
                                            .size(text_size::SMALL)
                                            .color(tokens.ok),
                                    );
                                }
                                Status::Connecting => {
                                    widgets::loading(ui, tr.connecting());
                                }
                                Status::Failed(e) => {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(format!("{} {e}", icon::WARNING))
                                                .size(text_size::SMALL)
                                                .color(tokens.danger),
                                        )
                                        .truncate(),
                                    );
                                }
                                Status::Idle => {}
                            }
                        });
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!(
                                    "{}:{} / {}",
                                    c.config.host, c.config.port, c.config.database
                                ))
                                .monospace()
                                .color(tokens.text_muted),
                            )
                            .truncate(),
                        );
                    });
                });
            });
        },
    );
    if !clicked_button {
        if response.double_clicked() {
            activate(app, c);
        } else if response.clicked() {
            app.connections.selected = Some(c.id.clone());
        }
    }
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
    let tr = app.tr();
    let tokens = theme::current(ctx);
    let Some(form) = app.form.as_mut() else {
        return;
    };
    // Enter salva quando o foco está num campo de texto do formulário (ou em nada). Num botão
    // focado (Cancelar, Testar, os seletores), o Enter é do botão: o egui o clica sozinho.
    let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
    let mut test = false;
    let title = if form.id.is_some() {
        tr.edit_connection()
    } else {
        tr.new_connection_full()
    };
    let testing = matches!(form.test, FormTest::Running(_));
    let outcome = widgets::dialog_full(
        ctx,
        DialogParts {
            id: egui::Id::new("connection-form"),
            title,
            subtitle: Some(tr.form_subtitle()),
            width: 520.0,
            cancel: tr.cancel(),
            action: Some(DialogAction {
                label: tr.save(),
                kind: ButtonKind::Primary,
                enabled: true,
                shortcut: None,
            }),
            busy: false,
        },
        |ui| {
            let error = form.error;
            let column = (ui.available_width() - space::L) / 2.0;
            let field = |ui: &mut egui::Ui,
                         label: &str,
                         hint: Option<(&str, bool)>,
                         add: &mut dyn FnMut(&mut egui::Ui)| {
                ui.allocate_ui(egui::vec2(column, 60.0), |ui| {
                    ui.set_width(column);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(label)
                                .size(text_size::SMALL)
                                .family(crate::fonts::strong(ui.ctx()))
                                .color(tokens.text_muted),
                        );
                        add(ui);
                        if let Some((hint, is_error)) = hint {
                            ui.label(egui::RichText::new(hint).size(text_size::SMALL).color(
                                if is_error {
                                    tokens.danger
                                } else {
                                    tokens.text_faint
                                },
                            ));
                        }
                    });
                });
            };
            let error_hint = |f: Field| (error == Some(f)).then(|| (f.message(tr), true));
            egui::Grid::new("connection-form-grid")
                .num_columns(2)
                .spacing([space::L, space::M])
                .show(ui, |ui| {
                    field(ui, tr.field_name(), error_hint(Field::Name), &mut |ui| {
                        ui.add(text_field(
                            &tokens,
                            "name",
                            &mut form.name,
                            false,
                            error == Some(Field::Name),
                        ));
                    });
                    field(ui, tr.field_environment(), None, &mut |ui| {
                        egui::ComboBox::from_id_salt("tag")
                            .selected_text(form.tag.label().to_uppercase())
                            .width(column)
                            .show_ui(ui, |ui| {
                                for tag in Tag::ALL {
                                    ui.selectable_value(
                                        &mut form.tag,
                                        tag,
                                        tag.label().to_uppercase(),
                                    );
                                }
                            });
                    });
                    ui.end_row();
                    field(ui, "Host", error_hint(Field::Host), &mut |ui| {
                        ui.add(text_field(
                            &tokens,
                            "host",
                            &mut form.host,
                            false,
                            error == Some(Field::Host),
                        ));
                    });
                    field(ui, tr.field_port(), error_hint(Field::Port), &mut |ui| {
                        ui.add(text_field(
                            &tokens,
                            "port",
                            &mut form.port,
                            false,
                            error == Some(Field::Port),
                        ));
                    });
                    ui.end_row();
                    field(ui, tr.field_user(), error_hint(Field::User), &mut |ui| {
                        ui.add(text_field(
                            &tokens,
                            "user",
                            &mut form.user,
                            false,
                            error == Some(Field::User),
                        ));
                    });
                    let password_hint = if form.id.is_some() {
                        tr.password_hint_edit()
                    } else {
                        tr.password_hint_new()
                    };
                    field(
                        ui,
                        tr.field_password(),
                        Some((password_hint, false)),
                        &mut |ui| {
                            ui.add(text_field(
                                &tokens,
                                "password",
                                &mut form.password,
                                true,
                                false,
                            ));
                        },
                    );
                    ui.end_row();
                    field(ui, tr.field_database(), None, &mut |ui| {
                        ui.add(text_field(
                            &tokens,
                            "database",
                            &mut form.database,
                            false,
                            false,
                        ));
                    });
                    field(ui, "TLS", None, &mut |ui| {
                        egui::ComboBox::from_id_salt("tls")
                            .selected_text(form.tls.label())
                            .width(column)
                            .show_ui(ui, |ui| {
                                for mode in TlsMode::ALL {
                                    ui.selectable_value(&mut form.tls, mode, mode.label());
                                }
                            });
                    });
                    ui.end_row();
                });
        },
        |ui| {
            if widgets::button_enabled(
                ui,
                !testing,
                ButtonKind::Normal,
                Some(icon::CONNECT),
                tr.test_connection(),
                tr.testing(),
            )
            .clicked()
            {
                test = true;
            }
            match &form.test {
                FormTest::Idle => {}
                FormTest::Running(_) => widgets::loading(ui, tr.testing()),
                FormTest::Done(Ok(ok)) => {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!(
                                "{} {}",
                                icon::OK,
                                tr.test_ok(ok.elapsed_ms, &ok.server_version)
                            ))
                            .size(text_size::SMALL)
                            .color(tokens.ok),
                        )
                        .truncate(),
                    );
                }
                FormTest::Done(Err(e)) => {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("{} {e}", icon::ERROR))
                                .size(text_size::SMALL)
                                .color(tokens.danger),
                        )
                        .truncate(),
                    );
                }
            }
        },
    );
    let focus_on_field = ctx
        .memory(|m| m.focused())
        .is_none_or(|id| FORM_FIELDS.iter().any(|f| field_id(f) == id));
    match outcome {
        DialogOutcome::Cancel => app.form = None,
        DialogOutcome::Confirm => save_form(app),
        DialogOutcome::Open if test => test_form(app),
        DialogOutcome::Open if enter && focus_on_field => save_form(app),
        DialogOutcome::Open => {}
    }
}

/// Os campos de texto do formulário, pelo nome do id.
const FORM_FIELDS: [&str; 6] = ["name", "host", "port", "user", "password", "database"];

fn field_id(name: &str) -> egui::Id {
    egui::Id::new(("connection-form-field", name))
}

/// Campo de texto do formulário; com erro, a borda fica vermelha (o texto do erro vem sob ele).
fn text_field<'a>(
    tokens: &theme::Tokens,
    name: &str,
    value: &'a mut String,
    password: bool,
    invalid: bool,
) -> egui::TextEdit<'a> {
    let border = if invalid {
        tokens.danger
    } else {
        tokens.border
    };
    egui::TextEdit::singleline(value)
        .id(field_id(name))
        .password(password)
        .desired_width(f32::INFINITY)
        .min_size(egui::vec2(0.0, size::CONTROL))
        .text_color(tokens.text)
        .frame(
            egui::Frame::new()
                .fill(tokens.field)
                .stroke(egui::Stroke::new(1.0, border))
                .corner_radius(radius::CONTROL)
                .inner_margin(egui::Margin::symmetric(8, 6)),
        )
}

/// "Testar conexão": a configuração digitada e a senha digitada (ou, na edição com o campo em
/// branco, a do Keychain). Nada é gravado; o resultado volta em `Event::TestFinished`.
fn test_form<D>(app: &mut App<D>)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let Some(form) = app.form.as_mut() else {
        return;
    };
    let config = match form.config() {
        Ok(config) => config,
        Err(field) => {
            form.error = Some(field);
            return;
        }
    };
    form.error = None;
    let password = if form.password.is_empty() {
        match &form.id {
            Some(id) => match app.deps.secrets.get(id) {
                Ok(secret) => secret.unwrap_or_default(),
                Err(e) => {
                    form.test = FormTest::Done(Err(e.to_string()));
                    return;
                }
            },
            None => Secret::default(),
        }
    } else {
        Secret::new(form.password.clone())
    };
    app.connections.test_seq += 1;
    let tag = format!("connection-test-{}", app.connections.test_seq);
    form.test = FormTest::Running(tag.clone());
    app.deps.sessions.test_connection(tag, config, password);
}

/// O resultado do "Testar conexão", se for do pedido em curso do formulário aberto.
pub(crate) fn test_finished<D: Driver>(
    app: &mut App<D>,
    tag: &str,
    result: Result<TestOk, hy_dbplus_engine::DriverError>,
) {
    if let Some(form) = app.form.as_mut()
        && form.test == FormTest::Running(tag.to_owned())
    {
        form.test = FormTest::Done(result.map_err(|e| e.to_string()));
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
        Err(field) => {
            form.error = Some(field);
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
    let tr = app.tr();
    let Some(id) = app.connections.confirm_remove.clone() else {
        return;
    };
    let Some(name) = app.deps.store.get(&id).map(|c| c.name.clone()) else {
        app.connections.confirm_remove = None;
        return;
    };
    let tokens = theme::current(ctx);
    let title = tr.remove_connection_title(&name);
    let outcome = widgets::dialog_full(
        ctx,
        DialogParts {
            id: egui::Id::new("connection-remove"),
            title: &title,
            subtitle: None,
            width: 400.0,
            cancel: tr.cancel(),
            action: Some(DialogAction {
                label: tr.remove_connection(),
                kind: ButtonKind::Danger,
                enabled: true,
                shortcut: None,
            }),
            busy: false,
        },
        |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(tr.remove_connection_body()).color(tokens.text_muted),
                )
                .wrap(),
            );
        },
        |_| {},
    );
    let decision = match outcome {
        DialogOutcome::Confirm => Some(true),
        DialogOutcome::Cancel => Some(false),
        DialogOutcome::Open => None,
    };
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
            // As queries salvas da conexão saem junto (FEAT-011).
            let queries = app.deps.queries.remove_connection(&id);
            let errors: Vec<String> = [saved.err(), secret.err(), queries.err()]
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
        assert_eq!(f.build().unwrap_err(), Field::Name);
        f.name = " pg-localhost ".into();
        f.port = "abc".into();
        assert_eq!(f.build().unwrap_err(), Field::Port);
        f.port = "0".into();
        assert_eq!(f.build().unwrap_err(), Field::Port);
        f.port = "54321x".into();
        assert_eq!(f.config().unwrap_err(), Field::Port);
        f.host = " ".into();
        assert_eq!(f.build().unwrap_err(), Field::Host);
        f.host = "localhost".into();
        f.user = String::new();
        f.port = "5432".into();
        assert_eq!(f.build().unwrap_err(), Field::User);
        f.user = "postgres".into();
        assert_eq!(
            Field::Port.message(Tr(hy_dbplus_engine::Language::PtBr)),
            "Use um número de 1 a 65535."
        );
        assert_eq!(
            Field::Port.message(Tr(hy_dbplus_engine::Language::En)),
            "Use a number from 1 to 65535."
        );
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
