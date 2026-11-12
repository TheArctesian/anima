#!/usr/bin/env bash
# Harden the Anima VPS (AlmaLinux 10): SSH keys only, automatic updates, SELinux labels for
# Headscale, Docker loopback publishing and the nftables firewall with an automatic rollback.
# Spec VPS-001, VPS-002, VPS-004, VPS-005, DNS-005; decisions D-9, D-10 and D-11.
#
# The operator runs each command by hand on the VPS (VPS-005). This script never prints
# key material: it only tests that authorized_keys holds a key. The other commands are in
# sshd.sh, selinux.sh and firewall.sh; open-ports.sh decides which ports are open to the
# internet, and inventory.sh prints the read-only listing that `check` starts with.
# shellcheck source-path=SCRIPTDIR
set -euo pipefail

# System commands. Override from the environment to test against fakes (D-14).
NFT=${HARDEN_NFT:-nft}
SYSTEMCTL=${HARDEN_SYSTEMCTL:-systemctl}
SYSTEMD_RUN=${HARDEN_SYSTEMD_RUN:-systemd-run}
SSHD=${HARDEN_SSHD:-sshd}
SS=${HARDEN_SS:-ss}
DNF=${HARDEN_DNF:-dnf}
FIREWALL_CMD=${HARDEN_FIREWALL_CMD:-firewall-cmd}
SEMANAGE=${HARDEN_SEMANAGE:-semanage}
RESTORECON=${HARDEN_RESTORECON:-restorecon}
GETENFORCE=${HARDEN_GETENFORCE:-getenforce}
STAT=${HARDEN_STAT:-stat}
PS=${HARDEN_PS:-ps}
IP=${HARDEN_IP:-ip}
ID=${HARDEN_ID:-id}
GETENT=${HARDEN_GETENT:-getent}
JQ=${HARDEN_JQ:-jq}
DOCKER=${HARDEN_DOCKER:-docker}
UNAME=${HARDEN_UNAME:-uname}
GROUPADD=${HARDEN_GROUPADD:-groupadd}
USERADD=${HARDEN_USERADD:-useradd}
RUNUSER=${HARDEN_RUNUSER:-runuser}

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SELF=$HERE/$(basename "${BASH_SOURCE[0]}")
ROOT=/
DRY_RUN=0
CONSOLE=0
RESTART_DOCKER=0
LIVE_RESTORE_WAIT=${HARDEN_LIVE_RESTORE_WAIT:-10}
ALLOW_CLOSING=
BROKER_SHA256=
PROBE_KEY=
STATE_DIR=/var/lib/anima
HEADSCALE_DIR=/opt/anima/headscale
SSHD_DROPIN=/etc/ssh/sshd_config.d/00-anima.conf
DNF_TIMER_DROPIN=/etc/systemd/system/dnf-automatic.timer.d/anima.conf
# Headscale's bind mounts (compose.yaml). container_file_t:s0 is the label `:z` would set.
LABEL_SPEC='/opt/anima/headscale/(config|lib)(/.*)?'
ADMIN_GROUP=wheel
USER_NAME_PATTERN='^[a-z_][a-z0-9_-]{0,31}$'
KEY_LINE_PATTERN='(^|[[:space:]])(ssh-|ecdsa-|sk-)[^[:space:]]+[[:space:]]+AAAA'

usage() {
  cat <<EOF
Usage: $SELF [--root DIR] [--dry-run] [--console] [--restart-docker]
             [--allow-closing PORT/PROTO,...] [--sha256=HASH] [--key=FILE] <command>

Commands (run as root):
  check     Print a read-only inventory (listeners and their units, containers, Docker
            networks, extra enabled units, firewalld, nftables tables), then report OS,
            admin user, the hub's SSH user anima-probe, effective sshd values, updates,
            SELinux and firewall state.
            Changes nothing.
  ssh       Install the sshd drop-in (keys only, no root, AllowUsers \$ANIMA_ADMIN_USER from
            the VPN only and anima-probe from the hub container only, VP-13),
            verify it with sshd -t, sshd -T and sshd -T -C for root and the admin user,
            then reload sshd. Refuses while sshd_config or a file it includes has an
            active Match line. ANIMA_ADMIN_USER must be a non-root member of group
            $ADMIN_GROUP with a key in ~/.ssh/authorized_keys (VPS_PUBLIC_IPV4, if set,
            is the public address the -C checks use):
            sudo --preserve-env=ANIMA_ADMIN_USER,VPS_PUBLIC_IPV4 $SELF ssh
  probe     Make anima-probe, the hub's SSH user (VP-13): uid and gid 10002, which the
            firewall confines, its own group only, no password, home /home/anima-probe. Then
            install the hub's public key (--key=FILE, one line as ssh-keygen wrote it) as its
            only key, with restrict,pty,from="172.30.0.10". Run it again to change the key.
  updates   Install dnf-automatic: all updates daily at 03:45 UTC, reboot at 04:30 when
            needed.
  selinux   Label Headscale's config/ and lib/ container_file_t (semanage, restorecon).
            With SELinux disabled it refuses, unless /etc/anima/selinux-disabled
            records the deviation (R-050).
  docker    Merge daemon.json into /etc/docker/daemon.json (default bind 127.0.0.1,
            SELinux labels on, live-restore, no userland proxy). Needs jq and, under SELinux, the selinux
            step first. Only with --restart-docker: reload Docker (turns live-restore
            on), then restart it; running containers keep running.
  firewall  Validate and apply nftables.conf, armed with a 180 s rollback. Refuses while
            a public port that is open now would close (see --allow-closing and
            /etc/anima/firewall-allow). Needs tailscale0 and an SSH session over the VPN,
            or --console when you use the Hostinger browser console:
            sudo --preserve-env=SSH_CONNECTION $SELF firewall
  confirm   From a NEW SSH session over the VPN (same sudo form as firewall): cancel
            the rollback, install the rules for boot and mask firewalld.
  broker    Install the Anima broker (R-055): the binary copied first to
            /usr/local/libexec/anima-broker.new (scp) and checked against --sha256,
            /etc/anima/broker.conf if absent (max_nodes = the node count + 5), the
            socket, service and audit timer units from ../broker/; restart the socket
            and the timer, then ask the broker for the node list.

Options:
  --root DIR        Treat DIR as the file system root (tests). Default /.
  --dry-run         Print the actions; write and change nothing.
  --console         firewall, confirm: you are on the provider console, not on SSH.
  --restart-docker  docker: reload, then restart Docker, keeping containers running.
  --allow-closing L firewall, confirm: close these open ports on purpose, for example
                    --allow-closing 9090/tcp,546/udp. Ports to keep open go in
                    /etc/anima/firewall-allow, one PORT/PROTO per line.
  --sha256=HASH     broker: the SHA-256 of anima-broker.new, as broker-build.ts printed it.
  --key=FILE        probe: the hub's public key file, for example /tmp/anima-probe.pub.

Exit codes: 0 done; 1 a step failed or check found violations; 2 refused because a
precondition is not met (nothing changed); 64 usage error.
EOF
}

say() { printf '%s\n' "$*"; }
refuse() { printf 'harden.sh: refused: %s\n' "$*" >&2; exit 2; }
fail() { printf 'harden.sh: error: %s\n' "$*" >&2; exit 1; }
usage_error() { usage >&2; exit 64; }
dry() { [ "$DRY_RUN" -eq 1 ]; }
path() { printf '%s%s' "${ROOT%/}" "$1"; }

# Run a command that changes the system, or only print it in a dry run.
run() {
  if dry; then say "[dry-run] $*"; else "$@"; fi
}

# Write standard input to a system path atomically (temporary file, then rename). The
# temporary file is created in the target folder, so it takes that folder's SELinux type.
write_file() {
  local target mode=$2 tmp
  target=$(path "$1")
  if dry; then say "[dry-run] write $target (mode $mode)"; return; fi
  mkdir -p "$(dirname "$target")"
  tmp=$(mktemp "$target.XXXXXX")
  cat >"$tmp"
  chmod "$mode" "$tmp"
  mv -f "$tmp" "$target"
}

# Keep a copy of a system file before replacing it; with "once", keep the first copy.
save_previous() {
  local source target
  source=$(path "$1")
  target=$(path "$STATE_DIR/$2")
  [ -f "$source" ] || return 0
  if [ "${3:-}" = once ] && [ -e "$target" ]; then return 0; fi
  if dry; then say "[dry-run] copy $source to $target"; return; fi
  mkdir -p "$(dirname "$target")"
  cp -p "$source" "$target"
}

require_root() {
  if [ "$ROOT" = / ] && ! dry && [ "$EUID" -ne 0 ]; then refuse "run as root (sudo)"; fi
}

cmd_updates() {
  require_root
  run "$DNF" install -y dnf-automatic
  save_previous /etc/dnf/automatic.conf automatic.conf.previous once
  write_file /etc/dnf/automatic.conf 0644 <"$HERE/dnf-automatic.conf"
  write_file "$DNF_TIMER_DROPIN" 0644 <"$HERE/dnf-automatic-timer.conf"
  run "$SYSTEMCTL" daemon-reload
  run "$SYSTEMCTL" enable --now dnf-automatic.timer
  say "Automatic updates enabled: daily at 03:45 UTC, reboot at 04:30 UTC when needed."
}

update_problems() {
  local conf line
  conf=$(path /etc/dnf/automatic.conf)
  $SYSTEMCTL is-enabled --quiet dnf-automatic.timer || say "dnf-automatic.timer is not enabled"
  [ -f "$(path "$DNF_TIMER_DROPIN")" ] || say "dnf-automatic.timer has no Anima schedule ($DNF_TIMER_DROPIN)"
  for line in "upgrade_type = default" "apply_updates = yes" "reboot = when-needed"; do
    grep -qxF "$line" "$conf" 2>/dev/null || say "/etc/dnf/automatic.conf: expected \"$line\""
  done
}

os_warnings() {
  local release version
  release=$(path /etc/os-release)
  if ! grep -qxE 'ID="?almalinux"?' "$release" 2>/dev/null || ! grep -qxE 'VERSION_ID="?10(\.[0-9]+)?"?' "$release"; then
    say "warning: not AlmaLinux 10; this script is written for it"
  fi
}

# Print a problem unless the Docker daemon is version 28 or newer. Before 28 Docker turns
# on route_localnet, so a port published on 127.0.0.1 is reachable from the network, and
# the Anima setup (daemon.json, WP-04's preflight) relies on 28's behaviour.
docker_version_problem() {
  local version
  if ! version=$($DOCKER version --format '{{.Server.Version}}' 2>/dev/null); then
    say "cannot read the Docker version (docker version); Docker 28 or newer is required"
  elif ! [[ ${version%%.*} =~ ^[0-9]+$ ]]; then
    say "cannot tell the Docker version from \"$version\"; Docker 28 or newer is required"
  elif [ "${version%%.*}" -lt 28 ]; then
    say "Docker $version is older than 28: ports published on loopback are reachable; upgrade Docker"
  fi
}

# Print "ID STARTED_AT" for each running container. A container that Docker stopped and
# started again keeps its ID but gets a new start time.
container_starts() {
  local ids
  ids=$($DOCKER ps -q --no-trunc) || return 1
  [ -n "$ids" ] || return 0
  read -r -a ids <<<"${ids//$'\n'/ }"
  $DOCKER inspect -f '{{.Id}} {{.State.StartedAt}}' "${ids[@]}"
}

# Wait up to LIVE_RESTORE_WAIT seconds for live-restore: dockerd applies a reload after
# `systemctl reload` returns. Fail unless it is on.
await_live_restore() {
  local tries=0 state
  while :; do
    state=$($DOCKER info --format '{{.LiveRestoreEnabled}}' 2>/dev/null) \
      || fail "docker info failed after the reload; Docker was not restarted"
    [ "$state" != true ] || return 0
    tries=$((tries + 1))
    [ "$tries" -lt "$LIVE_RESTORE_WAIT" ] \
      || fail "live-restore is not on after the reload; Docker was not restarted (a restart would stop every container)"
    sleep 1
  done
}

# Restart Docker without stopping a container (R-050). A daemon started without
# live-restore stops every container when it stops, so first reload it (SIGHUP), which
# turns live-restore on from daemon.json; the other options need the restart. Refuse to
# restart unless live-restore is on, and fail naming each container that was stopped.
restart_docker_live() {
  local before after line id gone=
  if dry; then say "[dry-run] $SYSTEMCTL reload docker, check live-restore, $SYSTEMCTL restart docker"; return; fi
  "$SYSTEMCTL" reload docker || fail "systemctl reload docker failed; Docker was not restarted"
  await_live_restore
  before=$(container_starts) || fail "docker ps failed; Docker was not restarted"
  "$SYSTEMCTL" restart docker || fail "systemctl restart docker failed"
  after=$(container_starts) || fail "docker ps failed after the restart"
  while IFS= read -r line; do
    [ -z "$line" ] || grep -qxF "$line" <<<"$after" || { id=${line%% *}; gone="$gone ${id:0:12}"; }
  done <<<"$before"
  [ -z "$gone" ] || fail "containers stopped by the restart:$gone; check them (docker ps -a) and start any that are down"
  say "Docker restarted; every running container kept running (live-restore)."
}

cmd_docker() {
  local target merged problems
  require_root
  command -v "$JQ" >/dev/null 2>&1 || fail "jq is required: dnf install jq"
  problems=$(docker_version_problem)
  [ -z "$problems" ] || refuse "$problems"
  # daemon.json turns on SELinux labels: an unlabelled lib/ would lock Headscale out.
  problems=$(label_problems)
  [ -z "$problems" ] || refuse "$problems; run harden.sh selinux first"
  target=$(path /etc/docker/daemon.json)
  if [ -e "$target" ]; then
    $JQ -e -s 'length == 1 and (.[0] | type == "object")' "$target" >/dev/null 2>&1 \
      || refuse "$target is not a single JSON object; fix it by hand first"
    # The Anima log settings are defaults: the operator's log-driver and log-opts win (R-050).
    # ip, selinux-enabled, live-restore and userland-proxy are forced.
    merged=$($JQ -s '.[1] * .[0] * (.[1] | del(."log-driver", ."log-opts"))' "$target" "$HERE/daemon.json") \
      || fail "jq could not merge daemon.json"
    save_previous /etc/docker/daemon.json daemon.json.previous
  else
    merged=$(cat "$HERE/daemon.json")
  fi
  write_file /etc/docker/daemon.json 0644 <<<"$merged"
  if [ "$RESTART_DOCKER" -eq 1 ]; then
    restart_docker_live
  else
    say "Apply it with: $SELF --restart-docker docker (a plain restart would stop every container)."
  fi
  say "Then recreate Headscale, which keeps its old label until then:"
  say "  docker compose -f $HEADSCALE_DIR/compose.yaml up -d --force-recreate"
}

cmd_check() {
  local problems line
  require_root
  inventory
  os_warnings
  line=$(selinux_deviation)
  if [ -n "$line" ] && [ "$($GETENFORCE 2>/dev/null)" = Disabled ]; then
    say "note: SELinux stays disabled, a recorded deviation: $line"
  fi
  problems=$(docker_version_problem; admin_problem; probe_problem; match_problems; sshd_mismatches "${ANIMA_ADMIN_USER:-}"; update_problems
    selinux_problems; firewall_problems; listener_violations)
  if [ -z "$problems" ]; then say "ok: no violations"; return 0; fi
  while IFS= read -r line; do say "VIOLATION: $line"; done <<<"$problems"
  return 1
}

# shellcheck source=sshd.sh
. "$HERE/sshd.sh"
# shellcheck source=probe.sh
. "$HERE/probe.sh"
# shellcheck source=selinux.sh
. "$HERE/selinux.sh"
# shellcheck source=open-ports.sh
. "$HERE/open-ports.sh"
# shellcheck source=firewall.sh
. "$HERE/firewall.sh"
# shellcheck source=inventory.sh
. "$HERE/inventory.sh"
# shellcheck source=broker.sh
. "$HERE/broker.sh"

main() {
  while [ $# -gt 0 ]; do
    case $1 in
      --root) [ $# -ge 2 ] || usage_error; ROOT=$2; shift 2 ;;
      --dry-run) DRY_RUN=1; shift ;;
      --console) CONSOLE=1; shift ;;
      --restart-docker) RESTART_DOCKER=1; shift ;;
      --allow-closing) [ $# -ge 2 ] || usage_error; ALLOW_CLOSING=$2; shift 2 ;;
      --sha256=*) BROKER_SHA256=${1#--sha256=}; shift ;;
      --key=*) PROBE_KEY=${1#--key=}; shift ;;
      -h | --help) usage; exit 0 ;;
      -*) usage_error ;;
      *) break ;;
    esac
  done
  [ $# -eq 1 ] || usage_error
  if [[ $ROOT != /* ]] || [ ! -d "$ROOT" ]; then
    printf 'harden.sh: --root must be an existing absolute directory\n' >&2
    exit 64
  fi
  case $1 in
    check | ssh | probe | updates | selinux | docker | firewall | confirm | broker) "cmd_$1" ;;
    *) usage_error ;;
  esac
}

main "$@"
