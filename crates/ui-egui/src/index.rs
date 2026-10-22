//! A sub-aba Index (FEAT-009, tela 3.6), só leitura: nome, algoritmo, único, colunas, condição,
//! include, parâmetros de armazenamento e comentário de cada índice.

use egui_extras::{Column as TableColumn, TableBuilder};
use hy_dbplus_engine::Driver;

use crate::app::{App, DbKey, TabView};
use crate::grid::{ROW_HEIGHT, cell, header};
use crate::structure::structure_status;
use crate::theme;

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
                "Carregando índices…",
                &mut retry,
            );
            return;
        };
        if structure.indexes.is_empty() {
            ui.label(egui::RichText::new("Nenhum índice.").color(tokens.text_muted));
            return;
        }
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(TableColumn::initial(200.0).at_least(80.0).clip(true))
            .column(TableColumn::exact(70.0))
            .column(TableColumn::exact(56.0))
            .column(TableColumn::initial(160.0).at_least(60.0).clip(true))
            .column(TableColumn::initial(160.0).at_least(60.0).clip(true))
            .column(TableColumn::initial(120.0).at_least(60.0).clip(true))
            .column(TableColumn::initial(120.0).at_least(60.0).clip(true))
            .column(TableColumn::remainder().at_least(80.0).clip(true))
            .header(ROW_HEIGHT + 4.0, |mut row| {
                for name in [
                    "nome",
                    "algoritmo",
                    "único",
                    "colunas",
                    "condição",
                    "include",
                    "parâmetros",
                    "comentário",
                ] {
                    row.col(|ui| header(ui, name, &tokens));
                }
            })
            .body(|body| {
                body.rows(ROW_HEIGHT, structure.indexes.len(), |mut row| {
                    let Some(i) = structure.indexes.get(row.index()) else {
                        return;
                    };
                    row.col(|ui| cell(ui, &i.name));
                    row.col(|ui| cell(ui, &i.algorithm));
                    row.col(|ui| cell(ui, if i.is_unique { "sim" } else { "não" }));
                    row.col(|ui| cell(ui, &i.columns.join(", ")));
                    row.col(|ui| cell(ui, i.condition.as_deref().unwrap_or_default()));
                    row.col(|ui| cell(ui, &i.include.join(", ")));
                    row.col(|ui| cell(ui, &i.storage_parameters.join(", ")));
                    row.col(|ui| cell(ui, i.comment.as_deref().unwrap_or_default()));
                });
            });
    });
    if retry {
        app.set_view(key, TabView::Index);
    }
}
