# shellcheck shell=bash
# The read-only inventory that `check` prints first (VPS-002, VPS-005): listeners and their
# units, containers, Docker networks, extra enabled units, firewalld and nftables tables.
# Nothing here changes the system: the operator sends this to the programme before the
# first change, because the VPS may already host other services. Sourced by harden.sh;
# needs its say, and SS, PS, DOCKER, SYSTEMCTL, FIREWALL_CMD, NFT and HERE.

# Programs expected to listen on the VPS (ss shows at most 15 characters of a name).
# Headscale uses the host network, so any docker-proxy is a port another container
# publishes, and dockerd or containerd should not listen on the network at all.
EXPECTED_PROGRAMS=" sshd headscale tailscaled chronyd systemd-resolve "

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
  say "policies:"
  $FIREWALL_CMD --get-active-policies
  say "direct rules:"
  $FIREWALL_CMD --direct --get-all-rules
}

inventory() {
  say "== Listening sockets (! = not sshd, Headscale, tailscaled, chronyd or resolved; docker-proxy is a port Docker publishes)"
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
