//! A faixa de bancos abertos (FEAT-008), à esquerda da árvore: um item por (conexão, banco) com a
//! cor do ambiente, o nome do banco e a conexão embaixo; clique seleciona, × fecha (com confirmação
//! quando há edição pendente).

use hy_dbplus_engine::Driver;

use crate::app::{App, CloseTarget, DbKey};
use crate::{theme, widgets};

pub const WIDTH: f32 = 84.0;
const ITEM_HEIGHT: f32 = 92.0;

pub fn show<D>(app: &mut App<D>, ui: &mut egui::Ui)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let tokens = theme::current(ui.ctx());
    let tr = app.tr();
    let mut open_palette = false;
    let mut activate: Option<DbKey> = None;
    let mut close: Option<DbKey> = None;
    egui::Panel::left("rail")
        .exact_size(WIDTH)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(tokens.bg)
                .inner_margin(egui::Margin::symmetric(theme::space::S as i8, 0)),
        )
        .show(ui, |ui| {
            ui.add_space(theme::space::XS + theme::space::HAIR);
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
                    let running = app.busy(&key);
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
                        egui::vec2(WIDTH - 2.0 * theme::space::S, ITEM_HEIGHT),
                        egui::Sense::click(),
                    );
                    ui.painter().rect(
                        rect,
                        tokens.radius,
                        fill,
                        egui::Stroke::new(if active { 2.0 } else { 1.0 }, color),
                        egui::StrokeKind::Inside,
                    );
                    ui.scope_builder(
                        egui::UiBuilder::new().max_rect(rect.shrink(theme::space::XS)),
                        |ui| {
                            ui.vertical_centered(|ui| {
                                // Rótulos sem seleção de texto: o clique tem de chegar ao item.
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(crate::fonts::icon::DATABASE)
                                            .size(theme::text_size::ICON),
                                    )
                                    .selectable(false),
                                );
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(database).small().strong(),
                                    )
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
                                if widgets::icon_button_full(
                                    ui,
                                    !running,
                                    crate::fonts::icon::CLOSE,
                                    tr.close_database(),
                                    None,
                                    tr.wait_for_run(),
                                )
                                .clicked()
                                {
                                    close = Some(key.clone());
                                }
                            });
                        },
                    );
                    if response
                        .on_hover_text(format!("{name} : {database}"))
                        .clicked()
                    {
                        activate = Some(key.clone());
                    }
                    ui.add_space(theme::space::XS);
                }
                // O "+" no fim abre a paleta para escolher outro banco.
                ui.vertical_centered(|ui| {
                    if widgets::icon_button(ui, crate::fonts::icon::ADD, tr.open_another_database())
                        .clicked()
                    {
                        open_palette = true;
                    }
                });
            });
        });
    if open_palette {
        crate::palette::toggle(app);
    }
    if let Some(key) = activate {
        app.activate(&key);
    }
    if let Some(key) = close {
        app.request_close_database(&key);
    }
}

/// Confirmação para fechar banco, aba ou conexão com trabalho a perder (tela 16): o título diz o
/// que fecha, o texto diz quanto se perde, e as saídas são revisar e gravar (com pendência de
/// tabela), cancelar e descartar.
pub fn confirm<D>(app: &mut App<D>, ctx: &egui::Context)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let Some(target) = app.confirm_close.clone() else {
        return;
    };
    let tr = app.tr();
    let tokens = theme::current(ctx);
    let (updates, deletes) = app.pending_counts(&target);
    let has_pending = updates + deletes > 0;
    // Aba de SQL perde o texto do editor; banco e conexão só perguntam por pendência de aba de
    // tabela (o texto das abas SQL é salvo em arquivo antes de fechar, FEAT-011). Célula aberta
    // sem outra pendência perde o valor digitado.
    let (title, discard) = match &target {
        CloseTarget::Database(key) => (tr.close_database_title(&key.1), tr.discard_and_close()),
        CloseTarget::Tab(key, index) => (
            tr.close_tab_title(
                &app.workspaces
                    .get(key)
                    .and_then(|ws| {
                        Some(ws.tabs.get(*index)?.title(ws.current_schema.as_deref(), tr))
                    })
                    .unwrap_or_default(),
            ),
            tr.discard_and_close(),
        ),
        CloseTarget::Connection(id) => (
            tr.disconnect_title(
                &app.deps
                    .store
                    .get(id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default(),
            ),
            tr.discard_and_disconnect(),
        ),
    };
    let is_query = matches!(&target, CloseTarget::Tab(key, index)
        if app.workspaces.get(key).and_then(|ws| ws.tabs.get(*index)).is_some_and(|t| t.query().is_some()));
    let body = if is_query {
        tr.query_text_body().to_owned()
    } else if has_pending {
        tr.pending_body(updates as u64, deletes as u64)
    } else {
        tr.cell_edit_body().to_owned()
    };
    let mut review = false;
    let outcome = widgets::dialog_full(
        ctx,
        widgets::DialogParts {
            id: egui::Id::new("confirm-close"),
            title: &title,
            subtitle: None,
            width: 420.0,
            cancel: tr.cancel(),
            action: Some(widgets::DialogAction {
                label: discard,
                kind: widgets::ButtonKind::Danger,
                enabled: true,
                shortcut: None,
            }),
            busy: false,
        },
        |ui| {
            ui.add(egui::Label::new(egui::RichText::new(&body).color(tokens.text_muted)).wrap());
        },
        |ui| {
            if has_pending
                && widgets::button(
                    ui,
                    widgets::ButtonKind::Normal,
                    None,
                    tr.review_and_commit(),
                )
                .clicked()
            {
                review = true;
            }
        },
    );
    match outcome {
        widgets::DialogOutcome::Confirm => app.confirm_close_now(),
        widgets::DialogOutcome::Cancel => app.confirm_close = None,
        widgets::DialogOutcome::Open if review => app.review_pending(),
        widgets::DialogOutcome::Open => {}
    }
}
