# Async, parallelism and background jobs

## Pick the right tool

| Need | Use | Not |
|---|---|---|
| Show progress, cancel, keep UI responsive for long work | background job (`JobTask` + `JobRunner`) | a thread spawned in UI code |
| Speed up one CPU-heavy computation | `rayon` inside a job step (native) | tokio |
| Many concurrent network calls / a server | tokio in the host or a network adapter | blocking threads per request |
| Work on other machines/processes | queue + worker binary (`QueueJobRunner`) | in-process threads |
| Periodic work in a GUI | timer → command | busy loops |

## Job model

- `JobRequest` (serde) → `App` builds a `JobTask` with all inputs copied in → `JobRunner::submit`.
- `JobTask::step()` does a bounded slice (≤ ~10 ms) and returns `Progress { done, total }` or
  `Finished(Result<Output, String>)`.
- `CancelToken` (`Arc<AtomicBool>`) is checked before each step; runners also check it while sleeping.
- Runners: thread pool (native), inline-per-frame (browser, tests), queue (distributed, later).
- Results that change data are applied in `App::poll()` on the owner thread — tasks never write to
  repositories.
- Shutdown: cancel + join on drop; workers finish their current step only.

## tokio rules (when a host needs it)

- The core stays sync. A tokio host owns `App` on a dedicated thread (actor + channel) or calls it via
  `spawn_blocking`.
- Never call `block_on` inside async code; never hold a `std::sync::Mutex` guard across `.await`.
- In the browser use `wasm_bindgen_futures::spawn_local`; futures are `!Send` there.

## Distributed jobs (Celery-like) – checklist

Persistent job table · idempotency key per request · retries with backoff and max attempts ·
visibility timeout / heartbeat · dead-letter queue · progress reporting channel · graceful shutdown
(stop taking work, finish step, requeue) · same `JobTask` code in the worker binary.
