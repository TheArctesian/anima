# shellcheck shell=bash
# Which ports are open to the internet: the ports the Anima table keeps open, the
# operator's allow file, the public sockets and firewalld's openings, and the guard that
# refuses to close any of them silently (VPS-002, VPS-005). Sourced by harden.sh; needs its
# say, refuse, fail and path, and SS, SYSTEMCTL, FIREWALL_CMD and ALLOW_CLOSING.

ALLOWED_PUBLIC=" 22/tcp 443/tcp 3478/udp 41641/udp "
LOCAL_ALLOW=/etc/anima/firewall-allow

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
