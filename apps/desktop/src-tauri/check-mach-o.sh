#!/bin/sh
# Checks a signed macOS application before it goes to the notary service (UPD-011; S12
# packaging 2.3, D-3 to D-5), for the release workflow (.github/workflows/release.yml, R-179).
# It runs the commands of the release builder (src/builder/carried/mach_o.rs and
# src/builder/artifact.rs) and decides on their output in the same way:
#   - Contents/MacOS holds exactly the application's own program and the seven carried programs;
#   - `codesign --verify --deep --strict` accepts the application;
#   - each Mach-O file of the application (by its first bytes, never by its name) holds exactly
#     x86_64 and arm64, has a Developer ID Application signature of the team, the hardened
#     runtime, a secure timestamp and no entitlement, for each architecture.
# The first file that fails stops the script with a failure.
#
#   check-mach-o.sh <path to the .app> <Team ID>
set -eu

APP=${1:?give the path of the application}
TEAM=${2:?give the Team ID}
PROGRAMS="anima anima-daemon anima-database anima-desktop anima-gateway anima-hub anima-link anima-local-hub"

fail() { echo "check-mach-o: $1" >&2; exit 1; }

# The Team ID goes into a code requirement: ten characters of A-Z and 0-9, nothing else.
echo "$TEAM" | grep -Eq '^[A-Z0-9]{10}$' || fail "the Team ID is not ten characters of A-Z and 0-9"
[ -d "$APP/Contents/MacOS" ] || fail "$APP is not an application"
REQUIREMENT="anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"$TEAM\""

is_mach_o() {
    case "$(od -An -tx1 -N4 "$1" | tr -d ' \n')" in
    feedface | cefaedfe | feedfacf | cffaedfe | cafebabe | bebafeca | cafebabf | bfbafeca) return 0 ;;
    *) return 1 ;;
    esac
}

# One architecture of one file: one code directory with the `runtime` flag, a secure timestamp
# (a signature without one prints `Signed Time=`), and no entitlement.
check_arch() {
    shown=$(codesign -d --verbose=4 --arch "$2" "$1" 2>&1) || fail "$1 ($2) has no signature"
    [ "$(echo "$shown" | grep -c '^CodeDirectory ')" = 1 ] || fail "$1 ($2) has not one code directory"
    echo "$shown" | grep '^CodeDirectory ' | grep -Eq 'flags=0x[0-9a-f]+\(([a-z-]+,)*runtime(,[a-z-]+)*\)' \
        || fail "$1 ($2) has no hardened runtime"
    stamp=$(echo "$shown" | sed -n 's/^Timestamp=//p' | tr -d ' ')
    [ -n "$stamp" ] && [ "$stamp" != none ] || fail "$1 ($2) has no secure timestamp"
    rights=$(codesign -d --entitlements - --xml --arch "$2" "$1" 2>/dev/null | tr -d ' \t\r\n')
    case "$rights" in
    '' | '<?xml'*'<plistversion="1.0"><dict></dict></plist>' | '<?xml'*'<plistversion="1.0"><dict/></plist>') ;;
    *) fail "$1 ($2) has an entitlement" ;;
    esac
    case "$rights" in *'<key>'*) fail "$1 ($2) has an entitlement" ;; esac
}

check_file() {
    archs=$(lipo -archs "$1" | tr ' ' '\n' | grep -v '^$' | sort | tr '\n' ' ')
    [ "$archs" = "arm64 x86_64 " ] || fail "$1 holds '$archs', not exactly arm64 and x86_64"
    codesign --verify --strict "-R=$REQUIREMENT" "$1" || fail "$1 has no Developer ID signature of $TEAM"
    check_arch "$1" x86_64
    check_arch "$1" arm64
}

found=$(ls -A "$APP/Contents/MacOS" | sort | tr '\n' ' ')
[ "$found" = "$PROGRAMS " ] || fail "Contents/MacOS holds '$found', not exactly '$PROGRAMS'"
codesign --verify --deep --strict "$APP" || fail "codesign does not accept $APP"

count=0
list=$(mktemp)
trap 'rm -f "$list"' EXIT
find "$APP" -type f >"$list"
while IFS= read -r file; do
    if is_mach_o "$file"; then
        check_file "$file"
        count=$((count + 1))
    fi
done <"$list"
[ "$count" -ge 8 ] || fail "only $count Mach-O files were found"
echo "check-mach-o: $count Mach-O files of $APP pass"
