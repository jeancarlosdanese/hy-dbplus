//! Telas 3.2, 3.3 e 3.4: conexão aberta. Barra superior colorida pelo ambiente com `AMBIENTE |
//! PostgreSQL <versão> : conexão : banco`, a faixa de bancos abertos (FEAT-008), o painel esquerdo com
//! a árvore do banco ativo (ou o convite ao ⌘K), o centro com a aba Data da tabela aberta (ou o
//! convite) e o painel direito com os campos da linha, que o botão "Painel" esconde e mostra.

use hy_dbplus_engine::Driver;

use crate::app::{App, Status, View};
use crate::{commit, data, palette, rail, record, structure, tabs, theme, tree};

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
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
                if ui.button("← Conexões").clicked() {
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
                            !app.connection_commit_running(&id),
                            egui::Button::new("Desconectar"),
                        )
                        .on_disabled_hover_text("Aguarde o Commit terminar")
                        .clicked()
                    {
                        app.request_disconnect(&id);
                    }
                    if ui.button("Banco ⌘K").clicked() {
                        palette::toggle(app);
                    }
                    let panel_label = if app.side_panel_open {
                        "Painel ▸"
                    } else {
                        "◂ Painel"
                    };

                    if ui
                        .button(panel_label)
                        .on_hover_text("Mostrar ou esconder o painel direito")
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
                    ui.colored_label(theme::error_color(), error);
                    if ui.small_button("OK").clicked() {
                        dismiss = true;
                    }
                }
            });
        });
        if dismiss && let Some(state) = app.conn_state.get_mut(&id) {
            state.error = None;
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
                    ui.colored_label(theme::error_color(), error);
                }
                if ui.button("Escolher banco (⌘K)").clicked() {
                    palette::toggle(app);
                }
            }
        });
    if let Some(key) = &active {
        tabs::show(app, ui, key);
    }
    let data_tab = active
        .as_ref()
        .and_then(|key| app.workspaces.get(key))
        .and_then(|ws| ws.data().cloned());
    if app.side_panel_open {
        egui::Panel::right("workspace-right")
            .default_size(300.0)
            .min_size(200.0)
            .max_size(640.0)
            .show(ui, |ui| match (&active, &data_tab) {
                (Some(key), Some(_)) => record::show(app, ui, key),
                _ => {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("Nenhuma tabela aberta").color(tokens.text_muted),
                        );
                    });
                }
            });
    }
    match (&active, &data_tab) {
        (Some(key), Some(_)) => {
            data::show(app, ui, key);
            commit::show(app, ui.ctx(), key);
            structure::ddl_dialog(app, ui.ctx(), key);
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
