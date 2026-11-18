//! A aba de SQL (FEAT-010; telas 13 a 15 do protótipo da FEAT-015): a barra com Executar ⌘↩ (a
//! ação principal), Executar tudo ⌘⇧↩, Cancelar e Formatar ⌘I, a etiqueta âmbar da transação aberta
//! e o limite; os avisos da aba em faixa logo abaixo; o editor com o realce do tema (layouter sobre
//! os spans da `sql::Analysis` da aba, refeita só quando o texto muda), os números de linha e o
//! statement sob o cursor em destaque; o rodapé com `Linha, coluna`; os resultados embaixo
//! (`results.rs`). O buffer é do `QueryTab`; o cursor é lido do estado do `TextEdit` a cada frame.

use hy_dbplus_engine::{Driver, TableKind};
use hy_dbplus_sql::{Analysis, CatalogView, ItemKind, Kind, Need, complete};

use crate::app::{App, CompletionState, DbKey, QueryNotice};
use crate::fonts::icon;
use crate::i18n::{self, Tr};
use crate::widgets::{self, BannerKind, ButtonKind};
use crate::{results, theme};

pub const LIMITS: [Option<usize>; 4] = [None, Some(100), Some(500), Some(1000)];

/// Executar (⌘↩): o statement sob o cursor.
pub fn run_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Enter)
}

/// Executar tudo (⌘⇧↩).
pub fn run_all_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        egui::Key::Enter,
    )
}

/// Formatar (⌘I).
pub fn format_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::I)
}

/// A largura da coluna dos números de linha.
const GUTTER: f32 = 40.0;
/// A largura do popup de sugestões.
const POPUP_WIDTH: f32 = 330.0;

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
    notice: Option<QueryNotice>,
    pending_cursor: Option<usize>,
}

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, key: &DbKey)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let tr = app.tr();
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

    let mut dismiss_notice = false;

    // Atalhos, antes dos widgets (consumidos aqui, o TextEdit não os vê). Com a paleta ou a
    // confirmação de fechar na tela, são delas.
    let modal = app.modal_open();
    ui.input_mut(|i| {
        if modal {
            return;
        }
        if i.consume_shortcut(&run_all_shortcut()) {
            run_all = true;
        } else if i.consume_shortcut(&run_shortcut()) {
            run_current = true;
        }
        if i.consume_shortcut(&format_shortcut()) && !q.running {
            beautify = true;
        }
    });

    toolbar(
        ui,
        tr,
        key,
        &q,
        open_transaction,
        &mut ToolbarActions {
            run_current: &mut run_current,
            run_all: &mut run_all,
            beautify: &mut beautify,
            cancel: &mut cancel,
            new_limit: &mut new_limit,
        },
    );
    // O aviso da aba (cancelada, statement incompleto, falha ao salvar…), na faixa sob a barra.
    if let Some(notice) = &q.notice {
        egui::Panel::top(egui::Id::new(("query-notice", key, q.id)))
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(tokens.bg)
                    .inner_margin(egui::Margin::symmetric(
                        theme::space::M as i8,
                        theme::space::S as i8,
                    )),
            )
            .show(ui, |ui| {
                let kind = if notice.is_error() {
                    BannerKind::Error
                } else {
                    BannerKind::Warning
                };
                dismiss_notice = widgets::banner(ui, kind, &notice.text(tr), Some(tr.close()));
            });
    }

    results::show(app, ui, key);

    // O editor: os números de linha e o TextEdit com o layouter de realce.
    let mut buffer = q.buffer.clone();
    let cursor_byte = q.cursor_byte;
    let current = Analysis::refresh(&mut analysis, &buffer)
        .statement_at(cursor_byte)
        .map(|s| s.range.clone());
    let font = egui::FontId::monospace(theme::text_size::BODY);
    let colors = Colors::of(&tokens);
    let editor_id = editor_id(key, q.id);
    let mut changed = false;
    let mut cursor_rect: Option<egui::Rect> = None;
    egui::Panel::bottom(egui::Id::new(("query-status", key, q.id)))
        .exact_size(theme::size::FOOTER)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(tokens.panel)
                .inner_margin(egui::Margin::symmetric(theme::space::M as i8, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                let (line, column) = line_column(&buffer, cursor_byte);
                ui.label(
                    egui::RichText::new(tr.line_column(line, column))
                        .size(theme::text_size::SMALL)
                        .color(tokens.text_muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(tr.saved_on_leave())
                            .size(theme::text_size::SMALL)
                            .color(tokens.text_faint),
                    );
                });
            });
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(tokens.field))
        .show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt(("sql-scroll", key, q.id))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        let row_height = ui.fonts_mut(|f| f.row_height(&font));
                        // O fundo da coluna entra antes dos números (o tamanho só se sabe depois).
                        let gutter_bg = ui.painter().add(egui::Shape::Noop);
                        let gutter = ui
                            .vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.add_space(theme::space::XS);
                                let lines = buffer.split('\n').count().max(1);
                                for n in 1..=lines {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(GUTTER, row_height),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().text(
                                        rect.right_center() - egui::vec2(theme::space::S, 0.0),
                                        egui::Align2::RIGHT_CENTER,
                                        n.to_string(),
                                        egui::FontId::monospace(theme::text_size::SMALL),
                                        tokens.text_faint,
                                    );
                                }
                            })
                            .response
                            .rect;
                        let column = egui::Rect::from_min_max(
                            gutter.min,
                            egui::pos2(gutter.max.x, gutter.max.y.max(ui.clip_rect().max.y)),
                        );
                        ui.painter().set(
                            gutter_bg,
                            egui::Shape::rect_filled(column, 0.0, tokens.panel),
                        );
                        ui.painter().vline(
                            column.max.x,
                            column.y_range(),
                            egui::Stroke::new(1.0, tokens.border),
                        );
                        let mut layouter =
                            |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
                                // O texto que o TextEdit passa já tem a tecla deste frame: a
                                // análise é refeita só nesse caso, e fica guardada para os frames
                                // seguintes.
                                let analysis = Analysis::refresh(&mut analysis, text.as_str());
                                let job = layout(analysis, &font, &colors, current.clone());
                                ui.fonts_mut(|f| f.layout_job(job))
                            };
                        // SQL gerado (FEAT-013): o cursor vai para o statement novo e o editor
                        // pega o foco; o estado do widget é a fonte do cursor, então é ele que muda.
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
                            .frame(egui::Frame::NONE)
                            .margin(egui::Margin::symmetric(
                                theme::space::M as i8,
                                theme::space::XS as i8,
                            ))
                            .lock_focus(true)
                            .desired_width(f32::INFINITY)
                            .desired_rows(12)
                            .layouter(&mut layouter)
                            .show(ui);
                        changed = output.response.changed();
                        // O statement sob o cursor ganha o traço de destaque na borda do editor.
                        // Com o texto mudado neste frame, a faixa é do texto antigo: o traço volta
                        // no frame seguinte, já com a análise nova.
                        if !changed && let Some(range) = &current {
                            let at = |byte: usize| {
                                output
                                    .galley
                                    .pos_from_cursor(egui::text::CCursor::new(cursor_chars_of(
                                        &buffer, byte,
                                    )))
                                    .translate(output.galley_pos.to_vec2())
                            };
                            let (top, bottom) = (at(range.start), at(range.end));
                            ui.painter().vline(
                                output.response.rect.left() + 1.0,
                                top.top()..=bottom.bottom(),
                                egui::Stroke::new(2.0, tokens.accent),
                            );
                        }
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
        clicked = show_popup(ui.ctx(), tr, editor_id, rect, state, up || down);
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
        if dismiss_notice {
            q.notice = None;
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

/// O que a barra pediu neste frame.
struct ToolbarActions<'a> {
    run_current: &'a mut bool,
    run_all: &'a mut bool,
    beautify: &'a mut bool,
    cancel: &'a mut bool,
    new_limit: &'a mut Option<Option<usize>>,
}

/// A barra do editor (tela 13): Executar (principal), Executar tudo, Cancelar (só durante a
/// execução) e Formatar, cada um com o atalho; o tempo da execução em curso; à direita a etiqueta
/// âmbar da transação aberta (aviso, não erro: AC-012) e o limite de linhas.
fn toolbar(
    ui: &mut egui::Ui,
    tr: Tr,
    key: &DbKey,
    q: &Header,
    open_transaction: bool,
    actions: &mut ToolbarActions<'_>,
) {
    let tokens = theme::current(ui.ctx());
    egui::Panel::top(egui::Id::new(("query-toolbar", key, q.id)))
        .exact_size(theme::size::BAR)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(tokens.bg)
                .inner_margin(egui::Margin::symmetric(theme::space::M as i8, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                let idle = !q.running;
                let has_text = !q.buffer.trim().is_empty();
                let blocked = if q.running {
                    tr.wait_for_run()
                } else {
                    tr.notice_nothing_to_run()
                };
                let run = i18n::shortcut_label(ui.ctx(), &run_shortcut());
                if widgets::button_shortcut(
                    ui,
                    idle && has_text,
                    ButtonKind::Primary,
                    Some(icon::RUN),
                    tr.run(),
                    &run,
                    blocked,
                )
                .on_hover_text(tr.run_hint())
                .clicked()
                {
                    *actions.run_current = true;
                }
                let run_all = i18n::shortcut_label(ui.ctx(), &run_all_shortcut());
                if widgets::button_shortcut(
                    ui,
                    idle && has_text,
                    ButtonKind::Normal,
                    Some(icon::RUN_ALL),
                    tr.run_all(),
                    &run_all,
                    blocked,
                )
                .on_hover_text(tr.run_all_hint())
                .clicked()
                {
                    *actions.run_all = true;
                }
                if widgets::button_enabled(
                    ui,
                    q.running,
                    ButtonKind::Normal,
                    Some(icon::STOP),
                    tr.cancel(),
                    tr.nothing_running(),
                )
                .clicked()
                {
                    *actions.cancel = true;
                }
                let format = i18n::shortcut_label(ui.ctx(), &format_shortcut());
                if widgets::button_shortcut(
                    ui,
                    idle,
                    ButtonKind::Ghost,
                    Some(icon::FORMAT),
                    tr.format_sql(),
                    &format,
                    tr.wait_for_run(),
                )
                .clicked()
                {
                    *actions.beautify = true;
                }
                if q.running {
                    let elapsed = q.started.map_or(0.0, |s| s.elapsed().as_secs_f32());
                    widgets::loading(ui, &tr.running_for(elapsed));
                    ui.ctx().request_repaint();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt(("query-limit", key, q.id))
                        .selected_text(tr.limit_rows(q.limit))
                        .show_ui(ui, |ui| {
                            for limit in LIMITS {
                                if ui
                                    .selectable_label(q.limit == limit, tr.limit_rows(limit))
                                    .clicked()
                                    && limit != q.limit
                                {
                                    *actions.new_limit = Some(limit);
                                }
                            }
                        });
                    ui.label(egui::RichText::new(tr.limit()).color(tokens.text_muted));
                    if open_transaction {
                        widgets::pending_chip(
                            ui,
                            &format!("{} {}", icon::WARNING, tr.open_transaction()),
                        )
                        .on_hover_text(tr.open_transaction_hint());
                    }
                });
            });
        });
}

/// O popup de sugestões embaixo do cursor (tela 14): até dez linhas visíveis com rolagem, cada
/// uma com o ícone do tipo, o nome em fonte fixa e o tipo e o detalhe à direita; o item escolhido
/// em destaque suave (e trazido à vista quando o teclado o moveu); o rodapé com as teclas. Clique
/// aceita. Devolve o índice clicado.
fn show_popup(
    ctx: &egui::Context,
    tr: Tr,
    editor_id: egui::Id,
    cursor_rect: egui::Rect,
    state: &CompletionState,
    scroll_to_selected: bool,
) -> Option<usize> {
    let tokens = theme::current(ctx);
    let mut clicked = None;
    let row = theme::size::ROW;
    let mono = egui::FontId::monospace(theme::text_size::SMALL);
    let small = |text: String| {
        egui::RichText::new(text)
            .size(theme::text_size::SMALL)
            .color(tokens.text_faint)
    };
    egui::Area::new(editor_id.with("completion"))
        .order(egui::Order::Foreground)
        .fixed_pos(cursor_rect.left_bottom() + egui::vec2(0.0, theme::space::XS))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(tokens.card)
                .stroke(egui::Stroke::new(1.0, tokens.border))
                .corner_radius(theme::radius::CARD)
                .inner_margin(theme::space::XS)
                .show(ui, |ui| {
                    ui.set_width(POPUP_WIDTH);
                    if state.items.is_empty() {
                        widgets::loading(ui, tr.completion_loading());
                        return;
                    }
                    egui::ScrollArea::vertical()
                        .max_height(row * 10.0)
                        .show(ui, |ui| {
                            for (i, item) in state.items.iter().enumerate() {
                                let selected = i == state.selected;
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), row),
                                    egui::Sense::click(),
                                );
                                let fill = if selected {
                                    tokens.accent_soft
                                } else if response.hovered() {
                                    tokens.hover
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                ui.painter().rect_filled(rect, theme::radius::CONTROL, fill);
                                // Pintado direto (sem rótulos filhos): o nome acessível da
                                // linha é só o do item, e o detalhe longo não empurra o nome.
                                let detail = item_detail(tr, item);
                                let inner = rect.shrink2(egui::vec2(theme::space::S, 0.0));
                                let painter = ui.painter_at(inner);
                                let detail = painter.layout_no_wrap(
                                    detail,
                                    egui::FontId::proportional(theme::text_size::SMALL),
                                    tokens.text_faint,
                                );
                                let detail_left = inner.right() - detail.size().x;
                                painter.galley(
                                    egui::pos2(
                                        detail_left,
                                        inner.center().y - detail.size().y / 2.0,
                                    ),
                                    detail,
                                    tokens.text_faint,
                                );
                                let icon = painter.text(
                                    inner.left_center(),
                                    egui::Align2::LEFT_CENTER,
                                    kind_icon(item),
                                    egui::FontId::proportional(theme::text_size::BODY),
                                    tokens.text_muted,
                                );
                                let name_rect = egui::Rect::from_min_max(
                                    egui::pos2(icon.right() + theme::space::S, inner.top()),
                                    egui::pos2(detail_left - theme::space::S, inner.bottom()),
                                );
                                ui.painter_at(name_rect).text(
                                    name_rect.left_center(),
                                    egui::Align2::LEFT_CENTER,
                                    &item.label,
                                    mono.clone(),
                                    tokens.text,
                                );
                                let response =
                                    response.on_hover_cursor(egui::CursorIcon::PointingHand);
                                response.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::SelectableLabel,
                                        true,
                                        selected,
                                        &item.label,
                                    )
                                });
                                if response.clicked() {
                                    clicked = Some(i);
                                }
                                if selected && scroll_to_selected {
                                    response.scroll_to_me(None);
                                }
                            }
                        });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = theme::space::M;
                        ui.label(small(format!("↑↓ {}", tr.hint_navigate())));
                        ui.label(small(format!("↩ / tab {}", tr.hint_accept())));
                        ui.label(small(format!("esc {}", tr.hint_close())));
                    });
                });
        });
    clicked
}

/// O detalhe à direita da sugestão: os dados do banco (tipo, schema, assinatura) e a palavra do
/// tipo no idioma, sem repetir: "integer · chave · coluna", "tabela · public", "schema".
fn item_detail(tr: Tr, item: &hy_dbplus_sql::Item) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let table_word = item.table_kind.map(|kind| match kind {
        TableKind::Table => tr.kind_table(),
        TableKind::View => tr.kind_view(),
        TableKind::MaterializedView => tr.kind_materialized_view(),
        TableKind::Foreign => tr.kind_foreign_table(),
    });
    if let Some(word) = table_word {
        parts.push(word);
    }
    if !item.detail.is_empty() {
        parts.push(&item.detail);
    }
    if item.primary_key {
        parts.push(tr.kind_key());
    }
    if table_word.is_none() {
        parts.push(kind_tag(tr, item.kind));
    }
    parts.join(" · ")
}

fn kind_tag(tr: Tr, kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Column => tr.kind_column(),
        ItemKind::Alias => tr.kind_alias(),
        ItemKind::Keyword => tr.kind_keyword(),
        ItemKind::Table => tr.kind_table(),
        ItemKind::View => tr.kind_view(),
        ItemKind::Schema => tr.kind_schema(),
        ItemKind::Function => tr.kind_function(),
    }
}

/// O ícone da sugestão: o mesmo da árvore para tabela, view e tabela estrangeira.
fn kind_icon(item: &hy_dbplus_sql::Item) -> &'static str {
    match (item.kind, item.table_kind) {
        (_, Some(kind)) => icon::table_kind(kind),
        (ItemKind::Column, _) => icon::COLUMN,
        (ItemKind::Alias, _) => icon::ALIAS,
        (ItemKind::Keyword, _) => icon::KEYWORD,
        (ItemKind::Table, _) => icon::TABLE,
        (ItemKind::View, _) => icon::VIEW,
        (ItemKind::Schema, _) => icon::SCHEMA,
        (ItemKind::Function, _) => icon::FUNCTION,
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

/// SQL só leitura com as cores do tema (gravação, DDL): o mesmo realce do editor, sem cursor.
pub(crate) fn highlighted(
    text: &str,
    font: &egui::FontId,
    tokens: &theme::Tokens,
) -> egui::text::LayoutJob {
    let colors = Colors::of(tokens);
    let mut job = layout(&Analysis::new(text), font, &colors, None);
    job.wrap.max_width = f32::INFINITY;
    job
}

struct Colors {
    keyword: egui::Color32,
    string: egui::Color32,
    number: egui::Color32,
    comment: egui::Color32,
    text: egui::Color32,
    current: egui::Color32,
}

impl Colors {
    fn of(tokens: &theme::Tokens) -> Self {
        Self {
            keyword: tokens.syntax_keyword,
            string: tokens.syntax_string,
            number: tokens.syntax_number,
            comment: tokens.syntax_comment,
            text: tokens.text,
            current: tokens.hover,
        }
    }
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
    fn detalhe_da_sugestao_traduzido_e_sem_repetir_o_tipo() {
        use crate::i18n::Language;
        let item = |kind, detail: &str, primary_key, table_kind| hy_dbplus_sql::Item {
            label: "x".into(),
            kind,
            detail: detail.into(),
            primary_key,
            table_kind,
        };
        let (pt, en) = (Tr(Language::PtBr), Tr(Language::En));
        let table = item(ItemKind::Table, "public", false, Some(TableKind::Table));
        assert_eq!(item_detail(pt, &table), "tabela · public");
        assert_eq!(item_detail(en, &table), "table · public");
        let matview = item(ItemKind::View, "", false, Some(TableKind::MaterializedView));
        assert_eq!(item_detail(en, &matview), "materialized view");
        let key = item(ItemKind::Column, "integer", true, None);
        assert_eq!(item_detail(pt, &key), "integer · chave · coluna");
        assert_eq!(item_detail(en, &key), "integer · key · column");
        let schema = item(ItemKind::Schema, "", false, None);
        assert_eq!(item_detail(pt, &schema), "schema");
        let keyword = item(ItemKind::Keyword, "", false, None);
        assert_eq!(item_detail(en, &keyword), "keyword");
    }

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
