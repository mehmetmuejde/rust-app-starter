# 15. Authentication only at network hosts

**Status:** Accepted (2026-10-12)

## Context

The starter kit runs locally: desktop and CLI read and write a SQLite file owned by the logged-in OS
user, the browser app stores data per origin in the visitor's browser. Nothing is reachable over the
network, so there is nobody to authenticate. Projects built on the template will add servers, sync or
remote agent access, and then need a consistent place for authentication and authorization.

## Decision

- No authentication in the template itself.
- When `App` becomes reachable over a network, authentication lives in the host (L4) – e.g. axum
  middleware in `apps/server` – and the core never sees credentials or tokens.
- Authorization goes through the core: the host passes a `Caller` (id, roles) with each command, and
  `App` checks the command's `Effect` (read/write/destructive) against it. Per-user data adds an owner
  to the domain model and to repository queries.
- Identity comes from an OpenID Connect provider, not from own password storage. Flows: Authorization
  Code + PKCE for browser (preferably via backend-for-frontend with an HttpOnly cookie) and desktop
  (system browser, loopback redirect, refresh token in the OS keychain); device flow or API tokens
  for the CLI; bearer tokens for network agent/MCP transports.
- The concept is documented in `docs/architecture/extensions.md`.

## Consequences

- The template stays free of auth dependencies and secrets.
- Adding a server requires a follow-up ADR choosing the identity provider and libraries, and a change
  to `App` for the `Caller` context (a public API change).
- Secrets never go into the wasm bundle, the repository or the SQLite file.
