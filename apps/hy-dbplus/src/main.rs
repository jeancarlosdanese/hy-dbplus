//! hy-dbplus: o app desktop. Cria o runtime, o engine e a janela (eframe); entrega o resto à
//! `ui-egui`. Falha ao abrir a janela vira mensagem no stderr e saída 1.

// No Windows, o build release é um programa gráfico: abre sem a janela de console (FEAT-016). O
// stderr some nesse modo, então o erro que impede a abertura vai também para `error.log`, ao lado do
// `connections.toml` (`report`); o build de debug segue com console.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::Path;
use std::process::ExitCode;

use eframe::egui;
use hy_dbplus_engine::{
    ConnectionStore, KeyringSecrets, MemorySecrets, PgSessions, PreferenceStore, QueryStore,
    SecretStore,
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
            report(&e);
            ExitCode::FAILURE
        }
    }
}

/// Windows release (sem console): o erro que impede a abertura fica em `error.log`, na pasta do
/// `connections.toml`, para não sair calado (achado do revisor). Falha ao gravar não tem a quem
/// avisar e é ignorada. Os erros deste caminho (runtime, arquivos de configuração, janela) não
/// carregam senha: ela só existe no chaveiro.
fn report(message: &str) {
    if cfg!(all(windows, not(debug_assertions)))
        && let Ok(path) = ConnectionStore::default_path()
        && let Some(dir) = path.parent()
    {
        let _ = write_error_log(dir, message);
    }
}

/// Grava `<dir>/error.log` com a mensagem, substituindo o anterior.
fn write_error_log(dir: &Path, message: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("error.log"), format!("hy-dbplus: {message}\n"))
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
    // Preferência ilegível não impede a abertura: vale o idioma do sistema (FEAT-015).
    let preferences =
        PreferenceStore::load(PreferenceStore::default_path().map_err(|e| e.to_string())?);
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
                    preferences,
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
    fn o_erro_de_abertura_vai_para_o_error_log() {
        let dir = std::env::temp_dir().join(format!("hy-dbplus-error-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        super::write_error_log(&dir.join("sub"), "não consegui abrir a janela: sem OpenGL")
            .unwrap();
        let text = std::fs::read_to_string(dir.join("sub").join("error.log")).unwrap();
        assert_eq!(text, "hy-dbplus: não consegui abrir a janela: sem OpenGL\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Fundo transparente fora do quadrado arredondado: o primeiro e o último pixel têm alfa 0
    /// (o `qlmanage` gerava fundo branco opaco e a Dock mostrava um quadrado branco).
    #[test]
    fn o_icone_embutido_decodifica_em_256_por_256_com_cantos_transparentes() {
        let icon = eframe::icon_data::from_png_bytes(super::ICON_PNG).unwrap();
        assert_eq!((icon.width, icon.height), (256, 256));
        let alpha = |i: usize| icon.rgba[i * 4 + 3];
        assert_eq!(alpha(0), 0, "canto superior esquerdo opaco");
        assert_eq!(alpha(256 * 256 - 1), 0, "canto inferior direito opaco");
        assert_eq!(alpha(128 * 256 + 128), 255, "centro transparente");
    }
}
