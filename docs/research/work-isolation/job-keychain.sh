#!/bin/bash
# W2 prototype B: a code-signing identity for ONE job from a keychain that is deleted after it.
# Measures each step in ms (20 s limit; a hang is a result and would mean a GUI prompt), and
# proves that the user's keychain search list is the same before and after.
# Usage: job-keychain.sh > results/job-keychain-macos-<version>.log   (no sudo; throwaway cert)
set -u
D=$(cd "$(mktemp -d)" && pwd -P)
KC=$D/job.keychain-db
PW=$(/usr/bin/openssl rand -hex 24)          # never printed
NAME="anima-w2-throwaway-$$"
LIST_BEFORE=$(security list-keychains -d user)
DEFAULT_BEFORE=$(security default-keychain -d user 2>&1)
restore() { echo "$LIST_BEFORE" | sed 's/^ *"//; s/"$//' | tr '\n' '\0' | xargs -0 security list-keychains -d user -s; }
cleanup() {
	[ "$(security list-keychains -d user)" = "$LIST_BEFORE" ] || restore
	[ -e "$KC" ] && security delete-keychain "$KC" 2>/dev/null
	rm -rf "$D"
}
trap cleanup EXIT
trap 'exit 1' INT TERM

echo "== job-keychain.sh  $(date -u +%FT%TZ)  macOS $(sw_vers -productVersion) $(uname -m)"
echo "uptime: $(uptime)"
echo "note: every timing below was taken under the load shown in that uptime line"
echo "search list before: $(echo "$LIST_BEFORE" | tr -s ' \n' ' ')"

# step <name> <command...>: time in ms, exit code, HANG if the 20 s limit hit, first error line
step() {
	name=$1; shift
	ms=$(perl -MTime::HiRes=time -e '$t=time; system("/usr/bin/timeout","20",@ARGV); $r=$?>>8; open(F,">&=3"); printf F "%.0f %d", (time-$t)*1000, $r' "$@" 3>&1 >"$D/out" 2>"$D/err")
	rc=${ms#* }; ms=${ms% *}
	hang=""; [ "$rc" = 124 ] && hang=" HANG(20 s limit; a prompt may have appeared)"
	printf '%-44s %6s ms  rc=%s%s  %s\n' "$name" "$ms" "$rc" "$hang" "$(head -1 "$D/err" | cut -c1-100)"
	return "$rc"
}
same_list() { [ "$(security list-keychains -d user)" = "$LIST_BEFORE" ] && echo same || echo CHANGED; }

cat >"$D/cert.cnf" <<CNF
[req]
distinguished_name=dn
x509_extensions=ext
prompt=no
[dn]
CN=$NAME
[ext]
basicConstraints=critical,CA:false
keyUsage=critical,digitalSignature
extendedKeyUsage=critical,codeSigning
CNF
printf '#include <stdio.h>\nint main(void){puts("hello");return 0;}\n' >"$D/hello.c"
step "cc hello.c (the Mach-O to sign)" cc "$D/hello.c" -o "$D/hello"
step "openssl: key + self-signed cert" /usr/bin/openssl req -x509 -newkey rsa:2048 -nodes -days 1 \
	-config "$D/cert.cnf" -keyout "$D/key.pem" -out "$D/cert.pem"
step "security create-keychain" security create-keychain -p "$PW" "$KC"
echo "  search list after create-keychain: $(same_list)"
step "security set-keychain-settings (no lock)" security set-keychain-settings "$KC"
step "security unlock-keychain" security unlock-keychain -p "$PW" "$KC"
step "security import key -T codesign" security import "$D/key.pem" -k "$KC" -T /usr/bin/codesign
step "security import cert" security import "$D/cert.pem" -k "$KC"
step "security set-key-partition-list" security set-key-partition-list -S apple-tool:,apple: -s -k "$PW" "$KC"
step "security find-identity <job keychain>" security find-identity -p codesigning "$KC"
echo "  identities in the job keychain: $(grep -c "$NAME" "$D/out") (lines with the throwaway name; 'valid': $(grep -c 'valid identities' "$D/out"))"
grep -E 'CSSMERR|identities found' "$D/out" | sed 's/^ */  /'
SHA=$(/usr/bin/openssl x509 -in "$D/cert.pem" -noout -fingerprint -sha1 | sed 's/.*=//; s/://g')

echo "== sign WITHOUT adding the keychain to the search list"
ok=no
step "codesign --keychain -s <name>" codesign -f --keychain "$KC" -s "$NAME" "$D/hello" && ok=name
[ $ok = no ] && step "codesign --keychain -s <sha1 of cert>" codesign -f --keychain "$KC" -s "$SHA" "$D/hello" && ok=sha1
echo "  search list now: $(same_list)"
if [ $ok = no ]; then
	echo "== codesign could not use the keychain outside the search list; adding it for one command"
	echo "$LIST_BEFORE" | sed 's/^ *"//; s/"$//' | { tr '\n' '\0'; printf '%s\0' "$KC"; } | xargs -0 security list-keychains -d user -s
	step "codesign (keychain in search list)" codesign -f --keychain "$KC" -s "$SHA" "$D/hello" && ok=searchlist
	restore
	echo "  search list restored: $(same_list)"
fi
echo "signing worked by: $ok"
step "codesign --verify --verbose=2" codesign --verify --verbose=2 "$D/hello"
head -3 "$D/err" | sed 's/^/  /; s|'"$D"'|<tmp>|'
step "codesign -dvv (who signed)" codesign -dvv "$D/hello"
grep -E '^(Authority|Signature|TeamIdentifier)' "$D/err" | sed 's/^/  /'
step "signed binary runs" "$D/hello"

echo "== delete"
step "security delete-keychain" security delete-keychain "$KC"
step "find-identity after delete" security find-identity -p codesigning "$KC"
echo "  its answer: $(grep -h 'identities found' "$D/out" "$D/err" | tr -s ' \n' ' '); throwaway identity listed: $(cat "$D/out" "$D/err" | grep -c "$NAME")"
step "codesign after delete (must fail)" codesign -f --keychain "$KC" -s "$NAME" "$D/hello"
echo "  keychain file gone: $([ ! -e "$KC" ] && echo yes || echo NO)"
rm -rf "$D"
echo "  temp folder gone: $([ ! -e "$D" ] && echo yes || echo NO)"
echo "search list after:  $(security list-keychains -d user | tr -s ' \n' ' ')"
echo "search list identical before/after: $(same_list)"
echo "default keychain identical before/after: $([ "$(security default-keychain -d user 2>&1)" = "$DEFAULT_BEFORE" ] && echo same || echo CHANGED)"
echo "hangs or prompts: see HANG marks above (none = no command waited on a prompt)"
echo "note: -p <password> is on the command line of security(1), so another process of the same user can see it in ps"
echo "uptime at end: $(uptime)"
