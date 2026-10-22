//! A paleta ⌘K (tela 3.2): busca entre os bancos da conexão aberta; Enter ou clique seleciona,
//! Esc fecha.

use hy_dbplus_engine::{ConnectionId, Driver};

use crate::app::{App, Palette, Status};
use crate::theme;

/// O atalho: ⌘K no macOS, Ctrl+K nos outros (egui traduz `COMMAND`).
pub fn shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::K)
}

pub fn show<D>(app: &mut App<D>, ctx: &egui::Context, id: &ConnectionId)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let Status::Connected { databases, .. } = app.status_of(id) else {
        app.palette = None;
        return;
    };
    let Some(palette) = app.palette.as_mut() else {
        return;
    };
    let tokens = theme::current(ctx);
    let query = palette.query.trim().to_lowercase();
    let matches: Vec<String> = databases
        .iter()
        .map(|d| d.name.clone())
        .filter(|name| query.is_empty() || name.to_lowercase().contains(&query))
        .collect();
    if palette.selected >= matches.len() {
        palette.selected = 0;
    }
    let mut chosen: Option<String> = None;
    let mut close = false;
    egui::Window::new("Escolher banco")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
        .show(ctx, |ui| {
            ui.set_min_width(360.0);
            let edit = ui.add(
                egui::TextEdit::singleline(&mut palette.query)
                    .hint_text("Banco")
                    .desired_width(f32::INFINITY),
            );
            if palette.focus_pending {
                edit.request_focus();
                palette.focus_pending = false;
            }
            ui.separator();
            if matches.is_empty() {
                ui.label(
                    egui::RichText::new("Nenhum banco com esse nome.").color(tokens.text_muted),
                );
            }
            for (i, name) in matches.iter().enumerate() {
                let selected = i == palette.selected;
                if ui.selectable_label(selected, name).clicked() {
                    chosen = Some(name.clone());
                }
            }
            ui.input(|input| {
                if input.key_pressed(egui::Key::Escape) {
                    close = true;
                }
                if input.key_pressed(egui::Key::ArrowDown) && !matches.is_empty() {
                    palette.selected = (palette.selected + 1) % matches.len();
                }
                if input.key_pressed(egui::Key::ArrowUp) && !matches.is_empty() {
                    palette.selected = (palette.selected + matches.len() - 1) % matches.len();
                }
                if input.key_pressed(egui::Key::Enter) {
                    chosen = matches.get(palette.selected).cloned();
                }
            });
        });
    if let Some(database) = chosen {
        app.palette = None;
        select_database(app, id, &database);
    } else if close {
        app.palette = None;
    }
}

/// Abre a paleta (ou a fecha, se já estava aberta).
pub fn toggle<D: Driver>(app: &mut App<D>) {
    app.palette = if app.palette.is_some() {
        None
    } else {
        Some(Palette {
            focus_pending: true,
            ..Palette::default()
        })
    };
}

pub fn select_database<D>(app: &mut App<D>, id: &ConnectionId, database: &str)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    if let Some(ws) = app.workspaces.get_mut(id) {
        ws.loading = true;
        ws.error = None;
    }
    app.deps.sessions.select_database(id, database);
}
