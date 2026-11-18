//! `cargo xtask version <x.y.z>` (FEAT-016): sobe a versão do produto nos três lugares que a guardam,
//! e só nas linhas dela. É o comando `bump` do `.claude/process.json`: a entrega o roda na `main`
//! depois do merge, e a trava do plugin aceita o commit só se cada arquivo mudou apenas pela troca
//! da versão.
//!
//! - `VERSION` (primeira linha): o que a trava do plugin lê (ela não lê o `Cargo.toml`);
//! - `Cargo.toml`, `[workspace.package] version`: o que o Cargo usa (`CARGO_PKG_VERSION`, o
//!   `Info.plist` do `.app`);
//! - `Cargo.lock`: a linha `version` de cada membro do workspace. Dependência de terceiro com a
//!   mesma versão fica como está (nada é resolvido de novo).
//!
//! `cargo xtask version --check <tag>` falha se a tag não é `v` + a versão do `VERSION` (o
//! `release.yml` roda isso antes de montar os pacotes).
//!
//! As partes puras (`parse`, `workspace_version`, `set_workspace_version`, `set_lock_versions`,
//! `check_tag`, `plan`) são testadas sem tocar arquivo; `run` lê, confere e grava.

use std::cmp::Ordering;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::Path;

pub const VERSION_FILE: &str = "VERSION";

/// `x.y.z`, só números, sem zero à esquerda (semver sem pré-lançamento nem build).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub fn parse(text: &str) -> Result<Version, String> {
    let invalid = || format!("versão `{text}` inválida: use x.y.z, só números (ex.: 0.2.0)");
    let parts: Vec<&str> = text.split('.').collect();
    let [major, minor, patch] = parts.as_slice() else {
        return Err(invalid());
    };
    let number = |part: &str| -> Result<u64, String> {
        let digits = !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
        let leading_zero = part.len() > 1 && part.starts_with('0');
        if !digits || leading_zero {
            return Err(invalid());
        }
        part.parse::<u64>().map_err(|_| invalid())
    };
    Ok(Version {
        major: number(major)?,
        minor: number(minor)?,
        patch: number(patch)?,
    })
}

/// A versão do arquivo `VERSION`, que só pode ter `x.y.z` e, opcionalmente, um fim de linha. Fora
/// disso o comando recusa: a troca regrava o arquivo inteiro, e a trava do plugin só aceita o
/// commit da versão se a única mudança for a versão (achado do revisor).
pub fn version_file(text: &str) -> Result<Version, String> {
    let body = text.strip_suffix('\n').unwrap_or(text);
    if body.is_empty() {
        return Err(format!("{VERSION_FILE} vazio"));
    }
    if body.contains(['\n', '\r']) || body.trim() != body {
        return Err(format!(
            "{VERSION_FILE} fora do formato: só a versão x.y.z e um fim de linha"
        ));
    }
    parse(body)
}

/// A linha `version = "..."` da seção `[workspace.package]`: (índice da linha, valor).
fn workspace_version_line(cargo_toml: &str) -> Result<(usize, String), String> {
    let mut in_section = false;
    for (i, line) in cargo_toml.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed == "[workspace.package]";
            continue;
        }
        if in_section
            && let Some(value) = trimmed
                .strip_prefix("version = \"")
                .and_then(|rest| rest.strip_suffix('"'))
        {
            return Ok((i, value.to_owned()));
        }
    }
    Err("Cargo.toml sem `version = \"...\"` em [workspace.package]".into())
}

pub fn workspace_version(cargo_toml: &str) -> Result<Version, String> {
    let (_, value) = workspace_version_line(cargo_toml)?;
    parse(&value)
}

/// Nas linhas cujo índice está em `targets`, troca só o `"<old>"` por `"<new>"`; o resto do texto
/// (recuo, espaço no fim, fins de linha) fica byte a byte, como a trava do plugin confere.
fn replace_lines(text: &str, targets: &[usize], old: Version, new: Version) -> String {
    let (from, to) = (format!("\"{old}\""), format!("\"{new}\""));
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split_inclusive('\n').enumerate() {
        if targets.contains(&i) {
            out.push_str(&line.replacen(&from, &to, 1));
        } else {
            out.push_str(line);
        }
    }
    out
}

pub fn set_workspace_version(
    cargo_toml: &str,
    old: Version,
    new: Version,
) -> Result<String, String> {
    let (line, value) = workspace_version_line(cargo_toml)?;
    if parse(&value)? != old {
        return Err(format!(
            "Cargo.toml: [workspace.package] está em {value}, não em {old}"
        ));
    }
    Ok(replace_lines(cargo_toml, &[line], old, new))
}

/// Troca `version = "<old>"` pelos de `new` nos blocos `[[package]]` dos membros do workspace.
/// Cada membro tem de aparecer uma vez, com a versão antiga; senão o lock não é o deste workspace.
pub fn set_lock_versions(
    lock: &str,
    members: &[String],
    old: Version,
    new: Version,
) -> Result<String, String> {
    let old_line = format!("version = \"{old}\"");
    let mut current: Option<&str> = None;
    let mut targets = Vec::new();
    let mut found: Vec<&str> = Vec::new();
    for (i, line) in lock.lines().enumerate() {
        if line == "[[package]]" {
            current = None;
        } else if let Some(name) = line
            .strip_prefix("name = \"")
            .and_then(|rest| rest.strip_suffix('"'))
        {
            current = Some(name);
        } else if line.starts_with("version = \"")
            && let Some(name) = current.filter(|n| members.iter().any(|m| m == n))
        {
            if line != old_line {
                return Err(format!(
                    "Cargo.lock: `{name}` está com `{line}`, não com a versão {old}; rode um `cargo \
                     build` e confira o lock antes"
                ));
            }
            targets.push(i);
            found.push(name);
        }
    }
    let missing: Vec<&str> = members
        .iter()
        .map(String::as_str)
        .filter(|m| !found.contains(m))
        .collect();
    if !missing.is_empty() {
        return Err(format!("Cargo.lock sem os membros {}", missing.join(", ")));
    }
    if targets.len() != members.len() {
        return Err("Cargo.lock com membro do workspace repetido".into());
    }
    Ok(replace_lines(lock, &targets, old, new))
}

/// A tag da versão: `v` + a versão do `VERSION`.
pub fn check_tag(tag: &str, version: Version) -> Result<(), String> {
    let expected = format!("v{version}");
    if tag == expected {
        Ok(())
    } else {
        Err(format!(
            "a tag `{tag}` não é a versão do {VERSION_FILE} ({version}): esperava `{expected}`"
        ))
    }
}

/// Os três arquivos já com a versão nova.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub old: Version,
    pub new: Version,
    pub version_file: String,
    pub cargo_toml: String,
    pub cargo_lock: String,
}

/// Confere os três arquivos e monta o texto novo de cada um, sem gravar nada.
pub fn plan(
    version_text: &str,
    cargo_toml: &str,
    cargo_lock: &str,
    members: &[String],
    new: &str,
) -> Result<Plan, String> {
    let old = version_file(version_text)?;
    let workspace = workspace_version(cargo_toml)?;
    if workspace != old {
        return Err(format!(
            "{VERSION_FILE} ({old}) e Cargo.toml ({workspace}) fora de compasso: acerte os dois antes"
        ));
    }
    let new = parse(new)?;
    match new.cmp(&old) {
        Ordering::Greater => {}
        Ordering::Equal => return Err(format!("a versão já é {old}")),
        Ordering::Less => {
            return Err(format!(
                "a versão não desce: {new} é menor que a atual {old}"
            ));
        }
    }
    Ok(Plan {
        old,
        new,
        version_file: if version_text.ends_with('\n') {
            format!("{new}\n")
        } else {
            new.to_string()
        },
        cargo_toml: set_workspace_version(cargo_toml, old, new)?,
        cargo_lock: set_lock_versions(cargo_lock, members, old, new)?,
    })
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("não consegui ler {}: {e}", path.display()))
}

/// Grava num temporário da mesma pasta e renomeia por cima: uma falha no meio nunca deixa o arquivo
/// truncado (achado do revisor: o `fs::write` direto trunca antes de escrever).
fn write(path: &Path, text: &str) -> Result<(), String> {
    let failed = |e: std::io::Error| format!("não consegui gravar {}: {e}", path.display());
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.xtask-version"));
    if let Err(e) = fs::write(&tmp, text).and_then(|()| fs::rename(&tmp, path)) {
        let _ = fs::remove_file(&tmp);
        return Err(failed(e));
    }
    Ok(())
}

/// Grava os textos novos; se uma gravação ou a conferência `verify` falhar, devolve os originais
/// aos arquivos já gravados (achado do revisor: antes ficava meio gravado).
fn write_all(
    files: &[(&Path, &str, &str)],
    verify: &dyn Fn() -> Result<(), String>,
) -> Result<(), String> {
    let mut written = 0;
    let mut result = Ok(());
    for (path, _, new) in files {
        if let Err(e) = write(path, new) {
            result = Err(e);
            break;
        }
        written += 1;
    }
    if result.is_ok() {
        result = verify();
    }
    let Err(err) = result else {
        return Ok(());
    };
    let mut restore_errors = Vec::new();
    for (path, original, _) in files.iter().take(written) {
        if let Err(e) = write(path, original) {
            restore_errors.push(e);
        }
    }
    if restore_errors.is_empty() {
        Err(format!(
            "{err}\nnada mudou: os arquivos voltaram como estavam"
        ))
    } else {
        Err(format!(
            "{err}\ne não consegui devolver os originais ({}): confira com `git diff`",
            restore_errors.join("; ")
        ))
    }
}

/// `<x.y.z>` sobe a versão; `--check <tag>` confere a tag. `members`: os pacotes do workspace;
/// `verify`: a conferência depois de gravar (no xtask, o `cargo metadata --locked`).
pub fn run(
    args: &[OsString],
    root: &Path,
    members: &[String],
    verify: &dyn Fn() -> Result<(), String>,
) -> Result<(), String> {
    let args: Vec<String> = args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let version_path = root.join(VERSION_FILE);
    match args.as_slice() {
        [flag, tag] if flag == "--check" => {
            let version = version_file(&read(&version_path)?)?;
            check_tag(tag, version)?;
            println!("tag {tag} confere com o {VERSION_FILE}");
            Ok(())
        }
        [new] if !new.starts_with('-') => {
            let toml_path = root.join("Cargo.toml");
            let lock_path = root.join("Cargo.lock");
            let version_text = read(&version_path)?;
            let toml = read(&toml_path)?;
            let lock = read(&lock_path)?;
            let plan = plan(&version_text, &toml, &lock, members, new)?;
            write_all(
                &[
                    (&toml_path, &toml, &plan.cargo_toml),
                    (&lock_path, &lock, &plan.cargo_lock),
                    (&version_path, &version_text, &plan.version_file),
                ],
                verify,
            )?;
            println!(
                "versão {} → {}: {VERSION_FILE}, Cargo.toml ([workspace.package]) e Cargo.lock ({} \
                 membros do workspace)",
                plan.old,
                plan.new,
                members.len()
            );
            Ok(())
        }
        _ => Err("uso: cargo xtask version <x.y.z> | --check <tag>".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOML: &str = "[workspace]\nmembers = [\"a\"]\n\n[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace.dependencies]\nfoo = { version = \"0.1.0\" }\n";

    const LOCK: &str = "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.1.0\"\ndependencies = [\n \"lib\",\n]\n\n[[package]]\nname = \"lib\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"terceiro\"\nversion = \"0.1.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n";

    fn members() -> Vec<String> {
        vec!["app".into(), "lib".into()]
    }

    fn v(text: &str) -> Version {
        parse(text).unwrap()
    }

    /// As linhas que mudaram entre dois textos (o que a trava do plugin olha).
    fn changed(a: &str, b: &str) -> Vec<(String, String)> {
        assert_eq!(a.lines().count(), b.lines().count());
        a.lines()
            .zip(b.lines())
            .filter(|(x, y)| x != y)
            .map(|(x, y)| (x.to_owned(), y.to_owned()))
            .collect()
    }

    #[test]
    fn versao_valida_e_ordem() {
        assert_eq!(v("0.2.0").to_string(), "0.2.0");
        assert_eq!(v("10.0.12").to_string(), "10.0.12");
        assert!(v("0.10.0") > v("0.9.9"));
        assert!(v("1.0.0") > v("0.99.99"));
        for bad in [
            "",
            "0.3",
            "0.3.0.1",
            "v0.3.0",
            "0.3.0-beta",
            "0.03.0",
            "a.b.c",
            " 0.3.0",
            "0..1",
            "0.3.+1",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn arquivo_version_le_a_primeira_linha() {
        assert_eq!(version_file("0.2.0\n").unwrap(), v("0.2.0"));
        assert_eq!(version_file("0.2.0").unwrap(), v("0.2.0"));
        for bad in [
            "",
            "\n",
            "\n0.2.0\n",
            "0.2.0\nlixo\n",
            "0.2.0\n\n",
            " 0.2.0\n",
            "0.2.0 \n",
            "0.2.0\r\n",
        ] {
            assert!(version_file(bad).is_err(), "{bad:?}");
        }
        // Sem fim de linha, o arquivo novo também sai sem.
        let plan = plan("0.1.0", TOML, LOCK, &members(), "0.2.0").unwrap();
        assert_eq!(plan.version_file, "0.2.0");
    }

    #[test]
    fn sobe_a_versao_so_nas_linhas_dela() {
        let plan = plan("0.1.0\n", TOML, LOCK, &members(), "0.2.0").unwrap();
        assert_eq!(plan.version_file, "0.2.0\n");
        assert_eq!(
            changed(TOML, &plan.cargo_toml),
            [(
                "version = \"0.1.0\"".to_owned(),
                "version = \"0.2.0\"".to_owned()
            )]
        );
        // A dependência `foo = { version = "0.1.0" }` e o terceiro do lock ficam em 0.1.0.
        assert!(plan.cargo_toml.contains("foo = { version = \"0.1.0\" }"));
        let lock = changed(LOCK, &plan.cargo_lock);
        assert_eq!(lock.len(), 2, "{lock:?}");
        assert!(
            plan.cargo_lock
                .contains("name = \"terceiro\"\nversion = \"0.1.0\"")
        );
        assert!(plan.cargo_lock.ends_with("crates.io-index\"\n"));
    }

    #[test]
    fn fins_de_linha_recuo_e_espaco_no_fim_ficam() {
        let toml = TOML.replace('\n', "\r\n");
        let out = set_workspace_version(&toml, v("0.1.0"), v("0.2.0")).unwrap();
        assert!(out.contains("version = \"0.2.0\"\r\nedition"));
        assert_eq!(out.matches("\r\n").count(), toml.matches("\r\n").count());
        // Só o valor muda dentro da linha (achado do revisor: a linha era remontada).
        let toml = TOML.replace(
            "version = \"0.1.0\"\nedition",
            "\tversion = \"0.1.0\"  \nedition",
        );
        let out = set_workspace_version(&toml, v("0.1.0"), v("0.2.0")).unwrap();
        assert_eq!(
            changed(&toml, &out),
            [(
                "\tversion = \"0.1.0\"  ".to_owned(),
                "\tversion = \"0.2.0\"  ".to_owned()
            )]
        );
        assert!(set_workspace_version(TOML, v("0.3.0"), v("0.4.0")).is_err());
    }

    #[test]
    fn recusa_versao_que_nao_sobe_ou_invalida() {
        for new in ["0.1.0", "0.0.9", "0.3", "v0.3.0", "0.3.0-beta", ""] {
            assert!(
                plan("0.1.0\n", TOML, LOCK, &members(), new).is_err(),
                "{new:?}"
            );
        }
    }

    #[test]
    fn recusa_arquivos_fora_de_compasso_ou_lock_alheio() {
        let err = plan("0.2.0\n", TOML, LOCK, &members(), "0.3.0").unwrap_err();
        assert!(err.contains("0.2.0") && err.contains("0.1.0"), "{err}");
        // Membro que falta no lock, membro com outra versão, Cargo.toml sem a seção.
        let mut more = members();
        more.push("outro".into());
        assert!(plan("0.1.0\n", TOML, LOCK, &more, "0.2.0").is_err());
        let lock = LOCK.replacen(
            "name = \"lib\"\nversion = \"0.1.0\"",
            "name = \"lib\"\nversion = \"0.0.1\"",
            1,
        );
        assert!(plan("0.1.0\n", TOML, &lock, &members(), "0.2.0").is_err());
        assert!(
            plan(
                "0.1.0\n",
                "[package]\nversion = \"0.1.0\"\n",
                LOCK,
                &members(),
                "0.2.0"
            )
            .is_err()
        );
    }

    #[test]
    fn confere_a_tag() {
        assert!(check_tag("v0.2.0", v("0.2.0")).is_ok());
        assert!(check_tag("v0.3.0", v("0.2.0")).is_err());
        assert!(check_tag("0.2.0", v("0.2.0")).is_err());
        assert!(check_tag("v0.2.0-rc", v("0.2.0")).is_err());
    }

    #[test]
    fn run_grava_os_tres_arquivos_e_recusa_sem_gravar() {
        let dir =
            std::env::temp_dir().join(format!("hy-dbplus-xtask-version-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(VERSION_FILE), "0.1.0\n").unwrap();
        fs::write(dir.join("Cargo.toml"), TOML).unwrap();
        fs::write(dir.join("Cargo.lock"), LOCK).unwrap();
        let args = |a: &[&str]| a.iter().map(OsString::from).collect::<Vec<_>>();
        let ok = || Ok(());
        let fails = || Err("o cargo recusou".to_owned());
        assert!(run(&args(&["0.1.0"]), &dir, &members(), &ok).is_err());
        assert_eq!(fs::read_to_string(dir.join("Cargo.toml")).unwrap(), TOML);
        assert!(run(&args(&[]), &dir, &members(), &ok).is_err());
        assert!(run(&args(&["--check"]), &dir, &members(), &ok).is_err());
        // A conferência falhou depois de gravar: os três voltam como estavam.
        let err = run(&args(&["0.2.0"]), &dir, &members(), &fails).unwrap_err();
        assert!(
            err.contains("o cargo recusou") && err.contains("voltaram"),
            "{err}"
        );
        assert_eq!(
            fs::read_to_string(dir.join(VERSION_FILE)).unwrap(),
            "0.1.0\n"
        );
        assert_eq!(fs::read_to_string(dir.join("Cargo.toml")).unwrap(), TOML);
        assert_eq!(fs::read_to_string(dir.join("Cargo.lock")).unwrap(), LOCK);
        run(&args(&["0.2.0"]), &dir, &members(), &ok).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join(VERSION_FILE)).unwrap(),
            "0.2.0\n"
        );
        assert!(
            fs::read_to_string(dir.join("Cargo.toml"))
                .unwrap()
                .contains("version = \"0.2.0\"\nedition")
        );
        // Gravação que falha no segundo arquivo: o primeiro volta e o segundo nem foi tocado.
        let first = dir.join("primeiro");
        fs::write(&first, "antes").unwrap();
        let missing = dir.join("pasta-que-nao-existe").join("segundo");
        let err = write_all(&[(&first, "antes", "depois"), (&missing, "x", "y")], &ok).unwrap_err();
        assert!(err.contains("voltaram"), "{err}");
        assert_eq!(fs::read_to_string(&first).unwrap(), "antes");
        assert!(!dir.join(".primeiro.xtask-version").exists());
        assert!(run(&args(&["--check", "v0.2.0"]), &dir, &members(), &fails).is_ok());
        assert!(run(&args(&["--check", "v0.1.0"]), &dir, &members(), &ok).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    /// O portão pega o descompasso: o `VERSION` e o `Cargo.toml` reais da raiz, e cada membro do
    /// workspace no `Cargo.lock`, na mesma versão.
    #[test]
    fn version_cargo_toml_e_lock_da_raiz_em_compasso() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let file = version_file(&read(&root.join(VERSION_FILE)).unwrap()).unwrap();
        let workspace = workspace_version(&read(&root.join("Cargo.toml")).unwrap()).unwrap();
        assert_eq!(file, workspace, "VERSION e Cargo.toml fora de compasso");
        let lock = read(&root.join("Cargo.lock")).unwrap();
        let expected = format!("version = \"{file}\"");
        let mut name = "";
        let mut checked = 0;
        for line in lock.lines() {
            if let Some(n) = line
                .strip_prefix("name = \"")
                .and_then(|r| r.strip_suffix('"'))
            {
                name = n;
            } else if line.starts_with("version = \"")
                && (name.starts_with("hy-dbplus") || name == "xtask")
            {
                assert_eq!(line, expected, "Cargo.lock: {name}");
                checked += 1;
            }
        }
        assert!(checked >= 9, "membros do workspace no lock: {checked}");
    }
}
