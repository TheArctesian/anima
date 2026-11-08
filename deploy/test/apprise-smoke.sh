#!/bin/sh
# Manual smoke test for the optional Apprise service in deploy/compose.yaml (D-13, NOT-002). Needs
# Docker. Not a gate.
#
# It copies only compose.yaml to a temporary folder and starts only `apprise`, with the hardening
# exactly as in the file (read-only root, no capabilities, user 1000, tmpfs mounts). No secret file
# is read or needed. A throwaway container on the same `notify` network then asks
# GET /status at the fixed address 172.30.3.10, and posts stateless /notify calls with json://, xml:// and
# form:// URLs to a listener in that container: all must be refused (APPRISE_ALLOW_SERVICES).
#
#   sh deploy/test/apprise-smoke.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
work=$(mktemp -d)
project="anima-apprise-smoke-$$"

cleanup() {
    status=$?
    (cd "$work" && docker compose -p "$project" --profile apprise down -v --remove-orphans >/dev/null 2>&1) || true
    rm -rf "$work"
    if [ "$status" -eq 0 ]; then echo "apprise-smoke: PASS"; else echo "apprise-smoke: FAIL (exit $status)" >&2; fi
}
trap cleanup EXIT

step() { printf '\n== %s\n' "$1"; }
fail() { echo "apprise-smoke: $1" >&2; exit 1; }

cp "$repo/deploy/compose.yaml" "$work/compose.yaml"
cd "$work"
# Dummy values for the variables that compose.yaml requires. Nothing here is a real address.
export ANIMA_HUB_ADDRESS=127.0.0.1
export ANIMA_ACME_EMAIL=smoke@example.invalid
export ANIMA_VPS_PUBLIC_ADDRESS=203.0.113.9
compose() { docker compose -p "$project" --profile apprise "$@"; }
image=$(compose config --images apprise)

step "Start only apprise and wait until it is healthy"
compose up -d --wait --wait-timeout 90 apprise
[ "$(compose ps --services --status running)" = apprise ] || fail "something besides apprise is running"
container=$(compose ps -q apprise)
echo "image:    $image"
echo "user:     $(docker inspect --format '{{.Config.User}}' "$container")"
echo "readonly: $(docker inspect --format '{{.HostConfig.ReadonlyRootfs}}' "$container")"
echo "capdrop:  $(docker inspect --format '{{.HostConfig.CapDrop}}' "$container")"
echo "capadd:   $(docker inspect --format '{{.HostConfig.CapAdd}}' "$container")"
echo "ports:    '$(docker port "$container")'"
[ -z "$(docker port "$container")" ] || fail "apprise publishes a port"

step "GET /status from a throwaway container on the notify network"
network="${project}_notify"
# Same image, so nothing else is pulled. `curl` exists in it.
probe() { docker run --rm --network "$network" --entrypoint "$@"; }
probe curl "$image" -fsS http://172.30.3.10:8000/status
echo

step "A json:// URL to any host is refused, and the listener gets nothing (S-1)"
# Apprise sits outside the hub's deny list, so only cloud services are allowed. The listener
# stands for a host on a protected network. The single quotes are meant: the container expands them.
# shellcheck disable=SC2016
docker run --rm --network "$network" --entrypoint sh "$image" -c '
    ip=$(hostname -i)
    python3 -c "
import http.server
class H(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        self.rfile.read(int(self.headers.get(\"Content-Length\", 0)))
        self.send_response(200); self.end_headers()
        print(\"listener got POST\", flush=True)
    def log_message(self, *a): pass
s = http.server.HTTPServer((\"0.0.0.0\", 9999), H)
s.timeout = 4
s.handle_request()
" > /tmp/listener.log 2>&1 &
    sleep 1
    for scheme in json xml form; do
        code=$(curl -sS -o /tmp/reply -w "%{http_code}" -X POST http://172.30.3.10:8000/notify \
            -H "Content-Type: application/json" \
            -d "{\"urls\":\"$scheme://$ip:9999\",\"body\":\"hello\"}")
        echo "POST /notify $scheme:// -> $code"
        case "$code" in 4??) ;; *) echo "expected a 4xx for $scheme" >&2; exit 1 ;; esac
    done
    sleep 5
    ! grep -q "listener got POST" /tmp/listener.log || { echo "the listener received a POST" >&2; exit 1; }
    echo "listener received nothing"
'

step "Nothing can be stored"
mounts=$(docker inspect --format '{{range .Mounts}}{{.Type}}:{{.Destination}} {{end}}' "$container")
echo "mounts: $mounts"
case "$mounts" in *volume:* | *bind:*) fail "apprise has a volume or bind mount" ;; esac
echo "tmpfs:  $(docker inspect --format '{{json .HostConfig.Tmpfs}}' "$container")"
# /status leaves its own writability probe (tmp_hc) behind; a stateless call must leave nothing else.
files=$(docker exec "$container" find /config /attach /plugin -type f -not -path '*tmp_hc*' -not -name '.tmp_hc')
[ -z "$files" ] || fail "apprise wrote state: $files"
echo "/config, /attach and /plugin hold no file but the status probe"
