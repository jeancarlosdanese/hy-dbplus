//! Telas 3.2, 3.3 e 3.4: conexão aberta. Barra superior colorida pelo ambiente com `AMBIENTE |
//! PostgreSQL <versão> : conexão : banco`, a faixa de bancos abertos (FEAT-008), o painel esquerdo com
//! a árvore do banco ativo (ou o convite ao ⌘K), o centro com a aba Data da tabela aberta (ou o
//! convite) e o painel direito com os campos da linha, que o botão "Painel" esconde e mostra.

use hy_dbplus_engine::Driver;

use crate::app::{App, Status, View};
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
    if ui.input_mut(|input| input.consume_shortcut(&palette::shortcut())) {
        palette::toggle(app);
    }
    let active = app.active.clone().filter(|k| k.0 == id);
    let color = theme::tag_color(connection.tag);
    egui::Panel::top("workspace-top")
        .frame(egui::Frame::new().fill(color).inner_margin(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button(tr.back_to_connections()).clicked() {
                    app.view = View::Connections;
                }
                let mut title = format!(
                    "{} | PostgreSQL {} : {}",
                    connection.tag.label().to_uppercase(),
                    server_version,
                    connection.name
                );
                if let Some((_, db)) = &active {
                    title.push_str(&format!(" : {db}"));
                }
                ui.label(
                    egui::RichText::new(title)
                        .strong()
                        .color(egui::Color32::WHITE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            !app.connection_busy(&id),
                            egui::Button::new(tr.disconnect()),
                        )
                        .on_disabled_hover_text(tr.wait_for_run())
                        .clicked()
                    {
                        app.request_disconnect(&id);
                    }
                    let palette_label = format!(
                        "{} {}",
                        tr.database(),
                        i18n::shortcut_label(ui.ctx(), &palette::shortcut())
                    );
                    if ui.button(palette_label).clicked() {
                        palette::toggle(app);
                    }
                    app.language_switch(ui);
                    let panel_label = if app.side_panel_open {
                        tr.panel_hide()
                    } else {
                        tr.panel_show()
                    };

                    if ui
                        .button(panel_label)
                        .on_hover_text(tr.panel_hint())
                        .clicked()
                    {
                        app.side_panel_open = !app.side_panel_open;
                    }
                });
            });
        });
    // Com um banco ativo a árvore ocupa o painel esquerdo: o "Abrindo…" e o erro do ⌘K vão numa
    // faixa abaixo da barra, senão o usuário escolhe um banco e nada acontece (achado do revisor).
    let state = app.conn_state.get(&id).cloned().unwrap_or_default();
    // Erro de arquivo ou chaveiro (ex.: idioma que não foi guardado): a tela de conexões mostra no
    // rodapé; aqui ele aparece logo abaixo da barra, senão some até o usuário voltar à lista. O
    // mesmo erro já guardado no estado da conexão (fechar aba ou banco) aparece só uma vez, lá.
    if let Some(notice) = app
        .notice
        .clone()
        .filter(|n| state.error.as_ref() != Some(n))
    {
        let mut dismiss = false;
        egui::Panel::top("workspace-app-notice").show(ui, |ui| {
            ui.add_space(theme::space::XS);
            dismiss = widgets::banner(ui, BannerKind::Error, &notice, Some(tr.close()));
            ui.add_space(theme::space::XS);
        });
        if dismiss {
            app.notice = None;
        }
    }
    if active.is_some() && (state.opening.is_some() || state.error.is_some()) {
        let mut dismiss = false;
        egui::Panel::top("workspace-notice").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(db) = &state.opening {
                    ui.label(
                        egui::RichText::new(format!("Abrindo {db}…")).color(tokens.text_muted),
                    );
                }
                if let Some(error) = &state.error {
                    ui.colored_label(theme::error_color(ui.ctx()), error);
                    if ui.small_button("OK").clicked() {
                        dismiss = true;
                    }
                }
            });
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
        .show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Items").strong());
                ui.label(egui::RichText::new("Queries").color(tokens.text_muted));
                ui.label(egui::RichText::new("History").color(tokens.text_muted));
            });
            ui.separator();
            if let Some(key) = &active {
                tree::show(app, ui, key);
            } else {
                if state.opening.is_some() {
                    ui.label(egui::RichText::new("Abrindo banco…").color(tokens.text_muted));
                } else {
                    ui.label(
                        egui::RichText::new("Nenhum banco selecionado.").color(tokens.text_muted),
                    );
                }
                if let Some(error) = &state.error {
                    ui.colored_label(theme::error_color(ui.ctx()), error);
                }
                let choose = format!(
                    "{} ({})",
                    tr.choose_database(),
                    i18n::shortcut_label(ui.ctx(), &palette::shortcut())
                );
                if ui.button(choose).clicked() {
                    palette::toggle(app);
                }
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
                        ui.label(
                            egui::RichText::new("Nenhuma tabela aberta").color(tokens.text_muted),
                        );
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
            egui::CentralPanel::default_margins().show(ui, |ui| {
                ui.centered_and_justified(|ui| {
                    let text = if active.is_some() {
                        "Escolha uma tabela na árvore."
                    } else {
                        "Nenhum banco selecionado. ⌘K para escolher."
                    };
                    ui.label(egui::RichText::new(text).color(tokens.text_muted));
                });
            });
        }
    }
    rail::confirm(app, ui.ctx());
    palette::show(app, ui.ctx(), &id);
}
