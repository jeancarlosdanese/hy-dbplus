//! A caixa de Commit (FEAT-007): o SQL exato que vai rodar, as contagens, Executar e Cancelar.
//! Erro do servidor fica na caixa com o statement que falhou destacado (statement ou contagem:
//! desfeito; conexão caída no COMMIT: a mensagem do driver diz) e as pendências continuam no grid.
//! Esc ou clique fora fecham a caixa quando nada está rodando.

use hy_dbplus_engine::{ConnectionId, Driver};

use crate::app::App;
use crate::theme;

pub fn show<D>(app: &mut App<D>, ctx: &egui::Context, id: &ConnectionId)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let Some(dialog) = app
        .workspaces
        .get(id)
        .and_then(|ws| ws.data.as_ref())
        .and_then(|tab| tab.commit.clone())
    else {
        return;
    };
    let tokens = theme::current(ctx);
    let mut execute = false;
    let mut cancel = false;
    let response = egui::Modal::new(egui::Id::new("commit-dialog")).show(ctx, |ui| {
        ui.set_width(560.0);
        ui.heading(format!(
            "{} updates, {} deletes",
            dialog.updates, dialog.deletes
        ));
        ui.label(
            egui::RichText::new("Numa transação: tudo ou nada.")
                .small()
                .color(tokens.text_muted),
        );
        ui.separator();
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                for (i, statement) in dialog.sql.iter().enumerate() {
                    let failed = dialog.failed == Some(i + 1);
                    let mut text = egui::RichText::new(format!("{statement};")).monospace();
                    if failed {
                        text = text.color(theme::error_color());
                    }
                    ui.add(egui::Label::new(text).wrap());
                    ui.add_space(4.0);
                }
            });
        if let Some(error) = &dialog.error {
            ui.separator();
            ui.add(egui::Label::new(egui::RichText::new(error).color(theme::error_color())).wrap());
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Copiar").clicked() {
                ui.ctx().copy_text(
                    dialog
                        .sql
                        .iter()
                        .map(|s| format!("{s};"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!dialog.running, egui::Button::new("Executar"))
                    .clicked()
                {
                    execute = true;
                }
                if ui
                    .add_enabled(!dialog.running, egui::Button::new("Cancelar"))
                    .clicked()
                {
                    cancel = true;
                }
                if dialog.running {
                    ui.label(
                        egui::RichText::new("Executando…")
                            .small()
                            .color(tokens.text_muted),
                    );
                }
            });
        });
    });
    if response.should_close() && !dialog.running {
        cancel = true;
    }
    if execute {
        app.execute_changes(id);
    } else if cancel {
        app.cancel_commit(id);
    }
}
