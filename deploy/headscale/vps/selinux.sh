# shellcheck shell=bash
# The `selinux` command and the checks of the labels Headscale needs (VPS-004, D-9).
# Sourced by harden.sh; needs its say, refuse, fail, run, path and require_root, and
# GETENFORCE, SEMANAGE, RESTORECON, STAT, DOCKER, HEADSCALE_DIR and LABEL_SPEC.

cmd_selinux() {
  local mode dir
  require_root
  mode=$($GETENFORCE) || fail "getenforce failed"
  [ "$mode" != Disabled ] || refuse "SELinux is disabled; set SELINUX=enforcing in /etc/selinux/config and reboot"
  [ "$mode" = Enforcing ] || say "warning: SELinux is $mode; set it to Enforcing (setenforce 1 and /etc/selinux/config)"
  for dir in config lib; do
    [ -d "$(path "$HEADSCALE_DIR/$dir")" ] || refuse "$HEADSCALE_DIR/$dir is missing; create it first (guide step 2.5)"
  done
  if $SEMANAGE fcontext -l -C | awk -v spec="$LABEL_SPEC" '$1 == spec { found = 1 } END { exit !found }'; then
    run "$SEMANAGE" fcontext -m -t container_file_t "$LABEL_SPEC"
  else
    run "$SEMANAGE" fcontext -a -t container_file_t "$LABEL_SPEC"
  fi
  run "$RESTORECON" -R "$(path "$HEADSCALE_DIR")"
  say "Labelled $HEADSCALE_DIR/config and lib container_file_t."
}

# Print each problem with the SELinux labels Headscale's volumes need, or nothing.
label_problems() {
  local mode dir label
  mode=$($GETENFORCE 2>/dev/null) || mode=unknown
  [ "$mode" != Disabled ] || return 0
  for dir in config lib; do
    label=$($STAT -c %C "$(path "$HEADSCALE_DIR/$dir")" 2>/dev/null) || { say "$HEADSCALE_DIR/$dir is missing"; continue; }
    case $label in
      *:container_file_t:*) ;;
      *) say "$HEADSCALE_DIR/$dir is labelled $label, not container_file_t" ;;
    esac
  done
}

selinux_problems() {
  local mode
  mode=$($GETENFORCE 2>/dev/null) || mode=unknown
  [ "$mode" = Enforcing ] || say "SELinux is $mode, not Enforcing"
  label_problems
  case $($DOCKER info --format '{{.SecurityOptions}}' 2>/dev/null) in
    *name=selinux*) ;;
    *) say "Docker runs containers without SELinux labels (daemon.json selinux-enabled)" ;;
  esac
}
