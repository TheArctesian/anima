#!/bin/sh
# Builds the PostgreSQL tree that the Anima application carries (one-click plan S3, OR-6),
# from the official source that postgresql.pin names.
#
#   build.sh <darwin-universal|linux> <work folder> <out folder>
#
# The out folder gets bin/, lib/ and share/: a tree that runs from any folder, links only
# system libraries, and holds the programs `anima-database` runs and nothing else.
# A work folder that already holds the tree of this pin and target is left as it is.
#
# Environment:
#   SIGN_IDENTITY              macOS: the Developer ID identity. Each Mach-O file is signed with
#                              it, the hardened runtime and a secure timestamp. Empty: an ad hoc
#                              signature for a trial on this computer; never for a release.
#   ANIMA_POSTGRESQL_SOURCE    A source archive that is already here; it is still checked
#                              against the pin. Unset: the archive is downloaded.
#
# Needs: a C compiler, make, bison, flex, perl; on Linux also patchelf and the zlib headers.
set -eu

TARGET=${1:?give the target: darwin-universal or linux}
WORK=${2:?give a work folder}
OUT=${3:?give the out folder}
HERE=$(cd "$(dirname "$0")" && pwd)
VERSION=$(sed -n 's/^version=//p' "$HERE/postgresql.pin")
SHA256=$(sed -n 's/^sha256=//p' "$HERE/postgresql.pin")
[ -n "$VERSION" ] && [ -n "$SHA256" ] || { echo "database-tree: postgresql.pin has no version or sha256" >&2; exit 1; }
# The deployment target of the application (bundle.macOS.minimumSystemVersion).
MACOS_MINIMUM=14.2
PROGRAMS="postgres initdb pg_ctl pg_isready pg_controldata pg_dump pg_restore pg_amcheck pg_upgrade"
MODULES="amcheck pg_stat_statements"
STAMP="postgresql $VERSION $SHA256 $TARGET signed-by:${SIGN_IDENTITY:-ad-hoc}"

fail() { echo "database-tree: $1" >&2; exit 1; }

mkdir -p "$WORK"
WORK=$(cd "$WORK" && pwd)
if [ -f "$WORK/tree/STAMP" ] && [ "$(cat "$WORK/tree/STAMP")" = "$STAMP" ]; then
    echo "database-tree: PostgreSQL $VERSION for $TARGET is built already"
else
    ARCHIVE=${ANIMA_POSTGRESQL_SOURCE:-$WORK/postgresql-$VERSION.tar.bz2}
    if [ ! -f "$ARCHIVE" ]; then
        curl --fail --silent --show-error --location --output "$ARCHIVE.part" \
            "https://ftp.postgresql.org/pub/source/v$VERSION/postgresql-$VERSION.tar.bz2"
        mv "$ARCHIVE.part" "$ARCHIVE"
    fi
    if command -v sha256sum >/dev/null 2>&1; then
        FOUND=$(sha256sum "$ARCHIVE" | cut -d' ' -f1)
    else
        FOUND=$(shasum -a 256 "$ARCHIVE" | cut -d' ' -f1)
    fi
    [ "$FOUND" = "$SHA256" ] || fail "the source archive is not the pinned one (sha256 $FOUND)"

    # One build. $1 names its folders; the rest is added to configure.
    build() {
        name=$1; shift
        rm -rf "$WORK/build-$name" "$WORK/install-$name"
        mkdir -p "$WORK/build-$name"
        ( cd "$WORK/build-$name"
          tar -xjf "$ARCHIVE" --strip-components=1
          # No ICU: the cluster uses the built-in C.UTF-8 provider. No OpenSSL: the server has
          # no network listener, and SCRAM and random numbers have code inside PostgreSQL.
          ./configure --prefix="$WORK/install-$name" \
              --without-icu --without-readline --without-openssl --without-llvm --disable-nls \
              CFLAGS="-O2" "$@" >configure.log 2>&1 || { tail -5 configure.log >&2; exit 1; }
          make -j"$(getconf _NPROCESSORS_ONLN)" >make.log 2>&1 || { tail -20 make.log >&2; exit 1; }
          make install >install.log 2>&1
          for module in $MODULES; do make -C "contrib/$module" install >>install.log 2>&1; done
        ) || fail "the build for $name failed; see $WORK/build-$name"
    }

    TREE="$WORK/tree"
    rm -rf "$TREE"
    mkdir -p "$TREE/bin" "$TREE/lib/postgresql"

    case "$TARGET" in
    darwin-universal)
        export MACOSX_DEPLOYMENT_TARGET=$MACOS_MINIMUM
        build arm64 --host=aarch64-apple-darwin CC="clang -arch arm64"
        build x86_64 --host=x86_64-apple-darwin CC="clang -arch x86_64"
        cp -R "$WORK/install-arm64/share" "$TREE/share"
        for program in $PROGRAMS; do
            lipo -create "$WORK/install-arm64/bin/$program" "$WORK/install-x86_64/bin/$program" \
                -output "$TREE/bin/$program"
        done
        lipo -create "$WORK/install-arm64/lib/libpq.5.dylib" "$WORK/install-x86_64/lib/libpq.5.dylib" \
            -output "$TREE/lib/libpq.5.dylib"
        for module in "$WORK"/install-arm64/lib/postgresql/*.dylib; do
            name=$(basename "$module")
            lipo -create "$module" "$WORK/install-x86_64/lib/postgresql/$name" \
                -output "$TREE/lib/postgresql/$name"
        done
        # The build writes libpq's own install path into each program that uses it. Point it
        # at the tree itself, so the tree runs from any folder.
        install_name_tool -id "@loader_path/../lib/libpq.5.dylib" "$TREE/lib/libpq.5.dylib"
        for file in "$TREE"/bin/* "$TREE"/lib/postgresql/*.dylib; do
            case "$file" in */lib/postgresql/*) place="@loader_path/../libpq.5.dylib" ;; *) place="@loader_path/../lib/libpq.5.dylib" ;; esac
            for arch in arm64 x86_64; do
                old=$(otool -arch "$arch" -L "$file" | awk '/libpq\.5/ && !/@loader_path/ {print $1}' | head -1)
                [ -z "$old" ] || install_name_tool -change "$old" "$place" "$file"
            done
        done
        for file in "$TREE"/bin/* "$TREE"/lib/*.dylib "$TREE"/lib/postgresql/*.dylib; do strip -x "$file"; done
        # The hardened runtime loads a library only when it has the Team ID of the program.
        # A Developer ID signature gives every file that Team ID, and no entitlement is due.
        if [ -n "${SIGN_IDENTITY:-}" ]; then
            codesign --force --sign "$SIGN_IDENTITY" --options runtime --timestamp \
                "$TREE"/lib/postgresql/*.dylib "$TREE"/lib/*.dylib "$TREE"/bin/* 2>"$WORK/codesign.log" ||
                { cat "$WORK/codesign.log" >&2; fail "codesign failed"; }
        else
            # A trial only: an ad hoc signature has no Team ID, so library validation is off.
            printf '%s\n' '<?xml version="1.0" encoding="UTF-8"?>' \
                '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">' \
                '<plist version="1.0"><dict><key>com.apple.security.cs.disable-library-validation</key><true/></dict></plist>' \
                >"$WORK/trial.entitlements"
            # codesign says "replacing existing signature" for each file; the check below
            # is what proves the result.
            codesign --force --sign - --options runtime \
                "$TREE"/lib/postgresql/*.dylib "$TREE"/lib/*.dylib 2>/dev/null
            codesign --force --sign - --options runtime --entitlements "$WORK/trial.entitlements" \
                "$TREE"/bin/* 2>/dev/null
        fi
        ;;
    linux)
        build linux
        cp -R "$WORK/install-linux/share" "$TREE/share"
        for program in $PROGRAMS; do
            cp "$WORK/install-linux/bin/$program" "$TREE/bin/$program"
            patchelf --set-rpath '$ORIGIN/../lib' "$TREE/bin/$program"
        done
        cp "$WORK/install-linux/lib/libpq.so.5" "$TREE/lib/libpq.so.5"
        for module in "$WORK"/install-linux/lib/postgresql/*.so; do
            cp "$module" "$TREE/lib/postgresql/"
            patchelf --set-rpath '$ORIGIN/..' "$TREE/lib/postgresql/$(basename "$module")"
        done
        strip "$TREE"/bin/* "$TREE"/lib/*.so.* "$TREE"/lib/postgresql/*.so
        ;;
    *) fail "unknown target $TARGET" ;;
    esac
    cp "$WORK/build-$( [ "$TARGET" = linux ] && echo linux || echo arm64 )/COPYRIGHT" "$TREE/COPYRIGHT"
    echo "$STAMP" >"$TREE/STAMP"
fi
TREE="$WORK/tree"

# Checks, also for a tree that was built before: a tree that fails one is never copied out.
case "$TARGET" in
darwin-universal)
    for file in "$TREE"/bin/* "$TREE"/lib/*.dylib "$TREE"/lib/postgresql/*.dylib; do
        [ "$(lipo -archs "$file")" = "x86_64 arm64" ] || fail "$file is not universal"
        outside=$(otool -L "$file" | awk 'NR > 1 && /^\t/ {print $1}' | grep -v -e '^@loader_path/' -e '^/usr/lib/' || true)
        [ -z "$outside" ] || fail "$file links outside the tree: $outside"
        newest=$(vtool -show-build "$file" | awk '/minos/ {print $2}' | sort -t. -k1,1n -k2,2n | tail -1)
        [ "$newest" = "$MACOS_MINIMUM" ] || fail "$file needs macOS $newest, not $MACOS_MINIMUM"
        codesign --verify --strict "$file" || fail "$file has no valid signature"
    done
    ;;
linux)
    for file in "$TREE"/bin/* "$TREE"/lib/postgresql/*.so; do
        outside=$(readelf -d "$file" | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p' |
            grep -v -e '^libc\.so\.6$' -e '^libm\.so\.6$' -e '^libz\.so\.1$' -e '^libpq\.so\.5$' -e '^ld-linux' || true)
        [ -z "$outside" ] || fail "$file needs a library outside the tree: $outside"
    done
    ;;
esac
"$TREE/bin/postgres" --version | grep -q "(PostgreSQL) $VERSION\$" || fail "the tree's postgres is not version $VERSION"

rm -rf "$OUT"
mkdir -p "$(dirname "$OUT")"
cp -R "$TREE" "$OUT"
echo "database-tree: PostgreSQL $VERSION for $TARGET is in $OUT ($(du -sh "$OUT" | cut -f1))"
