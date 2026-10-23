//! Os resultados do editor SQL (FEAT-010): painel inferior com uma sub-aba por statement
//! executado (`Query 1 … N`), cada uma com o grid só leitura (`N rows · tempo`, `truncado em L`),
//! a mensagem `OK, N linhas afetadas · tempo` ou o erro em vermelho com o statement. ⌘-clique numa
//! linha mostra o JSON dela no painel direito.

use egui_extras::{Column as TableColumn, TableBuilder};
use hy_dbplus_engine::Driver;

use crate::app::{App, DbKey};
use crate::grid::{ROW_HEIGHT, cell, header};
use crate::theme;

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, key: &DbKey)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    // Empresta a aba (os resultados podem ter milhares de linhas: nada de clonar por frame); as
    // ações são aplicadas depois, fora do empréstimo.
    let Some(q) = app.workspaces.get(key).and_then(|ws| ws.query()) else {
        return;
    };
    let mut focus: Option<usize> = None;
    let mut select: Option<(usize, usize)> = None;
    egui::Panel::bottom(egui::Id::new(("query-results", key, q.id)))
        .resizable(true)
        .default_size(260.0)
        .min_size(80.0)
        .show(ui, |ui| {
            if q.results.is_empty() {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(
                        "Nenhum resultado ainda. ⌘↩ roda o statement sob o cursor; ⌘⇧↩ roda todos.",
                    )
                    .color(tokens.text_muted),
                );
                return;
            }
            ui.horizontal(|ui| {
                for (i, r) in q.results.iter().enumerate() {
                    let mut text = egui::RichText::new(format!("Query {}", r.index + 1));
                    if r.result.is_err() {
                        text = text.color(theme::error_color());
                    }
                    if ui.selectable_label(q.active_result == i, text).clicked() {
                        focus = Some(i);
                    }
                }
            });
            ui.separator();
            let Some(r) = q.results.get(q.active_result) else {
                return;
            };
            let ms = r.elapsed.as_millis();
            match &r.result {
                Err(error) => {
                    ui.colored_label(theme::error_color(), error);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&r.sql)
                                .monospace()
                                .color(tokens.text_muted),
                        )
                        .wrap(),
                    );
                }
                Ok(outcome) => match &outcome.rows {
                    None => {
                        let text = match outcome.affected {
                            Some(n) => format!("OK, {n} linhas afetadas · {ms} ms"),
                            None => format!("OK · {ms} ms"),
                        };
                        ui.label(egui::RichText::new(text).monospace());
                    }
                    Some(rows) => {
                        // Truncado: as linhas que ficaram são exatamente o limite daquela execução.
                        let mut footer = format!("{} rows · {ms} ms", rows.rows.len());
                        if outcome.truncated {
                            footer.push_str(&format!(" · truncado em {}", rows.rows.len()));
                        }
                        ui.label(egui::RichText::new(footer).small().color(tokens.text_muted));
                        let modifiers = ui.input(|i| i.modifiers);
                        let selected = r.selected_row;
                        let result_index = q.active_result;
                        {
                            let ui = &mut *ui;
                            let mut builder = TableBuilder::new(ui)
                                .striped(true)
                                .resizable(true)
                                .sense(egui::Sense::click())
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .column(TableColumn::exact(44.0));
                            for _ in &rows.columns {
                                builder = builder
                                    .column(TableColumn::initial(140.0).at_least(60.0).clip(true));
                            }
                            builder
                                .header(ROW_HEIGHT + 4.0, |mut row| {
                                    row.col(|ui| header(ui, "#", &tokens));
                                    for name in &rows.columns {
                                        row.col(|ui| header(ui, name, &tokens));
                                    }
                                })
                                .body(|body| {
                                    body.rows(ROW_HEIGHT, rows.rows.len(), |mut row| {
                                        let index = row.index();
                                        row.set_selected(selected == Some(index));
                                        // O número da linha é um botão (como no grid de dados):
                                        // alvo do ⌘-clique também por acessibilidade.
                                        row.col(|ui| {
                                            if ui
                                                .selectable_label(
                                                    selected == Some(index),
                                                    egui::RichText::new((index + 1).to_string())
                                                        .small(),
                                                )
                                                .clicked()
                                                && modifiers.command
                                            {
                                                select = Some((result_index, index));
                                            }
                                        });
                                        if let Some(cells) = rows.rows.get(index) {
                                            for value in cells {
                                                row.col(|ui| match value {
                                                    Some(v) => cell(ui, v),
                                                    None => {
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new("NULL")
                                                                    .italics()
                                                                    .color(tokens.text_muted),
                                                            )
                                                            .selectable(false),
                                                        );
                                                    }
                                                });
                                            }
                                        }
                                        if row.response().clicked() && modifiers.command {
                                            select = Some((result_index, index));
                                        }
                                    });
                                });
                        }
                    }
                },
            }
        });
    if let Some(q) = app.workspaces.get_mut(key).and_then(|ws| ws.query_mut()) {
        if let Some(i) = focus {
            q.active_result = i;
        }
        if let Some((result, row)) = select
            && let Some(r) = q.results.get_mut(result)
        {
            r.selected_row = if r.selected_row == Some(row) {
                None
            } else {
                Some(row)
            };
        }
    }
    if select.is_some() {
        app.side_panel_open = true;
    }
}
