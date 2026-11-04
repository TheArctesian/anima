# Public DNS for anima.ing

`anima.ing.records.json` lists every public record in the `anima.ing` zone (Cloudflare),
in the Cloudflare API v4 "create DNS record" shape. The tests in `tests/` check it.

| Name | Type | Content | Proxied | Why |
| --- | --- | --- | --- | --- |
| `hs.anima.ing` | A | `${VPS_PUBLIC_IPV4}` | no | Headscale on the VPS (DNS-004) |
| `hub.anima.ing` | A | `${HUB_VPN_IPV4}` | no | The hub node's VPN address in `100.64.0.0/10` (DNS-004, DNS-005, NET-005) |
| `updates.anima.ing` | AAAA | `100::` | yes | Placeholder origin for the update redirect rule (WP-04 stage B, DNS-007) |
| `anima.ing` | CAA | `0 issue "letsencrypt.org"` | no | Only Let's Encrypt may issue certificates |
| `anima.ing` | CAA | `0 issuewild ";"` | no | No CA may issue wildcard certificates |

Names under `net.anima.ing` (the Headscale MagicDNS domain) have **no** public records (DNS-006).

The CAA records restrict issuance for the whole zone, so the certificates for `hs.anima.ing`
(Headscale, TLS-ALPN-01) and `hub.anima.ing` (hub, DNS-01, DNS-002) can come only from Let's
Encrypt, and nobody can get a wildcard certificate for `*.anima.ing`. Cloudflare adds hidden CAA
records for its own certificate authorities when Universal SSL is on, so the proxied
`updates.anima.ing` keeps its edge certificate; check that after applying (see Verify).

## Rules

- **DNS-008.** Each change to the Cloudflare account or the `anima.ing` zone needs the
  operator's confirmation in chat. Agents prepare this file; they never apply it.
- **DNS-003.** Never use the Global API Key. Create tokens in Cloudflare → My Profile →
  API Tokens → Create Token → template "Edit zone DNS", and keep only:
  - Permissions: **Zone → DNS → Edit** (nothing else).
  - Zone Resources: **Include → Specific zone → anima.ing**.
  - TTL: an end date.
  - Client IP Address Filtering: optional; restrict to the machine that uses the token.
- Use two separate tokens:
  1. **Operator token**: short-lived (end date today), used only to apply these records,
     revoked in the dashboard as soon as you finish.
  2. **Hub DNS-01 token**: used by the hub to renew `hub.anima.ing` (DNS-002). WP-04 keeps it
     in a Docker secret (SEC-004). It is never logged, never in an environment file, never in git.
- The token never appears on a command line, in shell history or in this repository. `curl`
  reads it from a header file (`-H @file`, curl 7.55 or later).

## Apply

Run on the operator's machine (not the VPS), with `curl`, `jq` and `envsubst` (gettext) installed.

1. Write the token to a header file readable only by you. `read -rs` does not echo, and
   `printf` is a shell builtin, so the token is not in any process's argv or in history:

   ```sh
   umask 077
   mkdir -p ~/.config/anima
   HDR=~/.config/anima/cloudflare-dns.header
   read -rs CF_TOKEN && printf 'Authorization: Bearer %s\n' "$CF_TOKEN" > "$HDR"; unset CF_TOKEN
   chmod 600 "$HDR"
   API=https://api.cloudflare.com/client/v4
   curl -sS -H @"$HDR" "$API/user/tokens/verify" | jq '{success, status: .result.status}'
   ```

   Expected: `"success": true` and `"status": "active"`.

2. Find the zone ID: Cloudflare dashboard → `anima.ing` → Overview → API → Zone ID (the
   scoped token has no permission to list zones). Then:

   ```sh
   ZONE_ID=<zone id from the dashboard>
   ```

3. Fill the placeholders. `envsubst` gets an explicit list, so nothing else is replaced.
   `HUB_VPN_IPV4` is known only after the VPS has joined the VPN: on the VPS run
   `tailscale ip -4`; it must start with `100.` and be in `100.64.0.0/10`.

   ```sh
   export VPS_PUBLIC_IPV4=<VPS public IPv4>
   export HUB_VPN_IPV4=<hub VPN IPv4>   # only once known; then run this step again
   IFS=. read -r a b _ <<<"$HUB_VPN_IPV4"
   [ "$a" = 100 ] && [ "$b" -ge 64 ] && [ "$b" -le 127 ] && echo "hub address in 100.64.0.0/10"
   FILLED=$(mktemp)
   envsubst '${VPS_PUBLIC_IPV4} ${HUB_VPN_IPV4}' < deploy/dns/anima.ing.records.json > "$FILLED"
   jq '.records[] | {type, name, content, data, proxied}' "$FILLED"
   ```

   Check the output before you continue. An unset variable becomes an empty `content`: apply
   a record only once its placeholder is filled, and `hub` only after the range check prints.

4. Define one helper that posts each selected record on its own (one `curl` POST per record):

   ```sh
   post() {
     jq -c ".records[] | select($1)" "$FILLED" | while read -r rec; do
       printf '%s' "$rec" | curl -sS -X POST -H @"$HDR" -H 'Content-Type: application/json' \
         --data @- "$API/zones/$ZONE_ID/dns_records" | jq '{success, errors, id: .result.id, name: .result.name}'
     done
   }
   exists() {
     curl -sS -H @"$HDR" "$API/zones/$ZONE_ID/dns_records?name=$1" | jq '.result[] | {id, type, name, content}'
   }
   ```

   Run `exists <name>` first; if a record already exists, update or delete it in the
   dashboard instead of adding a duplicate.

5. Apply in this order, each after confirmation in chat (DNS-008):

   | Step | When | Command |
   | --- | --- | --- |
   | hs | Before Headscale first starts and requests its certificate | `exists hs.anima.ing; post '.name == "hs.anima.ing"'` |
   | CAA | With `hs`, before any certificate is requested | `exists anima.ing; post '.type == "CAA"'` |
   | hub | After the VPS has joined the VPN and `HUB_VPN_IPV4` is set and filled (step 3) | `exists hub.anima.ing; post '.name == "hub.anima.ing"'` |
   | updates | With WP-04 stage B (the redirect rule) | `exists updates.anima.ing; post '.name == "updates.anima.ing"'` |

6. Clean up: `rm -f "$FILLED" "$HDR"`, then revoke the operator token in the dashboard.

## Verify

```sh
dig +short hs.anima.ing @1.1.1.1      # the VPS public IPv4
dig +short hub.anima.ing @1.1.1.1     # the hub VPN address, 100.64.0.0/10
dig +short hub.anima.ing              # same answer through the local resolver
dig +short CAA anima.ing @1.1.1.1     # 0 issue "letsencrypt.org" and 0 issuewild ";"
                                      # (Cloudflare may add its own CA entries)
curl -sSI https://updates.anima.ing/  # after WP-04 stage B: a valid certificate and a 302
```

DNS-005: from a machine **outside** the VPN (Tailscale off, for example on a phone hotspot),
`curl --max-time 10 https://hub.anima.ing` must fail (timeout or connection error). If it
connects, stop and report it: the hub must be reachable only over the VPN.

If `dig +short hub.anima.ing` is empty at a site but `@1.1.1.1` answers, the site resolver
(for example Pi-hole or dnsmasq rebind protection) drops private answers; allow `anima.ing`
there. The client can also take the hub address typed by the operator (NET-005).
