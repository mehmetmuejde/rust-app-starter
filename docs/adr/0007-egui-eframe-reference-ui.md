# 7. egui/eframe as the reference UI, swappable at the command boundary

**Status:** Accepted (2026-10-11)

## Context

The kit must ship a working desktop and browser UI without binding the architecture to one framework.
egui/eframe 0.36 compiles to native and wasm from one code base.

## Decision

- `starter-ui-egui` depends on `egui` only and talks to the core exclusively via `App::execute` and events.
- Desktop host: eframe with the wgpu renderer. Browser host: eframe with glow (WebGL2) for the widest
  browser support and the smallest bundle; WebGPU is a later option.
- No generic widget abstraction. Other frameworks (Dioxus, Iced, Slint, Tauri) get their own UI crate on
  the same `App`/`Command` boundary (see `docs/architecture/platform-abstraction.md`).

## Consequences

- One UI implementation for desktop and web.
- egui is 0.x: minor upgrades may break the UI crate; upgrades are deliberate.
- egui 0.36 requires Rust ≥ 1.95 (toolchain pinned in `rust-toolchain.toml`).
