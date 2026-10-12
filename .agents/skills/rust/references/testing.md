# Testing

## Pyramid

1. **Domain unit tests** – every rule of every value object (boundaries, unicode, invalid input).
2. **Command tests** – `App` with in-memory repository, inline job runner and manual clock; assert
   outputs, error codes and emitted events.
3. **Contract tests** – one suite for all repository adapters.
4. **Adapter tests** – reopen/persistence, migrations, newer schema refused, corrupt data, rollback.
5. **Concurrency tests** – job runner with real threads: completion, cancellation, shutdown time.
6. **UI tests** – `egui_kittest` (or the framework's harness) drives the real UI headless.
7. **Binary smoke tests** – `assert_cmd` runs the CLI against a temp dir; check JSON and exit codes.
8. **Architecture tests** – layer check, wasm build, lints.

## Rules

- Deterministic: inject clocks and runners; no sleeps in core tests; real-thread tests use deadlines.
- Tests may `unwrap`; production code may not.
- Name tests after behaviour (`delete_unknown_is_not_found`).
- A bug fix starts with a failing regression test.
- Do not mock what you own: use the in-memory implementations of your ports.
- Verify, then claim: run the commands and report the real result; say explicitly what was not tested
  (e.g. "rendered output checked manually" or "not checked in a browser").
