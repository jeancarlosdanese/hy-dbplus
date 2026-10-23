//! `cargo xtask bundle` (FEAT-014): monta `target/bundle/hy-dbplus.app` a partir do build release,
//! com o ícone (`packaging/icon.svg` → PNGs com alfa pelo AppKit via `packaging/render-icon.js` →
//! `.icns`), o `Info.plist` e a assinatura, e gera o `.dmg` com o `.app` e o atalho para
//! `/Applications`. Só ferramentas do macOS (`osascript`, `iconutil`, `codesign`, `hdiutil`,
//! `xcrun`); fora dele o comando recusa com erro claro.
//!
//! Assinatura: `HY_DBPLUS_SIGN_IDENTITY` (padrão `-`, ad hoc: o Gatekeeper pede botão direito →
//! Abrir na primeira vez). Com uma identidade Developer ID, assina com hardened runtime e
//! timestamp, assina o `.dmg` e, com `HY_DBPLUS_NOTARY_PROFILE` (perfil do `notarytool` guardado
//! no Keychain), notariza e grampeia. Nenhuma das duas variáveis carrega segredo.
//!
//! As partes puras (`plist`, `iconset_entries`, `layout`, `parse_args`) são testadas sem as
//! ferramentas; `run` as encadeia.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const APP_NAME: &str = "hy-dbplus";
/// O `APP_ID` de `apps/hy-dbplus/src/main.rs`.
pub const BUNDLE_ID: &str = "br.com.hy.dbplus";
const MIN_MACOS: &str = "12.0";
const X86_TARGET: &str = "x86_64-apple-darwin";
const ARM_TARGET: &str = "aarch64-apple-darwin";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// arm64 + x86_64 num binário só (`lipo`); exige o target x86_64 no rustup.
    pub universal: bool,
    pub dmg: bool,
}

/// `--universal` e `--no-dmg`; qualquer outra coisa é erro.
pub fn parse_args(args: &[OsString]) -> Result<Options, String> {
    let mut options = Options {
        universal: false,
        dmg: true,
    };
    for arg in args {
        match arg.to_string_lossy().as_ref() {
            "--universal" => options.universal = true,
            "--no-dmg" => options.dmg = false,
            other => {
                return Err(format!(
                    "argumento desconhecido `{other}` (aceita --universal, --no-dmg)"
                ));
            }
        }
    }
    Ok(options)
}

/// O `Info.plist` do app: nome, identificador, versão do workspace, executável, ícone, HiDPI.
pub fn plist(version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>{APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>{BUNDLE_ID}</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundleExecutable</key>
    <string>{APP_NAME}</string>
    <key>CFBundleIconFile</key>
    <string>{APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>{MIN_MACOS}</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>NSHumanReadableCopyright</key>
    <string>© Hyberica. MIT.</string>
</dict>
</plist>
"#
    )
}

/// Os arquivos do `.iconset` que o `iconutil` espera: nome e lado em pixels (os `@2x` dobram).
pub fn iconset_entries() -> [(&'static str, u32); 10] {
    [
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ]
}

/// Monta `<bundle_dir>/hy-dbplus.app` do zero: o binário em `Contents/MacOS`, o `.icns` em
/// `Contents/Resources`, o `Info.plist` e o `PkgInfo`. Devolve o caminho do `.app`.
pub fn layout(
    bundle_dir: &Path,
    binary: &Path,
    icns: &Path,
    version: &str,
) -> Result<PathBuf, String> {
    let app = bundle_dir.join(format!("{APP_NAME}.app"));
    if app.exists() {
        fs::remove_dir_all(&app)
            .map_err(|e| format!("não consegui limpar {}: {e}", app.display()))?;
    }
    let contents = app.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");
    for dir in [&macos, &resources] {
        fs::create_dir_all(dir)
            .map_err(|e| format!("não consegui criar {}: {e}", dir.display()))?;
    }
    let exe = macos.join(APP_NAME);
    fs::copy(binary, &exe)
        .map_err(|e| format!("não consegui copiar o binário {}: {e}", binary.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).map_err(|e| {
            format!(
                "não consegui dar permissão de execução a {}: {e}",
                exe.display()
            )
        })?;
    }
    let icon = resources.join(format!("{APP_NAME}.icns"));
    fs::copy(icns, &icon)
        .map_err(|e| format!("não consegui copiar o ícone {}: {e}", icns.display()))?;
    write(&contents.join("Info.plist"), &plist(version))?;
    write(&contents.join("PkgInfo"), "APPL????")?;
    Ok(app)
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    fs::write(path, text).map_err(|e| format!("não consegui gravar {}: {e}", path.display()))
}

/// O comando inteiro: build → iconset → `.app` → assinatura → `.dmg` (→ notarização). `target_dir`
/// vem do `cargo metadata` (respeita `CARGO_TARGET_DIR`).
pub fn run(args: &[OsString], root: &Path, target_dir: &Path, version: &str) -> Result<(), String> {
    let options = parse_args(args)?;
    if !cfg!(target_os = "macos") {
        return Err(
            "o bundle só é montado no macOS (precisa de osascript, iconutil, codesign e hdiutil)"
                .into(),
        );
    }
    for tool in ["osascript", "iconutil", "codesign", "hdiutil", "xcrun"] {
        if !on_path(tool) {
            return Err(format!(
                "`{tool}` não está no PATH: instale as ferramentas de linha de comando do Xcode"
            ));
        }
    }
    let identity = std::env::var_os("HY_DBPLUS_SIGN_IDENTITY")
        .map(|v| v.to_string_lossy().into_owned())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "-".to_owned());
    let ad_hoc = identity == "-";
    let notary = std::env::var_os("HY_DBPLUS_NOTARY_PROFILE")
        .map(|v| v.to_string_lossy().into_owned())
        .filter(|v| !v.trim().is_empty());
    if notary.is_some() && ad_hoc {
        return Err("HY_DBPLUS_NOTARY_PROFILE exige HY_DBPLUS_SIGN_IDENTITY (um Developer ID): a notarização recusa assinatura ad hoc".into());
    }
    if notary.is_some() && !options.dmg {
        return Err("HY_DBPLUS_NOTARY_PROFILE notariza o .dmg: não combina com --no-dmg".into());
    }

    let bundle_dir = target_dir.join("bundle");
    fs::create_dir_all(&bundle_dir)
        .map_err(|e| format!("não consegui criar {}: {e}", bundle_dir.display()))?;
    let binary = build(root, target_dir, &bundle_dir, options.universal)?;
    let icns = icns(root, &bundle_dir)?;
    let app = layout(&bundle_dir, &binary, &icns, version)?;
    sign(&app, &identity)?;
    println!("app: {}", app.display());
    if options.dmg {
        let arch = if options.universal {
            "universal"
        } else {
            "arm64"
        };
        let dmg = bundle_dir.join(format!("{APP_NAME}-{version}-{arch}.dmg"));
        make_dmg(&bundle_dir, &app, &dmg)?;
        if !ad_hoc {
            sign(&dmg, &identity)?;
        }
        if let Some(profile) = &notary {
            notarize(&dmg, profile)?;
        }
        println!("dmg: {}", dmg.display());
    }
    if ad_hoc {
        println!(
            "assinatura ad hoc: na primeira abertura o Gatekeeper pede botão direito → Abrir (ou \
             `xattr -d com.apple.quarantine`). Com um Developer ID, defina HY_DBPLUS_SIGN_IDENTITY \
             (e HY_DBPLUS_NOTARY_PROFILE para notarizar)."
        );
    }
    Ok(())
}

fn on_path(tool: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(tool).is_file()))
}

/// Roda um comando demorado com a saída no terminal (o build e a notarização levam minutos).
fn sh_live(what: &str, cmd: &mut Command) -> Result<(), String> {
    let status = cmd
        .status()
        .map_err(|e| format!("{what}: não consegui rodar: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{what} falhou ({status})"))
    }
}

/// Roda um comando e falha com a saída de erro dele.
fn sh(what: &str, cmd: &mut Command) -> Result<(), String> {
    let out = cmd
        .output()
        .map_err(|e| format!("{what}: não consegui rodar: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    Err(format!(
        "{what} falhou ({}):\n{}{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}

fn cargo() -> Command {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

/// `cargo build --release` do app; com `--universal`, os dois targets e `lipo`. Devolve o binário.
fn build(
    root: &Path,
    target_dir: &Path,
    bundle_dir: &Path,
    universal: bool,
) -> Result<PathBuf, String> {
    if !universal {
        sh_live(
            "cargo build --release",
            cargo()
                .args(["build", "--release", "-p", APP_NAME, "--locked"])
                .current_dir(root),
        )?;
        return Ok(target_dir.join("release").join(APP_NAME));
    }
    let installed = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map_err(|e| format!("rustup: não consegui listar os targets: {e}"))?;
    if !String::from_utf8_lossy(&installed.stdout)
        .lines()
        .any(|l| l.trim() == X86_TARGET)
    {
        return Err(format!(
            "--universal precisa do target {X86_TARGET}: rode `rustup target add {X86_TARGET}`"
        ));
    }
    let mut slices = Vec::new();
    for target in [ARM_TARGET, X86_TARGET] {
        sh_live(
            &format!("cargo build --release --target {target}"),
            cargo()
                .args([
                    "build",
                    "--release",
                    "-p",
                    APP_NAME,
                    "--locked",
                    "--target",
                    target,
                ])
                .current_dir(root),
        )?;
        slices.push(target_dir.join(target).join("release").join(APP_NAME));
    }
    let out = bundle_dir.join(format!("{APP_NAME}-universal"));
    let mut lipo = Command::new("lipo");
    lipo.arg("-create").arg("-output").arg(&out).args(&slices);
    sh("lipo", &mut lipo)?;
    Ok(out)
}

/// `packaging/icon.svg` → PNGs por tamanho com fundo transparente (`packaging/render-icon.js`,
/// AppKit via `osascript`) → `.iconset` → `.icns` (`iconutil`). O `qlmanage` pintava fundo branco
/// fora do quadrado arredondado, e a Dock mostrava um quadrado branco.
fn icns(root: &Path, bundle_dir: &Path) -> Result<PathBuf, String> {
    let svg = root.join("packaging").join("icon.svg");
    let script = root.join("packaging").join("render-icon.js");
    for file in [&svg, &script] {
        if !file.is_file() {
            return Err(format!("arquivo ausente: {}", file.display()));
        }
    }
    let iconset = bundle_dir.join(format!("{APP_NAME}.iconset"));
    if iconset.exists() {
        fs::remove_dir_all(&iconset)
            .map_err(|e| format!("não consegui limpar {}: {e}", iconset.display()))?;
    }
    fs::create_dir_all(&iconset)
        .map_err(|e| format!("não consegui criar {}: {e}", iconset.display()))?;
    for (name, size) in iconset_entries() {
        let png = iconset.join(name);
        let out = Command::new("osascript")
            .arg("-l")
            .arg("JavaScript")
            .arg(&script)
            .arg(&svg)
            .arg(&png)
            .arg(size.to_string())
            .output()
            .map_err(|e| {
                format!("render-icon.js ({size}px): não consegui rodar o osascript: {e}")
            })?;
        let said = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        if !out.status.success() || said != "ok" || !png.is_file() {
            return Err(format!(
                "render-icon.js ({size}px) falhou: {said}{}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }
    let icns = bundle_dir.join(format!("{APP_NAME}.icns"));
    sh(
        "iconutil",
        Command::new("iconutil")
            .arg("-c")
            .arg("icns")
            .arg("-o")
            .arg(&icns)
            .arg(&iconset),
    )?;
    Ok(icns)
}

/// `codesign` do `.app` (ou do `.dmg`): ad hoc com `-`; Developer ID com hardened runtime e
/// timestamp. Depois, `codesign --verify`.
fn sign(target: &Path, identity: &str) -> Result<(), String> {
    let mut cmd = Command::new("codesign");
    cmd.arg("--force").arg("--sign").arg(identity);
    if target.extension().is_some_and(|e| e == "app") {
        cmd.arg("--deep");
    }
    if identity != "-" {
        cmd.args(["--options", "runtime", "--timestamp"]);
    }
    cmd.arg(target);
    sh("codesign", &mut cmd)?;
    sh(
        "codesign --verify",
        Command::new("codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(target),
    )
}

/// O `.dmg`: pasta temporária com o `.app` e o link `Applications`, comprimido (`UDZO`).
fn make_dmg(bundle_dir: &Path, app: &Path, dmg: &Path) -> Result<(), String> {
    let staging = bundle_dir.join("dmg");
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .map_err(|e| format!("não consegui limpar {}: {e}", staging.display()))?;
    }
    fs::create_dir_all(&staging)
        .map_err(|e| format!("não consegui criar {}: {e}", staging.display()))?;
    // `cp -R` preserva permissões e assinatura (um `fs::copy` por arquivo perderia metadados).
    sh(
        "cp -R do .app",
        Command::new("cp").arg("-R").arg(app).arg(&staging),
    )?;
    #[cfg(unix)]
    std::os::unix::fs::symlink("/Applications", staging.join("Applications"))
        .map_err(|e| format!("não consegui criar o link Applications: {e}"))?;
    if dmg.exists() {
        fs::remove_file(dmg).map_err(|e| format!("não consegui apagar {}: {e}", dmg.display()))?;
    }
    sh(
        "hdiutil create",
        Command::new("hdiutil")
            .arg("create")
            .arg("-volname")
            .arg(APP_NAME)
            .arg("-srcfolder")
            .arg(&staging)
            .arg("-ov")
            .arg("-format")
            .arg("UDZO")
            .arg(dmg),
    )
}

/// `notarytool submit --wait` com o perfil do Keychain e `stapler staple` no `.dmg`.
fn notarize(dmg: &Path, profile: &str) -> Result<(), String> {
    sh_live(
        "xcrun notarytool submit",
        Command::new("xcrun")
            .args([
                "notarytool",
                "submit",
                "--wait",
                "--keychain-profile",
                profile,
            ])
            .arg(dmg),
    )?;
    sh(
        "xcrun stapler staple",
        Command::new("xcrun").args(["stapler", "staple"]).arg(dmg),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "hy-dbplus-bundle-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn argumentos_universal_no_dmg_e_desconhecido() {
        assert_eq!(
            parse_args(&[]).unwrap(),
            Options {
                universal: false,
                dmg: true
            }
        );
        assert_eq!(
            parse_args(&["--universal".into(), "--no-dmg".into()]).unwrap(),
            Options {
                universal: true,
                dmg: false
            }
        );
        assert!(parse_args(&["--xyz".into()]).unwrap_err().contains("--xyz"));
    }

    #[test]
    fn o_plist_tem_identificador_versao_executavel_icone_e_hidpi() {
        let p = plist("0.1.0");
        for needle in [
            "<key>CFBundleIdentifier</key>\n    <string>br.com.hy.dbplus</string>",
            "<key>CFBundleShortVersionString</key>\n    <string>0.1.0</string>",
            "<key>CFBundleVersion</key>\n    <string>0.1.0</string>",
            "<key>CFBundleExecutable</key>\n    <string>hy-dbplus</string>",
            "<key>CFBundleIconFile</key>\n    <string>hy-dbplus</string>",
            "<key>NSHighResolutionCapable</key>\n    <true/>",
            "<key>CFBundlePackageType</key>\n    <string>APPL</string>",
        ] {
            assert!(p.contains(needle), "{needle}");
        }
        assert!(p.starts_with("<?xml") && p.trim_end().ends_with("</plist>"));
    }

    /// O xtask não pode depender do app: o identificador é conferido lendo o fonte dele.
    #[test]
    fn o_bundle_id_e_o_app_id_do_main_do_app() {
        let main = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("apps/hy-dbplus/src/main.rs");
        let source = fs::read_to_string(main).unwrap();
        assert!(
            source.contains(&format!("const APP_ID: &str = \"{BUNDLE_ID}\";")),
            "APP_ID do main.rs difere de BUNDLE_ID ({BUNDLE_ID})"
        );
    }

    #[test]
    fn o_iconset_tem_os_dez_arquivos_com_os_lados_certos() {
        let entries = iconset_entries();
        assert_eq!(entries.len(), 10);
        for (name, size) in entries {
            let base: u32 = name
                .trim_start_matches("icon_")
                .split('x')
                .next()
                .unwrap()
                .parse()
                .unwrap();
            let expected = if name.contains("@2x") { base * 2 } else { base };
            assert_eq!(size, expected, "{name}");
        }
    }

    #[test]
    fn a_arvore_do_app_tem_binario_executavel_icone_plist_e_pkginfo() {
        let dir = temp("layout");
        let binary = dir.join("bin");
        let icns = dir.join("x.icns");
        fs::write(&binary, b"#!/bin/sh\n").unwrap();
        fs::write(&icns, b"icns").unwrap();
        let app = layout(&dir, &binary, &icns, "9.9.9").unwrap();
        assert_eq!(app, dir.join("hy-dbplus.app"));
        let contents = app.join("Contents");
        assert_eq!(
            fs::read(contents.join("MacOS/hy-dbplus")).unwrap(),
            b"#!/bin/sh\n"
        );
        assert_eq!(
            fs::read(contents.join("Resources/hy-dbplus.icns")).unwrap(),
            b"icns"
        );
        assert_eq!(
            fs::read_to_string(contents.join("PkgInfo")).unwrap(),
            "APPL????"
        );
        assert!(
            fs::read_to_string(contents.join("Info.plist"))
                .unwrap()
                .contains("<string>9.9.9</string>")
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(contents.join("MacOS/hy-dbplus"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111, "executável");
        }
        // Montar de novo recomeça do zero (um arquivo velho não sobrevive).
        fs::write(contents.join("velho"), b"x").unwrap();
        layout(&dir, &binary, &icns, "9.9.9").unwrap();
        assert!(!contents.join("velho").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
