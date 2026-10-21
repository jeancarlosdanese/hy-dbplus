//! Tarefas de build e verificação do hy-dbplus, via `cargo xtask <comando>` (alias em
//! `.cargo/config.toml`). No M0 só `layers`; os demais entram quando houver o que verificar.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

mod layers;

type Task = fn(&[OsString]) -> Result<(), String>;

/// Cada subcomando: nome, resumo de uma linha, função.
const COMMANDS: &[(&str, &str, Task)] = &[(
    "layers",
    "Confere as camadas das crates (docs/BRIEF.md §5): nada abaixo da UI depende de egui",
    run_layers,
)];

fn main() -> ExitCode {
    // `args_os`: a versão `String` da `std` entra em pânico com byte inválido (CLAUDE.md).
    run(std::env::args_os().skip(1))
}

/// Decide pelo primeiro argumento; o resto vai para o comando.
fn run(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let args: Vec<OsString> = args.into_iter().collect();
    let Some(first) = args.first() else {
        print_help();
        return ExitCode::FAILURE;
    };
    let name = first.to_string_lossy();
    if matches!(name.as_ref(), "help" | "-h" | "--help") {
        print_help();
        return ExitCode::SUCCESS;
    }
    let Some((_, _, task)) = COMMANDS.iter().find(|(n, _, _)| *n == name) else {
        eprintln!("xtask: comando desconhecido `{name}`\n");
        print_help();
        return ExitCode::FAILURE;
    };
    match task(args.get(1..).unwrap_or_default()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("xtask {name}: {err}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    eprintln!("uso: cargo xtask <comando>\n\ncomandos:");
    for (name, about, _) in COMMANDS {
        eprintln!("  {name:<10} {about}");
    }
    eprintln!("  {:<10} esta lista", "help");
}

/// Raiz do workspace: a pasta acima de `xtask/`.
fn root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask sem pasta pai".to_owned())
}

/// `cargo metadata` do workspace, sem resolver o grafo das dependências externas: só os
/// membros e o que cada um declara, que é o que as camadas olham.
fn metadata() -> Result<serde_json::Value, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
        .current_dir(root()?)
        .output()
        .map_err(|e| format!("não consegui rodar o cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo metadata falhou:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("cargo metadata: JSON inválido: {e}"))
}

fn run_layers(_: &[OsString]) -> Result<(), String> {
    let crates = layers::from_metadata(&metadata()?)?;
    println!("Camadas (docs/BRIEF.md §5)\n");
    println!("{:<22} {:<16} dependências do workspace", "crate", "camada");
    for c in &crates {
        let ws: Vec<String> = c
            .deps
            .iter()
            .filter(|d| d.workspace)
            .map(|d| {
                format!(
                    "{}{}",
                    layers::short_name(&d.name),
                    if d.kind == layers::DepKind::Dev {
                        " (dev)"
                    } else {
                        ""
                    }
                )
            })
            .collect();
        println!(
            "{:<22} {:<16} {}",
            c.name,
            layers::describe(layers::classify(&c.name)),
            ws.join(", ")
        );
    }
    let violations = layers::check(&crates);
    println!();
    if violations.is_empty() {
        println!("OK: {} crates, nenhuma violação de camada.", crates.len());
        return Ok(());
    }
    for v in &violations {
        println!("  - {v}");
    }
    Err(format!("{} violação(ões) de camada", violations.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exit(args: &[&str]) -> ExitCode {
        run(args.iter().map(OsString::from))
    }

    #[test]
    fn ajuda_sai_com_zero_e_sem_comando_ou_desconhecido_com_um() {
        for args in [&["help"][..], &["-h"], &["--help"]] {
            assert_eq!(exit(args), ExitCode::SUCCESS, "{args:?}");
        }
        assert_eq!(exit(&[]), ExitCode::FAILURE);
        assert_eq!(exit(&["xyz"]), ExitCode::FAILURE);
    }

    #[cfg(unix)]
    #[test]
    fn argumento_que_nao_e_utf8_nao_entra_em_panico() {
        use std::os::unix::ffi::OsStringExt;
        // Antes: std::env::args() fazia unwrap na conversão e o processo morria com 101.
        assert_eq!(run([OsString::from_vec(vec![0xff])]), ExitCode::FAILURE);
    }
}
