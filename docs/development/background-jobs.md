# Background jobs

## Five different things

| Concept | Purpose | In this kit |
|---|---|---|
| Async I/O | wait for many network/disk operations without threads | not used in the core; tokio only in future network hosts |
| CPU parallelism | split one computation over cores | `rayon` inside a job step (native only) |
| OS threads | run something beside the UI | `ThreadJobRunner` worker pool |
| Background jobs | user-visible units of work with progress and cancellation | `application::jobs` |
| Distributed workers | jobs on other processes/machines | concept only ([extensions](../architecture/extensions.md#external-workers-and-distributed-jobs-celery-like)) |

## Model

```mermaid
stateDiagram-v2
  [*] --> Queued: App::execute(StartJob)
  Queued --> Running: first step
  Running --> Running: step → Progress
  Running --> Succeeded: step → Finished(Ok)
  Running --> Failed: step → Finished(Err)
  Queued --> Cancelled: CancelToken
  Running --> Cancelled: CancelToken
```

- `JobRequest` (serde enum) – what to do; part of `Command::StartJob`.
- `App` prepares a `JobTask` on the owner thread with **all input copied in** (e.g. item names). The
  task never touches the repository, so it is `Send` and can run anywhere.
- `JobTask::step()` does one bounded slice of work and returns `Progress` or `Finished`.
- A `JobRunner` executes steps: `ThreadJobRunner` (native threads; sleeps `step_delay` in 50 ms slices so
  cancellation stays responsive) or `InlineJobRunner` (browser/tests; at most ~8 ms per poll).
- `App::poll()` collects `JobUpdate`s, updates `list_jobs` and emits `JobChanged` events. Results that
  change data must be applied here, on the owner thread.
- Shutdown: dropping `ThreadJobRunner` cancels running jobs and joins the workers.

## Adding a job

1. Add a variant to `JobRequest` (and its `label`), and an output variant to `JobOutput` if needed.
2. Implement `JobTask` for a struct holding the input; keep each `step` short (≤ ~10 ms).
3. In `App::start_job`, build the task from the request (snapshot inputs from the repository).
4. If the result changes data, apply it in `App::poll` when the job succeeds.
5. Test the task directly (loop `advance()`), and the flow through `App` with `InlineJobRunner`.

## Extension points

Retries (re-submit with attempt counter), timeouts (deadline checked in `advance`), persistent jobs
(store `JobRequest` + state in the repository and resume on start), idempotency keys (dedupe in
`start_job`), progress for nested work (sub-steps in `Progress`), external queues (`QueueJobRunner`).
