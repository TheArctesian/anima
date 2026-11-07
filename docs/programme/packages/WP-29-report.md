# WP-29 report — Hub check types

Status: done. Package branch `claude/wp-29-check-types`, from 59ab338. Plan and review log: [WP-29.md](WP-29.md).

## Tasks

| Task | Accepted SHA | What |
| --- | --- | --- |
| T1 listing | bb46eef0 | Docker and systemd checks through `sessions::link` with the static D-3 commands; pure parsers; migration 0310 (all four ids); `Judgement` (Starting = Degraded, Unjudged = Unknown) in `evaluate` |
| T4 http-urls | 0190c653 | `checks::http_urls` (K3, one statement); `web_interfaces` suggestions call it (WP-30 H-1) |
| T5 batching-test | afd10440 | The API-005 statement tests use a 500 ms loader window (`schema_batched_at`) |
| T2 tailscale-ping | adb10ded | Ping through the LocalAPI proxy; Unknown with a message without `ANIMA_TAILSCALE_LOCALAPI`; only permitted 100.64.0.0/10 IPv4 targets are sent |
| T3 snmp | 5350c0d7 | SNMP v2c and v3 authPriv (HMAC-SHA-256-192, AES-128-CFB, hand-written BER and USM); secrets sealed with `checks.checks.snmp:<id>`, bound to the version and user name |

`@pending` is gone from every check-type member of K1.

## Contract change requests

- **CR-1 (K3):** `checks::Probing` gains `localapi: Option<String>`. Accepted by the programme.

## Handoffs

- **H-1 WP-25 / client (`lib/forms/check/probes/validate.ts`).** The hub refuses three things the editor accepts: v3 passwords under 8 characters, user names over 32 bytes, and OIDs with more than 128 arcs. The hub also refuses a v2c user name and secrets of the other version. It keeps an empty secret only when the user name is unchanged too.
- **H-2 (`lib/checks/api.ts`, WP-25 H-3).** Spread the four config fragments into `CheckFields`; the hub now serves them.
- **H-3 WP-33 (`checks/rows.ts`).** Labels for DOCKER, SYSTEMD, SNMP and TAILSCALE_PING (WP-25 H-4).

## Needs a live device or the proxy

- Docker and systemd: a real SSH device with an accepted host key, where `docker ps` runs without sudo.
- Tailscale ping: the WP-23 proxy on the VPS. A 403 on ping means tailscaled wants write access.
- SNMP v3: tested only against a local agent, because net-snmp 5.6 has no SHA-256. It needs a real v3 SHA-256/AES agent. v2c and discovery match net-snmp captures.

## Open risks and coulds (issue #1)

- The sealed SNMP secrets are not bound to the device host; this is logged as an operator decision.
- A link error that is "SSH off" or "no credential" is recorded as Down. Unjudged would be truer to D-2.
- `Failure::Link(String)` could carry a typed reason.
- The scheduler's `observe` arm is a catch-all; it could list the variants.
- `store/sealed.rs` and `store/snmp_sealed.rs` repeat the seal-and-open flow.
- Some hash temporaries are not wiped, and a malformed datagram ends the attempt where RFC 3412 says to drop it.
- Add an audit `[set]` test for the v2c community.
- Several files are near 300 lines: `graphql/form.rs`, `config/rules.rs`, `probe/snmp/message.rs`.
