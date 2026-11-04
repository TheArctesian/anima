#!/usr/bin/env bash
# Renders the Headscale config template on the VPS (WP-03 T2; VPS-002, VPS-003).
#
#   VPS_PUBLIC_IPV4=<ip> ACME_EMAIL=<address> render.sh <template> <out>
#
# Refuses, and writes nothing, unless VPS_PUBLIC_IPV4 is a public dotted-quad IPv4
# address on a local interface and ACME_EMAIL looks like an address. Without this,
# envsubst would turn an unset VPS_PUBLIC_IPV4 into "", and the listeners ":443" and
# ":3478" would bind every interface. The output is written to a private temp file,
# checked, then moved into place, so <out> is never partial.
# ENVSUBST and IP name the envsubst and ip commands (tests pass fakes).
# Run it as root (sudo): the output is 0600, and the container runs as root without
# DAC_OVERRIDE, so it can read the file only if root owns it.
set -euo pipefail

ENVSUBST=${ENVSUBST:-envsubst}
IP=${IP:-ip}

fail() {
  printf 'render.sh: %s\n' "$1" >&2
  exit 1
}

octet='(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])'

# Prints nothing; exits non-zero unless $1 is a strict, public dotted quad.
check_ipv4() {
  local address=$1
  [[ $address =~ ^$octet\.$octet\.$octet\.$octet$ ]] || fail "VPS_PUBLIC_IPV4 is not a dotted-quad IPv4 address"
  local a=${BASH_REMATCH[1]} b=${BASH_REMATCH[2]} c=${BASH_REMATCH[3]}
  if ((a == 0 || a == 10 || a == 127 || a >= 224)) ||
    ((a == 100 && b >= 64 && b <= 127)) ||
    ((a == 169 && b == 254)) ||
    ((a == 172 && b >= 16 && b <= 31)) ||
    ((a == 192 && b == 168)) ||
    ((a == 192 && b == 0 && c == 2)) ||
    ((a == 198 && b == 51 && c == 100)) ||
    ((a == 203 && b == 0 && c == 113)); then
    fail "VPS_PUBLIC_IPV4 is not a public address (private, CGNAT, loopback, link-local, documentation or reserved)"
  fi
}

check_interface() {
  local addresses
  addresses=$("$IP" -4 -o addr show) || fail "cannot list local IPv4 addresses with '$IP'"
  [[ $addresses == *" $1/"* ]] || fail "VPS_PUBLIC_IPV4 is not on a local interface"
}

check_email() {
  [[ $1 =~ ^[^@[:space:]\"\\\$]+@[^@[:space:]\"\\\$]+\.[^@[:space:]\"\\\$]+$ ]] || fail "ACME_EMAIL does not look like an email address"
}

check_rendered() {
  local file=$1 address=$2
  grep -qxF "listen_addr: \"$address:443\"" "$file" || fail "rendered listen_addr is not $address:443"
  grep -qxF "    stun_listen_addr: \"$address:3478\"" "$file" || fail "rendered stun_listen_addr is not $address:3478"
  # shellcheck disable=SC2016 # a literal "${" is what must not remain
  if grep -qF '${' "$file"; then
    fail "a placeholder is left after rendering"
  fi
}

main() {
  (($# == 2)) || fail "usage: render.sh <template> <out>"
  local template=$1 out=$2
  [[ -f $template ]] || fail "template not found: $template"
  local address=${VPS_PUBLIC_IPV4:-}
  check_ipv4 "$address"
  check_interface "$address"
  check_email "${ACME_EMAIL:-}"

  umask 077
  temporary=$(mktemp "$(dirname "$out")/.render.XXXXXX") || fail "cannot create a temp file beside $out"
  trap 'rm -f "$temporary"' EXIT
  # shellcheck disable=SC2016 # the literal variable list limits envsubst to these two
  "$ENVSUBST" '${VPS_PUBLIC_IPV4} ${ACME_EMAIL}' <"$template" >"$temporary" || fail "envsubst failed"
  check_rendered "$temporary" "$address"
  mv -f "$temporary" "$out"
  trap - EXIT
}

main "$@"
