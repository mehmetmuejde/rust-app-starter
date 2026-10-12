---
name: rust-application-development
description: Apply when building, extending or reviewing a Rust application with a UI-independent core — desktop (egui/eframe, Iced, Slint, Tauri, Dioxus), browser (WebAssembly) and/or CLI frontends in one Cargo workspace. Covers layering and crate boundaries, typed commands as the single entry point, ports and adapters for storage (SQLite, IndexedDB, PostgreSQL), background jobs vs. async vs. threads, desktop/wasm compatibility, testing, security of agent-facing interfaces and the development workflow. Built around the rust-app-starter template but usable in any Rust application workspace.
---

# Rust application development

Rules and workflow for coding agents working on multi-frontend Rust applications. The reference
implementation is the **rust-app-starter** template; every rule here is implemented there. In other
projects, apply the rules and adapt names.

## When to use

- Adding a feature, entity, command, job, adapter or frontend to a Rust application workspace.
- Setting up a new Rust application (desktop + web + CLI) or reviewing one.
- Deciding where code belongs (which crate/layer) or whether a new crate/dependency is justified.

Not for: single-binary tools without a domain core, embedded/no_std work, pure library crates.

## Architecture in one picture

```text
L4 apps/*            composition roots: wire adapters into App, nothing else
L3 ui-*              frontends; depend on the core only
L2 adapters          storage-*, platform; depend on the core only (siblings of the UI)
L1 application       App::execute(Command) -> Result<Output, AppError>, events, ports, jobs
L0 domain            entities, value objects, rules – pure Rust, no I/O
```

Details: [references/architecture.md](references/architecture.md).

## Binding rules

1. **No business logic in UI code.** UIs translate user actions into commands and render outputs.
2. **No database or driver dependency in domain/application.** Persistence goes through ports.
3. **Isolate platform code.** `cfg(target_arch = "wasm32")`, OS APIs, threads and file paths live in
   adapters and hosts, never in the core.
4. **Reuse existing commands and services.** Every frontend (GUI, CLI, agent, test) goes through the same
   `App::execute`. Never add a second path to the same logic.
5. **New crates only with a reason:** a second implementation of a port, a different target, a
   heavy optional dependency, or a real size/compile-time problem. Otherwise add a module.
6. **Document public APIs** (`///` on every public item that is not self-explanatory; crate-level `//!`).
7. **Handle errors explicitly:** `thiserror` enums in libraries with stable codes, `anyhow` only in
   binaries; no stringly-typed errors across crate boundaries.
8. **No `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/`unreachable!` in production code.**
   Enforced by crate-root `#![deny(...)]`; tests may use them (`clippy.toml`).
9. **Measure performance before optimising;** record numbers in docs/ADRs.
10. **Every feature and every bug fix comes with tests** (bug fix: regression test first).
11. **Shared code must compile for desktop and wasm32** (`cargo xtask wasm`).
12. **No framework dependencies in the core** – no UI, async runtime, DB driver or web API in L0/L1.

Additionally: no `unsafe` (workspace `forbid`), no new dependency without a license check
(`cargo deny check`), no layer violation (`cargo xtask layers`).

## Standard workflow

1. **Understand the requirement.** Restate it; identify the domain concepts involved.
2. **Identify affected crates** using the layer table. Most features touch domain → application →
   adapter(s) → frontend(s), in that order.
3. **Check the architecture:** does an existing command/service/port cover it? Is a new crate or
   dependency really needed (rules 4, 5, 12)? Is it platform-specific (rule 3)?
4. **Write a short plan** (files, commands, migrations, tests). Ask the human for decisions that bind
   the project (new dependency, new crate, storage format change, public command change).
5. **Implement inside-out:** domain rules → command + `execute` arm → port/adapter changes (+ migration)
   → frontends.
6. **Add tests** at the level where the logic lives (see [references/testing.md](references/testing.md)).
7. **Run the gate:** `cargo fmt`, `cargo clippy -D warnings`, `cargo test` – in the template:
   `cargo xtask ci` (also layers, wasm check, cargo-deny).
8. **Check platforms:** wasm build for shared crates; run the desktop app / browser app after UI
   changes and look at the result.
9. **Update documentation:** command table, guides, ADR for binding decisions.
10. **Summarise:** what changed, what was verified (and how), what was not verified, open questions.
    Never claim a build or test passed without having run it.

## How-to index

| Task | Reference |
|---|---|
| Add a crate, workspace settings, lints, profiles | [cargo-workspaces.md](references/cargo-workspaces.md) |
| Add a command / service method / event | [architecture.md](references/architecture.md#commands) |
| Add or swap a UI framework, egui patterns | [ui-development.md](references/ui-development.md) |
| Desktop vs. browser differences, wasm pitfalls | [wasm.md](references/wasm.md) |
| Add a repository, migration, new storage backend | [persistence.md](references/persistence.md) |
| Background jobs, threads, rayon, tokio | [async-and-jobs.md](references/async-and-jobs.md) |
| Test strategy and patterns | [testing.md](references/testing.md) |
| Input validation, secrets, destructive actions, supply chain | [security.md](references/security.md) |
| CLI/MCP/agent interfaces, JSON Schema | [agent-integration.md](references/agent-integration.md) |

## Quick recipes (rust-app-starter)

- **New command:** variant in `Command` → entry in `COMMANDS` → arm in `App::execute` → test in
  `app.rs` → frontend wiring. `exec` in the CLI works immediately.
- **New entity field:** domain + migration file + adapter mapping + contract test + snapshot version
  handling in the browser adapter.
- **New job:** `JobRequest` variant → `JobTask` impl with short steps → build it in `App::start_job` →
  apply data-changing results in `App::poll`.
- **New frontend:** `crates/ui-<fw>` at L3 + host in `apps/<name>`; register in `xtask/src/layers.rs`.
- **New crate:** register in `LAYERS` (`xtask/src/layers.rs`), add to `WASM_CRATES` if browser-shared,
  inherit workspace package fields and lints, add the never-crash `#![deny]` block.
