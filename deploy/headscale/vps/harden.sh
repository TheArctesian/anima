#!/usr/bin/env bash
# Harden the Anima VPS (AlmaLinux 10): SSH keys only, automatic updates, SELinux labels for
# Headscale, Docker loopback publishing and the nftables firewall with an automatic rollback.
# Spec VPS-001, VPS-002, VPS-004, VPS-005, DNS-005; decisions D-9, D-10 and D-11.
#
# The operator runs each command by hand on the VPS (VPS-005). This script never prints
# key material: it only tests that authorized_keys holds a key. The read-only inventory
# and `check` are in inventory.sh, the firewall and its rollback in firewall.sh.
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
            verify it with sshd -t and sshd -T, then reload sshd. ANIMA_ADMIN_USER must
            be a non-root member of group $ADMIN_GROUP with a key in ~/.ssh/authorized_keys:
            sudo --preserve-env=ANIMA_ADMIN_USER $SELF ssh
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
  if [ "$user" = root ] || [ "$uid" = 0 ]; then say "$user is root; use a non-root $ADMIN_GROUP user"; return; fi
  case " $($ID -nG "$user") " in
    *" $ADMIN_GROUP "*) ;;
    *) say "$user is not in group $ADMIN_GROUP"; return ;;
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

restore_dropin() {
  if [ -f "$(path "$2")" ]; then mv -f "$(path "$2")" "$(path "$1")"; else rm -f "$(path "$1")"; fi
}

cmd_ssh() {
  local problem user=${ANIMA_ADMIN_USER:-} dropin=$SSHD_DROPIN
  local previous=$STATE_DIR/00-anima.conf.previous mismatches
  require_root
  problem=$(admin_problem)
  [ -z "$problem" ] || refuse "$problem"
  if dry; then
    say "[dry-run] write $(path "$dropin") with AllowUsers $user, then $SSHD -t, $SSHD -T, reload sshd"
    return
  fi
  rm -f "$(path "$previous")"
  save_previous "$dropin" 00-anima.conf.previous
  sed "s/\${ANIMA_ADMIN_USER}/$user/" "$HERE/sshd-anima.conf" | write_file "$dropin" 0600
  if ! $SSHD -t; then restore_dropin "$dropin" "$previous"; fail "sshd -t rejected the drop-in; restored"; fi
  mismatches=$(sshd_mismatches "$user")
  if [ -n "$mismatches" ]; then
    say "$mismatches" >&2
    restore_dropin "$dropin" "$previous"
    fail "effective sshd values are wrong; drop-in restored"
  fi
  if ! $SYSTEMCTL is-active --quiet sshd.service; then
    say "sshd.service is not running; the drop-in applies when it starts"
  elif ! $SYSTEMCTL reload sshd.service; then
    restore_dropin "$dropin" "$previous"
    fail "systemctl reload sshd failed; drop-in restored"
  fi
  say "SSH hardened. Keep this session open and test a new login as $user first."
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

cmd_selinux() {
  local mode dir
  require_root
  mode=$($GETENFORCE) || fail "getenforce failed"
  [ "$mode" != Disabled ] || refuse "SELinux is disabled; set SELINUX=enforcing in /etc/selinux/config and reboot"
  [ "$mode" = Enforcing ] || say "warning: SELinux is $mode; set it to Enforcing (setenforce 1 and /etc/selinux/config)"
  for dir in config lib; do
    [ -d "$(path "$HEADSCALE_DIR/$dir")" ] || refuse "$HEADSCALE_DIR/$dir is missing; create it first (guide step 2.5)"
  done
  if $SEMANAGE fcontext -l -C | awk -v spec="$LABEL_SPEC" '$1 == spec { found = 1 } END { exit !found }'; then
    run "$SEMANAGE" fcontext -m -t container_file_t "$LABEL_SPEC"
  else
    run "$SEMANAGE" fcontext -a -t container_file_t "$LABEL_SPEC"
  fi
  run "$RESTORECON" -R "$(path "$HEADSCALE_DIR")"
  say "Labelled $HEADSCALE_DIR/config and lib container_file_t."
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
    say "Restart Docker to apply: systemctl restart docker (containers restart once)."
  fi
}

# shellcheck source=inventory.sh
. "$HERE/inventory.sh"
# shellcheck source=firewall.sh
. "$HERE/firewall.sh"

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
  [[ $ROOT == /* ]] && [ -d "$ROOT" ] || { printf 'harden.sh: --root must be an existing absolute directory\n' >&2; exit 64; }
  case $1 in
    check | ssh | updates | selinux | docker | firewall | confirm) "cmd_$1" ;;
    *) usage_error ;;
  esac
}

main "$@"
