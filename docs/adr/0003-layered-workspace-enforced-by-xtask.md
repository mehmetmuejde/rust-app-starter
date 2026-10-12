# 3. Layered workspace, enforced by `cargo xtask layers`

**Status:** Accepted (2026-10-11)

## Context

"The core does not depend on the UI or the database" is easy to write down and easy to break.
`cargo metadata` exposes the full dependency graph, so the rule can be checked mechanically.

## Decision

Six library crates in four layers plus apps (see `docs/architecture/dependency-rules.md`):
L0 `domain` → L1 `application` → L2 adapters (`platform`, `storage-sqlite`, `storage-browser`) and
L3 `ui-egui` (siblings, both only on L0–L1) → L4 apps. `cargo xtask layers` fails when a crate depends
on a forbidden layer, when UI crates appear below L3, when drivers/runtimes/web APIs appear in L0–L1,
or when a crate is not registered. It runs in CI.

## Consequences

- Architecture violations fail the build, for humans and coding agents alike.
- New crates must be registered in `xtask/src/layers.rs`, which forces a conscious layer decision.
- Prefer few crates: a new crate needs a real reason (second implementation, different target, size).
