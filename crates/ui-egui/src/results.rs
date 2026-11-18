//! Os resultados do editor SQL (FEAT-010; telas 13 a 15 do protótipo da FEAT-015): painel inferior
//! com uma sub-aba por statement executado ("Resultado 1 … N", com o ícone de ok ou de erro) e, à
//! direita, linhas e tempo, "truncado em L" ou "falhou em"; o grid só leitura, a mensagem "OK, N
//! linhas afetadas" ou o erro em faixa vermelha com o comando. Com a transação abortada, a faixa
//! âmbar oferece "Executar ROLLBACK" (AC-012). ⌘-clique numa linha mostra o JSON dela no painel
//! direito.

use egui_extras::{Column as TableColumn, TableBuilder};
use hy_dbplus_engine::Driver;

use crate::app::{App, DbKey};
use crate::fonts::icon;
use crate::grid::{NUMBER_WIDTH, ROW_HEIGHT, cell, header};
use crate::widgets::{self, AccessibleLabel, BannerKind};
use crate::{editor, i18n, theme};

/// A altura inicial do painel (o usuário arrasta a borda).
const PANEL_HEIGHT: f32 = 270.0;
/// A altura máxima do comando que falhou, sob o erro.
const FAILED_SQL_HEIGHT: f32 = 160.0;

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, key: &DbKey)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let tr = app.tr();
    // Empresta a aba (os resultados podem ter milhares de linhas: nada de clonar por frame); as
    // ações são aplicadas depois, fora do empréstimo.
    let Some(ws) = app.workspaces.get(key) else {
        return;
    };
    let aborted = ws.transaction_aborted;
    let Some(q) = ws.query() else {
        return;
    };
    let mut focus: Option<usize> = None;
    let mut select: Option<(usize, usize)> = None;
    let mut rollback = false;
    egui::Panel::bottom(egui::Id::new(("query-results", key, q.id)))
        .resizable(true)
        .default_size(PANEL_HEIGHT)
        .min_size(80.0)
        .frame(egui::Frame::new().fill(tokens.bg))
        .show(ui, |ui| {
            // A barra das sub-abas, com o resumo do resultado ativo à direita.
            egui::Frame::new()
                .fill(tokens.panel)
                .inner_margin(egui::Margin::symmetric(theme::space::S as i8, 0))
                .show(ui, |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), theme::size::FOOTER),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                            if q.results.is_empty() {
                                ui.label(
                                    egui::RichText::new(tr.results())
                                        .size(theme::text_size::SMALL)
                                        .color(tokens.text_muted),
                                );
                            }
                            for (i, r) in q.results.iter().enumerate() {
                                let title = tr.result_tab(r.index as u64 + 1);
                                let (mark, color) = match &r.result {
                                    Ok(_) => (icon::OK, tokens.text_muted),
                                    Err(_) => (icon::ERROR, tokens.danger),
                                };
                                let active = q.active_result == i;
                                let mut text = egui::RichText::new(format!("{mark} {title}"))
                                    .size(theme::text_size::SMALL);
                                text = if r.result.is_err() {
                                    text.color(color)
                                } else if active {
                                    text.color(tokens.text)
                                } else {
                                    text.color(tokens.text_muted)
                                };
                                if active {
                                    text = text.family(crate::fonts::strong(ui.ctx()));
                                }
                                if ui
                                    .selectable_label(active, text)
                                    .with_accessible_label(&title)
                                    .clicked()
                                {
                                    focus = Some(i);
                                }
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let summary = if q.running {
                                        None
                                    } else {
                                        q.results.get(q.active_result).and_then(|r| {
                                            let ms = r.elapsed.as_millis();
                                            match &r.result {
                                                Err(_) => Some(tr.failed_in(ms)),
                                                Ok(outcome) => outcome.rows.as_ref().map(|rows| {
                                                    let n = rows.rows.len() as u64;
                                                    let mut text = tr.result_rows(n, ms);
                                                    if outcome.truncated {
                                                        text.push_str(&format!(
                                                            " · {}",
                                                            tr.truncated_at(n)
                                                        ));
                                                    }
                                                    text
                                                }),
                                            }
                                        })
                                    };
                                    if let Some(summary) = summary {
                                        ui.label(
                                            egui::RichText::new(summary)
                                                .size(theme::text_size::SMALL)
                                                .color(tokens.text_muted),
                                        );
                                    }
                                },
                            );
                        },
                    );
                });
            ui.add_space(theme::space::S);
            let margin = egui::Margin::symmetric(theme::space::M as i8, 0);
            // A faixa do ROLLBACK (tela 15): sob o erro quando ele está na tela; com outra sub-aba
            // ativa, no alto, para não sumir.
            let offer_rollback = aborted && !q.running;
            let active_failed = q
                .results
                .get(q.active_result)
                .is_some_and(|r| r.result.is_err());
            let mut rollback_banner = |ui: &mut egui::Ui| {
                egui::Frame::new().inner_margin(margin).show(ui, |ui| {
                    if widgets::banner(
                        ui,
                        BannerKind::Warning,
                        tr.transaction_aborted(),
                        Some(tr.run_rollback()),
                    ) {
                        rollback = true;
                    }
                });
                ui.add_space(theme::space::S);
            };
            if offer_rollback && !active_failed {
                rollback_banner(ui);
            }
            if q.results.is_empty() {
                if q.running {
                    ui.add_space(theme::space::L);
                    ui.vertical_centered(|ui| {
                        widgets::loading(ui, &tr.running_query(1, q.run_total.max(1) as u64));
                    });
                } else {
                    let hint = tr.no_results_hint(
                        &i18n::shortcut_label(ui.ctx(), &editor::run_shortcut()),
                        &i18n::shortcut_label(ui.ctx(), &editor::run_all_shortcut()),
                    );
                    widgets::empty_state(ui, icon::SQL, tr.no_results_title(), &hint);
                }
                return;
            }
            if q.running {
                egui::Frame::new().inner_margin(margin).show(ui, |ui| {
                    widgets::loading(
                        ui,
                        &tr.running_query(
                            q.results.len() as u64 + 1,
                            q.run_total.max(q.results.len() + 1) as u64,
                        ),
                    );
                });
                ui.add_space(theme::space::S);
            }
            let Some(r) = q.results.get(q.active_result) else {
                return;
            };
            match &r.result {
                Err(error) => {
                    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
                        widgets::banner(ui, BannerKind::Error, error, None);
                        ui.add_space(theme::space::S);
                        widgets::sql_block(ui, "query-failed-sql", &r.sql, FAILED_SQL_HEIGHT);
                    });
                    ui.add_space(theme::space::S);
                    if offer_rollback {
                        rollback_banner(ui);
                    }
                }
                Ok(outcome) => match &outcome.rows {
                    None => {
                        egui::Frame::new().inner_margin(margin).show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(
                                    tr.affected_rows(outcome.affected, r.elapsed.as_millis()),
                                )
                                .monospace(),
                            );
                        });
                    }
                    Some(rows) => {
                        let modifiers = ui.input(|i| i.modifiers);
                        let selected = r.selected_row;
                        let result_index = q.active_result;
                        // Barra horizontal por fora da tabela, como no grid da aba Data: a Table do
                        // egui_extras só rola na vertical.
                        egui::ScrollArea::horizontal()
                            .id_salt(("results-hscroll", key, q.id, q.run_seq, result_index))
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                // O estado da tabela (larguras, rolagem vertical) também é por
                                // execução e por resultado.
                                let mut builder = TableBuilder::new(ui)
                                    .id_salt(("results-table", q.run_seq, result_index))
                                    .striped(true)
                                    .resizable(true)
                                    .sense(egui::Sense::click())
                                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                    .column(TableColumn::exact(NUMBER_WIDTH));
                                for _ in &rows.columns {
                                    builder = builder.column(
                                        TableColumn::initial(140.0).at_least(60.0).clip(true),
                                    );
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
                                                        egui::RichText::new(
                                                            (index + 1).to_string(),
                                                        )
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
                                                                        .monospace()
                                                                        .italics()
                                                                        .color(tokens.text_faint),
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
                            });
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
    if rollback {
        app.run_rollback(key);
    }
}
