# WP-23 report — Gateway, compose and VPN policy

Status: **done** (files and tests). Live steps below wait for the operator (VPS-005).
Branch `claude/wp-23-gateway-policy` from `phase-2` at a6c57db. Not pushed, not merged into `phase-2`.

## What changed

| Task | Spec | Change |
| --- | --- | --- |
| T1 drives | RA-021, GW-001, GW-002 | `deploy/gateway/Dockerfile` makes `/var/lib/guacd/drives` (guacd 1000:1000, 0700); compose mounts the named volume `gateway_drives` there in guacd only (root stays read-only); hub gets `ANIMA_GUACD_DRIVE_ROOT=/var/lib/guacd/drives` (K7) |
| T2 localapi | D-4, VPN-005, GW-003, LOG-007 | New `deploy/localapi/` (Bun 1.3.14 distroless, uid 65532): `allow.ts` (pure allow list) and `server.ts` (edge). Only `GET /localapi/v0/status[?peers=true\|false]` and `POST /localapi/v0/ping?ip=<100.64.0.0/10>&type=disco` reach the host socket; the upstream request is rebuilt from parsed parts with no client headers or body; 403 never opens the socket; 1 MiB streaming cap (502), 10 s timeout (504). Compose service `localapi` (hardened, no port, `/var/run/tailscale` read-only), internal network `localapi` (172.30.2.0/24) shared only with the hub, hub `ANIMA_TAILSCALE_LOCALAPI=localapi:8080`, subnet added to `ANIMA_PROBE_DENY` |
| T3 policy | VPN-004, R-061, R-050 | Grant `tag:hub` → `device-stephens-mac` is now tcp:22 and tcp:5900 (VNC, criterion 3); tests block accepts 100.64.0.2:5900, denies 3389. New `tests/nft-eval.ts` and `vps-forward.test.ts` prove from `vps/nftables.conf` that a Docker bridge reaches tailnet and site addresses out of `tailscale0`, and a public-interface packet is dropped. No nftables change |

`policy.hujson` sha256: `ba1cd173b710cfcb38277b88f8329b79339c8a5bc81c495049d2bc7c53a4872b`.

## Review rounds

| Task | Round | Reviewer | Must | Result |
| --- | --- | --- | --- | --- |
| T1 | 1 | reviewer (Sonnet), read only | 0 | Accepted. 2 should (text-only Dockerfile check; one repeated hardening test) left as coulds |
| T2 | 1 | reviewer-security (Opus), 24 raw-socket bypass attempts, 43 mutants | 0 | 2 should fixed in round 2 (first-octet test; streaming cap, timeout, header and redirect tests, timeout made a config field) |
| T3 | 1 | reviewer-security (Opus), mutants on grant and evaluator | 0 | 1 should and 3 could fixed in round 2 (tests-block content, first match, policy drop, unknown verdict, empty oifname, test names) |

Round-2 diffs were tests and one config field; the orchestrator read them and accepted without a third round (R-066).

## Gates

`env -u DATABASE_URL BASE_REF=a6c57db bun run gates` on the package head (merge of T1, T2, T3 before this report): **12/12 pass**.
The hub test `k4_8_5_a_client_ping_sent_while_the_hub_s_send_waits_keeps_the_session` failed under load
in 2 of 4 builder runs and passed alone; it is not WP-23 code (handoff H-3).

## Handoffs

- **H-1 (WP-20, `contracts/conventions.md` K7):** add `ANIMA_TAILSCALE_LOCALAPI` and `ANIMA_GUACD_DRIVE_ROOT` to
  the environment table. `deploy/test/compose-hub.test.ts` accepts names from `phase-2-contracts.md` until
  then; drop that fallback when K7 is in `contracts/`. The proxy's own `ANIMA_LOCALAPI_LISTEN_PORT` and
  `ANIMA_LOCALAPI_SOCKET` are deploy-only and need no K7 row unless the programme wants one.
- **H-2 (programme, `deploy/README.md`, not owned):** add `ANIMA_LOCALAPI_SUBNET` and
  `ANIMA_TAILSCALE_LOCALAPI` to the variable table and a line on the `localapi` service.
- **H-3 (WP-22 or the K4 §8 owner):** the load-flaky terminal test above.
- **H-4 (P2-004 owner, WP-22/WP-27):** the SSH session target check does not yet read `ANIMA_PROBE_DENY`, so an
  SSH device could name `localapi:8080`; no impact today (Bun answers 400, nothing reaches tailscaled).
- **H-5 (WP-29, WP-28/VPN-005 users):** the hub must not log raw status output (it holds peer names and logins).

## Live steps (operator, in order)

`VPS` is the SSH target used for earlier deploys (for example `anima-admin@100.64.0.1`). Run from the repo
root at the package head. The current hub image needs no change: it ignores the two new variables.

1. Read-only checks: `ssh $VPS 'stat -L -c "%a %U %F" /var/run/tailscale/tailscaled.sock; sudo tailscale debug prefs | grep -E "\"OperatorUser\"|\"AdvertiseRoutes\""'`
   Expect `666 root socket`, no operator user, no 172.30.0.0/16 route.
2. Build on the Mac:
   `docker buildx build --platform linux/amd64 --load -t anima-guacd:local --build-context fonts=apps/client/src/styles/fonts deploy/gateway`
   `docker buildx build --platform linux/amd64 --load -t anima-localapi:local deploy/localapi`
3. Keep the old gateway image: `ssh $VPS 'sudo docker tag anima-guacd:local anima-guacd:previous'`
4. Load: `docker save anima-guacd:local anima-localapi:local | gzip | ssh $VPS 'gunzip | sudo docker load'`
5. Install the files: `tar -C deploy -cf - compose.yaml localapi gateway | ssh $VPS 'sudo cp /opt/anima/deploy/compose.yaml /opt/anima/deploy/compose.yaml.before-wp23 && sudo tar -C /opt/anima/deploy -xf -'`
6. Start: `ssh $VPS 'cd /opt/anima/deploy && sudo docker compose config -q && sudo docker compose up -d --no-build localapi guacd hub && sudo docker compose ps'`
   Expect localapi, guacd and hub healthy.
7. Check the proxy (no new image):
   `ssh $VPS 'for u in "GET /localapi/v0/status?peers=false" "POST /localapi/v0/ping?ip=100.64.0.2&type=disco" "GET /localapi/v0/prefs"; do set -- $u; sudo docker run --rm --network deploy_localapi --entrypoint /usr/local/bin/bun anima-localapi:local -e "const r=await fetch(\"http://localapi:8080$2\",{method:\"$1\"});console.log(\"$1 $2\",r.status)"; done'`
   Expect 200, 200, 403. A 403 on ping means tailscaled wants write permission: tell the programme (the check then reports Unknown).
8. Check the drives root: `ssh $VPS 'cd /opt/anima/deploy && sudo docker compose exec guacd ls -ldn /var/lib/guacd/drives'` — expect `drwx------ … 1000 1000`.
9. Policy: `scp deploy/headscale/config/policy.hujson $VPS:/tmp/policy.hujson` then
   `ssh $VPS 'sha256sum /tmp/policy.hujson && sudo cp /opt/anima/headscale/config/policy.hujson /root/policy.before-wp23.hujson && sudo install -m 0644 /tmp/policy.hujson /opt/anima/headscale/config/policy.new.hujson && sudo docker compose -f /opt/anima/headscale/compose.yaml exec headscale headscale policy check --file /etc/headscale/policy.new.hujson'`
   Expect the sha256 above and "Policy is valid". Then
   `ssh $VPS 'sudo mv /opt/anima/headscale/config/policy.new.hujson /opt/anima/headscale/config/policy.hujson && sudo docker compose -f /opt/anima/headscale/compose.yaml kill -s HUP headscale && sleep 3 && sudo docker compose -f /opt/anima/headscale/compose.yaml logs --tail 5 headscale && sudo docker compose -f /opt/anima/headscale/compose.yaml ps'`
   Expect "Policy reload completed with changes" and headscale running (restart policy is `always`).
10. With Screen Sharing on the Mac: `ssh $VPS 'sudo docker run --rm --network deploy_gateway --entrypoint /usr/local/bin/bun anima-localapi:local -e "setTimeout(()=>{console.log(\"timeout\");process.exit(1)},5000);const s=await Bun.connect({hostname:\"100.64.0.2\",port:5900,socket:{data(){}}});console.log(\"open\");s.end();process.exit(0)"'` — expect `open` (R-050 bridge → `tailscale0` path, live).

Undo: `docker tag anima-guacd:previous anima-guacd:local`, restore `compose.yaml.before-wp23`, `docker compose up -d guacd hub`, `docker compose rm -sf localapi`; for the policy, install `/root/policy.before-wp23.hujson` and send HUP as in step 9.

## Open coulds (issue #1)

- T1: Dockerfile checked as text only; a build-and-stat test would be stronger. One repeated guacd hardening test.
- T2: digest pins for `oven/bun` (the file pins tags everywhere today); K7 rows for the proxy's own variables.
- RDP grant for the Windows machine: one `device-<name>` host and a `tcp:3389` grant from `tag:hub`, with tests, when the machine has a name and address (D-5).
