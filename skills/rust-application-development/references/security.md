# Security

## Input and data

- Validate at the domain boundary with constructors (`ItemName::parse`); deserialisation of commands
  goes through the same validation (`#[serde(try_from = "String")]`).
- Treat stored data as untrusted on read: map invalid values to `StorageError::Corrupt`, never panic.
- Use parameterised SQL only (`params![...]`); never format SQL with user input.
- Bound everything that comes from outside: string lengths, list sizes, recursion depth, file sizes.

## Destructive and privileged actions

- Every command has an `Effect`. Destructive commands require explicit confirmation in every frontend
  that acts for someone else (CLI `--allow-destructive`, UI confirm dialog, MCP `confirm: true`).
- Unknown/unclassified commands are treated as destructive.
- Agents must not pass confirmation flags on their own; a human approves the specific action.

## Network-facing hosts (server, MCP over HTTP, control ports)

- Bind to `127.0.0.1` by default; require a token for every request; reject non-JSON early; cap request
  size and connection count (e.g. 4 MiB per line, 16 connections).
- Authorise per `Effect` (read-only tokens vs. write tokens).
- No secrets in logs, URLs or error messages returned to clients.

## Secrets and configuration

- Secrets come from the environment or the OS keychain, never from the repository. Provide
  `.env.example` with placeholders only when a host actually reads configuration.
- Do not log full command payloads at `info` if they can contain personal data.

## Supply chain

- `cargo deny check`: allowed licenses, RustSec advisories, only crates.io as source, no wildcards.
- Pin the toolchain; pin GitHub Actions by commit SHA.
- New dependency → check maintenance, license, transitive tree size; prefer `default-features = false`.
- `unsafe` is forbidden; if ever needed, isolate it in one crate with `// SAFETY:` comments and tests.
