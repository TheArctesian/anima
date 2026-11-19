#!/bin/sh
# Builds the device link as a C library for the phone apps (mobile plan M3-1, MOB-031), with the build
# tags of link/build.sh: a static archive for iOS (libphonelink.a) and a shared library for Android
# (libphonelink.so; Go has no static archive for Android). They go to target/phonelink/<Rust target>/,
# where the build of apps/desktop/src-tauri finds them for an iOS or Android target.
#
#   link/build-phone.sh [aarch64-apple-ios-sim aarch64-apple-ios aarch64-linux-android]
#
# Android needs NDK_HOME. iOS needs Xcode.
set -eu
cd "$(dirname "$0")"
go mod download github.com/tailscale/tailcat
tags_file="$(go list -m -f '{{.Dir}}' github.com/tailscale/tailcat)/build-tags.txt"
[ -s "$tags_file" ] || { echo "link/build-phone.sh: the build tags of Tailcat are not at $tags_file" >&2; exit 1; }
tags=$(tr '\n' ',' <"$tags_file" | sed 's/,$//')

android_clang() {
  case $(uname -s) in
    Darwin) host=darwin-x86_64 ;;
    Linux) host=linux-x86_64 ;;
    *) echo 'link/build-phone.sh: Android requires a macOS or Linux NDK host' >&2; return 1 ;;
  esac
  compiler="${NDK_HOME:?link/build-phone.sh: set NDK_HOME}/toolchains/llvm/prebuilt/$host/bin/aarch64-linux-android24-clang"
  [ -x "$compiler" ] || { echo "link/build-phone.sh: Android compiler missing at $compiler" >&2; return 1; }
  printf '%s\n' "$compiler"
}

targets=${*:-aarch64-apple-ios-sim aarch64-apple-ios aarch64-linux-android}
for target in $targets; do
  extra=
  build_vcs=auto
  case $target in
    aarch64-apple-ios-sim) os=ios mode=c-archive file=libphonelink.a cc="$(xcrun --sdk iphonesimulator --find clang) -isysroot $(xcrun --sdk iphonesimulator --show-sdk-path) -arch arm64 -mios-simulator-version-min=17.0" ;;
    aarch64-apple-ios) os=ios mode=c-archive file=libphonelink.a cc="$(xcrun --sdk iphoneos --find clang) -isysroot $(xcrun --sdk iphoneos --show-sdk-path) -arch arm64 -miphoneos-version-min=17.0" ;;
    # Android has no /etc/resolv.conf, so Go's own resolver finds no DNS server: netcgo uses the system's.
    # F-Droid removes a test fixture, which would otherwise change Go's VCS dirty flag.
    aarch64-linux-android) os=android mode=c-shared file=libphonelink.so extra=netcgo build_vcs=false; cc=$(android_clang) ;;
    *) echo "link/build-phone.sh: unknown target $target" >&2; exit 1 ;;
  esac
  out="../target/phonelink/$target"
  mkdir -p "$out"
  GOOS=$os GOARCH=arm64 CGO_ENABLED=1 CC="$cc" go build -buildvcs="$build_vcs" -trimpath -ldflags="-s -w" -tags "$tags${extra:+,$extra}" \
    -buildmode=$mode -o "$out/$file" ./cmd/phonelink
  echo "$out/$file: $(wc -c <"$out/$file" | tr -d ' ') bytes"
done
