# @file: docs/features/FEAT-014-empacotamento-macos/10-prd/acceptance.md

# FEAT-014 — Critérios de aceite

## `packaging/`

- [ ] `icon.svg` e `logo.svg`/`logo.png` no repositório, como aprovados (balão da Hyberica, azul-petróleo,
      fundo claro; nome `y-dbplus` em Inter Bold).

## `xtask bundle`

- [ ] `cargo xtask bundle` no macOS deixa `target/bundle/hy-dbplus.app` (abre pela Dock, com o ícone e o
      nome certos no Finder e no menu) e `target/bundle/hy-dbplus-<versão>-arm64.dmg` com o `.app` e o
      atalho `Applications`.
- [ ] `Info.plist` com identificador `br.com.hy.dbplus`, versão do workspace e HiDPI; `codesign --verify`
      passa (ad hoc por padrão); com `HY_DBPLUS_SIGN_IDENTITY` e `HY_DBPLUS_NOTARY_PROFILE`, assina com
      Developer ID e notariza.
- [ ] Fora do macOS, ou sem `qlmanage`/`iconutil`/`codesign`/`hdiutil`, erro claro; `--universal` sem o
      target x86_64 instalado diz o comando do rustup.
- [ ] Testes do xtask cobrem o `Info.plist`, o iconset e a árvore do `.app`.

## Portão e CI

- [ ] `scripts/quality-gate.sh --full` verde; [ ] workflow verde (push do Jean).

## Do Jean

- [ ] Rodar `cargo xtask bundle`, abrir o `.dmg`, arrastar para Applications, abrir pela Dock (primeira
      vez: botão direito → Abrir, porque a assinatura é ad hoc), conectar num banco e conferir o ícone na
      Dock e no `⌘Tab`.
