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
# With an empty module cache `go list -m` prints no folder. The tags leave out the port
# mapper (UPnP, NAT-PMP, PCP), so a build without them must never come out of this script.
go mod download github.com/tailscale/tailcat
tags_file="$(go list -m -f '{{.Dir}}' github.com/tailscale/tailcat)/build-tags.txt"
[ -s "$tags_file" ] || { echo "link/build.sh: the build tags of Tailcat are not at $tags_file" >&2; exit 1; }
tags=$(tr '\n' ',' <"$tags_file" | sed 's/,$//')
case ",$tags," in *,ts_omit_portmapper,*) ;; *) echo "link/build.sh: the build tags do not leave out the port mapper" >&2; exit 1 ;; esac
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
