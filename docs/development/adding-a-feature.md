# Adding a feature – checklist

Example: items get an optional *note*.

1. **Domain** (`crates/domain`): add the field/value object with its rules and unit tests.
2. **Persistence:** new migration `0002_item_note.sql`; map the column in `storage-sqlite`; bump the
   snapshot handling in `storage-browser` if the format changes (keep reading version 1).
   Extend `contract.rs` if the behaviour is new.
3. **Command** (`crates/application`): new variant (`SetItemNote { id, note }`) + `COMMANDS` entry +
   `execute` arm + event + tests ([commands.md](../architecture/commands.md#adding-a-command)).
4. **Frontends:** egui view (queue the command in `pending`), CLI subcommand (optional – `exec` already
   works).
5. **Docs:** command table, user-facing docs, ADR if a binding decision was made.
6. **Gate:** `cargo xtask ci` green.

Ask before you add: a new crate (rule 5 of the skill), a new runtime dependency (license check via
`cargo xtask deny`), anything platform-specific in the core (never).
