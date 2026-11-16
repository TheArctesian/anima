#!/bin/sh
# G1 prototype (watchdog.md): what systemd shows for a crash, a restart it schedules and a stop
# by a person. NOT RUN YET: it needs the operator's Linux machine.
#
# It uses units of the user's own manager, so it needs no root. It writes the unit files
# g1-watch-*.service in ~/.config/systemd/user and removes them at the end, also when the script
# is stopped. Unit files are used, not `systemd-run`, because systemd removes a transient unit
# that stopped cleanly, and with it the state to read.
#
# Needs: a user manager with XDG_RUNTIME_DIR set (a login session, or `loginctl enable-linger`;
# over a plain SSH session `systemctl --user` can fail with "Failed to connect to bus"), awk,
# and GNU date. Usage: systemd.sh <g1prog built for this machine> <work folder>
# Each sample line starts with the time since the step of the case, read from the clock.
set -u
PROG=$(readlink -f "$1")
WORK=$(readlink -f "$2")
UNITS=${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user
mkdir -p "$WORK" "$UNITS"

cleanup() {
	for file in "$UNITS"/g1-watch-*.service; do
		[ -e "$file" ] || continue
		unit=$(basename "$file")
		systemctl --user stop "$unit" 2>/dev/null
		systemctl --user reset-failed "$unit" 2>/dev/null
		rm -f "$file"
	done
	systemctl --user daemon-reload
}
trap cleanup EXIT
trap 'exit 1' INT TERM

now() { date +%s.%N; }

# The properties that a watchdog could read.
fields() {
	systemctl --user show "$1" \
		-p ActiveState,SubState,Result,NRestarts,ExecMainCode,ExecMainStatus,Restart,LoadState |
		tr '\n' ' '
	echo
}

# Prints the properties of $1 about each half second for $2 seconds after the time $3.
watch_for() {
	i=0
	while [ "$i" -lt $(($2 * 2)) ]; do
		printf 't+%5.1fs %s\n' "$(echo "$(now) $3" | awk '{print $1 - $2}')" "$(fields "$1")"
		sleep 0.5
		i=$((i + 1))
	done
}

# The journal entries of the manager for $1 that can tell who stopped it, one per line.
journal_of() {
	journalctl --user-unit="$1" -o json --no-pager 2>/dev/null |
		awk '{
			line = ""
			n = split("MESSAGE JOB_TYPE JOB_RESULT MESSAGE_ID EXIT_CODE EXIT_STATUS", keys, " ")
			for (k = 1; k <= n; k++) {
				pattern = "\"" keys[k] "\":\"[^\"]*\""
				if (match($0, pattern)) line = line substr($0, RSTART, RLENGTH) " "
			}
			if (line != "") print "  " line
		}'
}

# Writes and starts the unit $1 with Restart=$2; the rest are the arguments of g1prog.
run() {
	unit=$1
	restart=$2
	shift 2
	cat >"$UNITS/$unit.service" <<EOF
[Unit]
Description=G1 prototype of the Anima watchdog
StartLimitIntervalSec=30
StartLimitBurst=3

[Service]
WorkingDirectory=$WORK
ExecStart=$PROG $*
Restart=$restart
RestartSec=2
EOF
	systemctl --user daemon-reload
	systemctl --user start "$unit"
	sleep 1
}

echo "== S1 Restart=no, crash after 2 s with code 3"
t=$(now)
run g1-watch-s1.service no crash 2 3
watch_for g1-watch-s1.service 4 "$t"

echo "== S2 Restart=on-failure, crash after 1 s with code 1: systemd restarts, then reaches the start limit"
t=$(now)
run g1-watch-s2.service on-failure crash 1 1
watch_for g1-watch-s2.service 14 "$t"

echo "== S3 Restart=always, serve: systemctl stop (a person)"
run g1-watch-s3.service always serve
fields g1-watch-s3.service
t=$(now)
systemctl --user stop g1-watch-s3.service
watch_for g1-watch-s3.service 3 "$t"
journal_of g1-watch-s3.service

echo "== S4 Restart=always, serve: systemctl kill --signal=TERM (a person, the program exits 0)"
run g1-watch-s4.service always serve
t=$(now)
systemctl --user kill --signal=TERM g1-watch-s4.service
watch_for g1-watch-s4.service 4 "$t"
journal_of g1-watch-s4.service

echo "== S5 Restart=always, serve: kill -KILL of the main process from a shell"
run g1-watch-s5.service always serve
t=$(now)
kill -KILL "$(systemctl --user show -p MainPID --value g1-watch-s5.service)"
watch_for g1-watch-s5.service 4 "$t"

echo "== S6 Restart=always, plain (catches no signal): systemctl kill --signal=TERM (a person)"
run g1-watch-s6.service always plain
t=$(now)
systemctl --user kill --signal=TERM g1-watch-s6.service
watch_for g1-watch-s6.service 4 "$t"
journal_of g1-watch-s6.service

echo "== S7 Restart=no, serve: systemctl restart (the restart of WDG-021)"
run g1-watch-s7.service no serve
fields g1-watch-s7.service
t=$(now)
systemctl --user restart g1-watch-s7.service
watch_for g1-watch-s7.service 2 "$t"
journal_of g1-watch-s7.service

cleanup
trap - EXIT
echo "== end: $(systemctl --user list-units --all 'g1-watch-*' --no-legend | wc -l) units left"

# WDG-040 (the right to restart) needs root and is a manual step, not part of this script.
# Use a system unit that is safe to restart, for example a copy of g1-watch-s7 installed as
# /etc/systemd/system/g1-watch-system.service, and a test user that is not root.
# 1. As the test user: `systemctl --no-ask-password restart g1-watch-system.service`.
#    Record the error ("Interactive authentication required" is expected).
# 2. As root, write /etc/polkit-1/rules.d/49-g1-watch.rules:
#      polkit.addRule(function (action, subject) {
#        if (action.id == "org.freedesktop.systemd1.manage-units" &&
#            subject.user == "<test user>" &&
#            action.lookup("unit") == "g1-watch-system.service" &&
#            action.lookup("verb") == "restart") {
#          return polkit.Result.YES;
#        }
#      });
#    Repeat step 1. Then try `stop` of the same unit and `restart` of a second unit. Record the
#    three results.
# 3. Remove the rule file and the system unit.
# The other half of OD-WDG-2, membership of the group `docker`, gives full access to the Docker
# socket; it is not measured here.
