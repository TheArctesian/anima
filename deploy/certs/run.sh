#!/bin/sh
# Entry point of the `certs` service (DNS-002, DNS-003). Gets the hub certificate from
# Let's Encrypt with the DNS-01 challenge on Cloudflare, then checks for renewal every
# 12 hours. lego reads the Cloudflare token from CF_DNS_API_TOKEN_FILE.
# Starting this service accepts the Let's Encrypt subscriber agreement (--accept-tos).
set -eu

: "${ANIMA_ACME_EMAIL:?run.sh: ANIMA_ACME_EMAIL is not set}"
domain=hub.anima.ing
state=/lego
server=${ANIMA_ACME_SERVER:-https://acme-v02.api.letsencrypt.org/directory}
hook=/anima/certs/install.sh
chain=$state/certificates/$domain.crt
key=$state/certificates/$domain.key
retry_after_failure=3600
renew_every=43200

lego_cmd() {
    lego --accept-tos --email "$ANIMA_ACME_EMAIL" --server "$server" \
        --dns cloudflare --domains "$domain" --path "$state" "$@"
}

if [ -s "$chain" ]; then
    # A certificate exists. Install it again if an earlier install did not finish.
    if [ ! -s /certs/hub/key.pem ] || [ ! -s /certs/hub/fullchain.pem ]; then
        LEGO_CERT_PATH=$chain LEGO_CERT_KEY_PATH=$key sh "$hook"
    fi
elif ! lego_cmd run --run-hook "$hook"; then
    # Wait before the restart so a failing order does not hit the Let's Encrypt rate limit.
    echo "run.sh: the first certificate order failed; retrying in $retry_after_failure seconds" >&2
    sleep "$retry_after_failure"
    exit 1
fi

while true; do
    sleep "$renew_every"
    if ! lego_cmd renew --days 30 --renew-hook "$hook"; then
        echo "run.sh: renewal check failed; trying again in $renew_every seconds" >&2
    fi
done
