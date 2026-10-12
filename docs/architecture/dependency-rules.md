# Dependency rules

Checked by `cargo xtask layers` (in `cargo xtask ci` and the CI `lint` job). Source of truth:
`xtask/src/layers.rs`.

| Layer | Crates | May depend on (workspace) | Banned external crates |
|---|---|---|---|
| L0 | `starter-domain` | – | UI frameworks, drivers, runtimes, web APIs |
| L1 | `starter-application` | L0 | UI frameworks, drivers, runtimes, web APIs |
| L2 | `starter-platform`, `starter-storage-sqlite`, `starter-storage-browser` | L0, L1 | UI frameworks |
| L3 | `starter-ui-egui` | L0, L1 | – |
| L4 | `starter-desktop`, `starter-web`, `starter-cli` | L0–L3 | – |
| – | `xtask` | exempt | – |

- **UI frameworks:** `egui`, `eframe`, `egui_kittest`, `egui-wgpu`, `winit`, `wgpu`, `glow`, `rfd`,
  `dioxus`, `tauri`, `iced`, `slint`.
- **Infrastructure:** `rusqlite`, `sqlx`, `postgres`, `tokio-postgres`, `diesel`, `sea-orm`, `redis`,
  `tokio`, `async-std`, `reqwest`, `wasm-bindgen`, `wasm-bindgen-futures`, `web-sys`, `js-sys`,
  `directories`, `env_logger`.

## Why adapters and UI are siblings

L2 and L3 may both use only the core. The UI therefore cannot reach into SQLite, and a storage adapter
cannot call UI code. Only the host (L4) knows both and wires them together. Replacing egui or SQLite
touches one crate plus the host's `main`.

## Adding a crate

1. Ask whether a module in an existing crate is enough (see the skill's rule 5).
2. Create it under `crates/<name>`, inherit `[workspace.package]` fields and `[lints] workspace = true`,
   add the never-crash `#![deny(...)]` block to its root.
3. Register it in `LAYERS` in `xtask/src/layers.rs`. Unregistered crates fail the check.
4. If it must run in the browser, add it to `WASM_CRATES` in `xtask/src/main.rs`.

## Other enforced rules

| Rule | Mechanism |
|---|---|
| No `unsafe` | `[workspace.lints.rust] unsafe_code = "forbid"` |
| No panicking shortcuts in production code | crate-root `#![deny(clippy::unwrap_used, expect_used, panic, todo, unimplemented, unreachable)]`; `clippy.toml` allows them in tests |
| No warnings | `cargo xtask lint` runs clippy with `-D warnings` for native and wasm32 |
| Licenses, advisories, sources | `cargo xtask deny` (`deny.toml`) |
| Reproducible toolchain | `rust-toolchain.toml` pins 1.99.0 + wasm32 target |
