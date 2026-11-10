# Public DNS for anima.ing

`anima.ing.records.json` lists every public record in the `anima.ing` zone (Cloudflare),
in the Cloudflare API v4 "create DNS record" shape. `fill.ts` fills its placeholders after
checking them. The tests in `tests/` check both.

| Name | Type | Content | Proxied | Why |
| --- | --- | --- | --- | --- |
| `hs.anima.ing` | A | `${VPS_PUBLIC_IPV4}` | no | Headscale on the VPS (DNS-004) |
| `hub.anima.ing` | A | `${HUB_VPN_IPV4}` | no | The hub node's VPN address (DNS-004, DNS-005, NET-005) |
| `anima.ing` | CAA | `0 issue "letsencrypt.org"` | no | Keeps CAs other than Let's Encrypt and Cloudflare's out |

`HUB_VPN_IPV4` is a 100.64.0.0/10 address: the address Headscale gave the VPS host node `hub`.
It is known only after the VPS has joined the VPN (`tailscale ip -4` on the VPS).
`VPS_PUBLIC_IPV4` is the VPS public IPv4 address.

`releases.anima.ing` has no record in this file: it is the custom domain of the R2 bucket
`anima-releases`, and Cloudflare creates its record when the operator connects the domain
(DNS-007, R-107; see `docs/releases.md`). No other name serves updates.

Names under `net.anima.ing` (the Headscale MagicDNS domain) have **no** public records (DNS-006).

### What the CAA record does

The CAA record covers the whole zone. Let's Encrypt issues the certificates for `hs.anima.ing`
(Headscale, TLS-ALPN-01) and `hub.anima.ing` (hub, DNS-01, DNS-002). The R2 custom domain
`releases.anima.ing` needs Cloudflare Universal SSL, and with Universal SSL on, Cloudflare adds
its own CAA entries for the CAs it uses. These entries are hidden in the dashboard but are
served in DNS. So the effective set of allowed issuers is Let's Encrypt plus Cloudflare's CAs.
The record still keeps every other CA out. There is no `issuewild` record, so wildcard issuance
follows the same `issue` set.

## Rules

- **DNS-008.** Each change to the Cloudflare account or the `anima.ing` zone needs the
  operator's confirmation in chat. Agents prepare these files; they never apply them.
- **DNS-003.** Never use the Global API Key. Create each token in Cloudflare → My Profile →
  API Tokens → Create Token → template "Edit zone DNS", and keep only:
  - Permissions: **Zone → DNS → Edit** (nothing else).
  - Zone Resources: **Include → Specific zone → anima.ing**.
  - TTL and Client IP Address Filtering as below.
- There are two separate tokens:
  1. **Operator token.** Used only to apply these records. TTL end date: the day you use it.
     Client IP filtering: optional (your current address). Revoke it in the dashboard as soon
     as you finish, even before it expires.
  2. **Hub DNS-01 token.** Used by the hub to renew `hub.anima.ing` (DNS-002). Client IP
     Address Filtering is **required**: the VPS public IPv4 (`VPS_PUBLIC_IPV4`), and also the
     VPS public IPv6 if the VPS has one, because the hub's DNS-01 client may reach Cloudflare
     over IPv6 and the filter would then refuse it. The other way is to make the hub's DNS-01
     client use IPv4 only. Either works; the choice is a **WP-04 handoff** (stage A).
     TTL: an end date one year after creation, written into the operator's calendar with a
     reminder two weeks before. Rotate it by then: create a new token, replace the Docker
     secret, restart the hub, check renewal, revoke the old token. It does not expire silently.
     WP-04 keeps it in a Docker secret (SEC-004). It is never logged, never in an environment
     file, never in git.
- **Accepted risk.** "Zone DNS Edit" covers every record in `anima.ing`: Cloudflare cannot
  scope a token to one record. Someone who has the hub token can change any record in the zone
  until it is revoked. The IP filter limits use to the VPS. Later hardening: a CNAME from
  `_acme-challenge.hub.anima.ing` to a record in a separate zone that only the hub token can
  edit, so the hub token no longer covers `anima.ing`.
- The token never appears on a command line, in shell history, in a trace or in this
  repository. `curl` reads it from a header file (`-H @file`, curl 7.55 or later).
  **Never add `-v`, `--verbose`, `--trace` or `--trace-ascii` to these curl commands**: they
  print the `Authorization` header. Every `curl` here takes `-q` as its **first** argument,
  so it ignores `~/.curlrc`, which could turn on `verbose` and print the header.

## Apply

Run every command **from the repository root** on the operator's machine (not the VPS), with
`bun`, `curl` and `jq` installed. Use a child shell, so the `umask`, the trap and the variables
go away when you finish.

1. Start the child shell on its own:

   ```sh
   bash --noprofile --norc
   ```

   Then, inside it, paste:

   ```sh
   set +x
   umask 077
   HDR=$(mktemp "${TMPDIR:-/tmp}/cf.XXXXXX")
   FILLED=$(mktemp "${TMPDIR:-/tmp}/records.XXXXXX")
   trap 'rm -f "$HDR" "$FILLED"' EXIT
   API=https://api.cloudflare.com/client/v4
   ```

2. Write the operator token to the header file. `read -rs` does not echo it, and `printf` is a
   shell builtin, so the token is in no process's argv and not in history:

   ```sh
   read -rs CF_TOKEN && printf 'Authorization: Bearer %s\n' "$CF_TOKEN" > "$HDR"; unset CF_TOKEN
   curl -q -sS -H @"$HDR" "$API/user/tokens/verify" | jq -e '.success == true and .result.status == "active"'
   ```

   Expected: `true`.

3. Find the zone ID: Cloudflare dashboard → `anima.ing` → Overview → API → Zone ID (the
   scoped token has no permission to list zones). Then:

   ```sh
   ZONE_ID=<zone id from the dashboard>
   ```

4. Fill the records you are about to apply. `fill.ts` refuses (exit code not 0, nothing
   written) an unset or empty value, a value that is not a strict dotted quad, a
   `HUB_VPN_IPV4` outside 100.64.0.0/10, and a `VPS_PUBLIC_IPV4` that is private, CGNAT,
   loopback, link-local, documentation or reserved. Its messages never repeat the value.
   `--out FILE` must come first; then name the records, so only their placeholders are needed:

   ```sh
   export VPS_PUBLIC_IPV4=<VPS public IPv4>
   bun deploy/dns/fill.ts --out "$FILLED" hs.anima.ing anima.ing
   jq '.records[] | {type, name, content, data, proxied}' "$FILLED"
   ```

   Check the output before you continue.

5. Define the two helpers. `absent NAME TYPE` succeeds only when the API answers and the zone
   has no such record; otherwise it fails, so update or delete that record in the dashboard
   instead of adding a duplicate. `post FILTER` fails when the filter selects no record, refuses
   an A or AAAA record with empty content, sends each record in its own `curl` POST, and fails
   when Cloudflare answers `success: false`:

   ```sh
   absent() {
     curl -q -sS -H @"$HDR" "$API/zones/$ZONE_ID/dns_records?name=$1&type=$2" |
       jq -e '.success == true and (.result | length) == 0' > /dev/null && return 0
     echo "absent: $2 $1 exists or the lookup failed; not posting" >&2
     return 1
   }
   post() {
     jq -e "[.records[] | select($1)] | length > 0" "$FILLED" > /dev/null || {
       echo "post: the filter selects no record" >&2
       return 1
     }
     jq -c ".records[] | select($1)" "$FILLED" | while read -r rec; do
       if printf '%s' "$rec" | jq -e '(.type == "A" or .type == "AAAA") and ((.content // "") == "")' > /dev/null; then
         echo "post: refusing a record with empty content" >&2
         return 1
       fi
       printf '%s' "$rec" | curl -q -sS -X POST -H @"$HDR" -H 'Content-Type: application/json' \
         --data @- "$API/zones/$ZONE_ID/dns_records" |
         jq -e '{success, errors, id: .result.id, name: .result.name} | ., .success == true' || return 1
     done
   }
   ```

6. Apply in this order, each after confirmation in chat (DNS-008):

   | Step | When | Commands |
   | --- | --- | --- |
   | hs | Before Headscale first starts and requests its certificate | `absent hs.anima.ing A && post '.name == "hs.anima.ing"'` |
   | CAA | With `hs`, before any certificate is requested | `absent anima.ing CAA && post '.type == "CAA"'` |
   | hub | After the VPS has joined the VPN | `export HUB_VPN_IPV4=<address>`, then `bun deploy/dns/fill.ts --out "$FILLED" hub.anima.ing && absent hub.anima.ing A && post '.name == "hub.anima.ing"'` |

7. Finish: `exit` (the trap deletes the header file and the filled records), then revoke the
   operator token in the dashboard. If the shell died before `exit`, run
   `rm -f "${TMPDIR:-/tmp}"/cf.* "${TMPDIR:-/tmp}"/records.*`.

## Verify

```sh
dig +short hs.anima.ing @1.1.1.1      # the VPS public IPv4
dig +short hub.anima.ing @1.1.1.1     # the hub VPN address, in 100.64.0.0/10
dig +short hub.anima.ing              # same answer through the local resolver
dig +short CAA anima.ing @1.1.1.1
curl -q -sSI https://releases.anima.ing/  # after the R2 custom domain is connected: a valid certificate
```

`dig +short CAA anima.ing` should show the record from this file and the entries Cloudflare
adds, for example:

```text
0 issue "letsencrypt.org"
0 issue "pki.goog; cansignhttpexchanges=yes"
0 issue "ssl.com"
0 issuewild "letsencrypt.org"
0 issuewild "pki.goog; cansignhttpexchanges=yes"
0 issuewild "ssl.com"
```

Compare every CA in the answer with Cloudflare's documented list of CAs (dashboard: SSL/TLS →
Edge Certificates, and the Cloudflare docs on CAA records). Flag any CA that is in neither that
list nor this file.

DNS-005: from a machine **outside** the VPN (Tailscale off, for example on a phone hotspot),
`curl -q --max-time 10 https://hub.anima.ing` must fail (timeout or connection error). If it
connects, stop and report it: the hub must be reachable only over the VPN.

If `dig +short hub.anima.ing` is empty at a site but `@1.1.1.1` answers, the site resolver
(for example Pi-hole or dnsmasq rebind protection) drops private answers; allow `anima.ing`
there. The client can also take the hub address typed by the operator (NET-005).
