//! Project automation, run as `cargo xtask <command>` (alias in `.cargo/config.toml`).
//!
//! Plain Rust instead of shell scripts, so every command works the same on Windows, macOS and Linux.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

mod layers;

use std::{
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const USAGE: &str = "\
Usage: cargo xtask <command> [args]

Run
  desktop [-- ARGS]   run the desktop app (debug)
  cli [-- ARGS]       run the CLI, e.g. `cargo xtask cli -- list`
  web                 serve the browser app with hot reload on http://127.0.0.1:8080 (trunk)
  web-build           release build of the browser app into apps/web/dist (trunk)
  docker-web          container image `rust-app-web` of the browser app (apps/web/Dockerfile)

Check
  test                run all tests
  lint                rustfmt check + clippy (native and wasm32), warnings are errors
  layers              check the crate dependency rules (docs/architecture/dependency-rules.md)
  wasm                type-check the browser crates for wasm32-unknown-unknown
  deny                licenses, advisories and bans (cargo-deny)
  ci                  everything CI runs: lint, test, layers, wasm, deny
";

/// Crates that must compile for the browser.
const WASM_CRATES: &[&str] = &[
    "starter-domain",
    "starter-application",
    "starter-platform",
    "starter-storage-browser",
    "starter-ui-egui",
    "starter-web",
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (command, rest) = match args.split_first() {
        Some((command, rest)) => (command.as_str(), rest),
        None => {
            print!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let passthrough = rest
        .iter()
        .skip_while(|a| a.as_str() != "--")
        .skip(1)
        .cloned()
        .collect::<Vec<_>>();
    let result = match command {
        "desktop" => cargo(&[&["run", "-p", "starter-desktop", "--"], &strs(&passthrough)[..]].concat()),
        "cli" => cargo(&[&["run", "-q", "-p", "starter-cli", "--"], &strs(&passthrough)[..]].concat()),
        "web" => trunk(&["serve"]),
        "web-build" => trunk(&["build", "--release"]),
        "docker-web" => run_in(
            &workspace_root(),
            "docker",
            &["build", "-f", "apps/web/Dockerfile", "-t", "rust-app-web", "."],
        ),
        "test" => test(),
        "lint" => lint(),
        "layers" => layers::run(),
        "wasm" => wasm(),
        "deny" => deny(),
        "ci" => ci(),
        "help" | "-h" | "--help" => {
            print!("{USAGE}");
            Ok(())
        }
        other => Err(format!("unknown command '{other}'\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}

fn strs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

fn workspace_root() -> PathBuf {
    // xtask lives in <root>/xtask
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn run_in(dir: &Path, program: &str, args: &[&str]) -> Result<(), String> {
    eprintln!("$ {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|e| format!("could not start {program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed ({status})", args.join(" ")))
    }
}

fn cargo(args: &[&str]) -> Result<(), String> {
    run_in(
        &workspace_root(),
        &std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()),
        args,
    )
}

fn trunk(args: &[&str]) -> Result<(), String> {
    if !tool_available("trunk") {
        return Err("trunk is not installed: cargo install --locked trunk".into());
    }
    run_in(&workspace_root().join("apps/web"), "trunk", args)
}

fn tool_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn test() -> Result<(), String> {
    cargo(&["test", "--workspace", "--locked"])
}

fn lint() -> Result<(), String> {
    cargo(&["fmt", "--all", "--check"])?;
    cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--",
        "-D",
        "warnings",
    ])?;
    let mut args = vec!["clippy", "--target", "wasm32-unknown-unknown", "--locked"];
    for krate in WASM_CRATES {
        args.extend(["-p", krate]);
    }
    args.extend(["--", "-D", "warnings"]);
    cargo(&args)
}

fn wasm() -> Result<(), String> {
    let mut args = vec!["check", "--target", "wasm32-unknown-unknown", "--locked"];
    for krate in WASM_CRATES {
        args.extend(["-p", krate]);
    }
    cargo(&args)
}

fn deny() -> Result<(), String> {
    if !tool_available("cargo-deny") {
        return Err("cargo-deny is not installed: cargo install --locked cargo-deny".into());
    }
    cargo(&["deny", "check"])
}

fn ci() -> Result<(), String> {
    lint()?;
    test()?;
    layers::run()?;
    wasm()?;
    deny()?;
    eprintln!("xtask: ci passed");
    Ok(())
}
