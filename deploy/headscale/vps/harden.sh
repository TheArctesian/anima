#!/usr/bin/env bash
# Harden the Anima VPS (Ubuntu 24.04): SSH keys only, automatic security updates, Docker
# loopback publishing and the nftables firewall with an automatic rollback.
# Spec VPS-001, VPS-002, VPS-004, VPS-005, DNS-005; decisions D-10 and D-11.
#
# The operator runs each command by hand on the VPS (VPS-005). This script never prints
# key material: it only tests that authorized_keys holds a key.
set -euo pipefail

# System commands. Override from the environment to test against fakes (D-14).
NFT=${HARDEN_NFT:-nft}
SYSTEMCTL=${HARDEN_SYSTEMCTL:-systemctl}
SYSTEMD_RUN=${HARDEN_SYSTEMD_RUN:-systemd-run}
SSHD=${HARDEN_SSHD:-sshd}
SS=${HARDEN_SS:-ss}
APT_GET=${HARDEN_APT_GET:-apt-get}
UNATTENDED_UPGRADE=${HARDEN_UNATTENDED_UPGRADE:-unattended-upgrade}
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
STATE_DIR=/var/lib/anima
ROLLBACK_UNIT=anima-firewall-rollback
ROLLBACK_SECONDS=180
ROLLED_BACK=$STATE_DIR/firewall-rolled-back
SESSION=$STATE_DIR/firewall-session
NFT_DROPIN=/etc/systemd/system/nftables.service.d/anima.conf
USER_NAME_PATTERN='^[a-z_][a-z0-9_-]{0,31}$'
KEY_LINE_PATTERN='(^|[[:space:]])(ssh-|ecdsa-|sk-)[^[:space:]]+[[:space:]]+AAAA'
ALLOWED_PUBLIC=" 22/tcp 443/tcp 3478/udp 41641/udp "

usage() {
  cat <<EOF
Usage: $SELF [--root DIR] [--dry-run] [--console] [--restart-docker] <command>

Commands (run as root):
  check     Report OS, Docker version, VPN interface, admin user, effective sshd
            values, firewall state and public listeners. Changes nothing.
  ssh       Install the sshd drop-in (keys only, no root, AllowUsers \$ANIMA_ADMIN_USER),
            verify it with sshd -t and sshd -T, then reload ssh. ANIMA_ADMIN_USER must
            be a non-root member of group sudo with a key in ~/.ssh/authorized_keys:
            sudo --preserve-env=ANIMA_ADMIN_USER $SELF ssh
  updates   Install unattended-upgrades and the daily security update settings.
  docker    Merge daemon.json into /etc/docker/daemon.json (default bind 127.0.0.1).
            Docker is restarted only with --restart-docker. Needs jq.
  firewall  Validate and apply nftables.conf, armed with a 180 s rollback. Needs
            tailscale0 and an SSH session over the VPN, or --console when you use the
            Hostinger browser console:
            sudo --preserve-env=SSH_CONNECTION $SELF firewall
  confirm   From a NEW SSH session over the VPN (same sudo form as firewall): cancel
            the rollback and install the rules for boot.

Options:
  --root DIR        Treat DIR as the file system root (tests). Default /.
  --dry-run         Print the actions; write and change nothing.
  --console         firewall, confirm: you are on the provider console, not on SSH.
  --restart-docker  docker: restart Docker after the change.

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

# Write standard input to a system path atomically (temporary file, then rename).
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

# Print why a path would make sshd (StrictModes) ignore the keys, or nothing.
strict_modes_problem() {
  if [ -n "$(find "$1" -prune \( -perm -020 -o -perm -002 \) -print)" ]; then
    say "$1 is group- or world-writable; sshd would ignore the keys"
  elif [ -n "$(find "$1" -prune ! -user "$2" ! -user 0 -print)" ]; then
    say "$1 is not owned by the admin user or root; sshd would ignore the keys"
  fi
}

# Print why ANIMA_ADMIN_USER cannot be the only SSH login, or nothing when it can.
admin_problem() {
  local user=${ANIMA_ADMIN_USER:-} entry uid home keys problem
  [ -n "$user" ] || { say "set ANIMA_ADMIN_USER to the admin account"; return; }
  [[ $user =~ $USER_NAME_PATTERN ]] || { say "ANIMA_ADMIN_USER is not a valid user name"; return; }
  entry=$($GETENT passwd "$user") || { say "user $user does not exist"; return; }
  IFS=: read -r _ _ uid _ _ home _ <<<"$entry"
  if [ "$user" = root ] || [ "$uid" = 0 ]; then say "$user is root; use a non-root sudo user"; return; fi
  case " $($ID -nG "$user") " in
    *" sudo "*) ;;
    *) say "$user is not in group sudo"; return ;;
  esac
  [[ $home == /* ]] || { say "$user has no absolute home directory"; return; }
  keys=$(path "$home/.ssh/authorized_keys")
  [ -f "$keys" ] || { say "$user has no ~/.ssh/authorized_keys"; return; }
  if ! awk -v pattern="$KEY_LINE_PATTERN" '/^[[:space:]]*(#|$)/ { next } $0 ~ pattern { found = 1 }
      END { exit !found }' "$keys"; then
    say "$user has no key in ~/.ssh/authorized_keys"; return
  fi
  for entry in "$(path "$home")" "$(path "$home/.ssh")" "$keys"; do
    problem=$(strict_modes_problem "$entry" "$uid")
    [ -z "$problem" ] || { say "$problem"; return; }
  done
}

# Print each effective sshd value (sshd -T) that differs from the drop-in.
sshd_mismatches() {
  local effective line
  effective=$($SSHD -T) || { say "sshd -T failed"; return; }
  while IFS= read -r line; do
    grep -qixF "$line" <<<"$effective" || say "sshd: expected \"$line\""
  done <<EOF
passwordauthentication no
kbdinteractiveauthentication no
permitrootlogin no
permitemptypasswords no
pubkeyauthentication yes
authenticationmethods publickey
x11forwarding no
maxauthtries 3
logingracetime 30
allowusers $1
EOF
  # AllowUsers adds up across drop-ins: any other user is a second way in.
  awk -v want="allowusers $1" 'tolower($1) == "allowusers" && tolower($0) != want {
    print "sshd: unexpected \"" $0 "\" (AllowUsers adds up across drop-ins)" }' <<<"$effective"
}

is_vpn_address() {
  local address=${1#::ffff:} a b
  case $address in
    *:*) case $(tr 'A-F' 'a-f' <<<"$address") in fd7a:115c:a1e0:*) return 0 ;; esac; return 1 ;;
  esac
  [[ $address =~ ^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$ ]] || return 1
  IFS=. read -r a b _ <<<"$address"
  [ "$a" -eq 100 ] && [ "$b" -ge 64 ] && [ "$b" -le 127 ]
}

# True when this SSH session runs over the VPN at both ends.
vpn_session() {
  local client server
  read -r client _ server _ <<<"${SSH_CONNECTION:-}"
  is_vpn_address "${client:-}" && is_vpn_address "${server:-}"
}

# Print each listening socket that is reachable from outside and not allowed (VPS-002).
listener_violations() {
  local sockets netid local_address rest port host scope
  sockets=$($SS -Htulpn) || { say "ss -Htulpn failed"; return; }
  while read -r netid _ _ _ local_address _ rest; do
    [ -n "${local_address:-}" ] || continue
    port=${local_address##*:}
    host=${local_address%:*}
    host=${host#[}
    host=${host%]}
    scope=
    case $host in *%*) scope=${host#*%}; host=${host%%\%*} ;; esac
    case $scope in lo | tailscale0) continue ;; esac
    case $host in 127.* | ::1) continue ;; esac
    if is_vpn_address "$host"; then continue; fi
    case $ALLOWED_PUBLIC in *" $port/$netid "*) continue ;; esac
    say "public listener $netid $local_address $rest"
  done <<<"$sockets"
}

# Print each problem with the live and boot firewall state.
firewall_problems() {
  $IP link show tailscale0 >/dev/null 2>&1 || say "tailscale0 is missing"
  $NFT list table inet anima_filter >/dev/null 2>&1 || say "table inet anima_filter is not loaded"
  $SYSTEMCTL is-enabled --quiet nftables.service || say "nftables.service is not enabled"
  [ -f "$(path "$NFT_DROPIN")" ] || say "nftables.service would flush the whole ruleset on stop"
}

os_warnings() {
  local release version
  release=$(path /etc/os-release)
  if ! grep -qx 'ID=ubuntu' "$release" 2>/dev/null || ! grep -qx 'VERSION_ID="24.04"' "$release"; then
    say "warning: not Ubuntu 24.04; this script is written for it"
  fi
  if ! version=$($DOCKER version --format '{{.Server.Version}}' 2>/dev/null); then
    say "warning: cannot read the Docker version"
  elif [[ ${version%%.*} =~ ^[0-9]+$ ]] && [ "${version%%.*}" -lt 28 ]; then
    say "warning: Docker $version is older than 28; ports published on loopback are reachable"
  fi
}

cmd_check() {
  local problems line
  require_root
  os_warnings
  problems=$(admin_problem; sshd_mismatches "${ANIMA_ADMIN_USER:-}"; firewall_problems; listener_violations)
  if [ -z "$problems" ]; then say "ok: no violations"; return 0; fi
  while IFS= read -r line; do say "VIOLATION: $line"; done <<<"$problems"
  return 1
}

restore_dropin() {
  if [ -f "$(path "$2")" ]; then mv -f "$(path "$2")" "$(path "$1")"; else rm -f "$(path "$1")"; fi
}

cmd_ssh() {
  local problem user=${ANIMA_ADMIN_USER:-} dropin=/etc/ssh/sshd_config.d/10-anima.conf
  local previous=$STATE_DIR/10-anima.conf.previous mismatches
  require_root
  problem=$(admin_problem)
  [ -z "$problem" ] || refuse "$problem"
  if dry; then
    say "[dry-run] write $(path "$dropin") with AllowUsers $user, then $SSHD -t, $SSHD -T, reload ssh"
    return
  fi
  rm -f "$(path "$previous")"
  save_previous "$dropin" 10-anima.conf.previous
  sed "s/\${ANIMA_ADMIN_USER}/$user/" "$HERE/sshd-anima.conf" | write_file "$dropin" 0644
  mkdir -p "$(path /run/sshd)"
  if ! $SSHD -t; then restore_dropin "$dropin" "$previous"; fail "sshd -t rejected the drop-in; restored"; fi
  mismatches=$(sshd_mismatches "$user")
  if [ -n "$mismatches" ]; then
    say "$mismatches" >&2
    restore_dropin "$dropin" "$previous"
    fail "effective sshd values are wrong; drop-in restored"
  fi
  if ! $SYSTEMCTL is-active --quiet ssh; then
    say "ssh.service is not running; the drop-in applies when it starts"
  elif ! $SYSTEMCTL reload ssh; then
    restore_dropin "$dropin" "$previous"
    fail "systemctl reload ssh failed; drop-in restored"
  fi
  say "SSH hardened. Keep this session open and test a new login as $user first."
}

cmd_updates() {
  require_root
  run env DEBIAN_FRONTEND=noninteractive "$APT_GET" install -y unattended-upgrades
  write_file /etc/apt/apt.conf.d/20auto-upgrades 0644 <"$HERE/20auto-upgrades"
  write_file /etc/apt/apt.conf.d/52anima-unattended-upgrades 0644 <"$HERE/52anima-unattended-upgrades"
  run "$UNATTENDED_UPGRADE" --dry-run
  say "Automatic security updates enabled (daily, reboot at 04:30 when needed)."
}

cmd_docker() {
  local target merged
  require_root
  command -v "$JQ" >/dev/null 2>&1 || fail "jq is required: apt-get install jq"
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
    say "Restart Docker to apply: systemctl restart docker (containers restart once)."
  fi
}

# Write the rollback script: remove our table, then restore its previous version if any.
write_rollback() {
  printf 'table inet anima_filter\ndelete table inet anima_filter\n' >"$2"
  awk '/^table inet anima_filter \{$/ { keep = 1 } keep { print } keep && /^\}$/ { keep = 0 }' \
    "$1" >>"$2"
}

# Arm the rollback before applying, so a session cut by the new rules still gets it.
# The marker comes first, so confirm can tell that the rollback has started.
arm_rollback() {
  local nft_path
  nft_path=$(command -v "$NFT") || fail "cannot resolve the path of $NFT"
  case $nft_path in *"'"*) fail "the nft path contains a quote" ;; esac
  # A rollback left from an earlier run would restore older rules; it may not exist,
  # and if its name is still taken systemd-run below fails before anything is applied.
  $SYSTEMCTL stop "$ROLLBACK_UNIT.timer" >/dev/null 2>&1 || true
  $SYSTEMCTL reset-failed "$ROLLBACK_UNIT.service" >/dev/null 2>&1 || true
  $SYSTEMD_RUN --unit="$ROLLBACK_UNIT" --on-active="$ROLLBACK_SECONDS" --timer-property=AccuracySec=1s \
    /bin/sh -c "touch '$ROLLED_BACK' && '$nft_path' -f '$STATE_DIR/nftables.rollback'" \
    || fail "could not arm the rollback; nothing applied"
}

cmd_firewall() {
  local state
  require_root
  $IP link show tailscale0 >/dev/null 2>&1 || refuse "tailscale0 is missing; join the VPN first"
  if [ "$CONSOLE" -eq 0 ] && ! vpn_session; then
    refuse "this session does not come over the VPN; connect to the VPN address, or use --console"
  fi
  $NFT -c -f "$HERE/nftables.conf" || fail "nft -c rejected nftables.conf; nothing applied"
  if dry; then
    say "[dry-run] save ruleset to $STATE_DIR/nftables.previous, arm $ROLLBACK_UNIT ($ROLLBACK_SECONDS s), apply"
    return
  fi
  state=$(path "$STATE_DIR")
  mkdir -p "$state"
  chmod 0700 "$state"
  rm -f "$(path "$ROLLED_BACK")"
  $NFT list ruleset >"$state/nftables.previous" || fail "could not save the current ruleset"
  write_rollback "$state/nftables.previous" "$state/nftables.rollback"
  $NFT -c -f "$state/nftables.rollback" || fail "the rollback does not validate; nothing applied"
  printf '%s\n' "${SSH_CONNECTION:-console}" >"$(path "$SESSION")"
  cp "$HERE/nftables.conf" "$state/nftables.pending"
  arm_rollback
  if ! $NFT -f "$state/nftables.pending"; then
    $SYSTEMCTL stop "$ROLLBACK_UNIT.timer"
    fail "nft -f failed; the ruleset is unchanged"
  fi
  say "Firewall applied. It rolls back in $ROLLBACK_SECONDS s unless you confirm."
  say "Keep this session open. From a NEW SSH session over the VPN, run:"
  say "  sudo --preserve-env=SSH_CONNECTION $SELF confirm"
  say "If the new session cannot connect, do nothing: the previous rules come back."
}

# Refuse unless a new SSH session over the VPN (not the one that ran firewall) confirms.
require_new_vpn_session() {
  if [ "$CONSOLE" -eq 1 ]; then
    say "WARNING: --console skips the check that SSH over the VPN works with the new rules."
    say "WARNING: before you close the console, test a NEW SSH login over the VPN."
    return 0
  fi
  vpn_session || refuse "confirm from a NEW SSH session over the VPN, or use --console"
  [ -f "$(path "$SESSION")" ] || refuse "no recorded firewall session; run firewall again"
  [ "$(cat "$(path "$SESSION")")" != "${SSH_CONNECTION:-}" ] \
    || refuse "this is the session that ran firewall; confirm from a NEW SSH session over the VPN"
}

cmd_confirm() {
  local pending=$STATE_DIR/nftables.pending
  require_root
  [ -f "$(path "$pending")" ] || refuse "nothing to confirm; run firewall first"
  [ ! -e "$(path "$ROLLED_BACK")" ] || refuse "the rollback already ran; run firewall again"
  [ "$($SYSTEMCTL show --property=SubState --value "$ROLLBACK_UNIT.timer")" = waiting ] \
    || refuse "the rollback is not armed (it may have run); run firewall again"
  require_new_vpn_session
  if dry; then say "[dry-run] stop $ROLLBACK_UNIT.timer, install /etc/nftables.conf, enable nftables"; return; fi
  $SYSTEMCTL stop "$ROLLBACK_UNIT.timer" || fail "could not stop the rollback"
  [ ! -e "$(path "$ROLLED_BACK")" ] || fail "the rollback started while confirming; run firewall again"
  save_previous /etc/nftables.conf nftables.conf.previous once
  write_file /etc/nftables.conf 0755 <"$(path "$pending")"
  write_file "$NFT_DROPIN" 0644 <"$HERE/nftables-unit.conf"
  $SYSTEMCTL daemon-reload
  $SYSTEMCTL enable nftables.service
  rm -f "$(path "$pending")"
  say "Firewall confirmed and installed for boot."
}

main() {
  while [ $# -gt 0 ]; do
    case $1 in
      --root) [ $# -ge 2 ] || usage_error; ROOT=$2; shift 2 ;;
      --dry-run) DRY_RUN=1; shift ;;
      --console) CONSOLE=1; shift ;;
      --restart-docker) RESTART_DOCKER=1; shift ;;
      -h | --help) usage; exit 0 ;;
      -*) usage_error ;;
      *) break ;;
    esac
  done
  [ $# -eq 1 ] || usage_error
  [[ $ROOT == /* ]] && [ -d "$ROOT" ] || { printf 'harden.sh: --root must be an existing absolute directory\n' >&2; exit 64; }
  case $1 in
    check | ssh | updates | docker | firewall | confirm) "cmd_$1" ;;
    *) usage_error ;;
  esac
}

main "$@"
