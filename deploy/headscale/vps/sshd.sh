# shellcheck shell=bash
# The `ssh` command and the checks of the admin account and the effective sshd values
# (VPS-001). Sourced by harden.sh; needs its say, refuse, fail, dry, path, write_file,
# save_previous and require_root, and ROOT, SSHD, SYSTEMCTL, GETENT, ID, SSHD_DROPIN, STATE_DIR,
# ADMIN_GROUP, USER_NAME_PATTERN, KEY_LINE_PATTERN and HERE.

# Print why a path would make sshd (StrictModes) ignore the keys, or nothing.
strict_modes_problem() {
  if [ -n "$(find "$1" -prune \( -perm -020 -o -perm -002 \) -print)" ]; then
    say "$1 is group- or world-writable; sshd would ignore the keys"
  elif [ -n "$(find "$1" -prune ! -user "$2" ! -user 0 -print)" ]; then
    say "$1 is not owned by the admin user or root; sshd would ignore the keys"
  fi
}

# VP-13: the hub's unprivileged user, allowed only from the hub container's address. The
# address is `hub_source` in nftables.conf and ANIMA_HUB_IP in deploy/compose.yaml; the drop-in
# (sshd-anima.conf) holds the same entry. No other user and no other host part is accepted.
PROBE_USER=anima-probe
PROBE_ALLOW=$PROBE_USER@172.30.0.10
# The admin's host part: the tailnet range, so the admin logs in only over the VPN (VP-13 F5).
ADMIN_HOSTS=100.64.0.0/10

# Print why ANIMA_ADMIN_USER cannot be the admin SSH login, or nothing when it can.
admin_problem() {
  local user=${ANIMA_ADMIN_USER:-} entry uid home keys problem
  [ -n "$user" ] || { say "set ANIMA_ADMIN_USER to the admin account"; return; }
  [[ $user =~ $USER_NAME_PATTERN ]] || { say "ANIMA_ADMIN_USER is not a valid user name"; return; }
  [ "$user" != "$PROBE_USER" ] || { say "$PROBE_USER is the hub's user (VP-13); use another admin account"; return; }
  entry=$($GETENT passwd "$user") || { say "user $user does not exist"; return; }
  IFS=: read -r _ _ uid _ _ home _ <<<"$entry"
  if [ "$user" = root ] || [ "$uid" = 0 ]; then say "$user is root; use a non-root $ADMIN_GROUP user"; return; fi
  case " $($ID -nG "$user") " in
    *" $ADMIN_GROUP "*) ;;
    *) say "$user is not in group $ADMIN_GROUP"; return ;;
  esac
  [[ $home == /* ]] || { say "$user has no absolute home directory"; return; }
  keys=$(path "$home/.ssh/authorized_keys")
  [ -f "$keys" ] || { say "$user has no ~/.ssh/authorized_keys"; return; }
  if ! awk -v pattern="$KEY_LINE_PATTERN" '/^[[:space:]]*(#|$)/ { next } $0 ~ pattern { found = 1 }
      END { exit !found }' "$keys"; then
    say "$user has no key in ~/.ssh/authorized_keys"; return
  fi
  for entry in "$(path "$home")" "$(path "$home/.ssh")" "$keys"; do
    problem=$(strict_modes_problem "$entry" "$uid")
    [ -z "$problem" ] || { say "$problem"; return; }
  done
}

SSHD_CONFIG=/etc/ssh/sshd_config
# Includes nest at most this deep; sshd itself stops at 16.
SSHD_INCLUDE_DEPTH=16
# At most this many files are read, so no include pattern can make the walk run long.
SSHD_FILE_LIMIT=256
SSHD_FILES_SEEN=$'\n'
SSHD_FILES_READ=0

# Print, for one sshd config file, "I<tab>PATTERN" for each pattern of a plain Include
# line and "P<tab>PROBLEM" for each Match line and each Include the walk cannot follow.
# Each line is read as sshd reads it: leading blanks dropped, the keyword ends at a blank
# or "=", may be quoted, and is not case-sensitive. Only `Include PATTERN...` with no
# quotes, "=" or backslash is followed; any other Include form is reported instead.
sshd_scan() {
  awk -v name="${1#"${ROOT%/}"}" '{
    line = $0
    sub(/^[ \t\r]+/, "", line)
    if (line == "" || line ~ /^#/) next
    word = line
    sub(/[ \t\r=].*/, "", word)
    rest = substr(line, length(word) + 1)
    keyword = tolower(word)
    gsub(/["\047]/, "", keyword)
    where = "sshd: " name ":" FNR ": "
    if (keyword == "match") { print "P\t" where "a Match block can override the hardening; remove it"; next }
    if (keyword != "include") next
    if (word ~ /["\047]/ || rest !~ /^[ \t]/ || rest ~ /["\047=\\]/) {
      print "P\t" where "an Include this check cannot follow; write it as: Include /path/pattern"
      next
    }
    n = split(rest, words, /[ \t\r]+/)
    for (i = 1; i <= n; i++) if (words[i] != "") print "I\t" words[i]
  }' "$1"
}

# Print FILE (a path under ROOT) and every file it includes, depth first, each once. A
# relative Include is relative to /etc/ssh, as sshd reads it.
sshd_files() {
  local file=$1 depth=$2 kind pattern included
  [ -f "$file" ] || return 0
  case $SSHD_FILES_SEEN in *$'\n'"$file"$'\n'*) return 0 ;; esac
  SSHD_FILES_SEEN+=$file$'\n'
  SSHD_FILES_READ=$((SSHD_FILES_READ + 1))
  if [ "$SSHD_FILES_READ" -gt "$SSHD_FILE_LIMIT" ]; then
    [ "$SSHD_FILES_READ" -gt $((SSHD_FILE_LIMIT + 1)) ] || say "sshd: more than $SSHD_FILE_LIMIT included files"
    return
  fi
  [ "$depth" -le "$SSHD_INCLUDE_DEPTH" ] || { say "sshd: includes nest deeper than $SSHD_INCLUDE_DEPTH at $file"; return; }
  say "$file"
  while IFS=$'\t' read -r kind pattern; do
    [ "$kind" = I ] || continue
    case $pattern in /*) ;; *) pattern=/etc/ssh/$pattern ;; esac
    # shellcheck disable=SC2086 # the pattern is a glob, as in sshd
    for included in "${ROOT%/}"$pattern; do sshd_files "$included" $((depth + 1)); done
  done < <(sshd_scan "$file")
}

# Print each Match line, in any form, in sshd_config and the files it includes, and each
# Include the walk cannot follow. A Match block overrides the global values for the
# connections it matches, so it can turn passwords or root logins back on past the
# drop-in; sshd -T without -C does not show it.
match_problems() {
  local file kind problem
  while IFS= read -r file; do
    case $file in sshd:*) say "$file"; continue ;; esac
    while IFS=$'\t' read -r kind problem; do
      if [ "$kind" = P ]; then say "$problem"; fi
    done < <(sshd_scan "$file")
  done < <(sshd_files "$(path "$SSHD_CONFIG")" 0)
}

# Print each value in the output of sshd -T (with the -C arguments given, if any) that
# differs from the drop-in for admin user $1.
sshd_effective_mismatches() {
  local effective line label="sshd -T${2:+ -C $2}" admin_allow="$1@$ADMIN_HOSTS"
  effective=$($SSHD -T ${2:+-C "$2"}) || { say "$label failed"; return; }
  while IFS= read -r line; do
    grep -qixF "$line" <<<"$effective" || say "$label: expected \"$line\""
  done <<EOF
passwordauthentication no
kbdinteractiveauthentication no
permitrootlogin no
permitemptypasswords no
pubkeyauthentication yes
authenticationmethods publickey
x11forwarding no
maxauthtries 3
logingracetime 30
allowusers $admin_allow
allowusers $PROBE_ALLOW
EOF
  # AllowUsers adds up across drop-ins: any other user, or the admin or the probe user from
  # another host, is a second way in.
  awk -v admin="allowusers $admin_allow" -v probe="allowusers $PROBE_ALLOW" -v label="$label" \
    'tolower($1) == "allowusers" && tolower($0) != admin && tolower($0) != probe {
    print label ": unexpected \"" $0 "\" (AllowUsers adds up across drop-ins)" }' <<<"$effective"
}

# Print each effective sshd value that differs from the drop-in: first the global values,
# then, once they are right, the values for root and the admin user from a public address,
# for the admin user from the VPN and for the probe user from the hub, which a Match block
# could change. sshd -T -C prints the AllowUsers entries but does not apply them: the admin's
# login from the public address is refused by its host part (ADMIN_HOSTS), and is still checked
# here for the values a Match block could weaken.
sshd_mismatches() {
  local user=$1 public=${VPS_PUBLIC_IPV4:-198.51.100.7} global connection
  global=$(sshd_effective_mismatches "$user")
  if [ -n "$global" ]; then say "$global"; return; fi
  [[ $user =~ $USER_NAME_PATTERN ]] || return 0
  [[ $public =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]] || { say "VPS_PUBLIC_IPV4 is not an IPv4 address"; return; }
  for connection in "user=root,host=x,addr=$public" "user=$user,host=x,addr=$public" \
    "user=$user,host=x,addr=100.64.0.2" "user=$PROBE_USER,host=x,addr=${PROBE_ALLOW#*@}"; do
    sshd_effective_mismatches "$user" "$connection"
  done
}

restore_dropin() {
  if [ -f "$(path "$2")" ]; then mv -f "$(path "$2")" "$(path "$1")"; else rm -f "$(path "$1")"; fi
}

cmd_ssh() {
  local problem user=${ANIMA_ADMIN_USER:-} dropin=$SSHD_DROPIN
  local previous=$STATE_DIR/00-anima.conf.previous mismatches
  require_root
  problem=$(admin_problem)
  [ -z "$problem" ] || refuse "$problem"
  problem=$(match_problems)
  [ -z "$problem" ] || refuse "$problem"
  if dry; then
    say "[dry-run] write $(path "$dropin") with AllowUsers $user@$ADMIN_HOSTS $PROBE_ALLOW, then $SSHD -t, $SSHD -T, reload sshd"
    return
  fi
  rm -f "$(path "$previous")"
  save_previous "$dropin" 00-anima.conf.previous
  sed "s/\${ANIMA_ADMIN_USER}/$user/" "$HERE/sshd-anima.conf" | write_file "$dropin" 0600
  if ! $SSHD -t; then restore_dropin "$dropin" "$previous"; fail "sshd -t rejected the drop-in; restored"; fi
  mismatches=$(sshd_mismatches "$user")
  if [ -n "$mismatches" ]; then
    say "$mismatches" >&2
    restore_dropin "$dropin" "$previous"
    fail "effective sshd values are wrong; drop-in restored"
  fi
  if ! $SYSTEMCTL is-active --quiet sshd.service; then
    say "sshd.service is not running; the drop-in applies when it starts"
  elif ! $SYSTEMCTL reload sshd.service; then
    restore_dropin "$dropin" "$previous"
    fail "systemctl reload sshd failed; drop-in restored"
  fi
  say "SSH hardened. Keep this session open and test a new login as $user over the VPN first."
}
