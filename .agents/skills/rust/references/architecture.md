# Architecture

## Layers

| Layer | Contains | Depends on | Must not contain |
|---|---|---|---|
| L0 domain | entities, value objects (validated newtypes), domain errors, pure functions | std, serde, thiserror, uuid, schemars | I/O, clocks, randomness sources other than id generation, async |
| L1 application | `App` service, `Command`/`Output`, `AppError`, `AppEvent`, ports (traits), job model, in-memory test doubles | L0 | UI, drivers, runtimes, `web-sys`, threads, `cfg(target_arch)` |
| L2 adapters | repository implementations, job runners, platform services | L0–L1 + their driver | UI crates, other adapters |
| L3 frontends | UI crates | L0–L1 + UI framework | storage, platform, threads |
| L4 hosts | `main` functions: choose adapters, build `App`, start the frontend | everything | logic |

Adapters and frontends are **siblings**: the UI never imports SQLite, a storage adapter never imports the
UI. Only the host knows both. Enforce it with a `cargo metadata` check (template: `cargo xtask layers`).

## The App service

```rust
pub struct App { repo: Box<dyn ItemRepository>, jobs: Box<dyn JobRunner>, clock: Box<dyn Clock>, /* … */ }
impl App {
    pub fn execute(&mut self, command: Command) -> Result<Output, AppError>; // the only entry point
    pub fn poll(&mut self);                                                  // collect job updates
    pub fn drain_events(&mut self) -> Vec<AppEvent>;                         // what changed
}
```

- **Single owner, synchronous.** Exactly one thread owns `App`. No `Arc<Mutex<App>>` in frontends that
  run on one thread; hosts that are multi-threaded (servers, Tauri) wrap it once at the host.
- **Dependency injection by constructor** with trait objects. Generic type parameters only if a hot
  path needs monomorphisation (measure first).
- **Ports only with real implementations.** A trait exists because there are two implementations
  (e.g. SQLite + browser + in-memory). No speculative interfaces.

## Commands

- One serialisable enum, adjacently tagged: `{"command": "create_item", "params": {"name": "…"}}`.
- Doc comment on each variant (becomes the JSON Schema description).
- Static metadata table (`COMMANDS`: name, label, effect, description) with a test that compares it to
  the enum's schema, so they cannot drift.
- `Effect::{Read, Write, Destructive}` per command; unknown → treat as destructive.
- `execute` validates via domain constructors (`ItemName::parse`), calls ports, pushes events,
  returns a typed `Output`.
- Read operations are commands too (`ListItems`, `GetItem`), so every frontend and agent has one interface.

Adding a command: variant → metadata → `execute` arm (+ `Output` variant) → tests → frontends → docs.

## Events

`AppEvent` values (`ItemCreated`, `JobChanged`, …) are queued by `App` and pulled by frontends.
They are also the hook for audit logs, undo/redo (record inverse commands), macros (replay
`Vec<Command>`) and automation. Keep them small (ids, not whole objects) unless a consumer needs data.

## Errors

- Domain: `DomainError` (caller's fault).
- Ports: `StorageError` (backend, corrupt, duplicate, missing, newer schema).
- Application: `AppError { Validation, NotFound, Storage }` with `code() -> &'static str` and
  `is_user_error()`. Frontends map codes to messages/exit codes; never parse error strings.

## Anti-patterns

- UI calling a repository, opening a file, spawning a thread.
- A "service" per frontend (`CliItemService`, `GuiItemService`).
- `cfg(target_arch = "wasm32")` in domain/application.
- Generic `Repository<T>` traits before there is a second entity.
- God-crates (`common`, `utils`) – put code where it is used.
