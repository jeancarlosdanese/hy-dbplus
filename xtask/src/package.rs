//! Os pacotes do Linux e do Windows (FEAT-016), montados pelo `cargo xtask bundle` no próprio sistema:
//!
//! - Linux: `.deb` (binários em `/usr/bin`, atalho no menu, ícone no tema `hicolor`, licença) pelo
//!   `dpkg-deb`, e `.tar.gz` com os binários, o ícone, o atalho e a licença, pelo `tar`;
//! - Windows: `.zip` com os dois `.exe` e a licença, pelo `tar` do sistema (bsdtar, grava zip).
//!
//! Nome de todo pacote: `hy-dbplus-<versão>-<sistema>-<arquitetura>.<ext>` ([`artifact_name`]).
//! As partes puras (nomes, `control`, `.desktop`, árvores) são testadas em qualquer sistema; `linux`
//! e `windows` encadeiam o build, as árvores e a ferramenta de cada sistema.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::bundle::{APP_NAME, BUNDLE_ID, cargo, sh, sh_live};

/// O binário da CLI (`apps/hy-dbplus-cli`).
pub const CLI_NAME: &str = "hy-dbplus-cli";
/// Bibliotecas que o app carrega no Linux: libc e o runtime do Rust; GL e EGL pelo `glow` e pelo
/// `glutin` (`libwayland-egl` no Wayland); teclado, Wayland e X11 pelo `winit` (`libX11`,
/// `libX11-xcb`, `libXcursor`, `libXi`, `libxkbcommon` e `libxkbcommon-x11`). As de GL, Wayland e X11
/// são abertas em tempo de execução (`dlopen`) e não aparecem no `ldd`; sem `libxkbcommon-x11` ou
/// `libwayland-egl`, o app cai na abertura (achado do revisor). O CI instala o `.deb` num Debian
/// limpo para conferir que o `apt` resolve todas.
pub const DEB_DEPENDS: &str = "libc6, libgcc-s1, libgl1, libegl1, libwayland-client0, \
                               libwayland-egl1, libx11-6, libx11-xcb1, libxcursor1, libxi6, \
                               libxkbcommon0, libxkbcommon-x11-0";

/// `hy-dbplus-0.2.0-linux-x86_64.deb`.
pub fn artifact_name(version: &str, os: &str, arch: &str, ext: &str) -> String {
    format!("{APP_NAME}-{version}-{os}-{arch}.{ext}")
}

/// A arquitetura no nome do pacote: `x86_64` e `arm64`; outra é recusada (não há pacote para ela).
pub fn arch_label(rust_arch: &str) -> Result<&'static str, String> {
    match rust_arch {
        "x86_64" => Ok("x86_64"),
        "aarch64" => Ok("arm64"),
        other => Err(format!(
            "arquitetura `{other}` sem pacote (só x86_64 e aarch64)"
        )),
    }
}

/// A arquitetura no `control` do `.deb`.
pub fn deb_arch(rust_arch: &str) -> Result<&'static str, String> {
    match rust_arch {
        "x86_64" => Ok("amd64"),
        "aarch64" => Ok("arm64"),
        other => Err(format!("arquitetura `{other}` sem pacote .deb")),
    }
}

/// O `DEBIAN/control`. `installed_kb`: o tamanho instalado, em KiB; `homepage`: o `repository` do
/// workspace (vem do `cargo metadata`, sem cópia aqui).
pub fn deb_control(version: &str, arch: &str, installed_kb: u64, homepage: &str) -> String {
    format!(
        "Package: {APP_NAME}\n\
         Version: {version}\n\
         Section: database\n\
         Priority: optional\n\
         Architecture: {arch}\n\
         Maintainer: Hyberica <{homepage}>\n\
         Installed-Size: {installed_kb}\n\
         Depends: {DEB_DEPENDS}\n\
         Homepage: {homepage}\n\
         Description: cliente PostgreSQL desktop, leve e simples\n \
         Navega dados e escreve SQL com autocomplete; inclui a CLI hy-dbplus-cli.\n"
    )
}

/// O nome do atalho: o `app_id` da janela, para o Wayland casar a janela com o ícone.
pub fn desktop_file_name() -> String {
    format!("{BUNDLE_ID}.desktop")
}

/// O atalho do menu (`.desktop`).
pub fn desktop_entry() -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={APP_NAME}\n\
         GenericName=Database client\n\
         GenericName[pt_BR]=Cliente de banco de dados\n\
         Comment=Lightweight PostgreSQL client\n\
         Comment[pt_BR]=Cliente PostgreSQL leve e simples\n\
         Exec={APP_NAME}\n\
         Icon={APP_NAME}\n\
         Terminal=false\n\
         Categories=Development;Database;\n\
         StartupWMClass={BUNDLE_ID}\n"
    )
}

/// O que vai nos pacotes, já compilado ou no repositório.
#[derive(Debug, Clone)]
pub struct Inputs {
    pub app: PathBuf,
    pub cli: PathBuf,
    pub icon_png: PathBuf,
    pub license: PathBuf,
}

fn create_dir(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("não consegui criar {}: {e}", dir.display()))
}

/// Recria `dir` vazio (o que sobrou de um bundle anterior sairia no pacote).
fn fresh_dir(dir: &Path) -> Result<(), String> {
    if dir.exists() {
        fs::remove_dir_all(dir)
            .map_err(|e| format!("não consegui limpar {}: {e}", dir.display()))?;
    }
    create_dir(dir)
}

/// Copia com o modo dado (no Unix; no Windows o modo não se aplica).
fn copy(from: &Path, to: &Path, mode: u32) -> Result<u64, String> {
    let size = fs::copy(from, to).map_err(|e| {
        format!(
            "não consegui copiar {} para {}: {e}",
            from.display(),
            to.display()
        )
    })?;
    set_mode(to, mode)?;
    Ok(size)
}

fn write(path: &Path, text: &str, mode: u32) -> Result<u64, String> {
    fs::write(path, text).map_err(|e| format!("não consegui gravar {}: {e}", path.display()))?;
    set_mode(path, mode)?;
    Ok(text.len() as u64)
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|e| {
        format!(
            "não consegui ajustar as permissões de {}: {e}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn set_mode(_: &Path, _: u32) -> Result<(), String> {
    Ok(())
}

/// A árvore do `.deb` em `root` (recriada): `DEBIAN/control`, os binários em `usr/bin`, o atalho, o
/// ícone e a licença em `usr/share/doc/hy-dbplus/copyright`. Pastas 0755, binários 0755, o resto
/// 0644 (o `dpkg-deb --root-owner-group` põe tudo como root).
pub fn deb_tree(
    root: &Path,
    inputs: &Inputs,
    version: &str,
    arch: &str,
    homepage: &str,
) -> Result<(), String> {
    fresh_dir(root)?;
    let bin = root.join("usr/bin");
    let apps = root.join("usr/share/applications");
    let icons = root.join("usr/share/icons/hicolor/256x256/apps");
    let doc = root.join(format!("usr/share/doc/{APP_NAME}"));
    let debian = root.join("DEBIAN");
    for dir in [&bin, &apps, &icons, &doc, &debian] {
        create_dir(dir)?;
    }
    let mut size = 0;
    size += copy(&inputs.app, &bin.join(APP_NAME), 0o755)?;
    size += copy(&inputs.cli, &bin.join(CLI_NAME), 0o755)?;
    size += write(&apps.join(desktop_file_name()), &desktop_entry(), 0o644)?;
    size += copy(
        &inputs.icon_png,
        &icons.join(format!("{APP_NAME}.png")),
        0o644,
    )?;
    size += copy(&inputs.license, &doc.join("copyright"), 0o644)?;
    // As pastas criadas herdam a umask; o dpkg reclama de pasta que não seja 0755.
    for dir in dirs_under(root)? {
        set_mode(&dir, 0o755)?;
    }
    write(
        &debian.join("control"),
        &deb_control(version, arch, size.div_ceil(1024), homepage),
        0o644,
    )?;
    Ok(())
}

/// Todas as pastas sob `root`, ele incluído.
fn dirs_under(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = vec![root.to_path_buf()];
    let mut i = 0;
    while let Some(dir) = out.get(i).cloned() {
        let entries =
            fs::read_dir(&dir).map_err(|e| format!("não consegui ler {}: {e}", dir.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|e| format!("não consegui ler {}: {e}", dir.display()))?
                .path();
            if path.is_dir() {
                out.push(path);
            }
        }
        i += 1;
    }
    Ok(out)
}

/// A pasta do `.tar.gz` do Linux (recriada): binários, ícone, atalho e licença, lado a lado.
pub fn tarball_tree(dir: &Path, inputs: &Inputs) -> Result<(), String> {
    fresh_dir(dir)?;
    set_mode(dir, 0o755)?;
    copy(&inputs.app, &dir.join(APP_NAME), 0o755)?;
    copy(&inputs.cli, &dir.join(CLI_NAME), 0o755)?;
    copy(
        &inputs.icon_png,
        &dir.join(format!("{APP_NAME}.png")),
        0o644,
    )?;
    write(&dir.join(desktop_file_name()), &desktop_entry(), 0o644)?;
    copy(&inputs.license, &dir.join("LICENSE"), 0o644)?;
    Ok(())
}

/// A pasta do `.zip` do Windows (recriada): os dois `.exe` e a licença.
pub fn zip_tree(dir: &Path, inputs: &Inputs) -> Result<(), String> {
    fresh_dir(dir)?;
    copy(&inputs.app, &dir.join(format!("{APP_NAME}.exe")), 0o755)?;
    copy(&inputs.cli, &dir.join(format!("{CLI_NAME}.exe")), 0o755)?;
    copy(&inputs.license, &dir.join("LICENSE.txt"), 0o644)?;
    Ok(())
}

/// `cargo build --release` do app e da CLI; devolve os dois binários.
fn build(root: &Path, target_dir: &Path, exe_suffix: &str) -> Result<(PathBuf, PathBuf), String> {
    sh_live(
        "cargo build --release",
        cargo()
            .args([
                "build",
                "--release",
                "--locked",
                "-p",
                APP_NAME,
                "-p",
                CLI_NAME,
            ])
            .current_dir(root),
    )?;
    let release = target_dir.join("release");
    Ok((
        release.join(format!("{APP_NAME}{exe_suffix}")),
        release.join(format!("{CLI_NAME}{exe_suffix}")),
    ))
}

fn inputs(root: &Path, app: PathBuf, cli: PathBuf) -> Result<Inputs, String> {
    let inputs = Inputs {
        app,
        cli,
        icon_png: root.join("packaging").join("icon-256.png"),
        license: root.join("LICENSE"),
    };
    for file in [&inputs.app, &inputs.cli, &inputs.icon_png, &inputs.license] {
        if !file.is_file() {
            return Err(format!("arquivo ausente: {}", file.display()));
        }
    }
    Ok(inputs)
}

/// Linux: build → árvore do `.deb` → `dpkg-deb` → pasta do `.tar.gz` → `tar`.
pub fn linux(root: &Path, target_dir: &Path, version: &str, homepage: &str) -> Result<(), String> {
    let arch = std::env::consts::ARCH;
    let (label, deb_arch) = (arch_label(arch)?, deb_arch(arch)?);
    for tool in ["dpkg-deb", "tar"] {
        if !crate::bundle::on_path(tool) {
            return Err(format!(
                "`{tool}` não está no PATH (o .deb sai num Debian ou Ubuntu)"
            ));
        }
    }
    let bundle_dir = target_dir.join("bundle");
    create_dir(&bundle_dir)?;
    let (app, cli) = build(root, target_dir, "")?;
    let inputs = inputs(root, app, cli)?;

    let deb_root = bundle_dir
        .join("deb")
        .join(format!("{APP_NAME}_{version}_{deb_arch}"));
    deb_tree(&deb_root, &inputs, version, deb_arch, homepage)?;
    let deb = bundle_dir.join(artifact_name(version, "linux", label, "deb"));
    sh(
        "dpkg-deb --build",
        Command::new("dpkg-deb")
            .args(["--build", "--root-owner-group"])
            .arg(&deb_root)
            .arg(&deb),
    )?;
    println!("deb: {}", deb.display());

    let stem = format!("{APP_NAME}-{version}-linux-{label}");
    tarball_tree(&bundle_dir.join("tar").join(&stem), &inputs)?;
    let tarball = bundle_dir.join(artifact_name(version, "linux", label, "tar.gz"));
    sh(
        "tar",
        // Dono root em toda entrada (GNU tar): extraído com sudo, nada fica do usuário que montou
        // (achado do revisor; o .deb tem o mesmo pelo --root-owner-group).
        Command::new("tar")
            .args(["--owner=0", "--group=0", "--numeric-owner"])
            .arg("-czf")
            .arg(&tarball)
            .arg("-C")
            .arg(bundle_dir.join("tar"))
            .arg(&stem),
    )?;
    println!("tar.gz: {}", tarball.display());
    Ok(())
}

/// Windows: build → pasta do `.zip` → `tar -a` (o bsdtar do sistema, em `System32`; o `tar` do Git
/// no PATH é o GNU, que não grava zip).
pub fn windows(root: &Path, target_dir: &Path, version: &str) -> Result<(), String> {
    let label = arch_label(std::env::consts::ARCH)?;
    let tar = std::env::var_os("SystemRoot")
        .map(|r| PathBuf::from(r).join("System32").join("tar.exe"))
        .filter(|p| p.is_file())
        .ok_or_else(|| "tar.exe do Windows não encontrado em %SystemRoot%\\System32".to_owned())?;
    let bundle_dir = target_dir.join("bundle");
    create_dir(&bundle_dir)?;
    let (app, cli) = build(root, target_dir, ".exe")?;
    let inputs = inputs(root, app, cli)?;
    let stem = format!("{APP_NAME}-{version}-windows-{label}");
    zip_tree(&bundle_dir.join("zip").join(&stem), &inputs)?;
    let zip = bundle_dir.join(artifact_name(version, "windows", label, "zip"));
    sh(
        "tar -a (zip)",
        Command::new(tar)
            .arg("-a")
            .arg("-c")
            .arg("-f")
            .arg(&zip)
            .arg("-C")
            .arg(bundle_dir.join("zip"))
            .arg(&stem),
    )?;
    println!("zip: {}", zip.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hy-dbplus-xtask-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fake_inputs(dir: &Path) -> Inputs {
        let file = |name: &str, body: &str| {
            let path = dir.join(name);
            fs::write(&path, body).unwrap();
            path
        };
        Inputs {
            app: file("app-bin", "app"),
            cli: file("cli-bin", "cli"),
            icon_png: file("icon.png", "png"),
            license: file("LICENSE", "MIT"),
        }
    }

    #[test]
    fn nomes_dos_pacotes_e_arquiteturas() {
        assert_eq!(
            artifact_name("0.2.0", "linux", "x86_64", "tar.gz"),
            "hy-dbplus-0.2.0-linux-x86_64.tar.gz"
        );
        assert_eq!(
            artifact_name("0.2.0", "macos", "universal", "dmg"),
            "hy-dbplus-0.2.0-macos-universal.dmg"
        );
        assert_eq!(arch_label("x86_64").unwrap(), "x86_64");
        assert_eq!(arch_label("aarch64").unwrap(), "arm64");
        assert!(arch_label("riscv64").is_err());
        assert_eq!(deb_arch("x86_64").unwrap(), "amd64");
        assert_eq!(deb_arch("aarch64").unwrap(), "arm64");
        assert!(deb_arch("x86").is_err());
    }

    #[test]
    fn control_do_deb() {
        let control = deb_control("0.2.0", "amd64", 12345, "https://example.org/hy");
        for line in [
            "Package: hy-dbplus\n",
            "Version: 0.2.0\n",
            "Architecture: amd64\n",
            "Installed-Size: 12345\n",
            "Depends: libc6, libgcc-s1, libgl1, libegl1, libwayland-client0, libwayland-egl1, libx11-6, libx11-xcb1, libxcursor1, libxi6, libxkbcommon0, libxkbcommon-x11-0\n",
            "Homepage: https://example.org/hy\n",
            "Maintainer: Hyberica <https://example.org/hy>\n",
        ] {
            assert!(control.contains(line), "{line:?} em\n{control}");
        }
        // Formato do dpkg: termina em \n, a descrição longa começa com espaço e não há linha vazia.
        assert!(control.ends_with('\n'));
        assert!(control.contains("\n Navega dados"));
        assert!(!control.contains("\n\n"));
        // Nenhum campo com espaço no fim (o dpkg-deb recusa alguns).
        assert!(control.lines().all(|l| l.trim_end() == l), "{control}");
    }

    #[test]
    fn atalho_do_menu() {
        assert_eq!(desktop_file_name(), "br.com.hy.dbplus.desktop");
        let entry = desktop_entry();
        assert!(entry.starts_with("[Desktop Entry]\n"));
        for line in [
            "Type=Application",
            "Name=hy-dbplus",
            "Exec=hy-dbplus",
            "Icon=hy-dbplus",
            "Terminal=false",
            "Categories=Development;Database;",
            "StartupWMClass=br.com.hy.dbplus",
        ] {
            assert!(entry.lines().any(|l| l == line), "{line}");
        }
    }

    #[test]
    fn arvore_do_deb() {
        let dir = temp("deb");
        let inputs = fake_inputs(&dir);
        let root = dir.join("pkg");
        fs::create_dir_all(root.join("sobra")).unwrap();
        deb_tree(&root, &inputs, "0.2.0", "amd64", "https://example.org/hy").unwrap();
        assert!(!root.join("sobra").exists(), "a árvore não foi recriada");
        let files = [
            "DEBIAN/control",
            "usr/bin/hy-dbplus",
            "usr/bin/hy-dbplus-cli",
            "usr/share/applications/br.com.hy.dbplus.desktop",
            "usr/share/icons/hicolor/256x256/apps/hy-dbplus.png",
            "usr/share/doc/hy-dbplus/copyright",
        ];
        for file in files {
            assert!(root.join(file).is_file(), "{file}");
        }
        assert_eq!(
            fs::read_to_string(root.join("usr/bin/hy-dbplus")).unwrap(),
            "app"
        );
        let control = fs::read_to_string(root.join("DEBIAN/control")).unwrap();
        assert!(control.contains("Version: 0.2.0\n"));
        assert!(control.contains("Installed-Size: 1\n"), "{control}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = |p: &str| fs::metadata(root.join(p)).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode("usr/bin/hy-dbplus"), 0o755);
            assert_eq!(mode("usr/bin/hy-dbplus-cli"), 0o755);
            assert_eq!(mode("DEBIAN/control"), 0o644);
            assert_eq!(mode("usr/share/doc/hy-dbplus/copyright"), 0o644);
            for d in ["", "DEBIAN", "usr", "usr/share/icons/hicolor/256x256/apps"] {
                assert_eq!(mode(d), 0o755, "pasta {d:?}");
            }
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pastas_do_tar_gz_e_do_zip() {
        let dir = temp("tar-zip");
        let inputs = fake_inputs(&dir);
        let tar = dir.join("hy-dbplus-0.2.0-linux-x86_64");
        tarball_tree(&tar, &inputs).unwrap();
        let mut names: Vec<String> = fs::read_dir(&tar)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "LICENSE",
                "br.com.hy.dbplus.desktop",
                "hy-dbplus",
                "hy-dbplus-cli",
                "hy-dbplus.png"
            ]
        );
        let zip = dir.join("hy-dbplus-0.2.0-windows-x86_64");
        zip_tree(&zip, &inputs).unwrap();
        let mut names: Vec<String> = fs::read_dir(&zip)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["LICENSE.txt", "hy-dbplus-cli.exe", "hy-dbplus.exe"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn entrada_que_falta_e_recusada() {
        let dir = temp("inputs");
        let err = inputs(&dir, dir.join("nao-ha"), dir.join("nem-este")).unwrap_err();
        assert!(err.contains("arquivo ausente"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    /// O app do Windows abre sem janela de console em release (o atributo fica no `main.rs`).
    #[test]
    fn o_app_declara_o_subsistema_grafico_do_windows() {
        let main = Path::new(env!("CARGO_MANIFEST_DIR")).join("../apps/hy-dbplus/src/main.rs");
        let text = fs::read_to_string(main).unwrap();
        assert!(
            text.lines().any(|l| l.trim()
                == "#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = \"windows\")]"),
            "falta o windows_subsystem no main.rs do app"
        );
    }
}
