#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
# A local preview build using the repository's pinned Rust toolchain and lockfile.
# Only binaries and provenance enter the Nix payload, never host configuration.
cargo build --locked --release -p anima-daemon
mkdir -p experiments/anima-os/payload
install -m755 target/release/anima-daemon experiments/anima-os/payload/anima-daemon
strip experiments/anima-os/payload/anima-daemon
git rev-parse HEAD > experiments/anima-os/payload/SOURCE-REVISION
sha256sum Cargo.lock > experiments/anima-os/payload/CARGO-LOCK-SHA256
sha256sum experiments/anima-os/payload/anima-daemon > experiments/anima-os/payload/BINARY-SHA256
