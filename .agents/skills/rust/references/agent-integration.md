# Agent integration

Agents must use the **same commands** as humans. Never build agent-only shortcuts into the core.

## Machine interface ladder

1. **CLI with JSON** (exists in the template): `rust-app commands --schema`, `rust-app exec '<json>' --json`,
   envelope `{"ok", "result" | "error": {"code", "message"}}`, stable exit codes, destructive guard.
2. **MCP server** (extension): stdio transport first (`rmcp`), later HTTP with token auth.
3. **REST/RPC** (extension): server host accepting `Command` JSON.

## MCP design

- One tool per command variant. Name = command name; description = variant doc comment; input schema =
  the variant's `params` schema from `schemars`.
- Annotations from `Effect`: `readOnlyHint` for `Read`, `destructiveHint` for `Destructive`,
  `idempotentHint` where true.
- Destructive tools need `confirm: true` *and* a user-side allow-list; otherwise return an error that
  explains what would happen.
- Resources for read-only views (e.g. `app://items`), refreshed from events.
- Long jobs: return the job id; offer `get_job`/`cancel_job`; use MCP progress notifications when
  supported.
- Headless by default (no UI needed); an "attach to the running desktop app" mode needs a local,
  authenticated control channel (e.g. JSON lines over loopback TCP).

## UI automation and screenshots

- Drive the real UI through its test harness (`egui_kittest`), not OS-level synthetic input.
- Screenshots: kittest with `wgpu` + `snapshot` features, or a headless CPU rasteriser.
- Use accessibility labels so automation can find widgets by name.

## Working rules for coding agents

- Run the full gate before declaring work done; report what was and was not verified.
- One feature per change; keep docs and ADRs in the same change.
- Parallel agents: separate worktrees, separate `CARGO_TARGET_DIR`, separate ports and data dirs.
- Ask the human before binding decisions: new crate, new dependency, storage format, public command
  changes, license exceptions.
