//! A aba "SQL Query" (FEAT-010, tela 3.7): barra com limite, Beautify ⌘I, Run Current ⌘↩, Run All
//! ⌘⇧↩ e Cancelar; o editor com highlight (layouter sobre `sql::highlight`), gutter de linhas,
//! fundo leve no statement sob o cursor e a barra de status `linha, coluna`; os resultados embaixo
//! (`results.rs`). O buffer é do `QueryTab`; o cursor é lido do estado do `TextEdit` a cada frame.

use hy_dbplus_engine::Driver;
use hy_dbplus_sql::{Kind, highlight, statement_at};

use crate::app::{App, DbKey};
use crate::{results, theme};

pub const LIMITS: [Option<usize>; 4] = [None, Some(100), Some(500), Some(1000)];

/// O id do `TextEdit` da aba (a barra de abas o usa para deixar o ⌘W passar com o foco nele).
pub fn editor_id(key: &DbKey, tab: hy_dbplus_engine::QueryTabId) -> egui::Id {
    egui::Id::new(("sql-editor", key, tab))
}

/// O que a toolbar e o editor precisam da aba, sem os resultados (que podem ter milhares de
/// linhas e não cabem num clone por frame).
struct Header {
    id: hy_dbplus_engine::QueryTabId,
    buffer: String,
    cursor_byte: usize,
    limit: Option<usize>,
    running: bool,
    started: Option<std::time::Instant>,
    notice: Option<String>,
}

fn limit_label(limit: Option<usize>) -> String {
    limit.map_or_else(|| "No limit".to_owned(), |n| n.to_string())
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
    let open_transaction = ws.open_transaction;
    let transaction_aborted = ws.transaction_aborted;
    let Some(q) = ws.query().map(|q| Header {
        id: q.id,
        buffer: q.buffer.clone(),
        cursor_byte: q.cursor_byte(),
        limit: q.limit,
        running: q.running,
        started: q.started,
        notice: q.notice.clone(),
    }) else {
        return;
    };
    let mut run_current = false;
    let mut run_all = false;
    let mut beautify = false;
    let mut cancel = false;
    let mut new_limit: Option<Option<usize>> = None;

    // Atalhos, antes dos widgets (consumidos aqui, o TextEdit não os vê).
    let cmd = |key: egui::Key| egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key);
    let cmd_shift = |key: egui::Key| {
        egui::KeyboardShortcut::new(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, key)
    };
    ui.input_mut(|i| {
        if i.consume_shortcut(&cmd_shift(egui::Key::Enter)) {
            run_all = true;
        } else if i.consume_shortcut(&cmd(egui::Key::Enter)) {
            run_current = true;
        }
        if i.consume_shortcut(&cmd(egui::Key::I)) && !q.running {
            beautify = true;
        }
    });

    egui::Panel::top(egui::Id::new(("query-toolbar", key, q.id))).show(ui, |ui| {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt(("query-limit", key, q.id))
                .selected_text(limit_label(q.limit))
                .show_ui(ui, |ui| {
                    for limit in LIMITS {
                        if ui
                            .selectable_label(q.limit == limit, limit_label(limit))
                            .clicked()
                            && limit != q.limit
                        {
                            new_limit = Some(limit);
                        }
                    }
                });
            ui.label(
                egui::RichText::new("linhas")
                    .small()
                    .color(tokens.text_muted),
            );
            ui.separator();
            if ui
                .add_enabled(!q.running, egui::Button::new("Beautify ⌘I"))
                .clicked()
            {
                beautify = true;
            }
            let has_text = !q.buffer.trim().is_empty();
            if ui
                .add_enabled(!q.running && has_text, egui::Button::new("Run Current ⌘↩"))
                .on_hover_text("Executa o statement sob o cursor")
                .clicked()
            {
                run_current = true;
            }
            if ui
                .add_enabled(!q.running && has_text, egui::Button::new("Run All ⌘⇧↩"))
                .on_hover_text("Executa todos os statements, parando no primeiro erro")
                .clicked()
            {
                run_all = true;
            }
            if ui
                .add_enabled(q.running, egui::Button::new("Cancelar"))
                .clicked()
            {
                cancel = true;
            }
            if q.running {
                let elapsed = q.started.map(|s| s.elapsed().as_secs_f32()).unwrap_or(0.0);
                ui.label(
                    egui::RichText::new(format!("executando… {elapsed:.1} s"))
                        .small()
                        .color(tokens.text_muted),
                );
                ui.ctx().request_repaint();
            }
            if transaction_aborted {
                ui.label(
                    egui::RichText::new("transação abortada")
                        .small()
                        .color(theme::error_color()),
                )
                .on_hover_text(
                    "Um statement falhou dentro da transação: o servidor desfaz tudo no COMMIT. \
                     Rode ROLLBACK e refaça.",
                );
            } else if open_transaction {
                ui.label(
                    egui::RichText::new("transação aberta")
                        .small()
                        .color(theme::error_color()),
                )
                .on_hover_text("O último BEGIN ainda não teve COMMIT nem ROLLBACK");
            }
            if let Some(notice) = &q.notice {
                ui.label(egui::RichText::new(notice).small().color(tokens.text_muted));
            }
        });
    });

    results::show(app, ui, key);

    // O editor: gutter com os números de linha e o TextEdit com o layouter de highlight.
    let mut buffer = q.buffer.clone();
    let cursor_byte = q.cursor_byte;
    let current = statement_at(&buffer, cursor_byte).map(|s| s.range);
    let font = egui::FontId::monospace(13.0);
    let colors = Colors {
        keyword: tokens.accent,
        string: egui::Color32::from_rgb(0x3A, 0x8F, 0x5C),
        number: egui::Color32::from_rgb(0xB0, 0x62, 0x1F),
        comment: tokens.text_muted,
        text: tokens.text,
        current: tokens.hover,
    };
    let editor_id = editor_id(key, q.id);
    let mut changed = false;
    egui::CentralPanel::default_margins().show(ui, |ui| {
        egui::Panel::bottom(egui::Id::new(("query-status", key, q.id))).show(ui, |ui| {
            let (line, column) = line_column(&buffer, cursor_byte);
            ui.label(
                egui::RichText::new(format!("linha {line}, coluna {column}"))
                    .small()
                    .color(tokens.text_muted),
            );
        });
        egui::ScrollArea::both()
            .id_salt(("sql-scroll", key, q.id))
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    let row_height = ui.fonts_mut(|f| f.row_height(&font));
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.add_space(4.0);
                        let lines = buffer.split('\n').count().max(1);
                        for n in 1..=lines {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(36.0, row_height),
                                egui::Sense::hover(),
                            );
                            ui.painter().text(
                                rect.right_center(),
                                egui::Align2::RIGHT_CENTER,
                                n.to_string(),
                                font.clone(),
                                tokens.text_muted,
                            );
                        }
                    });
                    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
                        let job = layout(text.as_str(), &font, &colors, current.clone());
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut buffer)
                            .id(editor_id)
                            .code_editor()
                            .lock_focus(true)
                            .desired_width(f32::INFINITY)
                            .desired_rows(12)
                            .layouter(&mut layouter),
                    );
                    changed = response.changed();
                });
            });
    });
    // O cursor fica no estado do TextEdit; copiamos para a aba (statement do cursor, status).
    let cursor = egui::TextEdit::load_state(ui.ctx(), editor_id)
        .and_then(|state| state.cursor.char_range())
        .map(|range| range.primary.index.0);
    if let Some(q) = app.workspaces.get_mut(key).and_then(|ws| ws.query_mut()) {
        if changed {
            q.buffer = buffer;
        }
        if let Some(cursor) = cursor {
            q.cursor = cursor;
        }
        if let Some(limit) = new_limit {
            q.limit = limit;
        }
    }
    if beautify {
        app.beautify(key);
    }
    if cancel {
        app.cancel_sql(key);
    } else if run_all {
        app.run_all(key);
    } else if run_current {
        app.run_current(key);
    }
}

struct Colors {
    keyword: egui::Color32,
    string: egui::Color32,
    number: egui::Color32,
    comment: egui::Color32,
    text: egui::Color32,
    current: egui::Color32,
}

/// O `LayoutJob` do buffer: cada span do `sql::highlight` com a sua cor, o resto na cor do texto,
/// e o statement sob o cursor com fundo leve. Sem quebra de linha automática (ScrollArea).
fn layout(
    text: &str,
    font: &egui::FontId,
    colors: &Colors,
    current: Option<std::ops::Range<usize>>,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let format = |color: egui::Color32, start: usize, end: usize| egui::TextFormat {
        font_id: font.clone(),
        color,
        background: match &current {
            Some(range) if start >= range.start && end <= range.end => colors.current,
            _ => egui::Color32::TRANSPARENT,
        },
        ..Default::default()
    };
    let mut at = 0;
    for span in highlight(text) {
        if span.range.start > at
            && let Some(gap) = text.get(at..span.range.start)
        {
            job.append(gap, 0.0, format(colors.text, at, span.range.start));
        }
        let Some(piece) = text.get(span.range.clone()) else {
            continue;
        };
        let color = match span.kind {
            Kind::Keyword => colors.keyword,
            Kind::String => colors.string,
            Kind::Number => colors.number,
            Kind::Comment => colors.comment,
            Kind::Identifier | Kind::Operator | Kind::Plain => colors.text,
        };
        job.append(piece, 0.0, format(color, span.range.start, span.range.end));
        at = span.range.end;
    }
    if let Some(rest) = text.get(at..)
        && !rest.is_empty()
    {
        job.append(rest, 0.0, format(colors.text, at, text.len()));
    }
    job
}

/// Linha e coluna (1-based, em caracteres) do byte `at`.
fn line_column(buffer: &str, at: usize) -> (usize, usize) {
    let before = buffer.get(..at).unwrap_or(buffer);
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linha_e_coluna_contam_caracteres_a_partir_de_um() {
        let buffer = "select 'ação';\nselect 2";
        assert_eq!(line_column(buffer, 0), (1, 1));
        assert_eq!(line_column(buffer, buffer.len()), (2, 9));
        assert_eq!(line_column(buffer, buffer.find('\n').unwrap()), (1, 15));
    }

    #[test]
    fn o_layout_cobre_o_texto_inteiro_na_ordem() {
        let text = "select 'a' -- c\nfrom t where x = 'aber";
        let colors = Colors {
            keyword: egui::Color32::RED,
            string: egui::Color32::GREEN,
            number: egui::Color32::BLUE,
            comment: egui::Color32::GRAY,
            text: egui::Color32::WHITE,
            current: egui::Color32::BLACK,
        };
        let job = layout(text, &egui::FontId::monospace(13.0), &colors, Some(0..10));
        assert_eq!(job.text, text);
        let mut end = 0;
        for section in &job.sections {
            assert_eq!(
                section.byte_range.start.0, end,
                "sem buracos nem sobreposição"
            );
            end = section.byte_range.end.0;
        }
        assert_eq!(end, text.len());
        assert!(
            job.sections
                .iter()
                .any(|s| s.format.color == egui::Color32::RED)
        );
        assert!(
            job.sections
                .iter()
                .any(|s| s.format.background == egui::Color32::BLACK)
        );
    }
}
