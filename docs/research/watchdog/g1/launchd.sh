#!/bin/sh
# G1 prototype (watchdog.md): what launchd shows for a crash, a restart it schedules, and a stop
# by a person. Runs one job of the user domain, label ing.anima.g1.watch, and removes it at the
# end. Usage: launchd.sh <g1prog built for macOS> <work folder>
set -u
PROG=$1
WORK=$2
LABEL=ing.anima.g1.watch
DOMAIN=gui/$(id -u)
TARGET=$DOMAIN/$LABEL
mkdir -p "$WORK"

# The fields of `launchctl print` that a watchdog could read.
fields() {
	launchctl print "$TARGET" 2>&1 |
		grep -E '^\s*(state|pid|runs|last exit code|last terminating signal|immediate reason|job state|spawn type|properties|Could not find)' |
		sed 's/^[[:space:]]*//' | tr '\n' ';'
	echo
}

# Prints the fields each second for $1 seconds.
watch_for() {
	i=0
	while [ "$i" -lt "$1" ]; do
		printf 't+%02ds %s' "$i" "$(fields)"
		echo
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

echo "== L1 KeepAlive true, serve: the process is killed with SIGKILL (a crash)"
job '<true/>' serve
fields
kill -KILL "$(pid)"
watch_for 12

echo "== L2 KeepAlive true, serve: launchctl kill SIGTERM (a person)"
launchctl kill SIGTERM "$TARGET"
watch_for 12

echo "== L3 KeepAlive true, serve: launchctl disable, then launchctl bootout (a person)"
launchctl disable "$TARGET"
fields
launchctl print-disabled "$DOMAIN" | grep "$LABEL"
launchctl bootout "$TARGET"
fields
launchctl print-disabled "$DOMAIN" | grep "$LABEL"
launchctl enable "$TARGET"

echo "== L4 KeepAlive SuccessfulExit=false, crash after 2 s with code 1: launchd throttles"
job '<dict><key>SuccessfulExit</key><false/></dict>' crash 2 1
watch_for 25

echo "== L5 KeepAlive SuccessfulExit=false, serve: launchctl kill SIGTERM (a person, exit 0)"
job '<dict><key>SuccessfulExit</key><false/></dict>' serve
launchctl kill SIGTERM "$TARGET"
watch_for 5

echo "== L6 no KeepAlive, crash after 2 s with code 3"
job '<false/>' crash 2 3
watch_for 5

echo "== L7 no KeepAlive, serve: killed with SIGKILL (a crash)"
job '<false/>' serve
kill -KILL "$(pid)"
watch_for 3

echo "== L8 no KeepAlive, serve: launchctl kill SIGTERM (a person)"
job '<false/>' serve
launchctl kill SIGTERM "$TARGET"
watch_for 3

echo "== L9 no KeepAlive, serve: launchctl kickstart -k (the restart of WDG-021)"
fields
launchctl kickstart -k "$TARGET"
watch_for 3

echo "== L10 no KeepAlive, serve: launchctl kill SIGTERM, then launchctl kickstart (no -k)"
job '<false/>' serve
launchctl kill SIGTERM "$TARGET"
sleep 1
fields
launchctl kickstart "$TARGET"
watch_for 3

echo "== L11 no KeepAlive, serve, running: launchctl kickstart -k"
fields
launchctl kickstart -k "$TARGET"
watch_for 3

echo "== end: bootout"
launchctl bootout "$TARGET"
fields
