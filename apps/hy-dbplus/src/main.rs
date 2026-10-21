//! hy-dbplus: o app desktop. Cria a janela (eframe) e entrega o resto à `ui-egui`.
//! Falha ao abrir a janela (sem display, sem OpenGL) vira mensagem no stderr e saída 1.

use std::process::ExitCode;

use eframe::egui;
use hy_dbplus_ui_egui::App;

/// Identificador da janela no Wayland e nome do app para o sistema.
const APP_ID: &str = "br.com.hyberica.hy-dbplus";

fn main() -> ExitCode {
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
    match eframe::run_native(
        "hy-dbplus",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("hy-dbplus: não consegui abrir a janela: {e}");
            ExitCode::FAILURE
        }
    }
}
