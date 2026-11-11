#!/bin/sh
# Manual contract test for Headscale and Tailscale (VPN-014). Needs Docker. Not a gate.
#
# It starts a throwaway Headscale (image and digest from deploy/headscale/compose.yaml, the
# repository policy.hujson, test/contract-config.yaml) and six Tailscale containers, captures
# the status and node JSON that the hub and the deploy tools parse, redacts every key, and writes
# the fixtures. It contacts no live system. The public DERP map is fetched from Tailscale Inc.
# Pre-auth keys go into container files, never onto a command line, and are never printed.
# Raw captures stay in a mode 0700 temporary folder that the exit trap deletes. Fixtures are
# written to the repository only after every file passed the redaction.
#
#   sh deploy/headscale/test/contract.sh
set -eu
umask 077

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
tool="$repo/deploy/headscale"
hub_fixtures="$repo/crates/hub/src/vpn/tests/fixtures"
deploy_fixtures="$tool/tests/fixtures"
work=$(mktemp -d)
project="anima-contract-$$"
# A private IP literal: Tailscale 1.104 forces TLS on a re-login for a host name or a public address (see contract-config.yaml).
LOGIN=http://172.29.250.10:8080

dc() { docker compose -p "$project" -f "$work/compose.yaml" "$@"; }
hs() { dc exec -T headscale headscale "$@" </dev/null; }
ts() { name=$1; shift; dc exec -T "$name" tailscale "$@" </dev/null; }
# js FILE EXPR: print EXPR (over the parsed file as `j`); prints nothing for undefined.
js() { bun -e 'const j = await Bun.file(process.argv[1]).json(); const v = eval(process.argv[2]); if (v !== undefined) console.log(v);' "$1" "$2"; }
step() { printf '\n== %s\n' "$1"; }
fail() { echo "contract: $1" >&2; exit 1; }

cleanup() {
    status=$?
    dc down -v --remove-orphans >/dev/null 2>&1 || true
    rm -rf "$work"
    if [ "$status" -ne 0 ]; then echo "contract: FAIL (exit $status)" >&2; fi
}
trap cleanup EXIT

step "Prepare $work"
headscale_image=$(sed -n 's/^ *image: *//p' "$tool/compose.yaml")
tailscale_image=$(bun -e "import { TAILSCALE_TEST_IMAGE } from '$tool/src/version.ts'; console.log(TAILSCALE_TEST_IMAGE);")
case "$tailscale_image" in *'<'*) fail "TAILSCALE_TEST_IMAGE in src/version.ts is not pinned" ;; esac
mkdir "$work/config" "$work/raw" "$work/out"
cp "$here/contract-config.yaml" "$work/config/config.yaml"
cp "$tool/config/policy.hujson" "$work/config/policy.hujson"
cat > "$work/compose.yaml" <<EOF
services:
  headscale:
    image: $headscale_image
    command: serve
    volumes: ["$work/config:/etc/headscale:ro"]
    tmpfs: ["/var/lib/headscale", "/var/run/headscale"]
    networks:
      default:
        ipv4_address: 172.29.250.10
  hub: &node
    image: $tailscale_image
    hostname: hub
    entrypoint: ["tailscaled", "--tun=userspace-networking", "--state=mem:"]
  router: { <<: *node, hostname: router-home }
  mac: { <<: *node, hostname: mac }
  stranger: { <<: *node, hostname: stranger }
  odd: { <<: *node, hostname: odd }
  mac2: { <<: *node, hostname: mac2 }
networks:
  default:
    ipam:
      config:
        - subnet: 172.29.250.0/24
EOF
dc up -d >/dev/null
n=0
until hs users list >/dev/null 2>&1; do
    n=$((n + 1)); [ "$n" -le 60 ] || fail "Headscale did not start"
    sleep 1
done

step "Users"
hs users create operator -o json > "$work/raw/user-operator.json"
hs users create intruder -o json > "$work/raw/user-intruder.json"
operator_id=$(js "$work/raw/user-operator.json" 'j.id')
intruder_id=$(js "$work/raw/user-intruder.json" 'j.id')

step "Join five nodes with keys"
# join NAME SERVICE KEY-ARGS... -- UP-ARGS...: the key is read from the JSON reply into a file.
join() {
    name=$1; service=$2; shift 2
    key_args=""
    while [ "$1" != "--" ]; do key_args="$key_args $1"; shift; done
    shift
    # shellcheck disable=SC2086
    hs preauthkeys create --expiration 1h -o json $key_args > "$work/raw/$name.json"
    bun -e 'const j = await Bun.file(process.argv[1]).json(); if (typeof j.key !== "string") process.exit(1); await Bun.write(process.argv[2], j.key);' \
        "$work/raw/$name.json" "$work/raw/$name.key" || fail "the pre-auth key reply of $name has no key"
    docker cp "$work/raw/$name.key" "$(dc ps -q "$service"):/tmp/k"
    ts "$service" up --login-server "$LOGIN" --auth-key=file:/tmp/k --hostname "$name" "$@" >/dev/null
}
join hub hub --tags tag:hub --
join router-home router --tags tag:router,tag:site-home -- --advertise-routes=10.10.0.0/24
join mac mac --user "$operator_id" --
join stranger stranger --user "$intruder_id" --
join odd odd --tags tag:unknown --

step "Register mac2 by auth ID, then again (renewal)"
# auth_id [OLD-URL]: wait for a login URL that differs from OLD-URL and print it (it holds the auth ID: keep it out of logs).
auth_id() {
    n=0
    while :; do
        ts mac2 status --json > "$work/raw/mac2-status.json" || true
        url=$(js "$work/raw/mac2-status.json" 'j.AuthURL' 2>/dev/null || true)
        if [ -n "$url" ] && [ "$url" != "${1:-}" ]; then printf '%s\n' "$url"; return; fi
        n=$((n + 1)); [ "$n" -le 30 ] || fail "mac2 shows no login URL"
        sleep 1
    done
}
ts mac2 up --login-server "$LOGIN" --hostname mac2 >/dev/null 2>&1 &
first_pid=$!
first_url=$(auth_id "")
id=${first_url##*/}
# Evidence for B1's pattern: the shape of the ID, never the ID itself.
case "$id" in hskey-authreq-*) body=${id#hskey-authreq-}; prefix=yes ;; *) body=$id; prefix=no ;; esac
auth_shape="prefix hskey-authreq-: $prefix; $(printf '%s' "$body" | wc -c | tr -d ' ') characters after it; $(printf '%s' "$body" | tr -d 'A-Za-z0-9' | wc -c | tr -d ' ') outside A-Za-z0-9 (those are: $(printf '%s' "$body" | tr -d 'A-Za-z0-9' | fold -w1 | sort -u | tr -d '\n'))"
hs auth register --user operator --auth-id "$id" -o json > "$work/raw/register.json"
wait "$first_pid" || fail "mac2 did not finish its login"
# Let the first login settle before the renewal starts.
sleep 5
ts mac2 up --force-reauth --login-server "$LOGIN" --hostname mac2 >/dev/null 2>&1 &
second_pid=$!
second_url=$(auth_id "$first_url")
hs auth register --user operator --auth-id "${second_url##*/}" -o json > "$work/raw/register-again.json"
wait "$second_pid" || fail "mac2 did not finish its renewal"
echo "auth ID: $auth_shape"

step "Policy check (VPN-014)"
# Headscale refuses a tests row whose tag or user has no node, so the check runs after the joins.
hs policy check --file /etc/headscale/policy.hujson >/dev/null || fail "headscale policy check refused policy.hujson"

step "Approve the router route"
hs nodes list -o json > "$work/raw/nodes-before.json"
router_id=$(js "$work/raw/nodes-before.json" 'j.find((node) => node.name === "router-home").id')
hs nodes approve-routes --identifier="$router_id" --routes=10.10.0.0/24 >/dev/null

step "Capture status from the hub"
peer='Object.values(j.Peer ?? {}).find((p) => p.HostName === "router-home")'
n=0
while :; do
    ts hub status --json > "$work/raw/status-idle.json"
    [ "$(js "$work/raw/status-idle.json" "$peer?.Online === true && ($peer.PrimaryRoutes ?? []).length > 0")" = true ] && break
    n=$((n + 1)); [ "$n" -le 90 ] || fail "the hub never saw the router online with its route"
    sleep 1
done
router_ip=$(js "$work/raw/status-idle.json" "$peer.TailscaleIPs[0]")
active() {
    ts hub status --json > "$work/raw/status-active.json"
    [ "$(js "$work/raw/status-active.json" "$peer?.Active === true && ($peer.CurAddr ?? '') !== ''")" = true ]
}
how=
ts hub ping --c 3 "$router_ip" >/dev/null 2>&1 || true
if active; then how="disco ping"; fi
if [ -z "$how" ]; then
    ts hub ping --tsmp --c 3 "$router_ip" >/dev/null 2>&1 || true
    if active; then how="tsmp ping"; fi
fi
if [ -z "$how" ]; then
    ts hub nc "$router_ip" 22 </dev/null >/dev/null 2>&1 || true
    if active; then how="tailscale nc"; fi
fi
[ -n "$how" ] || fail "the router peer is not Active with a CurAddr after a ping, a TSMP ping and nc"
echo "active peer set by: $how"
echo "waiting 60 s with no traffic (BD-13)"
sleep 60
ts hub status --json > "$work/raw/status-after-60s.json"
echo "after 60 s: Active=$(js "$work/raw/status-after-60s.json" "$peer?.Active") CurAddr=$(js "$work/raw/status-after-60s.json" "$peer?.CurAddr")"
hs nodes list -o json > "$work/raw/nodes.json"
hs nodes list-routes -o json > "$work/raw/routes.json"

step "Check the inventory"
printf '%s\n' hub router-home mac stranger odd mac2 > "$work/raw/expected.txt"
bun "$tool/bin/inventory.ts" "$work/raw/nodes.json" "$work/raw/routes.json" "$work/raw/expected.txt" > "$work/raw/inventory.txt" || true
cat "$work/raw/inventory.txt"
printf '%s\n' '! foreign-user stranger (intruder)' '! unknown-tag odd (tag:unknown)' > "$work/raw/inventory-expected.txt"
diff "$work/raw/inventory-expected.txt" "$work/raw/inventory.txt" >/dev/null || fail "the inventory is not exactly foreign-user stranger and unknown-tag odd"

step "Redact and write the fixtures"
mkdir "$work/out/hub" "$work/out/deploy"
cp "$work/raw/hub.json" "$work/raw/preauthkey.json"
for pair in "status-idle:hub" "status-active:hub" "status-after-60s:hub" "nodes:deploy" "routes:deploy" "preauthkey:deploy" "register:deploy" "register-again:deploy"; do
    file=${pair%%:*}
    bun "$tool/bin/redact.ts" < "$work/raw/$file.json" > "$work/out/${pair##*:}/$file.json" || fail "redaction of $file failed"
done
mkdir -p "$hub_fixtures" "$deploy_fixtures"
cp "$work/out/hub/"*.json "$hub_fixtures/"
cp "$work/out/deploy/"*.json "$deploy_fixtures/"
grep -q REDACTED-PREAUTH-KEY "$deploy_fixtures/preauthkey.json" || fail "preauthkey.json has no placeholder"
grep -q REDACTED-PREAUTH-KEY "$deploy_fixtures/nodes.json" || fail "nodes.json has no placeholder"

echo "contract: ok"
