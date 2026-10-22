//! A aba Data (tela 3.4): grid virtualizado da página atual, rodapé com abas, contador, paginação e,
//! desde a FEAT-007, a edição: duplo clique na célula abre o campo (Enter confirma, Esc cancela),
//! célula pendente ganha fundo de destaque, ⌫ marca as linhas selecionadas para apagar (riscadas), o
//! chip "N alterações" leva a Commit (⌘S) e Descartar (⌘Z). Com pendência, ordenar, paginar e recarregar
//! ficam travados. Só tabela com chave primária edita; sem ela, aviso na linha de tamanhos.

use egui_extras::{Column as TableColumn, TableBuilder};
use hy_dbplus_engine::{Direction, Driver, Order, Page};

use crate::app::{App, DbKey, TabView};
use crate::theme;

const ROW_HEIGHT: f32 = 22.0;
pub const PAGE_SIZES: [u32; 3] = [100, 500, 1000];
/// Dica dos controles travados por edição pendente.
pub const LOCK_HINT: &str = "Commit ou descarte as alterações antes";
pub const READ_ONLY_HINT: &str = "sem chave primária: só leitura";

/// O centro da janela quando há uma tabela aberta.
pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, key: &DbKey)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let Some(tab) = app.workspaces.get(key).and_then(|ws| ws.data().cloned()) else {
        return;
    };
    let editable = app.editable(key);
    // Só leitura de verdade: as colunas já chegaram e nenhuma é chave (antes disso é só "ainda não").
    let columns_known = app.workspaces.get(key).is_some_and(|ws| {
        ws.columns
            .contains_key(&(tab.schema.clone(), tab.table.clone()))
    });
    let read_only = tab.rows.is_some() && columns_known && app.key_columns(key).is_empty();
    let locked = tab.has_changes();
    let mut next_page: Option<Page> = None;
    let mut select: Option<(usize, egui::Modifiers)> = None;
    let mut start_edit: Option<(usize, usize)> = None;
    let mut edit_text: Option<String> = None;
    let mut finish_edit: Option<bool> = None;
    let mut retry = false;
    let mut toggle_delete = false;
    let mut open_commit = false;
    let mut discard = false;
    let mut view_change: Option<TabView> = None;
    let mut open_ddl = false;

    // Atalhos, antes dos widgets: ⌘S abre o Commit; ⌘Z descarta (sem campo em foco: dentro de um
    // campo, ⌘Z é o desfazer do próprio campo); ⌫ sem campo em foco marca a seleção.
    let nothing_focused = ui.ctx().memory(|m| m.focused().is_none());
    let shortcut = |key: egui::Key| egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key);
    if locked
        && tab.commit.is_none()
        && ui.input_mut(|i| i.consume_shortcut(&shortcut(egui::Key::S)))
    {
        open_commit = true;
    }
    if locked
        && tab.commit.is_none()
        && nothing_focused
        && ui.input_mut(|i| i.consume_shortcut(&shortcut(egui::Key::Z)))
    {
        discard = true;
    }
    if editable
        && nothing_focused
        && tab.editing.is_none()
        && tab.commit.is_none()
        && !tab.selected_rows.is_empty()
        && ui.input(|i| i.key_pressed(egui::Key::Backspace))
    {
        toggle_delete = true;
    }

    egui::Panel::bottom("data-footer").show(ui, |ui| {
        ui.horizontal(|ui| {
            // Sub-abas (FEAT-009): Data, Structure e Index, por aba de tabela.
            for (view, label) in [
                (TabView::Data, "Data"),
                (TabView::Structure, "Structure"),
                (TabView::Index, "Index"),
            ] {
                if ui.selectable_label(tab.view == view, label).clicked() && tab.view != view {
                    view_change = Some(view);
                }
            }
            ui.separator();
            if tab.view != TabView::Data {
                if tab.view == TabView::Structure {
                    ui.add_enabled(false, egui::Button::new("+ Column"))
                        .on_disabled_hover_text("não disponível nesta versão");
                    ui.add_enabled(false, egui::Button::new("Triggers"))
                        .on_disabled_hover_text("em breve");
                    if ui
                        .add_enabled(tab.structure.is_some(), egui::Button::new("DDL"))
                        .on_hover_text("O CREATE TABLE montado a partir do catálogo")
                        .clicked()
                    {
                        open_ddl = true;
                    }
                }
                return;
            }
            ui.add_enabled(false, egui::Button::new("+ Row"))
                .on_disabled_hover_text("não disponível nesta versão");
            let can_delete = editable && !tab.selected_rows.is_empty() && tab.commit.is_none();
            if ui
                .add_enabled(can_delete, egui::Button::new("Delete"))
                .on_hover_text("Marca as linhas selecionadas para apagar (⌫); de novo desmarca")
                .on_disabled_hover_text(if read_only {
                    READ_ONLY_HINT
                } else {
                    "selecione linhas"
                })
                .clicked()
            {
                toggle_delete = true;
            }
            ui.add_enabled(false, egui::Button::new("Filters"))
                .on_disabled_hover_text("em breve");
            ui.add_enabled(false, egui::Button::new("Columns"))
                .on_disabled_hover_text("em breve");
            if locked {
                ui.separator();
                let (updates, deletes) = tab.change_counts();
                let total = updates + deletes;
                let chip = if total == 1 {
                    "1 alteração".to_owned()
                } else {
                    format!("{total} alterações")
                };
                ui.label(egui::RichText::new(chip).strong().color(tokens.accent));
                let idle = tab.commit.is_none();
                if ui
                    .add_enabled(idle, egui::Button::new("Commit"))
                    .on_hover_text("Mostra o SQL e executa numa transação (⌘S)")
                    .clicked()
                {
                    open_commit = true;
                }
                if ui
                    .add_enabled(idle, egui::Button::new("Descartar"))
                    .on_hover_text("Esquece as alterações; nada vai ao banco (⌘Z)")
                    .clicked()
                {
                    discard = true;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let shown = tab.rows.as_ref().map(|r| r.rows.len() as u64).unwrap_or(0);
                let (limit, offset) = (u64::from(tab.page.limit), tab.page.offset);
                let total = tab.stats.as_ref().map(|s| s.row_count);
                // Sem total ainda, página cheia sugere que há mais.
                let can_next = total.is_some_and(|t| offset + limit < t)
                    || (total.is_none() && shown == limit);
                let can_prev = offset > 0;
                let free = !tab.loading && !locked;
                let lock_hint = if locked { LOCK_HINT } else { "" };
                if ui
                    .add_enabled(can_next && free, egui::Button::new("›"))
                    .on_disabled_hover_text(lock_hint)
                    .clicked()
                {
                    next_page = Some(Page {
                        offset: offset + limit,
                        ..tab.page.clone()
                    });
                }
                if ui
                    .add_enabled(can_prev && free, egui::Button::new("‹"))
                    .on_disabled_hover_text(lock_hint)
                    .clicked()
                {
                    next_page = Some(Page {
                        offset: offset.saturating_sub(limit),
                        ..tab.page.clone()
                    });
                }
                let total_text = total.map_or_else(|| "…".to_owned(), |t| t.to_string());
                let selected = tab.selected_rows.len();
                let counter = if selected > 0 {
                    format!("{selected} of {shown} rows selected")
                } else if shown == 0 {
                    format!("0 of {total_text} rows")
                } else {
                    format!("{}-{} of {total_text} rows", offset + 1, offset + shown)
                };
                ui.label(egui::RichText::new(counter).monospace());
                ui.add_enabled_ui(free, |ui| {
                    egui::ComboBox::from_id_salt("page-size")
                        .selected_text(tab.page.limit.to_string())
                        .show_ui(ui, |ui| {
                            for size in PAGE_SIZES {
                                if ui
                                    .selectable_label(tab.page.limit == size, size.to_string())
                                    .clicked()
                                    && size != tab.page.limit
                                {
                                    next_page = Some(Page {
                                        limit: size,
                                        offset: 0,
                                        ..tab.page.clone()
                                    });
                                }
                            }
                        });
                });
                ui.label(
                    egui::RichText::new("por página")
                        .small()
                        .color(tokens.text_muted),
                );
            });
        });
    });

    match tab.view {
        TabView::Structure => crate::structure::show(app, ui, key),
        TabView::Index => crate::index::show(app, ui, key),
        TabView::Data => {}
    }
    if tab.view != TabView::Data {
        if let Some(view) = view_change {
            app.set_view(key, view);
        }
        if open_ddl && let Some(tab) = app.workspaces.get_mut(key).and_then(|ws| ws.data_mut()) {
            tab.ddl_open = true;
        }
        return;
    }
    egui::CentralPanel::default_margins().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{}.{}", tab.schema, tab.table)).strong());
            if read_only {
                ui.label(
                    egui::RichText::new(READ_ONLY_HINT)
                        .small()
                        .color(tokens.text_muted),
                );
            }
            if let Some(stats) = &tab.stats {
                ui.label(
                    egui::RichText::new(format!(
                        "{} rows · total {} · data {} · index {}",
                        stats.row_count, stats.total_size, stats.data_size, stats.index_size
                    ))
                    .small()
                    .color(tokens.text_muted),
                );
            }
            if locked {
                ui.label(
                    egui::RichText::new(LOCK_HINT)
                        .small()
                        .color(tokens.text_muted),
                )
                .on_hover_text(
                    "Ordenar, paginar, recarregar e trocar de tabela esperam o Commit ou o Descartar",
                );
            }
            if tab.loading {
                ui.label(
                    egui::RichText::new("carregando…")
                        .small()
                        .color(tokens.text_muted),
                );
            }
        });
        if let Some(comment) = tab.stats.as_ref().and_then(|s| s.comment.as_deref()) {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(comment)
                        .small()
                        .color(tokens.text_muted),
                )
                .wrap(),
            );
        }

        if let Some(error) = &tab.error {
            // O grid já carregado fica; o erro (página ou stats) vai numa faixa em cima dele.
            ui.horizontal(|ui| {
                ui.colored_label(theme::error_color(), error);
                if ui
                    .add_enabled(!locked, egui::Button::new("Tentar de novo").small())
                    .on_disabled_hover_text(LOCK_HINT)
                    .clicked()
                {
                    retry = true;
                }
            });
            if tab.rows.is_none() {
                return;
            }
        }
        let Some(rows) = tab.rows.as_ref() else {
            ui.label(egui::RichText::new("Carregando…").color(tokens.text_muted));
            return;
        };
        let types: Vec<Option<String>> = app
            .workspaces
            .get(key)
            .map(|ws| ws.catalog_columns(&tab.schema, &tab.table, &rows.columns))
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.map(|c| c.data_type.clone()))
            .collect();
        // Lidos antes de o TableBuilder tomar o `ui`: ⌘ alterna e ⇧ estende a seleção; Esc cancela a célula.
        let modifiers = ui.input(|i| i.modifiers);
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let mut builder = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .sense(egui::Sense::click())
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(TableColumn::exact(48.0));
        for _ in &rows.columns {
            builder = builder.column(TableColumn::initial(140.0).at_least(60.0).clip(true));
        }
        builder
            .header(ROW_HEIGHT + 4.0, |mut header| {
                header.col(|ui| {
                    ui.label(egui::RichText::new("#").small().color(tokens.text_muted));
                });
                for (i, name) in rows.columns.iter().enumerate() {
                    header.col(|ui| {
                        let current = tab
                            .page
                            .order
                            .as_ref()
                            .filter(|o| &o.column == name)
                            .map(|o| o.direction);
                        let arrow = match current {
                            Some(Direction::Asc) => " ▲",
                            Some(Direction::Desc) => " ▼",
                            None => "",
                        };
                        let label = ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!("{name}{arrow}")).strong(),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if let Some(Some(ty)) = types.get(i) {
                            ui.label(egui::RichText::new(ty).small().color(tokens.text_muted));
                        }
                        if label.clicked() && !tab.loading && !locked {
                            // asc → desc → sem ordem
                            let order = match current {
                                None => Some(Order {
                                    column: name.clone(),
                                    direction: Direction::Asc,
                                }),
                                Some(Direction::Asc) => Some(Order {
                                    column: name.clone(),
                                    direction: Direction::Desc,
                                }),
                                Some(Direction::Desc) => None,
                            };
                            next_page = Some(Page {
                                order,
                                offset: 0,
                                ..tab.page.clone()
                            });
                        }
                        if locked {
                            label.on_hover_text(LOCK_HINT);
                        }
                    });
                }
            })
            .body(|body| {
                body.rows(ROW_HEIGHT, rows.rows.len(), |mut row| {
                    let index = row.index();
                    let deleted = tab.deleted.contains(&index);
                    row.set_selected(tab.selected_rows.contains(&index));
                    row.col(|ui| {
                        // O número da linha é o alvo de seleção; o resto da linha também seleciona.
                        let number = (tab.page.offset + index as u64 + 1).to_string();
                        let mut text = egui::RichText::new(number).small();
                        if deleted {
                            text = text.strikethrough().color(theme::error_color());
                        }
                        if ui
                            .selectable_label(tab.selected_rows.contains(&index), text)
                            .clicked()
                        {
                            select = Some((index, modifiers));
                        }
                    });
                    for col in 0..rows.columns.len() {
                        let editing_here = tab
                            .editing
                            .as_ref()
                            .is_some_and(|e| e.row == index && e.col == col);
                        let pending = rows
                            .columns
                            .get(col)
                            .is_some_and(|name| tab.is_pending(index, name));
                        let value = tab.cell_value(index, col).flatten().map(str::to_owned);
                        let (_, response) = row.col(|ui| {
                            if editing_here {
                                let mut text = tab
                                    .editing
                                    .as_ref()
                                    .map(|e| e.text.clone())
                                    .unwrap_or_default();
                                let field = ui.add(
                                    egui::TextEdit::singleline(&mut text)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY),
                                );
                                if tab.editing.as_ref().is_some_and(|e| e.focus_pending) {
                                    field.request_focus();
                                }
                                edit_text = Some(text);
                                // Enter e Esc soltam o foco sozinhos; clicar fora também confirma.
                                if field.lost_focus() {
                                    finish_edit = Some(!escape);
                                }
                                return;
                            }
                            if pending {
                                ui.painter()
                                    .rect_filled(ui.max_rect(), 0.0, tokens.accent_soft);
                            }
                            let mut text = match &value {
                                Some(value) => egui::RichText::new(value).monospace(),
                                None => egui::RichText::new("NULL")
                                    .italics()
                                    .color(tokens.text_muted),
                            };
                            if deleted {
                                text = text.strikethrough().color(theme::error_color());
                            }
                            // Sem seleção de texto na célula: o Label selecionável consome o clique e ele
                            // não chega à linha (só o número selecionava). Copiar é pelo painel.
                            ui.add(egui::Label::new(text).selectable(false));
                        });
                        if response.double_clicked() && editable && !deleted {
                            start_edit = Some((index, col));
                        }
                    }
                    if row.response().clicked() {
                        select = Some((index, modifiers));
                    }
                });
            });
    });

    if let Some(view) = view_change {
        app.set_view(key, view);
    }
    if let Some(text) = edit_text
        && let Some(tab) = app.workspaces.get_mut(key).and_then(|ws| ws.data_mut())
        && let Some(edit) = tab.editing.as_mut()
    {
        edit.text = text;
        edit.focus_pending = false;
    }
    if let Some(confirm) = finish_edit {
        app.finish_edit(key, confirm);
    }
    if let Some((row, col)) = start_edit {
        app.start_edit(key, row, col);
    } else if let Some((index, modifiers)) = select {
        app.select_row(key, index, modifiers);
    }
    if toggle_delete {
        app.toggle_delete(key);
    }
    if open_commit {
        app.open_commit(key);
    } else if discard {
        app.discard_changes(key);
    }
    if retry {
        app.retry_data(key);
    } else if let Some(page) = next_page {
        app.load_data_page(key, page);
    }
}
