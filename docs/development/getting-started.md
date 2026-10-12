# Getting started

## Prerequisites

| Tool | Why | Install |
|---|---|---|
| rustup | toolchain from `rust-toolchain.toml` (Rust 1.99.0 + `wasm32-unknown-unknown`) is installed automatically on first `cargo` call | <https://rustup.rs> |
| trunk | browser build and dev server | `cargo install --locked trunk` |
| cargo-deny | license/advisory check (`cargo xtask deny`, part of `ci`) | `cargo install --locked cargo-deny` |

Linux desktop builds additionally need the usual windowing libraries at runtime (X11 or Wayland,
`libxkbcommon`); no system packages are needed to compile.

## First run

```bash
cargo xtask test                 # all tests
cargo xtask cli -- create "First item"
cargo xtask cli -- list
cargo xtask desktop              # opens the window; the item from the CLI is there
cargo xtask web                  # http://127.0.0.1:8080 – separate (browser) storage
```

Desktop and CLI share the same SQLite file in the per-user data directory:

| OS | Default location |
|---|---|
| macOS | `~/Library/Application Support/dev.rust-app-starter.rust-app/items.sqlite3` |
| Linux | `~/.local/share/rust-app/items.sqlite3` |
| Windows | `%APPDATA%\rust-app-starter\rust-app\data\items.sqlite3` |

Override with `--data-dir <dir>` or `RUST_APP_DATA_DIR`. Log level: `RUST_LOG=debug`.

## Everyday commands

```text
cargo xtask help
cargo xtask lint      # rustfmt check + clippy (native + wasm32), warnings are errors
cargo xtask layers    # dependency rules
cargo xtask wasm      # type-check browser crates for wasm32
cargo xtask ci        # everything CI runs – run before every commit
```

## Starting a new project from the template

1. Copy the repository (GitHub "Use this template" or clone without history).
2. Rename the crate prefix `starter-` and the binary names (`rust-app`, `rust-app-desktop`, `rust-app-web`)
   – a search/replace across `Cargo.toml` files, `xtask/src/layers.rs`, `xtask/src/main.rs`,
   `apps/web/index.html` and the docs. Update `APP_NAME`/`DB_NAME` constants in the hosts.
3. Replace `Item` with your first entity in `crates/domain`, adapt commands, repository and migrations.
4. Keep `docs/adr/`; add your own decisions from `0014` on (or restart numbering in a fresh repo).
5. Follow [adding-a-feature.md](adding-a-feature.md) for every feature.
