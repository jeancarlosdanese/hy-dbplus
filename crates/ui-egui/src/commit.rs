//! O diálogo de gravação (FEAT-007, tela 9 do protótipo da FEAT-015): o título diz quanto e onde
//! grava, o subtítulo traz o ambiente e a garantia da transação, e o corpo, o SQL exato que vai
//! rodar. Erro do servidor fica no diálogo com o comando que falhou (statement ou contagem:
//! desfeito; conexão caída no COMMIT: a mensagem do driver diz) e as pendências continuam no grid.
//! Esc, clique fora e Cancelar fecham quando nada está rodando; durante a gravação nada fecha.

use hy_dbplus_engine::Driver;

use crate::app::{App, DbKey};
use crate::fonts::icon;
use crate::{i18n, theme, widgets};

/// ⌘↩ grava.
pub fn commit_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Enter)
}

pub fn show<D>(app: &mut App<D>, ctx: &egui::Context, key: &DbKey)
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    let Some((dialog, table)) = app
        .workspaces
        .get(key)
        .and_then(|ws| ws.data())
        .and_then(|tab| Some((tab.commit.clone()?, tab.table.clone())))
    else {
        return;
    };
    let tr = app.tr();
    let tokens = theme::current(ctx);
    let tag = app.deps.store.get(&key.0).map(|c| c.tag);
    let title = tr.commit_title(dialog.updates as u64, dialog.deletes as u64, &table);
    let sql = dialog
        .sql
        .iter()
        .map(|s| format!("{s};"))
        .collect::<Vec<_>>()
        .join("\n");
    let failed = dialog
        .failed
        .and_then(|n| dialog.sql.get(n.checked_sub(1)?))
        .map(|s| format!("{s};"));
    let action_label = if dialog.running {
        tr.committing()
    } else {
        tr.commit_to_database()
    };
    let commit_label = i18n::shortcut_label(ctx, &commit_shortcut());
    let mut copy = false;
    let outcome = widgets::dialog_full(
        ctx,
        widgets::DialogParts {
            id: egui::Id::new("commit-dialog"),
            title: &title,
            subtitle: None,
            width: 600.0,
            cancel: tr.cancel(),
            action: Some(widgets::DialogAction {
                label: action_label,
                kind: widgets::ButtonKind::Primary,
                enabled: true,
                shortcut: Some(&commit_label),
            }),
            busy: dialog.running,
        },
        |ui| {
            ui.horizontal(|ui| {
                if let Some(tag) = tag {
                    widgets::env_tag(ui, tag);
                }
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(tr.commit_subtitle()).color(tokens.text_muted),
                    )
                    .wrap(),
                );
            });
            ui.add_space(theme::space::M);
            widgets::sql_block(ui, "commit-sql", &sql, 320.0);
            if let Some(error) = &dialog.error {
                ui.add_space(theme::space::M);
                widgets::banner(ui, widgets::BannerKind::Error, error, None);
                if let Some(statement) = &failed {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(statement)
                                .monospace()
                                .color(tokens.danger),
                        )
                        .wrap(),
                    );
                }
            }
        },
        |ui| {
            if widgets::button(
                ui,
                widgets::ButtonKind::Normal,
                Some(icon::COPY),
                tr.copy_sql(),
            )
            .clicked()
            {
                copy = true;
            }
        },
    );
    if copy {
        ctx.copy_text(sql.clone());
    }
    // ⌘↩ só vale com a gravação no topo: com a paleta ou a confirmação de fechar por cima, o
    // atalho é delas (achado do revisor). O topo do egui é o do frame anterior.
    let commit_layer = egui::Modal::default_area(egui::Id::new("commit-dialog")).layer();
    let on_top = ctx.memory(|m| m.top_modal_layer()) == Some(commit_layer)
        && app.palette.is_none()
        && app.confirm_close.is_none();
    let shortcut_commit =
        !dialog.running && on_top && ctx.input_mut(|i| i.consume_shortcut(&commit_shortcut()));
    match outcome {
        widgets::DialogOutcome::Confirm => app.execute_changes(key),
        widgets::DialogOutcome::Cancel => app.cancel_commit(key),
        widgets::DialogOutcome::Open if shortcut_commit => app.execute_changes(key),
        widgets::DialogOutcome::Open => {}
    }
}
