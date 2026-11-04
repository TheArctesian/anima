#!/usr/bin/env bash
# Daily Headscale backup to the rest-server on the home Linux PC (VPS-006, decision D-12).
#
# Run as root by headscale-backup.service. restic reads the repository (which holds the
# rest-server password) and the encryption password from files only; neither value is
# ever passed as an argument, put in the environment or printed. This script never runs
# `restic forget` or `restic prune`: the repository is append-only for the VPS and
# pruning runs on the home PC (prune.sh).
#
# Settings (environment, set by the unit):
#   HEADSCALE_DIR           Compose project folder (default /opt/anima/headscale)
#   RESTIC_REPOSITORY_FILE  default $CREDENTIALS_DIRECTORY/restic-repository
#   RESTIC_PASSWORD_FILE    default $CREDENTIALS_DIRECTORY/restic-password
#   CACHE_DIRECTORY         restic cache (systemd CacheDirectory=); none when unset
#   SQLITE3, RESTIC         command overrides for tests
set -euo pipefail

readonly SQLITE3="${SQLITE3:-sqlite3}"
readonly RESTIC="${RESTIC:-restic}"
readonly HEADSCALE_DIR="${HEADSCALE_DIR:-/opt/anima/headscale}"
readonly CREDENTIALS="${CREDENTIALS_DIRECTORY:-}"
readonly REPOSITORY_FILE="${RESTIC_REPOSITORY_FILE:-${CREDENTIALS:+$CREDENTIALS/restic-repository}}"
readonly PASSWORD_FILE="${RESTIC_PASSWORD_FILE:-${CREDENTIALS:+$CREDENTIALS/restic-password}}"
readonly LIB="$HEADSCALE_DIR/lib"
readonly CONFIG="$HEADSCALE_DIR/config"

# restic takes the repository and password from the flags below and nothing else.
unset RESTIC_REPOSITORY RESTIC_PASSWORD RESTIC_PASSWORD_COMMAND RESTIC_REPOSITORY_FILE RESTIC_PASSWORD_FILE

fail() {
  printf 'headscale-backup: %s\n' "$*" >&2
  exit 1
}

# Checks that a secret file is usable without ever reading its content into the shell.
require_secret_file() {
  local what="$1" path="$2"
  [[ -n "$path" ]] || fail "$what is not set (set it or run under systemd with LoadCredential=)"
  [[ -r "$path" && -s "$path" ]] || fail "$what is missing, empty or unreadable: $path"
}

require_secret_file "restic repository file" "$REPOSITORY_FILE"
require_secret_file "restic password file" "$PASSWORD_FILE"
[[ -r "$LIB/db.sqlite" ]] || fail "Headscale database not readable: $LIB/db.sqlite"

if [[ -n "${CACHE_DIRECTORY:-}" ]]; then
  cache=(--cache-dir "$CACHE_DIRECTORY")
else
  cache=(--no-cache)
fi

restic_run() {
  "$RESTIC" --repository-file "$REPOSITORY_FILE" --password-file "$PASSWORD_FILE" "${cache[@]}" "$@"
}

umask 077
staging="$(mktemp -d)" || fail "could not create a private temporary folder"
# The snapshot holds the Noise and DERP private keys: remove it however the script ends.
trap 'rm -rf -- "$staging"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
[[ "$staging" != *"'"* ]] || fail "temporary folder path contains a quote: $staging"

# VACUUM INTO takes a consistent copy of a live WAL-mode database.
"$SQLITE3" -batch -bail -readonly "$LIB/db.sqlite" "VACUUM INTO '$staging/db.sqlite'" \
  || fail "sqlite3 VACUUM INTO failed for $LIB/db.sqlite"
integrity="$("$SQLITE3" -batch -bail -readonly "$staging/db.sqlite" "PRAGMA integrity_check")" \
  || fail "sqlite3 integrity_check could not run on the snapshot"
[[ "$integrity" == "ok" ]] || fail "integrity_check of the database snapshot did not return ok"

for source in "$LIB/noise_private.key" "$LIB/derp_server_private.key" \
  "$CONFIG/config.yaml" "$CONFIG/policy.hujson"; do
  cp -- "$source" "$staging/" || fail "could not copy $source"
done

restic_run backup --host anima-vps --tag headscale "$staging" \
  || fail "restic backup failed (no snapshot is guaranteed)"
restic_run snapshots --latest 1 --host anima-vps --tag headscale \
  || fail "restic snapshots could not confirm the new snapshot"
printf 'headscale-backup: snapshot sent\n'
