//! A paleta ⌘K (tela 6 do protótipo da FEAT-015): diálogo com fundo escurecido no alto da tela,
//! busca entre os bancos da conexão aberta, setas e Enter ou clique selecionam, Esc e clique fora
//! fecham. A lista rola e segue a seleção.

use hy_dbplus_engine::{ConnectionId, Driver};

use crate::app::{App, Palette, Status};
use crate::fonts::icon;
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
    let tr = app.tr();
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
    let modal_id = egui::Id::new("palette");
    let response = egui::Modal::new(modal_id)
        .area(egui::Modal::default_area(modal_id).anchor(egui::Align2::CENTER_TOP, [0.0, 80.0]))
        .frame(
            egui::Frame::new()
                .fill(tokens.card)
                .stroke(egui::Stroke::new(1.0, tokens.border))
                .corner_radius(theme::radius::WINDOW)
                .inner_margin(theme::space::M),
        )
        .show(ctx, |ui| {
            ui.set_width(480.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(icon::SEARCH).color(tokens.text_faint));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("esc")
                            .size(theme::text_size::SMALL)
                            .color(tokens.text_faint),
                    );
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut palette.query)
                            .hint_text(tr.search_databases())
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY),
                    );
                    if palette.focus_pending {
                        edit.request_focus();
                        palette.focus_pending = false;
                    }
                });
            });
            ui.separator();
            // As teclas antes da lista: a seleção nova já sai desenhada e rolada neste frame.
            let mut moved = false;
            ui.input(|input| {
                if input.key_pressed(egui::Key::Escape) {
                    close = true;
                }
                if input.key_pressed(egui::Key::ArrowDown) && !matches.is_empty() {
                    palette.selected = (palette.selected + 1) % matches.len();
                    moved = true;
                }
                if input.key_pressed(egui::Key::ArrowUp) && !matches.is_empty() {
                    palette.selected = (palette.selected + matches.len() - 1) % matches.len();
                    moved = true;
                }
                if input.key_pressed(egui::Key::Enter) {
                    chosen = matches.get(palette.selected).cloned();
                }
            });
            if matches.is_empty() {
                ui.label(egui::RichText::new(tr.no_database_match()).color(tokens.text_muted));
                return;
            }
            ui.label(
                egui::RichText::new(tr.databases())
                    .size(theme::text_size::SMALL)
                    .color(tokens.text_faint),
            );
            // A lista rola dentro da janela: com muitos bancos ela passava do fim da tela (achado
            // do Jean, 17/11/2026). A seleção pelas setas rola junto.
            let max_height = (ctx.content_rect().height() - 220.0).max(120.0);
            egui::ScrollArea::vertical()
                .max_height(max_height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for (i, name) in matches.iter().enumerate() {
                        let selected = i == palette.selected;
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(icon::DATABASE).color(tokens.text_muted),
                                )
                                .selectable(false),
                            );
                            let response = ui.selectable_label(selected, name);
                            if response.clicked() {
                                chosen = Some(name.clone());
                            }
                            if selected {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!("↩ {}", tr.open_hint()))
                                                .size(theme::text_size::SMALL)
                                                .color(tokens.text_faint),
                                        );
                                    },
                                );
                                if moved {
                                    response.scroll_to_me(None);
                                }
                            }
                        });
                    }
                });
        });
    if response.should_close() {
        close = true;
    }
    if let Some(database) = chosen {
        app.palette = None;
        app.open_database(id, &database);
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
