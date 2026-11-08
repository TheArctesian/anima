#!/bin/sh
# Manual smoke test for deploy/compose.yaml (DB-002). Needs Docker. Not a gate.
#
# It works in a temporary copy of the deploy folder, so it never reads deploy/secrets.
# TEST-ONLY: the secret files are made readable by everyone (mode 0444), because the host
# owner (root:999) of a real install cannot be set without root. Never do this on the host.
# It contacts neither Let's Encrypt nor Cloudflare: `certs` runs only with a stub entrypoint
# (the hook on fixture certificates and `/lego --version`), never its real entry point.
# `migrate` runs the real migrations as the owner role; then `hub` starts with the runtime
# login only, must become healthy, and must answer TLS, never plain HTTP, on the published port.
#
#   sh deploy/test/smoke.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
work=$(mktemp -d)
project="anima-smoke-$$"

cleanup() {
    status=$?
    (cd "$work/deploy" && docker compose -p "$project" down -v --remove-orphans >/dev/null 2>&1) || true
    rm -rf "$work"
    if [ "$status" -eq 0 ]; then echo "smoke: PASS"; else echo "smoke: FAIL (exit $status)" >&2; fi
}
trap cleanup EXIT

step() { printf '\n== %s\n' "$1"; }
fail() { echo "smoke: $1" >&2; exit 1; }

step "Docker Engine version (NET-003)"
sh "$repo/deploy/bin/preflight" docker "$(docker version --format '{{.Server.Version}}')"

step "Prepare a temporary copy of deploy/ in $work"
mkdir -p "$work/deploy/dns" "$work/deploy/headscale" "$work/apps/desktop" "$work/quality" "$work/design"
cp -R "$repo/deploy/compose.yaml" "$repo/deploy/postgres" "$repo/deploy/certs" "$repo/deploy/gateway" "$repo/deploy/hub" "$work/deploy/"
cp -R "$repo/Cargo.toml" "$repo/Cargo.lock" "$repo/crates" "$work/"
cp -R "$repo/apps/desktop/src-tauri" "$work/apps/desktop/src-tauri"
# The web stage of the hub image: root and workspace manifests, the lock file and the client sources.
cp "$repo/package.json" "$repo/bun.lock" "$work/"
cp "$repo/apps/desktop/package.json" "$work/apps/desktop/"
cp "$repo/quality/package.json" "$work/quality/"
cp "$repo/design/package.json" "$work/design/"
cp "$repo/deploy/package.json" "$work/deploy/"
cp "$repo/deploy/dns/package.json" "$work/deploy/dns/"
cp "$repo/deploy/headscale/package.json" "$work/deploy/headscale/"
mkdir "$work/apps/client"
cp -R "$repo/apps/client/src" "$repo/apps/client/static" "$work/apps/client/"
cp "$repo/apps/client/package.json" "$repo/apps/client/svelte.config.js" "$repo/apps/client/vite.config.ts" \
    "$repo/apps/client/tsconfig.json" "$work/apps/client/"
find "$work/apps/client" -name '.env*' -exec rm -f {} +
printf '%s\n' 'smoke-test-not-a-token' |
    ANIMA_SECRETS_CHOWN=0 sh "$repo/deploy/bin/make-secrets" --cloudflare-token-stdin "$work/deploy/secrets"
sh "$repo/deploy/bin/preflight" secrets "$work/deploy/secrets"
chmod 0444 "$work/deploy/secrets/"*
# Fixture certificate for the hook test. Test-only: world-readable so that root without
# capabilities inside the container can read it.
mkdir "$work/fixture"
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes -days 1 \
    -subj /CN=hub.anima.ing -keyout "$work/fixture/chain.key" -out "$work/fixture/chain.crt" 2>/dev/null
chmod 0444 "$work/fixture/"*

cd "$work/deploy"
export ANIMA_HUB_ADDRESS=127.0.0.1
export ANIMA_ACME_EMAIL=smoke@example.invalid
export ANIMA_VPS_PUBLIC_ADDRESS=203.0.113.9
compose() { docker compose -p "$project" "$@"; }

step "docker compose config"
compose config --quiet
if (unset ANIMA_HUB_ADDRESS; compose config --quiet 2>/dev/null); then
    fail "compose config accepted a missing ANIMA_HUB_ADDRESS"
fi
echo "config is valid, and a missing ANIMA_HUB_ADDRESS is refused"

step "Build the gateway image and check the font (RA-011)"
compose build guacd
# The single quotes are meant: the shell inside the container expands the variables.
# shellcheck disable=SC2016
compose run --rm --no-deps --entrypoint sh guacd -c '
    test -f /usr/share/fonts/opendyslexic/OpenDyslexicMNerdFontMono-Regular.otf
    test -f /usr/share/fonts/opendyslexic/LICENSE-OpenDyslexic.txt
    if command -v fc-list >/dev/null 2>&1; then fc-list | grep -i opendyslexic; fi
    if command -v fc-match >/dev/null 2>&1; then
        # the guacd font-name must be this exact family (fc-match OpenDyslexic gives DejaVu Sans).
        family=$(fc-match -f "%{family}" "OpenDyslexicM Nerd Font Mono")
        [ "$family" = "OpenDyslexicM Nerd Font Mono" ] || { echo "fc-match gave $family" >&2; exit 1; }
        echo "fc-match: $family"
    fi
    id -un
'

step "Start postgres and wait until it is healthy"
compose up -d --wait --wait-timeout 120 postgres

step "Check the roles (DB-002, DB-003)"
roles=$(compose exec -T postgres psql -X -U postgres -Atc \
    "select rolname||' login='||rolcanlogin||' super='||rolsuper||' createrole='||rolcreaterole from pg_roles where rolname like 'anima%' order by 1")
echo "$roles"
expected="anima_hub login=true super=false createrole=false
anima_owner login=true super=false createrole=false
anima_runtime login=false super=false createrole=false"
[ "$roles" = "$expected" ] || fail "unexpected roles"
owner=$(compose exec -T postgres psql -X -U postgres -Atc "select pg_get_userbyid(datdba) from pg_database where datname='anima'")
[ "$owner" = anima_owner ] || fail "database anima is owned by '$owner'"
# The single quotes are meant: the shell inside the container expands the variables.
# shellcheck disable=SC2016
compose exec -T postgres sh -c \
    'PGPASSWORD=$(cat "$ANIMA_HUB_PASSWORD_FILE") psql -X -h 127.0.0.1 -U anima_hub -d anima -Atc "select current_user"'
# shellcheck disable=SC2016
compose exec -T postgres sh -c \
    'PGPASSWORD=$(cat "$ANIMA_OWNER_PASSWORD_FILE") psql -X -h 127.0.0.1 -U anima_owner -d anima -Atc "select current_user"'
ports=$(docker port "$(compose ps -q postgres)")
[ -z "$ports" ] || fail "postgres publishes $ports"
echo "no published port on postgres"

step "certs: real hardening, stub entrypoint (DNS-002, SEC-001)"
# The single quotes are meant: the shell inside the container expands the variables.
# shellcheck disable=SC2016
compose run --rm --no-deps --entrypoint sh -v "$work/fixture:/fixture:ro" \
    -e LEGO_CERT_PATH=/fixture/chain.crt -e LEGO_CERT_KEY_PATH=/fixture/chain.key certs -c '
    /lego --version
    sh /anima/certs/install.sh
    sh /anima/certs/install.sh | grep -q "up to date"
    ls -ln /certs/hub
    owner=$(stat -c "%u:%g %a" /certs/hub/tls.pem)
    [ "$owner" = "10001:10001 400" ] || { echo "tls.pem has owner/mode $owner" >&2; exit 1; }
'

step "Build the hub image from this checkout (no cached image)"
compose build hub

step "hub: runs as 10001 and reads its secrets"
# shellcheck disable=SC2016
compose run --rm --no-deps --entrypoint sh hub -c '
    [ "$(id -u):$(id -g)" = 10001:10001 ] || { echo "hub user is $(id -u):$(id -g)" >&2; exit 1; }
    test -r /run/secrets/database_url && test -r /run/secrets/anima_key && echo "secrets readable"
    test -r /certs/hub/tls.pem && echo "tls.pem readable"
    ! touch /certs/hub/x 2>/dev/null && echo "certs volume is read-only"
'

step "migrate: applies the migrations as the owner role and exits 0 (R-016)"
# Run migrate and wait at most 120 s for it to exit; a migrate that serves never exits.
run_migrate() {
    compose up -d --no-deps --force-recreate migrate >/dev/null
    id=$(compose ps -aq migrate)
    tries=0
    while [ "$(docker inspect -f '{{.State.Status}}' "$id")" != exited ]; do
        tries=$((tries + 1))
        [ "$tries" -le 120 ] || { compose logs migrate; fail "migrate did not exit within 120 s"; }
        sleep 1
    done
    compose logs migrate
    code=$(docker inspect -f '{{.State.ExitCode}}' "$id")
    [ "$code" = 0 ] || fail "migrate exited $code"
}
run_migrate
run_migrate >/dev/null
echo "migrate exited 0 twice (a second run applies nothing)"

step "hub: starts as the runtime login and serves TLS only (SEC-001, DB-003)"
compose up -d --no-deps --wait --wait-timeout 120 guacd hub ||
    {
        compose logs --tail 20 hub
        docker inspect -f '{{range .State.Health.Log}}{{.ExitCode}} {{.Output}}{{end}}' "$(compose ps -aq hub)"
        fail "hub did not become healthy (anima-hub health)"
    }
compose logs --tail 10 hub
echo "hub is healthy: anima-hub health answered ok over TLS"
# From this host the published port reaches the hub through Docker's proxy, so the hub sees
# a 172.16.0.0/12 peer. It closes that connection at accept, before TLS: no HTTP status.
code=$(curl -sk -o /dev/null -w '%{http_code}' --max-time 10 -X POST \
    -H 'content-type: application/json' -d '{"query":"{__typename}"}' "https://127.0.0.1:443/graphql" || true)
[ "$code" = 000 ] || fail "a proxied request to /graphql got HTTP status $code, not a closed connection"
code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 "http://127.0.0.1:443/health" || true)
[ "$code" = 000 ] || fail "the published port answered plain HTTP with status $code"
echo "the published port closes proxied TLS and plain HTTP without an HTTP answer"
refusal=$(compose exec -T postgres psql -X -U postgres -d anima -Atc \
    "select message from events.event_log where source = 'access' order by at desc limit 1")
case "$refusal" in
    "Refused a connection from 172."*) echo "the refusal is logged (NET-002): $refusal" ;;
    *) fail "the newest access event is '$refusal', not a 172.16/12 refusal" ;;
esac
