//! O painel de registro (lado direito da aba Data): as linhas selecionadas em **Form** (nome, tipo e
//! valor por campo; "Múltiplos valores" onde a seleção difere) ou em **JSON** (tipos pelo catálogo,
//! montado pelo engine). Só leitura: a edição é a FEAT-007.

use hy_dbplus_engine::{Column, ConnectionId, Driver, record};

use crate::app::{App, PanelMode};
use crate::theme;

/// Valor maior que isto aparece truncado no Form até "mostrar tudo".
const PREVIEW_CHARS: usize = 300;

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, id: &ConnectionId)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
    let Some(ws) = app.workspaces.get(id) else {
        return;
    };
    let Some(tab) = ws.data.as_ref() else { return };
    let Some(rows) = tab.rows.as_ref() else {
        ui.label(egui::RichText::new("Carregando…").color(tokens.text_muted));
        return;
    };
    let selected: Vec<&[Option<String>]> = tab
        .selected_rows
        .iter()
        .filter_map(|i| rows.rows.get(*i).map(Vec::as_slice))
        .collect();
    if selected.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("Nenhuma linha selecionada").color(tokens.text_muted));
        });
        return;
    }
    let columns: Vec<Option<&Column>> = ws.catalog_columns(&tab.schema, &tab.table, &rows.columns);
    let title = match tab.selected_rows.len() {
        1 => format!(
            "Linha {}",
            tab.page.offset + tab.selected_rows.first().copied().unwrap_or(0) as u64 + 1
        ),
        n => format!("{n} linhas"),
    };
    let mut mode = app.panel_mode;
    let mut toggled_field: Option<String> = None;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.selectable_value(&mut mode, PanelMode::Json, "JSON");
            ui.selectable_value(&mut mode, PanelMode::Form, "Form");
        });
    });
    ui.separator();
    match mode {
        PanelMode::Form => {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, name) in rows.columns.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(name)
                                .small()
                                .strong()
                                .color(tokens.text_muted),
                        );
                        if let Some(column) = columns.get(i).copied().flatten() {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(&column.data_type)
                                            .small()
                                            .color(tokens.text_muted),
                                    );
                                },
                            );
                        }
                    });
                    let first = selected.first().and_then(|r| r.get(i)).cloned().flatten();
                    let same = selected
                        .iter()
                        .all(|r| r.get(i).cloned().flatten() == first);
                    if !same {
                        ui.label(
                            egui::RichText::new("Múltiplos valores")
                                .italics()
                                .color(tokens.text_muted),
                        );
                    } else {
                        match first {
                            None => {
                                ui.label(
                                    egui::RichText::new("NULL")
                                        .italics()
                                        .color(tokens.text_muted),
                                );
                            }
                            Some(value) => {
                                let expanded = tab.expanded_fields.contains(name);
                                let long = value.chars().count() > PREVIEW_CHARS;
                                let shown: String = if long && !expanded {
                                    value.chars().take(PREVIEW_CHARS).collect::<String>() + "…"
                                } else {
                                    value.clone()
                                };
                                ui.label(egui::RichText::new(shown).monospace());
                                if long
                                    && ui
                                        .small_button(if expanded {
                                            "mostrar menos"
                                        } else {
                                            "mostrar tudo"
                                        })
                                        .clicked()
                                {
                                    toggled_field = Some(name.clone());
                                }
                            }
                        }
                    }
                    ui.add_space(6.0);
                }
            });
        }
        PanelMode::Json => {
            let types: Vec<Option<String>> = columns
                .iter()
                .map(|c| c.map(|c| c.data_type.clone()))
                .collect();
            let json = record::to_json(&rows.columns, &types, &selected);
            if ui.button("Copiar").clicked() {
                ui.ctx().copy_text(json.clone());
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(json).monospace()).wrap());
            });
        }
    }
    app.panel_mode = mode;
    if let Some(field) = toggled_field
        && let Some(tab) = app.workspaces.get_mut(id).and_then(|ws| ws.data.as_mut())
        && !tab.expanded_fields.remove(&field)
    {
        tab.expanded_fields.insert(field);
    }
}
