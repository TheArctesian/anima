# shellcheck shell=bash
# The `firewall` and `confirm` commands: apply nftables.conf with a 180 s rollback that only
# a new SSH session over the VPN can cancel, then install it for boot and mask firewalld
# (VPS-002, DNS-005, D-10). Sourced by harden.sh; uses the helpers of harden.sh and
# inventory.sh.

ROLLBACK_UNIT=anima-firewall-rollback
ROLLBACK_SECONDS=180
ROLLED_BACK=$STATE_DIR/firewall-rolled-back
SESSION=$STATE_DIR/firewall-session
FIREWALLD_WAS_ACTIVE=$STATE_DIR/firewalld-was-active
# Under SELinux a unit's /bin/sh runs as initrc_t and its nft as iptables_t, and the
# targeted policy lets iptables_t read no var_lib_t file, so the rollback could not read
# /var/lib/anima. The rollback unit therefore runs unconfined, as the backup unit does.
ROLLBACK_CONTEXT=system_u:system_r:unconfined_service_t:s0

# True when this SSH session runs over the VPN at both ends.
vpn_session() {
  local client server
  read -r client _ server _ <<<"${SSH_CONNECTION:-}"
  is_vpn_address "${client:-}" && is_vpn_address "${server:-}"
}

# Print each port that is open to the internet now and that the Anima table would close,
# unless --allow-closing names it. SSH (22/tcp) moves to the VPN on purpose (VPS-001).
closing_ports() {
  local sockets port what
  sockets=$(public_sockets) || { say "unknown (ss -Htulpn failed)"; return; }
  while read -r port what; do
    case $port in '' | 22/tcp) continue ;; esac
    if kept_open "$port"; then continue; fi
    case ",$ALLOW_CLOSING," in *",$port,"*) continue ;; esac
    say "$port ($what)"
  done < <(printf '%s\n' "$sockets"; firewalld_openings)
}

# Refuse while the firewall would silently close a port that is in use or allowed now.
refuse_closing() {
  local closing line
  closing=$(closing_ports)
  [ -n "$closing" ] || return 0
  while IFS= read -r line; do say "would close: $line"; done <<<"$closing" >&2
  refuse "the Anima firewall would close the ports above. Keep one open with a line PORT/PROTO in $LOCAL_ALLOW, or close it on purpose with --allow-closing PORT/PROTO,..."
}

# Print nftables.conf with the ports of LOCAL_ALLOW as public accepts after the marker line
# in the input chain. Nothing else changes; the VPN, prerouting and forward rules stay as
# reviewed.
render_rules() {
  local rules="" port
  for port in $(local_accepts); do
    rules+=$'\t\t'"${port#*/} dport ${port%/*} accept"$'\n'
  done
  RULES=$rules awk '$1 == "#" && $2 == "@anima-local-accepts:" { print; printf "%s", ENVIRON["RULES"]; next } { print }' \
    "$HERE/nftables.conf"
}

# Write the rollback script: remove our table, then restore its previous version if any.
write_rollback() {
  printf 'table inet anima_filter\ndelete table inet anima_filter\n' >"$2"
  awk '/^table inet anima_filter \{$/ { keep = 1 } keep { print } keep && /^\}$/ { keep = 0 }' \
    "$1" >>"$2"
}

# Prove, before anything changes, that a systemd unit with the rollback's context can read
# the rollback rules; otherwise the rollback would fail only when it is needed.
probe_rollback() {
  $SYSTEMD_RUN --wait --pipe --quiet --property=SELinuxContext="$ROLLBACK_CONTEXT" \
    /bin/sh -c "'$1' -c -f '$STATE_DIR/nftables.rollback'" \
    || fail "a systemd unit cannot read the rollback rules (SELinux?); nothing applied"
}

# Arm the rollback before applying, so a session cut by the new rules still gets it.
# The marker comes first, so confirm can tell that the rollback has started.
arm_rollback() {
  # A rollback left from an earlier run would restore older rules; it may not exist,
  # and if its name is still taken systemd-run below fails before anything is applied.
  $SYSTEMCTL stop "$ROLLBACK_UNIT.timer" >/dev/null 2>&1 || true
  $SYSTEMCTL reset-failed "$ROLLBACK_UNIT.service" >/dev/null 2>&1 || true
  $SYSTEMD_RUN --unit="$ROLLBACK_UNIT" --on-active="$ROLLBACK_SECONDS" --timer-property=AccuracySec=1s \
    --property=SELinuxContext="$ROLLBACK_CONTEXT" \
    /bin/sh -c "touch '$ROLLED_BACK' && '$1' -f '$STATE_DIR/nftables.rollback'" \
    || fail "could not arm the rollback; nothing applied"
}

cmd_firewall() {
  local state nft_path
  require_root
  $IP link show tailscale0 >/dev/null 2>&1 || refuse "tailscale0 is missing; join the VPN first"
  if [ "$CONSOLE" -eq 0 ] && ! vpn_session; then
    refuse "this session does not come over the VPN; connect to the VPN address, or use --console"
  fi
  local_accepts >/dev/null
  refuse_closing
  render_rules | $NFT -c -f /dev/stdin || fail "nft -c rejected the rules; nothing applied"
  if dry; then
    say "[dry-run] save ruleset to $STATE_DIR/nftables.previous, arm $ROLLBACK_UNIT ($ROLLBACK_SECONDS s), apply"
    return
  fi
  nft_path=$(command -v "$NFT") || fail "cannot resolve the path of $NFT"
  case $nft_path in *"'"*) fail "the nft path contains a quote" ;; esac
  state=$(path "$STATE_DIR")
  mkdir -p "$state"
  chmod 0700 "$state"
  rm -f "$(path "$ROLLED_BACK")"
  $NFT list ruleset >"$state/nftables.previous" || fail "could not save the current ruleset"
  write_rollback "$state/nftables.previous" "$state/nftables.rollback"
  $NFT -c -f "$state/nftables.rollback" || fail "the rollback does not validate; nothing applied"
  probe_rollback "$nft_path"
  printf '%s\n' "${SSH_CONNECTION:-console}" >"$(path "$SESSION")"
  render_rules >"$state/nftables.pending"
  arm_rollback "$nft_path"
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

# Make EL's nftables.service load the Anima rules at boot, keeping what the file holds.
install_boot_include() {
  local conf
  conf=$(path "$NFT_SYSCONFIG")
  if grep -qxF "$NFT_INCLUDE" "$conf" 2>/dev/null; then return 0; fi
  save_previous "$NFT_SYSCONFIG" nftables.conf.previous once
  { if [ -f "$conf" ]; then cat "$conf"; fi; printf '\n# Anima firewall (VPS-002)\n%s\n' "$NFT_INCLUDE"; } \
    | write_file "$NFT_SYSCONFIG" 0600
}

# firewalld would filter beside the Anima table, outside the rollback; the Anima table is
# the only host firewall (D-10). Docker keeps its own rules without firewalld.
mask_firewalld() {
  if $SYSTEMCTL is-active --quiet firewalld.service; then : >"$(path "$FIREWALLD_WAS_ACTIVE")"; fi
  $SYSTEMCTL mask firewalld.service || fail "could not mask firewalld; the Anima firewall is installed"
  $SYSTEMCTL stop firewalld.service || fail "could not stop firewalld; the Anima firewall is installed"
}

cmd_confirm() {
  local pending=$STATE_DIR/nftables.pending
  require_root
  [ -f "$(path "$pending")" ] || refuse "nothing to confirm; run firewall first"
  [ ! -e "$(path "$ROLLED_BACK")" ] || refuse "the rollback already ran; run firewall again"
  [ "$($SYSTEMCTL show --property=SubState --value "$ROLLBACK_UNIT.timer")" = waiting ] \
    || refuse "the rollback is not armed (it may have run); run firewall again"
  require_new_vpn_session
  refuse_closing
  if dry; then say "[dry-run] stop $ROLLBACK_UNIT.timer, install $NFT_RULES, enable nftables, mask firewalld"; return; fi
  $SYSTEMCTL stop "$ROLLBACK_UNIT.timer" || fail "could not stop the rollback"
  [ ! -e "$(path "$ROLLED_BACK")" ] || fail "the rollback started while confirming; run firewall again"
  write_file "$NFT_RULES" 0600 <"$(path "$pending")"
  install_boot_include
  write_file "$NFT_DROPIN" 0644 <"$HERE/nftables-unit.conf"
  $SYSTEMCTL daemon-reload
  $SYSTEMCTL enable nftables.service
  mask_firewalld
  rm -f "$(path "$pending")"
  say "Firewall confirmed, installed for boot, and firewalld masked."
}
