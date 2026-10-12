# 14. Docker image for the browser app

**Status:** Accepted (2026-10-12)

## Context

The browser app is a set of static files (`index.html`, JS loader, wasm module). Teams want to run it
the same way everywhere – locally, in staging, behind their own reverse proxy – without installing the
Rust toolchain and trunk on the target machine. The desktop app needs a display and cannot sensibly
run in a container; the CLI works on local files.

## Decision

- One multi-stage `apps/web/Dockerfile`, built from the repository root (`cargo xtask docker-web`).
- Build stage: official `rust` image with the same version as `rust-toolchain.toml`, prebuilt trunk
  release verified by SHA-256, BuildKit cache mounts for the cargo registry and `target/`.
- Runtime stage: `nginxinc/nginx-unprivileged` (non-root, port 8080) with `apps/web/nginx.conf`: gzip
  for wasm/JS, `index.html` revalidated, hashed assets cached immutably, basic security headers.
- Base images pinned by digest; amd64 and arm64 supported.
- No images for the desktop app and the CLI for now.

## Consequences

- `docker run -p 8080:8080 rust-app-web` serves the app; the container is stateless because all data
  lives in the visitor's browser.
- Rust version, trunk version/checksums and image digests must be updated together when upgrading.
- CI does not build the image yet; adding a `docker build` job is a small step when a registry exists.
- A future server or worker gets its own Dockerfile and `docker compose` setup
  (`docs/architecture/extensions.md`).
