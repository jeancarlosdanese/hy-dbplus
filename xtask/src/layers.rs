//! Camadas do workspace (docs/BRIEF.md §5), conferidas por `cargo xtask layers`.
//!
//! O motor de regras trabalha sobre um modelo pequeno e independente do cargo, para ser
//! testado sem ele; `from_metadata` monta esse modelo a partir de `cargo metadata --no-deps`.
//! Estrutura inspirada no `xtask/src/layers.rs` do PdfCraft, reduzida ao que este workspace tem.
//!
//! Regras:
//!   1. crate de camada N só depende de crate do workspace de camada menor que N (nada na
//!      mesma camada: `automation` e `ui-egui` não se conhecem);
//!   2. crate de UI (`egui*`, `eframe`, `winit`, `wgpu*`, ...) só em `ui-egui` e nas isentas
//!      (apps e xtask), inclusive como dependência de teste ou de build;
//!   3. crate de camada não depende de crate isenta (app ou xtask): o sentido é só o inverso;
//!   4. toda crate do workspace está na tabela; crate nova é decisão registrada aqui.

use serde_json::Value;

/// Classe de uma crate do workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Crate de camada N, sem UI.
    Layer(u8),
    /// Crate de camada N que pode usar os toolkits de UI (só a `ui-egui`).
    Ui(u8),
    /// Apps e ferramentas de build: fora das regras.
    Exempt,
}

impl Class {
    fn layer(self) -> Option<u8> {
        match self {
            Class::Layer(l) | Class::Ui(l) => Some(l),
            Class::Exempt => None,
        }
    }
}

/// Toda crate do workspace e sua classe, pelo nome curto (`short_name`): o pacote
/// `hy-dbplus-engine` é `engine`, o app `hy-dbplus-cli` é `cli`, o app `hy-dbplus` fica.
pub const TABLE: &[(&str, Class)] = &[
    ("driver", Class::Layer(0)),
    ("catalog", Class::Layer(1)),
    ("sql", Class::Layer(2)),
    ("engine", Class::Layer(3)),
    ("automation", Class::Layer(4)),
    ("ui-egui", Class::Ui(4)),
    // apps/ e xtask/: fora das regras
    ("hy-dbplus", Class::Exempt),
    ("cli", Class::Exempt),
    ("xtask", Class::Exempt),
];

/// Crates externas que são toolkit de UI ou janela (`*` no fim casa por prefixo).
pub const UI_CRATES: &[&str] = &[
    "egui*",
    "eframe",
    "winit",
    "wgpu*",
    "glow",
    "accesskit*",
    "rfd",
];

/// Nome curto: `hy-dbplus-engine` vira `engine`; os apps e o `xtask` ficam como estão.
pub fn short_name(pkg: &str) -> &str {
    pkg.strip_prefix("hy-dbplus-").unwrap_or(pkg)
}

pub fn classify(pkg: &str) -> Option<Class> {
    let short = short_name(pkg);
    TABLE.iter().find(|(n, _)| *n == short).map(|(_, c)| *c)
}

fn is_ui_crate(name: &str) -> bool {
    UI_CRATES.iter().any(|p| match p.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == *p,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

#[derive(Debug, Clone)]
pub struct Dep {
    pub name: String,
    pub kind: DepKind,
    /// A dependência é outra crate deste workspace.
    pub workspace: bool,
}

#[derive(Debug, Clone)]
pub struct Crate {
    pub name: String,
    pub deps: Vec<Dep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    Unregistered {
        krate: String,
    },
    Upward {
        krate: String,
        dep: String,
        from: u8,
        to: u8,
        kind: DepKind,
    },
    UiOutsideUi {
        krate: String,
        dep: String,
        kind: DepKind,
    },
    OnExempt {
        krate: String,
        dep: String,
        kind: DepKind,
    },
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Violation::Unregistered { krate } => write!(
                f,
                "{krate}: crate fora da tabela; registre a camada em xtask/src/layers.rs (docs/BRIEF.md §5)"
            ),
            Violation::Upward {
                krate,
                dep,
                from,
                to,
                kind,
            } => write!(
                f,
                "{krate} (camada {from}) -> {dep} (camada {to}) [{kind:?}]: só pode depender de camada menor"
            ),
            Violation::OnExempt { krate, dep, kind } => write!(
                f,
                "{krate} depende de `{dep}` [{kind:?}], que é app ou ferramenta de build: só o inverso é permitido"
            ),
            Violation::UiOutsideUi { krate, dep, kind } => write!(
                f,
                "{krate} depende da crate de UI `{dep}` [{kind:?}]: egui e afins só em ui-egui, nos apps e no xtask"
            ),
        }
    }
}

/// Aplica as regras ao grafo; a lista volta ordenada e sem repetição.
pub fn check(crates: &[Crate]) -> Vec<Violation> {
    let mut out = Vec::new();
    for c in crates {
        let Some(class) = classify(&c.name) else {
            out.push(Violation::Unregistered {
                krate: c.name.clone(),
            });
            continue;
        };
        let Some(layer) = class.layer() else {
            continue; // isenta
        };
        for d in &c.deps {
            if d.workspace {
                let to = match classify(&d.name) {
                    Some(Class::Exempt) => {
                        out.push(Violation::OnExempt {
                            krate: c.name.clone(),
                            dep: d.name.clone(),
                            kind: d.kind,
                        });
                        continue;
                    }
                    Some(dep_class) => dep_class.layer().unwrap_or(u8::MAX),
                    None => continue, // fora da tabela: já apontada na própria crate
                };
                if to >= layer {
                    out.push(Violation::Upward {
                        krate: c.name.clone(),
                        dep: d.name.clone(),
                        from: layer,
                        to,
                        kind: d.kind,
                    });
                }
            } else if class != Class::Ui(layer) && is_ui_crate(&d.name) {
                out.push(Violation::UiOutsideUi {
                    krate: c.name.clone(),
                    dep: d.name.clone(),
                    kind: d.kind,
                });
            }
        }
    }
    out.sort_by_key(ToString::to_string);
    out.dedup();
    out
}

/// Modelo a partir de `cargo metadata --format-version 1 --no-deps`: só os membros do
/// workspace entram, com todas as suas dependências declaradas.
pub fn from_metadata(meta: &Value) -> Result<Vec<Crate>, String> {
    let pkgs = meta
        .get("packages")
        .and_then(Value::as_array)
        .ok_or("metadata sem `packages`")?;
    let members: Vec<&str> = pkgs
        .iter()
        .filter_map(|p| p.get("name").and_then(Value::as_str))
        .collect();
    let mut out = Vec::new();
    for p in pkgs {
        let name = p
            .get("name")
            .and_then(Value::as_str)
            .ok_or("pacote sem `name`")?
            .to_owned();
        let mut deps = Vec::new();
        for d in p
            .get("dependencies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let dname = d
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let kind = match d.get("kind").and_then(Value::as_str) {
                Some("dev") => DepKind::Dev,
                Some("build") => DepKind::Build,
                _ => DepKind::Normal,
            };
            let workspace = members.contains(&dname.as_str());
            deps.push(Dep {
                name: dname,
                kind,
                workspace,
            });
        }
        out.push(Crate { name, deps });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn describe(class: Option<Class>) -> String {
    match class {
        Some(Class::Layer(l)) => format!("camada {l}"),
        Some(Class::Ui(l)) => format!("camada {l} (UI)"),
        Some(Class::Exempt) => "isenta".into(),
        None => "FORA DA TABELA".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::DepKind::*;
    use super::*;

    fn c(name: &str, deps: &[(&str, DepKind, bool)]) -> Crate {
        Crate {
            name: name.into(),
            deps: deps
                .iter()
                .map(|(n, k, w)| Dep {
                    name: (*n).into(),
                    kind: *k,
                    workspace: *w,
                })
                .collect(),
        }
    }

    #[test]
    fn o_grafo_do_brief_passa() {
        let g = [
            c("hy-dbplus-driver", &[("tokio-postgres", Normal, false)]),
            c("hy-dbplus-catalog", &[("hy-dbplus-driver", Normal, true)]),
            c(
                "hy-dbplus-sql",
                &[
                    ("hy-dbplus-catalog", Normal, true),
                    ("sqlparser", Normal, false),
                ],
            ),
            c(
                "hy-dbplus-engine",
                &[
                    ("hy-dbplus-driver", Normal, true),
                    ("hy-dbplus-catalog", Normal, true),
                    ("hy-dbplus-sql", Normal, true),
                ],
            ),
            c(
                "hy-dbplus-automation",
                &[("hy-dbplus-engine", Normal, true)],
            ),
            c(
                "hy-dbplus-ui-egui",
                &[
                    ("hy-dbplus-engine", Normal, true),
                    ("egui", Normal, false),
                    ("eframe", Normal, false),
                    ("egui_kittest", Dev, false),
                ],
            ),
            c(
                "hy-dbplus",
                &[
                    ("hy-dbplus-ui-egui", Normal, true),
                    ("eframe", Normal, false),
                ],
            ),
            c("hy-dbplus-cli", &[("hy-dbplus-automation", Normal, true)]),
            c("xtask", &[("serde_json", Normal, false)]),
        ];
        assert_eq!(check(&g), vec![]);
    }

    #[test]
    fn dependencia_para_cima_reprova() {
        let v = check(&[c("hy-dbplus-driver", &[("hy-dbplus-engine", Normal, true)])]);
        assert!(
            matches!(v[..], [Violation::Upward { from: 0, to: 3, .. }]),
            "{v:?}"
        );
        let v = check(&[c("hy-dbplus-catalog", &[("hy-dbplus-sql", Dev, true)])]);
        assert!(
            matches!(
                v[..],
                [Violation::Upward {
                    from: 1,
                    to: 2,
                    kind: Dev,
                    ..
                }]
            ),
            "{v:?}"
        );
    }

    #[test]
    fn mesma_camada_reprova_nos_dois_sentidos() {
        let v = check(&[c(
            "hy-dbplus-automation",
            &[("hy-dbplus-ui-egui", Normal, true)],
        )]);
        assert!(
            matches!(v[..], [Violation::Upward { from: 4, to: 4, .. }]),
            "{v:?}"
        );
        let v = check(&[c(
            "hy-dbplus-ui-egui",
            &[("hy-dbplus-automation", Normal, true)],
        )]);
        assert!(
            matches!(v[..], [Violation::Upward { from: 4, to: 4, .. }]),
            "{v:?}"
        );
    }

    #[test]
    fn crate_de_ui_fora_da_ui_egui_reprova_mesmo_em_teste() {
        for dep in [
            "egui",
            "eframe",
            "winit",
            "egui_extras",
            "egui_kittest",
            "wgpu",
            "wgpu-core",
            "glow",
            "accesskit_winit",
            "rfd",
        ] {
            let v = check(&[c("hy-dbplus-engine", &[(dep, Normal, false)])]);
            assert!(
                matches!(v[..], [Violation::UiOutsideUi { .. }]),
                "{dep}: {v:?}"
            );
        }
        let v = check(&[c("hy-dbplus-sql", &[("egui_kittest", Dev, false)])]);
        assert!(
            matches!(v[..], [Violation::UiOutsideUi { kind: Dev, .. }]),
            "{v:?}"
        );
        let v = check(&[c("hy-dbplus-driver", &[("winit", Build, false)])]);
        assert!(
            matches!(v[..], [Violation::UiOutsideUi { kind: Build, .. }]),
            "{v:?}"
        );
    }

    #[test]
    fn crate_de_ui_na_ui_egui_e_nos_apps_passa() {
        assert_eq!(
            check(&[c(
                "hy-dbplus-ui-egui",
                &[("egui", Normal, false), ("egui_kittest", Dev, false)]
            )]),
            vec![]
        );
        for app in ["hy-dbplus", "hy-dbplus-cli", "xtask"] {
            assert_eq!(
                check(&[c(
                    app,
                    &[("egui", Normal, false), ("hy-dbplus-ui-egui", Normal, true)]
                )]),
                vec![]
            );
        }
    }

    #[test]
    fn crate_de_camada_que_depende_de_app_ou_xtask_reprova() {
        // Hoje inalcançável (apps e xtask são só binário), mas um app com lib.rs abriria o caminho.
        let v = check(&[c("hy-dbplus-engine", &[("hy-dbplus", Normal, true)])]);
        assert!(matches!(v[..], [Violation::OnExempt { .. }]), "{v:?}");
        let v = check(&[c("hy-dbplus-driver", &[("xtask", Dev, true)])]);
        assert!(
            matches!(v[..], [Violation::OnExempt { kind: Dev, .. }]),
            "{v:?}"
        );
    }

    #[test]
    fn crate_fora_da_tabela_reprova() {
        let v = check(&[c("hy-dbplus-misterio", &[])]);
        assert!(
            matches!(&v[..], [Violation::Unregistered { krate }] if krate == "hy-dbplus-misterio")
        );
    }

    #[test]
    fn from_metadata_le_so_o_que_precisa() {
        let meta = serde_json::json!({
            "packages": [
                {"name": "hy-dbplus-engine", "dependencies": [
                    {"name": "hy-dbplus-driver", "kind": null},
                    {"name": "egui_kittest", "kind": "dev"},
                    {"name": "cc", "kind": "build"}
                ]},
                {"name": "hy-dbplus-driver", "dependencies": []}
            ]
        });
        let crates = from_metadata(&meta).unwrap();
        assert_eq!(crates.len(), 2);
        assert_eq!(crates[0].name, "hy-dbplus-driver");
        let engine = &crates[1];
        assert_eq!(engine.deps.len(), 3);
        assert!(engine.deps[0].workspace && engine.deps[0].kind == Normal);
        assert!(!engine.deps[1].workspace && engine.deps[1].kind == Dev);
        assert_eq!(engine.deps[2].kind, Build);
        assert!(from_metadata(&serde_json::json!({})).is_err());
    }
}
