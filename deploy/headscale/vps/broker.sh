# shellcheck shell=bash
# `harden.sh broker` (R-055, VP-2, VP-8): install the Anima broker in one command. The operator
# first copies the Linux binary to /usr/local/libexec/anima-broker.new with scp (README.md) and
# passes the SHA-256 that broker-build.ts printed (--sha256=). The units come from ../broker/
# beside this folder. Sourced by harden.sh; needs its say, refuse, fail, dry, run, path,
# write_file and require_root, and SYSTEMCTL, UNAME, HERE and BROKER_SHA256.

BROKER_NEW=/usr/local/libexec/anima-broker.new
BROKER_BIN=/usr/local/libexec/anima-broker
BROKER_CONF=/etc/anima/broker.conf
BROKER_UNIT_DIR=/etc/systemd/system
BROKER_SOCKET=anima-broker.socket
BROKER_TIMER=anima-broker-audit.timer
# Repository file → installed unit name (a file may not be called *service, quality:names).
BROKER_UNITS=(
  "anima-broker.socket:anima-broker.socket"
  "anima-broker@.unit:anima-broker@.service"
  "anima-broker-audit.unit:anima-broker-audit.service"
  "anima-broker-audit.timer:anima-broker-audit.timer"
)
# max_nodes leaves room for this many new nodes above the count at install time.
BROKER_SPARE_NODES=5

# Refuse a binary whose SHA-256 is not the one broker-build.ts printed. It runs as root next.
broker_verify() {
  local new=$1 sum
  if [ -z "$BROKER_SHA256" ]; then
    dry || refuse "pass --sha256=<the SHA-256 broker-build.ts printed>"
    say "[dry-run] no --sha256; a real run refuses"
    return
  fi
  [[ $BROKER_SHA256 =~ ^[0-9a-f]{64}$ ]] || refuse "--sha256 must be 64 lower-case hex digits"
  [ -f "$new" ] || return 0
  sum=$(sha256sum "$new") || fail "sha256sum $BROKER_NEW failed"
  [ "${sum%% *}" = "$BROKER_SHA256" ] || refuse "$BROKER_NEW does not match --sha256; copy it again"
  say "$BROKER_NEW matches --sha256."
}

# Print the node count from the new binary; it reads `headscale nodes list` and changes nothing.
broker_node_count() {
  local new=$1 count
  count=$("$new" count-nodes) || refuse "$BROKER_NEW count-nodes failed: a binary for another architecture, or Headscale is not running"
  [[ $count =~ ^[0-9]+$ ]] || refuse "$BROKER_NEW count-nodes printed no number"
  printf '%s' "$count"
}

# Write broker.conf only if it is absent: the operator's max_nodes is kept.
broker_config() {
  local count=$1
  if [ -e "$(path "$BROKER_CONF")" ]; then
    say "Keeping $BROKER_CONF."
  elif [ -n "$count" ]; then
    write_file "$BROKER_CONF" 0644 <<<"max_nodes=$((count + BROKER_SPARE_NODES))"
  else
    say "[dry-run] write $(path "$BROKER_CONF") with max_nodes = the node count + $BROKER_SPARE_NODES"
  fi
}

# Stop what starts the old binary (an upgrade); a running request keeps its own copy.
broker_stop() {
  local unit
  for unit in "$BROKER_SOCKET" "$BROKER_TIMER"; do
    [ ! -e "$(path "$BROKER_UNIT_DIR/$unit")" ] || run "$SYSTEMCTL" stop "$unit"
  done
}

# Copy the binary to a temporary file in the same folder, then rename it over the old one.
broker_install_binary() {
  local new=$1 bin tmp
  bin=$(path "$BROKER_BIN")
  if dry; then say "[dry-run] install -m 0755 $new to a temporary file beside $bin, then mv -f it to $bin"; return; fi
  tmp=$(mktemp "$bin.XXXXXX")
  install -m 0755 "$new" "$tmp" || { rm -f "$tmp"; fail "installing $BROKER_BIN failed"; }
  mv -f "$tmp" "$bin"
}

cmd_broker() {
  local new units arch entry count=
  require_root
  new=$(path "$BROKER_NEW")
  units=$HERE/../broker
  for entry in "${BROKER_UNITS[@]}"; do
    [ -f "$units/${entry%%:*}" ] || refuse "the broker units are not in $units; copy deploy/headscale/broker/ beside vps/"
  done
  if [ ! -f "$new" ]; then
    dry || refuse "$BROKER_NEW is missing; copy the binary there with scp first (README.md)"
    say "[dry-run] $BROKER_NEW is missing; a real run refuses"
  fi
  arch=$($UNAME -m)
  case $arch in
    x86_64 | aarch64) ;;
    *) refuse "this machine is $arch; the broker is built for x86_64 (or aarch64 with the arm64 build)" ;;
  esac
  broker_verify "$new"
  if dry; then
    say "[dry-run] $BROKER_NEW count-nodes"
  else
    count=$(broker_node_count "$new") || exit $?
  fi
  broker_config "$count"
  broker_stop
  broker_install_binary "$new"
  run rm -f "$new"
  for entry in "${BROKER_UNITS[@]}"; do
    write_file "$BROKER_UNIT_DIR/${entry#*:}" 0644 <"$units/${entry%%:*}"
  done
  run "$SYSTEMCTL" daemon-reload
  run "$SYSTEMCTL" enable "$BROKER_SOCKET" "$BROKER_TIMER"
  run "$SYSTEMCTL" restart "$BROKER_SOCKET" "$BROKER_TIMER"
  if dry; then say "[dry-run] $(path "$BROKER_BIN") check"; return; fi
  "$(path "$BROKER_BIN")" check || fail "the broker does not answer; see journalctl -u 'anima-broker@*'"
  say "Broker installed. The hub reaches it at /run/anima-broker/broker.sock; the audit runs every 5 minutes."
}
