//! Tema do hy-dbplus: claro e escuro, identidade Hyberica (azul-petróleo), seguindo o sistema.
//!
//! Os tokens são dados puros (testáveis sem janela), indexados por `egui::Theme`; `apply`
//! entrega ao egui as duas variantes e deixa a preferência em "sistema", para a janela
//! acompanhar o macOS. Os valores vêm do protótipo aprovado da FEAT-015
//! (`docs/features/FEAT-015-redesenho-ux/20-prototipos/prototipo.html`, tela 17).

use egui::{Color32, CornerRadius, Stroke, Theme, Visuals};

/// Escala de espaçamento (px): o que separa elementos sai daqui, nunca de número solto.
pub mod space {
    pub const XS: f32 = 4.0;
    pub const S: f32 = 8.0;
    pub const M: f32 = 12.0;
    pub const L: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

/// Escala de tipo (px): título de tela, título de seção, corpo e detalhe.
pub mod text_size {
    pub const TITLE: f32 = 20.0;
    pub const HEADING: f32 = 16.0;
    pub const BODY: f32 = 13.0;
    pub const SMALL: f32 = 12.0;
}

/// Alturas fixas: linha do grid, barras (superior, de abas, de ferramentas), rodapé e controles.
pub mod size {
    pub const ROW: f32 = 24.0;
    pub const BAR: f32 = 40.0;
    pub const FOOTER: f32 = 32.0;
    pub const CONTROL: f32 = 28.0;
    /// A faixa de ambiente sob a barra superior.
    pub const ENV_STRIPE: f32 = 3.0;
}

/// Raios dos cantos: controle, cartão e janela (diálogo, paleta).
pub mod radius {
    pub const CONTROL: u8 = 6;
    pub const CARD: u8 = 8;
    pub const WINDOW: u8 = 12;
}

/// As cores de uma variante. Nomes pelo papel, não pela cor: a paleta muda, o papel fica.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
    pub theme: Theme,
    /// Fundo da janela, atrás dos painéis.
    pub bg: Color32,
    /// Fundo dos painéis.
    pub panel: Color32,
    /// Fundo de cartão, popup e janela flutuante.
    pub card: Color32,
    /// Fundo de campo de texto e do grid.
    pub field: Color32,
    /// Texto principal, secundário e terciário (dica, contagem, placeholder).
    pub text: Color32,
    pub text_muted: Color32,
    pub text_faint: Color32,
    /// Borda de cartão e divisor entre painéis.
    pub border: Color32,
    pub divider: Color32,
    /// Fundo sob o ponteiro e ao pressionar.
    pub hover: Color32,
    pub pressed: Color32,
    /// Linha alternada do grid.
    pub stripe: Color32,
    /// Cor de destaque (seleção, link, foco, botão principal), o texto sobre ela e a versão
    /// suave para fundo de seleção.
    pub accent: Color32,
    pub accent_ink: Color32,
    pub accent_soft: Color32,
    /// Edição pendente (célula alterada, linha nova): fundo e texto. Nunca a cor da seleção.
    pub pending: Color32,
    pub pending_ink: Color32,
    /// Erro e ação destrutiva, e o fundo da faixa de erro.
    pub danger: Color32,
    pub danger_soft: Color32,
    /// Sucesso (conectado, gravado).
    pub ok: Color32,
    /// Aviso (transação aberta, resultado truncado) e o fundo da faixa de aviso.
    pub warn: Color32,
    pub warn_soft: Color32,
    /// Realce do SQL: palavra-chave, texto, número e comentário.
    pub syntax_keyword: Color32,
    pub syntax_string: Color32,
    pub syntax_number: Color32,
    pub syntax_comment: Color32,
    /// Raio dos cantos dos widgets.
    pub radius: u8,
}

impl Tokens {
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self {
                theme,
                bg: rgb(0xE9EEEE),
                panel: rgb(0xF5F8F8),
                card: Color32::WHITE,
                field: Color32::WHITE,
                text: rgb(0x12262B),
                text_muted: rgb(0x4D6368),
                text_faint: rgb(0x87999D),
                border: rgb(0xD8E0E2),
                divider: rgb(0xD8E0E2),
                hover: rgb(0xE4EBEC),
                pressed: rgb(0xD6E0E2),
                stripe: rgb(0xF8FAFA),
                accent: rgb(0x1C5F72),
                accent_ink: Color32::WHITE,
                accent_soft: rgb(0xD3E7EC),
                pending: rgb(0xFBEBC2),
                pending_ink: rgb(0x7A5200),
                danger: rgb(0xC4313F),
                danger_soft: rgb(0xF9DEE0),
                ok: rgb(0x22805A),
                warn: rgb(0xA96B08),
                warn_soft: rgb(0xFBEBC2),
                syntax_keyword: rgb(0x1C5F72),
                syntax_string: rgb(0x22805A),
                syntax_number: rgb(0xA9520F),
                syntax_comment: rgb(0x87999D),
                radius: radius::CONTROL,
            },
            Theme::Dark => Self {
                theme,
                bg: rgb(0x081215),
                panel: rgb(0x0E1A1E),
                card: rgb(0x142329),
                field: rgb(0x0A1518),
                text: rgb(0xE3ECEE),
                text_muted: rgb(0x9DB0B5),
                text_faint: rgb(0x667D83),
                border: rgb(0x21353B),
                divider: rgb(0x21353B),
                hover: rgb(0x1A2D33),
                pressed: rgb(0x223940),
                stripe: rgb(0x111F24),
                accent: rgb(0x5DBBD0),
                accent_ink: rgb(0x06222A),
                accent_soft: rgb(0x173F4A),
                pending: rgb(0x4A3A10),
                pending_ink: rgb(0xF3CE6B),
                danger: rgb(0xF0707B),
                danger_soft: rgb(0x47202A),
                ok: rgb(0x5CC797),
                warn: rgb(0xE8B04A),
                warn_soft: rgb(0x4A3A10),
                syntax_keyword: rgb(0x5DBBD0),
                syntax_string: rgb(0x7FD3A6),
                syntax_number: rgb(0xE9A869),
                syntax_comment: rgb(0x667D83),
                radius: radius::CONTROL,
            },
        }
    }
}

/// Cor a partir do hexadecimal `0xRRGGBB`, como no protótipo.
const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// Os `Visuals` do egui para uma variante, a partir dos tokens.
pub fn visuals(theme: Theme) -> Visuals {
    let t = Tokens::for_theme(theme);
    let mut v = theme.default_visuals();
    v.panel_fill = t.panel;
    v.window_fill = t.card;
    v.window_stroke = Stroke::new(1.0, t.border);
    v.extreme_bg_color = t.field;
    // Fundo das linhas alternadas da `Table` do egui_extras.
    v.faint_bg_color = t.stripe;
    v.error_fg_color = t.danger;
    v.warn_fg_color = t.warn;
    v.selection.bg_fill = t.accent_soft;
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v.hyperlink_color = t.accent;
    v.override_text_color = Some(t.text);
    v.window_corner_radius = CornerRadius::same(radius::WINDOW);
    v.menu_corner_radius = CornerRadius::same(radius::CARD);
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

/// Entrega ao egui as duas variantes e segue o tema do sistema; a escala de tipo e o espaçamento
/// valem para as duas.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::System);
    ctx.set_visuals_of(Theme::Light, visuals(Theme::Light));
    ctx.set_visuals_of(Theme::Dark, visuals(Theme::Dark));
    ctx.all_styles_mut(|style| {
        use egui::{FontFamily, FontId, TextStyle};
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(text_size::HEADING, FontFamily::Proportional),
            ),
            (
                TextStyle::Body,
                FontId::new(text_size::BODY, FontFamily::Proportional),
            ),
            (
                TextStyle::Button,
                FontId::new(text_size::BODY, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(text_size::SMALL - 1.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(text_size::SMALL, FontFamily::Monospace),
            ),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(space::S, space::XS + 2.0);
        style.spacing.button_padding = egui::vec2(space::S + 2.0, space::XS);
        style.spacing.interact_size.y = size::CONTROL - space::XS;
        style.spacing.scroll = scroll_style();
    });
}

/// Barra de rolagem sempre visível e com espaço próprio: a flutuante do egui só aparece com o
/// ponteiro em cima, e a lista parecia não ter fim (pedido do Jean, 17/11/2026).
pub fn scroll_style() -> egui::style::ScrollStyle {
    egui::style::ScrollStyle {
        bar_width: 8.0,
        // A alça na cor do texto (com transparência): com a cor de fundo dos widgets, que no tema
        // claro é a do campo (branca), ela sumia sobre o painel.
        foreground_color: true,
        dormant_handle_opacity: 0.35,
        dormant_background_opacity: 0.15,
        active_handle_opacity: 0.8,
        ..egui::style::ScrollStyle::solid()
    }
}

/// Os tokens da variante que o egui está mostrando agora. Sem sinal do sistema (Linux em X11,
/// Wayland sem portal) o egui cai no escuro; no macOS o winit informa a aparência e as mudanças.
pub fn current(ctx: &egui::Context) -> Tokens {
    Tokens::for_theme(ctx.theme())
}

/// Cor da tag de ambiente (etiqueta e faixa): verde local, violeta dev, âmbar staging, vermelho
/// prod. Igual nas duas variantes: é sinal, não decoração; nenhuma é a cor de destaque.
pub fn tag_color(tag: hy_dbplus_engine::Tag) -> Color32 {
    match tag {
        hy_dbplus_engine::Tag::Local => rgb(0x2B8A5F),
        hy_dbplus_engine::Tag::Dev => rgb(0x6A5AD1),
        hy_dbplus_engine::Tag::Staging => rgb(0xC98112),
        hy_dbplus_engine::Tag::Prod => rgb(0xD2453B),
    }
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

    /// Contraste WCAG entre duas cores opacas.
    fn contrast(a: Color32, b: Color32) -> f32 {
        fn lum(c: Color32) -> f32 {
            let ch = |v: u8| {
                let v = f32::from(v) / 255.0;
                if v <= 0.039_28 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
        }
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn pendencia_nao_se_confunde_com_selecao_nem_com_hover() {
        for theme in [Theme::Light, Theme::Dark] {
            let t = Tokens::for_theme(theme);
            assert_ne!(t.pending, t.accent_soft, "{theme:?}");
            assert_ne!(t.pending, t.hover, "{theme:?}");
            assert_ne!(t.pending, t.stripe, "{theme:?}");
        }
    }

    #[test]
    fn ambientes_sao_distintos_entre_si_e_do_destaque_e_do_erro() {
        use hy_dbplus_engine::Tag;
        let tags = [Tag::Local, Tag::Dev, Tag::Staging, Tag::Prod].map(tag_color);
        for (i, a) in tags.iter().enumerate() {
            for b in tags.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
            for theme in [Theme::Light, Theme::Dark] {
                let t = Tokens::for_theme(theme);
                assert_ne!(*a, t.accent, "{theme:?}");
                assert_ne!(*a, t.danger, "{theme:?}");
            }
            // A etiqueta leva texto branco.
            assert!(contrast(*a, Color32::WHITE) >= 3.0, "{a:?}");
        }
    }

    #[test]
    fn textos_tem_contraste_legivel_nos_dois_temas() {
        for theme in [Theme::Light, Theme::Dark] {
            let t = Tokens::for_theme(theme);
            for bg in [t.panel, t.card, t.field] {
                assert!(contrast(t.text, bg) >= 7.0, "{theme:?} texto");
                assert!(
                    contrast(t.text_muted, bg) >= 4.5,
                    "{theme:?} texto secundário"
                );
                assert!(contrast(t.danger, bg) >= 4.5, "{theme:?} erro");
            }
            assert!(
                contrast(t.accent_ink, t.accent) >= 4.5,
                "{theme:?} botão principal"
            );
            assert!(
                contrast(t.pending_ink, t.pending) >= 4.5,
                "{theme:?} pendência"
            );
            assert!(contrast(t.text, t.accent_soft) >= 4.5, "{theme:?} seleção");
        }
    }

    #[test]
    fn a_cor_de_erro_das_telas_segue_o_tema_e_e_legivel() {
        let ctx = egui::Context::default();
        apply(&ctx);
        for (pref, theme) in [
            (egui::ThemePreference::Light, Theme::Light),
            (egui::ThemePreference::Dark, Theme::Dark),
        ] {
            ctx.set_theme(pref);
            let t = Tokens::for_theme(theme);
            assert_eq!(current(&ctx).danger, t.danger);
            for bg in [t.panel, t.card, t.field] {
                assert!(contrast(t.danger, bg) >= 4.5, "{theme:?}");
            }
        }
    }

    #[test]
    fn a_barra_de_rolagem_fica_visivel_e_com_espaco_proprio() {
        let ctx = egui::Context::default();
        apply(&ctx);
        for theme in [Theme::Light, Theme::Dark] {
            let scroll = ctx.style_of(theme).spacing.scroll;
            assert!(!scroll.floating, "{theme:?}");
            assert!(scroll.dormant_handle_opacity > 0.0, "{theme:?}");
            assert!(scroll.allocated_width() > 0.0, "{theme:?}");
            assert!(scroll.foreground_color, "{theme:?}: alça na cor do texto");
        }
    }
}
