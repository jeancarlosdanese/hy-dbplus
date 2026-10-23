# @file: docs/features/FEAT-014-empacotamento-macos/50-dev/impl-notes.md

# Notas de Implementação — FEAT-014 empacotamento-macos

## Estado atual

_Atualizado em 23/10/2026._

- **Feito:** docs, `packaging/` e BRIEF (`da82b5a`); `xtask bundle` (`xtask/src/bundle.rs`: `parse_args`,
  `plist`, `iconset_entries`, `layout` testados; `run` encadeia build → iconset → `.app` → `codesign` →
  `.dmg` → notarização opcional), `CLAUDE.md` "Comandos". Rodado de ponta a ponta em 23/10: 64 s,
  `.app` assinado ad hoc (`codesign --verify` ok), `.dmg` UDZO de 5,9 MB, `.icns` com os dez tamanhos
  (`f82e8d4`). Ícone embutido no binário (`packaging/icon-256.png`, Dock no `cargo run` e por cima do
  cache) e ⌘M minimizando a janela, com testes: no commit seguinte.
- **Falta:** push e conferência do Jean (aceite "Do Jean").
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

1. `xtask/src/bundle.rs`: `parse_args`, `plist(version)`, `iconset_entries()`, `layout(bundle_dir,
   binary, icns, version)` puros e testados; `run(args, root, target_dir, version)` encadeia build →
   iconset → layout → codesign → dmg → notarização. `COMMANDS` ganha `bundle`. Reviewer (xtask é
   sensível). Commit `feat(xtask)`.
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

### 23/10/2026 — Conferência do Jean: ícone na Dock e ⌘M

- O `.app` instalado abriu com o tile preto genérico na Dock (cache da Dock: o diálogo do Keychain já
  mostrava o ícone certo). Além do `killall Dock`, o app passa a embutir o ícone
  (`packaging/icon-256.png` por `include_bytes!` e `with_icon` no `ViewportBuilder`): o eframe o põe na
  Dock ao abrir, o que vale no `cargo run` e passa por cima do cache.
- ⌘M não minimizava (o menu padrão do winit no macOS não traz o atalho): `App::ui` consome ⌘M e manda
  `ViewportCommand::Minimized(true)`. Teste kittest pelo `viewport_output`.

### 23/10/2026 — Revisão do `xtask bundle` (`process:reviewer`): aplicada

- [x] **Baixa.** `HY_DBPLUS_NOTARY_PROFILE` com `--no-dmg` era ignorada em silêncio: agora é erro.
- [x] **Baixa.** Diretório de build fixo em `root/target`: agora é o `target_directory` do `cargo metadata`
  (`CARGO_TARGET_DIR` vale). Teste.
- [x] **Baixa.** `cargo build` e `notarytool --wait` rodavam mudos até o fim: `sh_live` herda o terminal.
- [x] **Baixa.** `BUNDLE_ID` duplicava o `APP_ID` do `main.rs` sem nada prender os dois: teste do xtask lê o
  fonte do app e confere.
- [x] **Baixa.** Brief dizia "texto exato" do plist (o teste confere as chaves); plano com nomes velhos.
- [ ] **Baixa (fica).** O comentário de `.cargo/config.toml` ainda diz "M0: só `layers`"; o arquivo é
  sensível e não entrou nesta revisão: ajuste num próximo commit do xtask.

### 23/10/2026 — `xtask bundle`

- `bundle.rs`: `--universal` (dois targets + `lipo`, com checagem do target x86_64 no rustup) e `--no-dmg`;
  ferramentas conferidas no PATH antes de começar; `HY_DBPLUS_NOTARY_PROFILE` sem Developer ID é erro
  (a notarização recusa ad hoc). O `qlmanage` rendeu os sete tamanhos com fundo transparente fora do
  quadrado arredondado (o risco anotado não se confirmou). `codesign --deep` só no `.app`; o `.dmg` é
  assinado só com identidade real. `cp -R` para a pasta do `.dmg` (preserva a assinatura); link
  `/Applications` absoluto. `spctl --assess` dá "rejected" com ad hoc: esperado, é o Gatekeeper.
- Versão do app pelo `cargo metadata` (`app_version` no `main.rs`, testado).

### 23/10/2026 — Criação

- A pendência do Keychain (senha das conexões pedida a cada build, porque o binário sem assinatura
  estável muda de identidade) foi observada ao longo do desenvolvimento e decidida para o M8 na FEAT-003;
  não estava registrada nos docs dela. Fica registrada aqui.
- Jean pediu o instalador para fechar o dia; ícone a partir do `hyberica-icon.svg` e nome no padrão do
  `hy-vox-preto.svg`, em azul-petróleo; três variantes de nome e dois fundos apresentados, escolhido
  fundo claro e `y-dbplus`, azul mais escuro na segunda rodada. Docs e `packaging/` criados.
