//! hy-dbplus-cli: o hy-dbplus sem janela. No M8 roda a tabela de tools da `automation`
//! e o servidor MCP opt-in; no M0 só imprime o uso e a versão.
//!
//! Argumentos chegam como `OsString` (`args_os`): a versão `String` da `std` entra em pânico
//! com um byte que não é UTF-8, e o binário é a superfície mais exposta a shell e agentes.

use std::ffi::OsString;
use std::process::ExitCode;

const USAGE: &str = "\
uso: hy-dbplus-cli [--help | --version]

Ainda sem comandos. No M8 (docs/BRIEF.md §7) entram `tools` (lista as tools e seus
schemas JSON), `run <tool>` (executa uma tool) e `mcp` (servidor MCP em stdin/stdout, opt-in).
";

fn main() -> ExitCode {
    run(std::env::args_os().skip(1))
}

/// Decide pelo primeiro argumento; o resto é ignorado até haver comandos.
fn run(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let first = args.into_iter().next();
    match first.as_deref().and_then(|a| a.to_str()) {
        None if first.is_none() => {
            print!("hy-dbplus-cli {}\n\n{USAGE}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("help" | "-h" | "--help") => {
            print!("hy-dbplus-cli {}\n\n{USAGE}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--version" | "-V") => {
            println!("hy-dbplus-cli {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            let shown = first
                .map(|a| a.to_string_lossy().into_owned())
                .unwrap_or_default();
            eprintln!("hy-dbplus-cli: comando desconhecido `{shown}`\n\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exit(args: &[&str]) -> ExitCode {
        run(args.iter().map(OsString::from))
    }

    #[test]
    fn sem_argumento_ajuda_e_versao_saem_com_zero() {
        for args in [
            &[][..],
            &["--help"],
            &["-h"],
            &["help"],
            &["--version"],
            &["-V"],
        ] {
            assert_eq!(exit(args), ExitCode::SUCCESS, "{args:?}");
        }
    }

    #[test]
    fn comando_desconhecido_sai_com_um() {
        assert_eq!(exit(&["tools"]), ExitCode::FAILURE);
        assert_eq!(exit(&["xyz", "--help"]), ExitCode::FAILURE);
    }

    #[cfg(unix)]
    #[test]
    fn argumento_que_nao_e_utf8_nao_entra_em_panico() {
        use std::os::unix::ffi::OsStringExt;
        // Antes: std::env::args() fazia unwrap na conversão e o processo morria com 101.
        let arg = OsString::from_vec(vec![0xff]);
        assert_eq!(run([arg]), ExitCode::FAILURE);
    }
}
