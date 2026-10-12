# 13. CLI with clap, JSON output and stable exit codes

**Status:** Accepted (2026-10-11)

## Context

The CLI is a frontend for people, scripts and agents.

## Decision

- clap 4 (derive) with `--data-dir` (env `RUST_APP_DATA_DIR`) and `--json`.
- JSON envelope: `{"ok": true, "result": …}` / `{"ok": false, "error": {"code", "message"}}`.
- Exit codes: 0 success, 1 invalid request, 2 usage error, 3 storage/internal error.
- `exec` runs any `Command` JSON; destructive commands need `--allow-destructive`.
- `commands --schema` prints the command JSON Schema.

## Consequences

- Scripts and agents can rely on machine-readable output before MCP exists.
- Shell completion (`clap_complete`) is a small later addition.
