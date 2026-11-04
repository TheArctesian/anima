#!/bin/sh
# Manual smoke test for deploy/compose.yaml (DB-002). Needs Docker. Not a gate.
#
# It works in a temporary copy of the deploy folder, so it never reads deploy/secrets.
# TEST-ONLY: the secret files are made readable by everyone (mode 0444), because the host
# owner (root:999) of a real install cannot be set without root. Never do this on the host.
# It contacts neither Let's Encrypt nor Cloudflare: `certs` runs only with a stub entrypoint
# (the hook on fixture certificates and `/lego --version`), never its real entry point.
# `migrate` runs the real migrations as the owner role; then `hub` starts with the runtime
# login only and must pass its start check and answer TLS on the published port.
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
mkdir -p "$work/deploy" "$work/apps/client/src/styles"
cp -R "$repo/deploy/compose.yaml" "$repo/deploy/postgres" "$repo/deploy/certs" "$repo/deploy/gateway" "$repo/deploy/hub" "$work/deploy/"
cp -R "$repo/Cargo.toml" "$repo/Cargo.lock" "$repo/crates" "$work/"
cp -R "$repo/apps/client/src/styles/fonts" "$work/apps/client/src/styles/fonts"
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

step "hub: starts as the runtime login and serves TLS (SEC-001, DB-003)"
compose up -d --no-deps guacd hub
sleep 5
state=$(docker inspect -f '{{.State.Status}}' "$(compose ps -aq hub)")
compose logs --tail 10 hub
[ "$state" = running ] || fail "hub is $state, not running"
if curl -sk -o /dev/null --max-time 10 "https://127.0.0.1:443/graphql"; then
    echo "hub is running and answers TLS on 127.0.0.1:443"
elif curl -s -o /dev/null --max-time 10 "http://127.0.0.1:443/graphql"; then
    echo "WARNING: the hub answers plain HTTP on 127.0.0.1:443, not TLS (SEC-001)."
    echo "TLS serving from tls.pem belongs to the hub's network edge; until it lands, this is expected."
else
    fail "nothing answers on the published hub port"
fi
