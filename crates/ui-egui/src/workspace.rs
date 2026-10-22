//! Tela 3.2: conexão aberta, sem banco selecionado. Barra superior colorida pelo ambiente com
//! `AMBIENTE | PostgreSQL <versão> : conexão`, painel esquerdo com os bancos (até o ⌘K do M2),
//! centro e painel direito vazios.

use hy_dbplus_engine::ConnectionId;

use crate::app::{App, Status, View};
use crate::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui, id: &ConnectionId) {
    let tokens = theme::current(ui.ctx());
    let Some(connection) = app.deps.store.get(id).cloned() else {
        app.view = View::Connections;
        return;
    };
    let Status::Connected {
        server_version,
        databases,
    } = app.status_of(id)
    else {
        app.view = View::Connections;
        return;
    };
    let color = theme::tag_color(connection.tag);
    egui::Panel::top("workspace-top")
        .frame(egui::Frame::new().fill(color).inner_margin(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("← Conexões").clicked() {
                    app.view = View::Connections;
                }
                ui.label(
                    egui::RichText::new(format!(
                        "{} | PostgreSQL {} : {}",
                        connection.tag.label().to_uppercase(),
                        server_version,
                        connection.name
                    ))
                    .strong()
                    .color(egui::Color32::WHITE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Desconectar").clicked() {
                        app.deps.sessions.disconnect(id);
                    }
                });
            });
        });
    egui::Panel::left("workspace-left")
        .default_size(240.0)
        .show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Items").strong());
                ui.label(egui::RichText::new("Queries").color(tokens.text_muted));
                ui.label(egui::RichText::new("History").color(tokens.text_muted));
            });
            ui.separator();
            ui.label(
                egui::RichText::new("Bancos")
                    .small()
                    .color(tokens.text_muted),
            );
            egui::ScrollArea::vertical().show(ui, |ui| {
                for db in &databases {
                    let current = db.name == connection.config.database;
                    ui.label(if current {
                        egui::RichText::new(&db.name).strong()
                    } else {
                        egui::RichText::new(&db.name)
                    });
                }
            });
        });
    egui::Panel::right("workspace-right")
        .default_size(220.0)
        .show(ui, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Nenhuma linha selecionada").color(tokens.text_muted));
            });
        });
    egui::CentralPanel::default_margins().show(ui, |ui| {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("Nenhum banco selecionado. ⌘K para escolher (M2).")
                    .color(tokens.text_muted),
            );
        });
    });
}
