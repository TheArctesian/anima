#!/bin/sh
# lego hook (DNS-002, SEC-001). Copies the certificate chain and the key where the hub reads
# them: /certs/hub/fullchain.pem and /certs/hub/key.pem. Each file is copied to a temporary
# name in the same folder and then renamed, so the hub never sees a partial file.
#
# lego sets LEGO_CERT_PATH (chain) and LEGO_CERT_KEY_PATH (key) for its hooks.
# ANIMA_CERTS_DIR: target folder (default /certs/hub).
# ANIMA_CERTS_OWNER: owner of the files (default 10001:10001, the hub user); empty skips chown.
set -eu
umask 077

cert=${LEGO_CERT_PATH:?install.sh: LEGO_CERT_PATH is not set}
key=${LEGO_CERT_KEY_PATH:?install.sh: LEGO_CERT_KEY_PATH is not set}
dest=${ANIMA_CERTS_DIR:-/certs/hub}
owner=${ANIMA_CERTS_OWNER-10001:10001}

mkdir -p "$dest"
chmod 0755 "$dest"

# install_file SOURCE NAME MODE
install_file() {
    tmp=$(mktemp "$dest/.$2.XXXXXX")
    trap 'rm -f "$tmp"' EXIT
    cp "$1" "$tmp"
    if [ -n "$owner" ]; then
        chown "$owner" "$tmp"
    fi
    chmod "$3" "$tmp"
    mv "$tmp" "$dest/$2"
    trap - EXIT
}

install_file "$key" key.pem 0400
install_file "$cert" fullchain.pem 0444
echo "install.sh: installed the certificate for $dest"
