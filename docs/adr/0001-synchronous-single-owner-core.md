# 1. Synchronous application core with a single owner

**Status:** Accepted (2026-10-11)

## Context

The core must run unchanged on desktop, in the CLI and in the browser (`wasm32-unknown-unknown`).
An async core (tokio, async repository traits, SQLx) is natural for servers but brings problems on the
other targets: async traits are not dyn-compatible without boxing, futures must be `Send` natively but
are `!Send` in the browser, SQLx has no wasm support, and immediate-mode UIs (egui) need a runtime on a
side thread plus channels. A single-owner session with detached jobs avoids all of this and scales
to large desktop applications.

## Decision

- `App` is synchronous and has exactly one owner (UI thread, CLI main thread, test). No `Arc<Mutex<App>>`.
- Short operations run directly in `App::execute`.
- Long or CPU-heavy work leaves the core as a self-contained job (see ADR 0006); results return through
  `App::poll` and are applied on the owner thread.
- tokio is allowed only in future hosts that need network I/O (server, MCP over HTTP). Such a host owns
  `App` on a dedicated thread (actor) or calls it via `spawn_blocking`.

## Consequences

- One code path for every target; tests are plain `#[test]` functions.
- Asynchronous backends (IndexedDB) need an in-memory mirror (ADR 0005).
- A future server cannot run many requests against one `App` in parallel; it serialises them through
  the owner. Revisit when a project needs multi-user throughput on one process.
