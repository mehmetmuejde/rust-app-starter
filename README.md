# Rust App Starter

A small, working foundation for Rust applications that run as **desktop app**, **browser app
(WebAssembly)** and **CLI** – from one code base, with one application core.

- **Rust-first:** domain, application logic, storage and UI are Rust.
- **One core, three frontends:** egui desktop (eframe + wgpu), egui in the browser (WebGL2), clap CLI –
  all execute the same typed `Command`s through `App::execute`.
- **Swappable edges:** SQLite natively, IndexedDB in the browser, thread pool or inline job runner –
  behind small ports with real implementations and a shared contract test suite.
- **Enforced architecture:** `cargo xtask layers` fails the build when the core depends on UI, drivers
  or platform APIs.
- **Agent-ready:** JSON command format with JSON Schema, `rust-app exec --json`, destructive commands
  guarded; MCP is a documented extension.
- **Batteries for development:** pinned toolchain, never-crash lints, 50+ tests (unit, contract, UI,
  CLI smoke), cargo-deny, GitHub Actions on Linux/macOS/Windows + wasm.

The example domain is a deliberately simple *item manager* (create, rename, delete, list, report job).
Replace it with your own; keep the structure.

## Quick start

```bash
# prerequisites: rustup (toolchain is installed from rust-toolchain.toml), plus
cargo install --locked trunk cargo-deny

cargo xtask cli -- create "First item"
cargo xtask cli -- list
cargo xtask desktop          # native window
cargo xtask web              # http://127.0.0.1:8080
cargo xtask ci               # full quality gate
```

The CLI can also be installed as a standalone `rust-app` command:
`cargo install --locked --path apps/cli` (see [CLI](docs/development/cli.md)).

More: [Getting started](docs/development/getting-started.md).

## Architecture

One application core, three thin hosts. Dependencies only point downwards; `cargo xtask layers` fails
the build otherwise.

```text
 ┌──────────────────────────────────────────────────────────────────────────────┐
 │ L4  apps/           composition roots – create adapters, hand them to App    │
 │     desktop          web                         cli                         │
 │     eframe + wgpu    eframe + WebGL2 (trunk)     clap, --json, exec          │
 └────────┬───────────────────────────────┬─────────────────────────────────────┘
          │ GUI apps                      │ all apps
 ┌────────▼─────────────────────┐ ┌───────▼─────────────────────────────────────┐
 │ L3  ui-egui                  │ │ L2  adapters                                │
 │     Shell: items, jobs,      │ │     storage-sqlite    SQLite (desktop, cli) │
 │     about, status bar        │ │     storage-browser   memory + IndexedDB    │
 │     knows no storage         │ │     platform          threads, paths, logs  │
 └────────┬─────────────────────┘ └───────┬─────────────────────────────────────┘
          │ sends Command                 │ implements ports
 ┌────────▼───────────────────────────────▼─────────────────────────────────────┐
 │ L1  application     App::execute(Command) → Result<Output, AppError>         │
 │                     events · jobs · ports: ItemRepository, JobRunner, Clock  │
 ├──────────────────────────────────────────────────────────────────────────────┤
 │ L0  domain          Item, ItemName, Timestamp – rules only, no I/O           │
 └──────────────────────────────────────────────────────────────────────────────┘

 desktop = ui-egui + storage-sqlite  + platform (thread pool)
 web     = ui-egui + storage-browser + platform (inline jobs, console log)
 cli     =           storage-sqlite  + platform (thread pool)
```

Every action takes the same path, whether it comes from a button, the CLI, JSON from an agent or a
test:

```text
 button · CLI args · JSON · test
              │
              ▼
          Command  (typed enum, serde, JSON Schema)
              │
              ▼
        App::execute ──▶ domain rules ──▶ ItemRepository ──▶ SQLite │ IndexedDB │ memory
              │
              ├──▶ Result<Output, AppError>   back to the caller
              └──▶ AppEvent                   drained by the UI (refresh, messages)

 long work:  StartJob ──▶ JobRunner (threads │ inline) ──▶ App::poll ──▶ JobChanged events
```

### What is included

- **Persistence:** SQLite with migrations (desktop, CLI – they share one file) and memory + IndexedDB
  snapshot (browser). Both implement the same `ItemRepository` port and pass the same contract tests.
  PostgreSQL is a documented extension behind a server. → [database](docs/development/database.md)
- **UI:** egui/eframe, one `Shell` for desktop and browser. Iced, Slint, Dioxus and Tauri are documented
  as alternatives with their integration pattern. → [platform abstraction](docs/architecture/platform-abstraction.md)
- **Background jobs:** step-based jobs with progress and cancellation; a thread pool natively, inline
  slices per frame in the browser. → [background jobs](docs/development/background-jobs.md)
- **CLI and agents:** every command as JSON via `rust-app exec`, JSON Schema via `rust-app commands
  --schema`, destructive commands need an explicit flag. → [cli](docs/development/cli.md)
- **Docker:** container image for the browser app (static files on unprivileged nginx).
  → [webassembly](docs/development/webassembly.md#docker)
- **Authentication:** not needed for local, single-user data; the concept for server and network
  access is documented. → [extensions](docs/architecture/extensions.md#authentication-and-authorization)

## Layout

```text
crates/
  domain/            L0  entities, value objects, rules – pure Rust
  application/       L1  App, Command/Output, events, ports, jobs, in-memory repo, contract tests
  platform/          L2  native thread job runner, data dirs, logging; browser helpers
  storage-sqlite/    L2  ItemRepository on SQLite (rusqlite, migrations)
  storage-browser/   L2  ItemRepository on memory + IndexedDB snapshot
  ui-egui/           L3  egui Shell (depends on egui only)
apps/
  desktop/           L4  eframe native host        → rust-app-desktop
  web/               L4  eframe web host (trunk)   → rust-app-web
  cli/               L4  clap host                 → rust-app
xtask/                   cargo xtask: run, lint, layers, wasm, deny, ci
docs/                    architecture, development guides, ADRs
.agents/skills/rust/      rules for coding agents (skill)
```

## Documentation

| Topic | Document |
|---|---|
| Architecture overview | [docs/architecture/overview.md](docs/architecture/overview.md) |
| Dependency rules | [docs/architecture/dependency-rules.md](docs/architecture/dependency-rules.md) |
| Commands | [docs/architecture/commands.md](docs/architecture/commands.md) |
| Platform abstraction & swapping the UI | [docs/architecture/platform-abstraction.md](docs/architecture/platform-abstraction.md) |
| Extensions (server/PostgreSQL, workers, MCP, GPU) | [docs/architecture/extensions.md](docs/architecture/extensions.md) |
| Desktop · Web · CLI | [desktop](docs/development/desktop.md) · [webassembly](docs/development/webassembly.md) · [cli](docs/development/cli.md) |
| Persistence · Jobs · Testing | [database](docs/development/database.md) · [background-jobs](docs/development/background-jobs.md) · [testing](docs/development/testing.md) |
| Adding a feature | [docs/development/adding-a-feature.md](docs/development/adding-a-feature.md) |
| Decisions | [docs/adr/](docs/adr/README.md) |
| Agent skill | [.agents/skills/rust/SKILL.md](.agents/skills/rust/SKILL.md) |

## Status

| Area | State |
|---|---|
| Desktop (macOS verified; Windows/Linux built in CI) | working |
| Browser (wasm32, trunk) | builds; manual browser check |
| CLI | working, smoke-tested |
| SQLite / IndexedDB persistence | working, contract-tested |
| Local background jobs (threads / inline) | working |
| Docker image of the browser app | builds and serves locally |
| Authentication | not needed locally; concept for network hosts documented |
| PostgreSQL, server, distributed workers, MCP, GPU compute, other UI frameworks | documented extension points |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as
above, without any additional terms or conditions.
