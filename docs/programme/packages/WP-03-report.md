# WP-03 report (package orchestrator)

Status: **done**. Branch `claude/wp-03-vpn`, based on `phase-1` (bdbc9e7, R-024, merged in). Tip: see the last line. Not pushed, not merged.
`bun run gates` on the tip: **TIP_GATES**. Plan, decisions D-1..D-14 and the review log are in [WP-03.md](WP-03.md).

Nothing was run against the Hostinger VPS, the Hostinger account, Cloudflare or a Headscale server. Everything is files and scripts. Reviewers tested locally: Headscale 0.29.7 with network none, Tailscale 1.80/1.104 in userspace, ubuntu:24.04 with systemd 255, nftables 1.0.9, restic 0.16.4 and network namespaces.

## What is delivered

| Output | Path | Spec |
| --- | --- | --- |
| Headscale config: `server_url https://hs.anima.ing`, embedded DERP (region 999) with STUN, MagicDNS `net.anima.ing`, `node.expiry 180d`, TLS-ALPN-01, gRPC and metrics on loopback only, no OIDC | `deploy/headscale/config/config.yaml`, `render.sh` (fails closed, writes root 0600) | VPN-001, VPN-002, VPN-003, VPS-003, DNS-006 |
| Compose: Headscale 0.29.7 pinned by digest, host network bound to `${VPS_PUBLIC_IPV4}:443/:3478`, read-only, `cap_drop ALL` + `NET_BIND_SERVICE`, exact env key set | `deploy/headscale/compose.yaml` | VPS-003, LOG-007 |
| Access policy (grants): operator → hub 443/22 and the site subnets; hub → site subnets; each site router → its own subnet; every `tagOwners` list empty; no `autoApprovers`; a `tests` block. Local evaluator and a table of allowed and denied connections | `deploy/headscale/config/policy.hujson`, `src/policy.ts`, `src/tailnet.ts`, `src/hujson.ts` | VPN-004, VPN-003 |
| Firewall: nftables `inet anima_filter`, input policy drop; public 443/tcp, 3478/udp, 41641/udp; 22/tcp only on `tailscale0`. A raw prerouting drop for the host's own VPN addresses and 127/8 arriving from other interfaces (beats Docker DNAT), and a forward drop into `tailscale0` from the public side. A systemd drop-in so a stop never flushes Docker's or tailscaled's tables | `deploy/headscale/vps/nftables.conf`, `nftables-unit.conf` | VPS-002, DNS-005 |
| Hardening script: `check`, `ssh`, `updates`, `docker`, `firewall`, `confirm`; `--dry-run`, `--console`; a 180 s automatic rollback that only a new SSH session over the VPN can confirm | `deploy/headscale/vps/harden.sh`, `sshd-anima.conf`, `20auto-upgrades`, `52anima-unattended-upgrades`, `daemon.json` | VPS-001, VPS-002, VPS-004 |
| Daily backup off the VPS: SQLite `VACUUM INTO` + integrity check + keys/config/policy → restic → append-only `rest-server` on the home PC over the VPN. Home-PC prune keeps everything for 1 year and refuses future-dated or flooded snapshot lists. Sandboxed systemd units | `deploy/headscale/backup/` | VPS-006 |
| Public DNS records, declarative and not applied: `hs` A (unproxied), `hub` A to the VPN address, `updates` AAAA `100::` (proxied, set up with WP-04), CAA `letsencrypt.org`. A fill/validate step and the scoped-token rules | `deploy/dns/anima.ing.records.json`, `fill.ts`, `address.ts`, `README.md` | DNS-001, DNS-003, DNS-004, DNS-005, DNS-008, NET-005 |
| Operator guide: ordered steps per machine with the change table; Tailscale at login on macOS and Linux; route approval; key expiry and staggered renewal; VPN-007 full-tunnel warning; OD-10 risk and mitigations; restore drill; verification | `docs/setup/vpn.md`, `deploy/headscale/README.md` (file map) | VPN-001–007, VPS-001–006, OD-10 |

Tests: `bun test deploy/` gives **301 pass, 0 fail** (14 files: 12 in `deploy/headscale`, 2 in `deploy/dns`). They cover policy evaluation against the allow/deny table, HuJSON parsing (with a fuzz script), config and compose keys, duration grammars, the firewall rules as text, `harden.sh` against fake `nft`, `systemctl` and `sshd`, the backup and prune scripts against fake `restic` and `sqlite3`, unit sandbox keys, and DNS records and fill validation. Packet behaviour and systemd behaviour were proved by the security reviewers in containers (see the review log), not in the test suite.

## Operator changes, in order (runbook)

None of these has been done. Under R-025 the programme orchestrator applies them. Each is an operator change (VPS-005, DNS-008).

**Source of truth.** The exact commands are in `docs/setup/vpn.md`, cited as "guide §N.M". Short commands are repeated here. Long blocks are not, so they cannot drift from the reviewed guide.

**Placeholders.**
- `<VPS_PUBLIC_IPV4>`
- `<ADMIN_USER>`: the VPS sudo user with an SSH key.
- `<HUB_VPN_IPV4>`: printed in step 4.
- `<YOUR_EMAIL>`: the ACME contact.
- `<NODE_ID>` and `<AUTH_ID>`: printed by Headscale or the device.

`hsctl` is the shell function from guide §2.7. Define it again in each new VPS session:
```bash
hsctl() { sudo docker compose -f /opt/anima/headscale/compose.yaml exec -T headscale headscale "$@"; }
```

**Secret** = a secret value. Its name is given; the value is never shown here. **Device** = needs a person at a physical machine or a browser session that the orchestrator cannot drive.

**Gate.** Steps 1–7 can start now. **Do not start step 8 (firewall) until I send "final review clear".** The package-level security review is still running.

| # | Step | Host / API | Commands | Check | Undo |
|---|---|---|---|---|---|
| 1 | `hs` A record and CAA record | Cloudflare API v4 from the operator's Mac (`deploy/dns/README.md` Apply 1–7, rows `hs.anima.ing` and `anima.ing` CAA only) | **Secret:** a Cloudflare operator token (zone `anima.ing`, DNS:Edit, end date today), read with `read -rs` into a 0600 header file. `export VPS_PUBLIC_IPV4=…; bun deploy/dns/fill.ts --out "$FILLED" hs.anima.ing anima.ing`, then `absent … && post …` per row. Revoke the token afterwards (dashboard → API Tokens) | `dig +short hs.anima.ing @1.1.1.1` prints the VPS IPv4; `dig +short CAA anima.ing @1.1.1.1` prints `0 issue "letsencrypt.org"` | Delete both records (dashboard, or `DELETE /zones/$ZONE_ID/dns_records/<id>`) |
| 2 | Panel firewall | Hostinger panel (**Device**: browser) | Inbound: allow 443/tcp, 3478/udp, 41641/udp, plus 22/tcp for now (guide §1). Turn on 2FA on Hostinger and Cloudflare | `nc -vz <VPS_PUBLIC_IPV4> 22` from the Mac still works | Remove the rules |
| 3 | Packages, files, Headscale | VPS over public SSH: `ssh <ADMIN_USER>@<VPS_PUBLIC_IPV4>` | Guide §2.1–2.7 in full: Docker repo and packages, Tailscale repo plus `tailscale sqlite3 restic jq gettext-base`, `scp -r deploy/headscale …:anima-headscale`, the folders, `sudo env VPS_PUBLIC_IPV4=… ACME_EMAIL=… bash ./config/render.sh …`, install compose and policy, check the digest, `sudo docker compose -f /opt/anima/headscale/compose.yaml up -d` | `… compose ps` shows healthy (the log warns that policy tests fail: expected). From the Mac, `openssl s_client -connect hs.anima.ing:443 …` (§2.8) shows a Let's Encrypt issuer. `hsctl apikeys list` is empty | `sudo docker compose -f /opt/anima/headscale/compose.yaml down`. Before any node joins: `sudo rm -rf /opt/anima/headscale/lib` (destroys the server keys; never after clients join). Optionally `apt-get remove` the packages |
| 4 | User `operator`; VPS joins as `hub` | VPS (public SSH) | `hsctl users create operator`, then guide §3.1–3.3: `sudo -v`; `hsctl preauthkeys create --tags tag:hub --expiration 1h \| sudo sh -c 'umask 077; cat > /run/anima-preauth'`; `sudo systemctl enable --now tailscaled`; `sudo tailscale up --login-server https://hs.anima.ing --auth-key=file:/run/anima-preauth --hostname hub --accept-routes`; `sudo rm -f /run/anima-preauth` | `hsctl preauthkeys list` shows the key used. `tailscale ip -4` gives `<HUB_VPN_IPV4>`. `hsctl nodes list` shows hub with `tag:hub` and no expiry. `getent hosts hub.net.anima.ing` | `sudo tailscale logout`; `hsctl nodes delete --identifier <NODE_ID>`; `hsctl preauthkeys expire --id <KEY_ID>` if unused |
| 5 | `hub` A record | Cloudflare API from the Mac (README Apply again, row `hub.anima.ing`) | **Secret:** a second short-lived Cloudflare operator token. `export HUB_VPN_IPV4=…; bun deploy/dns/fill.ts --out "$FILLED" hub.anima.ing`, then `absent`/`post`. Revoke the token | `dig +short hub.anima.ing @1.1.1.1` prints `<HUB_VPN_IPV4>` | Delete the record |
| 6 | Harden SSH, updates, Docker | VPS (public SSH) | Guide §4.0–4.4: copy `vps/` to `/opt/anima/headscale/vps` (root-owned); `export ANIMA_ADMIN_USER=<ADMIN_USER>`; `sudo --preserve-env=ANIMA_ADMIN_USER ./harden.sh check`. **Device:** log in at the Hostinger browser console as `<ADMIN_USER>` and run `sudo -v` (**Secret:** the console password; set it with `sudo passwd <ADMIN_USER>` if there is none). Then `… ./harden.sh ssh` (keep the session open; test a second login), `apt-cache policy \| grep -E 'o=(Docker\|Tailscale)'`, `sudo ./harden.sh updates`, `sudo ./harden.sh --restart-docker docker`. `--dry-run` previews each | A second SSH login works. `sudo sshd -T \| grep -E 'passwordauthentication\|permitrootlogin\|allowusers'` shows no / no / `<ADMIN_USER>`. `unattended-upgrade --dry-run --debug` lists Docker and Tailscale. Headscale is healthy again | ssh: `sudo rm /etc/ssh/sshd_config.d/10-anima.conf`, restore `/var/lib/anima/10-anima.conf.previous` if it exists, `sudo systemctl reload ssh`. updates: `sudo rm /etc/apt/apt.conf.d/20auto-upgrades /etc/apt/apt.conf.d/52anima-unattended-upgrades`. docker: restore `/var/lib/anima/daemon.json.previous` to `/etc/docker/daemon.json`, or remove the file, then `sudo systemctl restart docker` |
| 7 | Admin Mac joins | **Device:** the admin Mac, plus the VPS over public SSH | Guide §6A–6C: install Tailscale ≥ 1.80 and turn on start at login; `"$TS" login --login-server https://hs.anima.ing --accept-routes`; read the auth ID **on the Mac's screen**; on the VPS run `hsctl auth register --user operator --auth-id <AUTH_ID>`, then the guarded `EXP=$(date -u -d '+180 days' …) && [ -n "$EXP" ] && hsctl nodes expire …` | `hsctl nodes list`: new node, no tags, expiry ≤ +180 d. `ssh <ADMIN_USER>@<HUB_VPN_IPV4>` logs in over the VPN | `hsctl nodes delete --identifier <NODE_ID>`; `"$TS" logout` |
| 8 | **Firewall (gated)** | VPS over **VPN** SSH: `ssh <ADMIN_USER>@<HUB_VPN_IPV4>` | Guide §4.6–4.7: `sudo --preserve-env=ANIMA_ADMIN_USER,SSH_CONNECTION ./harden.sh firewall`, then within 180 s, in a **new** VPN SSH session, `… ./harden.sh confirm` | Guide §4.9: `./harden.sh check` prints `ok: no violations`. From the Mac with Tailscale off, `nc -vz -w 5 <VPS_PUBLIC_IPV4> 22` fails and 443 still answers. Over the VPN, SSH works | Before confirm: wait 180 s and the rollback restores. After confirm: guide §4.8 from the Hostinger console (**Device**): stop the rollback timer, `sudo nft delete table inet anima_filter`, `sudo systemctl disable nftables.service`, restore `/var/lib/anima/nftables.conf.previous` |
| 9 | Remove 22/tcp from the panel | Hostinger panel (**Device**) | Delete the 22/tcp rule | With Tailscale off, public 22 times out | Add the rule back |
| 10 | Subnet routers | **Device:** home Linux PC and office 2 Linux PC, plus the VPS over VPN SSH | Guide §5.1–5.4: IP forwarding and tailscaled; on the VPS `hsctl preauthkeys create --tags tag:router,tag:site-home --expiration 1h` (`tag:site-office2` for office 2). The key is shown once; type it on the router at the `read -rsp` prompt. `sudo tailscale up … --auth-key=file:/run/anima-preauth --advertise-routes=10.10.0.0/24 --hostname home-router` (office 2: `10.30.0.0/24`, `office2-router`). `hsctl nodes approve-routes --identifier <NODE_ID> --routes <one subnet>` | `hsctl nodes list-routes`: approved and served. `hsctl nodes list`: exactly the two tags. From the VPS, `ping -c 3` a home device | `hsctl nodes delete --identifier <NODE_ID>`; on the router `sudo tailscale logout` |
| 11 | Other clients | **Device:** the office 2 Mac (and any Linux client), plus the VPS | As step 7. The second Mac gets `+150 days` (stagger, guide §6B.5) | As step 7. `tailscale status` on the client | As step 7 |
| 12 | Full policy reload | VPS (VPN SSH) | Guide §8 (only after hub, client and both routers are enrolled): install as `policy.hujson.new`, `hsctl policy check --file …`, move it into place, reload. If the check fails: delete `.new` and stop | The check exits 0. The log shows the policy reloaded | Restore the previous file and reload (guide §8) |
| 13 | Backups | **Device:** the home Linux PC, plus the VPS | Guide §9: on the home PC, `rest-server` and the `rest-server-allow.conf` drop-in with `<HUB_VPN_IPV4>`, `systemctl enable --now rest-server.service restic-prune.timer`. On the VPS, install the backup units, then `systemctl enable --now headscale-backup.timer`, a first run, and the restore drill. **Secrets:** the restic repository password (`/etc/restic/headscale-password` on the VPS and the home PC, 0600 root, and kept offline) and the rest-server htpasswd password. Both come from the password manager at a prompt | `systemctl list-timers` shows both timers. A `restic … --no-cache --no-lock snapshots` on the home PC lists the first snapshot. The restore drill passes `integrity_check` | `systemctl disable --now` the three units; remove the unit files; the repository stays |
| 14 | Recurring: renewal | Each client, plus the VPS (VPN SSH, from the *other* Mac) | Guide §7: about 30 days before expiry, `"$TS" up --force-reauth --login-server https://hs.anima.ing --accept-routes`, then approve as in step 7 | `hsctl nodes list` shows the new expiry ≈ +180 d | none needed |
| 15 | When needed: rebuild or restore | VPS, plus the home PC | Guide §9 restore drill, and `deploy/headscale/backup/README.md` "Restore" | Nodes reconnect without re-registering | none needed |
| 16 | Later, with WP-04 | Cloudflare | The hub DNS-01 token (scoped, IP-filtered, with a rotation date) and the `updates` record | WP-04 | WP-04 |

**Which steps need the operator in person.**
- Steps 2, 9 and the §4.8 recovery need the Hostinger browser panel or console.
- Step 6 includes a one-time console proof that cannot be skipped.
- Steps 7, 10, 11 and 13 need hands on the Macs, the home PC and the office 2 PC.
- The orchestrator can drive steps 1, 3–6 and 8 alone, from the operator's Mac, but only if that Mac is the admin Mac.
- If no one can reach the console tonight, stop after step 7: step 8 must not run without a proven console fallback.

## Handoffs (PKG-001)

- **Programme orchestrator, root `package.json`:** add `"deploy/headscale"` and `"deploy/dns"` to `workspaces` and refresh `bun.lock`. Until then, `bun run --filter '*'` skips them and their `tsc` fails with TS2688 (no `@types/bun`). Their tests run with `bun test deploy/`. R-021(5) lets WP-04 add `deploy` as a workspace. If it does, it must list the two nested packages, or make `deploy` a glob such as `deploy/*`, rather than a package that contains them.
- **WP-04 (hub deployment):**
  - Publish the hub only on the hub's VPN address (R-021(3) bridge plus a VPN-address publish works). The prerouting drop lets DNAT to a bridge container through when the packet came in on `tailscale0`.
  - A bridge container cannot reach the host's own VPN address. It can reach tailnet peers, MagicDNS and the site subnets.
  - Docker networks must keep the default `docker0`/`br-*` bridge names, or the forward chain drops their traffic into `tailscale0`.
  - `daemon.json` sets the default publish address to 127.0.0.1.
  - The hub DNS-01 Cloudflare token needs IP filtering for the VPS's IPv4 and IPv6, and a rotation date.
  - The Anima database backup can use its own repository on the same `rest-server` (VPS-006).
- **WP-04 (metrics):** Headscale's metrics listener on 127.0.0.1:9090 also serves its debug endpoints to every local user and to host-network processes on the VPS. If the hub runs with host networking, it can reach them.
- **WP-04 / hub vault:** the hub's credential vault runs on the same VPS as Headscale. A VPS compromise exposes both VPN reach and device credentials (OD-9, OD-10). The guide says so.

## Deviations from the brief

- **No `tag:client`.** Clients are personal nodes of user `operator` (D-1). This keeps them under `node.expiry` (VPN-003), since tagged nodes never expire.
- **Every `tagOwners` list is empty.** Otherwise a client could self-assign a tag with `--advertise-tags` when approved by `auth register` (T1 round 1). Tagging works only through tagged pre-auth keys or `nodes tag`.
- **Unit files are `*.unit` in the repository** (the names gate bans "service") and install as `.service`/`.timer`.
- **Headscale binds the public IPv4 only**, not `0.0.0.0`, so the hub can use `<VPN>:443` (D-6, D-7).
- **Backup retention is `--keep-within 1y`**, not count-based, so a compromised VPS cannot evict real snapshots (D-12).
- **Extra files beyond the brief:** `deploy/headscale/src/*` (policy evaluator, HuJSON, durations, nftables parser), `deploy/dns/fill.ts` and `address.ts`, `vps/nftables-unit.conf`, `backup/rest-server-allow.conf`.

## Deferred (for issue #1, R-007)

- Alerts: a stale backup (no snapshot for more than 36 h), a prune refusal, and a VPS appending one junk snapshot a day (each kept for a year, bounded by `--max-size`).
- An automated node-inventory check against an expected list (OD-10 detection; manual in the guide today).
- `_acme-challenge` CNAME delegation to a separate zone, so the hub DNS-01 token cannot edit the main zone (DNS-003).
- Filesystem snapshots or an offline copy of the home PC's restic repository (protects against a home-PC compromise).
- A test that every `harden.sh` subcommand, flag and path named in the guide exists.
- Verify pack data before prune (`restic check --read-data-subset` or `repair index --read-all-packs`). The VPS holds the repository key and could append crafted index or pack data (final review S4).
- An automated check that the home PC's `allow.conf` holds the current hub VPN IPv4 (alerts on hub re-registration).
- DHCPv6 accept rule in the firewall, only if the VPS gets IPv6 by DHCPv6.
- `Package-Blacklist` or a hold so unattended upgrades do not cross Docker major versions.
- A separate tag-owner identity for `tag:hub`/`tag:router` if tags ever need non-empty owners.

## Open questions

1. **Tailnet Lock (OD-10).** The spec asks to "verify this before Phase 1". Checked locally against the pinned 0.29.7 image:
   - the CLI has no lock command;
   - the config has no lock key;
   - the binary has no `hscontrol` TKA code (its `/machine/tka/*` strings come from the linked Tailscale client library).

   No one checked upstream documentation or the issue tracker. Status: open; the programme confirms before Phase 1 acceptance.
2. **apt Origin strings.** `origin=Docker` and `origin=Tailscale` were checked in an ubuntu:24.04 container with the official repositories. The operator confirms them on the VPS (guide step 4.3).
3. **Office 2 Mac.** Local routing to its own LAN while the office 2 router advertises the same subnet is not tested (guide step 6).
4. **Untested on a real VPS:**
   - `sqlite3 -readonly` `VACUUM INTO` with the live WAL and the `-shm` ownership;
   - `systemd-run … sh -c` rollback timing under load;
   - the drop-ins on the VPS's own systemd and Docker versions.
   The guide's verification section covers each.
5. **Headscale accepts unknown tags.** `preauthkeys create --tags tag:nope` succeeds in 0.29.7. The guide says to copy tag names exactly and to check `nodes list` after each join.
6. **Accept rows without `proto` in the `tests` block.** Headscale passes an accept row if any one protocol is allowed, so the server check is a smoke test. The local evaluator is the real check.

## Notes

- Task branches `wp/03-vpn/{dns,server,policy,vps,backup,guide}` are merged into the package branch and can be deleted. So can their agent worktrees.
- R-019 was passed to every builder, reviewer and gate run: DATABASE_URL was left unset.

Tip: TIP_SHA
