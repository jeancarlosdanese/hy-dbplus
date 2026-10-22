//! O que os grids de leitura (Structure, Index) têm em comum: altura da linha, cabeçalho e célula.

use crate::theme::Tokens;

pub const ROW_HEIGHT: f32 = 22.0;

pub fn header(ui: &mut egui::Ui, text: &str, tokens: &Tokens) {
    ui.label(
        egui::RichText::new(text)
            .small()
            .strong()
            .color(tokens.text_muted),
    );
}

/// Texto monoespaçado sem seleção (o clique fica livre para a linha).
pub fn cell(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(egui::RichText::new(text).monospace()).selectable(false));
}
