//! Telas 3.2, 3.3 e 3.4: conexão aberta. Barra superior colorida pelo ambiente com `AMBIENTE |
//! PostgreSQL <versão> : conexão : banco`, painel esquerdo com a árvore ou o convite ao ⌘K, centro com a
//! aba Data da tabela aberta (ou o convite) e painel direito com os tamanhos ou os campos da linha, que o
//! botão "Painel" esconde e mostra.

use hy_dbplus_engine::{ConnectionId, Driver};

use crate::app::{App, Status, View};
use crate::{commit, data, palette, record, theme, tree};

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, id: &ConnectionId)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let Some(connection) = app.deps.store.get(id).cloned() else {
        app.view = View::Connections;
        return;
    };
    let Status::Connected { server_version, .. } = app.status_of(id) else {
        app.view = View::Connections;
        return;
    };
    if ui.input_mut(|input| input.consume_shortcut(&palette::shortcut())) {
        palette::toggle(app);
    }
    let database = app.workspaces.get(id).and_then(|ws| ws.database.clone());
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
                if let Some(db) = &database {
                    title.push_str(&format!(" : {db}"));
                }
                ui.label(
                    egui::RichText::new(title)
                        .strong()
                        .color(egui::Color32::WHITE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Desconectar").clicked() {
                        app.deps.sessions.disconnect(id);
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
            if database.is_some() {
                tree::show(app, ui, id);
            } else {
                let (loading, error) = app
                    .workspaces
                    .get(id)
                    .map(|ws| (ws.loading, ws.error.clone()))
                    .unwrap_or_default();
                if loading {
                    ui.label(egui::RichText::new("Abrindo banco…").color(tokens.text_muted));
                } else {
                    ui.label(
                        egui::RichText::new("Nenhum banco selecionado.").color(tokens.text_muted),
                    );
                }
                if let Some(error) = error {
                    ui.colored_label(theme::error_color(), error);
                }
                if ui.button("Escolher banco (⌘K)").clicked() {
                    palette::toggle(app);
                }
            }
        });
    let data_tab = app.workspaces.get(id).and_then(|ws| ws.data.clone());
    if app.side_panel_open {
        egui::Panel::right("workspace-right")
            .default_size(300.0)
            .min_size(200.0)
            .max_size(640.0)
            .show(ui, |ui| {
                if data_tab.is_some() {
                    record::show(app, ui, id);
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("Nenhuma tabela aberta").color(tokens.text_muted),
                        );
                    });
                }
            });
    }
    if data_tab.is_some() {
        data::show(app, ui, id);
        commit::show(app, ui.ctx(), id);
    } else {
        egui::CentralPanel::default_margins().show(ui, |ui| {
            ui.centered_and_justified(|ui| {
                let text = if database.is_some() {
                    "Escolha uma tabela na árvore."
                } else {
                    "Nenhum banco selecionado. ⌘K para escolher."
                };
                ui.label(egui::RichText::new(text).color(tokens.text_muted));
            });
        });
    }
    palette::show(app, ui.ctx(), id);
}
