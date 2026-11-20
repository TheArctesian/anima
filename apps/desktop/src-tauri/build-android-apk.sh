#!/bin/sh
set -eu

fail() {
  printf 'build-android-apk: %s\n' "$1" >&2
  exit 1
}

[ "$#" -eq 2 ] || fail 'usage: build-android-apk.sh VERSION VERSION_CODE'
version=$1
version_code=$2
case $version in *[!0-9.]*|'') fail 'use a numeric major.minor.patch version' ;; esac
printf '%s\n' "$version" | grep -Eq '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$' ||
  fail 'use a numeric major.minor.patch version'
case $version_code in 0*|*[!0-9]*|'') fail 'use a positive Android version code' ;; esac
[ "${#version_code}" -le 10 ] && [ "$version_code" -le 2100000000 ] ||
  fail 'Android version code exceeds 2100000000'

cd "$(dirname "$0")/../../.."
export ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:?set ANDROID_HOME to the Android SDK}}"
export NDK_HOME="${NDK_HOME:-$ANDROID_HOME/ndk/28.2.13676358}"
export ANIMA_ANDROID_BUILD_VERSION="$version"
SOURCE_DATE_EPOCH=$(git show -s --format=%ct HEAD)
export SOURCE_DATE_EPOCH
rustflag_separator=$(printf '\037')
# Rust leaves source paths in the application library's panic locations.
# Encoded flags take precedence over the Android link arguments supplied by Tauri.
android_link_flags="-Clink-arg=-landroid${rustflag_separator}-Clink-arg=-llog${rustflag_separator}-Clink-arg=-lOpenSLES"
source_path_flags="--remap-path-prefix=$(pwd -P)=/anima${rustflag_separator}--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"
export CARGO_ENCODED_RUSTFLAGS="$android_link_flags$rustflag_separator$source_path_flags"

java_version=$("${JAVA_HOME:+$JAVA_HOME/bin/}java" -version 2>&1) || fail 'install JDK 21 and set JAVA_HOME'
case $java_version in *'version "21.'*|*'version "21"'*) ;; *) fail 'set JAVA_HOME to JDK 21' ;; esac

android_project=apps/desktop/src-tauri/gen/android
apk="$android_project/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk"
output=target/android/Anima-unsigned.apk
config=$(printf '{"version":"%s","build":{"beforeBuildCommand":"bun --bun run --cwd ../client build"},"bundle":{"android":{"versionCode":%s},"createUpdaterArtifacts":false}}' "$version" "$version_code")

bun install --frozen-lockfile
if [ ! -f "$android_project/gradlew" ]; then
  bun run --cwd apps/desktop tauri android init --ci --skip-targets-install
fi
link/build-phone.sh aarch64-linux-android
bun run --cwd apps/desktop tauri android build --ci --target aarch64 --apk --config "$config"

[ -s "$apk" ] || fail "unsigned APK missing at $apk"
mkdir -p "$(dirname "$output")"
cp "$apk" "$output"
printf '%s\n' "$output"
