# 2. Typed command enum as the frontend contract

**Status:** Accepted (2026-10-11)

## Context

Every frontend (egui, CLI, tests, later MCP/REST) must reach the same logic. Dispatching string ids
with untyped `serde_json::Value` params would leave tools with free-form schemas and force every
command to validate its params by hand.

## Decision

- One `Command` enum, serialised adjacently tagged: `{"command": "create_item", "params": {...}}`.
- `App::execute(Command) -> Result<Output, AppError>` is the only entry point.
- `COMMANDS` holds static metadata (name, label, `Effect::{Read, Write, Destructive}`, description).
  A unit test keeps it in sync with the enum's JSON Schema.
- `schemars` generates the JSON Schema of `Command` for tooling and agents.

## Consequences

- Invalid input fails at deserialisation with a precise message; no hand-written param parsing.
- Agents get real input schemas; `Effect` drives confirmation (`--allow-destructive`, MCP hints).
- Adding a command touches three places (variant, metadata, `execute` arm); the compiler and a test
  enforce two of them.
