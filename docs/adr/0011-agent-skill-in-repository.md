# 11. Agent skill versioned in this repository

**Status:** Superseded by ADR-0016

## Context

The kit ships a reusable development skill for coding agents (`rust-application-development`).
By default, AI helper files are not committed in Mehmet's projects.

## Decision

The skill is a deliverable of this open-source project and is versioned under
`skills/rust-application-development/`. This is a deliberate exception to the default.
Other AI helper files (`CLAUDE.md`, `.claude/`, …) stay uncommitted.

## Consequences

- Skill and code evolve together; reviews can check that rules match the implementation.
- Users copy or symlink the folder into their agent's skill directory.
