//! hy-dbplus: o app desktop. Cria o runtime, o engine e a janela (eframe); entrega o resto à
//! `ui-egui`. Falha ao abrir a janela vira mensagem no stderr e saída 1.

use std::process::ExitCode;

use eframe::egui;
use hy_dbplus_engine::{ConnectionStore, KeyringSecrets, MemorySecrets, PgSessions, SecretStore};
use hy_dbplus_ui_egui::{Deps, PgApp};

/// Bundle id (docs/BRIEF.md decisão 2): janela no Wayland e nome do app para o sistema.
const APP_ID: &str = "br.com.hy.dbplus";

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
    // HY_DBPLUS_SECRETS=memory: sem chaveiro (desenvolvimento e CI); senha vale só na sessão.
    let secrets: Box<dyn SecretStore> =
        if std::env::var_os("HY_DBPLUS_SECRETS").is_some_and(|v| v == "memory") {
            Box::new(MemorySecrets::default())
        } else {
            Box::new(KeyringSecrets::new())
        };
    let viewport = egui::ViewportBuilder::default()
        .with_title("hy-dbplus")
        .with_app_id(APP_ID)
        .with_inner_size([1200.0, 780.0])
        .with_min_inner_size([800.0, 500.0]);
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
                    sessions,
                    events,
                },
            )))
        }),
    )
    .map_err(|e| format!("não consegui abrir a janela: {e}"))
}
