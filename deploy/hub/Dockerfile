# Hub image (SEC-001). Build context: the repository root.
#   docker build -f deploy/hub/Dockerfile .
# The web client's static build (D-2). Installs only what apps/client needs (E-5); bun reads every
# workspace member's manifest, so each one is copied, and nothing else of them.
# The output is static files, so the stage runs on the build host and never under emulation.
FROM --platform=$BUILDPLATFORM oven/bun:1.3.14-slim AS web
WORKDIR /src
COPY package.json bun.lock ./
COPY apps/client/package.json ./apps/client/package.json
COPY apps/desktop/package.json ./apps/desktop/package.json
COPY quality/package.json ./quality/package.json
COPY design/package.json ./design/package.json
COPY deploy/package.json ./deploy/package.json
COPY deploy/dns/package.json ./deploy/dns/package.json
COPY deploy/headscale/package.json ./deploy/headscale/package.json
RUN bun install --frozen-lockfile --filter @anima/client
COPY apps/client ./apps/client
RUN bun run --cwd apps/client build

FROM rust:1.100.0-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# Workspace members: cargo reads their manifests, but builds nothing of them here.
COPY apps/desktop/src-tauri ./apps/desktop/src-tauri
COPY apps/desktop/keyboard ./apps/desktop/keyboard
COPY apps/desktop/account-session ./apps/desktop/account-session
COPY apps/desktop/runner ./apps/desktop/runner
RUN cargo build --release --locked --bin anima-hub

FROM debian:bookworm-20261027-slim
RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 anima \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin anima
COPY --from=build /src/target/release/anima-hub /usr/local/bin/anima-hub
COPY --from=web /src/apps/client/build /usr/share/anima/web
USER 10001:10001
ENTRYPOINT ["anima-hub"]
