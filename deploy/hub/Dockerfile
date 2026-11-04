# Hub image (WP-04 D-6). Build context: the repository root.
#   docker build -f deploy/hub/Dockerfile .
FROM rust:1.93-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --locked -p anima-hub

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 anima \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin anima
COPY --from=build /src/target/release/anima-hub /usr/local/bin/anima-hub
USER 10001:10001
ENTRYPOINT ["anima-hub"]
