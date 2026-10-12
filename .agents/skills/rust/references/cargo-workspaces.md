# Cargo workspaces

## Root `Cargo.toml`

- `resolver = "3"`, `members = ["crates/*", "apps/*", "xtask"]`.
- `[workspace.package]`: version, edition 2024, license, `rust-version`, `publish = false`.
- `[workspace.dependencies]`: every dependency version once; crates use `{ workspace = true }` and add
  features locally.
- `[workspace.lints]`: `unsafe_code = "forbid"`, clippy `dbg_macro`/`todo = "warn"`.
- Profiles: `[profile.dev.package."*"] opt-level = 2` (fast deps in debug), a size-optimised `[profile.web]`
  (`lto = "fat"`, `opt-level = "s"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`).

## Crate checklist

```toml
[package]
name = "starter-<name>"          # one prefix for the whole workspace → easy rename
description = "<one sentence: responsibility>"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
publish.workspace = true

[lints]
workspace = true
```

Crate root:

```rust
//! What this crate is for, what it must not do.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo,
        clippy::unimplemented, clippy::unreachable)]
```

`clippy.toml`: `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-panic-in-tests = true`.
(The never-crash lints are crate attributes, not workspace lints, because workspace lints would also hit
integration tests and examples.)

Then: register the crate in the layer table, add it to the wasm check list if it is shared with the
browser.

## Target-specific dependencies

```toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
directories = { workspace = true }

[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { workspace = true, features = ["Window"] }
```

Matching modules: `#[cfg(not(target_arch = "wasm32"))] pub mod native;` /
`#[cfg(target_arch = "wasm32")] pub mod web;`. A host binary that only makes sense on one target gets a
stub `main` on the other (prints a hint, exits 2) so `cargo build --workspace` keeps working.

## Features

- Features are additive; never use a feature to switch between incompatible implementations.
- Test-support code for other crates: a `contract-tests`-style feature enabled only as dev-dependency,
  plus `#[cfg(any(test, feature = "..."))]` on the module.

## Toolchain and tooling

- `rust-toolchain.toml` pins the exact version, components (`rustfmt`, `clippy`) and targets
  (`wasm32-unknown-unknown`). Bump deliberately; MSRV of UI frameworks moves fast (egui 0.36 → 1.95).
- `xtask` pattern: a workspace binary + alias `xtask = "run --package xtask --"` in `.cargo/config.toml`.
  Plain Rust, cross-platform, no shell scripts. Keep its dependencies minimal.
- `cargo deny check` for licenses/advisories/sources; exceptions per crate with a reason.
