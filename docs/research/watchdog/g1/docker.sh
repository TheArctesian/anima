#!/bin/sh
# G1 prototype (watchdog.md): what Docker shows for a crash, a restart it makes, a stop by a
# person and a health probe. Builds the image g1-watch:local from scratch with g1prog, runs
# containers named g1-*, and removes them and the image at the end. Downloads nothing.
# Usage: docker.sh <g1prog built for Linux on this machine's architecture> <work folder>
set -u
PROG=$1
WORK=$2
IMAGE=g1-watch:local
mkdir -p "$WORK"
cp "$PROG" "$WORK/g1prog"
cat >"$WORK/Dockerfile" <<'EOF'
FROM scratch
COPY g1prog /g1prog
WORKDIR /
HEALTHCHECK --interval=2s --timeout=1s --retries=2 CMD ["/g1prog", "health"]
ENTRYPOINT ["/g1prog"]
EOF
docker build -q -t "$IMAGE" "$WORK" >/dev/null || exit 1

docker events --filter label=g1=1 --format \
	'{{.Action}} {{.Actor.Attributes.name}} exitCode={{index .Actor.Attributes "exitCode"}} signal={{index .Actor.Attributes "signal"}}' \
	>"$WORK/events.log" 2>&1 &
EVENTS=$!
sleep 1

# The fields of `docker inspect` that a watchdog could read.
fields() {
	docker inspect "$1" --format \
		'status={{.State.Status}} restarting={{.State.Restarting}} exit={{.State.ExitCode}} restarts={{.RestartCount}} health={{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}} policy={{.HostConfig.RestartPolicy.Name}}:{{.HostConfig.RestartPolicy.MaximumRetryCount}}' 2>&1
}

# Prints the fields of $1 each half second for $2 seconds.
watch_for() {
	i=0
	while [ "$i" -lt $(($2 * 2)) ]; do
		printf 't+%04.1fs %s\n' "$(echo "$i / 2" | bc -l)" "$(fields "$1")"
		sleep 0.5
		i=$((i + 1))
	done
}

# Runs container $1 with restart policy $2; the rest are the arguments of g1prog.
run() {
	name=$1
	policy=$2
	shift 2
	health=--no-healthcheck
	[ "$1" = serve ] && [ "${2:-0}" != 0 ] && health=
	docker run -d --name "$name" --label g1=1 --restart "$policy" $health "$IMAGE" "$@" >/dev/null
}

events_of() {
	sleep 1
	grep " $1 " "$WORK/events.log" | grep -v '^exec_' | tr '\n' ';'
	echo
}

echo "== D1 restart no, crash after 2 s with code 3"
run g1-d1 no crash 2 3
watch_for g1-d1 4
events_of g1-d1

echo "== D2 restart on-failure, crash after 1 s with code 1: Docker restarts with its delay"
run g1-d2 on-failure crash 1 1
watch_for g1-d2 20
docker rm -f g1-d2 >/dev/null

echo "== D3 restart on-failure:2, crash after 1 s with code 1: Docker stops its attempts"
run g1-d3 on-failure:2 crash 1 1
watch_for g1-d3 8
events_of g1-d3

echo "== D4 restart always, serve: docker stop (a person)"
run g1-d4 always serve
sleep 1
docker stop g1-d4 >/dev/null
watch_for g1-d4 4
events_of g1-d4

echo "== D5 restart always, serve: docker kill (a person, SIGKILL)"
run g1-d5 always serve
sleep 1
docker kill g1-d5 >/dev/null
watch_for g1-d5 4
events_of g1-d5

echo "== D6 restart always, serve: docker kill --signal TERM (a person, the program exits 0)"
run g1-d6 always serve
sleep 1
docker kill --signal TERM g1-d6 >/dev/null
watch_for g1-d6 4
events_of g1-d6

echo "== D7 restart always, serve, unhealthy after 6 s: the health probe"
run g1-d7 always serve 6
watch_for g1-d7 14
events_of g1-d7

echo "== D8 restart always, serve: docker pause"
run g1-d8 always serve
sleep 1
docker pause g1-d8 >/dev/null
watch_for g1-d8 2
docker unpause g1-d8 >/dev/null

echo "== D9 restart no, serve: docker restart (the restart of WDG-021)"
run g1-d9 no serve
sleep 1
docker restart g1-d9 >/dev/null
watch_for g1-d9 2
events_of g1-d9

echo "== D10 restart no, stopped by docker stop: docker start"
docker stop g1-d9 >/dev/null
fields g1-d9
docker start g1-d9 >/dev/null
watch_for g1-d9 2

kill "$EVENTS"
# shellcheck disable=SC2046 # each container id is one word
docker rm -f $(docker ps -aq --filter label=g1=1) >/dev/null
docker rmi "$IMAGE" >/dev/null
echo "== end: $(docker ps -aq --filter label=g1=1 | wc -l | tr -d ' ') containers left"
