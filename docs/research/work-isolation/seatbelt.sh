#!/bin/bash
# W2 prototype A: a native job WITHOUT a second macOS user, held only by `sandbox-exec` and a
# profile. Measures which reads, writes, keychain calls, tools and network a job still has,
# and the start cost of sandbox-exec (median of 20 runs of /usr/bin/true, with and without).
# Usage: seatbelt.sh > results/seatbelt-macos-<version>.log   (no sudo; removes what it makes)
set -u
HERE=$(cd "$(dirname "$0")" && pwd -P)
JOB=$(mktemp -d "$HERE/.job.XXXXXX")       # the job folder is under $HOME on purpose
SIDE=$(cd "$(mktemp -d)" && pwd -P)         # stand-in for the daemon's folder, outside the job
TOKEN=$SIDE/anima-daemon.token
PROBE=$HOME/.anima-w2-write-probe.$$
echo "fake-token-not-a-secret" >"$TOKEN"
mkdir -p "$JOB/tmp"
trap 'rm -rf "$JOB" "$SIDE"; rm -f "$PROBE"' EXIT
trap 'exit 1' INT TERM

echo "== seatbelt.sh  $(date -u +%FT%TZ)  macOS $(sw_vers -productVersion) $(uname -m)"
echo "uptime: $(uptime)"
echo "note: every timing below was taken under the load shown in that uptime line"
echo "man sandbox-exec says: $(man sandbox-exec 2>/dev/null | col -b | grep -i -m1 deprecated | sed 's/^ *//')"

ANC= d=$HERE                              # stat (not list) of each folder above the job folder
while [ "$d" != / ]; do ANC="$ANC (literal \"$d\")"; d=$(dirname "$d"); done
cat >"$SIDE/job.sb" <<SB
(version 1)
(allow default)
(deny file-read* (subpath "$HOME"))
(allow file-read-metadata $ANC)
(allow file-read* (subpath "$JOB"))
(deny file-read* (subpath "$SIDE"))
(deny file-write*)
(allow file-write* (subpath "$JOB") (literal "/dev/null") (literal "/dev/tty") (literal "/dev/dtracehelper"))
SB
# profiles 2 and 3: the same, and no Mach connection to the keychain services / to any service
{ cat "$SIDE/job.sb"; echo '(deny mach-lookup (global-name "com.apple.SecurityServer") (global-name "com.apple.securityd.xpc") (global-name "com.apple.secd") (global-name "com.apple.security.agent"))'; } >"$SIDE/job2.sb"
{ cat "$SIDE/job.sb"; echo '(deny mach-lookup)'; } >"$SIDE/job3.sb"
echo "== profile (job folder $JOB, token $TOKEN)"
cat "$SIDE/job.sb"
SB_RUN=(sandbox-exec -f "$SIDE/job.sb" env TMPDIR="$JOB/tmp")

# probe <name> <shell command>: runs it outside (unless name starts with "-") and inside the
# sandbox; prints the exit codes and the first line of stderr inside. Output is thrown away.
probe() {
	name=$1 out="-"
	case $name in -*) name=${name#-} ;; *) (cd "$JOB" && bash -c "$2") >/dev/null 2>&1; out=$? ;; esac
	err=$( (cd "$JOB" && "${SB_RUN[@]}" bash -c "$2") 2>&1 >/dev/null)
	in=$?
	verdict=ALLOWED
	[ "$in" -ne 0 ] && verdict=DENIED-or-failed
	printf '%-34s outside rc=%-3s inside rc=%-3s %s  %s\n' "$name" "$out" "$in" "$verdict" "$(echo "$err" | head -1 | cut -c1-110)"
}

echo "== probes (rc 0 = the command worked)"
if ! sandbox-exec -f "$SIDE/job.sb" /usr/bin/true 2>"$SIDE/err"; then
	echo "sandbox-exec could not start here: $(cat "$SIDE/err") -- not permitted here"
	exit 0
fi
probe "list ~/.ssh" "ls '$HOME/.ssh'"
probe "read login.keychain-db (16 B)" "head -c 16 '$HOME/Library/Keychains/login.keychain-db'"
probe "read fake daemon token" "cat '$TOKEN'"
probe "read file in job folder" "echo x >in.txt && cat in.txt"
probe "security find-identity -v" "security find-identity -v"
probe "security find-identity -v -p codesigning" "security find-identity -v -p codesigning"
echo "  (rc=0 only says the tool ran: see the identity counts below. Names are not logged.)"
probe "security find-generic-password" "security find-generic-password -s anima-w2-no-such-item-$$ 2>&1 | grep -q 'could not be found'"
probe "-write to \$HOME" "touch '$PROBE'"
probe "-write to /tmp" "touch /tmp/anima-w2-probe.$$"
probe "-write to stand-in daemon folder" "touch '$SIDE/x'"
probe "-write to job tmp" "touch \"\$TMPDIR/x\""
probe "list /Users" "ls /Users"
probe "list \$HOME" "ls '$HOME'"
probe "git --version" "git --version"
probe "git --version, HOME=job folder" "HOME=\$PWD git --version"
printf '#include <stdio.h>\nint main(void){puts("hello");return 0;}\n' >"$JOB/hello.c"
probe "cc hello.c && ./hello" "cc hello.c -o hello && ./hello"
if command -v swiftc >/dev/null; then
	echo 'print("hello")' >"$JOB/hello.swift"
	probe "swiftc hello.swift (default)" "swiftc hello.swift -o hs1 && ./hs1"
	probe "swiftc with HOME+module cache in job" "HOME=\$PWD swiftc -module-cache-path \$PWD/mc hello.swift -o hs2 && ./hs2"
else echo "swiftc: not present"; fi
probe "-curl -sI https://example.com" "curl -sI -m 20 https://example.com | grep -qi '^HTTP/'"
probe "ps of other processes" "ps -axo pid= | wc -l | grep -qv '^ *[01]\$'"
probe "docker socket of the host" "docker -H unix:///var/run/docker.sock version --format '{{.Server.Version}}'"
echo "  (/var/run/docker.sock -> $(readlink /var/run/docker.sock); /bin/ps is setuid root)"
count() { "$@" security find-identity -v 2>&1 | tail -1 | tr -s ' '; }
echo "identities counted (names not logged): outside:$(count env) inside job.sb:$(count "${SB_RUN[@]}")"
for n in 2 3; do
	echo "== profile $n: job.sb plus this line"
	tail -1 "$SIDE/job$n.sb"
	SB_RUN=(sandbox-exec -f "$SIDE/job$n.sb" env TMPDIR="$JOB/tmp")
	probe "$n: security find-identity -v" "security find-identity -v"
	echo "  identities counted inside:$(count "${SB_RUN[@]}")"
	probe "$n: security find-generic-password" "security find-generic-password -s anima-w2-no-such-item-$$ 2>&1 | grep -q 'could not be found'"
	probe "$n: cc hello.c && ./hello" "cc hello.c -o hello$n && ./hello$n"
	probe "$n: codesign -s - (ad hoc)" "codesign -f -s - hello$n && codesign --verify hello$n"
	probe "$n: git --version, HOME=job folder" "HOME=\$PWD git --version"
done
echo "  (network under profiles 2 and 3: not measured, only one curl call was allowed)"
[ -e "$PROBE" ] && echo "write probe in \$HOME EXISTS (sandbox did not hold)" || echo "write probe in \$HOME absent after the run: yes"

# median of 20 wall-clock times in ms for a command
median20() {
	for _ in $(seq 20); do
		perl -MTime::HiRes=time -e '$t=time; system(@ARGV); printf "%.2f\n", (time-$t)*1000' "$@"
	done | sort -n | awk '{a[NR]=$1} END{printf "median %.2f ms  min %.2f  max %.2f  (n=%d)", (a[10]+a[11])/2, a[1], a[NR], NR}'
}
echo "== start overhead, /usr/bin/true"
echo "without sandbox-exec: $(median20 /usr/bin/true)"
echo "with sandbox-exec:    $(median20 sandbox-exec -f "$SIDE/job.sb" /usr/bin/true)"
echo "uptime at end: $(uptime)"

rm -rf "$JOB" "$SIDE"
rm -f "$PROBE"
echo "== clean-up: job folder gone=$([ ! -e "$JOB" ] && echo yes || echo NO) daemon stand-in gone=$([ ! -e "$SIDE" ] && echo yes || echo NO)"
