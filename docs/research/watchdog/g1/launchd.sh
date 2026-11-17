#!/bin/sh
# G1 prototype (watchdog.md): what launchd shows for a crash, a restart it schedules, and a stop
# by a person. Runs one job of the user domain, label ing.anima.g1.watch, and removes it at the
# end, also when the script is stopped. Usage: launchd.sh <g1prog built for macOS> <work folder>
# Each sample line starts with the time since the step of the case, read from the clock.
set -u
PROG=$1
WORK=$2
LABEL=ing.anima.g1.watch
DOMAIN=gui/$(id -u)
TARGET=$DOMAIN/$LABEL
mkdir -p "$WORK"

cleanup() {
	launchctl enable "$TARGET" 2>/dev/null
	launchctl bootout "$TARGET" 2>/dev/null
}
trap cleanup EXIT
trap 'exit 1' INT TERM

now() { perl -MTime::HiRes=time -e 'printf "%.1f", time'; }

# The fields of `launchctl print` that a watchdog could read.
fields() {
	launchctl print "$TARGET" 2>&1 |
		grep -E '^\s*(state|pid|runs|last exit code|last terminating signal|immediate reason|Could not find)' |
		sed 's/^[[:space:]]*//' | tr '\n' ';'
	echo
}

# Prints the fields about each second for $1 seconds after the time $2.
watch_for() {
	i=0
	while [ "$i" -lt "$1" ]; do
		printf 't+%5.1fs %s\n' "$(echo "$(now) $2" | awk '{print $1 - $2}')" "$(fields)"
		sleep 1
		i=$((i + 1))
	done
}

# Writes the job file with $1 as its KeepAlive value and the rest as the arguments.
job() {
	keep=$1
	shift
	args=""
	for word in "$PROG" "$@"; do args="$args<string>$word</string>"; done
	cat >"$WORK/$LABEL.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>$LABEL</string>
<key>ProgramArguments</key><array>$args</array>
<key>WorkingDirectory</key><string>$WORK</string>
<key>RunAtLoad</key><true/>
<key>KeepAlive</key>$keep
</dict></plist>
EOF
	launchctl bootout "$TARGET" 2>/dev/null
	sleep 1
	launchctl bootstrap "$DOMAIN" "$WORK/$LABEL.plist"
	sleep 2
}

pid() { launchctl print "$TARGET" 2>/dev/null | awk '/^\tpid = /{print $3}'; }

echo "== L1 KeepAlive true, serve: the process gets SIGKILL from a shell (a crash, or a person's kill -9)"
job '<true/>' serve
fields
t=$(now)
kill -KILL "$(pid)"
watch_for 12 "$t"

echo "== L2 KeepAlive true, serve: launchctl kill SIGTERM (a person; the program exits 0)"
t=$(now)
launchctl kill SIGTERM "$TARGET"
watch_for 12 "$t"

echo "== L3 KeepAlive true, serve: launchctl disable, then launchctl bootout (a person)"
launchctl disable "$TARGET"
fields
launchctl print-disabled "$DOMAIN" | grep "$LABEL"
launchctl bootout "$TARGET"
fields
launchctl print-disabled "$DOMAIN" | grep "$LABEL"
launchctl enable "$TARGET"

echo "== L4 KeepAlive SuccessfulExit=false, crash after 2 s with code 1: launchd throttles"
t=$(now)
job '<dict><key>SuccessfulExit</key><false/></dict>' crash 2 1
watch_for 25 "$t"

echo "== L5 KeepAlive SuccessfulExit=false, serve: launchctl kill SIGTERM (a person, exit 0)"
job '<dict><key>SuccessfulExit</key><false/></dict>' serve
t=$(now)
launchctl kill SIGTERM "$TARGET"
watch_for 5 "$t"

echo "== L6 no KeepAlive, crash after 2 s with code 3"
t=$(now)
job '<false/>' crash 2 3
watch_for 5 "$t"

echo "== L7 no KeepAlive, serve: SIGKILL from a shell"
job '<false/>' serve
t=$(now)
kill -KILL "$(pid)"
watch_for 3 "$t"

echo "== L8 no KeepAlive, serve: launchctl kill SIGTERM (a person)"
job '<false/>' serve
t=$(now)
launchctl kill SIGTERM "$TARGET"
watch_for 3 "$t"

echo "== L9 no KeepAlive, not running: launchctl kickstart -k"
fields
t=$(now)
launchctl kickstart -k "$TARGET"
watch_for 3 "$t"

echo "== L10 no KeepAlive, serve: launchctl kill SIGTERM, then launchctl kickstart (no -k)"
job '<false/>' serve
launchctl kill SIGTERM "$TARGET"
sleep 1
fields
t=$(now)
launchctl kickstart "$TARGET"
watch_for 3 "$t"

echo "== L11 no KeepAlive, serve, running: launchctl kickstart -k"
fields
t=$(now)
launchctl kickstart -k "$TARGET"
watch_for 3 "$t"

echo "== L12 KeepAlive true, serve: launchctl stop (a person)"
job '<true/>' serve
t=$(now)
launchctl stop "$LABEL"
watch_for 12 "$t"

echo "== L13 KeepAlive true, serve: kill -TERM from a shell (a person)"
t=$(now)
kill -TERM "$(pid)"
watch_for 12 "$t"

echo "== L14 no KeepAlive, plain (catches no signal): launchctl kill SIGTERM (a person)"
job '<false/>' plain
t=$(now)
launchctl kill SIGTERM "$TARGET"
watch_for 3 "$t"

echo "== L15 no KeepAlive, plain, running: launchctl kickstart -k"
job '<false/>' plain
fields
t=$(now)
launchctl kickstart -k "$TARGET"
watch_for 3 "$t"

echo "== end: bootout"
launchctl bootout "$TARGET"
fields
