# CLI

Binary `rust-app` (`apps/cli`). Same `App`, same SQLite file as the desktop app. The CLI is a standalone
program: the desktop app does not need to be running, the CLI opens the database itself.

## Running it

During development, without installing anything:

```bash
cargo xtask cli -- list               # everything after `--` goes to the CLI
```

To use it as a regular `rust-app` command from any directory:

```bash
cargo install --locked --path apps/cli    # builds a release binary into ~/.cargo/bin
rust-app list
cargo uninstall starter-cli               # removes it again
```

`~/.cargo/bin` must be on `PATH` (rustup sets this up). Re-run `cargo install` after code changes – the
installed binary does not update itself.

## Commands

```bash
rust-app create "Groceries"
rust-app list
rust-app get <id>
rust-app rename <id> "New name"
rust-app delete <id>
rust-app info
rust-app report                       # runs the report job, progress on stderr
rust-app commands                     # all commands with effect
rust-app commands --schema            # JSON Schema of the command format
rust-app exec '{"command":"list_items"}'
echo '{"command":"info"}' | rust-app exec -
rust-app exec '{"command":"delete_item","params":{"id":"…"}}' --allow-destructive
```

## Options

| Option | Env | Meaning |
|---|---|---|
| `--data-dir <dir>` | `RUST_APP_DATA_DIR` | database directory |
| `--json` | – | JSON envelope on stdout |
| – | `RUST_LOG` | log level (stderr) |

## JSON output and exit codes

```json
{"ok":true,"result":{"kind":"item","data":{"id":"…","name":"Groceries","created_at":1760000000000,"updated_at":1760000000000}}}
{"ok":false,"error":{"code":"validation","message":"the name must not be empty"}}
```

| Exit code | Meaning | Error codes |
|---|---|---|
| 0 | success | – |
| 1 | invalid request | `validation`, `not_found`, `invalid_command`, `destructive_refused` |
| 2 | usage error (clap) | – |
| 3 | storage or internal error | `storage`, `internal` |

## For agents

`exec` + `--json` + `commands --schema` form a complete machine interface: discover commands from the
schema, execute JSON, parse the envelope, branch on `error.code`. Destructive commands require an
explicit flag – an agent should only pass it after a human approved the specific action.

## Shell completion (extension)

Add `clap_complete` and a `completions <shell>` subcommand when needed.
