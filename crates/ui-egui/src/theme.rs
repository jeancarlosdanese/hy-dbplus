//! Tema do hy-dbplus: claro e escuro, cores inspiradas no TablePlus, seguindo o sistema.
//!
//! Os tokens são dados puros (testáveis sem janela), indexados por `egui::Theme`; `apply`
//! entrega ao egui as duas variantes e deixa a preferência em "sistema", para a janela
//! acompanhar o macOS.

use egui::{Color32, CornerRadius, Stroke, Theme, Visuals};

/// As cores de uma variante. Nomes pelo papel, não pela cor: a paleta muda, o papel fica.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
    pub theme: Theme,
    /// Fundo dos painéis.
    pub panel: Color32,
    /// Fundo de cartão, popup e janela flutuante.
    pub card: Color32,
    /// Fundo de campo de texto e do grid.
    pub field: Color32,
    /// Texto principal e secundário.
    pub text: Color32,
    pub text_muted: Color32,
    /// Borda de cartão e divisor entre painéis.
    pub border: Color32,
    pub divider: Color32,
    /// Fundo sob o ponteiro e ao pressionar.
    pub hover: Color32,
    pub pressed: Color32,
    /// Cor de destaque (seleção, link, foco) e sua versão suave para fundo de seleção.
    pub accent: Color32,
    pub accent_soft: Color32,
    /// Raio dos cantos dos widgets.
    pub radius: u8,
}

impl Tokens {
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self {
                theme,
                panel: Color32::from_rgb(0xF4, 0xF4, 0xF6),
                card: Color32::WHITE,
                field: Color32::WHITE,
                text: Color32::from_rgb(0x1D, 0x1D, 0x1F),
                text_muted: Color32::from_rgb(0x6E, 0x6E, 0x73),
                border: Color32::from_rgb(0xD9, 0xD9, 0xDE),
                divider: Color32::from_rgb(0xE5, 0xE5, 0xEA),
                hover: Color32::from_rgb(0xE8, 0xE8, 0xED),
                pressed: Color32::from_rgb(0xDC, 0xDC, 0xE2),
                accent: Color32::from_rgb(0x2F, 0x6F, 0xED),
                accent_soft: Color32::from_rgb(0xD6, 0xE4, 0xFB),
                radius: 6,
            },
            Theme::Dark => Self {
                theme,
                panel: Color32::from_rgb(0x1E, 0x1E, 0x22),
                card: Color32::from_rgb(0x28, 0x28, 0x2D),
                field: Color32::from_rgb(0x16, 0x16, 0x1A),
                text: Color32::from_rgb(0xE6, 0xE6, 0xEA),
                text_muted: Color32::from_rgb(0x9A, 0x9A, 0xA3),
                border: Color32::from_rgb(0x3A, 0x3A, 0x42),
                divider: Color32::from_rgb(0x32, 0x32, 0x39),
                hover: Color32::from_rgb(0x33, 0x33, 0x3A),
                pressed: Color32::from_rgb(0x3E, 0x3E, 0x47),
                accent: Color32::from_rgb(0x4C, 0x8D, 0xFF),
                accent_soft: Color32::from_rgb(0x24, 0x3A, 0x5E),
                radius: 6,
            },
        }
    }
}

/// Os `Visuals` do egui para uma variante, a partir dos tokens.
pub fn visuals(theme: Theme) -> Visuals {
    let t = Tokens::for_theme(theme);
    let mut v = theme.default_visuals();
    v.panel_fill = t.panel;
    v.window_fill = t.card;
    v.window_stroke = Stroke::new(1.0, t.border);
    v.extreme_bg_color = t.field;
    v.faint_bg_color = t.hover;
    v.selection.bg_fill = t.accent_soft;
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v.hyperlink_color = t.accent;
    v.override_text_color = Some(t.text);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = CornerRadius::same(t.radius);
        w.fg_stroke.color = t.text;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.divider);
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_fill = t.field;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.hovered.weak_bg_fill = t.hover;
    v.widgets.hovered.bg_fill = t.hover;
    v.widgets.hovered.bg_stroke = Stroke::NONE;
    v.widgets.active.weak_bg_fill = t.pressed;
    v.widgets.active.bg_fill = t.pressed;
    v.widgets.open.weak_bg_fill = t.hover;
    v
}

/// Entrega ao egui as duas variantes e segue o tema do sistema.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::System);
    ctx.set_visuals_of(Theme::Light, visuals(Theme::Light));
    ctx.set_visuals_of(Theme::Dark, visuals(Theme::Dark));
}

/// Os tokens da variante que o egui está mostrando agora. Sem sinal do sistema (Linux em X11,
/// Wayland sem portal) o egui cai no escuro; no macOS o winit informa a aparência e as mudanças.
pub fn current(ctx: &egui::Context) -> Tokens {
    Tokens::for_theme(ctx.theme())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_variantes_tem_fundo_e_texto_proprios() {
        let light = Tokens::for_theme(Theme::Light);
        let dark = Tokens::for_theme(Theme::Dark);
        assert_ne!(light.panel, dark.panel);
        assert_ne!(light.text, dark.text);
        assert_eq!((light.theme, dark.theme), (Theme::Light, Theme::Dark));
    }

    #[test]
    fn os_visuals_seguem_a_variante_e_os_tokens() {
        for theme in [Theme::Light, Theme::Dark] {
            let v = visuals(theme);
            let t = Tokens::for_theme(theme);
            assert_eq!(v.dark_mode, theme == Theme::Dark);
            assert_eq!(v.panel_fill, t.panel);
            assert_eq!(v.override_text_color, Some(t.text));
            assert_eq!(v.selection.stroke.color, t.accent);
        }
    }

    #[test]
    fn apply_segue_o_sistema_e_registra_as_duas_variantes() {
        let ctx = egui::Context::default();
        apply(&ctx);
        assert_eq!(
            ctx.options(|o| o.theme_preference),
            egui::ThemePreference::System
        );
        assert_eq!(
            ctx.style_of(Theme::Light).visuals.panel_fill,
            Tokens::for_theme(Theme::Light).panel
        );
        assert_eq!(
            ctx.style_of(Theme::Dark).visuals.panel_fill,
            Tokens::for_theme(Theme::Dark).panel
        );
        assert_eq!(current(&ctx).theme, ctx.theme());
    }
}
