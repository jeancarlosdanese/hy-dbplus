//! A barra de abas do banco ativo (FEAT-008, desenho da FEAT-015): uma aba por tabela aberta,
//! rótulo `schema.tabela` (só `tabela` no schema atual), e as abas "Consulta N" (FEAT-010, "+" ou
//! ⌘T), cada uma com o ícone do tipo; × em cada aba e ⌘W na ativa; fechar uma aba com edição
//! pendente ou com texto passa pela confirmação. O ponto âmbar marca pendência (ou SQL em execução).

use hy_dbplus_engine::Driver;

use crate::app::{App, DbKey};
use crate::fonts::icon;
use crate::{i18n, theme, widgets};

/// ⌘W: fecha a aba ativa.
pub fn close_tab_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::W)
}

/// ⌘T: uma aba de SQL nova no banco ativo.
pub fn new_query_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::T)
}

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
    let tr = app.tr();
    // (título, pendência, ocupada, é SQL)
    let labels: Vec<(String, bool, bool, bool)> = ws
        .tabs
        .iter()
        .map(|t| {
            let pending =
                t.table().is_some_and(|t| t.has_changes()) || t.query().is_some_and(|q| q.running);
            (
                t.title(current_schema.as_deref(), tr),
                pending,
                t.busy(),
                t.query().is_some(),
            )
        })
        .collect();
    let mut focus: Option<usize> = None;
    let mut close: Option<usize> = None;
    let mut new_query = false;
    let dialog_open = app.table_dialog_open();
    if !dialog_open
        && app.confirm_close.is_none()
        && app.palette.is_none()
        && ui.input_mut(|i| i.consume_shortcut(&new_query_shortcut()))
    {
        new_query = true;
    }
    // ⌘W vale sem campo em foco ou com o foco no editor de SQL da aba ativa (o que se perderia
    // passa pela confirmação, ver `Tab::would_lose_work`); noutro campo (busca da árvore, Form) é
    // dele. Não com a paleta, a gravação ou o DDL abertos nem com algo rodando.
    let active_running = active.and_then(|i| labels.get(i)).is_some_and(|l| l.2);
    let editor_id = ws.query().map(|q| crate::editor::editor_id(key, q.id));
    let focused = ui.ctx().memory(|m| m.focused());
    let focus_ok = focused.is_none() || (focused.is_some() && focused == editor_id);
    if app.confirm_close.is_none()
        && app.palette.is_none()
        && !dialog_open
        && focus_ok
        && !active_running
        && ui.input_mut(|i| i.consume_shortcut(&close_tab_shortcut()))
    {
        close = active;
    }
    let new_query_label = i18n::shortcut_label(ui.ctx(), &new_query_shortcut());
    let close_label = i18n::shortcut_label(ui.ctx(), &close_tab_shortcut());
    egui::Panel::top(egui::Id::new(("workspace-tabs", key)))
        .frame(
            egui::Frame::new()
                .fill(tokens.panel)
                .inner_margin(egui::Margin {
                    left: theme::space::S as i8,
                    right: theme::space::S as i8,
                    top: theme::space::XS as i8,
                    bottom: 0,
                }),
        )
        .show(ui, |ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = theme::space::XS;
                    for (i, (label, pending, running, is_query)) in labels.iter().enumerate() {
                        let selected = active == Some(i);
                        // A aba ativa tem o fundo do conteúdo e fica "colada" nele.
                        let fill = if selected { tokens.bg } else { tokens.panel };
                        egui::Frame::new()
                            .fill(fill)
                            .corner_radius(egui::CornerRadius {
                                nw: theme::radius::CONTROL,
                                ne: theme::radius::CONTROL,
                                sw: 0,
                                se: 0,
                            })
                            .inner_margin(egui::Margin {
                                left: (theme::space::S + theme::space::HAIR) as i8,
                                right: theme::space::XS as i8,
                                top: theme::space::XS as i8,
                                bottom: theme::space::XS as i8,
                            })
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = theme::space::XS;
                                    let kind = if *is_query { icon::SQL } else { icon::TABLE };
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(kind).color(tokens.text_muted),
                                        )
                                        .selectable(false),
                                    );
                                    let text =
                                        egui::RichText::new(label.as_str()).color(if selected {
                                            tokens.text
                                        } else {
                                            tokens.text_muted
                                        });
                                    let response = ui.add(
                                        egui::Button::new(text).frame(false).selected(selected),
                                    );
                                    if response.clicked() {
                                        focus = Some(i);
                                    }
                                    if *pending {
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new("●")
                                                    .size(theme::text_size::DOT)
                                                    .color(tokens.warn),
                                            )
                                            .selectable(false),
                                        );
                                    }
                                    if widgets::icon_button_full(
                                        ui,
                                        !running,
                                        icon::CLOSE,
                                        tr.close_tab(),
                                        Some(&close_label),
                                        tr.wait_for_run(),
                                    )
                                    .clicked()
                                    {
                                        close = Some(i);
                                    }
                                });
                            });
                    }
                    if widgets::icon_button_full(
                        ui,
                        true,
                        icon::ADD,
                        tr.new_query(),
                        Some(&new_query_label),
                        "",
                    )
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
