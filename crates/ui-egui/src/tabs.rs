//! A barra de abas do banco ativo (FEAT-008): uma aba por tabela aberta, rótulo `schema.tabela`
//! (só `tabela` no schema atual), e as abas "SQL Query N" (FEAT-010, **+ SQL** ou ⌘T); × em cada
//! aba e ⌘W na ativa; fechar uma aba com edição pendente ou com texto passa pela confirmação. O
//! ponto na aba marca pendência (ou SQL em execução).

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
    let current_schema = ws.current_schema.clone();
    let active = ws.active_tab;
    let labels: Vec<(String, bool, bool)> = ws
        .tabs
        .iter()
        .map(|t| {
            let pending =
                t.table().is_some_and(|t| t.has_changes()) || t.query().is_some_and(|q| q.running);
            (t.title(current_schema.as_deref()), pending, t.busy())
        })
        .collect();
    let mut focus: Option<usize> = None;
    let mut close: Option<usize> = None;
    let mut new_query = false;
    if ui.input_mut(|i| {
        i.consume_shortcut(&egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND,
            egui::Key::T,
        ))
    }) {
        new_query = true;
    }
    // ⌘W vale sem campo em foco ou com o foco no editor de SQL da aba ativa (o que se perderia
    // passa pela confirmação, ver `Tab::would_lose_work`); noutro campo (busca da árvore, Form) é
    // dele. Não com a paleta aberta nem com algo rodando.
    let active_running = active.and_then(|i| labels.get(i)).is_some_and(|l| l.2);
    let editor_id = ws.query().map(|q| crate::editor::editor_id(key, q.id));
    let focused = ui.ctx().memory(|m| m.focused());
    let focus_ok = focused.is_none() || (focused.is_some() && focused == editor_id);
    if app.confirm_close.is_none()
        && app.palette.is_none()
        && focus_ok
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
                        .on_disabled_hover_text("Aguarde a execução terminar")
                        .clicked()
                    {
                        close = Some(i);
                    }
                    ui.add_space(6.0);
                }
                if ui
                    .small_button("+ SQL")
                    .on_hover_text("Uma aba de SQL neste banco (⌘T)")
                    .clicked()
                {
                    new_query = true;
                }
            });
        });
    });
    if new_query {
        app.open_query_tab(key);
    }
    if let Some(i) = close {
        app.request_close_tab(key, i);
    } else if let Some(i) = focus
        && let Some(ws) = app.workspaces.get_mut(key)
    {
        ws.active_tab = Some(i);
    }
}
