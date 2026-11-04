# shellcheck shell=bash
# The `selinux` command and the checks of the labels Headscale needs (VPS-004, D-9).
# Sourced by harden.sh; needs its say, refuse, fail, run, path and require_root, and
# GETENFORCE, SEMANAGE, RESTORECON, STAT, DOCKER, HEADSCALE_DIR and LABEL_SPEC.

# A recorded deviation from D-9 (R-050): while this file holds a reason, SELinux may stay
# disabled, so that turning it on (a relabel and reboots) does not stop the operator's
# other services. It is ignored, and reported, once SELinux is on.
SELINUX_DEVIATION=/etc/anima/selinux-disabled

# Print the recorded reason for keeping SELinux disabled, or nothing.
selinux_deviation() {
  local file
  file=$(path "$SELINUX_DEVIATION")
  [ -f "$file" ] || return 0
  sed -e 's/#.*//' -e '/^[[:space:]]*$/d' "$file" | head -n 1
}

cmd_selinux() {
  local mode dir
  require_root
  mode=$($GETENFORCE) || fail "getenforce failed"
  if [ "$mode" = Disabled ]; then
    [ -z "$(selinux_deviation)" ] || { say "SELinux is disabled, a recorded deviation ($SELINUX_DEVIATION); nothing to label."; return 0; }
    refuse "SELinux is disabled; turn it on (guide 2.0b), or record the deviation in $SELINUX_DEVIATION"
  fi
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

# Print a problem unless the Headscale container runs confined (container_t). A container
# keeps the label it was created with: after daemon.json turns labels on, Headscale stays
# spc_t until it is recreated.
headscale_label_problem() {
  local mode id label
  mode=$($GETENFORCE 2>/dev/null) || mode=unknown
  [ "$mode" != Disabled ] || return 0
  id=$($DOCKER ps -q --filter label=com.docker.compose.project=headscale \
    --filter label=com.docker.compose.service=headscale 2>/dev/null) || { say "docker ps failed"; return; }
  id=${id%%$'\n'*}
  [ -n "$id" ] || { say "the Headscale container is not running"; return; }
  label=$($DOCKER inspect -f '{{.ProcessLabel}}' "$id" 2>/dev/null) || label=
  case $label in
    *:container_t:*) ;;
    *) say "Headscale runs as ${label:-an unknown label}, not container_t; recreate it (docker compose up -d --force-recreate)" ;;
  esac
}

selinux_problems() {
  local mode
  mode=$($GETENFORCE 2>/dev/null) || mode=unknown
  if [ -n "$(selinux_deviation)" ]; then
    [ "$mode" = Disabled ] && return 0
    say "$SELINUX_DEVIATION records SELinux as disabled, but it is $mode; remove the file"
  fi
  [ "$mode" = Enforcing ] || say "SELinux is $mode, not Enforcing"
  label_problems
  case $($DOCKER info --format '{{.SecurityOptions}}' 2>/dev/null) in
    *name=selinux*) ;;
    *) say "Docker runs containers without SELinux labels (daemon.json selinux-enabled)" ;;
  esac
  headscale_label_problem
}
