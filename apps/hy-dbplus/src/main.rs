//! hy-dbplus: o app desktop. Cria o runtime, o engine e a janela (eframe); entrega o resto à
//! `ui-egui`. Falha ao abrir a janela vira mensagem no stderr e saída 1.

use std::process::ExitCode;

use eframe::egui;
use hy_dbplus_engine::{
    ConnectionStore, KeyringSecrets, MemorySecrets, PgSessions, QueryStore, SecretStore,
};
use hy_dbplus_ui_egui::{Deps, PgApp};

/// Bundle id (docs/BRIEF.md decisão 2): janela no Wayland e nome do app para o sistema.
const APP_ID: &str = "br.com.hy.dbplus";
/// O ícone da Dock (FEAT-014): `packaging/icon.svg` em 256 px (receita no brief da FEAT-014).
const ICON_PNG: &[u8] = include_bytes!("../../../packaging/icon-256.png");

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("hy-dbplus: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("hy-dbplus-io")
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    let path = ConnectionStore::default_path().map_err(|e| e.to_string())?;
    let store = ConnectionStore::load(&path).map_err(|e| e.to_string())?;
    let queries = QueryStore::new(QueryStore::default_dir().map_err(|e| e.to_string())?);
    // HY_DBPLUS_SECRETS=memory: sem chaveiro (desenvolvimento e CI); senha vale só na sessão.
    let secrets: Box<dyn SecretStore> =
        if std::env::var_os("HY_DBPLUS_SECRETS").is_some_and(|v| v == "memory") {
            Box::new(MemorySecrets::default())
        } else {
            Box::new(KeyringSecrets::new())
        };
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("hy-dbplus")
        .with_app_id(APP_ID)
        .with_inner_size([1200.0, 780.0])
        .with_min_inner_size([800.0, 500.0]);
    // O ícone embutido (FEAT-014): o eframe o põe na Dock ao abrir, então aparece também no
    // `cargo run` e passa por cima de um tile antigo em cache. O `.app` tem o `.icns` próprio.
    match eframe::icon_data::from_png_bytes(ICON_PNG) {
        Ok(icon) => viewport = viewport.with_icon(icon),
        // Sem pânico e sem silêncio: fica o ícone padrão do egui e o stderr diz por quê.
        Err(e) => eprintln!("hy-dbplus: ícone embutido inválido: {e}"),
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let handle = runtime.handle().clone();
    eframe::run_native(
        "hy-dbplus",
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let (sessions, events) = PgSessions::new(
                hy_dbplus_engine::PostgresDriver::default(),
                handle,
                move || ctx.request_repaint(),
            );
            Ok(Box::new(PgApp::new(
                cc,
                Deps {
                    store,
                    secrets,
                    queries,
                    sessions,
                    events,
                },
            )))
        }),
    )
    .map_err(|e| format!("não consegui abrir a janela: {e}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn o_icone_embutido_decodifica_em_256_por_256() {
        let icon = eframe::icon_data::from_png_bytes(super::ICON_PNG).unwrap();
        assert_eq!((icon.width, icon.height), (256, 256));
    }
}
