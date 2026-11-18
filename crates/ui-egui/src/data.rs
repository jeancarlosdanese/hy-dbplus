//! A aba de tabela (telas 7 a 12 do protótipo da FEAT-015). Barra no alto com Dados | Estrutura |
//! Índices, `schema.tabela`, linhas e tamanho, e o recarregar (⌘R); na sub-aba Dados, o grid
//! virtualizado da página e o rodapé com a dica de edição ou as pendências e a paginação. Edição
//! (FEAT-007): duplo clique abre o campo (Enter confirma, Esc cancela), a célula pendente fica âmbar
//! mesmo na linha selecionada, ⌫ marca as linhas selecionadas para apagar (riscadas), "Revisar e
//! gravar" (⌘S) abre a gravação e "Descartar" (⌘Z) esquece. Com pendência, ordenar, paginar e
//! recarregar ficam travados. Só tabela com chave primária edita.

use egui_extras::{Column as TableColumn, TableBuilder};
use hy_dbplus_engine::{Direction, Driver, Order, Page};

use crate::app::{App, DbKey, TabView};
use crate::fonts::icon;
use crate::grid::ROW_HEIGHT;
use crate::theme::{self, Tokens};
use crate::{i18n, widgets};

pub const PAGE_SIZES: [u32; 3] = [100, 500, 1000];

/// ⌘R: recarrega a página atual.
pub fn reload_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::R)
}

/// Fundo e cor do texto de uma célula. Pendência e exclusão pintam a célula por cima da cor de
/// seleção da linha: a alteração não some quando a linha é selecionada (AC-006).
pub(crate) fn cell_colors(
    t: &Tokens,
    pending: bool,
    deleted: bool,
) -> (Option<egui::Color32>, egui::Color32) {
    if deleted {
        (Some(t.danger_soft), t.danger)
    } else if pending {
        (Some(t.pending), t.pending_ink)
    } else {
        (None, t.text)
    }
}

/// Tipo do PostgreSQL que se alinha à direita (números).
pub(crate) fn is_numeric_type(data_type: &str) -> bool {
    let ty = data_type.trim().to_ascii_lowercase();
    if ty.ends_with("[]") {
        return false;
    }
    let base = ty.split('(').next().unwrap_or_default().trim();
    matches!(
        base,
        "smallint"
            | "integer"
            | "bigint"
            | "int"
            | "int2"
            | "int4"
            | "int8"
            | "numeric"
            | "decimal"
            | "real"
            | "double precision"
            | "float4"
            | "float8"
            | "money"
            | "oid"
            | "smallserial"
            | "serial"
            | "bigserial"
    )
}

/// O centro da janela quando há uma tabela aberta.
pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, key: &DbKey)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let tr = app.tr();
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

    // Atalhos, antes dos widgets e só na sub-aba Data (fora dela não são consumidos nem agem): ⌘S
    // abre a gravação; ⌘Z descarta (sem campo em foco: dentro de um campo, ⌘Z é o desfazer do
    // próprio campo); ⌘R recarrega a página; ⌫ sem campo em foco marca a seleção.
    let in_data = tab.view == TabView::Data;
    let nothing_focused = ui.ctx().memory(|m| m.focused().is_none());
    let shortcut = |key: egui::Key| egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key);
    // Célula aberta também trava: a página nova descartaria o que se digitou (achado do revisor).
    let can_reload = !tab.loading && !locked && tab.commit.is_none() && tab.editing.is_none();
    let mut reload = false;
    if in_data
        && locked
        && tab.commit.is_none()
        && ui.input_mut(|i| i.consume_shortcut(&shortcut(egui::Key::S)))
    {
        open_commit = true;
    }
    if in_data
        && locked
        && tab.commit.is_none()
        && nothing_focused
        && ui.input_mut(|i| i.consume_shortcut(&shortcut(egui::Key::Z)))
    {
        discard = true;
    }
    if in_data && can_reload && ui.input_mut(|i| i.consume_shortcut(&reload_shortcut())) {
        reload = true;
    }
    if in_data
        && editable
        && nothing_focused
        && tab.editing.is_none()
        && tab.commit.is_none()
        && !tab.selected_rows.is_empty()
        && ui.input(|i| i.key_pressed(egui::Key::Backspace))
    {
        toggle_delete = true;
    }

    // A barra da aba: sub-abas (FEAT-009), a tabela com linhas e tamanho, e as ações da sub-aba.
    egui::Panel::top(egui::Id::new(("data-toolbar", key)))
        .exact_size(theme::size::BAR)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(tokens.bg)
                .inner_margin(egui::Margin::symmetric(12, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                let mut view = tab.view;
                if widgets::segmented(
                    ui,
                    &mut view,
                    &[
                        (TabView::Data, tr.view_data()),
                        (TabView::Structure, tr.view_structure()),
                        (TabView::Index, tr.view_indexes()),
                    ],
                ) {
                    view_change = Some(view);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if tab.view == TabView::Data {
                        let reload_label = i18n::shortcut_label(ui.ctx(), &reload_shortcut());
                        if widgets::icon_button_full(
                            ui,
                            can_reload,
                            icon::REFRESH,
                            tr.reload(),
                            Some(&reload_label),
                            if locked || tab.editing.is_some() {
                                tr.lock_hint()
                            } else {
                                ""
                            },
                        )
                        .clicked()
                        {
                            reload = true;
                        }
                    }
                    if tab.view == TabView::Structure
                        && widgets::button_enabled(
                            ui,
                            tab.structure.is_some(),
                            widgets::ButtonKind::Normal,
                            Some(icon::SQL),
                            tr.view_ddl(),
                            "",
                        )
                        .clicked()
                    {
                        open_ddl = true;
                    }
                    if tab.loading {
                        ui.spinner();
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add_space(theme::space::S);
                        let name = ui.label(
                            egui::RichText::new(format!("{}.{}", tab.schema, tab.table))
                                .family(crate::fonts::strong(ui.ctx()))
                                .color(tokens.text),
                        );
                        if let Some(comment) = tab.stats.as_ref().and_then(|s| s.comment.as_deref())
                        {
                            name.on_hover_text(comment);
                        }
                        if let Some(stats) = &tab.stats {
                            ui.label(
                                egui::RichText::new(
                                    tr.table_summary(stats.row_count, &stats.total_size),
                                )
                                .size(theme::text_size::SMALL)
                                .color(tokens.text_muted),
                            )
                            .on_hover_text(format!(
                                "{} {} · {} {}",
                                tr.view_data(),
                                stats.data_size,
                                tr.view_indexes(),
                                stats.index_size
                            ));
                        }
                        if read_only {
                            ui.label(
                                egui::RichText::new(tr.read_only_tag())
                                    .size(theme::text_size::SMALL)
                                    .color(tokens.text_faint),
                            );
                        }
                    });
                });
            });
        });

    if in_data {
        footer(
            ui,
            tr,
            &tab,
            editable,
            read_only,
            &mut FooterActions {
                next_page: &mut next_page,
                toggle_delete: &mut toggle_delete,
                open_commit: &mut open_commit,
                discard: &mut discard,
            },
        );
    }

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
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(tokens.bg))
        .show(ui, |ui| {
            if let Some(error) = &tab.error {
                // O grid já carregado fica; o erro (página ou stats) vai numa faixa em cima dele.
                let action = (!locked).then(|| tr.retry());
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .show(ui, |ui| {
                        if widgets::banner(ui, widgets::BannerKind::Error, error, action) {
                            retry = true;
                        }
                    });
                if tab.rows.is_none() {
                    return;
                }
            }
            let Some(rows) = tab.rows.as_ref() else {
                ui.add_space(theme::space::L);
                widgets::loading(ui, tr.loading());
                return;
            };
            // (tipo, chave primária) de cada coluna, pelo catálogo.
            let catalog: Vec<(Option<String>, bool)> = app
                .workspaces
                .get(key)
                .map(|ws| ws.catalog_columns(&tab.schema, &tab.table, &rows.columns))
                .unwrap_or_default()
                .into_iter()
                .map(|c| {
                    c.map_or((None, false), |c| {
                        (Some(c.data_type.clone()), c.is_primary_key)
                    })
                })
                .collect();
            let numeric: Vec<bool> = catalog
                .iter()
                .map(|(ty, _)| ty.as_deref().is_some_and(is_numeric_type))
                .collect();
            // Lidos antes de o TableBuilder tomar o `ui`: ⌘ alterna e ⇧ estende a seleção; Esc cancela a célula.
            let modifiers = ui.input(|i| i.modifiers);
            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
            // A Table do egui_extras só rola na vertical: com muitas colunas as da direita ficavam fora
            // da tela sem jeito de alcançar. O ScrollArea horizontal por fora dá a barra de baixo; a
            // rolagem vertical continua dentro da tabela (mesmo arranjo da demo oficial do egui).
            egui::ScrollArea::horizontal()
                .id_salt(("data-grid-hscroll", key, &tab.schema, &tab.table))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Larguras de coluna e rolagem vertical são de cada tabela de cada banco, não da aba.
                    let mut builder = TableBuilder::new(ui)
                        .id_salt(("data-table", key, &tab.schema, &tab.table))
                        .striped(true)
                        .resizable(true)
                        .sense(egui::Sense::click())
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(TableColumn::exact(48.0));
                    for _ in &rows.columns {
                        builder =
                            builder.column(TableColumn::initial(140.0).at_least(60.0).clip(true));
                    }
                    builder
                        .header(ROW_HEIGHT * 1.6, |mut header| {
                            header.col(|ui| {
                                ui.label(
                                    egui::RichText::new("#")
                                        .size(theme::text_size::SMALL)
                                        .color(tokens.text_faint),
                                );
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
                                        Some(Direction::Asc) => format!(" {}", icon::UP),
                                        Some(Direction::Desc) => format!(" {}", icon::DOWN),
                                        None => String::new(),
                                    };
                                    let (ty, primary) = catalog.get(i).cloned().unwrap_or_default();
                                    let key_mark = if primary {
                                        format!("{} ", icon::KEY)
                                    } else {
                                        String::new()
                                    };
                                    let mut label = None;
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 0.0;
                                        label = Some(
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(format!(
                                                        "{key_mark}{name}{arrow}"
                                                    ))
                                                    .family(crate::fonts::strong(ui.ctx()))
                                                    .color(tokens.text),
                                                )
                                                .selectable(false)
                                                .sense(egui::Sense::click()),
                                            ),
                                        );
                                        if let Some(ty) = &ty {
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(ty)
                                                        .size(theme::text_size::SMALL - 1.0)
                                                        .color(tokens.text_faint),
                                                )
                                                .selectable(false),
                                            );
                                        }
                                    });
                                    let Some(label) = label else {
                                        return;
                                    };
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
                                        label.on_hover_text(tr.lock_hint());
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
                                    let mut text = egui::RichText::new(number)
                                        .size(theme::text_size::SMALL)
                                        .color(tokens.text_faint);
                                    if deleted {
                                        text = text.strikethrough().color(tokens.danger);
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
                                    let value =
                                        tab.cell_value(index, col).flatten().map(str::to_owned);
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
                                                    .background_color(tokens.field)
                                                    .desired_width(f32::INFINITY),
                                            );
                                            if tab.editing.as_ref().is_some_and(|e| e.focus_pending)
                                            {
                                                field.request_focus();
                                            }
                                            edit_text = Some(text);
                                            // Enter e Esc soltam o foco sozinhos; clicar fora também confirma.
                                            if field.lost_focus() {
                                                finish_edit = Some(!escape);
                                            }
                                            return;
                                        }
                                        let (fill, ink) = cell_colors(&tokens, pending, deleted);
                                        if let Some(fill) = fill {
                                            ui.painter().rect_filled(ui.max_rect(), 0.0, fill);
                                        }
                                        let mut text = match &value {
                                            Some(value) => {
                                                egui::RichText::new(value).monospace().color(ink)
                                            }
                                            None => egui::RichText::new("NULL")
                                                .italics()
                                                .size(theme::text_size::SMALL)
                                                .color(if pending {
                                                    ink
                                                } else {
                                                    tokens.text_faint
                                                }),
                                        };
                                        if deleted {
                                            text = text.strikethrough();
                                        }
                                        // Número à direita, como numa planilha.
                                        let layout = if numeric.get(col).copied().unwrap_or(false) {
                                            egui::Layout::right_to_left(egui::Align::Center)
                                        } else {
                                            egui::Layout::left_to_right(egui::Align::Center)
                                        };
                                        // Sem seleção de texto na célula: o Label selecionável consome o clique e ele
                                        // não chega à linha (só o número selecionava). Copiar é pelo painel.
                                        ui.with_layout(layout, |ui| {
                                            ui.add(egui::Label::new(text).selectable(false));
                                        });
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
    } else if reload {
        app.reload_data(key);
    } else if let Some(page) = next_page {
        app.load_data_page(key, page);
    }
}

/// O que o rodapé pede; o `show` age depois de desenhar.
struct FooterActions<'a> {
    next_page: &'a mut Option<Page>,
    toggle_delete: &'a mut bool,
    open_commit: &'a mut bool,
    discard: &'a mut bool,
}

/// O rodapé da sub-aba Dados (telas 7 e 8): a dica de edição ou as pendências com Descartar e
/// Revisar e gravar; à direita, linhas por página, o contador e a paginação.
fn footer(
    ui: &mut egui::Ui,
    tr: i18n::Tr,
    tab: &crate::app::DataTab,
    editable: bool,
    read_only: bool,
    acts: &mut FooterActions<'_>,
) {
    let tokens = theme::current(ui.ctx());
    let locked = tab.has_changes();
    egui::Panel::bottom("data-footer")
        .exact_size(theme::size::FOOTER)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(tokens.panel)
                .inner_margin(egui::Margin::symmetric(12, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                let idle = tab.commit.is_none();
                if locked {
                    let (updates, deletes) = tab.change_counts();
                    widgets::pending_chip(ui, &tr.pending_chip(updates as u64, deletes as u64))
                        .on_hover_text(tr.lock_detail());
                    let z = i18n::shortcut_label(
                        ui.ctx(),
                        &egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Z),
                    );
                    if widgets::button_shortcut(
                        ui,
                        idle,
                        widgets::ButtonKind::Ghost,
                        Some(icon::UNDO),
                        tr.discard(),
                        &z,
                        "",
                    )
                    .clicked()
                    {
                        *acts.discard = true;
                    }
                    let s = i18n::shortcut_label(
                        ui.ctx(),
                        &egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S),
                    );
                    if widgets::button_shortcut(
                        ui,
                        idle,
                        widgets::ButtonKind::Primary,
                        None,
                        tr.review_and_commit(),
                        &s,
                        "",
                    )
                    .clicked()
                    {
                        *acts.open_commit = true;
                    }
                } else if editable {
                    ui.label(
                        egui::RichText::new(tr.double_click_hint())
                            .size(theme::text_size::SMALL)
                            .color(tokens.text_faint),
                    );
                }
                // Apagar pelo mouse: só com linha selecionada numa tabela que edita (⌫ faz o mesmo).
                if editable && !tab.selected_rows.is_empty() && idle && !read_only {
                    let response = widgets::button(
                        ui,
                        widgets::ButtonKind::Ghost,
                        Some(icon::DELETE),
                        tr.delete_rows(),
                    )
                    .on_hover_text(tr.delete_rows_hint());
                    if response.clicked() {
                        *acts.toggle_delete = true;
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
                    let lock_hint = if locked { tr.lock_hint() } else { "" };
                    if widgets::icon_button_full(
                        ui,
                        can_next && free,
                        icon::NEXT,
                        tr.next_page(),
                        None,
                        lock_hint,
                    )
                    .clicked()
                    {
                        *acts.next_page = Some(Page {
                            offset: offset + limit,
                            ..tab.page.clone()
                        });
                    }
                    if widgets::icon_button_full(
                        ui,
                        can_prev && free,
                        icon::PREVIOUS,
                        tr.previous_page(),
                        None,
                        lock_hint,
                    )
                    .clicked()
                    {
                        *acts.next_page = Some(Page {
                            offset: offset.saturating_sub(limit),
                            ..tab.page.clone()
                        });
                    }
                    let selected = tab.selected_rows.len() as u64;
                    let counter = if selected > 0 {
                        tr.selected_rows(selected, shown)
                    } else if shown == 0 {
                        tr.page_range(0, 0, total)
                    } else {
                        tr.page_range(offset + 1, offset + shown, total)
                    };
                    ui.label(
                        egui::RichText::new(counter)
                            .monospace()
                            .color(tokens.text_muted),
                    );
                    ui.add_enabled_ui(free, |ui| {
                        egui::ComboBox::from_id_salt("page-size")
                            .selected_text(tab.page.limit.to_string())
                            .width(64.0)
                            .show_ui(ui, |ui| {
                                for size in PAGE_SIZES {
                                    if ui
                                        .selectable_label(tab.page.limit == size, size.to_string())
                                        .clicked()
                                        && size != tab.page.limit
                                    {
                                        *acts.next_page = Some(Page {
                                            limit: size,
                                            offset: 0,
                                            ..tab.page.clone()
                                        });
                                    }
                                }
                            });
                    })
                    .response
                    .on_disabled_hover_text(lock_hint);
                    ui.label(
                        egui::RichText::new(tr.rows_per_page())
                            .size(theme::text_size::SMALL)
                            .color(tokens.text_faint),
                    );
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pendencia_e_exclusao_pintam_a_celula_por_cima_da_selecao() {
        for which in [egui::Theme::Light, egui::Theme::Dark] {
            let t = Tokens::for_theme(which);
            let selection = theme::visuals(which).selection.bg_fill;
            let (fill, ink) = cell_colors(&t, true, false);
            assert_eq!(fill, Some(t.pending));
            assert_eq!(ink, t.pending_ink);
            assert_ne!(fill, Some(selection), "pendência igual à seleção");
            // Exclusão vence a pendência: a linha inteira vai sair.
            assert_eq!(cell_colors(&t, true, true).0, Some(t.danger_soft));
            assert_eq!(cell_colors(&t, false, false), (None, t.text));
        }
    }

    #[test]
    fn numeros_alinham_a_direita_pelo_tipo() {
        for ty in [
            "integer",
            "bigint",
            "numeric(12,2)",
            "double precision",
            " INT8 ",
        ] {
            assert!(is_numeric_type(ty), "{ty}");
        }
        for ty in [
            "text",
            "character varying(80)",
            "timestamptz",
            "uuid",
            "",
            "integer[]",
            "numeric(12,2)[]",
        ] {
            assert!(!is_numeric_type(ty), "{ty}");
        }
    }
}
