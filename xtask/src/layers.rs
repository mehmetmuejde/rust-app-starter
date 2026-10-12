//! `cargo xtask layers`: enforces the dependency rules of `docs/architecture/dependency-rules.md`.
//!
//! Reads `cargo metadata` and checks every workspace crate against a layer table:
//! - a crate may depend only on workspace crates of the layers listed in [`allowed_layers`];
//! - framework and driver crates are banned below the layer that may use them;
//! - every workspace crate must be registered in [`LAYERS`].

use std::process::Command;

use serde_json::Value;

/// Layer of every workspace crate. Register new crates here.
const LAYERS: &[(&str, u8)] = &[
    ("starter-domain", 0),
    ("starter-application", 1),
    ("starter-platform", 2),
    ("starter-storage-sqlite", 2),
    ("starter-storage-browser", 2),
    ("starter-ui-egui", 3),
    ("starter-desktop", 4),
    ("starter-web", 4),
    ("starter-cli", 4),
];

/// Tooling outside the layer model.
const EXEMPT: &[&str] = &["xtask"];

/// UI frameworks: only the UI layer (3) and apps (4) may use them.
const UI_CRATES: &[&str] = &[
    "egui",
    "eframe",
    "egui_kittest",
    "egui-wgpu",
    "winit",
    "wgpu",
    "glow",
    "rfd",
    "dioxus",
    "tauri",
    "iced",
    "slint",
];

/// Drivers, runtimes and platform APIs: banned in the core (layers 0 and 1).
const INFRA_CRATES: &[&str] = &[
    "rusqlite",
    "sqlx",
    "postgres",
    "tokio-postgres",
    "diesel",
    "sea-orm",
    "redis",
    "tokio",
    "async-std",
    "reqwest",
    "wasm-bindgen",
    "wasm-bindgen-futures",
    "web-sys",
    "js-sys",
    "directories",
    "env_logger",
];

/// Workspace layers a crate of `layer` may depend on. Adapters (2) and UI (3) are siblings:
/// both build on the core only, never on each other.
fn allowed_layers(layer: u8) -> &'static [u8] {
    match layer {
        0 => &[],
        1 => &[0],
        2 | 3 => &[0, 1],
        _ => &[0, 1, 2, 3],
    }
}

pub struct Package {
    pub name: String,
    pub deps: Vec<String>,
}

pub fn run() -> Result<(), String> {
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(crate::workspace_root())
        .output()
        .map_err(|e| format!("cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let metadata: Value = serde_json::from_slice(&output.stdout).map_err(|e| format!("cargo metadata JSON: {e}"))?;
    let packages = parse(&metadata);
    let violations = check(&packages);
    if violations.is_empty() {
        eprintln!("xtask: layers ok ({} crates)", packages.len());
        Ok(())
    } else {
        Err(format!(
            "{} layer violation(s):\n  {}",
            violations.len(),
            violations.join("\n  ")
        ))
    }
}

fn parse(metadata: &Value) -> Vec<Package> {
    let empty = Vec::new();
    metadata["packages"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .map(|p| Package {
            name: p["name"].as_str().unwrap_or_default().to_owned(),
            deps: p["dependencies"]
                .as_array()
                .unwrap_or(&empty)
                .iter()
                .filter(|d| d["kind"].as_str() != Some("build"))
                .filter_map(|d| d["name"].as_str().map(str::to_owned))
                .collect(),
        })
        .collect()
}

fn layer_of(name: &str) -> Option<u8> {
    LAYERS.iter().find(|(n, _)| *n == name).map(|(_, l)| *l)
}

pub fn check(packages: &[Package]) -> Vec<String> {
    let mut violations = Vec::new();
    for package in packages.iter().filter(|p| !EXEMPT.contains(&p.name.as_str())) {
        let Some(layer) = layer_of(&package.name) else {
            violations.push(format!(
                "{}: not registered in xtask/src/layers.rs LAYERS",
                package.name
            ));
            continue;
        };
        for dep in &package.deps {
            if dep == &package.name {
                continue;
            }
            if let Some(dep_layer) = layer_of(dep) {
                if !allowed_layers(layer).contains(&dep_layer) {
                    violations.push(format!(
                        "{} (layer {layer}) must not depend on {dep} (layer {dep_layer})",
                        package.name
                    ));
                }
            } else if UI_CRATES.contains(&dep.as_str()) && layer < 3 {
                violations.push(format!(
                    "{} (layer {layer}) must not depend on UI crate {dep}",
                    package.name
                ));
            } else if INFRA_CRATES.contains(&dep.as_str()) && layer < 2 {
                violations.push(format!(
                    "{} (layer {layer}) must not depend on infrastructure crate {dep}",
                    package.name
                ));
            }
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(name: &str, deps: &[&str]) -> Package {
        Package {
            name: name.into(),
            deps: deps.iter().map(|d| (*d).into()).collect(),
        }
    }

    #[test]
    fn valid_graph_passes() {
        let packages = [
            pkg("starter-domain", &["serde"]),
            pkg("starter-application", &["starter-domain"]),
            pkg("starter-ui-egui", &["starter-application", "egui"]),
            pkg(
                "starter-desktop",
                &["starter-ui-egui", "starter-storage-sqlite", "eframe"],
            ),
            pkg("xtask", &["anything"]),
        ];
        assert!(check(&packages).is_empty());
    }

    #[test]
    fn upward_dependency_fails() {
        let violations = check(&[pkg("starter-domain", &["starter-application"])]);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ui_must_not_use_storage() {
        let violations = check(&[pkg("starter-ui-egui", &["starter-storage-sqlite"])]);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn core_must_not_use_ui_or_drivers() {
        let violations = check(&[pkg("starter-application", &["egui", "rusqlite", "tokio"])]);
        assert_eq!(violations.len(), 3);
    }

    #[test]
    fn unregistered_crate_fails() {
        assert_eq!(check(&[pkg("starter-new", &[])]).len(), 1);
    }
}
