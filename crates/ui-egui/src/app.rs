//! O app egui: no M0, uma janela vazia com o tema aplicado e um texto de boas-vindas.

use crate::theme;

/// Estado da interface. Vazio no M0; sessões, abas e painéis entram nos marcos seguintes.
#[derive(Default)]
pub struct App {}

impl App {
    /// Cria o app e aplica o tema ao contexto do egui.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        Self::default()
    }

    /// O conteúdo da janela, separado do `eframe::App` para ser testável com `egui_kittest` (M2).
    /// O `ui` raiz do eframe vem sem margem nem fundo; o `CentralPanel` põe o fundo do tema.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let tokens = theme::current(ui.ctx());
        egui::CentralPanel::default_margins().show(ui, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("hy-dbplus")
                        .size(22.0)
                        .color(tokens.text_muted),
                );
            });
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui(ui);
    }
}
