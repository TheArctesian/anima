#!/bin/bash
# W2 prototype E: the measurements of Sections 4.1 and 4.2 on a Linux computer, with Podman and no
# root right. Throwaway code of a prototype, not code of the product. It installs nothing, uses no
# sudo, and removes each container that it made; the pulled images stay.
# Usage, as the user who will own the jobs:  bash linux-job.sh
# It writes ~/w2-linux.log. The log holds no address and no name of the computer.
set -u
LOG=$HOME/w2-linux.log
IMG=docker.io/library/alpine:3
L=anima-w2e=$$
now() { date +%s%3N; }
FLAGS=(--cap-drop ALL --security-opt no-new-privileges --pids-limit 512 --memory 2g --read-only
	--tmpfs /work:rw,exec,size=64m --user 1000:1000 -w /work --label "$L")
job() { podman run --rm "${FLAGS[@]}" "$@"; }
cleanup() {
	left=$(podman ps -aq --filter "label=$L")
	# shellcheck disable=SC2086
	[ -n "$left" ] && podman rm -f $left >/dev/null 2>&1
	[ -n "${SERVER:-}" ] && kill "$SERVER" 2>/dev/null
	[ -n "${JOB:-}" ] && rm -rf "$JOB"
}
trap cleanup EXIT
trap 'exit 1' INT TERM

main() {
	echo "== linux-job.sh  $(date -u +%FT%TZ)"
	# shellcheck disable=SC1091
	echo "system: $(. /etc/os-release && echo "$PRETTY_NAME")  kernel $(uname -r) $(uname -m)  $(nproc) cpu  $(awk '/MemTotal/{printf "%.0f GiB", $2/1048576}' /proc/meminfo)"
	echo "load: $(cut -d' ' -f1-3 /proc/loadavg)   user is root: $([ "$(id -u)" = 0 ] && echo YES || echo no)"
	command -v podman >/dev/null || { echo "podman is not installed: nothing measured"; return; }
	echo "podman: $(podman info --format 'version {{.Version.Version}} rootless={{.Host.Security.Rootless}} cgroup={{.Host.CgroupsVersion}} manager={{.Host.CgroupManager}} runtime={{.Host.OCIRuntime.Name}} driver={{.Store.GraphDriverName}}' 2>&1)"
	echo "controllers given to this user: $(cat "/sys/fs/cgroup/user.slice/user-$(id -u).slice/user@$(id -u).service/cgroup.controllers" 2>&1)"
	timeout 180 podman pull -q "$IMG" >/dev/null 2>&1 || { echo "pull of $IMG failed or over 3 min: nothing measured"; return; }
	BEFORE=$(podman ps -aq | sort)

	plain=$(podman run --rm "$IMG" /bin/true 2>&1) || { echo "podman cannot run a container at all on this computer: $(echo "$plain" | head -3 | tr '\n' ' ')"; echo "== stopped: nothing measured"; return; }
	flagged=$(job "$IMG" /bin/true 2>&1) || echo "WARNING: a container with the flags of a job does not start; the results below are not valid: $(echo "$flagged" | head -3 | tr '\n' ' ')"

	echo "== E1 start of a new container to the end of its first command, 10 runs"
	for _ in $(seq 10); do t=$(now); job "$IMG" /bin/true; echo $(($(now) - t)); done | sort -n |
		awk '{a[NR]=$1} END{printf "median %d ms  min %d  max %d  (n=%d)\n", (a[5]+a[6])/2, a[1], a[NR], NR}'

	echo "== E2 what the job sees"
	JOB=$(mktemp -d)
	job --userns keep-id -v "$JOB:/job" "$IMG" sh -c '
		echo "  id: $(id)"
		echo "  caps: $(grep -E "^Cap(Prm|Eff|Bnd)" /proc/self/status | tr "\n\t" "  ")  $(grep NoNewPrivs /proc/self/status | tr "\t" " ")"
		echo "  write /: $(touch /x 2>&1 || true)"
		echo "  write /work: $(touch /work/x 2>&1 && echo ok)   write /job: $(echo made-by-job >/job/out.txt 2>&1 && echo ok)"
		echo "  a container socket inside: $(ls /run/podman/podman.sock /var/run/docker.sock 2>/dev/null | wc -l)"
		echo "  memory limit: $(cat /sys/fs/cgroup/memory.max 2>&1)  pids limit: $(cat /sys/fs/cgroup/pids.max 2>&1)"
	' 2>&1
	echo "  the file of the job belongs to this user: $([ -O "$JOB/out.txt" ] && echo yes || echo NO)"
	rm -rf "$JOB"; JOB=

	echo "== E3 the limits hold"
	job --memory 256m --memory-swap 256m "$IMG" sh -c 'tail /dev/zero' >/dev/null 2>&1
	echo "  a job that fills the memory (limit 256 MiB): exit $? (137 = stopped by the kernel)"
	echo "  a job that fills its work folder (limit 64 MiB): $(job "$IMG" sh -c 'dd if=/dev/zero of=/work/fill bs=1M count=200 2>&1 | grep -m1 -o "No space left on device"; ls -l /work/fill | awk "{print \$5, \"bytes written\"}"' 2>&1 | tr '\n' ' ')"
	echo "  a job that starts 300 processes (limit 64): $(job --pids-limit 64 "$IMG" sh -c 'i=0; while [ $i -lt 300 ]; do sleep 20 & i=$((i+1)); done 2>&1 | grep -c "fork"; cat /sys/fs/cgroup/pids.current' 2>&1 | tr '\n' ' ') (refused starts, then processes that ran)"
	t=$(now); timeout 5 podman run --rm "${FLAGS[@]}" "$IMG" sleep 60 >/dev/null 2>&1
	echo "  a job past its time (limit 5 s by timeout): exit $? after $(($(now) - t)) ms; container left: $(podman ps -aq --filter "label=$L" | wc -l)"
	cleanup_left=$(podman ps -aq --filter "label=$L"); [ -n "$cleanup_left" ] && podman rm -f $cleanup_left >/dev/null 2>&1 # shellcheck disable=SC2086

	echo "== E4 what a job reaches on the network, with no rule added"
	PORT=$((20000 + RANDOM % 20000))
	python3 -m http.server "$PORT" --bind 0.0.0.0 --directory "$(mktemp -d)" >/dev/null 2>&1 &
	SERVER=$!
	sleep 1
	LAN=$(ip -4 route get 1.1.1.1 2>/dev/null | sed -n 's/.* src \([0-9.]*\).*/\1/p')
	LINK=$(ip -4 -o addr 2>/dev/null | awk '$4 ~ /^100\.(6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\./ {sub("/.*", "", $4); print $4; exit}')
	GATE=$(ip -4 route show default 2>/dev/null | awk '{print $3; exit}')
	reach() { job "$IMG" sh -c "wget -q -T 4 -O /dev/null http://$1 2>/dev/null && echo REACHED || echo 'not reached'" 2>&1 | tail -1; }
	echo "  network mode: $(podman info --format '{{.Host.NetworkBackend}}' 2>/dev/null) $(command -v pasta >/dev/null && echo pasta) $(command -v slirp4netns >/dev/null && echo slirp4netns)"
	echo "  the public internet (example.com): $(reach example.com)"
	echo "  a server on the loopback of the computer, by 10.0.2.2: $(reach "10.0.2.2:$PORT")"
	echo "  the same server by the name host.containers.internal: $(reach "host.containers.internal:$PORT")"
	echo "  the same server by the address of the computer on its network: $([ -n "$LAN" ] && reach "$LAN:$PORT" || echo 'no address found')"
	echo "  the same server by the address of the link or VPN (100.64.0.0/10): $([ -n "$LINK" ] && reach "$LINK:$PORT" || echo 'no such address on this computer')"
	echo "  the router of the network, port 80: $([ -n "$GATE" ] && reach "$GATE" || echo 'no router found')"
	echo "  with --network none, the public internet: $(job --network none "$IMG" sh -c 'wget -q -T 4 -O /dev/null http://example.com 2>/dev/null && echo REACHED || echo "not reached"' 2>&1 | tail -1)"
	kill "$SERVER" 2>/dev/null; SERVER=

	echo "== E5 idle memory, and what stays after kill -9 of the program that holds the job"
	podman run -d --name "w2e-idle-$$" "${FLAGS[@]}" "$IMG" sleep 300 >/dev/null 2>&1
	sleep 3
	echo "  idle: $(podman stats --no-stream --format 'mem {{.MemUsage}} pids {{.PIDs}}' "w2e-idle-$$" 2>&1)"
	CONMON=$(podman inspect --format '{{.State.ConmonPid}}' "w2e-idle-$$" 2>/dev/null)
	PID=$(podman inspect --format '{{.State.Pid}}' "w2e-idle-$$" 2>/dev/null)
	kill -9 "$CONMON" 2>/dev/null
	sleep 2
	echo "  after kill -9 of conmon: the process of the job still runs: $([ -d "/proc/$PID" ] && echo YES || echo no); podman says: $(podman inspect --format '{{.State.Status}}' "w2e-idle-$$" 2>&1 | tail -1)"
	t=$(now); podman rm -f "w2e-idle-$$" >/dev/null 2>&1
	echo "  podman rm -f: $(($(now) - t)) ms; the process is gone: $([ -d "/proc/$PID" ] && echo NO || echo yes)"

	echo "== E6 a job that needs Docker: a rootless Docker service in a rootless container (Section 4.2)"
	if timeout 240 podman pull -q docker.io/library/docker:dind-rootless >/dev/null 2>&1; then
		for flags in "" "--security-opt seccomp=unconfined --device /dev/net/tun" "--security-opt seccomp=unconfined --security-opt unmask=ALL --device /dev/net/tun" "--privileged"; do
			t=$(now)
			# shellcheck disable=SC2086
			podman run -d --name "w2e-dind-$$" --label "$L" $flags -e DOCKER_TLS_CERTDIR= docker.io/library/docker:dind-rootless >/dev/null 2>&1
			up=no
			while [ $(($(now) - t)) -lt 40000 ]; do
				podman exec "w2e-dind-$$" docker -H unix:///run/user/1000/docker.sock version >/dev/null 2>&1 && { up=yes; break; }
				sleep 0.5
			done
			echo "  flags: ${flags:-(none)}: the service answers: $up$([ $up = yes ] && echo " after $(($(now) - t)) ms")"
			if [ $up = yes ]; then
				echo "    idle: $(podman stats --no-stream --format 'mem {{.MemUsage}} pids {{.PIDs}}' "w2e-dind-$$" 2>&1)"
				echo "    hello-world inside: $(timeout 120 podman exec "w2e-dind-$$" docker -H unix:///run/user/1000/docker.sock run --rm hello-world 2>&1 | grep -m1 -E 'Hello from Docker|rror|denied' | cut -c1-200)"
			fi
			podman rm -f -v "w2e-dind-$$" >/dev/null 2>&1
			[ $up = yes ] && break
		done
	else
		echo "  pull of docker:dind-rootless failed or over 4 min: not measured"
	fi

	echo "== clean-up: containers of this script left: $(podman ps -aq --filter "label=$L" | wc -l); podman ps -a same as before: $([ "$BEFORE" = "$(podman ps -aq | sort)" ] && echo yes || echo NO)"
	echo "load at end: $(cut -d' ' -f1-3 /proc/loadavg)"
	echo "== done"
}
main 2>&1 | tee "$LOG"
echo "The log is $LOG"
