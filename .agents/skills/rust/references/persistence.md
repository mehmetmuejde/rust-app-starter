# Persistence

## Port design

```rust
pub trait ItemRepository {
    fn insert(&mut self, item: &Item) -> Result<(), StorageError>; // Duplicate if id exists
    fn update(&mut self, item: &Item) -> Result<(), StorageError>; // Missing if id unknown
    fn delete(&mut self, id: ItemId) -> Result<bool, StorageError>;
    fn get(&self, id: ItemId) -> Result<Option<Item>, StorageError>;
    fn list(&self) -> Result<Vec<Item>, StorageError>;             // defined order
    fn describe(&self) -> String;                                  // for diagnostics
}
```

- One method = one unit of work = one transaction. Multi-step operations get their own port method
  (`move_items`), implemented atomically per backend. Do not leak transactions into the application.
- Synchronous by design; asynchronous backends use a memory mirror (see wasm.md) or live behind a server.
- Errors are typed (`Duplicate`, `Missing`, `Corrupt`, `Backend`, `NewerSchema`), never strings only.
- Domain types are stored via their validated constructors on read; invalid stored data → `Corrupt`,
  never a panic.

## Contract tests

A shared suite (`contract::run_all(&mut || Box::new(MyRepo::new()))`) defines the behaviour every
adapter must have. A new adapter is done when it passes the suite plus its own specifics (reopen,
migration, corruption, rollback).

## SQLite (native)

- `rusqlite` with `bundled`; WAL; `busy_timeout`; `foreign_keys = ON`; `STRICT` tables.
- Migrations: numbered SQL files embedded with `include_str!`, version in `PRAGMA user_version`, each in
  a transaction. Refuse databases with a newer version (never "repair" them). Never edit released
  migrations.
- Tests: `open_in_memory()` and `tempfile` directories.

## Browser

- In-memory repository + versioned JSON snapshot in IndexedDB, written after each change, loaded before
  the UI. Refuse newer/corrupt snapshots. Request `navigator.storage.persist()`.
- Grow path: per-record IndexedDB, or SQLite-in-wasm (`rusqlite` + `sqlite-wasm-rs`, OPFS only in a worker).

## PostgreSQL / shared data

- Only behind a server process. The browser and desktop talk to an API, never to the database.
- Server adapter: pool (`sqlx`/`deadpool-postgres`), migrations in the adapter crate, contract suite
  against a containerised test database in CI.

## Schema evolution checklist

1. New migration file (and snapshot version bump with backward-compatible reading in the browser).
2. Adapter mapping for new columns/fields.
3. Contract/regression tests for old data (open a database created with the previous version).
4. Document the change.
