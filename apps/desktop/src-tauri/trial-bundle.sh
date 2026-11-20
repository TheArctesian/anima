#!/bin/sh
# Makes a trial bundle of the application with every program that "Create a hub on this
# computer" needs (one-click plan S12) and the daemon of this computer (HMD-052, R-206): for a trial on the computer that built it. It runs the
# build commands of the release builder's bundle step (src/builder/) by hand, in its order,
# with an ad hoc signature on macOS. It is not that step:
#   - it runs in your environment, with your cargo settings, and not in the scrubbed one;
#   - it does not check that the tree is the commit's, and it builds what is in the folder;
#   - it copies with cp and makes none of the builder's checks of the staged files and of the
#     Mach-O files of the application;
#   - it has no notary step, and on Linux it builds for this computer's architecture;
#   - its application does not update itself: the shell is compiled with no updater and the
#     client never checks the feed (UPD-001, DEF-001). A released application does both, and
#     so does a build of the release workflow (ANIMA_RELEASE_BUILD, below).
# So a trial that passes does not prove that a release job passes.
#
# By hand it is NEVER a release: a release starts only from the Builds view (BLD-001, UPD-007)
# or from the release workflow.
#
#   trial-bundle.sh [version]    with no version: the version of the commit that is checked
#                                out, from deploy/bin/commit-version (R-183)
#   trial-bundle.sh --marks      prints the marks that a build with this environment gets, and
#                                builds nothing (for the test of the two modes)
#
# Environment:
#   ANIMA_RELEASE_BUILD   Only the release workflow sets it, to 1, in its jobs `macos` and
#                   `linux`, for a release and for its trial run alike. The application then
#                   has the updater and reads the public feed, as an application of the Builds
#                   path does: the two marks of a local build are not set, and a mark of the
#                   caller's environment is removed. On macOS it needs SIGN_IDENTITY. Unset,
#                   or any other value: a trial bundle, which does not update itself (DEF-001).
#   SIGN_IDENTITY   macOS only. Unset or empty: the ad hoc signature of a trial. The release
#                   workflow (.github/workflows/release.yml, job `macos`, R-179) sets it to the
#                   Developer ID identity that tauri.conf.json names: the tree and the
#                   application are then signed with it, the hardened runtime and a secure
#                   timestamp. The workflow makes the checks, the notary step and the updater
#                   file after this script.
#
# macOS: a universal Anima.ing.app and a DMG. Linux: an AppImage for this computer's
# architecture. The script prints where they are.
#
# Needs what the gates need, the tools of database-tree/build.sh and, on macOS, both Rust
# targets (aarch64-apple-darwin, x86_64-apple-darwin).
set -eu

MARKS_ONLY=false
if [ "${1:-}" = --marks ]; then MARKS_ONLY=true; shift; fi
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
# With no version, the version of the commit that is checked out (R-183).
VERSION=${1:-$(cd "$ROOT" && sh deploy/bin/commit-version)}
IDENTITY=${SIGN_IDENTITY:-}
# The identity goes into a JSON text: no quote and no backslash.
case "$IDENTITY" in *\"* | *\\*) echo "trial-bundle: SIGN_IDENTITY holds a quote or a backslash" >&2; exit 1 ;; esac
echo "$VERSION" | grep -Eq '^(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})$' || { echo "trial-bundle: the version is not like 0.1.0" >&2; exit 1; }
TARGET_FOLDER=${CARGO_TARGET_DIR:-$ROOT/target}
STAGE=$HERE/binaries
RUST_PROGRAMS="anima anima-local-hub anima-hub anima-database anima-gateway anima-daemon"
EXTERNAL='"binaries/anima","binaries/anima-local-hub","binaries/anima-hub","binaries/anima-database","binaries/anima-gateway","binaries/anima-daemon","binaries/anima-link"'
CARRIED="\"externalBin\":[$EXTERNAL],\"resources\":{\"database\":\"database\"}"
export CI=true
# A trial bundle keeps its version: the published version can be lower, and an application that
# checks the feed would install it (DEF-001). The shell reads the first when it is compiled
# (src/desktop.rs), the client the second when it is built (src/lib/updates/local-build.ts).
# Only a build of the release workflow has the updater. The shell asks only if its mark is
# set, so an empty mark is still a mark: the two are removed, never emptied.
if [ "${ANIMA_RELEASE_BUILD:-}" = 1 ]; then
    if [ "$(uname -s)" = Darwin ] && [ -z "$IDENTITY" ]; then
        echo "trial-bundle: ANIMA_RELEASE_BUILD needs SIGN_IDENTITY on macOS; a release has no ad hoc signature" >&2
        exit 1
    fi
    unset ANIMA_LOCAL_BUILD VITE_ANIMA_LOCAL_BUILD
else
    export ANIMA_LOCAL_BUILD=1
    export VITE_ANIMA_LOCAL_BUILD=1
fi
if [ "$MARKS_ONLY" = true ]; then
    echo "ANIMA_LOCAL_BUILD=${ANIMA_LOCAL_BUILD-unset} VITE_ANIMA_LOCAL_BUILD=${VITE_ANIMA_LOCAL_BUILD-unset}"
    exit 0
fi

cargo_build() {
    ( cd "$ROOT" && ANIMA_VERSION=$VERSION cargo build --release --locked \
        --package anima-cli --bin anima \
        --package anima-local-hub --bin anima-local-hub \
        --package anima-hub --bin anima-hub \
        --package anima-database --bin anima-database \
        --package anima-gateway --bin anima-gateway \
        --package anima-daemon --bin anima-daemon "$@" )
}

tauri() {
    ( cd "$ROOT" && bun run --cwd apps/desktop tauri "$@" )
}

# As the bundle step: nothing of an earlier run is used. The tree is built anew.
case "$TARGET_FOLDER" in /*) ;; *) echo "trial-bundle: CARGO_TARGET_DIR is not an absolute path" >&2; exit 1 ;; esac
rm -rf "$STAGE" "$TARGET_FOLDER/database-tree" \
    "$TARGET_FOLDER/universal-apple-darwin/release/bundle" "$TARGET_FOLDER/release/bundle"
# The order of the bundle step: the tree, the link, then the Rust programs.
mkdir -p "$STAGE"

case "$(uname -s)" in
Darwin)
    # An empty SIGN_IDENTITY: the recipe gives the tree an ad hoc signature.
    SIGN_IDENTITY="$IDENTITY" /bin/sh "$HERE/database-tree/build.sh" darwin-universal "$TARGET_FOLDER/database-tree" "$HERE/database"
    /bin/sh "$ROOT/link/build.sh"
    cargo_build --target aarch64-apple-darwin --target x86_64-apple-darwin
    for name in $RUST_PROGRAMS; do
        for triple in aarch64-apple-darwin x86_64-apple-darwin; do
            cp "$TARGET_FOLDER/$triple/release/$name" "$STAGE/$name-$triple"
        done
    done
    cp "$ROOT/link/bin/anima-link-darwin-arm64" "$STAGE/anima-link-aarch64-apple-darwin"
    cp "$ROOT/link/bin/anima-link-darwin-amd64" "$STAGE/anima-link-x86_64-apple-darwin"
    for name in $RUST_PROGRAMS anima-link; do
        lipo -create -output "$STAGE/$name-universal-apple-darwin" \
            "$STAGE/$name-aarch64-apple-darwin" "$STAGE/$name-x86_64-apple-darwin"
    done
    MAC="\"macOS\":{\"signingIdentity\":\"${IDENTITY:--}\"}"
    tauri build --no-bundle --target universal-apple-darwin \
        --config "{\"version\":\"$VERSION\",\"bundle\":{$CARRIED}}"
    tauri bundle --target universal-apple-darwin --bundles app,dmg \
        --config "{\"version\":\"$VERSION\",\"bundle\":{\"createUpdaterArtifacts\":false,$MAC,$CARRIED}}"
    echo "trial-bundle: $TARGET_FOLDER/universal-apple-darwin/release/bundle/macos and .../dmg"
    ;;
Linux)
    triple=$(rustc -vV | sed -n 's/^host: //p')
    case "$triple" in x86_64-*) go_arch=amd64 ;; aarch64-*) go_arch=arm64 ;; *) echo "trial-bundle: no build for $triple" >&2; exit 1 ;; esac
    /bin/sh "$HERE/database-tree/build.sh" linux "$TARGET_FOLDER/database-tree" "$HERE/database"
    /bin/sh "$ROOT/link/build.sh"
    cargo_build
    for name in $RUST_PROGRAMS; do cp "$TARGET_FOLDER/release/$name" "$STAGE/$name-$triple"; done
    cp "$ROOT/link/bin/anima-link-linux-$go_arch" "$STAGE/anima-link-$triple"
    tauri build --no-bundle --config "{\"version\":\"$VERSION\",\"bundle\":{$CARRIED}}"
    APPIMAGE_EXTRACT_AND_RUN=1 tauri bundle --bundles appimage --no-sign \
        --config "{\"version\":\"$VERSION\",\"bundle\":{\"createUpdaterArtifacts\":false,$CARRIED}}"
    echo "trial-bundle: $TARGET_FOLDER/release/bundle/appimage"
    ;;
*)
    echo "trial-bundle: no bundle for this system" >&2
    exit 1
    ;;
esac
