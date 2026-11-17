<!-- FEAT_EM_ANDAMENTO — fase A (fundação) em execução -->
# @file: docs/features/FEAT-015-redesenho-ux/context.index.md

# FEAT-015: redesenho de UX de todas as telas

- Brief: `00-brief/brief.md`
- Aceite: `10-prd/acceptance.md`
- Protótipo: `20-prototipos/README.md` (abrir `20-prototipos/prototipo.html` no navegador)
- Plano e notas: `50-dev/impl-notes.md`

## Status da implementação

Protótipo, aceite e plano aprovados pelo Jean em 17/11/2026. **Fase A (fundação) implementada** e sem commit:
tokens novos em `crates/ui-egui/src/theme.rs`, fontes e ícones em `fonts.rs`, componentes em `widgets.rs`, textos
em `i18n.rs`, preferência de idioma em `crates/engine/src/preferences.rs` e o seletor PT | EN nas conexões e na
barra superior. As telas ainda usam o desenho antigo, só com a paleta e as fontes novas; elas mudam nas fases B a
E. Próximo: revisão da fase A, "pode commitar" e o detalhamento da fase B (conexões).

**Cabe ao mantenedor:** conferência visual no app ao fim de cada fase (nesta: nitidez da Inter, cores nos dois
temas e o seletor PT | EN).
