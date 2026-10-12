# 6. Own job model with pluggable runners

**Status:** Accepted (2026-10-11)

## Context

The kit needs background work with progress and cancellation on desktop and in the browser, and a path
towards external workers later. Async I/O, CPU parallelism, OS threads, background jobs and distributed
workers are different concepts and must not be mixed up.

## Decision

- `application::jobs` defines `JobRequest` (serde enum), `JobState`, `CancelToken`, and `JobTask` —
  a self-contained task that works in bounded `step()`s and never touches the repository.
- `JobRunner` is a port: `ThreadJobRunner` (native, small thread pool) in `starter-platform`,
  `InlineJobRunner` (runs slices per poll; browser and tests) in `starter-application`.
- `App::poll` collects updates on the owner thread and emits `JobChanged` events.
- Distributed workers (queue + separate process) are documented, not implemented.

## Consequences

- Same job code on every platform; progress and cancellation come from the step model.
- Steps must be short; CPU-heavy steps may use rayon internally (native only).
- Persistence, retries and timeouts are extension points, added when a project needs them.
