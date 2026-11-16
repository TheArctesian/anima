#!/bin/sh
# Builds the release programs of anima-link into link/bin, one for each
# platform, with the build tags that Tailcat's own releases use (they leave
# out the parts of the Tailscale library that the link does not need).
# A program larger than the limit fails the build.
#
#   link/build.sh
set -eu
cd "$(dirname "$0")"
LIMIT_BYTES=20000000
tags=$(tr '\n' ',' <"$(go list -m -f '{{.Dir}}' github.com/tailscale/tailcat)/build-tags.txt" | sed 's/,$//')
mkdir -p bin
for target in darwin/arm64 darwin/amd64 linux/amd64 linux/arm64; do
  os=${target%/*} arch=${target#*/}
  out="bin/anima-link-$os-$arch"
  GOOS=$os GOARCH=$arch go build -trimpath -ldflags="-s -w" -tags "$tags" -o "$out" ./cmd/anima-link
  size=$(wc -c <"$out" | tr -d ' ')
  echo "$out: $size bytes"
  if [ "$size" -gt "$LIMIT_BYTES" ]; then
    echo "$out is larger than $LIMIT_BYTES bytes" >&2
    exit 1
  fi
done
