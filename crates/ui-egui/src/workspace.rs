//! A conexão aberta (telas 5 a 16 do protótipo da FEAT-015): barra superior neutra com a etiqueta
//! do ambiente, `conexão / banco` e a versão, e a faixa fina na cor do ambiente embaixo; a faixa de
//! bancos abertos (FEAT-008); o painel esquerdo com a árvore do banco ativo (ou o convite ao ⌘K); o
//! centro com a aba ativa (ou o estado vazio) e o painel de registro, que o ícone da barra mostra e
//! esconde.

use hy_dbplus_engine::Driver;

use crate::app::{App, Status, View};
use crate::fonts::icon;
use crate::widgets::{self, BannerKind};
use crate::{commit, data, editor, i18n, palette, rail, record, structure, tabs, theme, tree};

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let tr = app.tr();
    let Some(id) = app.focus.clone() else {
        app.view = View::Connections;
        return;
    };
    let Some(connection) = app.deps.store.get(&id).cloned() else {
        app.view = View::Connections;
        return;
    };
    let Status::Connected { server_version, .. } = app.status_of(&id) else {
        app.view = View::Connections;
        return;
    };
    if !app.table_dialog_open()
        && app.confirm_close.is_none()
        && ui.input_mut(|input| input.consume_shortcut(&palette::shortcut()))
    {
        palette::toggle(app);
    }
    let active = app.active.clone().filter(|k| k.0 == id);
    top_bar(app, ui, &id, &connection, &server_version, active.as_ref());
    // A faixa do ambiente sob a barra: o sinal de onde se está, sem pintar a barra inteira.
    egui::Panel::top("workspace-env-stripe")
        .exact_size(theme::size::ENV_STRIPE)
        .resizable(false)
        .frame(egui::Frame::new().fill(theme::tag_color(connection.tag)))
        .show(ui, |_| {});
    // Com um banco ativo a árvore ocupa o painel esquerdo: o "Abrindo…" e o erro do ⌘K vão numa
    // faixa abaixo da barra, senão o usuário escolhe um banco e nada acontece (achado do revisor).
    let state = app.conn_state.get(&id).cloned().unwrap_or_default();
    // Erro de arquivo ou chaveiro (ex.: idioma que não foi guardado): a tela de conexões mostra no
    // rodapé; aqui ele aparece logo abaixo da barra, senão some até o usuário voltar à lista. O
    // mesmo erro já guardado no estado da conexão (fechar aba ou banco) aparece só uma vez, lá.
    let banner_frame = egui::Frame::new()
        .fill(tokens.panel)
        .inner_margin(egui::Margin::symmetric(12, 6));
    if let Some(notice) = app
        .notice
        .clone()
        .filter(|n| state.error.as_ref() != Some(n))
    {
        let mut dismiss = false;
        egui::Panel::top("workspace-app-notice")
            .frame(banner_frame)
            .show(ui, |ui| {
                dismiss = widgets::banner(ui, BannerKind::Error, &notice, Some(tr.close()));
            });
        if dismiss {
            app.notice = None;
        }
    }
    if active.is_some() && (state.opening.is_some() || state.error.is_some()) {
        let mut dismiss = false;
        egui::Panel::top("workspace-notice")
            .frame(banner_frame)
            .show(ui, |ui| {
                if let Some(db) = &state.opening {
                    widgets::loading(ui, &tr.opening_database(db));
                }
                if let Some(error) = &state.error {
                    dismiss = widgets::banner(ui, BannerKind::Error, error, Some(tr.ok()));
                }
            });
        if dismiss && let Some(conn) = app.conn_state.get_mut(&id) {
            // O mesmo erro no aviso do app sai junto, senão volta na lista de conexões.
            if app.notice.is_some() && app.notice == conn.error {
                app.notice = None;
            }
            conn.error = None;
        }
    }
    rail::show(app, ui);
    egui::Panel::left("workspace-left")
        .default_size(260.0)
        .frame(
            egui::Frame::new()
                .fill(tokens.panel)
                .inner_margin(egui::Margin::symmetric(8, 8)),
        )
        .show(ui, |ui| {
            if let Some(key) = &active {
                tree::show(app, ui, key);
            } else if state.opening.is_some() {
                widgets::loading(ui, tr.opening_database_generic());
            } else {
                ui.add_space(ui.available_height() / 5.0);
                widgets::empty_state(
                    ui,
                    icon::DATABASE,
                    tr.no_database_title(),
                    tr.no_database_hint(),
                );
                if let Some(error) = &state.error {
                    ui.add_space(theme::space::S);
                    widgets::banner(ui, BannerKind::Error, error, None);
                }
                ui.add_space(theme::space::M);
                ui.vertical_centered(|ui| {
                    let choose = format!(
                        "{} ({})",
                        tr.choose_database(),
                        i18n::shortcut_label(ui.ctx(), &palette::shortcut())
                    );
                    if widgets::button(ui, widgets::ButtonKind::Primary, None, &choose).clicked() {
                        palette::toggle(app);
                    }
                });
            }
        });
    if let Some(key) = &active {
        tabs::show(app, ui, key);
    }
    // A aba ativa: de tabela, de SQL ou nenhuma.
    let (is_table, is_query) = active
        .as_ref()
        .and_then(|key| app.workspaces.get(key))
        .map(|ws| (ws.data().is_some(), ws.query().is_some()))
        .unwrap_or((false, false));
    if app.side_panel_open {
        egui::Panel::right("workspace-right")
            .default_size(300.0)
            .min_size(200.0)
            .max_size(640.0)
            .show(ui, |ui| match &active {
                Some(key) if is_table || is_query => record::show(app, ui, key),
                _ => {
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new(tr.no_table_open()).color(tokens.text_muted));
                    });
                }
            });
    }
    match &active {
        Some(key) if is_table => {
            data::show(app, ui, key);
            commit::show(app, ui.ctx(), key);
            structure::ddl_dialog(app, ui.ctx(), key);
        }
        Some(key) if is_query => {
            editor::show(app, ui, key);
        }
        _ => {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(tokens.bg))
                .show(ui, |ui| {
                    ui.add_space(ui.available_height() / 3.0);
                    let hint = tr.nothing_open_hint(
                        active.is_some(),
                        &i18n::shortcut_label(ui.ctx(), &palette::shortcut()),
                        &i18n::shortcut_label(ui.ctx(), &tabs::new_query_shortcut()),
                    );
                    widgets::empty_state(ui, icon::TABLE, tr.nothing_open(), &hint);
                });
        }
    }
    rail::confirm(app, ui.ctx());
    palette::show(app, ui.ctx(), &id);
}

/// A barra superior (telas 5 a 16): voltar às conexões, etiqueta do ambiente, `conexão / banco` e a
/// versão do servidor; à direita o campo da paleta (⌘K), PT | EN, o painel de registro e desconectar.
fn top_bar<D>(
    app: &mut App<D>,
    ui: &mut egui::Ui,
    id: &hy_dbplus_engine::ConnectionId,
    connection: &hy_dbplus_engine::Connection,
    server_version: &str,
    active: Option<&crate::app::DbKey>,
) where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let tr = app.tr();
    egui::Panel::top("workspace-top")
        .exact_size(theme::size::BAR)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(tokens.panel)
                .inner_margin(egui::Margin::symmetric(8, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                if widgets::button(
                    ui,
                    widgets::ButtonKind::Ghost,
                    Some(icon::PREVIOUS),
                    tr.connections(),
                )
                .clicked()
                {
                    app.view = View::Connections;
                }
                widgets::env_tag(ui, connection.tag);
                // Os controles da direita primeiro: o caminho e a versão ficam com o que sobra.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::icon_button_full(
                        ui,
                        !app.connection_busy(id),
                        icon::DISCONNECT,
                        tr.disconnect(),
                        None,
                        tr.wait_for_run(),
                    )
                    .clicked()
                    {
                        app.request_disconnect(id);
                    }
                    let panel_hint = if app.side_panel_open {
                        tr.record_panel_hide()
                    } else {
                        tr.record_panel_show()
                    };
                    if widgets::icon_button(ui, icon::PANEL, panel_hint).clicked() {
                        app.side_panel_open = !app.side_panel_open;
                    }
                    app.language_switch(ui);
                    let shortcut = i18n::shortcut_label(ui.ctx(), &palette::shortcut());
                    let go_to = egui::Button::new(
                        egui::RichText::new(format!("{} {}", icon::SEARCH, tr.go_to_database()))
                            .color(tokens.text_faint),
                    )
                    .right_text(egui::RichText::new(shortcut).color(tokens.text_faint))
                    .fill(tokens.field)
                    .stroke(egui::Stroke::new(1.0, tokens.border))
                    .corner_radius(theme::radius::CONTROL)
                    .min_size(egui::vec2(220.0, theme::size::CONTROL));
                    if ui.add(go_to).clicked() {
                        palette::toggle(app);
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let mut crumb = connection.name.clone();
                        if let Some((_, db)) = active {
                            crumb.push_str(&format!(" / {db}"));
                        }
                        ui.label(
                            egui::RichText::new(crumb)
                                .family(crate::fonts::strong(ui.ctx()))
                                .color(tokens.text),
                        );
                        ui.label(
                            egui::RichText::new(format!("PostgreSQL {server_version}"))
                                .size(theme::text_size::SMALL)
                                .color(tokens.text_faint),
                        );
                    });
                });
            });
        });
}
