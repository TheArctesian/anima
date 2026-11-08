# Hub image (SEC-001). Build context: the repository root.
#   docker build -f deploy/hub/Dockerfile .
# The web client's static build (D-2). Installs only what apps/client needs (E-5); bun reads every
# workspace member's manifest, so each one is copied, and nothing else of them.
FROM oven/bun:1.3.14-slim AS web
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

FROM rust:1.93-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# A workspace member: cargo reads its manifest, but builds nothing of it here.
COPY apps/desktop/src-tauri ./apps/desktop/src-tauri
RUN cargo build --release --locked --bin anima-hub

FROM debian:bookworm-20261027-slim
RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 anima \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin anima
COPY --from=build /src/target/release/anima-hub /usr/local/bin/anima-hub
COPY --from=web /src/apps/client/build /usr/share/anima/web
# A new named volume copies this owner and mode (E-3, SEC-005).
RUN mkdir -p /var/lib/anima/recordings \
    && chown 10001:10001 /var/lib/anima/recordings \
    && chmod 0700 /var/lib/anima/recordings
USER 10001:10001
ENTRYPOINT ["anima-hub"]
