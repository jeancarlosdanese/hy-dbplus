# @file: docs/features/FEAT-014-empacotamento-macos/50-dev/impl-notes.md

# Notas de Implementação — FEAT-014 empacotamento-macos

## Estado atual

_Atualizado em 23/10/2026._

- **Feito:** docs, `packaging/` (ícone e logomarca aprovados) e a linha do M8 no BRIEF.
- **Falta:** `xtask bundle` (build, iconset, `Info.plist`, árvore, `codesign`, `hdiutil`, notarização
  opcional); testes; `CLAUDE.md` ("Comandos").
- **Depende de:** nada no código; das ferramentas do macOS na máquina (todas presentes em 23/10).
- **Para retomar:** este arquivo, o brief, `xtask/src/main.rs` (tabela `COMMANDS`) e `layers.rs` como
  molde de comando com `cargo metadata`.

## Decisões

- **[2026-10-23] Tudo no `xtask`, sem `cargo-bundle`**: o BRIEF quer Rust only e sem dependência sem
  motivo; o `.app` é uma árvore de arquivos e um plist, e o resto são comandos do macOS que já existem.
- **[2026-10-23] Ad hoc agora, Developer ID depois** (decisão do Jean): não há certificado na máquina
  (`security find-identity` sem identidades). O comando já está pronto para o Developer ID e a
  notarização por variáveis de ambiente; nada muda no código quando o certificado chegar.
- **[2026-10-23] Ícone e logomarca** (aprovados em 23/10 depois de duas rodadas): a forma é a mesma do
  `hyberica-icon.svg` (o balão com o "h"); a cor é azul-petróleo mais fechado que o do PostgreSQL;
  fundo claro no ícone; nome `y-dbplus` (o balão é o "h"), como `y-vox` no hy-vox (o `hy-vox-preto.svg` escreve com hífen). O ícone não tem
  texto, então não depende de fonte; a logomarca depende da Inter e por isso vai também em PNG.
- **[2026-10-23] arm64 por padrão, `--universal` opcional**: a máquina do Jean é arm64; o universal
  dobra o tempo de build e precisa do target x86_64 instalado.
- **[2026-10-23] Versão do `cargo metadata`**: a do workspace (`0.1.0`); a tag é do Jean.

## Plano

1. `xtask/src/bundle.rs`: `plist(version)`, `iconset_sizes()`, `layout(app_dir, binary, icns)` puros
   e testados; `run_bundle(args)` encadeia build → iconset → layout → codesign → dmg → notarização.
   `COMMANDS` ganha `bundle`. Reviewer (xtask é sensível). Commit `feat(xtask)`.
2. `CLAUDE.md` "Comandos" (o BRIEF M8 já cita a FEAT).

## Riscos e lições a conferir

- `qlmanage -t -s N` grava `<nome>.svg.png` na pasta de saída e, para SVG quadrado, dá N×N; conferir
  que não acrescenta fundo preto (no teste de 23/10 o fundo ficou transparente fora do quadrado
  arredondado).
- `codesign --deep` com ad hoc é aceito; `--options runtime` exige identidade real (ad hoc com
  hardened runtime bloqueia JIT, inofensivo aqui, mas fica fora para não confundir).
- O `hdiutil create` com `-srcfolder` copia o link `Applications`; conferir que o link relativo
  sobrevive (`ln -s /Applications`).

## Log

### 23/10/2026 — Criação

- A pendência do Keychain (senha das conexões pedida a cada build, porque o binário sem assinatura
  estável muda de identidade) foi observada ao longo do desenvolvimento e decidida para o M8 na FEAT-003;
  não estava registrada nos docs dela. Fica registrada aqui.
- Jean pediu o instalador para fechar o dia; ícone a partir do `hyberica-icon.svg` e nome no padrão do
  `hy-vox-preto.svg`, em azul-petróleo; três variantes de nome e dois fundos apresentados, escolhido
  fundo claro e `y-dbplus`, azul mais escuro na segunda rodada. Docs e `packaging/` criados.
