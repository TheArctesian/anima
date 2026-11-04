# Sourced by backup.sh and restore.sh. Defines how to reach PostgreSQL and the shared names.
# shellcheck shell=bash

ANIMA_DB_NAME="${ANIMA_DB_NAME:-anima}"
ANIMA_DB_OWNER="${ANIMA_DB_OWNER:-anima_owner}"
ANIMA_DB_SUPERUSER="${ANIMA_DB_SUPERUSER:-postgres}"
ANIMA_COMPOSE_FILE="${ANIMA_COMPOSE_FILE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/compose.yaml}"
export ANIMA_DB_NAME ANIMA_DB_OWNER ANIMA_DB_SUPERUSER

# Runs a PostgreSQL client command. Standard input and output pass through.
# Docker mode: inside the postgres container, over its Unix socket (no password).
# Local mode (ANIMA_PG_LOCAL=1): the local binary, with the libpq variables (PGHOST ...).
pg_run() {
    if [ "${ANIMA_PG_LOCAL:-0}" = 1 ]; then
        "$@"
    else
        docker compose -f "$ANIMA_COMPOSE_FILE" exec -T postgres "$@"
    fi
}

fail() {
    printf '%s: %s\n' "$(basename "$0")" "$*" >&2
    exit 1
}
