# 4. SQLite via rusqlite for native persistence

**Status:** Accepted (2026-10-11)

## Context

Desktop and CLI need a local, zero-setup database. Candidates: `rusqlite` (sync, mature) and `sqlx`
(async, compile-time checked queries, also PostgreSQL). The core is synchronous (ADR 0001).

## Decision

- `starter-storage-sqlite` implements `ItemRepository` with `rusqlite` 0.40 (`bundled` SQLite).
- WAL journal mode, 5 s busy timeout, foreign keys on.
- Migrations: embedded SQL files, tracked with `PRAGMA user_version`, one transaction each.
  A database from a newer build is refused, never modified.
- PostgreSQL is not implemented; it belongs behind a server (see `docs/architecture/extensions.md`).

## Consequences

- No runtime, no connection pool needed for a single-user file.
- SQL is written by hand and covered by the repository contract tests.
- A server/PostgreSQL extension will choose its own driver (likely SQLx) in a new ADR.
