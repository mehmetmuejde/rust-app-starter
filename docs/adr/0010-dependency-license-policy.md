# 10. Dependency license policy with cargo-deny

**Status:** Accepted (2026-10-11)

## Context

Every dependency's license becomes part of every product built from the template. `directories`
(OS data paths) depends on `option-ext`, licensed MPL-2.0 (file-level weak copyleft).

## Decision

- `cargo deny check` runs locally (`cargo xtask deny`) and in CI: advisories, licenses, bans, sources.
- Only permissive licenses are allowed globally (see `deny.toml`).
- Weak copyleft is allowed per crate only, after a decision. Current exception: `option-ext` (MPL-2.0),
  used unmodified.

## Consequences

- A new non-permissive dependency fails CI and requires a conscious decision.
- MPL-2.0 obligations only arise if `option-ext` itself is modified and distributed.
