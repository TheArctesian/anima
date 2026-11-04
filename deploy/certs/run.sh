#!/bin/sh
# Entry point of the `certs` service (DNS-002, DNS-003). Gets the hub certificate from
# Let's Encrypt with the DNS-01 challenge on Cloudflare, then checks for renewal every
# 12 hours. lego reads the Cloudflare token from CF_DNS_API_TOKEN_FILE. Compose runs this
# service with `init: true`, so a stop signal ends the loop at once.
# Starting this service accepts the Let's Encrypt subscriber agreement (--accept-tos).
#
# Settings other than the e-mail address are for tests; compose sets none of them:
# ANIMA_LEGO (default /lego; in goacme/lego the program itself),
# ANIMA_LEGO_STATE (/var/lib/lego), ANIMA_INSTALL_HOOK (/anima/certs/install.sh),
# ANIMA_RETRY_AFTER_FAILURE_SECONDS (3600), ANIMA_RENEW_EVERY_SECONDS (43200).
set -eu

: "${ANIMA_ACME_EMAIL:?run.sh: ANIMA_ACME_EMAIL is not set}"
domain=hub.anima.ing
lego=${ANIMA_LEGO:-/lego}
state=${ANIMA_LEGO_STATE:-/var/lib/lego}
server=${ANIMA_ACME_SERVER:-https://acme-v02.api.letsencrypt.org/directory}
hook=${ANIMA_INSTALL_HOOK:-/anima/certs/install.sh}
chain=$state/certificates/$domain.crt
key=$state/certificates/$domain.key
retry_after_failure=${ANIMA_RETRY_AFTER_FAILURE_SECONDS:-3600}
renew_every=${ANIMA_RENEW_EVERY_SECONDS:-43200}

lego_cmd() {
    "$lego" --accept-tos --email "$ANIMA_ACME_EMAIL" --server "$server" \
        --dns cloudflare --domains "$domain" --path "$state" "$@"
}

# Installs what lego holds now. install.sh changes nothing when the installed file already
# matches, so this is safe at every start and after every renewal check.
install_current() {
    LEGO_CERT_PATH=$chain LEGO_CERT_KEY_PATH=$key sh "$hook"
}

# One renewal check, then the install. A renewal whose hook failed is installed here, at the
# latest one check later.
check_renewal() {
    if ! lego_cmd renew --days 30 --renew-hook "$hook"; then
        echo "run.sh: renewal check failed; trying again in $renew_every seconds" >&2
    fi
    if ! install_current; then
        echo "run.sh: installing the certificate failed; trying again in $renew_every seconds" >&2
    fi
}

# A start with a certificate checks renewal at once, so a restart after a long stop does not
# serve an expired certificate for 12 hours.
if [ -s "$chain" ]; then
    check_renewal
elif ! lego_cmd run --run-hook "$hook"; then
    # Wait before the restart so a failing order does not hit the Let's Encrypt rate limit.
    echo "run.sh: the first certificate order failed; retrying in $retry_after_failure seconds" >&2
    sleep "$retry_after_failure"
    exit 1
fi

while true; do
    sleep "$renew_every"
    check_renewal
done
