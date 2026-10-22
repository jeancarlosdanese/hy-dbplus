//! A aba Data (tela 3.4): grid virtualizado da página atual, rodapé com abas, contador e
//! paginação, e o painel direito com tamanhos, comentário e os campos da linha selecionada.

use egui_extras::{Column as TableColumn, TableBuilder};
use hy_dbplus_engine::{ConnectionId, Direction, Driver, Order, Page};

use crate::app::{App, DataTab};
use crate::theme;

const ROW_HEIGHT: f32 = 22.0;
pub const PAGE_SIZES: [u32; 3] = [100, 500, 1000];

/// O centro da janela quando há uma tabela aberta.
pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, id: &ConnectionId)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let Some(tab) = app.workspaces.get(id).and_then(|ws| ws.data.clone()) else {
        return;
    };
    let mut next_page: Option<Page> = None;
    let mut select: Option<usize> = None;
    let mut retry = false;

    egui::Panel::bottom("data-footer").show(ui, |ui| {
        ui.horizontal(|ui| {
            let _ = ui.selectable_label(true, "Data");
            ui.add_enabled(false, egui::Button::new("Structure"))
                .on_disabled_hover_text("em breve");
            ui.add_enabled(false, egui::Button::new("Index"))
                .on_disabled_hover_text("em breve");
            ui.separator();
            ui.add_enabled(false, egui::Button::new("+ Row"))
                .on_disabled_hover_text("não disponível nesta versão");
            ui.add_enabled(false, egui::Button::new("Filters"))
                .on_disabled_hover_text("em breve");
            ui.add_enabled(false, egui::Button::new("Columns"))
                .on_disabled_hover_text("em breve");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let shown = tab.rows.as_ref().map(|r| r.rows.len() as u64).unwrap_or(0);
                let (limit, offset) = (u64::from(tab.page.limit), tab.page.offset);
                let total = tab.stats.as_ref().map(|s| s.row_count);
                // Sem total ainda, página cheia sugere que há mais.
                let can_next = total.is_some_and(|t| offset + limit < t)
                    || (total.is_none() && shown == limit);
                let can_prev = offset > 0;
                if ui
                    .add_enabled(can_next && !tab.loading, egui::Button::new("›"))
                    .clicked()
                {
                    next_page = Some(Page {
                        offset: offset + limit,
                        ..tab.page.clone()
                    });
                }
                if ui
                    .add_enabled(can_prev && !tab.loading, egui::Button::new("‹"))
                    .clicked()
                {
                    next_page = Some(Page {
                        offset: offset.saturating_sub(limit),
                        ..tab.page.clone()
                    });
                }
                let total_text = total.map_or_else(|| "…".to_owned(), |t| t.to_string());
                let counter = if shown == 0 {
                    format!("0 of {total_text} rows")
                } else {
                    format!("{}-{} of {total_text} rows", offset + 1, offset + shown)
                };
                ui.label(egui::RichText::new(counter).monospace());
                egui::ComboBox::from_id_salt("page-size")
                    .selected_text(tab.page.limit.to_string())
                    .show_ui(ui, |ui| {
                        for size in PAGE_SIZES {
                            if ui
                                .selectable_label(tab.page.limit == size, size.to_string())
                                .clicked()
                                && size != tab.page.limit
                                && !tab.loading
                            {
                                next_page = Some(Page {
                                    limit: size,
                                    offset: 0,
                                    ..tab.page.clone()
                                });
                            }
                        }
                    });
                ui.label(
                    egui::RichText::new("por página")
                        .small()
                        .color(tokens.text_muted),
                );
            });
        });
    });

    egui::CentralPanel::default_margins().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{}.{}", tab.schema, tab.table)).strong());
            if tab.loading {
                ui.label(
                    egui::RichText::new("carregando…")
                        .small()
                        .color(tokens.text_muted),
                );
            }
        });
        if let Some(error) = &tab.error {
            // O grid já carregado fica; o erro (página ou stats) vai numa faixa em cima dele.
            ui.horizontal(|ui| {
                ui.colored_label(theme::error_color(), error);
                if ui.small_button("Tentar de novo").clicked() {
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
        let types: Vec<Option<String>> = {
            let cols = app
                .workspaces
                .get(id)
                .and_then(|ws| ws.columns.get(&(tab.schema.clone(), tab.table.clone())));
            rows.columns
                .iter()
                .map(|name| {
                    cols.and_then(|c| c.iter().find(|c| &c.name == name))
                        .map(|c| c.data_type.clone())
                })
                .collect()
        };
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
                        if label.clicked() && !tab.loading {
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
                    });
                }
            })
            .body(|body| {
                body.rows(ROW_HEIGHT, rows.rows.len(), |mut row| {
                    let index = row.index();
                    row.set_selected(tab.selected_row == Some(index));
                    row.col(|ui| {
                        // O número da linha é o alvo de seleção; o resto da linha também seleciona.
                        let number = (tab.page.offset + index as u64 + 1).to_string();
                        if ui
                            .selectable_label(
                                tab.selected_row == Some(index),
                                egui::RichText::new(number).small(),
                            )
                            .clicked()
                        {
                            select = Some(index);
                        }
                    });
                    if let Some(cells) = rows.rows.get(index) {
                        for cell in cells {
                            row.col(|ui| match cell {
                                Some(value) => {
                                    ui.label(egui::RichText::new(value).monospace());
                                }
                                None => {
                                    ui.label(
                                        egui::RichText::new("NULL")
                                            .italics()
                                            .color(tokens.text_muted),
                                    );
                                }
                            });
                        }
                    }
                    if row.response().clicked() {
                        select = Some(index);
                    }
                });
            });
    });

    if let Some(index) = select
        && let Some(tab) = app.workspaces.get_mut(id).and_then(|ws| ws.data.as_mut())
    {
        tab.selected_row = if tab.selected_row == Some(index) {
            None
        } else {
            Some(index)
        };
    }
    if retry {
        app.retry_data(id);
    } else if let Some(page) = next_page {
        app.load_data_page(id, page);
    }
}

/// O painel direito: tamanhos e comentário, ou os campos da linha selecionada. Tudo quebra linha
/// na largura do painel: valor ou comentário longo não trava a largura mínima.
pub fn side_panel(ui: &mut egui::Ui, tab: &DataTab) {
    let tokens = theme::current(ui.ctx());
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
    if let (Some(index), Some(rows)) = (tab.selected_row, tab.rows.as_ref())
        && let Some(cells) = rows.rows.get(index)
    {
        ui.label(
            egui::RichText::new(format!("Linha {}", tab.page.offset + index as u64 + 1)).strong(),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (name, cell) in rows.columns.iter().zip(cells) {
                ui.label(egui::RichText::new(name).small().color(tokens.text_muted));
                match cell {
                    Some(value) => ui.label(egui::RichText::new(value).monospace()),
                    None => ui.label(
                        egui::RichText::new("NULL")
                            .italics()
                            .color(tokens.text_muted),
                    ),
                };
                ui.add_space(6.0);
            }
        });
        return;
    }
    ui.label(egui::RichText::new(&tab.table).strong());
    ui.separator();
    match &tab.stats {
        Some(stats) => {
            for (name, value) in [
                ("total_size", stats.total_size.as_str()),
                ("data_size", stats.data_size.as_str()),
                ("index_size", stats.index_size.as_str()),
                ("rows", &stats.row_count.to_string()),
            ] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(name).small().color(tokens.text_muted));
                    ui.label(value);
                });
            }
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("comment")
                    .small()
                    .color(tokens.text_muted),
            );
            ui.label(stats.comment.as_deref().unwrap_or("—"));
        }
        None => {
            ui.label(
                egui::RichText::new("carregando…")
                    .small()
                    .color(tokens.text_muted),
            );
        }
    }
}
