//! A faixa de bancos abertos (FEAT-008), à esquerda da árvore: um item por (conexão, banco) com a
//! cor do ambiente, o nome do banco e a conexão embaixo; clique seleciona, × fecha (com confirmação
//! quando há edição pendente).

use hy_dbplus_engine::Driver;

use crate::app::{App, DbKey};
use crate::theme;

pub const WIDTH: f32 = 84.0;
const ITEM_HEIGHT: f32 = 92.0;

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let mut activate: Option<DbKey> = None;
    let mut close: Option<DbKey> = None;
    egui::Panel::left("rail")
        .exact_size(WIDTH)
        .resizable(false)
        .show(ui, |ui| {
            ui.add_space(6.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                for key in app.rail.clone() {
                    let (id, database) = &key;
                    let tag = app.deps.store.get(id).map(|c| c.tag);
                    let name = app
                        .deps
                        .store
                        .get(id)
                        .map(|c| c.name.clone())
                        .unwrap_or_default();
                    let active = app.active.as_ref() == Some(&key);
                    let color = tag.map(theme::tag_color).unwrap_or(tokens.border);
                    let fill = if active {
                        tokens.accent_soft
                    } else {
                        tokens.panel
                    };
                    // O fundo clicável é registrado ANTES dos filhos: no hit test do egui o empate
                    // vai ao último registrado, e com o fundo por cima o × nunca receberia o clique
                    // (achado do revisor, 22/10/2026). Altura fixa: nome truncado, sem quebra.
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(WIDTH - 16.0, ITEM_HEIGHT),
                        egui::Sense::click(),
                    );
                    ui.painter().rect(
                        rect,
                        tokens.radius,
                        fill,
                        egui::Stroke::new(if active { 2.0 } else { 1.0 }, color),
                        egui::StrokeKind::Inside,
                    );
                    ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink(4.0)), |ui| {
                        ui.vertical_centered(|ui| {
                            // Rótulos sem seleção de texto: o clique tem de chegar ao item.
                            ui.add(
                                egui::Label::new(egui::RichText::new("🛢").size(18.0))
                                    .selectable(false),
                            );
                            ui.add(
                                egui::Label::new(egui::RichText::new(database).small().strong())
                                    .truncate()
                                    .selectable(false),
                            );
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&name).small().color(tokens.text_muted),
                                )
                                .truncate()
                                .selectable(false),
                            );
                            if ui
                                .small_button("×")
                                .on_hover_text("Fechar este banco")
                                .clicked()
                            {
                                close = Some(key.clone());
                            }
                        });
                    });
                    if response
                        .on_hover_text(format!("{name} : {database}"))
                        .clicked()
                    {
                        activate = Some(key.clone());
                    }
                    ui.add_space(4.0);
                }
            });
        });
    if let Some(key) = activate {
        app.activate(&key);
    }
    if let Some(key) = close {
        if app.has_pending_changes(&key) {
            app.confirm_close = Some(key);
        } else {
            app.close_database(&key);
        }
    }
}

/// Confirmação para fechar um banco com edição pendente.
pub fn confirm<D>(app: &mut App<D>, ctx: &egui::Context)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let Some(key) = app.confirm_close.clone() else {
        return;
    };
    let mut decision: Option<bool> = None;
    let response = egui::Modal::new(egui::Id::new("confirm-close-db")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.heading("Fechar o banco?");
        ui.label(format!(
            "{} tem alterações pendentes. Fechar descarta tudo sem gravar.",
            key.1
        ));
        ui.separator();
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Descartar e fechar").clicked() {
                    decision = Some(true);
                }
                if ui.button("Cancelar").clicked() {
                    decision = Some(false);
                }
            });
        });
    });
    if response.should_close() {
        decision = Some(false);
    }
    match decision {
        Some(true) => {
            app.confirm_close = None;
            app.close_database(&key);
        }
        Some(false) => app.confirm_close = None,
        None => {}
    }
}
