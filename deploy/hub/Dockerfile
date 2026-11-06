# Hub image (SEC-001). Build context: the repository root.
#   docker build -f deploy/hub/Dockerfile .
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
USER 10001:10001
ENTRYPOINT ["anima-hub"]
