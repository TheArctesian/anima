# shellcheck shell=bash
# `harden.sh broker` (R-055, VP-2, VP-8): install the Anima broker in one command. The operator
# first copies the Linux binary to /usr/local/libexec/anima-broker.new with scp (README.md). The
# units come from ../broker/ beside this folder. Sourced by harden.sh; needs its say, refuse,
# fail, dry, run, path, write_file and require_root, and SYSTEMCTL, UNAME and HERE.

BROKER_NEW=/usr/local/libexec/anima-broker.new
BROKER_BIN=/usr/local/libexec/anima-broker
BROKER_CONF=/etc/anima/broker.conf
BROKER_UNIT_DIR=/etc/systemd/system
# max_nodes leaves room for this many new nodes above the count at install time.
BROKER_SPARE_NODES=5

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

cmd_broker() {
  local new units arch count=
  require_root
  new=$(path "$BROKER_NEW")
  units=$HERE/../broker
  [ -f "$units/anima-broker.socket" ] && [ -f "$units/anima-broker@.unit" ] \
    || refuse "the broker units are not in $units; copy deploy/headscale/broker/ beside vps/"
  if [ ! -f "$new" ]; then
    dry || refuse "$BROKER_NEW is missing; copy the binary there with scp first (README.md)"
    say "[dry-run] $BROKER_NEW is missing; a real run refuses"
  fi
  arch=$($UNAME -m)
  case $arch in
    x86_64 | aarch64) ;;
    *) refuse "this machine is $arch; the broker is built for x86_64 (or aarch64 with the arm64 build)" ;;
  esac
  if [ -f "$new" ]; then count=$(broker_node_count "$new") || exit $?; fi
  broker_config "$count"
  run install -m 0755 "$new" "$(path "$BROKER_BIN")"
  run rm -f "$new"
  write_file "$BROKER_UNIT_DIR/anima-broker.socket" 0644 <"$units/anima-broker.socket"
  write_file "$BROKER_UNIT_DIR/anima-broker@.service" 0644 <"$units/anima-broker@.unit"
  run "$SYSTEMCTL" daemon-reload
  run "$SYSTEMCTL" enable --now anima-broker.socket
  if dry; then say "[dry-run] $(path "$BROKER_BIN") check"; return; fi
  "$(path "$BROKER_BIN")" check || fail "the broker does not answer; see journalctl -u 'anima-broker@*'"
  say "Broker installed. The hub reaches it at /run/anima-broker/broker.sock."
}
