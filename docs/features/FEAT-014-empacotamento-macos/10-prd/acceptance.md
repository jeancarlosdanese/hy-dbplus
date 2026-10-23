# @file: docs/features/FEAT-014-empacotamento-macos/10-prd/acceptance.md

# FEAT-014 — Critérios de aceite

## `packaging/`

- [x] `icon.svg` e `logo.svg`/`logo.png` no repositório, como aprovados (balão da Hyberica, azul-petróleo,
      fundo claro; nome `y-dbplus` em Inter Bold).

## `xtask bundle`

- [x] `cargo xtask bundle` no macOS deixa `target/bundle/hy-dbplus.app` (abre pela Dock, com o ícone e o
      nome certos no Finder e no menu) e `target/bundle/hy-dbplus-<versão>-arm64.dmg` com o `.app` e o
      atalho `Applications`.
- [x] `Info.plist` com identificador `br.com.hy.dbplus`, versão do workspace e HiDPI; `codesign --verify`
      passa (ad hoc por padrão); com `HY_DBPLUS_SIGN_IDENTITY` e `HY_DBPLUS_NOTARY_PROFILE`, assina com
      Developer ID e notariza (caminho escrito, não exercitado: sem certificado na máquina).
- [x] Fora do macOS, ou sem `osascript`/`iconutil`/`codesign`/`hdiutil`, erro claro; `--universal` sem o
      target x86_64 instalado diz o comando do rustup.
- [x] Testes do xtask cobrem o `Info.plist`, o iconset e a árvore do `.app`.

## Dock e janela

- [x] O app embute `packaging/icon-256.png` e o passa ao viewport: a Dock mostra o balão também no
      `cargo run`; PNG inválido avisa no stderr e cai no ícone padrão (teste do app decodifica em 256×256
      e exige cantos com alfa 0: o PNG antigo, de fundo branco, reprova).
- [x] ⌘M minimiza a janela nas duas telas (teste pelo `viewport_output`).

## Portão e CI

- [x] `scripts/quality-gate.sh --full` verde; [ ] workflow verde (push do Jean).

## Do Jean

- [ ] Rodar `cargo xtask bundle`, abrir o `.dmg`, arrastar para Applications, abrir pela Dock (primeira
      vez: botão direito → Abrir, porque a assinatura é ad hoc), conectar num banco e conferir o ícone na
      Dock e no `⌘Tab`.
