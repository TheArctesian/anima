#!/usr/bin/env bash
# Daily database backup (DB-007, VPS-006). Usage: backup.sh
# Writes $ANIMA_BACKUP_DIR/anima-<UTC time>.dump and anima-<UTC time>.dump.counts, prunes old
# ones by the time in their names, and prints the path of the dump.
# The files get mode 0600 from the umask. Do not remove it.
set -euo pipefail
umask 077

# shellcheck source=pg-access.sh source-path=SCRIPTDIR
. "$(dirname "${BASH_SOURCE[0]}")/pg-access.sh"

backup_dir="${ANIMA_BACKUP_DIR:-/var/backups/anima}"
keep_days="${ANIMA_BACKUP_KEEP_DAYS:-14}"
case "$keep_days" in '' | *[!0-9]*) fail "ANIMA_BACKUP_KEEP_DAYS must be a whole number" ;; esac

mkdir -p "$backup_dir"
stamp=$(date -u +%Y%m%dT%H%M%SZ)
final="$backup_dir/anima-$stamp.dump"
partial="$final.partial"
counts_partial="$final.counts.partial"
lock_folder="$backup_dir/.lock.d"
snapshot_pid=""
pipe_folder=""

# A failed run leaves no partial file and no lock; the snapshot session ends.
cleanup() {
    rm -f "$partial" "$counts_partial"
    if [ -n "$pipe_folder" ]; then rm -rf "$pipe_folder"; fi
    if [ -n "$snapshot_pid" ]; then
        kill "$snapshot_pid" 2>/dev/null || true
        wait "$snapshot_pid" 2>/dev/null || true
    fi
    rmdir "$lock_folder" 2>/dev/null || true
}

# Two runs at once would clash: flock if the host has it, else a lock folder.
if [ "${ANIMA_BACKUP_LOCK:-}" != folder ] && command -v flock >/dev/null; then
    exec 9>"$backup_dir/.lock"
    flock -n 9 || fail "another backup is running"
    trap cleanup EXIT
else
    mkdir "$lock_folder" 2>/dev/null || fail "another backup is running (or remove $lock_folder after a crash)"
    trap cleanup EXIT
fi
rm -f "$backup_dir"/anima-*.dump.partial "$backup_dir"/anima-*.counts.partial

# One session holds a snapshot open. The dump and the row counts both read it, so they agree
# even while the hub writes. Only table names and counts are read. Two named pipes carry the
# session, because macOS ships a bash without coproc.
pipe_folder=$(mktemp -d)
mkfifo "$pipe_folder/in" "$pipe_folder/out"
pg_run psql -X -q -At -v ON_ERROR_STOP=1 -U "$ANIMA_DB_OWNER" -d "$ANIMA_DB_NAME" \
    <"$pipe_folder/in" >"$pipe_folder/out" &
snapshot_pid=$!
exec 7>"$pipe_folder/in" 8<"$pipe_folder/out"
printf '%s\n' "begin isolation level repeatable read read only;" "select pg_export_snapshot();" >&7 ||
    fail "cannot open the database"
read -r snapshot_id <&8 || fail "cannot open a snapshot of $ANIMA_DB_NAME"

pg_run pg_dump -Fc --snapshot="$snapshot_id" -U "$ANIMA_DB_OWNER" -d "$ANIMA_DB_NAME" >"$partial" ||
    fail "pg_dump failed"
# The host may have no pg_restore, so the check runs through the same path as the dump.
require_readable_dump "$partial"

{ counts_query; printf '%s\n' "select '--end-of-counts--';"; } >&7
finished=0
while read -r line <&8; do
    if [ "$line" = "--end-of-counts--" ]; then finished=1; break; fi
    printf '%s\n' "$line" >>"$counts_partial"
done
[ "$finished" = 1 ] || fail "the row count query did not finish"
printf '%s\n' "commit;" '\q' >&7
wait "$snapshot_pid" || fail "the snapshot session failed"
snapshot_pid=""
[ -s "$counts_partial" ] || fail "the database has no tables"

# The dump is moved last, so a dump never lacks its counts.
mv "$counts_partial" "$final.counts"
mv "$partial" "$final"

# Only a good new backup gets here (set -e). Age comes from the time in the file name.
cutoff=$(date -u -d "$keep_days days ago" +%Y%m%dT%H%M%SZ 2>/dev/null ||
    date -u -v-"$keep_days"d +%Y%m%dT%H%M%SZ)
# Only files named exactly like ours are pruned: anima-<UTC time>.dump and its .counts.
for old in "$backup_dir"/anima-*; do
    name=$(basename "$old")
    [[ $name =~ ^anima-([0-9]{8}T[0-9]{6}Z)\.dump(\.counts)?$ ]] || continue
    old_stamp=${BASH_REMATCH[1]}
    if [[ $old_stamp < $cutoff && $old_stamp != "$stamp" ]]; then rm -f "$old"; fi
done

printf '%s\n' "$final"
