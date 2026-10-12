# 8. trunk for the browser build

**Status:** Accepted (2026-10-11)

## Context

The web app needs wasm compilation, `wasm-bindgen`, an HTML entry point and a dev server. The
alternative is driving the wasm-bindgen CLI from xtask; the eframe template uses trunk.

## Decision

Use trunk (`apps/web/index.html`, wrapped by `cargo xtask web` and `cargo xtask web-build`). Release
builds use `[profile.web]` (fat LTO, `opt-level = "s"`, `panic = "abort"`). trunk downloads the
`wasm-bindgen` CLI matching `Cargo.lock`.

## Consequences

- No custom bundling code; hot reload during development.
- Developers need `cargo install --locked trunk`; CI installs it with `taiki-e/install-action`.
- Pre-compression and custom asset pipelines are left to the hosting setup.
