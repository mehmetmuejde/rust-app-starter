# Persistence

All persistence goes through the `ItemRepository` port (`crates/application/src/ports.rs`). Every
adapter must pass the shared contract suite `starter_application::contract::run_all`.

## Native: SQLite (`crates/storage-sqlite`)

- `rusqlite` 0.40 with bundled SQLite (no system library needed).
- File: `<data dir>/items.sqlite3`, WAL mode, `busy_timeout` 5 s, foreign keys on.
- Every repository call is one statement in autocommit mode (an implicit transaction).
- Multiple processes (desktop + CLI) may use the file at the same time; SQLite serialises writers.

### Migrations

- SQL files in `crates/storage-sqlite/migrations/`, embedded with `include_str!` and listed in
  `MIGRATIONS` in order. File *n* produces schema version *n* (`PRAGMA user_version`).
- On open, missing migrations run, each in its own transaction.
- A database with a **higher** version than the build knows is refused (`StorageError::NewerSchema`)
  and left untouched – downgrades never corrupt data.
- Never edit a released migration; add a new file. Tables use `STRICT` typing.

### Transactions across several operations

When a command needs several writes atomically, add a method to the repository port that performs the
whole unit of work (e.g. `move_items(...)`) and implement it with `conn.transaction()` in SQLite and
as a single snapshot in the browser adapter. Do not expose raw transactions to the application layer.

### Test databases

- Unit/contract tests: `SqliteItemRepository::open_in_memory()`.
- File-based tests: `tempfile::tempdir()` + `SqliteItemRepository::open`.

## Browser: memory + IndexedDB (`crates/storage-browser`)

- Working state: `InMemoryItemRepository`.
- After every successful change a JSON snapshot `{"version": 1, "items": [...]}` is written to
  IndexedDB database `rust-app`, object store `snapshots`, key `items`. Writes are issued immediately
  and IndexedDB applies them in order.
- Loaded once before the UI starts. A snapshot from a newer version or with invalid content is refused
  (error shown on the loading screen) instead of being overwritten.
- If issuing the write fails synchronously, the in-memory change is rolled back.
- Limits: the last change can be lost if the tab closes within milliseconds; each change rewrites the
  whole snapshot (fine for thousands of records). Beyond that: one IndexedDB record per entity, or
  SQLite in wasm (`rusqlite` supports `wasm32-unknown-unknown` via `sqlite-wasm-rs`; needs a
  wasm-capable clang and a Web Worker for OPFS persistence).

## Server-side: PostgreSQL (extension)

Not implemented. The concept (server host, `storage-postgres` adapter, pooling, migrations, the browser
talking to an API instead of a database) is in [extensions.md](../architecture/extensions.md#server-and-postgresql).
