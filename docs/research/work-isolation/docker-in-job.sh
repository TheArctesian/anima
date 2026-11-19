#!/bin/bash
# W2 prototype D: a job that needs Docker gets its OWN Docker daemon as a sidecar on a private
# network (docker:dind, then docker:dind-rootless), never the socket of the machine. Measures
# the flags each needs, start time, idle memory, an inner `docker run hello-world`, clean-up.
# Usage: docker-in-job.sh > results/docker-in-job-docker-<version>.log   (skips if no daemon)
set -u
PATH=/usr/local/bin:$PATH
L=anima-w2d=$$
NET=w2d-net-$$
echo "== docker-in-job.sh  $(date -u +%FT%TZ)  macOS $(sw_vers -productVersion) $(uname -m)"
echo "uptime: $(uptime)"
echo "note: every timing below was taken under the load shown in that uptime line"
/usr/bin/timeout 10 docker info >/dev/null 2>&1 || { echo "docker daemon not running: D skipped"; exit 0; }
echo "docker: $(docker info --format 'server {{.ServerVersion}} {{.OperatingSystem}} kernel {{.KernelVersion}} id {{.ID}}')"
snap() { docker ps -a --format '{{.ID}} {{.Names}}' | sort; echo --; docker network ls --format '{{.ID}} {{.Name}}' | sort; echo --; docker volume ls -q | sort; }
BEFORE=$(snap)
echo "before: $(docker ps -aq | wc -l | tr -d ' ') containers, $(docker network ls -q | wc -l | tr -d ' ') networks, $(docker volume ls -q | wc -l | tr -d ' ') volumes"
now() { perl -MTime::HiRes=time -e 'printf "%.0f", time*1000'; }
cleanup() {
	left=$(docker ps -aq --filter "label=$L")
	# shellcheck disable=SC2086
	[ -n "$left" ] && docker rm -f -v $left >/dev/null 2>&1
	vols=$(docker volume ls -q --filter "label=$L")
	# shellcheck disable=SC2086
	[ -n "$vols" ] && docker volume rm $vols >/dev/null 2>&1
	docker network rm "$NET" >/dev/null 2>&1
}
trap cleanup EXIT
trap 'exit 1' INT TERM
docker network create --label "$L" "$NET" >/dev/null
MARK=6$$   # the inner long-running container is `sleep $MARK`, counted in the VM's process list
vm_sleeps() { docker run --rm --pid=host --label "$L" alpine:3 ps -o args | grep -c "^sleep $MARK"; }

# docker CLI inside the sidecar; the rootless image has its socket under /run/user/1000
inner() { docker exec w2d-side sh -c '[ -S /run/user/1000/docker.sock ] && export DOCKER_HOST=unix:///run/user/1000/docker.sock; exec docker "$@"' sh "$@"; }
# try <image> <data dir in the sidecar> <job user> <sidecar flags...>: 0 if hello-world ran inside
try() {
	img=$1 data=$2 juser=$3; shift 3
	for v in certs data; do docker volume create --label "$L" "w2d-$v" >/dev/null; done
	echo "-- sidecar flags: ${*:-(none)}"
	t=$(now)
	docker run -d --name w2d-side --label "$L" --network "$NET" --network-alias docker "$@" \
		-e DOCKER_TLS_CERTDIR=/certs -v w2d-certs:/certs/client -v "w2d-data:$data" "$img" >/dev/null 2>"$TMPF" ||
		{ echo "   docker run failed: $(head -1 "$TMPF")"; return 1; }
	up=no
	while [ $(($(now) - t)) -lt 60000 ]; do
		[ "$(docker inspect w2d-side --format '{{.State.Running}}' 2>/dev/null)" = true ] || break
		inner version --format '{{.Server.Version}}' >/dev/null 2>&1 && { up=yes; break; }
		sleep 0.3
	done
	if [ $up = no ]; then
		echo "   inner daemon did NOT start: state=$(docker inspect w2d-side --format '{{.State.Status}} exit={{.State.ExitCode}}')"
		docker logs w2d-side 2>&1 | grep -iE 'error|denied|permitted|fail' | head -4 | cut -c1-200 | sed 's/^/   log: /'
		return 1
	fi
	echo "   inner daemon answers after $(($(now) - t)) ms (docker run -d to first 'docker version' by docker exec, poll about 0.4 s)"
	sleep 5
	echo "   idle sidecar: $(docker stats --no-stream --format 'mem {{.MemUsage}} pids {{.PIDs}}' w2d-side)   privileged=$(docker inspect w2d-side --format '{{.HostConfig.Privileged}} secopt={{.HostConfig.SecurityOpt}} user={{.Config.User}}')"
	echo "   inner daemon: $(inner info --format 'id {{.ID}} security={{range .SecurityOptions}}{{.}} {{end}} driver {{.Driver}}' 2>&1 | cut -c1-200)"
	# the job container: hardened, no socket, talks TLS to the sidecar by its network name
	J=(docker run --rm --label "$L" --network "$NET" --cap-drop ALL --security-opt no-new-privileges --user "$juser"
		-e DOCKER_HOST=tcp://docker:2376 -e DOCKER_TLS_VERIFY=1 -e DOCKER_CERT_PATH=/certs/client -v w2d-certs:/certs/client:ro)
	echo "   job container (user $juser, cap-drop ALL): host socket inside: $("${J[@]}" "$img" sh -c '[ -e /var/run/docker.sock ] && echo YES || echo no' 2>&1 | tail -1)"
	t=$(now)
	out=$("${J[@]}" "$img" docker run --name inner-hello hello-world 2>&1)
	hrc=$?
	echo "   job: docker run hello-world over TLS: rc=$hrc in $(($(now) - t)) ms (with the inner pull): $(echo "$out" | grep -m1 -E 'Hello from Docker|rror|denied' | cut -c1-420)"
	docker save alpine:3 | "${J[@]}" -i "$img" docker load >/dev/null 2>&1
	"${J[@]}" "$img" docker run -d --name inner-sleep alpine:3 sleep "$MARK" >/dev/null 2>"$TMPF"
	echo "   job: inner long-running container: rc=$? $(head -1 "$TMPF" | cut -c1-160)"
	echo "   inner docker ps -a: $("${J[@]}" "$img" docker ps -a --format '{{.Names}}={{.State}}' 2>&1 | tr '\n' ' ')"
	echo "   host docker ps -a shows inner containers: $(docker ps -a --format '{{.Names}}' | grep -c '^inner-')   'sleep $MARK' processes in the VM: $(vm_sleeps)"
	return $hrc
}
remove() {
	t=$(now)
	docker rm -f -v w2d-side >/dev/null 2>&1
	docker volume rm w2d-certs w2d-data >/dev/null 2>&1
	echo "   sidecar + 2 volumes removed in $(($(now) - t)) ms; 'sleep $MARK' processes left in the VM: $(vm_sleeps); volumes left: $(docker volume ls -q --filter "label=$L" | wc -l | tr -d ' ')"
}
# variant <image> <data dir> <job user> <compressed size>: pull (3 min limit), then flag sets from weak to strong
variant() {
	echo "== $1"
	t=$(now)
	/usr/bin/timeout 180 docker pull -q "$1" >/dev/null 2>&1 || { echo "pull failed or over 3 min: not measured"; return; }
	size=$(docker image inspect "$1" --format '{{.Size}}')
	echo "image: $size B unpacked on disk, pull took $(($(now) - t)) ms (short if the layers were local); compressed arm64 pull size, read once by hand from 'docker manifest inspect -v' on 2026-11-19: $4 B"
	U="--security-opt seccomp=unconfined --security-opt apparmor=unconfined"
	for flags in "" "$U" "--device /dev/net/tun" "--security-opt seccomp=unconfined --device /dev/net/tun" "$U --device /dev/net/tun" \
		"$U --security-opt systempaths=unconfined --device /dev/net/tun" "$U --cap-add SYS_ADMIN --cap-add NET_ADMIN" "--privileged"; do
		# shellcheck disable=SC2086
		if try "$1" "$2" "$3" $flags; then remove; echo "   weakest flag set with a working inner hello-world: ${flags:-(none)}"; return; fi
		remove
	done
	echo "   no flag set worked"
}
TMPF=$(mktemp)
variant docker:dind /var/lib/docker 1000:1000 106185872
variant docker:dind-rootless /home/rootless/.local/share/docker 1000:1000 121552092
rm -f "$TMPF"
cleanup
AFTER=$(snap)
echo "== clean-up: after: $(docker ps -aq | wc -l | tr -d ' ') containers, $(docker network ls -q | wc -l | tr -d ' ') networks, $(docker volume ls -q | wc -l | tr -d ' ') volumes"
echo "docker ps -a + network ls + volume ls identical before/after: $([ "$BEFORE" = "$AFTER" ] && echo yes || { echo NO; diff <(echo "$BEFORE") <(echo "$AFTER"); })"
echo "note: the pulled images stay (images are not removed): $(docker images --format '{{.Repository}}:{{.Tag}}' | grep -E '^(docker|alpine):' | tr '\n' ' ')"
echo "uptime at end: $(uptime)"
