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

# Print one line per entry of `firewall-cmd --zone=ZONE --list-all` that can let traffic
# in: "PORT/PROTO" for each port, "service NAME" for each service, and "ZONE:KEY" for
# anything else that is set (sources, protocols, forward-ports, source-ports, masquerade,
# rich rules, a target other than default, keys a newer firewalld adds). Interfaces,
# priorities, intra-zone forwarding and ICMP blocks let nothing in from outside.
zone_entries() {
  $FIREWALL_CMD --zone="$1" --list-all | awk -v zone="$1" '
    /^\t/ { if (key != "") value[key] = value[key] " " $0; next }
    /^[[:space:]]+[a-z][a-z -]*:/ {
      key = $0; sub(/^[[:space:]]+/, "", key); sub(/:.*/, "", key)
      value[key] = $0; sub(/^[^:]*:/, "", value[key]); order[++n] = key
    }
    END {
      for (i = 1; i <= n; i++) {
        k = order[i]; v = value[k]; gsub(/^[[:space:]]+|[[:space:]]+$/, "", v)
        if (k ~ /^(interfaces|ingress-priority|egress-priority|forward|icmp-blocks)$/) continue
        if (k == "ports" || k == "services") {
          c = split(v, words, /[[:space:]]+/)
          for (j = 1; j <= c; j++) if (words[j] != "") print (k == "ports" ? "" : "service ") words[j]
          continue
        }
        if (k == "target" && v == "default") continue
        if (k ~ /^(masquerade|icmp-block-inversion)$/ && v == "no") continue
        if (v != "") { gsub(/ /, "-", k); print zone ":" k }
      }
    }'
}

# Print the ports of firewalld service $2 in zone $1, or "service:NAME" when the service
# also opens protocols or source ports or includes other services.
service_openings() {
  local service_info port
  service_info=$($FIREWALL_CMD --info-service="$2") || { say "service:$2 firewalld zone $1 (unknown service)"; return; }
  while read -r port; do
    say "$port firewalld zone $1 service $2"
  done < <(awk '$1 == "ports:" { for (i = 2; i <= NF; i++) print $i }' <<<"$service_info")
  if awk '$1 ~ /^(protocols|source-ports|includes):$/ && NF > 1 { found = 1 } END { exit !found }' <<<"$service_info"; then
    say "service:$2 firewalld zone $1 (protocols, source ports or included services)"
  fi
}

# Print "TOKEN firewalld ..." for each policy and direct rule firewalld has beside its zones.
# Docker adds the docker-forwarding policy for its own bridges; allow-host-ipv6 is built in.
firewalld_extras() {
  local policy
  while read -r policy _; do
    case $policy in '' | allow-host-ipv6 | docker-forwarding) continue ;; esac
    say "policy:$policy firewalld policy $policy"
  done < <($FIREWALL_CMD --get-active-policies | grep -v '^[[:space:]]')
  if [ -n "$($FIREWALL_CMD --direct --get-all-rules)$($FIREWALL_CMD --direct --get-all-chains)$($FIREWALL_CMD --direct --get-all-passthroughs)" ]; then
    say "direct-rules firewalld direct rules, chains or passthroughs"
  fi
}

# Print "PORT/PROTO firewalld zone ZONE ..." for each opening firewalld has now, and a
# token (ZONE:KEY, service:NAME, policy:NAME, direct-rules) for anything else that could
# let traffic in: the guard refuses those too, because no port check can see through them.
firewalld_openings() {
  local zones zone entry
  $SYSTEMCTL is-active --quiet firewalld.service || return 0
  zones=$($FIREWALL_CMD --get-active-zones) || { say "unknown firewall-cmd --get-active-zones failed"; return; }
  while read -r zone _; do
    # Docker's own zone holds docker0 (target ACCEPT); it is not an opening to the internet.
    case $zone in '' | docker) continue ;; esac
    while read -r entry; do
      case $entry in
        '') ;;
        "service "*) service_openings "$zone" "${entry#service }" ;;
        *) say "$entry firewalld zone $zone" ;;
      esac
    done < <(zone_entries "$zone")
  done < <(grep -v '^[[:space:]]' <<<"$zones")
  firewalld_extras
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
    # Docker's DNAT runs before the input chain, so the Anima table does not close a port
    # Docker publishes; daemon.json moves it to 127.0.0.1 when the container is recreated.
    case $what in
      *'(("docker-proxy",'* | *'(("dockerd",'*)
        say "note: $port ($what) is published by Docker, not closed by this firewall; see guide 4.4" >&2
        continue ;;
    esac
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
