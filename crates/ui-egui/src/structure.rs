//! A sub-aba Structure (FEAT-009, tela 3.5), só leitura: colunas com chave, tipo, nulo, default
//! (ou identidade/gerada) e comentário; as restrições com a FK apontando para a tabela referenciada;
//! para view, a definição. O botão DDL do rodapé abre a caixa com o `CREATE` montado pelo catálogo.

use egui_extras::{Column as TableColumn, TableBuilder};
use hy_dbplus_engine::{ConstraintKind, Driver, TableStructure, table_ddl};

use crate::app::{App, DbKey, TabView};
use crate::grid::{ROW_HEIGHT, cell, header};
use crate::theme;

/// O centro da aba quando a sub-aba é Structure.
pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, key: &DbKey)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let Some(tab) = app.workspaces.get(key).and_then(|ws| ws.data()) else {
        return;
    };
    let (structure, error, requested) = (
        tab.structure.clone(),
        tab.structure_error.clone(),
        tab.structure_requested,
    );
    let mut retry = false;
    egui::CentralPanel::default_margins().show(ui, |ui| {
        let Some(structure) = structure else {
            structure_status(
                ui,
                error.as_deref(),
                requested,
                "Carregando estrutura…",
                &mut retry,
            );
            return;
        };
        egui::ScrollArea::both().show(ui, |ui| {
            columns_grid(ui, &structure, &tokens);
            ui.add_space(12.0);
            if let Some(definition) = &structure.view_definition {
                ui.label(egui::RichText::new("Definição").strong());
                ui.add(egui::Label::new(egui::RichText::new(definition).monospace()).wrap());
            } else {
                constraints_grid(ui, &structure, &tokens);
            }
        });
    });
    if retry {
        app.set_view(key, TabView::Structure);
    }
}

/// Sem estrutura ainda: o erro com "Tentar de novo", ou "Carregando…" só se há pedido em voo.
pub fn structure_status(
    ui: &mut egui::Ui,
    error: Option<&str>,
    requested: bool,
    loading: &str,
    retry: &mut bool,
) {
    let tokens = theme::current(ui.ctx());
    if let Some(error) = error {
        ui.horizontal(|ui| {
            ui.colored_label(theme::error_color(), error);
            if ui.small_button("Tentar de novo").clicked() {
                *retry = true;
            }
        });
    } else if requested {
        ui.label(egui::RichText::new(loading).color(tokens.text_muted));
    }
}

fn columns_grid(ui: &mut egui::Ui, structure: &TableStructure, tokens: &theme::Tokens) {
    ui.label(egui::RichText::new("Colunas").strong());
    ui.push_id("structure-columns", |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(TableColumn::exact(36.0))
            .column(TableColumn::initial(180.0).at_least(80.0).clip(true))
            .column(TableColumn::initial(160.0).at_least(80.0).clip(true))
            .column(TableColumn::exact(48.0))
            .column(TableColumn::initial(220.0).at_least(80.0).clip(true))
            .column(TableColumn::remainder().at_least(80.0).clip(true))
            .header(ROW_HEIGHT + 4.0, |mut row| {
                for name in ["#", "coluna", "tipo", "nulo", "default", "comentário"] {
                    row.col(|ui| header(ui, name, tokens));
                }
            })
            .body(|body| {
                body.rows(ROW_HEIGHT, structure.columns.len(), |mut row| {
                    let Some(c) = structure.columns.get(row.index()) else {
                        return;
                    };
                    row.col(|ui| cell(ui, &c.position.to_string()));
                    row.col(|ui| {
                        let name = if c.is_primary_key {
                            format!("🔑 {}", c.name)
                        } else {
                            c.name.clone()
                        };
                        cell(ui, &name);
                    });
                    row.col(|ui| cell(ui, &c.data_type));
                    row.col(|ui| cell(ui, if c.nullable { "sim" } else { "não" }));
                    row.col(|ui| {
                        let default = if let Some(expr) = &c.generated {
                            format!("GENERATED ALWAYS AS ({expr}) STORED")
                        } else if let Some(identity) = &c.identity {
                            format!("GENERATED {identity} AS IDENTITY")
                        } else {
                            c.default.clone().unwrap_or_default()
                        };
                        cell(ui, &default);
                    });
                    row.col(|ui| cell(ui, c.comment.as_deref().unwrap_or_default()));
                });
            });
    });
}

fn constraints_grid(ui: &mut egui::Ui, structure: &TableStructure, tokens: &theme::Tokens) {
    ui.label(egui::RichText::new("Restrições").strong());
    if structure.constraints.is_empty() {
        ui.label(egui::RichText::new("Nenhuma.").color(tokens.text_muted));
        return;
    }
    ui.push_id("structure-constraints", |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(TableColumn::initial(200.0).at_least(80.0).clip(true))
            .column(TableColumn::exact(100.0))
            .column(TableColumn::initial(160.0).at_least(60.0).clip(true))
            .column(TableColumn::remainder().at_least(120.0).clip(true))
            .header(ROW_HEIGHT + 4.0, |mut row| {
                for name in ["nome", "tipo", "colunas", "definição"] {
                    row.col(|ui| header(ui, name, tokens));
                }
            })
            .body(|body| {
                body.rows(ROW_HEIGHT, structure.constraints.len(), |mut row| {
                    let Some(c) = structure.constraints.get(row.index()) else {
                        return;
                    };
                    row.col(|ui| cell(ui, &c.name));
                    row.col(|ui| cell(ui, c.kind.label()));
                    row.col(|ui| cell(ui, &c.columns.join(", ")));
                    row.col(|ui| {
                        let text = match (&c.kind, &c.references) {
                            (ConstraintKind::ForeignKey, Some(r)) => format!(
                                "→ {}.{}({}) on update {} / on delete {}",
                                r.schema,
                                r.table,
                                r.columns.join(", "),
                                r.on_update,
                                r.on_delete
                            ),
                            _ => c.definition.clone(),
                        };
                        cell(ui, &text);
                    });
                });
            });
    });
}

/// A caixa do DDL: o texto montado pelo catálogo, rolável, com Copiar e Fechar.
pub fn ddl_dialog<D>(app: &mut App<D>, ctx: &egui::Context, key: &DbKey)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let Some(tab) = app.workspaces.get(key).and_then(|ws| ws.data()) else {
        return;
    };
    if !tab.ddl_open {
        return;
    }
    let Some(structure) = tab.structure.clone() else {
        return;
    };
    let ddl = table_ddl(&structure);
    let mut close = false;
    let response = egui::Modal::new(egui::Id::new("ddl-dialog")).show(ctx, |ui| {
        ui.set_width(720.0);
        ui.heading(format!(
            "DDL de {}.{}",
            structure.table.schema, structure.table.name
        ));
        ui.separator();
        egui::ScrollArea::both().max_height(420.0).show(ui, |ui| {
            ui.add(egui::Label::new(egui::RichText::new(&ddl).monospace()).wrap());
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Copiar").clicked() {
                ui.ctx().copy_text(ddl.clone());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Fechar").clicked() {
                    close = true;
                }
            });
        });
    });
    if (close || response.should_close())
        && let Some(tab) = app.workspaces.get_mut(key).and_then(|ws| ws.data_mut())
    {
        tab.ddl_open = false;
    }
}
