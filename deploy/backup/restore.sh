#!/usr/bin/env bash
# Restore a dump (DB-007). Usage: restore.sh --yes <dump>
# Loads the dump into a new database, then swaps it in for $ANIMA_DB_NAME. The old database
# stays under another name. The hub and migrate services must be stopped.
set -euo pipefail
umask 077

# shellcheck source=pg-access.sh source-path=SCRIPTDIR
. "$(dirname "${BASH_SOURCE[0]}")/pg-access.sh"

usage="usage: restore.sh --yes <dump>"
confirmed=0
dump=""
for argument in "$@"; do
    case "$argument" in
        --yes) confirmed=1 ;;
        -*) fail "unknown option $argument ($usage; use restore-check.sh for a drill)" ;;
        *) [ -z "$dump" ] || fail "give one dump only ($usage)"; dump="$argument" ;;
    esac
done
[ -n "$dump" ] || fail "$usage"
[ -f "$dump" ] || fail "no such file: $dump"
[ "$confirmed" = 1 ] || fail "this replaces the database $ANIMA_DB_NAME; add --yes ($usage)"

stamp=$(date -u +%Y%m%dT%H%M%SZ)
new_db="${ANIMA_DB_NAME}_restoring_$stamp"
previous_db="${ANIMA_DB_NAME}_previous_$stamp"
swapped=0

require_services_stopped() {
    [ "${ANIMA_PG_LOCAL:-0}" != 1 ] || return 0
    local running
    running=$(docker compose -f "$ANIMA_COMPOSE_FILE" ps --status running --status restarting --services hub migrate) ||
        fail "cannot list services"
    [ -z "$running" ] || fail "stop these services first: $(echo "$running" | tr '\n' ' ')"
}

# The new database goes unless it was swapped in; a failed run leaves the live database alone.
# shellcheck disable=SC2329 # called by the EXIT trap
discard_new_database() { [ "$swapped" = 1 ] || drop_database "$new_db"; }

require_readable_dump "$dump"
require_services_stopped
trap discard_new_database EXIT
create_database "$new_db"
restore_into "$new_db" "$dump"

# The restore took time: check again that nothing started meanwhile.
require_services_stopped
exists=$(superuser_sql -At -c "select count(*) from pg_database where datname = '$ANIMA_DB_NAME'")
{
    echo "set client_min_messages = warning;"
    if [ "$exists" != 0 ]; then echo "alter database \"$ANIMA_DB_NAME\" rename to \"$previous_db\";"; fi
    echo "alter database \"$new_db\" rename to \"$ANIMA_DB_NAME\";"
} | superuser_sql --single-transaction || fail "cannot swap the databases (is something connected to $ANIMA_DB_NAME?)"
swapped=1

echo "Restored $dump into $ANIMA_DB_NAME."
if [ "$exists" != 0 ]; then
    echo "The old database is kept as $previous_db. When you are sure, drop it:"
    echo "  docker compose exec postgres psql -U $ANIMA_DB_SUPERUSER -d postgres -c 'drop database \"$previous_db\"'"
fi
echo "Next: docker compose up -d migrate, wait for it to exit 0, then docker compose up -d hub."
