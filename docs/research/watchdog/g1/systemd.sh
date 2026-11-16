#!/bin/sh
# G1 prototype (watchdog.md): what systemd shows for a crash, a restart it schedules and a stop
# by a person. NOT RUN YET: it needs the operator's Linux machine. It uses transient units of
# the user's own manager (systemd-run --user), so it needs no root and leaves no unit file; each
# unit is named g1-watch-*. Usage: systemd.sh <g1prog built for this machine> <work folder>
set -u
PROG=$1
WORK=$2
mkdir -p "$WORK"

# The properties that a watchdog could read.
fields() {
	systemctl --user show "$1" \
		-p ActiveState,SubState,Result,NRestarts,ExecMainCode,ExecMainStatus,Restart,StartLimitBurst |
		tr '\n' ' '
	echo
}

# Prints the properties of $1 each half second for $2 seconds.
watch_for() {
	i=0
	while [ "$i" -lt $(($2 * 2)) ]; do
		printf 't+%04.1fs %s' "$(echo "$i / 2" | bc -l)" "$(fields "$1")"
		sleep 0.5
		i=$((i + 1))
	done
}

# The journal lines of the manager for $1 that tell who stopped it.
journal_of() {
	journalctl --user -u "$1" -o json --no-pager 2>/dev/null |
		grep -o '"\(MESSAGE\|JOB_TYPE\|JOB_RESULT\|MESSAGE_ID\|_SYSTEMD_USER_UNIT\)":"[^"]*"' |
		tr '\n' ' '
	echo
}

# Starts the unit $1 with Restart=$2; the rest are the arguments of g1prog.
run() {
	unit=$1
	restart=$2
	shift 2
	systemctl --user reset-failed "$unit" 2>/dev/null
	systemd-run --user --quiet --unit="$unit" --working-directory="$WORK" \
		-p Restart="$restart" -p RestartSec=2 -p StartLimitIntervalSec=30 -p StartLimitBurst=3 \
		"$PROG" "$@"
	sleep 1
}

echo "== S1 Restart=no, crash after 2 s with code 3"
run g1-watch-s1 no crash 2 3
watch_for g1-watch-s1 4

echo "== S2 Restart=on-failure, crash after 1 s with code 1: systemd restarts, then reaches the start limit"
run g1-watch-s2 on-failure crash 1 1
watch_for g1-watch-s2 14

echo "== S3 Restart=always, serve: systemctl stop (a person)"
run g1-watch-s3 always serve
systemctl --user stop g1-watch-s3
watch_for g1-watch-s3 3
journal_of g1-watch-s3

echo "== S4 Restart=always, serve: systemctl kill --signal=TERM (a person, the program exits 0)"
run g1-watch-s4 always serve
systemctl --user kill --signal=TERM g1-watch-s4
watch_for g1-watch-s4 4
journal_of g1-watch-s4

echo "== S5 Restart=always, serve: kill -KILL of the main process (a crash)"
run g1-watch-s5 always serve
kill -KILL "$(systemctl --user show -p MainPID --value g1-watch-s5)"
watch_for g1-watch-s5 4

echo "== S6 Restart=no, serve: systemctl restart (the restart of WDG-021)"
run g1-watch-s6 no serve
fields g1-watch-s6
systemctl --user restart g1-watch-s6
watch_for g1-watch-s6 2
journal_of g1-watch-s6

echo "== end"
for unit in s1 s2 s3 s4 s5 s6; do
	systemctl --user stop "g1-watch-$unit" 2>/dev/null
	systemctl --user reset-failed "g1-watch-$unit" 2>/dev/null
done
systemctl --user list-units --all 'g1-watch-*' --no-legend | wc -l

# WDG-040 (the right to restart) needs root and is a manual step, not part of this script:
# 1. As a user that is not root and has no polkit rule: `systemctl restart <a system unit>`.
#    Record the error.
# 2. Add a polkit rule that permits org.freedesktop.systemd1.manage-units for that user and one
#    unit name only (action.lookup("unit") and action.lookup("verb") == "restart"). Repeat
#    step 1 for that unit, then for a second unit. Record both results.
# 3. Remove the rule.
