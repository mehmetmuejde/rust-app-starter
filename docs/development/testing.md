# Testing

| Level | Where | Example |
|---|---|---|
| Domain unit tests | `crates/domain/src/*` | name validation, timestamp formatting |
| Command tests | `crates/application/src/app.rs` | `App` + `InMemoryItemRepository` + `InlineJobRunner` + `ManualClock` |
| Repository contract | `crates/application/src/contract.rs` | run by every adapter: memory, SQLite, browser |
| Adapter specifics | `crates/storage-*/src/lib.rs` | migrations, newer schema refused, corrupt data, rollback |
| Concurrency | `crates/platform/src/native.rs` | jobs on threads, cancellation, shutdown |
| UI | `crates/ui-egui/src/views.rs` | `egui_kittest` drives the real `Shell` headless |
| CLI smoke | `apps/cli/tests/cli.rs` | runs the binary (`assert_cmd`) against a temp dir: JSON, exit codes |
| Architecture | `xtask/src/layers.rs` + `cargo xtask layers` | dependency rules |
| Browser build | `cargo xtask wasm`, `cargo xtask web-build` | compiles for wasm32 |

Rules:
- Deterministic tests: inject `ManualClock`, use `InlineJobRunner`, no sleeps in the core.
- Tests may `unwrap`/`expect`/`panic` (allowed by `clippy.toml`); production code may not.
- A new adapter must call `contract::run_all` (enable the `contract-tests` feature as dev-dependency).
- A bug fix comes with a regression test.

Not automated: rendering correctness of the desktop window and the browser canvas. Check them by running
the apps (`cargo xtask desktop`, `cargo xtask web`) after UI changes. For pixel tests, enable
`egui_kittest`'s `wgpu` + `snapshot` features.
