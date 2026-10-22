//! A barra de abas de tabela do banco ativo (FEAT-008): uma aba por tabela aberta, rótulo
//! `schema.tabela` (só `tabela` no schema atual), × em cada aba e ⌘W na ativa; fechar uma aba com
//! edição pendente passa pela confirmação. O ponto na aba marca pendência.

use hy_dbplus_engine::Driver;

use crate::app::{App, DbKey};
use crate::theme;

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, key: &DbKey)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let Some(ws) = app.workspaces.get(key) else {
        return;
    };
    if ws.tabs.is_empty() {
        return;
    }
    let current_schema = ws.current_schema.clone();
    let active = ws.active_tab;
    let labels: Vec<(String, bool, bool)> = ws
        .tabs
        .iter()
        .map(|t| {
            let label = if current_schema.as_deref() == Some(t.schema.as_str()) {
                t.table.clone()
            } else {
                format!("{}.{}", t.schema, t.table)
            };
            (label, t.has_changes(), t.commit_running())
        })
        .collect();
    let mut focus: Option<usize> = None;
    let mut close: Option<usize> = None;
    // ⌘W só sem campo em foco (num campo é dele) e sem Commit rodando na ativa.
    let nothing_focused = ui.ctx().memory(|m| m.focused().is_none());
    let active_running = active.and_then(|i| labels.get(i)).is_some_and(|l| l.2);
    if app.confirm_close.is_none()
        && nothing_focused
        && !active_running
        && ui.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::W,
            ))
        })
    {
        close = active;
    }
    egui::Panel::top(egui::Id::new(("workspace-tabs", key))).show(ui, |ui| {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, (label, pending, running)) in labels.iter().enumerate() {
                    let selected = active == Some(i);
                    let mut text = egui::RichText::new(if *pending {
                        format!("● {label}")
                    } else {
                        label.clone()
                    });
                    if *pending {
                        text = text.color(tokens.accent);
                    }
                    if ui.selectable_label(selected, text).clicked() {
                        focus = Some(i);
                    }
                    if ui
                        .add_enabled(!running, egui::Button::new("×").small())
                        .on_hover_text("Fechar a aba (⌘W fecha a ativa)")
                        .on_disabled_hover_text("Aguarde o Commit terminar")
                        .clicked()
                    {
                        close = Some(i);
                    }
                    ui.add_space(6.0);
                }
            });
        });
    });
    if let Some(i) = close {
        app.request_close_tab(key, i);
    } else if let Some(i) = focus
        && let Some(ws) = app.workspaces.get_mut(key)
    {
        ws.active_tab = Some(i);
    }
}
