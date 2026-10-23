# @file: docs/features/FEAT-014-empacotamento-macos/00-brief/brief.md

# FEAT-014 — Empacotamento macOS: `hy-dbplus.app`, `.dmg`, ícone e assinatura

## Problema

O app só roda por `cargo run`. O M8 do BRIEF pede o `.app` do macOS para instalar e abrir pela Dock
como qualquer programa. Não há ícone, `Info.plist` nem assinatura; sem assinatura estável o Keychain
pede a senha das conexões a cada build (observado desde o M1; registrado no log destas notas).

## Objetivo

`cargo xtask bundle` monta `target/bundle/hy-dbplus.app` a partir do build release, com o ícone da
Hyberica em azul-petróleo, assina e gera `target/bundle/hy-dbplus-<versão>-arm64.dmg` com o `.app` e
o atalho para `/Applications`. Hoje a assinatura é *ad hoc* (não há certificado Developer ID na
máquina); o mesmo comando assina com Developer ID e notariza quando as variáveis de ambiente
apontarem o certificado e o perfil do `notarytool`. Só ferramentas do macOS, nenhuma dependência nova.

## Escopo (Must)

1. **`packaging/`** (diretório protegido pelo processo): `icon.svg` (o balão com o "h" vazado do
   `hyberica-icon.svg`, azul-petróleo `#1C4F5E → #07252E` sobre o quadrado arredondado claro do macOS;
   sem texto) e `logo.svg` + `logo.png` (balão + `y-dbplus` em Inter Bold, padrão `hy-vox`; o PNG é a
   versão renderizada, porque o texto depende da fonte) e `icon-256.png`, o ícone em 256 px que o app
   embute (`include_bytes!` + `with_icon`: a Dock mostra o balão também no `cargo run` e por cima do
   cache). Aprovados pelo Jean em 23/10/2026. Para regenerar o PNG depois de mudar o SVG:
   `qlmanage -t -s 256 -o /tmp/icone packaging/icon.svg && cp /tmp/icone/icon.svg.png packaging/icon-256.png`
   (o `xtask bundle` regenera só o `.icns`).
2. **`xtask bundle`** (`xtask/src/bundle.rs`), só no macOS (fora dele, erro claro):
   - `cargo build --release -p hy-dbplus --locked`; com `--universal`, também
     `--target x86_64-apple-darwin` e `lipo` (exige o target no rustup; sem ele, erro que diz o
     `rustup target add`).
   - Árvore: `Contents/MacOS/hy-dbplus`, `Contents/Resources/hy-dbplus.icns`, `Contents/Info.plist`,
     `Contents/PkgInfo` (`APPL????`).
   - `Info.plist` gerado pelo xtask: `CFBundleName` e `CFBundleDisplayName` `hy-dbplus`,
     `CFBundleIdentifier` `br.com.hy.dbplus` (o `APP_ID` do `main.rs`), `CFBundleVersion` e
     `CFBundleShortVersionString` com a versão do workspace (`cargo metadata`), `CFBundleExecutable`,
     `CFBundleIconFile`, `CFBundlePackageType APPL`, `LSMinimumSystemVersion 12.0`,
     `NSHighResolutionCapable`, `LSApplicationCategoryType public.app-category.developer-tools`,
     `NSHumanReadableCopyright`.
   - Ícone: `packaging/icon.svg` → PNG em 16, 32, 64, 128, 256, 512 e 1024 (`qlmanage -t -s N`) →
     `hy-dbplus.iconset` com os pares `@2x` → `iconutil -c icns`.
   - Assinatura: `codesign --force --deep --sign <identidade>` com `HY_DBPLUS_SIGN_IDENTITY` (padrão
     `-`, ad hoc); com identidade real, `--options runtime --timestamp`. Entitlements não são
     necessários (sem sandbox; rede cliente é livre).
   - DMG: `hy-dbplus-<versão>-arm64.dmg` (ou `-universal.dmg` com `--universal`): pasta temporária
     com o `.app` e o link `Applications`, `hdiutil create -format UDZO`; com
     identidade real, assina o `.dmg`; com `HY_DBPLUS_NOTARY_PROFILE`, `xcrun notarytool submit --wait`
     e `xcrun stapler staple` no `.dmg`. `--no-dmg` para só o `.app`.
   - Saída: caminhos gerados e, no modo ad hoc, o aviso de que o Gatekeeper pede botão direito →
     Abrir na primeira vez (ou `xattr -d com.apple.quarantine`). O diretório de build vem do
     `cargo metadata` (`CARGO_TARGET_DIR` vale); `HY_DBPLUS_NOTARY_PROFILE` com `--no-dmg` é erro.
3. **Testes** (`cargo test -p xtask`, sem `codesign`/`hdiutil`): o `Info.plist` gerado (as chaves que
   importam, com a versão de exemplo), o identificador igual ao `APP_ID` do `main.rs`, a lista de tamanhos do iconset com os nomes `@2x`, e a montagem da
   árvore do `.app` num diretório temporário a partir de um binário e de um `.icns` falsos (caminhos,
   `PkgInfo`, bit de execução).
4. **Docs**: BRIEF M8 cita a FEAT; `CLAUDE.md` da raiz ganha o comando em "Comandos".

## Won't

- CI gerando o `.dmg` (entra quando houver tag de versão; a tag é do Jean); README do M8; Windows e
  Linux (`.msi`, `.deb`); Sparkle/atualização automática; App Store; sandbox.
- Resolver o aviso do Keychain por build: só a assinatura com Developer ID (estável entre builds)
  resolve; a ad hoc muda a cada build. Fica documentado.

## Restrições

- Sem dependência nova: `serde_json` (já no xtask) lê o `cargo metadata`; o resto é `std::process` com
  as ferramentas do macOS. `xtask/` é sensível: reviewer antes de cada commit. `packaging/` é
  protegido: criado com a aprovação do Jean nesta FEAT.
- Commits: `docs` (com `packaging/`); `feat(xtask)`.
