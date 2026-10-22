//! O painel de registro (lado direito da aba Data): as linhas selecionadas em **Form** (nome, tipo e
//! valor por campo; "Múltiplos valores" onde a seleção difere) ou em **JSON** (tipos pelo catálogo,
//! montado pelo engine). Com chave primária (FEAT-007) cada campo do Form é editável, com **NULL** e
//! **Reverter**; o valor digitado vale para todas as linhas selecionadas. Os valores mostrados são os
//! pendentes, quando há.

use hy_dbplus_engine::{Column, ConnectionId, Driver, record};

use crate::app::{App, PanelMode};
use crate::theme;

/// Valor maior que isto aparece truncado no Form (só leitura) até "mostrar tudo".
const PREVIEW_CHARS: usize = 300;

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, id: &ConnectionId)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
    let editable = app.editable(id);
    let Some(ws) = app.workspaces.get(id) else {
        return;
    };
    let Some(tab) = ws.data.as_ref() else { return };
    let Some(rows) = tab.rows.as_ref() else {
        ui.label(egui::RichText::new("Carregando…").color(tokens.text_muted));
        return;
    };
    let indexes: Vec<usize> = tab
        .selected_rows
        .iter()
        .copied()
        .filter(|i| *i < rows.rows.len())
        .collect();
    if indexes.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("Nenhuma linha selecionada").color(tokens.text_muted));
        });
        return;
    }
    // O que a seleção mostra: o valor pendente onde há, senão o da página. O Form editável olha só
    // as linhas vivas (as marcadas para apagar não recebem o que se digita, senão o campo "difere"
    // delas a cada tecla); o JSON mostra todas.
    let values_of = |rows_idx: &[usize]| -> Vec<Vec<Option<String>>> {
        rows_idx
            .iter()
            .map(|row| {
                (0..rows.columns.len())
                    .map(|col| tab.cell_value(*row, col).flatten().map(str::to_owned))
                    .collect()
            })
            .collect()
    };
    let live: Vec<usize> = indexes
        .iter()
        .copied()
        .filter(|i| !tab.deleted.contains(i))
        .collect();
    let all_deleted = live.is_empty();
    let all_values = values_of(&indexes);
    let selected = if editable && !all_deleted {
        values_of(&live)
    } else {
        all_values.clone()
    };
    let columns: Vec<Option<&Column>> = ws.catalog_columns(&tab.schema, &tab.table, &rows.columns);
    let title = match indexes.len() {
        1 => format!(
            "Linha {}",
            tab.page.offset + indexes.first().copied().unwrap_or(0) as u64 + 1
        ),
        n => format!("{n} linhas"),
    };
    let mut mode = app.panel_mode;
    let mut toggled_field: Option<String> = None;
    let mut field_edits: Vec<(String, Option<String>)> = Vec::new();
    let mut reverts: Vec<String> = Vec::new();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).strong());
        if all_deleted {
            ui.label(
                egui::RichText::new("marcada para apagar")
                    .small()
                    .color(theme::error_color()),
            );
        }
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
                    let pending_any = indexes.iter().any(|row| tab.is_pending(*row, name));
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(name)
                                .small()
                                .strong()
                                .color(tokens.text_muted),
                        );
                        if pending_any {
                            ui.label(egui::RichText::new("alterado").small().color(tokens.accent));
                        }
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
                    let first: Option<String> =
                        selected.first().and_then(|r| r.get(i)).cloned().flatten();
                    let same = selected
                        .iter()
                        .all(|r| r.get(i).cloned().flatten() == first);
                    if editable && !all_deleted {
                        // Campo editável: o texto é o valor comum (vazio se NULL ou se difere) e o
                        // que se digita vale para todas as linhas selecionadas.
                        let mut text = if same {
                            first.clone().unwrap_or_default()
                        } else {
                            String::new()
                        };
                        let hint = if !same {
                            "Múltiplos valores"
                        } else if first.is_none() {
                            "NULL"
                        } else {
                            ""
                        };
                        let field = ui.add(
                            egui::TextEdit::multiline(&mut text)
                                .id_salt(("form-field", i))
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(1)
                                .desired_width(f32::INFINITY)
                                .hint_text(hint),
                        );
                        if field.changed() {
                            field_edits.push((name.clone(), Some(text)));
                        }
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    !same || first.is_some(),
                                    egui::Button::new("NULL").small(),
                                )
                                .on_hover_text("Põe NULL nas linhas selecionadas")
                                .clicked()
                            {
                                field_edits.push((name.clone(), None));
                            }
                            if pending_any && ui.small_button("Reverter").clicked() {
                                reverts.push(name.clone());
                            }
                        });
                    } else if !same {
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
            let slices: Vec<&[Option<String>]> = all_values.iter().map(Vec::as_slice).collect();
            let json = record::to_json(&rows.columns, &types, &slices);
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
    for (column, value) in field_edits {
        app.set_field(id, &column, value);
    }
    for column in reverts {
        app.revert_field(id, &column);
    }
}
