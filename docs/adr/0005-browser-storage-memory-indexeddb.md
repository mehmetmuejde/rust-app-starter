# 5. Browser persistence: in-memory state mirrored to IndexedDB

**Status:** Accepted (2026-10-11)

## Context

Browser storage (IndexedDB, OPFS) is asynchronous; the core is synchronous (ADR 0001). Options were
an in-memory repository with snapshots, SQLite compiled to wasm (`rusqlite` + `sqlite-wasm-rs`, needs a
wasm-capable clang and a Web Worker for OPFS), or localStorage (synchronous but ≈ 5 MB).

## Decision

`starter-storage-browser` keeps all items in an `InMemoryItemRepository`. After every successful change
it writes a JSON snapshot (`{"version": 1, "items": [...]}`) to IndexedDB. The snapshot is loaded before
the UI starts. A newer or unreadable snapshot is refused. The host requests persistent storage.

## Consequences

- Pure Rust (raw `web-sys`), no extra JS, works on the main thread.
- A change becomes durable a few milliseconds after `insert` returns; closing the tab in that window can
  lose it.
- Each write rewrites the snapshot: fine for thousands of records. Larger data sets should switch to one
  IndexedDB record per entity or to SQLite-in-wasm (new ADR).
- The browser never talks to PostgreSQL; shared data goes through a server API.
