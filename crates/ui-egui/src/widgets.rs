//! Os componentes do sistema visual (FEAT-015, tela 17 do protótipo): botões, controle
//! segmentado, etiqueta e faixa de ambiente, faixas de aviso e de erro, estado vazio, carregando e
//! o diálogo modal. As telas usam estes em vez de montar cor, raio e altura à mão.

// As telas passam a usar os componentes nas fases B a E da FEAT-015; a fase F tira esta licença.
#![allow(dead_code)]

use egui::{Color32, CornerRadius, Response, RichText, Stroke, Ui};
use hy_dbplus_engine::Tag;

use crate::fonts;
use crate::theme::{self, Tokens, radius, size, space, text_size};

/// O papel de um botão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// A ação principal da tela ou do diálogo (uma só por vez).
    Primary,
    /// Ação comum, com borda.
    Normal,
    /// Ação secundária, sem borda nem fundo até o ponteiro passar.
    Ghost,
    /// Ação que apaga ou descarta.
    Danger,
}

/// Um botão com o ícone opcional na frente do texto. Altura fixa de controle.
pub fn button(ui: &mut Ui, kind: ButtonKind, icon: Option<&str>, label: &str) -> Response {
    let t = theme::current(ui.ctx());
    ui.add(styled_button(
        &t,
        fonts::strong(ui.ctx()),
        kind,
        icon,
        label,
    ))
}

/// [`button`] desabilitado com a dica do motivo.
pub fn button_enabled(
    ui: &mut Ui,
    enabled: bool,
    kind: ButtonKind,
    icon: Option<&str>,
    label: &str,
    disabled_hint: &str,
) -> Response {
    let t = theme::current(ui.ctx());
    ui.add_enabled(
        enabled,
        styled_button(&t, fonts::strong(ui.ctx()), kind, icon, label),
    )
    .on_disabled_hover_text(disabled_hint)
}

fn styled_button<'a>(
    t: &Tokens,
    strong: egui::FontFamily,
    kind: ButtonKind,
    icon: Option<&str>,
    label: &str,
) -> egui::Button<'a> {
    let (fill, stroke, ink) = match kind {
        ButtonKind::Primary => (t.accent, Stroke::new(1.0, t.accent), t.accent_ink),
        ButtonKind::Normal => (t.card, Stroke::new(1.0, t.border), t.text),
        ButtonKind::Ghost => (Color32::TRANSPARENT, Stroke::NONE, t.text_muted),
        ButtonKind::Danger => (t.danger, Stroke::new(1.0, t.danger), danger_ink(t)),
    };
    let text = match icon {
        Some(icon) => format!("{icon} {label}"),
        None => label.to_owned(),
    };
    let mut text = RichText::new(text).color(ink);
    if kind != ButtonKind::Ghost {
        text = text.family(strong);
    }
    egui::Button::new(text)
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(radius::CONTROL))
        .min_size(egui::vec2(size::CONTROL, size::CONTROL))
}

/// Botão só com ícone (quadrado), com a dica que diz o que ele faz.
pub fn icon_button(ui: &mut Ui, icon: &str, hint: &str) -> Response {
    let t = theme::current(ui.ctx());
    ui.add(
        egui::Button::new(
            RichText::new(icon)
                .size(text_size::HEADING)
                .color(t.text_muted),
        )
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(radius::CONTROL))
        .min_size(egui::vec2(size::CONTROL, size::CONTROL)),
    )
    .on_hover_text(hint)
    .with_accessible_label(hint)
}

/// O nome que o leitor de tela (e o teste) usa para um widget só com ícone.
trait AccessibleLabel {
    fn with_accessible_label(self, label: &str) -> Self;
}

impl AccessibleLabel for Response {
    fn with_accessible_label(self, label: &str) -> Self {
        let enabled = self.enabled();
        self.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
        self
    }
}

/// Texto sobre o vermelho: branco no claro, escuro no escuro (o vermelho do escuro é claro).
fn danger_ink(t: &Tokens) -> Color32 {
    match t.theme {
        egui::Theme::Light => Color32::WHITE,
        egui::Theme::Dark => Color32::from_rgb(0x2A, 0x0A, 0x0E),
    }
}

/// Controle segmentado: uma opção ligada por vez. Devolve `true` quando o usuário trocou.
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    let t = theme::current(ui.ctx());
    let mut changed = false;
    egui::Frame::new()
        .fill(t.hover)
        .corner_radius(CornerRadius::same(radius::CONTROL + 1))
        .inner_margin(2.0)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                for (option, label) in options {
                    let on = *value == *option;
                    let text = RichText::new(*label)
                        .size(text_size::SMALL - 1.0)
                        .family(fonts::strong(ui.ctx()))
                        .color(if on { t.text } else { t.text_muted });
                    let response = ui.add(
                        egui::Button::new(text)
                            .selected(on)
                            .fill(if on { t.card } else { Color32::TRANSPARENT })
                            .stroke(Stroke::NONE)
                            .corner_radius(CornerRadius::same(radius::CONTROL - 1))
                            .min_size(egui::vec2(28.0, size::CONTROL - 8.0)),
                    );
                    if response.clicked() && !on {
                        *value = *option;
                        changed = true;
                    }
                }
            });
        });
    changed
}

/// Etiqueta de ambiente: o nome em maiúsculas sobre a cor da tag.
pub fn env_tag(ui: &mut Ui, tag: Tag) -> Response {
    let text = RichText::new(tag.label().to_uppercase())
        .size(text_size::SMALL - 1.0)
        .family(fonts::strong(ui.ctx()))
        .color(Color32::WHITE);
    egui::Frame::new()
        .fill(theme::tag_color(tag))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(egui::Margin::symmetric(7, 1))
        .show(ui, |ui| ui.label(text))
        .response
}

/// A faixa fina de ambiente, na largura inteira do `ui`, sob a barra superior.
pub fn env_stripe(ui: &mut Ui, tag: Tag) {
    let width = ui.available_width();
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, size::ENV_STRIPE), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme::tag_color(tag));
}

/// O tom de uma faixa de mensagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerKind {
    Warning,
    Error,
}

/// Faixa de aviso ou de erro na largura do `ui`, com uma ação opcional à direita. Devolve `true`
/// quando a ação foi clicada.
pub fn banner(ui: &mut Ui, kind: BannerKind, text: &str, action: Option<&str>) -> bool {
    let t = theme::current(ui.ctx());
    let (fill, ink, icon) = match kind {
        BannerKind::Warning => (t.warn_soft, t.warn, fonts::icon::WARNING),
        BannerKind::Error => (t.danger_soft, t.danger, fonts::icon::ERROR),
    };
    let mut clicked = false;
    egui::Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(radius::CONTROL))
        .inner_margin(egui::Margin::symmetric(space::M as i8, space::S as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).color(ink));
                ui.add(egui::Label::new(RichText::new(text).color(t.text)).wrap());
                if let Some(action) = action {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        clicked = button(ui, ButtonKind::Normal, None, action).clicked();
                    });
                }
            });
        });
    clicked
}

/// Estado vazio no centro do `ui`: ícone, título e a dica do que fazer.
pub fn empty_state(ui: &mut Ui, icon: &str, title: &str, hint: &str) {
    let t = theme::current(ui.ctx());
    ui.vertical_centered(|ui| {
        ui.add_space(space::XL);
        ui.label(RichText::new(icon).size(32.0).color(t.text_faint));
        ui.add_space(space::S);
        ui.label(
            RichText::new(title)
                .size(text_size::HEADING)
                .family(fonts::strong(ui.ctx()))
                .color(t.text),
        );
        ui.add_space(space::XS);
        ui.label(RichText::new(hint).color(t.text_muted));
    });
}

/// Carregando: o indicador girando e o texto.
pub fn loading(ui: &mut Ui, text: &str) {
    let t = theme::current(ui.ctx());
    ui.horizontal(|ui| {
        ui.add(
            egui::Spinner::new()
                .size(text_size::BODY)
                .color(t.text_muted),
        );
        ui.label(RichText::new(text).color(t.text_muted));
    });
}

/// O que o usuário fez no diálogo neste frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogOutcome {
    /// Continua aberto.
    Open,
    /// Clicou na ação principal.
    Confirm,
    /// Cancelou: botão, Esc ou clique fora.
    Cancel,
}

/// A ação principal de um diálogo.
#[derive(Debug, Clone, Copy)]
pub struct DialogAction<'a> {
    pub label: &'a str,
    pub kind: ButtonKind,
    pub enabled: bool,
}

/// Diálogo modal centralizado com fundo escurecido: título, conteúdo e rodapé com Cancelar e a
/// ação principal à direita. Esc e clique fora cancelam.
pub fn dialog(
    ctx: &egui::Context,
    id: egui::Id,
    title: &str,
    cancel: &str,
    action: Option<DialogAction<'_>>,
    body: impl FnOnce(&mut Ui),
) -> DialogOutcome {
    dialog_full(
        ctx,
        DialogParts {
            id,
            title,
            subtitle: None,
            width: 380.0,
            cancel,
            action,
        },
        body,
        |_| {},
    )
}

/// O que um diálogo mostra fora do conteúdo.
#[derive(Debug, Clone, Copy)]
pub struct DialogParts<'a> {
    pub id: egui::Id,
    pub title: &'a str,
    /// Uma linha sob o título.
    pub subtitle: Option<&'a str>,
    /// Largura mínima do conteúdo.
    pub width: f32,
    pub cancel: &'a str,
    pub action: Option<DialogAction<'a>>,
}

/// [`dialog`] com subtítulo, largura e o lado esquerdo do rodapé (ex.: "Testar conexão").
pub fn dialog_full(
    ctx: &egui::Context,
    parts: DialogParts<'_>,
    body: impl FnOnce(&mut Ui),
    footer_left: impl FnOnce(&mut Ui),
) -> DialogOutcome {
    let DialogParts {
        id,
        title,
        subtitle,
        width,
        cancel,
        action,
    } = parts;
    let t = theme::current(ctx);
    let mut outcome = DialogOutcome::Open;
    let response = egui::Modal::new(id)
        .frame(
            egui::Frame::new()
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(radius::WINDOW))
                .inner_margin(space::XL as i8),
        )
        .show(ctx, |ui| {
            ui.set_min_width(width);
            ui.set_max_width(width);
            ui.label(
                RichText::new(title)
                    .size(text_size::HEADING)
                    .family(fonts::strong(ui.ctx()))
                    .color(t.text),
            );
            if let Some(subtitle) = subtitle {
                ui.label(RichText::new(subtitle).color(t.text_muted));
            }
            ui.add_space(space::M);
            body(ui);
            ui.add_space(space::L);
            // Os botões primeiro, da direita: o lado esquerdo fica com a largura que sobra e um
            // texto longo (erro do "Testar conexão") não passa por baixo deles.
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(action) = action {
                        let confirm = ui.add_enabled(
                            action.enabled,
                            styled_button(
                                &t,
                                fonts::strong(ui.ctx()),
                                action.kind,
                                None,
                                action.label,
                            ),
                        );
                        if confirm.clicked() {
                            outcome = DialogOutcome::Confirm;
                        }
                    }
                    if button(ui, ButtonKind::Normal, None, cancel).clicked() {
                        outcome = DialogOutcome::Cancel;
                    }
                    ui.with_layout(
                        egui::Layout::left_to_right(egui::Align::Center),
                        footer_left,
                    );
                });
            });
        });
    if outcome == DialogOutcome::Open && response.should_close() {
        outcome = DialogOutcome::Cancel;
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[derive(Default)]
    struct State {
        outcomes: Vec<DialogOutcome>,
    }

    fn harness() -> Harness<'static, State> {
        Harness::builder()
            .with_size(egui::vec2(800.0, 600.0))
            .build_ui_state(
                |ui, state: &mut State| {
                    let outcome = dialog(
                        ui.ctx(),
                        egui::Id::new("teste"),
                        "Remover conexão",
                        "Cancelar",
                        Some(DialogAction {
                            label: "Remover",
                            kind: ButtonKind::Danger,
                            enabled: true,
                        }),
                        |ui| {
                            ui.label("Corpo");
                        },
                    );
                    if outcome != DialogOutcome::Open {
                        state.outcomes.push(outcome);
                    }
                },
                State::default(),
            )
    }

    #[test]
    fn esc_cancela_o_dialogo() {
        let mut h = harness();
        h.run();
        h.key_press(egui::Key::Escape);
        h.run();
        assert_eq!(h.state().outcomes, vec![DialogOutcome::Cancel]);
    }

    #[test]
    fn clique_fora_cancela_o_dialogo() {
        let mut h = harness();
        h.run();
        let pos = egui::pos2(10.0, 10.0);
        h.hover_at(pos);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run();
        assert_eq!(h.state().outcomes, vec![DialogOutcome::Cancel]);
    }

    #[test]
    fn os_botoes_confirmam_e_cancelam_e_clique_dentro_nao_fecha() {
        let mut h = harness();
        h.run();
        h.get_by_label("Corpo").click();
        h.run();
        assert!(h.state().outcomes.is_empty());
        h.get_by_label("Remover").click();
        h.run();
        assert_eq!(h.state().outcomes, vec![DialogOutcome::Confirm]);
        h.get_by_label("Cancelar").click();
        h.run();
        assert_eq!(
            h.state().outcomes,
            vec![DialogOutcome::Confirm, DialogOutcome::Cancel]
        );
    }

    #[test]
    fn segmentado_troca_a_opcao_e_avisa_so_na_troca() {
        #[derive(Clone, Copy, PartialEq, Debug)]
        enum Opt {
            A,
            B,
        }
        let mut h = Harness::new_ui_state(
            |ui, state: &mut (Opt, u32)| {
                if segmented(ui, &mut state.0, &[(Opt::A, "PT"), (Opt::B, "EN")]) {
                    state.1 += 1;
                }
            },
            (Opt::A, 0),
        );
        h.run();
        h.get_by_label("PT").click();
        h.run();
        assert_eq!(*h.state(), (Opt::A, 0));
        h.get_by_label("EN").click();
        h.run();
        assert_eq!(*h.state(), (Opt::B, 1));
    }

    #[test]
    fn faixa_devolve_o_clique_da_acao() {
        let mut h = Harness::new_ui_state(
            |ui, clicks: &mut u32| {
                if banner(ui, BannerKind::Error, "Falhou", Some("Tentar de novo")) {
                    *clicks += 1;
                }
            },
            0,
        );
        h.run();
        h.get_by_label("Tentar de novo").click();
        h.run();
        assert_eq!(*h.state(), 1);
    }
}
