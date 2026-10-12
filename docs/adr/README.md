# Architecture Decision Records

Binding decisions of this project, in short Nygard format (Title, Status, Context, Decision,
Consequences). Old ADRs are never deleted or rewritten; a revised decision gets a new ADR and the old
one the status "Superseded by ADR-XXXX".

| # | Decision | Status |
|---|---|---|
| [0001](0001-synchronous-single-owner-core.md) | Synchronous application core with a single owner | Accepted |
| [0002](0002-typed-command-enum.md) | Typed command enum as the frontend contract | Accepted |
| [0003](0003-layered-workspace-enforced-by-xtask.md) | Layered workspace, enforced by `cargo xtask layers` | Accepted |
| [0004](0004-sqlite-via-rusqlite.md) | SQLite via rusqlite for native persistence | Accepted |
| [0005](0005-browser-storage-memory-indexeddb.md) | Browser persistence: in-memory state mirrored to IndexedDB | Accepted |
| [0006](0006-own-job-runner-port.md) | Own job model with pluggable runners | Accepted |
| [0007](0007-egui-eframe-reference-ui.md) | egui/eframe as the reference UI | Accepted |
| [0008](0008-trunk-for-web-build.md) | trunk for the browser build | Accepted |
| [0009](0009-license-mit-or-apache.md) | License: MIT OR Apache-2.0 | Accepted |
| [0010](0010-dependency-license-policy.md) | Dependency license policy with cargo-deny | Accepted |
| [0011](0011-agent-skill-in-repository.md) | Agent skill versioned in this repository | Accepted |
| [0012](0012-log-facade.md) | `log` facade for logging | Accepted |
| [0013](0013-clap-cli.md) | CLI with clap, JSON output and stable exit codes | Accepted |
| [0014](0014-docker-image-for-the-browser-app.md) | Docker image for the browser app | Accepted |
| [0015](0015-authentication-at-network-hosts.md) | Authentication only at network hosts | Accepted |

New ADR: copy the format, take the next number, add it to this table.
