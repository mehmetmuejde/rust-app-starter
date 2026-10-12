# 12. `log` facade for logging

**Status:** Accepted (2026-10-11)

## Context

The core is synchronous (ADR 0001), so tracing spans across async tasks bring little. eframe and many
dependencies already use the `log` facade.

## Decision

Libraries log through `log`. Native hosts install `env_logger` (`RUST_LOG`, default `info`) via
`starter_platform::native::init_logging`; the browser host routes records to the console via
`starter_platform::web::init_logging`.

## Consequences

- Minimal dependencies, works on every target.
- Projects that add async services or need structured spans switch to `tracing` (it can consume `log`
  records via `tracing-log`), recorded in a new ADR.
