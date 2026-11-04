# shellcheck shell=bash
# The `firewall` and `confirm` commands: apply nftables.conf with a 180 s rollback that only
# a new SSH session over the VPN can cancel, then install it for boot and mask firewalld
# (VPS-002, DNS-005, D-10). Sourced by harden.sh after open-ports.sh; needs harden.sh's
# say, refuse, fail, dry, path, write_file, save_previous, require_root and
# docker_version_problem, NFT, SYSTEMCTL,
# SYSTEMD_RUN, IP, SS, STATE_DIR, CONSOLE, HERE and SELF, and open-ports.sh's
# is_vpn_address, local_accepts, refuse_closing and LOCAL_ALLOW.

NFT_RULES=/etc/nftables/anima.nft
NFT_SYSCONFIG=/etc/sysconfig/nftables.conf
NFT_INCLUDE="include \"$NFT_RULES\""
NFT_DROPIN=/etc/systemd/system/nftables.service.d/anima.conf
NFT_UNIT_FRAGMENT=/usr/lib/systemd/system/nftables.service
NFT_UNIT_PROPERTIES=ExecStart,ExecStartPre,ExecStartPost,ExecReload,ExecStop,ExecStopPost,FragmentPath,DropInPaths

ROLLBACK_UNIT=anima-firewall-rollback
ROLLBACK_SECONDS=180
ROLLED_BACK=$STATE_DIR/firewall-rolled-back
SESSION=$STATE_DIR/firewall-session
PEERS=$STATE_DIR/firewall-peers
FIREWALLD_WAS_ACTIVE=$STATE_DIR/firewalld-was-active
# Under SELinux a unit's /bin/sh runs as initrc_t and its nft as iptables_t, and the
# targeted policy lets iptables_t read no var_lib_t file, so the rollback could not read
# /var/lib/anima. The rollback unit therefore runs unconfined, as the backup unit does.
ROLLBACK_CONTEXT=system_u:system_r:unconfined_service_t:s0

# Print each problem with the live and boot firewall state.
firewall_problems() {
  $IP link show tailscale0 >/dev/null 2>&1 || say "tailscale0 is missing"
  $NFT list table inet anima_filter >/dev/null 2>&1 || say "table inet anima_filter is not loaded"
  $SYSTEMCTL is-enabled --quiet nftables.service || say "nftables.service is not enabled"
  if grep -qxF "$NFT_INCLUDE" "$(path "$NFT_SYSCONFIG")" 2>/dev/null; then
    $NFT -c -f "$(path "$NFT_SYSCONFIG")" >/dev/null 2>&1 || say "nft -c rejects $NFT_SYSCONFIG; nftables.service would fail at boot"
  else
    say "$NFT_SYSCONFIG does not load $NFT_RULES at boot"
  fi
  sysconfig_extra_lines
  docker_range_problems
  [ -f "$(path "$NFT_DROPIN")" ] || say "nftables.service would flush the whole ruleset on stop or reload"
  nft_unit_problems
  [ "$($SYSTEMCTL is-enabled firewalld.service 2>/dev/null)" = masked ] || say "firewalld is not masked"
  (local_accepts >/dev/null) 2>/dev/null || say "$LOCAL_ALLOW has a line that is not one PORT/PROTO"
}

# Print nftables.service's settings as systemd has loaded them, one "Exec...=ARGV" line per
# command, its arguments up to systemd's " ; ignore_errors=" (/usr/sbin/nft written as
# /sbin/nft), and the FragmentPath and DropInPaths lines.
# systemd prints no line for a command list that is empty.
nft_unit_settings() {
  $SYSTEMCTL show -p "$NFT_UNIT_PROPERTIES" nftables.service | awk '
    /^Exec/ {
      key = $0; sub(/=.*/, "", key); line = $0
      while ((start = index(line, "argv[]=")) > 0) {
        line = substr(line, start + 7); end = index(line, " ; ignore_errors=")
        argv = end > 0 ? substr(line, 1, end - 1) : line; sub(/^\/usr\/sbin\/nft /, "/sbin/nft ", argv)
        print key "=" argv; line = end > 0 ? substr(line, end) : ""
      }
      next
    }
    { print }'
}

# Print each part of nftables.service the Anima drop-in cannot override: a unit file other
# than the package's (an override in /etc or /run), another drop-in, and an enabled
# iptables or ip6tables service, any of which could flush Docker's tables.
nft_unit_foreign() {
  local settings dropin unit
  settings=$(nft_unit_settings) || { say "systemctl show nftables.service failed"; return; }
  grep -qxF "FragmentPath=$NFT_UNIT_FRAGMENT" <<<"$settings" \
    || say "nftables.service is not loaded from $NFT_UNIT_FRAGMENT; remove the override in /etc or /run"
  while read -r dropin; do
    [ -z "$dropin" ] || [ "$dropin" = "$NFT_DROPIN" ] || say "nftables.service has another drop-in: $dropin"
  done < <(awk '/^DropInPaths=/ { sub(/^DropInPaths=/, ""); for (i = 1; i <= NF; i++) print $i }' <<<"$settings")
  for unit in iptables.service ip6tables.service; do
    if $SYSTEMCTL is-enabled --quiet "$unit" 2>/dev/null; then say "$unit is enabled; it replaces the ruleset when it starts or stops"; fi
  done
}

# Print each problem with nftables.service once the Anima drop-in is in place: anything
# nft_unit_foreign finds, and any command other than `nft -f` of the boot file at start
# and reload.
nft_unit_problems() {
  local settings line
  nft_unit_foreign
  settings=$(nft_unit_settings) || return 0
  case " $(sed -n 's/^DropInPaths=//p' <<<"$settings") " in
    *" $NFT_DROPIN "*) ;;
    *) say "nftables.service has not loaded $NFT_DROPIN (systemctl daemon-reload)" ;;
  esac
  while IFS= read -r line; do
    case $line in
      Exec*) ;;
      *) continue ;;
    esac
    case $line in
      "ExecStart=/sbin/nft -f $NFT_SYSCONFIG" | "ExecReload=/sbin/nft -f $NFT_SYSCONFIG") ;;
      *) say "nftables.service runs \"$line\"; only nft -f $NFT_SYSCONFIG is expected, at start and reload" ;;
    esac
  done <<<"$settings"
}

# Print each interface other than lo, tailscale0, docker0 and br-* that holds an IPv4
# address in Docker's range 172.16.0.0/12 (a provider private network, for example): the
# table's prerouting drop for that range would cut it off, so the operator decides first.
docker_range_problems() {
  local addresses
  addresses=$($IP -o -4 addr show) || { say "ip -o -4 addr show failed"; return; }
  awk '{
    device = $2; sub(/@.*/, "", device); split($4, octet, /[.\/]/)
    if (octet[1] == 172 && octet[2] >= 16 && octet[2] <= 31 && device !~ /^(lo|tailscale0|docker0|br-.*)$/)
      print "interface " device " holds " $4 " in 172.16.0.0/12, which the Anima table drops off lo, tailscale0 and the Docker bridges"
  }' <<<"$addresses"
}

# Print each active line of the boot file other than the Anima include. EL ships it as
# comments only; a rule or an include there could flush Docker's and tailscaled's tables
# at boot, which no check of this file alone can rule out.
sysconfig_extra_lines() {
  local conf
  conf=$(path "$NFT_SYSCONFIG")
  [ -f "$conf" ] || return 0
  awk -v name="$NFT_SYSCONFIG" -v anima_include="$NFT_INCLUDE" '/^[[:space:]]*(#|$)/ || $0 == anima_include { next }
    { print name ":" FNR ": only the Anima include may be active in this file" }' "$conf"
}

# Refuse while the boot file holds anything but comments and the Anima include.
refuse_sysconfig_extras() {
  local extras
  extras=$(sysconfig_extra_lines)
  [ -z "$extras" ] || refuse "$extras; move those rules into a file of their own, or comment them out, first"
}

# Print "ADDRESS PORT" for an address and port as ss or SSH_CONNECTION print them: no
# brackets, no IPv4-mapped prefix, no scope, lower case.
peer_key() {
  local address=${1#[}
  address=${address%]}
  address=${address#::ffff:}
  address=${address%%\%*}
  printf '%s %s\n' "$(tr 'A-F' 'a-f' <<<"$address")" "$2"
}

# Print "ADDRESS PORT" for each SSH connection established now.
ssh_peers() {
  local sockets peer
  sockets=$($SS -Htn state established '( sport = :22 )') || return 1
  while read -r _ _ _ peer _; do
    [ -n "${peer:-}" ] || continue
    peer_key "${peer%:*}" "${peer##*:}"
  done <<<"$sockets"
}

# True when this SSH session runs over the VPN at both ends.
vpn_session() {
  local client server
  read -r client _ server _ <<<"${SSH_CONNECTION:-}"
  is_vpn_address "${client:-}" && is_vpn_address "${server:-}"
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
  local state nft_path problems
  require_root
  $IP link show tailscale0 >/dev/null 2>&1 || refuse "tailscale0 is missing; join the VPN first"
  if [ "$CONSOLE" -eq 0 ] && ! vpn_session; then
    refuse "this session does not come over the VPN; connect to the VPN address, or use --console"
  fi
  local_accepts >/dev/null
  refuse_closing
  refuse_sysconfig_extras
  problems=$(docker_version_problem; docker_range_problems)
  [ -z "$problems" ] || refuse "$problems; ask the programme before applying the firewall"
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
  ssh_peers >"$(path "$PEERS")" || fail "ss could not list the SSH connections; nothing applied"
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
  # Any session open before firewall ran proves nothing about new connections.
  local client port
  read -r client port _ <<<"${SSH_CONNECTION:-}"
  [ -f "$(path "$PEERS")" ] || refuse "no recorded SSH connections; run firewall again"
  ! grep -qxF "$(peer_key "$client" "$port")" "$(path "$PEERS")" \
    || refuse "this SSH connection was open before firewall ran; confirm from a NEW SSH session over the VPN"
}

# Refuse unless nft accepts the boot file as it will be once confirm installs the rules:
# what the file holds now, without the Anima include, then the pending rules.
check_boot_candidate() {
  { grep -vxF "$NFT_INCLUDE" "$(path "$NFT_SYSCONFIG")" 2>/dev/null || true
    printf 'include "%s"\n' "$(path "$1")"; } | $NFT -c -f /dev/stdin \
    || refuse "nft -c rejects the boot file with the new rules; nothing installed"
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
  local pending=$STATE_DIR/nftables.pending problems
  require_root
  [ -f "$(path "$pending")" ] || refuse "nothing to confirm; run firewall first"
  [ ! -e "$(path "$ROLLED_BACK")" ] || refuse "the rollback already ran; run firewall again"
  [ "$($SYSTEMCTL show --property=SubState --value "$ROLLBACK_UNIT.timer")" = waiting ] \
    || refuse "the rollback is not armed (it may have run); run firewall again"
  require_new_vpn_session
  refuse_closing
  refuse_sysconfig_extras
  problems=$(nft_unit_foreign)
  [ -z "$problems" ] || refuse "$problems"
  check_boot_candidate "$pending"
  if dry; then say "[dry-run] stop $ROLLBACK_UNIT.timer, install $NFT_RULES, enable nftables, mask firewalld"; return; fi
  $SYSTEMCTL stop "$ROLLBACK_UNIT.timer" || fail "could not stop the rollback"
  [ ! -e "$(path "$ROLLED_BACK")" ] || fail "the rollback started while confirming; run firewall again"
  write_file "$NFT_RULES" 0600 <"$(path "$pending")"
  install_boot_include
  write_file "$NFT_DROPIN" 0644 <"$HERE/nftables-unit.conf"
  $SYSTEMCTL daemon-reload
  problems=$(nft_unit_problems)
  [ -z "$problems" ] || fail "$problems; firewalld left running, the Anima rules are live until reboot"
  $SYSTEMCTL enable nftables.service
  $NFT -c -f "$(path "$NFT_SYSCONFIG")" || fail "nft -c rejects $NFT_SYSCONFIG; firewalld left running, the Anima rules are live until reboot"
  mask_firewalld
  rm -f "$(path "$pending")"
  say "Firewall confirmed, installed for boot, and firewalld masked."
}
