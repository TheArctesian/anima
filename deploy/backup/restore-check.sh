#!/usr/bin/env bash
# Restore drill (DB-007). Usage: restore-check.sh <dump>
# Restores the dump into anima_restore_check, compares its row counts with the manifest
# <dump>.counts that backup.sh wrote from the same snapshot, checks the privileges of
# anima_runtime on the logs, and drops the check database. It never reads the live database,
# so it can run at any time. It prints table names and counts, never row contents.
set -euo pipefail
umask 077

# shellcheck source=pg-access.sh source-path=SCRIPTDIR
. "$(dirname "${BASH_SOURCE[0]}")/pg-access.sh"

check_db=anima_restore_check
usage="usage: restore-check.sh <dump>"
dump=""
for argument in "$@"; do
    case "$argument" in
        -*) fail "unknown option $argument ($usage)" ;;
        *) [ -z "$dump" ] || fail "give one dump only ($usage)"; dump="$argument" ;;
    esac
done
[ -n "$dump" ] || fail "$usage"
[ -f "$dump" ] || fail "no such file: $dump"
manifest="$dump.counts"
[ -f "$manifest" ] || fail "the manifest $manifest is missing; the drill cannot compare without it"

require_readable_dump "$dump"
trap 'drop_database "$check_db"' EXIT
create_database "$check_db"
restore_into "$check_db" "$dump"

status=0
restored_counts=$(count_rows "$check_db")
if [ -z "$restored_counts" ]; then
    echo "The restored copy has no tables." >&2
    status=1
elif [ "$restored_counts" != "$(cat "$manifest")" ]; then
    echo "Row counts differ (< manifest, > restored copy):" >&2
    diff "$manifest" <(echo "$restored_counts") >&2 || true
    status=1
fi

# The runtime group may read and append the logs, and nothing else (LOG-006).
bad_privileges=$(pg_run psql -X -q -At -v ON_ERROR_STOP=1 -U "$ANIMA_DB_OWNER" -d "$check_db" <<'SQL'
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
    echo "Restore check passed: $(echo "$restored_counts" | wc -l | tr -d ' ') tables match the manifest."
fi
exit "$status"
