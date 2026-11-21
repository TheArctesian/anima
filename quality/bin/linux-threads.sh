#!/usr/bin/env bash
# Runs the tests of the thread runner and the holder (crates `anima-daemon` and `agent-threads`)
# on Linux in Docker, from a Mac, before a push of a change that touches processes. Linux reads
# `/proc` where a Mac reads `ps`, so a fault can pass on a Mac and fail on GitHub.
#
# The source is mounted read-only. The tests `units::` write into the source tree, so they are
# left out here; GitHub runs them. Two Docker volumes keep the build and the registry between runs.
# Arguments after `--` go to nextest (for example a filter of tests).
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
toolchain="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$root/rust-toolchain.toml")"
image="rust:${toolchain}-bookworm"
case "$(docker info --format '{{.Architecture}}')" in
  aarch64 | arm64) nextest_url="https://get.nexte.st/latest/linux-arm" ;;
  *) nextest_url="https://get.nexte.st/latest/linux" ;;
esac
if [[ "${1:-}" == "--" ]]; then shift; fi
extra=""
if [[ $# -gt 0 ]]; then extra="$(printf ' %q' "$@")"; fi

echo "linux threads: $image, crates anima-daemon and agent-threads, units:: left out"
docker run --rm \
  -v "$root":/src:ro \
  -v anima-linux-target:/target \
  -v anima-linux-cargo:/usr/local/cargo/registry \
  -e NEXTEST_URL="$nextest_url" \
  -w /src "$image" bash -c "
set -euo pipefail
if [[ ! -x /target/cargo-nextest ]]; then
  curl -LsSf \"\$NEXTEST_URL\" | tar -xzf - -C /target
fi
id tester >/dev/null 2>&1 || useradd -m tester
chown -R tester /target /usr/local/cargo/registry /usr/local/rustup
su tester -s /bin/bash -c '
  set -euo pipefail
  cd /src
  export HOME=/home/tester PATH=/usr/local/cargo/bin:\$PATH CARGO_HOME=/usr/local/cargo
  export CARGO_TARGET_DIR=/target/build RUSTC_WRAPPER= RUSTUP_HOME=/usr/local/rustup
  /target/cargo-nextest nextest run --cargo-quiet -p anima-daemon -p agent-threads \
    --no-fail-fast -E \"not test(/^units::/)\"$extra
'
"
