# Anima Agent Implementation Plan — Phase 2

| Item | Value |
| --- | --- |
| Document | Agent implementation plan, Phase 2 |
| Scope | Phase 2 of [design-spec.md](../design-spec.md) (Section 9): RDP and VNC sessions, web interfaces, Wake-on-LAN, SFTP, the live session list, SSH metrics, four more check types, the device detail view with charts |
| Base | `phase-2`, cut from `phase-1` at f3c6feb (Phase 1 accepted, PR #3 open against `main`) |
| Contract deltas | [phase-2-contracts.md](phase-2-contracts.md) |
| Version | 1.0 (adopted, R-065) |
| Date | 2026-11-06 |
| Language | Simplified Technical English |

## 1. What stays from Phase 1

This plan uses the Phase 1 plan ([implementation-plan.md](../implementation-plan.md)) without change for:

- the roles and models (Section 2, AGT-001 to AGT-005);
- the task, package and programme loops (Section 3, LOOP-001 to LOOP-004);
- the contract rules (Section 4, CON-001 to CON-004) and the shared-file rules (PKG-001 to PKG-003);
- the gates (Section 7). Run them with `env -u DATABASE_URL BASE_REF=<base> bun run gates`;
- the files and communication rules (Section 8) and the operator checkpoints (Section 9);
- every ruling R-001 to R-064 in [programme.md](programme.md). R-024 and R-027 apply: the programme
  orchestrator merges reviewed work that passes 12/12 into `phase-2`, and a package may start when the
  interfaces it uses are on `phase-2`.

Phase 2 changes these items:

- **P2-001.** The integration branch is `phase-2`. Package branches are `wp/NN-slug` from `phase-2`.
  `--base phase-2` replaces `--base phase-1` in the baseline gate.
- **P2-002.** The K1 additions carry the directive `@pending` until the hub implements them
  (decision D-1). WP-20 changes the CON-002 test so that it ignores a `@pending` member only while
  the hub does not declare it. When the hub declares the member, the programme orchestrator removes
  `@pending` at integration. WP-34 checks that no `@pending` remains.
- **P2-003.** Every new SSH use by the hub (SFTP, metrics, Docker and systemd checks, Wake-on-LAN)
  goes through one SSH link in `sessions` (K3 delta). It pins the accepted host key (SEC-006,
  SEC-007) and runs only commands built from static text (CHK-012, MET-002).
- **P2-004.** Every hub connection to a device (SSH, RDP, VNC, SFTP, metrics, checks) obeys the
  target rule (R-032) and the deny list `ANIMA_PROBE_DENY` (R-030). This closes R-032 E-7 (D-7).

## 2. Contracts for Phase 2

| ID | Change | Where it goes |
| --- | --- | --- |
| K1 | Protocols RDP and VNC, remote access settings, web interfaces, metrics, four check types, live session changes, files tickets, Wake-on-LAN | `contracts/schema.graphql`, with `@pending` (P2-002) |
| K2 | Ranges 0700–0799 `metrics`, 0800–0849 `web_interfaces`; new tables in `devices`, `checks`, `sessions`; new credential kinds | `contracts/database.md` |
| K3 | `sessions` SSH link, wake and files; `metrics` and `web_interfaces` faces; new import edges | `contracts/hub-domains.md` |
| K4 | RDP and VNC handshake and relay values; certificate pinning; the files tunnel `/files` | `contracts/tunnel.md` |
| K5 | Chart colours | `contracts/tokens.md` |
| K6 | `contracts/fixtures/phase2.json`: RDP, VNC and web devices, a GPU device, new checks | `contracts/fixtures/` |
| K7 | IDs `WebInterfaceId`, `MetricSeriesId`; variables `ANIMA_GUACD_DRIVE_ROOT`, `ANIMA_TAILSCALE_LOCALAPI` | `contracts/conventions.md` |

The CON-002 test failed when one enum value was added to K1 (`Protocol.RDP: in K1, not in the hub`).
Thus the deltas are in [phase-2-contracts.md](phase-2-contracts.md) and `contracts/` is not changed yet.
After the operator approves the deltas (checkpoint 1), WP-20 adds the `@pending` rule to the test and the
programme orchestrator copies the deltas into `contracts/` in the same commit.

## 3. Work packages

Each package owns its paths. Paths inside `crates/hub/src/sessions/` are split between WP-22 and WP-27.
The face `sessions.rs`, `schema.rs`, `main.rs`, `lib.rs` and `quality/hub-modules.json` are programme files:
packages send handoff notes, and the programme orchestrator applies them at integration (PKG-001).

| ID | Package | Owned paths | Depends on | Builders | Reviewers | Spec IDs |
| --- | --- | --- | --- | --- | --- | --- |
| WP-20 | Phase 2 skeleton and contract gate | `crates/hub/tests/it/contract.rs`, `crates/hub/src/ids.rs`, `config.rs`, new leaf `targets.rs` (moves `AddressPolicy` out of `checks`), `quality/`, `contracts/fixtures/phase2.json`, stylelint `@keyframes` rule, `AGENTS.md` (bun) | Operator approves the deltas | Sonnet, Haiku | Sonnet | CON-002, CQ-010, MV-008, REPO-004, K6, K7 |
| WP-21 | Hub remote access settings | `crates/hub/src/devices/`, `crates/hub/src/credentials/`, migrations 0110–0199, 0210–0299 | WP-20 | Sonnet, Opus (credentials) | Sonnet, Opus (security) | Section 6.2, 6.3.2, RA-020, RA-030, RA-031 (MAC), RA-032, RA-033 (flags), DEV-010–013, SEC-003, SEC-005 |
| WP-22 | Hub RDP and VNC sessions, live list | `sessions/tunnel/`, `sessions/start.rs`, `sessions/model.rs`, `sessions/live.rs`, `sessions/graphql/`, `sessions/trust/`, migrations 0610–0649 | WP-21 (types, R-027) | Opus | Opus (security), Sonnet | HUB-002, GW-010–015, RA-004–008, RA-020, RA-021, RA-030, RA-033, RA-034, SEC-007 (RDP), K4 §2–5 |
| WP-23 | Gateway, compose and VPN policy | `deploy/gateway/`, `deploy/compose.yaml`, `deploy/headscale/config/policy.hujson`, `deploy/headscale/tests/`, `deploy/test/`, the LocalAPI proxy (D-4) | WP-20 | Sonnet, Haiku | Opus (security) | GW-001–004, RA-021, VPN-004, VPN-005, VPS-005 |
| WP-24 | Client RDP and VNC session tabs | `apps/client/src/lib/sessions/` except `files/`, `live/`, `web/` | WP-20 (mock, CON-003) | Opus | Sonnet (interface), Opus (security) | RA-001, RA-002, RA-004–007, RA-030, RA-033, GW-015, MAT-008, MOT session tab open |
| WP-25 | Client device form | `apps/client/src/lib/forms/`, `routes/devices/[id]/settings/` | WP-20 | Sonnet | Sonnet (interface) | Section 6.2, 6.3.2, RA-020, RA-030–033, WEB-001, WEB-002, DEV-011, DEV-013, new check editors |
| WP-26 | Client device list and header | `apps/client/src/lib/devices/`, `routes/devices/[id]/header/` | WP-20; WP-33 bars (R-027) | Sonnet | Sonnet (interface) | Section 6.1, DEV-020–022, RA-020, RA-030, WEB-003, MET-004, MET-008, MOT menu and reorder |
| WP-27 | Hub SSH link, SFTP and Wake-on-LAN | `sessions/link/`, `sessions/files/`, `sessions/wake/`, migrations 0650–0699 | WP-21 (MAC, flags) | Opus | Opus (security), Sonnet | RA-031, RA-032, CHK-012, MET-002, VPN-006, LOG-005, LOG-007, K4 §9 |
| WP-28 | Hub metrics | `crates/hub/src/metrics/`, migrations 0700–0799 | WP-27 link (R-027) | Opus (collector), Sonnet (parsers) | Sonnet, Opus (security) | HUB-004, HUB-005, MET-001–003, MET-005–009, DB-004, DB-005, DB-008, DB-009, DEV-015 |
| WP-29 | Hub check types | `crates/hub/src/checks/`, migrations 0310–0399 | WP-27 link; WP-23 proxy for Tailscale ping | Sonnet, Opus (SNMP v3) | Sonnet, Opus (security) | Section 6.4 Phase 2 types, CHK-001–003, CHK-008, CHK-009, CHK-012, SEC-003 |
| WP-30 | Hub web interfaces | `crates/hub/src/web_interfaces/`, migrations 0800–0849 | WP-20 | Sonnet | Sonnet, Opus (security) | WEB-001, WEB-002, WEB-008, WEB-009, LOG-005 |
| WP-31 | Desktop web view and web tab | `apps/desktop/src-tauri/`, `apps/client/src/lib/desktop/`, `apps/client/src/lib/sessions/web/` | WP-20; OD-7 spike is task 1 | Opus | Opus (security), Sonnet (interface) | WEB-004–008, WEB-010, OD-7, REPO-003, MAT-008 |
| WP-32 | Client files, wake and live sessions | `apps/client/src/lib/sessions/files/`, `lib/sessions/live/`, the navigation bar item (D-8) | WP-20 | Sonnet, Opus (transfers) | Sonnet (interface), Opus (security) | RA-031, RA-032, RA-034, MOT menu, MOT-009 |
| WP-33 | Client detail view and charts | `apps/client/src/lib/charts/`, `lib/metrics/`, `routes/devices/[id]/overview/`, `checks/`, `sessions/`, `tabs/` | WP-20 (K5 chart names) | Opus (chart motion), Sonnet | Sonnet (interface) | Section 6.7, MET-003, MET-007–009, MOT-008, MOT chart update and loading |
| WP-34 | Phase 2 integration and acceptance | `docs/research/`, acceptance tests | All | Sonnet | Opus | Section 4 of this plan, spec Section 5.8, CQ-050 |

Package notes:

- **WP-20** also closes the Phase 1 handoffs in the WP-17 report: `pnpm` to `bun` in `AGENTS.md` and spec
  Section 13, the MV-008 `@keyframes` ban, and the cursor time-range bound (CQ1-02, `events/page.rs`) as a
  handoff to the `events` owner.
- **WP-22** task 1 records guacd 1.6.0's `args` for `rdp` and `vnc` and confirms `cert-fingerprints`,
  `disable-copy`, `disable-paste`, `disable-download` and `disable-upload` exist (D-5). It mounts `/tunnel`
  through a handoff to `lib.rs` (R-062 left it unmounted). The wake step in `start.rs` calls WP-27's
  `sessions::wake` once it is on `phase-2`; until then `wakeSent` is always false.
- **WP-27** builds the link first (task 1) and merges it early, because WP-28 and WP-29 use it.
- **WP-31** starts with the OD-7 spike: can WKWebView (macOS) and WebKitGTK (Linux) show a per-device
  data store, block the Tauri IPC and report an untrusted certificate's SHA-256? The result picks the
  embedded tab or the system-browser fallback (D-9).

## 4. Waves and critical path

```text
Wave 0 (serial)    WP-20 skeleton + contract freeze (operator checkpoint 1)
                       |
Wave 1 (RDP, VNC)  WP-21 --> WP-22     WP-23     WP-24     WP-25
                       |       (WP-22 starts on WP-21's types, R-027)
Wave 2             WP-27     WP-30     WP-31     WP-26
                       |
Wave 3             WP-28     WP-29     WP-32     WP-33
                       |
Wave 4 (serial)    WP-34 integration and acceptance
```

- **P2-WAVE-001.** RDP and VNC go first (operator request): WP-21, WP-22, WP-23 and WP-24 get the first
  four slots. WAVE-002 limits stay: 5 package orchestrators, 3 builders and 3 reviewers per package.
- **P2-WAVE-002.** Critical paths:
  - RDP and VNC: WP-20 → WP-21 → WP-22 → WP-34 (hub) and WP-20 → WP-24 → WP-34 (client). The live
    RDP check waits for the operator's Windows machine (Section 6).
  - Metrics (longest): WP-20 → WP-27 (link) → WP-28 → WP-33 (live charts) → WP-34.
- **P2-WAVE-003.** WP-26 takes the metric bars from WP-33 when they are on `phase-2` (R-027). Until
  then it uses a placeholder with the final geometry (MOT-007).

## 5. Phase 2 acceptance criteria

Drawn from spec Sections 6.3 to 6.7. "Operator" means a live step only the operator can do.

| # | Criterion | Spec IDs | Operator |
| --- | --- | --- | --- |
| 1 | The operator opens an RDP session to a Windows device from its card. The resolution follows the window, text copy and paste work, and Ctrl+Alt+Delete goes from the Keys menu. | RA-004–006, GW-010–012 | Yes: a Windows machine with RDP and NLA on, reachable by the hub |
| 2 | The hub records the RDP server certificate at first use. A changed certificate stops the session with a warning. `ignore-cert` and `security=rdp` show a warning badge. | SEC-007 (RDP, D-5), RA-020 | Yes, for the live certificate |
| 3 | The operator opens a VNC session to macOS Screen Sharing and to a Linux desktop. The card and header show "VNC". | RA-030 | Yes: Screen Sharing on, a VPN grant on tcp:5900 |
| 4 | When the gateway asks for a missing credential, the client asks. The hub keeps it only after "Save". | GW-015 | No |
| 5 | Copy off and paste off work for each device: the hub enforces them for RDP and VNC; the client for SSH (D-6). | RA-033 | No |
| 6 | An SSH session has a file panel. Upload and download work and obey the device's flags. Each transfer has an audit entry with no content. | RA-032, LOG-005, LOG-007 | Yes, against a real SSH server |
| 7 | The client lists the live sessions of every client. "Stop" ends one. | RA-034, RA-008 | No |
| 8 | A Down device with a MAC address gets a Wake-on-LAN packet from its site relay before the session connects. | RA-031, VPN-006 | Yes: a device that sleeps on a LAN and a relay at that site (D-2) |
| 9 | A web interface opens in a tab with its own data store and no access to Tauri commands. The toolbar works. An untrusted certificate shows its fingerprint, is pinned on accept, and a change stops with a warning. A Down device shows the error page. | WEB-003–010 | Yes: OD-7 on macOS and on Linux; a real device web interface |
| 10 | The hub suggests a web interface for each HTTP(S) check; it shows only after the operator accepts it. | WEB-001, WEB-002 | No |
| 11 | The hub collects SSH metrics every 30 seconds with read-only commands from Linux and macOS devices, and finds GPUs at first collection, each 24 hours and after a restart. | MET-001, MET-002, MET-006, HUB-004 | Yes, for GPU vendors (synthetic in tests) |
| 12 | The device card shows CPU, memory, disk and VRAM bars, in `status-degraded` above 85 %. No VRAM bar without a GPU with its own memory. | MET-004, MET-007, MET-008 | No |
| 13 | The device detail view has the five tabs of Section 6.7, charts for 1 hour, 24 hours and 7 days, a chart per GPU, and the header of DEV-020–022 with VNC and Web buttons. | MET-003, MET-009, DEV-020–022, WEB-003 | No |
| 14 | Metric aggregates and retention follow DB-008 and DB-009 with daily partitions. Deleting a device deletes its metrics. | DB-004, DB-005, DB-008, DB-009, DEV-015 | No |
| 15 | Docker container, SNMP, Tailscale ping and systemd checks run with fixed type identifiers and start no shell command with operator text. | Section 6.4, CHK-009, CHK-012 | Yes: Tailscale ping needs the LocalAPI proxy on the VPS (D-4) |
| 16 | No log contains a password, key, community string, token or session content. No session content or file content is recorded. | LOG-007, RA-008, SEC-009 | No |
| 17 | Each motion role Phase 2 uses (session tab open, menu, chart update, loading) has a reduced-motion form. The Phase 2 motion audit has no open "layout jump" or "wrong motion". Nothing moves or blurs over a live display. | MOT-040, MOT-041, Section 5.8, MAT-008 | No |
| 18 | Gates pass 12/12, the baseline is empty, and K1 has no `@pending`. | CQ-010, CON-002 | No |

## 6. Infrastructure changes

Each change on the VPS, Headscale or Cloudflare needs the operator's confirmation in chat (VPS-005,
DNS-008). WP-23 delivers files and steps; the programme orchestrator asks before it applies them.

| Item | Change | Why |
| --- | --- | --- |
| guacd | Keep `guacamole/guacd:1.6.0`. WP-22 confirms the RDP and VNC arguments it needs. No new fonts. | GW-001, D-5 |
| Compose, gateway | A named volume `gateway_drives` mounted read-write at `/var/lib/guacd/drives` in guacd only; the root stays read-only. | RA-021 |
| Compose, hub | Mount `/tunnel` (hub code). Set `ANIMA_GUACD_DRIVE_ROOT`. Set `ANIMA_TAILSCALE_LOCALAPI` if D-4 is accepted. | K4, K7 |
| LocalAPI proxy (D-4) | A small container that reads the host's `tailscaled` socket and allows only `GET /localapi/v0/status` and `POST /localapi/v0/ping`, on the `gateway` network. | Tailscale ping check, VPN-005 |
| Headscale ACL | One grant per client computer the operator adds as an RDP or VNC device (R-061 pattern): `tag:hub` to `device-<name>` on tcp:3389 or tcp:5900. Policy tests change with it. Site devices need no change. | VPN-004 |
| VPS nftables | No change expected: forwarding from Docker bridges to `tailscale0` is already allowed. WP-23 proves it with a test. | R-050 |
| Site machines | One relay device per site with SSH and the wake program (D-2), set as the site's wake relay. | RA-031 |
| Test machines | A Windows machine with RDP and NLA; a Mac with Screen Sharing; one device with a web interface on an untrusted certificate. | Criteria 1, 3, 9 |

## 7. Open decisions

| # | Decision | Recommendation | Operator |
| --- | --- | --- | --- |
| D-1 | How CON-002 treats Phase 2 additions before the hub has them | `@pending` on each K1 addition; the test ignores it only while the hub lacks it; WP-34 requires none | No |
| D-2 | How Wake-on-LAN crosses sites (VPN-006) | A wake relay per site: the hub runs one static command over the SSH link on a site device, with the MAC rendered by the hub from parsed bytes. Rule that this meets CHK-012, because no operator text reaches the shell. The Phase 6 site agent replaces it | Yes (spec meaning of CHK-012, site machines) |
| D-3 | Docker and systemd checks without operator text in a command | Static listing commands (`docker ps -a --no-trunc --format '{{json .}}'`, `systemctl list-units --all --plain --no-legend --no-pager`); the hub finds the name in the output | No |
| D-4 | Tailscale ping from a hub in a container | The LocalAPI proxy of Section 6. Without it, the check type ships and reports Unknown with a message | Yes (VPS change) |
| D-5 | RDP server certificate pinning | The hub reads the certificate itself before the handshake (as R-034 does for SSH), records it on accept, and gives guacd `cert-fingerprints` with `ignore-cert` off. New end reason `SERVER_CERTIFICATE_MISMATCH` | No |
| D-6 | Copy and paste off for SSH | The client enforces them; the hub relays bytes and cannot. RDP and VNC are enforced in the hub relay and in guacd | No |
| D-7 | Deny list for sessions | Apply `ANIMA_PROBE_DENY` to every session target (P2-004). Without it, an RDP device at a hub address could make guacd reach the hub | No |
| D-8 | Where the live session list goes | A navigation bar item with the live count that opens a popover (Menu role) | No |
| D-9 | Web view when OD-7 fails on a platform | That platform opens the system browser and shows the pinned fingerprint beside the button (OD-7) | No |
| D-10 | RD Gateway and RemoteApp (spec 6.3.2) | Defer both to issue #1; keep them out of K1. The shared drive (RA-021) stays in Phase 2 | Yes (spec change) |
| D-11 | Phase 1 escalations E-1, E-3, E-4, E-5 | E-1: read criterion 7 as "2 intervals plus one probe timeout". E-3: use `ease-exit` on the outgoing layer, as the spec says. E-4: SET-002 lists the LOG-008 row. E-5: the Modal role drops blur (MOT-006 wins) | Yes (spec changes) |
| D-12 | Metric storage | One row per sample in daily partitions, minute, hour and day aggregates in PostgreSQL (OD-5). No TimescaleDB | No |
| D-13 | SNMP versions | v2c and v3 (authPriv, SHA-256 and AES-128); secrets sealed like HTTP check secrets | No |
| D-14 | SFTP transport | A ticketed WebSocket `/files` (K4 §9) on its own SSH connection, so the terminal tunnel (K4 §8) does not change | No |
| D-15 | Live session changes and metric samples on the bus | `sessions` and `metrics` each own a broadcast feed; `events::Bus` stays for logs, status and heartbeats | No |

## 8. Operator checkpoints for Phase 2

1. Approve the contract deltas and the decisions marked "Operator" (before Wave 1).
2. Each VPS, Headscale or Cloudflare change of Section 6.
3. Supply the test machines of Section 6 before WP-34.
4. Before the merge of `phase-2` into `main`.
