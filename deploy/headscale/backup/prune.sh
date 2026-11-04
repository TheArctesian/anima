#!/usr/bin/env bash
# Weekly retention for the Headscale backups, run on the home Linux PC (VPS-006, D-12).
#
# Run as the `restic` user by restic-prune.service, directly against the local repository
# folder: the home PC has full access, the VPS only appends through rest-server.
#
# The VPS chooses its snapshot times, so a compromised VPS could append junk snapshots
# dated in the future, or many per day, to push every real snapshot out of a count-based
# policy. Two defences:
#   - retention is by age (--keep-within*), never by count;
#   - before forgetting anything, refuse when a snapshot is dated more than an hour in
#     the future or a UTC day has more than two snapshots. The operator then inspects
#     the list by hand.
# Snapshots are grouped by host and tag because each backup comes from a new temporary
# folder, so the default grouping by path would keep every snapshot.
#
# Settings (environment, set by the unit):
#   RESTIC_REPOSITORY_PATH  default /srv/restic/headscale-vps/headscale
#   RESTIC_PASSWORD_FILE    default $CREDENTIALS_DIRECTORY/restic-password
#   CACHE_DIRECTORY         restic cache (systemd CacheDirectory=); none when unset
#   PRUNE_NOW               current time in Unix seconds (tests); default `date -u +%s`
#   RESTIC, JQ              command overrides for tests
set -euo pipefail

readonly RESTIC="${RESTIC:-restic}"
readonly JQ="${JQ:-jq}"
readonly REPOSITORY="${RESTIC_REPOSITORY_PATH:-/srv/restic/headscale-vps/headscale}"
readonly CREDENTIALS="${CREDENTIALS_DIRECTORY:-}"
readonly PASSWORD_FILE="${RESTIC_PASSWORD_FILE:-${CREDENTIALS:+$CREDENTIALS/restic-password}}"
readonly FUTURE_SLACK_SECONDS=3600
readonly MAX_SNAPSHOTS_PER_DAY=2

unset RESTIC_REPOSITORY RESTIC_PASSWORD RESTIC_PASSWORD_COMMAND RESTIC_REPOSITORY_FILE RESTIC_PASSWORD_FILE

# Reads `restic snapshots --json`; prints "<snapshots after $latest> <most snapshots on one UTC day>".
# A time it cannot parse is an error, so an odd list is refused rather than half-checked.
# shellcheck disable=SC2016 # a jq program: $times and $latest are jq variables, not shell.
readonly GUARD='
def epoch:
  (capture("^(?<d>[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2})(\\.[0-9]+)?(?<z>Z|[+-][0-9]{2}:[0-9]{2})$")
    // error("unparsable snapshot time: \(.)"))
  | ((.d + "Z") | fromdateiso8601)
    - (if .z == "Z" then 0
       else ((.z[1:3] | tonumber) * 3600 + (.z[4:6] | tonumber) * 60)
            * (if .z[0:1] == "-" then -1 else 1 end)
       end);
if type != "array" then error("snapshot list is not an array") else . end
| [.[] | .time | epoch] as $times
| "\([$times[] | select(. > $latest)] | length) \([$times[] | . / 86400 | floor] | group_by(.) | map(length) | max // 0)"
'

fail() {
  printf 'restic-prune: %s\n' "$*" >&2
  exit 1
}

[[ -n "$PASSWORD_FILE" ]] || fail "restic password file is not set (set it or use LoadCredential=)"
[[ -r "$PASSWORD_FILE" && -s "$PASSWORD_FILE" ]] \
  || fail "restic password file is missing, empty or unreadable: $PASSWORD_FILE"
[[ -d "$REPOSITORY" ]] || fail "repository folder not found: $REPOSITORY"
command -v "$JQ" > /dev/null || fail "jq is required to check the snapshot list before pruning"
now="${PRUNE_NOW:-$(date -u +%s)}"
[[ "$now" =~ ^[0-9]+$ ]] || fail "current time is not a Unix timestamp: $now"

if [[ -n "${CACHE_DIRECTORY:-}" ]]; then
  cache=(--cache-dir "$CACHE_DIRECTORY")
else
  cache=(--no-cache)
fi

restic_run() {
  "$RESTIC" --repo "$REPOSITORY" --password-file "$PASSWORD_FILE" "${cache[@]}" "$@"
}

verdict="$(restic_run snapshots --json --host anima-vps --tag headscale \
  | "$JQ" -r --argjson latest "$((now + FUTURE_SLACK_SECONDS))" "$GUARD")" \
  || fail "could not read and check the snapshot list; nothing was pruned"
read -r future busiest <<< "$verdict"
[[ "$future" =~ ^[0-9]+$ && "$busiest" =~ ^[0-9]+$ ]] \
  || fail "unexpected snapshot check result; nothing was pruned"
((future == 0)) \
  || fail "refusing to prune: $future snapshot(s) dated more than an hour in the future"
((busiest <= MAX_SNAPSHOTS_PER_DAY)) \
  || fail "refusing to prune: $busiest snapshots on one UTC day (more than $MAX_SNAPSHOTS_PER_DAY)"

restic_run forget --host anima-vps --tag headscale --group-by host,tags \
  --keep-within 14d --keep-within-weekly 2m --keep-within-monthly 1y --prune \
  || fail "restic forget --prune failed"
restic_run check || fail "restic check failed after pruning"
printf 'restic-prune: retention applied\n'
