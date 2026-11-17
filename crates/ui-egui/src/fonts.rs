//! Fontes e ícones do app (FEAT-015): Inter na interface, JetBrains Mono em dados e SQL, ambas
//! embutidas no binário (SIL OFL 1.1, licenças em `assets/fonts/`), e os ícones Phosphor (MIT)
//! cortados em tempo de compilação só para os que o app usa. As fontes padrão do egui ficam como
//! reserva, para símbolos que a Inter não tem.

use egui::{FontData, FontDefinitions, FontFamily};

const INTER: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
const MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");

const INTER_NAME: &str = "inter";
const INTER_SEMIBOLD_NAME: &str = "inter-semibold";
const MONO_NAME: &str = "jetbrains-mono";
/// A família do texto forte (título, nome de conexão, aba ativa).
const STRONG_FAMILY: &str = "strong";

egui_phosphor::subset! {
    /// Os ícones do app: só estes vão para o binário. Ícone novo entra aqui.
    pub mod icons {
        use regular::{
            ARROW_CLOCKWISE, ARROW_COUNTER_CLOCKWISE, CARET_DOWN, CARET_LEFT, CARET_RIGHT, CARET_UP,
            CHECK, CLOCK, CODE, COPY, DATABASE, EYE, FAST_FORWARD, FUNCTION, KEY, LINK,
            MAGIC_WAND, MAGNIFYING_GLASS, PENCIL_SIMPLE, PLAY, PLUGS, PLUGS_CONNECTED, PLUS,
            SIDEBAR_SIMPLE, STOP, TABLE, TRASH, WARNING, WARNING_CIRCLE, X,
        };
    }
}

/// O ícone de cada papel na interface. A tela pede o papel; a tabela decide o desenho.
// As telas passam a usar os ícones nas fases B a E da FEAT-015; a fase F tira esta licença.
#[allow(dead_code)]
pub mod icon {
    use super::icons::regular as r;

    pub const DATABASE: &str = r::DATABASE;
    pub const TABLE: &str = r::TABLE;
    pub const VIEW: &str = r::EYE;
    pub const FUNCTION: &str = r::FUNCTION;
    pub const KEY: &str = r::KEY;
    pub const FOREIGN_KEY: &str = r::LINK;
    pub const SEARCH: &str = r::MAGNIFYING_GLASS;
    pub const ADD: &str = r::PLUS;
    pub const CLOSE: &str = r::X;
    pub const EXPAND: &str = r::CARET_RIGHT;
    pub const COLLAPSE: &str = r::CARET_DOWN;
    pub const PREVIOUS: &str = r::CARET_LEFT;
    pub const NEXT: &str = r::CARET_RIGHT;
    pub const UP: &str = r::CARET_UP;
    pub const RUN: &str = r::PLAY;
    pub const RUN_ALL: &str = r::FAST_FORWARD;
    pub const STOP: &str = r::STOP;
    pub const REFRESH: &str = r::ARROW_CLOCKWISE;
    pub const UNDO: &str = r::ARROW_COUNTER_CLOCKWISE;
    pub const PANEL: &str = r::SIDEBAR_SIMPLE;
    pub const CONNECT: &str = r::PLUGS_CONNECTED;
    pub const DISCONNECT: &str = r::PLUGS;
    pub const EDIT: &str = r::PENCIL_SIMPLE;
    pub const DELETE: &str = r::TRASH;
    pub const COPY: &str = r::COPY;
    pub const SQL: &str = r::CODE;
    pub const FORMAT: &str = r::MAGIC_WAND;
    pub const WARNING: &str = r::WARNING;
    pub const ERROR: &str = r::WARNING_CIRCLE;
    pub const OK: &str = r::CHECK;
    pub const HISTORY: &str = r::CLOCK;
}

// O corte dos ícones precisa ficar muito abaixo da fonte inteira (~490 KB): conferido na compilação.
const _: () = assert!(icons::regular::FONT_LEN < 60_000);

/// As definições de fonte do app: Inter e JetBrains Mono na frente das fontes padrão do egui,
/// os ícones logo depois da Inter e a família `strong` com a Inter SemiBold.
pub fn definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    for (name, bytes) in [
        (INTER_NAME, INTER),
        (INTER_SEMIBOLD_NAME, INTER_SEMIBOLD),
        (MONO_NAME, MONO),
    ] {
        fonts
            .font_data
            .insert(name.to_owned(), FontData::from_static(bytes).into());
    }
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, INTER_NAME.to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, MONO_NAME.to_owned());
    // Ícones como reserva da Proportional (segunda posição: não cobrem a Inter).
    icons::regular::add_to_fonts(&mut fonts);
    let mut strong = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    strong.insert(0, INTER_SEMIBOLD_NAME.to_owned());
    fonts
        .families
        .insert(FontFamily::Name(STRONG_FAMILY.into()), strong);
    fonts
}

/// Registra as fontes no contexto (vale a partir do próximo frame).
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(definitions());
}

/// A família do texto forte: Inter SemiBold, com as mesmas reservas da Proportional. Sem as
/// fontes do app instaladas (harness de teste) vale a Proportional: família nomeada que não
/// existe derruba o egui. Só dentro de um frame: antes do primeiro `Context::run` o egui ainda não
/// tem fontes e `ctx.fonts` entra em pânico.
pub fn strong(ctx: &egui::Context) -> FontFamily {
    let family = FontFamily::Name(STRONG_FAMILY.into());
    if ctx.fonts(|f| f.definitions().families.contains_key(&family)) {
        family
    } else {
        FontFamily::Proportional
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inter_e_mono_vem_primeiro_e_os_icones_logo_depois() {
        let fonts = definitions();
        let prop = &fonts.families[&FontFamily::Proportional];
        assert_eq!(prop.first().map(String::as_str), Some(INTER_NAME));
        assert_eq!(
            prop.get(1).map(String::as_str),
            Some(icons::regular::FONT_NAME)
        );
        let mono = &fonts.families[&FontFamily::Monospace];
        assert_eq!(mono.first().map(String::as_str), Some(MONO_NAME));
        let strong = &fonts.families[&FontFamily::Name(STRONG_FAMILY.into())];
        assert_eq!(
            strong.first().map(String::as_str),
            Some(INTER_SEMIBOLD_NAME)
        );
        // As fontes padrão do egui continuam como reserva.
        assert!(prop.len() > 2);
    }

    #[test]
    fn os_icones_e_os_textos_tem_glifo() {
        let ctx = egui::Context::default();
        install(&ctx);
        // As fontes entram no começo do frame; as texturas do frame não vão a GPU nenhuma.
        let mut output = ctx.run_ui(Default::default(), |_| {});
        output.textures_delta.clear();
        ctx.fonts_mut(|f| {
            let body = egui::FontId::proportional(13.0);
            for ch in [
                icon::DATABASE,
                icon::TABLE,
                icon::RUN,
                icon::WARNING,
                "ç",
                "ã",
                "⌘",
            ]
            .iter()
            .flat_map(|s| s.chars())
            {
                assert!(f.has_glyph(&body, ch), "sem glifo para {ch:?}");
            }
            assert!(f.has_glyph(&egui::FontId::monospace(12.0), 'λ'));
        });
    }

    #[test]
    fn texto_forte_sem_as_fontes_instaladas_cai_na_proportional() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(Default::default(), |_| {});
        output.textures_delta.clear();
        assert_eq!(strong(&ctx), FontFamily::Proportional);
        install(&ctx);
        let mut output = ctx.run_ui(Default::default(), |_| {});
        output.textures_delta.clear();
        assert_eq!(strong(&ctx), FontFamily::Name(STRONG_FAMILY.into()));
    }
}
