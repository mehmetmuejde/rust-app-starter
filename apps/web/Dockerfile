# syntax=docker/dockerfile:1.7
#
# Browser app as a container: build the wasm bundle with trunk, serve the static files with nginx.
# Build from the repository root (the whole workspace is needed):
#
#   docker build -f apps/web/Dockerfile -t rust-app-web .     # or: cargo xtask docker-web
#   docker run --rm -p 8080:8080 rust-app-web                 # http://127.0.0.1:8080
#
# Data stays in the visitor's browser (IndexedDB); the container is stateless.

# ---- build ------------------------------------------------------------------------------------------
# Same Rust version as rust-toolchain.toml; update both together.
FROM rust:1.99-slim-trixie@sha256:2752b332db73fdbb7dc576f06c82ed1f312005784ef913d7e04a28f5f55dc581 AS build

ARG TRUNK_VERSION=0.21.14
ARG TRUNK_SHA256_AMD64=f2b4680cd239693a646a2795e4633c625328d7b2a044fbe749fa3a2fe9e7036b
ARG TRUNK_SHA256_ARM64=b1d8e60e454f7fc182d9a4d95d1506ffbae947d8ba90f8f6f02da93b60f980f9
ARG TARGETARCH

RUN apt-get update \
 && apt-get install -y --no-install-recommends curl \
 && rm -rf /var/lib/apt/lists/*

# Prebuilt trunk release, verified against its published checksum.
RUN set -eu; \
    case "$TARGETARCH" in \
      amd64) triple=x86_64-unknown-linux-gnu;  sum="$TRUNK_SHA256_AMD64" ;; \
      arm64) triple=aarch64-unknown-linux-gnu; sum="$TRUNK_SHA256_ARM64" ;; \
      *) echo "unsupported architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    curl -fsSL -o /tmp/trunk.tar.gz \
      "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-${triple}.tar.gz"; \
    echo "${sum}  /tmp/trunk.tar.gz" | sha256sum -c -; \
    tar -xzf /tmp/trunk.tar.gz -C /usr/local/bin trunk; \
    rm /tmp/trunk.tar.gz

WORKDIR /src
COPY . .

# rust-toolchain.toml selects the toolchain and the wasm32 target; trunk fetches the matching
# wasm-bindgen and wasm-opt. Cache mounts keep the registry and build artifacts between builds.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cd apps/web && trunk build --release --locked

# ---- runtime ----------------------------------------------------------------------------------------
# Unprivileged nginx: runs as a non-root user and listens on 8080.
FROM nginxinc/nginx-unprivileged:1.31-alpine@sha256:884e0b7f42f303641b0f4e78755aea647521dca0b89ca3be7992ad78b5c07434

COPY apps/web/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /src/apps/web/dist /usr/share/nginx/html

EXPOSE 8080
