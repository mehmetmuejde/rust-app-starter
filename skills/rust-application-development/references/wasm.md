# Desktop and WebAssembly compatibility

## Checklist for shared crates (domain, application, UI)

- [ ] `cargo check --target wasm32-unknown-unknown -p <crate>` passes (template: `cargo xtask wasm`).
- [ ] No `std::time::Instant`/`SystemTime::now()` – they panic in the browser. Use `web-time`.
- [ ] No `std::thread::{spawn, sleep}`, no blocking waits, no `std::fs`, no `std::net`.
- [ ] No `cfg(target_arch)` in domain/application; put it in adapters/hosts.
- [ ] Randomness/UUIDs: `uuid` with the `js` feature on wasm32 (`getrandom` needs a JS backend).
- [ ] No C dependencies unless you have a wasm-capable clang (e.g. SQLite in wasm).
- [ ] Panics abort in release builds (`panic = "abort"`); never rely on unwinding.

## Runtime differences

| Area | Native | Browser |
|---|---|---|
| Threads | OS threads, rayon | none without COOP/COEP + atomics; use inline slices or Web Workers |
| Async | tokio (only in hosts that need it) | `wasm_bindgen_futures::spawn_local`, single-threaded, `!Send` futures |
| Storage | files, SQLite | IndexedDB / OPFS (async), localStorage (sync, ~5 MB) |
| Files | `std::fs` | `<input type=file>`, drag-and-drop, Blob downloads |
| Network | any | `fetch`, WebSocket; CORS applies; no raw TCP, no DB connections |
| GPU | wgpu: Metal/DX12/Vulkan | WebGL2 (glow) or WebGPU (wgpu) |
| Entry | `fn main` | `main` → `spawn_local(async { … WebRunner::start … })` |
| Logging | stderr | browser console |

## Patterns

- **Async browser API behind a sync port:** keep state in memory, persist asynchronously in order
  (snapshot or per-record writes), load everything before the UI starts.
- **Jobs without threads:** step-based tasks; an inline runner executes ≤ 8 ms per frame.
- **CPU-heavy browser work:** a dedicated Web Worker running a second instance of the same wasm module;
  messages as JSON (`JobRequest` in, `JobState` out), binary data as transferables.
- **Build:** trunk (`<link data-trunk rel="rust" data-cargo-profile-release="web">`). If `wasm-opt`
  fails validation (bulk memory etc.), pass the features rustc emits via `data-wasm-opt-params`.
- **Size:** fat LTO, `opt-level = "s"` for the bundle, `opt-level = 3` for hot crates; measure with
  `gzip -9 | wc -c` and record the number.

## Debugging

- Browser console shows `log` output (console logger) and panics (if a panic hook is installed).
- `?workers=0`-style query switches help to isolate worker problems.
- Start-up errors should replace the loading text, not leave a blank canvas.
