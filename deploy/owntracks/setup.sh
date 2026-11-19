#!/bin/sh
# Makes the password of the OwnTracks server and the file that holds the Anima
# ingest token. Run it one time, in this folder, before `docker compose up -d`.
# It prints the password one time. It writes only into ./secrets (not in git).
set -eu

cd "$(dirname "$0")"
umask 077
mkdir -p secrets

if [ -e secrets/htpasswd ]; then
    echo "secrets/htpasswd exists: the password is not changed. Delete the file to make a new one." >&2
else
    user="${OWNTRACKS_USER:-owntracks}"
    password="$(openssl rand -base64 24 | tr -d '/+=' | cut -c1-24)"
    printf '%s:%s\n' "$user" "$(openssl passwd -apr1 "$password")" > secrets/htpasswd
    echo "OwnTracks user:     $user"
    echo "OwnTracks password: $password   (shown one time)"
fi

# The token comes from `anima location token add owntracks`. Give it on standard
# input, so that it is not in the shell history:  ./setup.sh < token-file
if [ ! -t 0 ]; then
    token="$(cat)"
    if [ -n "$token" ]; then
        basic="$(printf 'owntracks:%s' "$token" | base64 | tr -d '\n')"
        printf 'proxy_set_header Authorization "Basic %s";\n' "$basic" > secrets/anima-authorization
        echo "The Anima ingest token is stored in secrets/anima-authorization."
    fi
fi
[ -e secrets/anima-authorization ] || : > secrets/anima-authorization
chmod 644 secrets/htpasswd secrets/anima-authorization
