# shellcheck shell=sh
# The fake sshd for the harden.sh tests (D-14). `sshd -T [-C user=U,host=H,addr=A]` reads
# $FAKE_ROOT/etc/ssh/sshd_config and the files it includes, as sshd does: in the global
# section the first value wins and AllowUsers adds up; without -C Match blocks are skipped;
# with -C each Match block that matches (All, User, Address; other criteria count as
# matching) overrides the global values, the first matching value winning. FAKE_SSHD_C_CONF
# holds lines that only -C sees, as if from a source the Match scan cannot read.
# Any other call (sshd -t) exits $FAKE_SSHD_SYNTAX.
[ "$1" = -T ] || exit "${FAKE_SSHD_SYNTAX:-0}"
connection=
[ "${2:-}" != -C ] || connection=$3
includes=0

# Print the lines of standard input with each Include replaced by the files it names.
expand() {
  while IFS= read -r line || [ -n "$line" ]; do
    set -f
    # shellcheck disable=SC2086 # split the line into words
    set -- $line
    set +f
    if [ "$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')" = include ]; then
      shift
      for pattern; do
        case $pattern in /*) ;; *) pattern=/etc/ssh/$pattern ;; esac
        # shellcheck disable=SC2086 # the pattern is a glob
        for file in "$FAKE_ROOT"$pattern; do
          includes=$((includes + 1))
          if [ "$includes" -le 32 ] && [ -f "$file" ]; then expand <"$file"; fi
        done
      done
    else
      printf '%s\n' "$line"
    fi
  done
}

{
  expand <"$FAKE_ROOT/etc/ssh/sshd_config"
  if [ -n "$connection" ] && [ -n "${FAKE_SSHD_C_CONF:-}" ]; then printf 'Match All\n%b\n' "$FAKE_SSHD_C_CONF"; fi
} | awk -v connection="$connection" '
function listed(list, value,   n, p, i, re) {
  n = split(list, p, ",")
  for (i = 1; i <= n; i++) {
    re = p[i]; gsub(/\./, "\\.", re); gsub(/\*/, ".*", re); gsub(/\?/, ".", re)
    if (value ~ ("^" re "$")) return 1
  }
  return 0
}
function block_matches(   i, k, ok) {
  if (connection == "") return 0
  ok = 1
  for (i = 2; i <= NF; i++) {
    k = tolower($i)
    if (k == "all") continue
    if (k == "user") ok = ok && listed($(i + 1), c["user"])
    else if (k == "address") ok = ok && listed($(i + 1), c["addr"])
    i++
  }
  return ok
}
BEGIN {
  n = split("passwordauthentication yes kbdinteractiveauthentication yes permitrootlogin prohibit-password permitemptypasswords no pubkeyauthentication yes authenticationmethods any x11forwarding no maxauthtries 6 logingracetime 120", d, " ")
  for (i = 1; i < n; i += 2) dflt[d[i]] = d[i + 1]
  n = split(connection, parts, ",")
  for (i = 1; i <= n; i++) { split(parts[i], kv, "="); c[kv[1]] = kv[2] }
}
/^[[:space:]]*(#|$)/ { next }
tolower($1) == "match" { inmatch = 1; active = block_matches(); next }
inmatch && !active { next }
{
  k = tolower($1)
  if (k == "allowusers") {
    for (i = 2; i <= NF; i++) if (inmatch) match_users[++nm] = $i; else users[++nu] = $i
    next
  }
  if (inmatch) { if (!(k in matched)) matched[k] = $2 } else if (!(k in global)) global[k] = $2
}
END {
  for (k in dflt) print k " " ((k in matched) ? matched[k] : (k in global) ? global[k] : dflt[k])
  if (nm) for (i = 1; i <= nm; i++) print "allowusers " match_users[i]
  else for (i = 1; i <= nu; i++) print "allowusers " users[i]
}'
