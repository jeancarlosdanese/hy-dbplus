//! A sub-aba Estrutura (FEAT-009, tela 10 do protótipo da FEAT-015), só leitura: colunas com chave,
//! tipo, nulo, padrão (ou identidade/gerada) e comentário; as restrições com a chave estrangeira
//! apontando para a tabela referenciada; para view, a definição. "Ver DDL", na barra da aba, abre o
//! diálogo com o `CREATE` montado pelo catálogo (tela 12).

use egui_extras::{Column as TableColumn, TableBuilder};
use hy_dbplus_engine::{ConstraintKind, Driver, TableStructure, table_ddl};

use crate::app::{App, DbKey, TabView};
use crate::fonts::icon;
use crate::grid::{ROW_HEIGHT, cell, cell_muted, header, section, status_footer};
use crate::i18n::Tr;
use crate::{theme, widgets};

/// O centro da aba quando a sub-aba é Structure.
pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui, key: &DbKey)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let tr = app.tr();
    let Some(tab) = app.workspaces.get(key).and_then(|ws| ws.data()) else {
        return;
    };
    let (structure, error, requested) = (
        tab.structure.clone(),
        tab.structure_error.clone(),
        tab.structure_requested,
    );
    let mut retry = false;
    if let Some(structure) = &structure {
        let summary = tr.structure_summary(
            structure.columns.len() as u64,
            structure.constraints.len() as u64,
        );
        status_footer(ui, "structure-footer", &summary);
    }
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(tokens.bg)
                .inner_margin(egui::Margin::symmetric(12, 4)),
        )
        .show(ui, |ui| {
            let Some(structure) = structure else {
                structure_status(
                    ui,
                    tr,
                    error.as_deref(),
                    requested,
                    tr.loading_structure(),
                    &mut retry,
                );
                return;
            };
            egui::ScrollArea::both().show(ui, |ui| {
                columns_grid(ui, tr, &structure, &tokens);
                if let Some(definition) = &structure.view_definition {
                    section(ui, tr.definition(), &tokens);
                    widgets::sql_block(ui, "view-definition", definition, f32::INFINITY);
                } else {
                    constraints_grid(ui, tr, &structure, &tokens);
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
    tr: Tr,
    error: Option<&str>,
    requested: bool,
    loading: &str,
    retry: &mut bool,
) {
    ui.add_space(theme::space::S);
    if let Some(error) = error {
        if widgets::banner(ui, widgets::BannerKind::Error, error, Some(tr.retry())) {
            *retry = true;
        }
    } else if requested {
        widgets::loading(ui, loading);
    }
}

fn columns_grid(ui: &mut egui::Ui, tr: Tr, structure: &TableStructure, tokens: &theme::Tokens) {
    section(ui, tr.columns_title(), tokens);
    ui.push_id("structure-columns", |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(TableColumn::exact(36.0))
            .column(TableColumn::initial(180.0).at_least(80.0).clip(true))
            .column(TableColumn::initial(160.0).at_least(80.0).clip(true))
            .column(TableColumn::exact(84.0))
            .column(TableColumn::initial(220.0).at_least(80.0).clip(true))
            .column(TableColumn::remainder().at_least(80.0).clip(true))
            .header(ROW_HEIGHT + 4.0, |mut row| {
                for name in [
                    "#",
                    tr.column(),
                    tr.data_type(),
                    tr.nullable(),
                    tr.default_value(),
                    tr.comment(),
                ] {
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
                            format!("{} {}", icon::KEY, c.name)
                        } else {
                            c.name.clone()
                        };
                        cell(ui, &name);
                    });
                    row.col(|ui| cell(ui, &c.data_type));
                    row.col(|ui| {
                        cell_muted(ui, if c.nullable { tr.yes() } else { tr.no() }, tokens)
                    });
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
                    row.col(|ui| cell_muted(ui, c.comment.as_deref().unwrap_or_default(), tokens));
                });
            });
    });
}

/// O tipo da restrição por extenso, no idioma.
fn constraint_kind(tr: Tr, kind: ConstraintKind) -> &'static str {
    match kind {
        ConstraintKind::PrimaryKey => tr.primary_key(),
        ConstraintKind::ForeignKey => tr.foreign_key(),
        ConstraintKind::Unique => tr.unique_constraint(),
        ConstraintKind::Check => tr.check_constraint(),
        ConstraintKind::Exclusion => tr.exclusion_constraint(),
    }
}

fn constraints_grid(ui: &mut egui::Ui, tr: Tr, structure: &TableStructure, tokens: &theme::Tokens) {
    section(ui, tr.constraints_title(), tokens);
    if structure.constraints.is_empty() {
        ui.label(egui::RichText::new(tr.no_constraints()).color(tokens.text_muted));
        return;
    }
    ui.push_id("structure-constraints", |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(TableColumn::initial(200.0).at_least(80.0).clip(true))
            .column(TableColumn::exact(140.0))
            .column(TableColumn::initial(160.0).at_least(60.0).clip(true))
            .column(TableColumn::remainder().at_least(120.0).clip(true))
            .header(ROW_HEIGHT + 4.0, |mut row| {
                for name in [
                    tr.name(),
                    tr.data_type(),
                    tr.columns_title(),
                    tr.definition(),
                ] {
                    row.col(|ui| header(ui, name, tokens));
                }
            })
            .body(|body| {
                body.rows(ROW_HEIGHT, structure.constraints.len(), |mut row| {
                    let Some(c) = structure.constraints.get(row.index()) else {
                        return;
                    };
                    row.col(|ui| cell(ui, &c.name));
                    row.col(|ui| cell_muted(ui, constraint_kind(tr, c.kind), tokens));
                    row.col(|ui| cell(ui, &c.columns.join(", ")));
                    row.col(|ui| {
                        let text = match (&c.kind, &c.references) {
                            (ConstraintKind::ForeignKey, Some(r)) => format!(
                                "{} {}.{}({}) on update {} / on delete {}",
                                icon::FOREIGN_KEY,
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

/// O diálogo do DDL (tela 12): o texto montado pelo catálogo, colorido e rolável, com Fechar e
/// Copiar.
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
    let tr = app.tr();
    let ddl = table_ddl(&structure);
    let title = format!("DDL: {}.{}", structure.table.schema, structure.table.name);
    let outcome = widgets::dialog_full(
        ctx,
        widgets::DialogParts {
            id: egui::Id::new("ddl-dialog"),
            title: &title,
            subtitle: Some(tr.ddl_subtitle()),
            width: 680.0,
            cancel: tr.close(),
            action: Some(widgets::DialogAction {
                label: tr.copy(),
                kind: widgets::ButtonKind::Primary,
                enabled: true,
                shortcut: None,
            }),
            busy: false,
        },
        |ui| widgets::sql_block(ui, "ddl-sql", &ddl, 420.0),
        |_| {},
    );
    match outcome {
        // Copiar não fecha: dá para copiar e continuar lendo.
        widgets::DialogOutcome::Confirm => ctx.copy_text(ddl),
        widgets::DialogOutcome::Cancel => {
            if let Some(tab) = app.workspaces.get_mut(key).and_then(|ws| ws.data_mut()) {
                tab.ddl_open = false;
            }
        }
        widgets::DialogOutcome::Open => {}
    }
}
