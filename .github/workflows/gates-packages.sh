#!/bin/sh
# Installs what a job of gates.yml needs on an Ubuntu machine of GitHub. Each argument names
# one set: shell (the packages of the Tauri shell), postgres (PostgreSQL 17, from the
# PostgreSQL project's own package source), display (a virtual display), shellcheck.
set -eu

packages=""
for set in "$@"; do
  case "$set" in
    shell)
      packages="$packages build-essential curl file pkg-config libssl-dev libxdo-dev \
        libwebkit2gtk-4.1-dev libsoup-3.0-dev librsvg2-dev libdbus-1-dev libayatana-appindicator3-dev"
      ;;
    postgres) packages="$packages postgresql-17" ;;
    display) packages="$packages xvfb" ;;
    shellcheck) packages="$packages shellcheck" ;;
    *)
      echo "gates-packages: no such set: $set" >&2
      exit 2
      ;;
  esac
done

case " $* " in
  *" postgres "*)
    sudo apt-get update
    sudo apt-get install -y --no-install-recommends postgresql-common ca-certificates
    sudo /usr/share/postgresql-common/pgdg/apt.postgresql.org.sh -y
    ;;
esac
sudo apt-get update
# shellcheck disable=SC2086 # the list is words, one for each package
sudo apt-get install -y --no-install-recommends $packages
