#!/bin/sh
# Runs once, on the first start of an empty data volume, from /docker-entrypoint-initdb.d
# of the official postgres image. Compose mounts roles.sql at /anima/roles.sql and sets
# ANIMA_OWNER_PASSWORD_FILE and ANIMA_HUB_PASSWORD_FILE to the mounted secrets (DB-003).
set -eu

: "${ANIMA_OWNER_PASSWORD_FILE:?set ANIMA_OWNER_PASSWORD_FILE to the owner password file}"
: "${ANIMA_HUB_PASSWORD_FILE:?set ANIMA_HUB_PASSWORD_FILE to the hub password file}"
for file in "$ANIMA_OWNER_PASSWORD_FILE" "$ANIMA_HUB_PASSWORD_FILE"; do
    if [ ! -s "$file" ]; then
        echo "10-anima-roles: $file is missing or empty; no role was made" >&2
        exit 1
    fi
done

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname postgres -f /anima/roles.sql
