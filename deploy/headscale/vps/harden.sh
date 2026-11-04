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

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SELF=$HERE/$(basename "${BASH_SOURCE[0]}")
ROOT=/
DRY_RUN=0
CONSOLE=0
RESTART_DOCKER=0
ALLOW_CLOSING=
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
             [--allow-closing PORT/PROTO,...] <command>

Commands (run as root):
  check     Print a read-only inventory (listeners and their units, containers, Docker
            networks, extra enabled units, firewalld, nftables tables), then report OS,
            admin user, effective sshd values, updates, SELinux and firewall state.
            Changes nothing.
  ssh       Install the sshd drop-in (keys only, no root, AllowUsers \$ANIMA_ADMIN_USER),
            verify it with sshd -t, sshd -T and sshd -T -C for root and the admin user,
            then reload sshd. Refuses while sshd_config or a file it includes has an
            active Match line. ANIMA_ADMIN_USER must be a non-root member of group
            $ADMIN_GROUP with a key in ~/.ssh/authorized_keys (VPS_PUBLIC_IPV4, if set,
            is the public address the -C checks use):
            sudo --preserve-env=ANIMA_ADMIN_USER,VPS_PUBLIC_IPV4 $SELF ssh
  updates   Install dnf-automatic: all updates daily at 03:45 UTC, reboot at 04:30 when
            needed.
  selinux   Label Headscale's config/ and lib/ container_file_t (semanage, restorecon).
  docker    Merge daemon.json into /etc/docker/daemon.json (default bind 127.0.0.1,
            SELinux labels on). Needs jq and, under SELinux, the selinux step first.
            Docker is restarted only with --restart-docker.
  firewall  Validate and apply nftables.conf, armed with a 180 s rollback. Refuses while
            a public port that is open now would close (see --allow-closing and
            /etc/anima/firewall-allow). Needs tailscale0 and an SSH session over the VPN,
            or --console when you use the Hostinger browser console:
            sudo --preserve-env=SSH_CONNECTION $SELF firewall
  confirm   From a NEW SSH session over the VPN (same sudo form as firewall): cancel
            the rollback, install the rules for boot and mask firewalld.

Options:
  --root DIR        Treat DIR as the file system root (tests). Default /.
  --dry-run         Print the actions; write and change nothing.
  --console         firewall, confirm: you are on the provider console, not on SSH.
  --restart-docker  docker: restart Docker after the change.
  --allow-closing L firewall, confirm: close these open ports on purpose, for example
                    --allow-closing 9090/tcp,546/udp. Ports to keep open go in
                    /etc/anima/firewall-allow, one PORT/PROTO per line.

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
  if ! version=$($DOCKER version --format '{{.Server.Version}}' 2>/dev/null); then
    say "warning: cannot read the Docker version"
  elif [[ ${version%%.*} =~ ^[0-9]+$ ]] && [ "${version%%.*}" -lt 28 ]; then
    say "warning: Docker $version is older than 28; ports published on loopback are reachable"
  fi
}

cmd_docker() {
  local target merged problems
  require_root
  command -v "$JQ" >/dev/null 2>&1 || fail "jq is required: dnf install jq"
  # daemon.json turns on SELinux labels: an unlabelled lib/ would lock Headscale out.
  problems=$(label_problems)
  [ -z "$problems" ] || refuse "$problems; run harden.sh selinux first"
  target=$(path /etc/docker/daemon.json)
  if [ -e "$target" ]; then
    $JQ -e -s 'length == 1 and (.[0] | type == "object")' "$target" >/dev/null 2>&1 \
      || refuse "$target is not a single JSON object; fix it by hand first"
    merged=$($JQ -s '.[0] * .[1]' "$target" "$HERE/daemon.json") || fail "jq could not merge daemon.json"
    save_previous /etc/docker/daemon.json daemon.json.previous
  else
    merged=$(cat "$HERE/daemon.json")
  fi
  write_file /etc/docker/daemon.json 0644 <<<"$merged"
  if [ "$RESTART_DOCKER" -eq 1 ]; then
    run "$SYSTEMCTL" restart docker
  else
    say "Restart Docker to apply: systemctl restart docker."
  fi
  say "Then recreate Headscale, which keeps its old label until then:"
  say "  docker compose -f $HEADSCALE_DIR/compose.yaml up -d --force-recreate"
}

cmd_check() {
  local problems line
  require_root
  inventory
  os_warnings
  problems=$(admin_problem; match_problems; sshd_mismatches "${ANIMA_ADMIN_USER:-}"; update_problems
    selinux_problems; firewall_problems; listener_violations)
  if [ -z "$problems" ]; then say "ok: no violations"; return 0; fi
  while IFS= read -r line; do say "VIOLATION: $line"; done <<<"$problems"
  return 1
}

# shellcheck source=sshd.sh
. "$HERE/sshd.sh"
# shellcheck source=selinux.sh
. "$HERE/selinux.sh"
# shellcheck source=open-ports.sh
. "$HERE/open-ports.sh"
# shellcheck source=firewall.sh
. "$HERE/firewall.sh"
# shellcheck source=inventory.sh
. "$HERE/inventory.sh"

main() {
  while [ $# -gt 0 ]; do
    case $1 in
      --root) [ $# -ge 2 ] || usage_error; ROOT=$2; shift 2 ;;
      --dry-run) DRY_RUN=1; shift ;;
      --console) CONSOLE=1; shift ;;
      --restart-docker) RESTART_DOCKER=1; shift ;;
      --allow-closing) [ $# -ge 2 ] || usage_error; ALLOW_CLOSING=$2; shift 2 ;;
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
    check | ssh | updates | selinux | docker | firewall | confirm) "cmd_$1" ;;
    *) usage_error ;;
  esac
}

main "$@"
