//! O que os grids de leitura (Estrutura, Índices) têm em comum: altura da linha, cabeçalho, célula,
//! título de seção e o rodapé com a contagem.

use crate::theme::{self, Tokens};

pub const ROW_HEIGHT: f32 = 22.0;
/// A coluna "#" do grid de dados e do de resultados: a mesma largura nos dois.
pub const NUMBER_WIDTH: f32 = 48.0;

pub fn header(ui: &mut egui::Ui, text: &str, tokens: &Tokens) {
    ui.label(
        egui::RichText::new(text)
            .size(theme::text_size::SMALL)
            .family(crate::fonts::strong(ui.ctx()))
            .color(tokens.text_muted),
    );
}

/// Texto monoespaçado sem seleção (o clique fica livre para a linha).
pub fn cell(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(egui::RichText::new(text).monospace()).selectable(false));
}

/// Texto de interface (sim/não, tipo de restrição) na célula, apagado.
pub fn cell_muted(ui: &mut egui::Ui, text: &str, tokens: &Tokens) {
    ui.add(egui::Label::new(egui::RichText::new(text).color(tokens.text_muted)).selectable(false));
}

/// O título de uma seção ("Colunas", "Restrições").
pub fn section(ui: &mut egui::Ui, text: &str, tokens: &Tokens) {
    ui.add_space(theme::space::M);
    ui.label(
        egui::RichText::new(text)
            .size(theme::text_size::SMALL)
            .family(crate::fonts::strong(ui.ctx()))
            .color(tokens.text_faint),
    );
    ui.add_space(theme::space::XS);
}

/// O rodapé com a contagem ("6 colunas · 3 restrições").
pub fn status_footer(ui: &mut egui::Ui, id: &'static str, text: &str) {
    let tokens = theme::current(ui.ctx());
    egui::Panel::bottom(id)
        .exact_size(theme::size::FOOTER)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(tokens.panel)
                .inner_margin(egui::Margin::symmetric(12, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(
                    egui::RichText::new(text)
                        .size(theme::text_size::SMALL)
                        .color(tokens.text_muted),
                );
            });
        });
}
