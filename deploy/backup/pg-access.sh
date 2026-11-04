# Sourced by backup.sh, restore.sh and restore-check.sh. Reaches PostgreSQL and holds what they share.
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

superuser_sql() { pg_run psql -X -q -v ON_ERROR_STOP=1 -U "$ANIMA_DB_SUPERUSER" -d postgres "$@"; }

# SQL that prints one line per table, "schema.table count", partitions included.
# It never selects a column, so no row content can reach the output.
counts_query() {
    cat <<'SQL'
select t || ' ' || (xpath('/row/c/text()',
        query_to_xml(format('select count(*) as c from %s', t), false, true, '')))[1]::text
from (select format('%I.%I', schemaname, tablename) as t from pg_tables
      where schemaname not in ('pg_catalog', 'information_schema')) tables
order by 1;
SQL
}

# Exact row counts of a database, read as the owner.
count_rows() {
    counts_query | pg_run psql -X -q -At -v ON_ERROR_STOP=1 -U "$ANIMA_DB_OWNER" -d "$1"
}

# A new empty database with the same owner and grants as deploy/postgres/roles.sql.
# The statements are kept in step by hand: roles.sql also makes roles, so it cannot be included.
create_database() {
    superuser_sql -v db="$1" -v owner="$ANIMA_DB_OWNER" <<'SQL'
set client_min_messages = warning;
drop database if exists :"db" with (force);
create database :"db" owner :"owner";
revoke connect, temporary on database :"db" from public;
grant connect on database :"db" to anima_runtime;
SQL
}

# Drops a database. A failure is a warning, because it leaves a database behind.
drop_database() {
    superuser_sql -c "set client_min_messages = warning" -c "drop database if exists \"$1\" with (force)" ||
        printf '%s: warning: could not drop database %s; drop it by hand\n' "$(basename "$0")" "$1" >&2
}

# Fails unless pg_restore can read the table of contents of the dump.
require_readable_dump() {
    pg_run pg_restore --list <"$1" >/dev/null || fail "$1 is not a readable dump"
}

# Loads the dump into an existing empty database, in one transaction.
restore_into() {
    pg_run pg_restore --exit-on-error --single-transaction \
        -U "$ANIMA_DB_OWNER" -d "$1" <"$2" || fail "pg_restore failed"
}
