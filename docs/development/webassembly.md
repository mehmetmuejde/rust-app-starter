# WebAssembly (browser) app

`apps/web` runs the same egui `Shell` and `App` in the browser.

```bash
cargo xtask web          # trunk serve: http://127.0.0.1:8080, rebuild on change
cargo xtask web-build    # release bundle in apps/web/dist (static files, host anywhere)
cargo xtask docker-web   # container image rust-app-web (see Docker below)
```

## Build pipeline

1. trunk reads `apps/web/index.html`; the `<link data-trunk rel="rust">` tag selects the binary
   `rust-app-web`.
2. `cargo build --target wasm32-unknown-unknown` (release: `--profile web` – fat LTO, `opt-level = "s"`,
   `panic = "abort"`, stripped).
3. trunk runs `wasm-bindgen` (version matched to `Cargo.lock`, downloaded by trunk) and `wasm-opt -Oz`
   (release). The wasm features passed to `wasm-opt` must match what rustc emits (bulk memory, sign-ext,
   …), otherwise validation fails.
4. Output: `index.html`, `*.js` loader, `*_bg.wasm`. Measured on 2026-10-11: 3.9 MB raw, 1.76 MB gzip.

## JavaScript bootstrap

trunk injects a small module script that loads the wasm and calls `main`. `main` (wasm32 only) calls
`spawn_local(run())`, which:
1. requests persistent storage (`navigator.storage.persist()`),
2. opens IndexedDB and loads the snapshot **before** the UI starts,
3. starts `eframe::WebRunner` on `<canvas id="app_canvas">` and removes the loading text.
Fatal start-up errors replace the loading text with a message and are logged to the console.

## Storage in the browser

See [database.md](database.md#browser). Key facts: in-memory state, JSON snapshot in IndexedDB
database `rust-app`, store `snapshots`, key `items`; durable a few milliseconds after each change;
per-origin and per-browser, never shared with the desktop app.

## Differences to native

| Topic | Browser behaviour |
|---|---|
| Threads | none by default (no `SharedArrayBuffer` without COOP/COEP); jobs run inline, a slice per frame |
| Renderer | WebGL2 via glow; WebGPU possible later by switching eframe to `wgpu` |
| Time | `std::time::Instant` panics → use `web-time` (`SystemClock` already does) |
| Random / UUID | `uuid` with `js` feature (Web Crypto) |
| Files | no file system; `<input type=file>` / drag-and-drop in, downloads out |
| Network | `fetch` / WebSocket only; no TCP, no direct database connections |
| Panics | `panic = "abort"` in release; avoid panics (they are denied by lints anyway) |
| Storage quota | browser-managed; may be evicted without persistence permission |

## Docker

`apps/web/Dockerfile` builds the release bundle with trunk and serves it with unprivileged nginx on
port 8080 ([ADR 0014](../adr/0014-docker-image-for-the-browser-app.md)). Build from the repository root:

```bash
cargo xtask docker-web                         # = docker build -f apps/web/Dockerfile -t rust-app-web .
docker run --rm -p 8080:8080 rust-app-web      # http://127.0.0.1:8080
```

- The container is stateless: all data stays in the visitor's browser (IndexedDB, per origin).
- `apps/web/nginx.conf`: gzip for wasm/JS, `index.html` revalidated on every load, hashed assets
  cached for a year, basic security headers.
- Pinned inputs: the Rust image tag must match `rust-toolchain.toml`; base images are pinned by digest,
  the trunk download by SHA-256. Update them together when upgrading.
- Works on amd64 and arm64 hosts. The desktop app is not containerised (it needs a display); the CLI
  has no image yet.
- Serve it over HTTPS in production (reverse proxy or platform TLS); browsers only grant persistent
  storage reliably on secure origins.

## Web Workers and WebGPU (when needed)

- CPU-heavy browser work: run a second instance of the wasm module in a dedicated Web Worker and
  exchange `JobRequest`/`JobState` as JSON. A
  `WorkerJobRunner` would implement `JobRunner`.
- Real wasm threads need `+atomics,+bulk-memory` (nightly `build-std`) and cross-origin isolation
  headers – avoid unless necessary.
- WebGPU: enable eframe's `wgpu` feature for the web host; keep a WebGL fallback for older browsers.
