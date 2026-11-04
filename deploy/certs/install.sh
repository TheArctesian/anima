#!/bin/sh
# lego hook (DNS-002, SEC-001). Writes the key and the certificate chain as one PEM file,
# /certs/hub/tls.pem, which the hub reads for both its certificate and its key. The file is
# built under a temporary name in the same folder and renamed, so the hub never sees a
# partial file or a key that does not match its chain. A checksum of the last installed
# content is kept outside the hub's volume: root without capabilities cannot read the
# 0400 file it gave to the hub user, so it compares checksums. If the content is the same,
# nothing changes.
#
# lego sets LEGO_CERT_PATH (chain) and LEGO_CERT_KEY_PATH (key) for its hooks.
# ANIMA_CERTS_DIR: target folder (default /certs/hub).
# ANIMA_CERTS_STAMP: checksum file (default /var/lib/lego/tls.pem.sha256).
# ANIMA_CERTS_OWNER: owner of the file (default 10001:10001, the hub user); empty skips chown.
#
# Order matters under cap_drop ALL: root may chmod only files it owns, so chmod comes
# before chown and never after it.
set -eu
umask 077

cert=${LEGO_CERT_PATH:?install.sh: LEGO_CERT_PATH is not set}
key=${LEGO_CERT_KEY_PATH:?install.sh: LEGO_CERT_KEY_PATH is not set}
dest=${ANIMA_CERTS_DIR:-/certs/hub}
owner=${ANIMA_CERTS_OWNER-10001:10001}
stamp=${ANIMA_CERTS_STAMP:-/var/lib/lego/tls.pem.sha256}

mkdir -p "$dest"
chmod 0755 "$dest"

tmp=$(mktemp "$dest/.tls.pem.XXXXXX")
trap 'rm -f "$tmp"' EXIT
cat "$key" "$cert" >"$tmp"

sum=$(sha256sum <"$tmp")
if [ -f "$dest/tls.pem" ] && [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$sum" ]; then
    echo "install.sh: $dest/tls.pem is up to date"
    exit 0
fi

chmod 0400 "$tmp"
if [ -n "$owner" ]; then
    chown "$owner" "$tmp"
fi
mv "$tmp" "$dest/tls.pem"
trap - EXIT
printf '%s\n' "$sum" >"$stamp"
echo "install.sh: installed $dest/tls.pem"
