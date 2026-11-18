//! O painel de registro (lado direito da aba Dados, tela 8): as linhas selecionadas em **Campos**
//! (nome, tipo e valor por campo; "Múltiplos valores" onde a seleção difere) ou em **JSON** (tipos pelo catálogo,
//! montado pelo engine). Com chave primária (FEAT-007) cada campo do Form é editável, com **NULL** e
//! **Reverter**; o valor digitado vale para todas as linhas selecionadas. Os valores mostrados são os
//! pendentes, quando há.

use hy_dbplus_engine::{Column, Driver, record};

use crate::app::{App, DbKey, PanelMode, TabView};
use crate::fonts::icon;
use crate::{i18n, theme, widgets};

/// Valor maior que isto aparece truncado no Form (só leitura) até "mostrar tudo".
const PREVIEW_CHARS: usize = 300;

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, key: &DbKey)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let tr = app.tr();
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
    let editable = app.editable(key);
    let Some(ws) = app.workspaces.get(key) else {
        return;
    };
    // Aba de SQL: só o JSON da linha escolhida com ⌘-clique no resultado (sem tipos do catálogo).
    if let Some(q) = ws.query() {
        let selected = q
            .results
            .get(q.active_result)
            .and_then(|r| Some((r, r.selected_row?)))
            .and_then(|(r, row)| {
                r.result
                    .as_ref()
                    .ok()
                    .and_then(|o| o.rows.as_ref())
                    .and_then(|rows| Some((rows.columns.clone(), rows.rows.get(row)?.clone())))
            });
        match selected {
            None => {
                let hint = tr.no_row_selected_sql(i18n::command_label(ui.ctx()));
                ui.centered_and_justified(|ui| {
                    ui.label(egui::RichText::new(hint).color(tokens.text_muted));
                });
            }
            Some((columns, row)) => {
                let types: Vec<Option<String>> = vec![None; columns.len()];
                let json = record::to_json(&columns, &types, &[row.as_slice()]);
                if widgets::button(ui, widgets::ButtonKind::Normal, Some(icon::COPY), tr.copy())
                    .clicked()
                {
                    ui.ctx().copy_text(json.clone());
                }
                ui.add_space(theme::space::S);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add(egui::Label::new(egui::RichText::new(json).monospace()).wrap());
                });
            }
        }
        return;
    }
    let Some(tab) = ws.data() else { return };
    // Em Structure e Index o grid não está na tela: nada de Form editável sobre uma linha invisível.
    if tab.view != TabView::Data {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new(tr.no_row_selected()).color(tokens.text_muted));
        });
        return;
    }
    let Some(rows) = tab.rows.as_ref() else {
        widgets::loading(ui, tr.loading());
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
            ui.label(egui::RichText::new(tr.no_row_selected()).color(tokens.text_muted));
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
        1 => tr.row_title(tab.page.offset + indexes.first().copied().unwrap_or(0) as u64 + 1),
        n => tr.rows(n as u64),
    };
    let mut mode = app.panel_mode;
    let mut toggled_field: Option<String> = None;
    let mut field_edits: Vec<(String, Option<String>)> = Vec::new();
    let mut reverts: Vec<String> = Vec::new();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(title)
                .family(crate::fonts::strong(ui.ctx()))
                .color(tokens.text),
        );
        if all_deleted {
            ui.label(
                egui::RichText::new(tr.marked_for_deletion())
                    .size(theme::text_size::SMALL)
                    .color(tokens.danger),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            widgets::segmented(
                ui,
                &mut mode,
                &[(PanelMode::Form, tr.fields()), (PanelMode::Json, "JSON")],
            );
        });
    });
    ui.add_space(theme::space::S);
    match mode {
        PanelMode::Form => {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, name) in rows.columns.iter().enumerate() {
                    let pending_any = indexes.iter().any(|row| tab.is_pending(*row, name));
                    let column = columns.get(i).copied().flatten();
                    let primary = column.is_some_and(|c| c.is_primary_key);
                    ui.horizontal(|ui| {
                        let shown = if primary {
                            format!("{} {name}", icon::KEY)
                        } else {
                            name.clone()
                        };
                        ui.label(
                            egui::RichText::new(shown)
                                .size(theme::text_size::SMALL)
                                .family(crate::fonts::strong(ui.ctx()))
                                .color(tokens.text_muted),
                        );
                        if pending_any {
                            widgets::pending_chip(ui, tr.changed());
                        }
                        if let Some(column) = column {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(&column.data_type)
                                            .size(theme::text_size::SMALL)
                                            .color(tokens.text_faint),
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
                            tr.multiple_values()
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
                                .background_color(if pending_any {
                                    tokens.pending
                                } else {
                                    tokens.field
                                })
                                .text_color(if pending_any {
                                    tokens.pending_ink
                                } else {
                                    tokens.text
                                })
                                .hint_text(hint),
                        );
                        if field.changed() {
                            field_edits.push((name.clone(), Some(text)));
                        }
                        ui.horizontal(|ui| {
                            if widgets::button_enabled(
                                ui,
                                !same || first.is_some(),
                                widgets::ButtonKind::Ghost,
                                None,
                                tr.set_null(),
                                "",
                            )
                            .on_hover_text(tr.set_null_hint())
                            .clicked()
                            {
                                field_edits.push((name.clone(), None));
                            }
                            if pending_any
                                && widgets::button(
                                    ui,
                                    widgets::ButtonKind::Ghost,
                                    Some(icon::UNDO),
                                    tr.revert_field(),
                                )
                                .clicked()
                            {
                                reverts.push(name.clone());
                            }
                        });
                    } else if !same {
                        ui.label(
                            egui::RichText::new(tr.multiple_values())
                                .italics()
                                .color(tokens.text_muted),
                        );
                    } else {
                        match first {
                            None => {
                                ui.label(
                                    egui::RichText::new("NULL")
                                        .italics()
                                        .color(tokens.text_faint),
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
                                            tr.show_less()
                                        } else {
                                            tr.show_all()
                                        })
                                        .clicked()
                                {
                                    toggled_field = Some(name.clone());
                                }
                            }
                        }
                    }
                    ui.add_space(theme::space::M);
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
            if widgets::button(ui, widgets::ButtonKind::Normal, Some(icon::COPY), tr.copy())
                .clicked()
            {
                ui.ctx().copy_text(json.clone());
            }
            ui.add_space(theme::space::S);
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(json).monospace()).wrap());
            });
        }
    }
    app.panel_mode = mode;
    if let Some(field) = toggled_field
        && let Some(tab) = app.workspaces.get_mut(key).and_then(|ws| ws.data_mut())
        && !tab.expanded_fields.remove(&field)
    {
        tab.expanded_fields.insert(field);
    }
    for (column, value) in field_edits {
        app.set_field(key, &column, value);
    }
    for column in reverts {
        app.revert_field(key, &column);
    }
}
