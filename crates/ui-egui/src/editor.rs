//! A aba "SQL Query" (FEAT-010, tela 3.7): barra com limite, Beautify ⌘I, Run Current ⌘↩, Run All
//! ⌘⇧↩ e Cancelar; o editor com highlight (layouter sobre os spans da `sql::Analysis` da aba,
//! refeita só quando o texto muda), gutter de linhas,
//! fundo leve no statement sob o cursor e a barra de status `linha, coluna`; os resultados embaixo
//! (`results.rs`). O buffer é do `QueryTab`; o cursor é lido do estado do `TextEdit` a cada frame.

use hy_dbplus_engine::Driver;
use hy_dbplus_sql::{Analysis, CatalogView, ItemKind, Kind, Need, complete};

use crate::app::{App, CompletionState, DbKey};
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
    pending_cursor: Option<usize>,
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
    // Teclas do popup de autocomplete (FEAT-012), consumidas antes do TextEdit e só com o editor
    // focado no frame anterior (o Esc já tirou o foco neste; e sem foco as teclas são de outro
    // widget): com o popup aberto, ↑/↓ movem, Tab e Enter aceitam (quando há itens), Esc fecha;
    // ⌃Space abre. Shift junto é do editor (seleção, desindentar).
    let (popup_open, has_items, editor_focused) =
        match app.workspaces.get(key).and_then(|ws| ws.query()) {
            Some(q) => (
                q.completion.is_some(),
                q.completion.as_ref().is_some_and(|c| !c.items.is_empty()),
                ui.memory(|m| m.had_focus_last_frame(editor_id(key, q.id))),
            ),
            None => return,
        };
    let mut force_open = false;
    let (mut up, mut down, mut accept, mut close) = (false, false, false, false);
    let (mut pasted, mut command) = (false, false);
    ui.input_mut(|i| {
        pasted = i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)));
        command = i.modifiers.command;
        if !editor_focused || i.modifiers.shift {
            return;
        }
        force_open = i.consume_key(egui::Modifiers::CTRL, egui::Key::Space);
        if popup_open {
            up = i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp);
            down = i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown);
            if has_items {
                accept = i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
            }
            close = i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
        }
    });
    if popup_open && let Some(q) = app.workspaces.get_mut(key).and_then(|ws| ws.query_mut()) {
        if let Some(c) = q.completion.as_mut() {
            let len = c.items.len().max(1);
            if down {
                c.selected = (c.selected + 1) % len;
            }
            if up {
                c.selected = (c.selected + len - 1) % len;
            }
        }
        if accept {
            let index = q.completion.as_ref().map_or(0, |c| c.selected);
            q.accept_completion(index);
        } else if close {
            q.completion = None;
            // O egui solta o foco no Esc antes de qualquer widget ver a tecla: com o popup aberto
            // o Esc é dele, e o editor fica com o foco.
            let id = editor_id(key, q.id);
            ui.memory_mut(|m| m.request_focus(id));
        }
    }
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
        pending_cursor: q.pending_cursor,
    }) else {
        return;
    };
    // A análise do buffer sai da aba durante o frame (o layouter a refaz se o texto mudou) e
    // volta no fim; o cursor pendente é consumido aqui.
    let mut analysis = None;
    if let Some(tab) = app.workspaces.get_mut(key).and_then(|ws| ws.query_mut()) {
        tab.pending_cursor = None;
        analysis = tab.analysis.take();
    }
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
    let current = Analysis::refresh(&mut analysis, &buffer)
        .statement_at(cursor_byte)
        .map(|s| s.range.clone());
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
    let mut cursor_rect: Option<egui::Rect> = None;
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
                        // O texto que o TextEdit passa já tem a tecla deste frame: a análise é
                        // refeita só nesse caso, e fica guardada para os frames seguintes.
                        let analysis = Analysis::refresh(&mut analysis, text.as_str());
                        let job = layout(analysis, &font, &colors, current.clone());
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    // SQL gerado (FEAT-013): o cursor vai para o statement novo e o editor pega
                    // o foco; o estado do widget é a fonte do cursor, então é ele que muda.
                    if let Some(at) = q.pending_cursor {
                        let mut state =
                            egui::TextEdit::load_state(ui.ctx(), editor_id).unwrap_or_default();
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(
                                egui::text::CCursor::new(at),
                            )));
                        state.store(ui.ctx(), editor_id);
                        ui.memory_mut(|m| m.request_focus(editor_id));
                    }
                    let output = egui::TextEdit::multiline(&mut buffer)
                        .id(editor_id)
                        .code_editor()
                        .lock_focus(true)
                        .desired_width(f32::INFINITY)
                        .desired_rows(12)
                        .layouter(&mut layouter)
                        .show(ui);
                    changed = output.response.changed();
                    // Onde o popup ancora: embaixo do cursor, em coordenadas de tela.
                    cursor_rect = output.cursor_range.map(|r| {
                        output
                            .galley
                            .pos_from_cursor(r.primary)
                            .translate(output.galley_pos.to_vec2())
                    });
                });
            });
    });
    // O cursor fica no estado do TextEdit; copiamos para a aba (statement do cursor, status).
    let cursor = egui::TextEdit::load_state(ui.ctx(), editor_id)
        .and_then(|state| state.cursor.char_range())
        .map(|range| range.primary.index.0);
    // O popup: abre ao digitar identificador ou `.` (texto inserido, não apagado, colado nem
    // desfeito), ou por ⌃Space; fecha quando o texto muda de outro jeito, quando o cursor anda
    // sem texto, quando o editor perde o foco (sem cursor na tela), ou quando o motor não tem o
    // que sugerir.
    let cursor_moved = cursor.is_some_and(|c| c != cursor_chars_of(&buffer, q.cursor_byte));
    let typed_ident = changed
        && !pasted
        && !command
        && buffer.len() > q.buffer.len()
        && cursor
            .and_then(|c| byte_of(&buffer, c))
            .and_then(|b| buffer.get(..b)?.chars().last())
            .is_some_and(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.');
    let keep_open = cursor_rect.is_some()
        && ((popup_open
            && !accept
            && !close
            && (!changed || typed_ident)
            && (changed || !cursor_moved))
            || typed_ident
            || force_open);
    let mut popup: Option<CompletionState> = None;
    let mut needs: Vec<Need> = Vec::new();
    if keep_open && let Some(ws) = app.workspaces.get(key) {
        let view = CatalogView {
            current_schema: ws.current_schema.as_deref(),
            schemas: &ws.schemas,
            objects: &ws.objects,
            columns: &ws.columns,
        };
        let analysis = Analysis::refresh(&mut analysis, &buffer);
        let cursor_byte = cursor
            .and_then(|c| byte_of(&buffer, c))
            .unwrap_or(buffer.len());
        if let Some(c) = complete(analysis, cursor_byte, &view) {
            let previous = ws.query().and_then(|q| q.completion.as_ref());
            // O item escolhido sobrevive enquanto a lista é a mesma; lista nova começa do zero.
            let selected = previous
                .filter(|p| p.items == c.items)
                .map_or(0, |p| p.selected);
            let requested = previous.map(|p| p.requested.clone()).unwrap_or_default();
            needs = c
                .needs
                .iter()
                .filter(|n| !requested.contains(n))
                .cloned()
                .collect();
            if !c.items.is_empty() || !c.needs.is_empty() {
                popup = Some(CompletionState {
                    replace: c.replace,
                    items: c.items,
                    selected,
                    requested: requested.into_iter().chain(needs.iter().cloned()).collect(),
                });
            }
        }
    }
    let mut clicked: Option<usize> = None;
    if let (Some(state), Some(rect)) = (&popup, cursor_rect) {
        clicked = show_popup(ui.ctx(), editor_id, rect, state, &tokens, up || down);
    }
    if let Some(q) = app.workspaces.get_mut(key).and_then(|ws| ws.query_mut()) {
        q.analysis = analysis;
        if changed {
            q.buffer = buffer;
        }
        if let Some(cursor) = cursor {
            q.cursor = cursor;
        }
        if let Some(limit) = new_limit {
            q.limit = limit;
        }
        q.completion = popup;
        if let Some(index) = clicked {
            q.accept_completion(index);
        }
    }
    for need in needs {
        match need {
            Need::Columns { schema, table } => {
                app.deps
                    .sessions
                    .load_columns(&key.0, &key.1, &schema, &table);
            }
            Need::Schema(schema) => app.deps.sessions.load_schema(&key.0, &key.1, &schema),
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

/// O popup de sugestões embaixo do cursor: até dez linhas visíveis com rolagem, o item escolhido
/// realçado (e trazido à vista quando o teclado o moveu); clique aceita. Devolve o índice clicado.
fn show_popup(
    ctx: &egui::Context,
    editor_id: egui::Id,
    cursor_rect: egui::Rect,
    state: &CompletionState,
    tokens: &theme::Tokens,
    scroll_to_selected: bool,
) -> Option<usize> {
    let mut clicked = None;
    let row = 20.0;
    egui::Area::new(editor_id.with("completion"))
        .order(egui::Order::Foreground)
        .fixed_pos(cursor_rect.left_bottom() + egui::vec2(0.0, 2.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(260.0);
                if state.items.is_empty() {
                    ui.label(
                        egui::RichText::new("carregando colunas…")
                            .small()
                            .color(tokens.text_muted),
                    );
                    return;
                }
                egui::ScrollArea::vertical()
                    .max_height(row * 10.0)
                    .show(ui, |ui| {
                        for (i, item) in state.items.iter().enumerate() {
                            let selected = i == state.selected;
                            let response = ui
                                .horizontal(|ui| {
                                    ui.set_min_height(row);
                                    let label = ui.selectable_label(
                                        selected,
                                        egui::RichText::new(&item.label).monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(kind_tag(item.kind))
                                            .small()
                                            .color(tokens.text_muted),
                                    );
                                    if !item.detail.is_empty() {
                                        ui.label(
                                            egui::RichText::new(&item.detail)
                                                .small()
                                                .color(tokens.text_muted),
                                        );
                                    }
                                    label
                                })
                                .inner;
                            if response.clicked() {
                                clicked = Some(i);
                            }
                            if selected && scroll_to_selected {
                                response.scroll_to_me(None);
                            }
                        }
                    });
            });
        });
    clicked
}

fn kind_tag(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Column => "coluna",
        ItemKind::Alias => "alias",
        ItemKind::Keyword => "keyword",
        ItemKind::Table => "tabela",
        ItemKind::View => "view",
        ItemKind::Schema => "schema",
        ItemKind::Function => "função",
    }
}

/// O byte do caractere `chars` (o egui conta o cursor em caracteres).
fn byte_of(buffer: &str, chars: usize) -> Option<usize> {
    buffer
        .char_indices()
        .nth(chars)
        .map(|(b, _)| b)
        .or((chars == buffer.chars().count()).then_some(buffer.len()))
}

/// O índice em caracteres do byte `at` (o inverso de [`byte_of`]).
fn cursor_chars_of(buffer: &str, at: usize) -> usize {
    buffer.get(..at).map_or(0, |s| s.chars().count())
}

struct Colors {
    keyword: egui::Color32,
    string: egui::Color32,
    number: egui::Color32,
    comment: egui::Color32,
    text: egui::Color32,
    current: egui::Color32,
}

/// O `LayoutJob` do buffer: cada span da análise com a sua cor, o resto na cor do texto, e o
/// statement sob o cursor com fundo leve. Sem quebra de linha automática (ScrollArea).
fn layout(
    analysis: &Analysis,
    font: &egui::FontId,
    colors: &Colors,
    current: Option<std::ops::Range<usize>>,
) -> egui::text::LayoutJob {
    let text = analysis.text();
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
    for span in analysis.spans() {
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
        let job = layout(
            &Analysis::new(text),
            &egui::FontId::monospace(13.0),
            &colors,
            Some(0..10),
        );
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
