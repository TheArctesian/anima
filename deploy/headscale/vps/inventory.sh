# shellcheck shell=bash
# Read-only inventory of the VPS and the `check` command (VPS-002, VPS-005). Sourced by
# harden.sh, which defines the commands, paths and helpers used here. Nothing in this file
# changes the system: the inventory is what the operator sends to the programme before the
# first change, because the VPS may already host other services.

ALLOWED_PUBLIC=" 22/tcp 443/tcp 3478/udp 41641/udp "
# Programs expected to listen on the VPS (ss shows at most 15 characters of a name).
EXPECTED_PROGRAMS=" sshd headscale tailscaled dockerd docker-proxy containerd chronyd systemd-resolve "
LOCAL_ALLOW=/etc/anima/firewall-allow
NFT_RULES=/etc/nftables/anima.nft
NFT_SYSCONFIG=/etc/sysconfig/nftables.conf
NFT_INCLUDE="include \"$NFT_RULES\""
NFT_DROPIN=/etc/systemd/system/nftables.service.d/anima.conf

is_vpn_address() {
  local address=${1#::ffff:} a b
  case $address in
    *:*) case $(tr 'A-F' 'a-f' <<<"$address") in fd7a:115c:a1e0:*) return 0 ;; esac; return 1 ;;
  esac
  [[ $address =~ ^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$ ]] || return 1
  IFS=. read -r a b _ <<<"$address"
  [ "$a" -eq 100 ] && [ "$b" -ge 64 ] && [ "$b" -le 127 ]
}

# Print the operator's extra public ports from LOCAL_ALLOW, one PORT/PROTO per line. The
# file may only open single tcp or udp ports to the internet; any other line is an error.
local_accepts() {
  local file line
  file=$(path "$LOCAL_ALLOW")
  [ -f "$file" ] || return 0
  while IFS= read -r line || [ -n "$line" ]; do
    line=${line%%#*}
    line=${line//[[:space:]]/}
    [ -n "$line" ] || continue
    if ! [[ $line =~ ^([1-9][0-9]{0,4})/(tcp|udp)$ ]] || [ "${BASH_REMATCH[1]}" -gt 65535 ]; then
      fail "$LOCAL_ALLOW: not one PORT/PROTO (tcp or udp): $line"
    fi
    [ "$line" != 22/tcp ] || fail "$LOCAL_ALLOW: 22/tcp stays on the VPN only (VPS-001)"
    say "$line"
  done <"$file"
}

# True when the Anima table leaves PORT/PROTO open to the internet.
kept_open() {
  case "$ALLOWED_PUBLIC$(local_accepts | tr '\n' ' ') " in *" $1 "*) return 0 ;; esac
  return 1
}

# Print "PORT/PROTO NETID ADDRESS PROCESS" for each socket reachable from outside.
public_sockets() {
  local sockets netid local_address rest port host scope
  sockets=$($SS -Htulpn) || return 1
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
    say "$port/$netid $netid $local_address $rest"
  done <<<"$sockets"
}

# Print "PORT/PROTO firewalld zone ZONE ..." for each opening firewalld has now.
firewalld_openings() {
  local zones zone service port
  $SYSTEMCTL is-active --quiet firewalld.service || return 0
  zones=$($FIREWALL_CMD --get-active-zones) || { say "unknown firewall-cmd --get-active-zones failed"; return; }
  while read -r zone _; do
    # Docker's own zone holds docker0 (target ACCEPT); it is not an opening to the internet.
    case $zone in '' | docker) continue ;; esac
    for port in $($FIREWALL_CMD --zone="$zone" --list-ports); do say "$port firewalld zone $zone"; done
    for service in $($FIREWALL_CMD --zone="$zone" --list-services); do
      for port in $($FIREWALL_CMD --info-service="$service" | awk '$1 == "ports:" { $1 = ""; print }'); do
        say "$port firewalld zone $zone service $service"
      done
    done
    [ -z "$($FIREWALL_CMD --zone="$zone" --list-rich-rules)" ] || say "rich-rules firewalld zone $zone"
  done < <(grep -v '^[[:space:]]' <<<"$zones")
}

# Print each public listener the Anima table does not leave open (VPS-002).
listener_violations() {
  local sockets port socket
  sockets=$(public_sockets) || { say "ss -Htulpn failed"; return; }
  while read -r port socket; do
    [ -n "$port" ] || continue
    kept_open "$port" || say "public listener $socket"
  done <<<"$sockets"
}

# Print each problem with the live and boot firewall state.
firewall_problems() {
  $IP link show tailscale0 >/dev/null 2>&1 || say "tailscale0 is missing"
  $NFT list table inet anima_filter >/dev/null 2>&1 || say "table inet anima_filter is not loaded"
  $SYSTEMCTL is-enabled --quiet nftables.service || say "nftables.service is not enabled"
  grep -qxF "$NFT_INCLUDE" "$(path "$NFT_SYSCONFIG")" 2>/dev/null \
    || say "$NFT_SYSCONFIG does not load $NFT_RULES at boot"
  ! grep -qE '^[[:space:]]*flush[[:space:]]+ruleset' "$(path "$NFT_SYSCONFIG")" 2>/dev/null \
    || say "$NFT_SYSCONFIG flushes the whole ruleset (Docker's and tailscaled's tables too)"
  [ -f "$(path "$NFT_DROPIN")" ] || say "nftables.service would flush the whole ruleset on stop or reload"
  [ "$($SYSTEMCTL is-enabled firewalld.service 2>/dev/null)" = masked ] || say "firewalld is not masked"
  (local_accepts >/dev/null) 2>/dev/null || say "$LOCAL_ALLOW has a line that is not one PORT/PROTO"
}

# Print each problem with the SELinux labels Headscale's volumes need, or nothing.
label_problems() {
  local mode dir label
  mode=$($GETENFORCE 2>/dev/null) || mode=unknown
  [ "$mode" != Disabled ] || return 0
  for dir in config lib; do
    label=$($STAT -c %C "$(path "$HEADSCALE_DIR/$dir")" 2>/dev/null) || { say "$HEADSCALE_DIR/$dir is missing"; continue; }
    case $label in
      *:container_file_t:*) ;;
      *) say "$HEADSCALE_DIR/$dir is labelled $label, not container_file_t" ;;
    esac
  done
}

selinux_problems() {
  local mode
  mode=$($GETENFORCE 2>/dev/null) || mode=unknown
  [ "$mode" = Enforcing ] || say "SELinux is $mode, not Enforcing"
  label_problems
  case $($DOCKER info --format '{{.SecurityOptions}}' 2>/dev/null) in
    *name=selinux*) ;;
    *) say "Docker runs containers without SELinux labels (daemon.json selinux-enabled)" ;;
  esac
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

# Print each listening socket with its program and unit; mark the unexpected ones.
listener_inventory() {
  local sockets netid local_address rest program pid unit mark
  sockets=$($SS -Htulpn) || { say "(ss -Htulpn failed)"; return; }
  while read -r netid _ _ _ local_address _ rest; do
    [ -n "${local_address:-}" ] || continue
    program=$(sed -n 's/.*users:(("\([^"]*\)",pid=\([0-9]*\).*/\1/p' <<<"$rest")
    pid=$(sed -n 's/.*users:(("[^"]*",pid=\([0-9]*\).*/\1/p' <<<"$rest")
    unit=$([ -z "$pid" ] || $PS -o unit= -p "$pid" 2>/dev/null) || unit=
    case $EXPECTED_PROGRAMS in *" ${program:-?} "*) mark="  " ;; *) mark="! " ;; esac
    say "$mark$netid $local_address ${program:-?} ${unit:-?}"
  done <<<"$sockets"
}

# Print each Docker network with its bridge; mark bridges the forward chain does not allow.
network_inventory() {
  local networks name driver id bridge mark
  networks=$($DOCKER network ls --no-trunc --format '{{.Name}} {{.Driver}} {{.ID}}' 2>/dev/null) \
    || { say "(docker network ls failed)"; return; }
  while read -r name driver id; do
    [ -n "$name" ] || continue
    bridge=
    if [ "$driver" = bridge ]; then
      bridge=$($DOCKER network inspect -f '{{index .Options "com.docker.network.bridge.name"}}' "$name")
      case $bridge in '' | '<no value>') bridge=br-${id:0:12} ;; esac
    fi
    case $bridge in '' | docker0 | br-*) mark="  " ;; *) mark="! " ;; esac
    say "$mark$name $driver ${bridge:--}"
  done <<<"$networks"
}

# Print the enabled services that a stock AlmaLinux 10 VPS with Anima does not have.
unit_inventory() {
  local unit
  while read -r unit _; do
    grep -qxF "$unit" "$HERE/stock-units.txt" || say "! $unit"
  done < <($SYSTEMCTL list-unit-files --type=service --state=enabled --no-legend)
}

firewalld_inventory() {
  local zone
  if ! $SYSTEMCTL is-active --quiet firewalld.service; then
    say "firewalld: $($SYSTEMCTL is-enabled firewalld.service 2>/dev/null || true), not running"
    return
  fi
  while read -r zone _; do
    [ -z "$zone" ] || $FIREWALL_CMD --zone="$zone" --list-all
  done < <($FIREWALL_CMD --get-active-zones | grep -v '^[[:space:]]')
}

inventory() {
  say "== Listening sockets (! = not sshd, Headscale, tailscaled, Docker, chronyd or resolved)"
  listener_inventory
  say "== Running containers"
  $DOCKER ps --format '{{.Names}} {{.Image}} {{.Ports}}' 2>/dev/null || say "(docker ps failed)"
  say "== Docker networks (! = bridge name not docker0 or br-*: no route into the VPN)"
  network_inventory
  say "== Enabled services not in the stock list (stock-units.txt)"
  unit_inventory
  say "== firewalld"
  firewalld_inventory
  say "== nftables tables"
  $NFT list tables 2>/dev/null || say "(nft list tables failed)"
  say "== End of inventory"
}

cmd_check() {
  local problems line
  require_root
  inventory
  os_warnings
  problems=$(admin_problem; sshd_mismatches "${ANIMA_ADMIN_USER:-}"; update_problems
    selinux_problems; firewall_problems; listener_violations)
  if [ -z "$problems" ]; then say "ok: no violations"; return 0; fi
  while IFS= read -r line; do say "VIOLATION: $line"; done <<<"$problems"
  return 1
}
