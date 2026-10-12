# Commands

`Command` (`crates/application/src/command.rs`) is the single contract between frontends and the core.

## Wire format

```json
{"command": "create_item", "params": {"name": "Groceries"}}
{"command": "list_items"}
{"command": "start_job", "params": {"job": {"kind": "generate_report", "step_delay_ms": 0}}}
```

Results (`Output`) are serialised as `{"kind": "...", "data": ...}`. The full JSON Schema:
`rust-app commands --schema`.

| Command | Effect | Output |
|---|---|---|
| `create_item { name }` | write | `item` |
| `rename_item { id, name }` | write | `item` |
| `delete_item { id }` | destructive | `deleted` |
| `list_items` | read | `items` |
| `get_item { id }` | read | `item` |
| `info` | read | `info` |
| `start_job { job }` | write | `job_started` |
| `cancel_job { id }` | write | `cancel_requested` |
| `list_jobs` | read | `jobs` |

## Errors

`AppError` has a stable `code()`: `validation` (domain rule broken), `not_found`, `storage`.
`is_user_error()` separates caller mistakes from infrastructure failures (CLI exit code 1 vs. 3).

## Effects and confirmation

`Effect::{Read, Write, Destructive}` lives in `COMMANDS`. Frontends acting on behalf of someone else
must confirm destructive commands: the CLI's `exec` requires `--allow-destructive`, the egui UI asks
"Delete? Yes/No", and an MCP server would map `Destructive` to `destructiveHint` plus an explicit
confirmation parameter. Unknown metadata is treated as destructive.

## Events

After state changes, `App` queues `AppEvent`s (`item_created`, `item_updated`, `item_deleted`,
`job_changed`). Frontends call `drain_events()` and refresh. Events are also the hook for
audit logs, undo/redo, macros and automation:

- **Audit log:** store every executed `Command` with its result and timestamp – they are serialisable.
- **Undo/redo:** record the inverse command for each write (inverse-command model).
- **Macros/automation:** a macro is a `Vec<Command>`; replay it through `execute`.

## Adding a command

1. Add the variant to `Command` (with a doc comment – it becomes the schema description).
2. Add a `CommandInfo` to `COMMANDS` (`catalogue_matches_enum` fails otherwise).
3. Handle it in `App::execute` (the compiler fails otherwise) and add an `Output` variant if needed.
4. Add a test in `crates/application/src/app.rs`.
5. Expose it in the frontends that need it (egui view, CLI subcommand – `exec` works immediately).
6. Update the table above.
