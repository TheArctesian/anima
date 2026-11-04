#!/usr/bin/env bash
# Restore a dump (DB-007). Usage: restore.sh --yes <dump>   or   restore.sh --check <dump>
#   --yes    replace the database $ANIMA_DB_NAME. The hub and migrate services must be stopped.
#   --check  restore into anima_restore_check, compare it with the live database, drop it.
#            Safe while the hub runs. Prints table names and counts, never row contents.
set -euo pipefail
umask 077

# shellcheck source=pg-access.sh source-path=SCRIPTDIR
. "$(dirname "${BASH_SOURCE[0]}")/pg-access.sh"

check_db=anima_restore_check
mode=""
dump=""
for argument in "$@"; do
    case "$argument" in
        --yes) mode=restore ;;
        --check) mode=check ;;
        -*) fail "unknown option $argument" ;;
        *) dump="$argument" ;;
    esac
done
[ -n "$mode" ] || fail "usage: restore.sh (--yes | --check) <dump>"
[ -f "$dump" ] || fail "usage: restore.sh (--yes | --check) <dump>: no such file"

superuser_sql() { pg_run psql -X -q -v ON_ERROR_STOP=1 -U "$ANIMA_DB_SUPERUSER" -d postgres "$@"; }
owner_sql() { pg_run psql -X -q -At -v ON_ERROR_STOP=1 -U "$ANIMA_DB_OWNER" -d "$1"; }

# Same grants as deploy/postgres/roles.sql (kept in step by hand: roles.sql also makes roles).
create_database() {
    superuser_sql -v db="$1" -v owner="$ANIMA_DB_OWNER" <<'SQL'
set client_min_messages = warning;
drop database if exists :"db" with (force);
create database :"db" owner :"owner";
revoke connect, temporary on database :"db" from public;
grant connect on database :"db" to anima_runtime;
SQL
}

restore_into() {
    pg_run pg_restore --exit-on-error --single-transaction \
        -U "$ANIMA_DB_OWNER" -d "$1" <"$dump" || fail "pg_restore failed"
}

if [ "$mode" = restore ]; then
    if [ "${ANIMA_PG_LOCAL:-0}" != 1 ]; then
        running=$(docker compose -f "$ANIMA_COMPOSE_FILE" ps --status running --services hub migrate) ||
            fail "cannot list services"
        [ -z "$running" ] || fail "stop these services first: $(echo "$running" | tr '\n' ' ')"
    fi
    create_database "$ANIMA_DB_NAME"
    restore_into "$ANIMA_DB_NAME"
    printf 'Restored %s into %s.\nNext: docker compose up -d migrate, then docker compose up -d hub.\n' \
        "$dump" "$ANIMA_DB_NAME"
    exit 0
fi

# --check
# shellcheck disable=SC2329 # called by the EXIT trap
drop_check_database() {
    superuser_sql -c "set client_min_messages = warning" \
        -c "drop database if exists $check_db with (force)" || true
}
trap drop_check_database EXIT
create_database "$check_db"
restore_into "$check_db"

# One line per table: "schema.table count". Partitions are listed too.
count_rows() {
    owner_sql "$1" <<'SQL'
select t || ' ' || (xpath('/row/c/text()',
        query_to_xml(format('select count(*) as c from %s', t), false, true, '')))[1]::text
from (select format('%I.%I', schemaname, tablename) as t from pg_tables
      where schemaname not in ('pg_catalog', 'information_schema')) tables
order by 1;
SQL
}

status=0
source_counts=$(count_rows "$ANIMA_DB_NAME")
restored_counts=$(count_rows "$check_db")
if [ "$source_counts" != "$restored_counts" ]; then
    echo "Row counts differ (< live database, > restored copy):" >&2
    diff <(echo "$source_counts") <(echo "$restored_counts") >&2 || true
    status=1
fi
[ -n "$restored_counts" ] || { echo "The restored copy has no tables." >&2; status=1; }

# The runtime group may read and append the logs, and nothing else (LOG-006).
bad_privileges=$(owner_sql "$check_db" <<'SQL'
select format('%I.%I', schemaname, tablename)
from pg_tables
where schemaname = 'events' and tablename ~ '^(audit_log|event_log)(_|$)'
  and (has_table_privilege('anima_runtime', format('%I.%I', schemaname, tablename), 'update')
    or has_table_privilege('anima_runtime', format('%I.%I', schemaname, tablename), 'delete')
    or has_table_privilege('anima_runtime', format('%I.%I', schemaname, tablename), 'truncate')
    or not has_table_privilege('anima_runtime', format('%I.%I', schemaname, tablename), 'select')
    or not has_table_privilege('anima_runtime', format('%I.%I', schemaname, tablename), 'insert'))
order by 1;
SQL
)
if [ -n "$bad_privileges" ]; then
    echo "Wrong privileges for anima_runtime on:" >&2
    echo "$bad_privileges" >&2
    status=1
fi

if [ "$status" = 0 ]; then
    echo "Restore check passed: $(echo "$restored_counts" | wc -l | tr -d ' ') tables match."
fi
exit "$status"
