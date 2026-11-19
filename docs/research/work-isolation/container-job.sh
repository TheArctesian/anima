#!/bin/bash
# W2 prototype C: one job in a new, hardened container with no Docker socket and one new empty
# job folder as the only host path. Measures cold start (median of 10), idle memory, what the
# job sees inside, and that container and folder are gone after a normal end and a docker kill.
# Usage: container-job.sh > results/container-job-docker-<version>.log   (skips if no daemon)
set -u
PATH=/usr/local/bin:$PATH
L=anima-w2c=$$
echo "== container-job.sh  $(date -u +%FT%TZ)  macOS $(sw_vers -productVersion) $(uname -m)"
echo "uptime: $(uptime)"
echo "note: every timing below was taken under the load shown in that uptime line"
if ! /usr/bin/timeout 10 docker info >/dev/null 2>&1; then
	echo "docker daemon not running: C skipped"
	exit 0
fi
echo "docker: $(docker info --format 'server {{.ServerVersion}} {{.OperatingSystem}} kernel {{.KernelVersion}} {{.NCPU}} cpu {{.MemTotal}} B, security {{.SecurityOptions}}')"
IMG=$(docker images --format '{{.Repository}}:{{.Tag}}' | grep -E '^(alpine|busybox|debian):' | head -1)
if [ -z "$IMG" ]; then
	IMG=alpine:3
	echo "no small local image: pulling $IMG (the one pull of this script)"
	/usr/bin/timeout 180 docker pull -q "$IMG" || { echo "pull failed or over 3 min: not measured"; exit 0; }
fi
echo "image: $IMG $(docker image inspect "$IMG" --format '{{.Size}} B {{.Architecture}}')"
BEFORE=$(docker ps -aq | sort)
now() { perl -MTime::HiRes=time -e 'printf "%.0f", time*1000'; }
gone() { [ -z "$(docker ps -aq --filter "name=^$1\$")" ] && echo yes || echo NO; }
JOB=
cleanup() {
	left=$(docker ps -aq --filter "label=$L")
	# shellcheck disable=SC2086
	[ -n "$left" ] && docker rm -f $left >/dev/null
	[ -n "$JOB" ] && rm -rf "$JOB"
}
trap cleanup EXIT
trap 'exit 1' INT TERM
# job <name> <docker run options...> -- <command...>: a new folder, then the hardened container
job() {
	name=$1; shift
	JOB=$(cd "$(mktemp -d)" && pwd -P)
	opts=()
	while [ "$1" != -- ]; do opts+=("$1"); shift; done
	shift
	docker run --rm --name "$name" --label "$L" ${opts[@]+"${opts[@]}"} \
		--cap-drop ALL --security-opt no-new-privileges --pids-limit 512 --memory 2g \
		--read-only --tmpfs /work:rw,exec,size=256m,uid=1000,gid=1000 --user 1000:1000 \
		-v "$JOB:/job" -w /work "$IMG" "$@"
}
end_job() { rm -rf "$JOB"; echo "  container gone: $(gone "$1")  job folder gone: $([ ! -e "$JOB" ] && echo yes || echo NO)"; JOB=; }

echo "== flags of every job: --rm --cap-drop ALL --security-opt no-new-privileges --pids-limit 512 --memory 2g --read-only --tmpfs /work --user 1000:1000 -v <new folder>:/job (no socket mount)"
echo "== C1 start of a new container to the end of its first command (docker run ... /bin/true; image local, daemon up), wall clock, 10 runs"
for i in $(seq 10); do
	t=$(now); job "w2c-start-$i" -- /bin/true; echo $(($(now) - t)); rm -rf "$JOB"
done | sort -n | awk '{a[NR]=$1} END{printf "median %d ms  min %d  max %d  (n=%d)\n", (a[5]+a[6])/2, a[1], a[NR], NR}'
JOB=

echo "== C2 what the job sees (one normal run to the end)"
job w2c-see -- sh -c '
	echo "  id: $(id)"
	echo "  caps: $(grep -E "^Cap(Prm|Eff|Bnd)" /proc/self/status | tr "\n\t" "  ")  $(grep NoNewPrivs /proc/self/status | tr "\t" " ")"
	echo "  /var/run/docker.sock present: $([ -e /var/run/docker.sock ] && echo YES || echo no)   /run/docker.sock: $([ -e /run/docker.sock ] && echo YES || echo no)"
	echo "  write /: $(touch /x 2>&1 || true)"
	echo "  write /work: $(touch /work/x 2>&1 && echo ok)   write /job: $(echo made-by-job >/job/out.txt 2>&1 && echo ok)"
	echo "  exec from /work: $(cp /bin/busybox /work/echo 2>&1 && /work/echo ok)"
	echo "  mounts that are not proc/sys/dev/tmpfs/overlay:"; grep -vE "^(proc|sysfs|tmpfs|devpts|mqueue|shm|cgroup|overlay|devtmpfs) " /proc/mounts | sed "s/^/    /"
	echo "  memory limit: $(cat /sys/fs/cgroup/memory.max 2>&1)  pids limit: $(cat /sys/fs/cgroup/pids.max 2>&1)"
'
echo "  file from the job on the host: $(ls -ln "$JOB/out.txt" 2>&1 | awk '{print $1, $3":"$4, $9}')"
end_job w2c-see

echo "== C3 idle container: memory, mounts, then docker kill in the middle of a sleep"
job w2c-idle -d -- sleep 300 >/dev/null
sleep 3
echo "  docker stats --no-stream: $(docker stats --no-stream --format 'mem {{.MemUsage}} pids {{.PIDs}} cpu {{.CPUPerc}}' w2c-idle)"
echo "  mounts (docker inspect): $(docker inspect w2c-idle --format '{{range .Mounts}}[{{.Type}} {{.Source}} -> {{.Destination}} rw={{.RW}}] {{end}}')"
echo "  tmpfs: $(docker inspect w2c-idle --format '{{.HostConfig.Tmpfs}}')  privileged=$(docker inspect w2c-idle --format '{{.HostConfig.Privileged}} readonly-rootfs={{.HostConfig.ReadonlyRootfs}} capdrop={{.HostConfig.CapDrop}} secopt={{.HostConfig.SecurityOpt}}')"
t=$(now)
docker kill w2c-idle >/dev/null
while [ "$(gone w2c-idle)" = NO ]; do sleep 0.1; done
echo "  docker kill -> container removed by --rm: $(($(now) - t)) ms"
end_job w2c-idle

AFTER=$(docker ps -aq | sort)
echo "== clean-up: containers with label $L left: $(docker ps -aq --filter "label=$L" | wc -l | tr -d ' ')  docker ps -a same as before: $([ "$BEFORE" = "$AFTER" ] && echo yes || echo NO)"
echo "note: the image $IMG stays (images are not removed)"
echo "uptime at end: $(uptime)"
