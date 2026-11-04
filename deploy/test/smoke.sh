#!/bin/sh
# Manual smoke test for deploy/compose.yaml (WP-04 D-9). Needs Docker. Not a gate.
#
# It works in a temporary copy of the deploy folder, so it never reads deploy/secrets.
# TEST-ONLY: the secret files are made readable by everyone (mode 0444), because the host
# owner (root:999) of a real install cannot be set without root. Never do this on the host.
# It contacts neither Let's Encrypt nor Cloudflare: the certs and hub services are not started.
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

step "Prepare a temporary copy of deploy/ in $work"
mkdir -p "$work/deploy" "$work/apps/client/src/styles"
cp -R "$repo/deploy/compose.yaml" "$repo/deploy/postgres" "$repo/deploy/certs" "$repo/deploy/gateway" "$work/deploy/"
cp -R "$repo/apps/client/src/styles/fonts" "$work/apps/client/src/styles/fonts"
ANIMA_SECRETS_CHOWN=0 sh "$repo/deploy/bin/make-secrets" "$work/deploy/secrets" </dev/null
printf '%s' 'smoke-test-not-a-token' >"$work/deploy/secrets/cloudflare_dns_token"
chmod 0444 "$work/deploy/secrets/"*

cd "$work/deploy"
export ANIMA_HUB_ADDRESS=127.0.0.1
export ANIMA_ACME_EMAIL=smoke@example.invalid
compose() { docker compose -p "$project" "$@"; }

step "docker compose config"
compose config --quiet
if (unset ANIMA_HUB_ADDRESS; compose config --quiet 2>/dev/null); then
    fail "compose config accepted a missing ANIMA_HUB_ADDRESS"
fi
echo "config is valid, and a missing ANIMA_HUB_ADDRESS is refused"

step "Build the gateway image and check the font (RA-011)"
compose build guacd
compose run --rm --no-deps --entrypoint sh guacd -c '
    test -f /usr/share/fonts/opendyslexic/OpenDyslexicMNerdFontMono-Regular.otf
    test -f /usr/share/fonts/opendyslexic/LICENSE-OpenDyslexic.txt
    if command -v fc-list >/dev/null 2>&1; then fc-list | grep -i opendyslexic; fi
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
