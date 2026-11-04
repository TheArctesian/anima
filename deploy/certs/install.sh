#!/bin/sh
# lego hook (DNS-002, SEC-001). Writes the key and the certificate chain as one PEM file,
# /certs/hub/tls.pem, which the hub reads for both its certificate and its key. The file is
# built under a temporary name in the same folder and renamed, so the hub never sees a
# partial file or a key that does not match its chain. Root without capabilities cannot
# read the 0400 file it gave to the hub user, so a stamp outside the hub's volume holds the
# checksum of the installed content and the file's inode, size, modification time, owner
# and mode. If the new content has the same checksum and the installed file still matches
# the stamp (so it was not emptied, replaced or touched), nothing changes.
#
# lego sets LEGO_CERT_PATH (chain) and LEGO_CERT_KEY_PATH (key) for its hooks.
# ANIMA_CERTS_DIR: target folder (default /certs/hub).
# ANIMA_CERTS_STAMP: stamp file (default /var/lib/lego/tls.pem.sha256).
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

# file_state FILE: inode, size, mtime, owner and mode (GNU stat, else BSD stat for tests).
file_state() {
    stat -c '%i %s %Y %u %a' "$1" 2>/dev/null || stat -f '%i %z %m %u %Lp' "$1"
}

# stamp_text SUM: the two stamp lines for the installed file.
stamp_text() {
    printf '%s\n%s' "$1" "$(file_state "$dest/tls.pem")"
}

tmp=$(mktemp "$dest/.tls.pem.XXXXXX")
trap 'rm -f "$tmp"' EXIT
cat "$key" "$cert" >"$tmp"

sum=$(sha256sum <"$tmp" 2>/dev/null || shasum -a 256 <"$tmp")
if [ -f "$dest/tls.pem" ] && [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$(stamp_text "$sum")" ]; then
    echo "install.sh: $dest/tls.pem is up to date"
    exit 0
fi

chmod 0400 "$tmp"
if [ -n "$owner" ]; then
    chown "$owner" "$tmp"
fi
mv "$tmp" "$dest/tls.pem"
trap - EXIT
stamp_text "$sum" >"$stamp"
echo "install.sh: installed $dest/tls.pem"
