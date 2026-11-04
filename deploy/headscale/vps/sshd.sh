# shellcheck shell=bash
# The `ssh` command and the checks of the admin account and the effective sshd values
# (VPS-001). Sourced by harden.sh; needs its say, refuse, fail, dry, path, write_file,
# save_previous and require_root, and SSHD, SYSTEMCTL, GETENT, ID, SSHD_DROPIN, STATE_DIR,
# ADMIN_GROUP, USER_NAME_PATTERN, KEY_LINE_PATTERN and HERE.

# Print why a path would make sshd (StrictModes) ignore the keys, or nothing.
strict_modes_problem() {
  if [ -n "$(find "$1" -prune \( -perm -020 -o -perm -002 \) -print)" ]; then
    say "$1 is group- or world-writable; sshd would ignore the keys"
  elif [ -n "$(find "$1" -prune ! -user "$2" ! -user 0 -print)" ]; then
    say "$1 is not owned by the admin user or root; sshd would ignore the keys"
  fi
}

# Print why ANIMA_ADMIN_USER cannot be the only SSH login, or nothing when it can.
admin_problem() {
  local user=${ANIMA_ADMIN_USER:-} entry uid home keys problem
  [ -n "$user" ] || { say "set ANIMA_ADMIN_USER to the admin account"; return; }
  [[ $user =~ $USER_NAME_PATTERN ]] || { say "ANIMA_ADMIN_USER is not a valid user name"; return; }
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

# Print each effective sshd value (sshd -T) that differs from the drop-in.
sshd_mismatches() {
  local effective line
  effective=$($SSHD -T) || { say "sshd -T failed"; return; }
  while IFS= read -r line; do
    grep -qixF "$line" <<<"$effective" || say "sshd: expected \"$line\""
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
allowusers $1
EOF
  # AllowUsers adds up across drop-ins: any other user is a second way in.
  awk -v want="allowusers $1" 'tolower($1) == "allowusers" && tolower($0) != want {
    print "sshd: unexpected \"" $0 "\" (AllowUsers adds up across drop-ins)" }' <<<"$effective"
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
  if dry; then
    say "[dry-run] write $(path "$dropin") with AllowUsers $user, then $SSHD -t, $SSHD -T, reload sshd"
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
  say "SSH hardened. Keep this session open and test a new login as $user first."
}
