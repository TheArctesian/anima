#!/usr/bin/env bash
# Weekly retention for the Headscale backups, run on the home Linux PC (VPS-006, D-12).
#
# Run as the `restic` user by restic-prune.service, directly against the local repository
# folder: the home PC has full access, the VPS only appends through rest-server.
# Snapshots are grouped by host and tag because each backup comes from a new temporary
# folder, so the default grouping by path would keep every snapshot.
#
# Settings (environment, set by the unit):
#   RESTIC_REPOSITORY_PATH  default /srv/restic/headscale-vps/headscale
#   RESTIC_PASSWORD_FILE    default $CREDENTIALS_DIRECTORY/restic-password
#   CACHE_DIRECTORY         restic cache (systemd CacheDirectory=); none when unset
#   RESTIC                  command override for tests
set -euo pipefail

readonly RESTIC="${RESTIC:-restic}"
readonly REPOSITORY="${RESTIC_REPOSITORY_PATH:-/srv/restic/headscale-vps/headscale}"
readonly CREDENTIALS="${CREDENTIALS_DIRECTORY:-}"
readonly PASSWORD_FILE="${RESTIC_PASSWORD_FILE:-${CREDENTIALS:+$CREDENTIALS/restic-password}}"

unset RESTIC_REPOSITORY RESTIC_PASSWORD RESTIC_PASSWORD_COMMAND RESTIC_REPOSITORY_FILE RESTIC_PASSWORD_FILE

fail() {
  printf 'restic-prune: %s\n' "$*" >&2
  exit 1
}

[[ -n "$PASSWORD_FILE" ]] || fail "restic password file is not set (set it or use LoadCredential=)"
[[ -r "$PASSWORD_FILE" && -s "$PASSWORD_FILE" ]] \
  || fail "restic password file is missing, empty or unreadable: $PASSWORD_FILE"
[[ -d "$REPOSITORY" ]] || fail "repository folder not found: $REPOSITORY"

if [[ -n "${CACHE_DIRECTORY:-}" ]]; then
  cache=(--cache-dir "$CACHE_DIRECTORY")
else
  cache=(--no-cache)
fi

restic_run() {
  "$RESTIC" --repo "$REPOSITORY" --password-file "$PASSWORD_FILE" "${cache[@]}" "$@"
}

restic_run forget --host anima-vps --tag headscale --group-by host,tags \
  --keep-daily 14 --keep-weekly 8 --keep-monthly 12 --prune \
  || fail "restic forget --prune failed"
restic_run check || fail "restic check failed after pruning"
printf 'restic-prune: retention applied\n'
