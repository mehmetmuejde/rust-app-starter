# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Local agent notes for this project (this file is not committed; see the global git ignore).

## Decisions recorded for agents

- **AI files:** `CLAUDE.md`, `.claude/` etc. stay uncommitted (default).
- **Exception (2026-10-11, decided by Mehmet):** `skills/rust-application-development/` is a product
  deliverable and IS versioned in this repo (ADR 0011). It is not covered by the global ignore list,
  so no `.gitignore` negation is needed.
- ADRs live in `docs/adr/` (not `docs/decisions/` as the master prompt suggested).

## Working rules

- Follow `skills/rust-application-development/SKILL.md`.
- Gate before every commit: `cargo xtask ci` (fmt, clippy native+wasm, tests, layers, wasm, cargo-deny).
- New crate → register in `xtask/src/layers.rs` (and `WASM_CRATES` in `xtask/src/main.rs` if shared with the browser).
- Never commit or push without Mehmet's explicit approval. Repo: github.com/mehmetmuejde/rust-app-starter (public, branch main).

## Commands

All automation is plain Rust in `xtask/` (alias in `.cargo/config.toml`), so it behaves the same on every OS.

- `cargo xtask ci` – full gate: lint, test, layers, wasm, deny
- `cargo xtask lint` / `test` / `layers` / `wasm` / `deny` – the individual gate steps
- `cargo xtask cli -- <args>` – run the CLI, e.g. `cargo xtask cli -- list`
- `cargo xtask desktop` – native window; `cargo xtask web` – browser app on http://127.0.0.1:8080 (needs `trunk`)
- Single test: `cargo test -p starter-application <test_name>` (any crate: `-p starter-<crate>`)
- `cargo-deny` and `trunk` must be installed (`cargo install --locked cargo-deny trunk`); `xtask` fails with a hint if missing.

The toolchain is pinned in `rust-toolchain.toml` (includes the `wasm32-unknown-unknown` target); edition 2024.

## Architecture (needs several files to see)

One application core, three thin hosts (`apps/desktop`, `apps/web`, `apps/cli`). Layers, enforced by `cargo xtask layers`
(`xtask/src/layers.rs` holds the layer table and the banned-crate lists):

- L0 `domain` – entities and rules, no I/O
- L1 `application` – `App::execute(Command) -> Result<Output, AppError>`, events, jobs, ports (`ItemRepository`, `JobRunner`, `Clock`)
- L2 adapters – `storage-sqlite`, `storage-browser` (memory + IndexedDB), `platform` (threads/paths natively, inline jobs in the browser)
- L3 `ui-egui` – one `Shell` shared by desktop and browser
- L4 `apps/*` – composition roots that only wire adapters into `App`

L2 and L3 are siblings and must not depend on each other; L0/L1 must not pull in UI, DB drivers, async runtimes or web APIs.

Key mechanics:
- **Every action is a typed `Command`** (serde + JSON Schema) – button, CLI args, `rust-app exec --json` and tests all go through `App::execute`. Never add a second path to the same logic.
- **`App` is synchronous with a single owner** (ADR 0001). Long work is a job handed to a `JobRunner`; results return via `App::poll`, which emits `AppEvent`s that the UI drains.
- **Ports have contract tests** (`crates/application/src/contract.rs`, feature `contract-tests`); every storage adapter must run `contract::run_all`.
- **Shared crates must compile for wasm32** – keep `cfg(target_arch = "wasm32")` and OS APIs in adapters/hosts, never in the core.
- Production code forbids `unwrap`/`expect`/`panic!`/`todo!`/`unsafe`; tests may use them (`clippy.toml`).

Deeper docs: `docs/architecture/`, `docs/development/` (e.g. `adding-a-feature.md`), and the 15 ADRs in `docs/adr/`.
