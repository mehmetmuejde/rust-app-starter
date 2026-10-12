# 16. Agent skill lives in `.agents/skills/rust/`

**Status:** Accepted (2026-10-12)

## Context

ADR 0011 versions the development skill for coding agents under `skills/rust-application-development/`.
Agent tools look for skills in a dot-folder, and the long folder name carries no information inside a
folder that is already named `skills`.

## Decision

The skill is versioned under `.agents/skills/rust/` (skill name `rust`). The path `.agents/` is on the
global git ignore list of the maintainer, so the project `.gitignore` re-includes it with `!.agents/`.
Other AI helper files stay as decided in ADR 0011.

## Consequences

- Supersedes the location in ADR 0011; the reasons for versioning the skill remain valid.
- Users copy or symlink `.agents/skills/rust/` into their agent's skill directory.
- The `!.agents/` negation must stay in `.gitignore`, otherwise the skill silently drops out of the repository.
