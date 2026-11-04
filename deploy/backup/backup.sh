#!/usr/bin/env bash
# Daily database backup (DB-007, VPS-006). Usage: backup.sh
# Writes $ANIMA_BACKUP_DIR/anima-<UTC time>.dump, checks it, prunes old dumps, prints the path.
set -euo pipefail
umask 077

# shellcheck source=pg-access.sh source-path=SCRIPTDIR
. "$(dirname "${BASH_SOURCE[0]}")/pg-access.sh"

backup_dir="${ANIMA_BACKUP_DIR:-/var/backups/anima}"
keep_days="${ANIMA_BACKUP_KEEP_DAYS:-14}"
case "$keep_days" in '' | *[!0-9]*) fail "ANIMA_BACKUP_KEEP_DAYS must be a whole number" ;; esac

mkdir -p "$backup_dir"
final="$backup_dir/anima-$(date -u +%Y%m%dT%H%M%SZ).dump"
partial="$final.partial"

# A failed run leaves no partial file; stale partial files of earlier failed runs go too.
cleanup() { rm -f "$partial"; }
trap cleanup EXIT
rm -f "$backup_dir"/anima-*.dump.partial

pg_run pg_dump -Fc -U "$ANIMA_DB_OWNER" -d "$ANIMA_DB_NAME" >"$partial" ||
    fail "pg_dump failed"
# The host may have no pg_restore, so the check runs through the same path as the dump.
pg_run pg_restore --list <"$partial" >/dev/null || fail "the dump is not readable"

chmod 600 "$partial"
mv "$partial" "$final"

# Only a good new dump gets here (set -e), so a failed run never prunes.
find "$backup_dir" -maxdepth 1 -type f -name 'anima-*.dump' ! -path "$final" \
    -mtime "+$keep_days" -delete

printf '%s\n' "$final"
