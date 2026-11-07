# Anima Agent Implementation Plan — Phase 3

| Item | Value |
| --- | --- |
| Document | Agent implementation plan, Phase 3 |
| Scope | Phase 3 of [design-spec.md](../design-spec.md) (Section 9): the web client, notifications, the Prometheus endpoint and API keys, session sharing and recording, the Settings page items. It also takes the Phase 2 leftovers in Section 9 of this plan |
| Platforms | macOS, Linux, Web |
| Base | `phase-3`, cut from `phase-2` at 04ab9b61 (Phase 2 complete, all gates pass) |
| Contract deltas | [phase-3-contracts.md](phase-3-contracts.md) |
| Version | 0.1 (proposed, for operator checkpoint 1) |
| Date | 2026-11-07 |
| Language | Simplified Technical English |

## 1. What stays from Phases 1 and 2

This plan uses these items without change:

- the Phase 1 plan ([implementation-plan.md](../implementation-plan.md)): roles and models (AGT-001 to AGT-005),
  loops (LOOP-001 to LOOP-004), contract rules (CON-001 to CON-004), shared-file rules (PKG-001 to PKG-003),
  gates (Section 7), files and communication (Section 8);
- the Phase 2 plan ([phase-2-plan.md](phase-2-plan.md)) rules P2-002 (`@pending`), P2-003 (one SSH path) and
  P2-004 (target rule and deny list on every hub connection to a device);
- every ruling R-001 to R-080 and CR-1 to CR-4 in [programme.md](programme.md). These apply in full:
  - R-066 (early start against the mock, two review rounds, scoped mutation testing, one motion audit, 7
    package orchestrators);
  - R-067 (merge each accepted task at once);
  - R-068 (client-only tasks run gates 4–12; one cargo run at a time per package);
  - R-072 (daily jobs go in `upkeep.rs`, not `main.rs`);
  - R-076 (no Windows machine: RDP checks wait for the final test).

The programme files stay programme files (PKG-001): `schema.rs`, `main.rs`, `lib.rs`, `upkeep.rs`,
`quality/hub-modules.json` and the face `<domain>.rs` of a domain that two packages share. Packages send
handoff notes, and the programme orchestrator applies them at integration.

Phase 3 changes these items:

- **P3-001.** The integration branch is `phase-3`. Package branches are `wp/NN-slug` from `phase-3`. The
  baseline gate uses `--base phase-3`.
- **P3-002.** Every K1 addition carries `@pending` until the hub declares it (D-1, as P2-002). The CON-002
  test already has the rule. WP-53 checks that no `@pending` remains.
- **P3-003.** Every new outbound connection of the hub (webhook, ntfy, SMTP, Apprise, RDAP) goes through one
  path in the leaf `targets`: resolve the name, check every address against the target rule (R-032) and the
  deny list `ANIMA_PROBE_DENY` (R-030), connect only to a checked address, and never resolve the name again.
  A redirect is refused, or checked again as a new target where this plan permits it (D-17, D-29). WP-41
  moves the resolver from `checks/probe/resolve.rs` into `targets` for this.
- **P3-004.** Every new secret is sealed with `credentials::Vault` and a fixed context (R-010): notification
  secrets and recording keys. Every new bearer value is 32 bytes from the OS random source, and the hub keeps
  only its SHA-256: API keys, browser tokens and share tokens. Share tokens and their hashes live only in
  memory.
- **P3-005.** The signed-out surface of the hub is a closed list (K7 "HTTP surface"): the GraphQL roots
  `login`, `signInBrowser` and `joinShare`; `/push/{token}`; the ticketed tunnels; the web client's static
  files; `/health`. `/metrics` needs a token. Every route stays behind the NET-002 layer.
- **P3-006.** No Windows machine (R-076). Sharing and recording are proved live over SSH and VNC (macOS Screen
  Sharing). RDP sharing and recording are proved with the fake guacd only. The Phase 2 operator steps O-1 to
  O-9 stay in [phase-2-acceptance.md](phase-2-acceptance.md). This plan does not plan them again.
- **P3-007.** `main.rs` is at its limit (R-072). New routers are mounted from `lib.rs` (handoffs). New
  background tasks are spawned from `upkeep.rs` (daily jobs) or from a new wiring module
  `crates/hub/src/spawns.rs` (long-running tasks: the notification dispatcher, the scheduler, the collector),
  listed under `wiring` in `quality/hub-modules.json`.

## 2. Contracts for Phase 3

| ID | Change | Where it goes |
| --- | --- | --- |
| K1 | Browser sign-in, API keys, notifications, sharing, recording, session limit, retention, check defaults, the domain expiry check type, event source `NOTIFICATION` | `contracts/schema.graphql`, with `@pending` (P3-002) |
| K2 | Ranges: access 0010–0029, devices 0120–0129, checks 0320–0349, events 0503, sessions 0660–0679, notifications 0850–0899, retention 0900–0919, trust 0920–0939 | `contracts/database.md` |
| K3 | New domains `notifications`, `retention`, `ssh`, `trust` (the last two from Q2-05); new leaves `api_errors`, `text`, `fingerprint`; `targets::resolve_checked`; `events::Trail`; `sessions::Serving`; new import edges | `contracts/hub-domains.md` |
| K4 | Session limit and recording flag at `startSession`; the guacd join; Section 10 the watch tunnel `/watch`; Section 11 recording; Section 12 the playback tunnel `/recording` | `contracts/tunnel.md` |
| K5 | No new semantic names. A `light` mode for the semantic colours. Five derived primitives if D-27 takes option B | `contracts/tokens.md` |
| K6 | `contracts/fixtures/phase3.json` | `contracts/fixtures/` |
| K7 | IDs `ApiKeyId`, `ChannelId`, `DeliveryId`, `ShareId`, `RecordingId`; variables `ANIMA_PUBLIC_ORIGIN`, `ANIMA_WEB_ROOT`, `ANIMA_RECORDING_ROOT`, `ANIMA_RECORDING_MAX_BYTES`; browser and API-key transport; the HTTP surface; the Prometheus series; never-log additions | `contracts/conventions.md` |

As in Phase 2, the deltas stay in [phase-3-contracts.md](phase-3-contracts.md) until the operator approves
them (checkpoint 1). Then WP-40 and the programme orchestrator copy them into `contracts/` in one commit, and
the file headers say "frozen for Phase 3".

## 3. Work packages

Each package owns its paths. The paths inside `crates/hub/src/sessions/` are split between WP-41 (the moves),
WP-47 and WP-48, one owner at a time. The paths inside `crates/hub/src/access/` belong to WP-43 only.

| ID | Package | Owned paths | Depends on | Builders | Reviewers | Spec IDs |
| --- | --- | --- | --- | --- | --- | --- |
| WP-40 | Phase 3 skeleton and contract gate | `crates/hub/tests/it/contract.rs`, `crates/hub/src/ids.rs`, `config.rs` (K7 variables), `quality/hub-modules.json` (new domains and edges), `contracts/fixtures/phase3.json`, the client mock's fixture loader | Operator approves the deltas | Sonnet, Haiku | Sonnet | CON-002, CON-003, REPO-004, K6, K7 |
| WP-41 | Hub groundwork (CQ-050 Phase 2 hub items) | New leaves `api_errors.rs`, `text.rs`, `fingerprint.rs`; `targets.rs` (`resolve_checked`); `events/trail/`; new domains `ssh/` and `trust/`, moved out of `sessions/`; `sessions/upgrade.rs` and the shared relay skeleton; deletion of `sessions/target.rs`; each domain's `graphql/error.rs` and `trail.rs` (moves only); migration 0920 | WP-40 (K3 split), early start allowed (R-066) | Opus (Q2-08, Q2-09, resolver), Sonnet (moves) | Opus (security), Sonnet | CQ-050, CQ-051, Q2-05 to Q2-11, Q2-14, P3-003 |
| WP-42 | Client groundwork and platform seam | `apps/client/src/lib/forms/` (moves), `lib/desktop/` (renames), `lib/web-interfaces/` (takes `lib/sessions/web/` and `lib/desktop/webView/`), `lib/hub/` import face, `lib/metrics/` (Q2-16), new `lib/platform/`, the casing rule in `quality/src/` (handoff), `apps/desktop/src-tauri/src/os_program.rs` (Q2-13), `deploy/localapi/allow.ts` and the two Phase 1 cast sites (Q2-15) | WP-40, early start allowed (moves only) | Sonnet, Haiku | Sonnet, Opus (platform seam) | Q2-12, Q2-13, Q2-15, Q2-16, CLI-001, UPD-001 |
| WP-43 | Hub access: browser sign-in, API keys, web client serving | `crates/hub/src/access/`, migrations 0010–0029, new wiring module `crates/hub/src/web_client.rs` (static files and headers; mount is a `lib.rs` handoff) | WP-40; WP-41 T1 (`api_errors`) | Opus | Opus (security), Sonnet | Section 1.3 (Web), SEC-001, SEC-002, API-003, INT-003, NET-002, D-2, D-3, D-4, D-23 |
| WP-44 | Client web build | `apps/client/src/routes/sign-in/`, `lib/access/`, `lib/platform/browser.ts`, `lib/web-interfaces/open/` (WEB-011), `apps/client/svelte.config.js` (CSP hashes), browser test projects for Chromium, WebKit and Firefox | WP-42; WP-43 (mock first, CON-003) | Sonnet, Opus (sign-in) | Opus (security), Sonnet (interface) | CLI-001, WEB-011, UPD-001, LOG-008, MV-001, D-5, D-6 |
| WP-45 | Hub notifications | `crates/hub/src/notifications/`, migrations 0850–0899, events migration 0503 (handoff), and three reviewed handoff edits in `checks/` (R-071 pattern): `observe` in the heartbeat transaction, `adopt_defaults` in `createCheck`, the `Check.notifications` resolver | WP-41 T1–T4 | Opus (dispatcher, target rule, sealing), Sonnet (channel encoders, templates) | Opus (security), Sonnet | NOT-001 to NOT-007, Section 6.5, SEC-003, SEC-005, LOG-005, LOG-007, P3-003, D-16 to D-22 |
| WP-46 | Hub checks, retention and Prometheus | `crates/hub/src/checks/` (with the new `checks/exposition/` and `checks/probe/rdap.rs`), new domain `crates/hub/src/retention/`, migrations 0320–0349 and 0900–0919, retention parameters in `events/` (handoff for `metrics/` and `sessions/recording/`), `upkeep.rs` (handoff) | WP-41; WP-43 `bearer_layer` for the `/metrics` mount | Sonnet, Opus (RDAP target rule, exposition load) | Sonnet, Opus (security) | INT-001, INT-002, HUB-005, DB-005, DB-009, Section 6.4 (domain expiry), CHK-009, Section 7 (retention, default check settings), D-24, D-25, D-26, D-29 |
| WP-47 | Hub session sharing and session limit | `sessions/share/`, `sessions/start.rs` (limit), the fan-out parts of `sessions/tunnel/` and `sessions/terminal/`, `sessions/graphql/` share fields, `crates/hub/src/devices/` (`session_limit`, `record_sessions`), migrations 0120–0129 | WP-41 T5 to T7 (split, relay skeleton) | Opus | Opus (security), Sonnet | RA-035, RA-036, GW-014, SEC-002 (D-7), D-7 to D-10, K4 §10 |
| WP-48 | Hub session recording | `sessions/recording/` (writer, store, playback route), the tee in `sessions/tunnel/relaying/` and `sessions/terminal/relay.rs` (after WP-47 merges, R-027), migrations 0660–0679 | WP-41 T5 to T7; WP-47 T1 (devices flag) | Opus | Opus (security), Sonnet | RA-008, RA-040, RA-041, SEC-003, SEC-004, LOG-007, D-11 to D-15, K4 §11, §12 |
| WP-49 | Client sharing, recording and watch page | `apps/client/src/lib/sessions/share/`, `lib/sessions/recording/` (both players), `lib/sessions/watch/`, route `apps/client/src/routes/shared/`, the Recording badge in `lib/sessions/view/`, playback rows in `routes/devices/[id]/detail/` sessions tab (handoff) | WP-40 mock; WP-44 signed-out route guard | Opus (players, watch tunnel), Sonnet | Opus (security), Sonnet (interface) | RA-035, RA-040, RA-041, MAT-008, MOT session tab open, RA-034 (H-2 `clientAddr`) |
| WP-50 | Client settings, notifications and light theme | `apps/client/src/routes/settings/`, new `lib/notifications/`, `lib/api-keys/`, `lib/retention/`, `lib/theme/`, `lib/forms/check/` (channel editor, domain expiry editor, defaults pre-fill), `lib/forms/device/` (session limit, recording), `design/tokens/` light values and the generated outputs | WP-40 mock; WP-42 moves | Sonnet, Opus (theme switch, secret fields) | Sonnet (interface), Opus (security) | SET-001, Section 7, UI-005, TOK-011, MOT-043, NOT-003, NOT-004, NOT-007, INT-003, RA-036, RA-040, D-27, D-28 |
| WP-51 | Hub GPU metrics | `crates/hub/src/metrics/` | WP-41 T5 (`ssh` domain) | Opus (discovery), Sonnet (parsers) | Sonnet, Opus (security) | MET-006 to MET-009, MET-002, CHK-012, R-073, D-30 |
| WP-52 | Deploy and infrastructure | `deploy/` (compose, hub Dockerfile web stage, optional Apprise service, deny list, backup exclusion, README), `deploy/headscale/` (only if D-8 or D-24 needs a grant), `docs/setup/` | WP-40, early start allowed | Sonnet, Haiku | Opus (security) | Section 6 of this plan, VPS-005, VPS-006, DNS-008, DB-007, R-031 |
| WP-53 | Phase 3 integration and acceptance | `docs/research/`, acceptance tests, `docs/programme/phase-3-acceptance.md` | All | Sonnet | Opus | Section 5 of this plan, spec Section 5.8, CQ-050 |

Package notes:

- **WP-40** adds `EventSource.NOTIFICATION` to the fixtures and registers the five new IDs. It updates
  `quality/hub-modules.json` twice: once for `notifications` and `retention`, and once, as a handoff from
  WP-41, for `ssh` and `trust`.
- **WP-41** runs its tasks in this order, and each merges under R-067:
  - T1 Q2-06 `api_errors` leaf;
  - T2 Q2-07 `events::Trail`, with the web interface `device` field;
  - T3 Q2-11 `text` leaf;
  - T4 Q2-09 and the resolver move into `targets` (security, mutation run);
  - T5 Q2-05a, the `ssh` domain;
  - T6 Q2-05b, the `trust` domain and migration 0920;
  - T7 Q2-08, the relay skeleton and the `Quiet` timer (security, mutation run);
  - T8 Q2-10 fingerprint leaf and Q2-14 seal fallbacks.

  A Wave 1 hub package that edits a domain's `graphql/` folder starts after T1 and T2 merge. WP-47 and WP-48
  start after T5 to T7 merge. WP-41 adds no behaviour: the import gate and the existing tests prove each move.
- **WP-42** delivers `lib/platform/`: one module that says what this client can do (`keyStore`, `webViewTabs`,
  `popOut`, `updates`, `processLog`, `thisMachine`, `systemSettings`). The desktop answers come from
  `lib/desktop/`; the browser answers are all "no". Every desktop-only control reads `lib/platform/`, never
  `isTauri()` itself. `updater.ts` is loaded with a dynamic import behind it, so the web client never loads the
  updater (UPD-001).
- **WP-43** task 1 serves `ANIMA_WEB_ROOT` with the headers in K7. Task 2 is browser sign-in (cookie, Origin
  check, WebSocket cookie). Task 3 is API keys and the token-kind rules in the guard. Task 4 is
  `bearer_layer` for routes outside GraphQL. `CLIENT_ORIGINS` in `lib.rs` does not change: the web client
  has the hub's own origin and needs no CORS.
- **WP-45** task 1 proves two things with tests before any channel code: reqwest connects only to a checked
  address and follows no redirect; lettre connects to a checked address with TLS and the host name as SNI.
- **WP-46** moves the checks retention out of `checks/scheduler/daily.rs` into `upkeep.rs` (R-072). Every
  retention job then takes its period as a parameter.
- **WP-47** task 1 records guacd 1.6.0's join handshake: `select $<connection id>`, the `args` it returns, and
  the effect of `read-only` (as WP-22 did for D-5). WP-47 owns the devices columns for both the limit and the
  recording flag, so `devices/` has one owner.
- **WP-48** task 1 proves that the vendored `guacamole-common-js` 1.5.5 `Guacamole.SessionRecording` plays a
  stream that the hub teed, with seek. The vendor file does not change (R-045, R-046).
- **WP-49** builds the watch page as a page with no navigation bar and no GraphQL call except `joinShare`.
- **WP-50** task 1 is a contrast spike for the light theme. Its result goes to the programme orchestrator for
  D-27 before any light value merges.
- **WP-52** adds a client build stage to the hub image. The release build still builds only
  `--bin anima-hub` (R-031).

## 4. Waves and critical path

```text
Wave 0 (serial)    WP-40 skeleton + contract freeze (operator checkpoint 1)
                   early start (R-066): WP-41  WP-42  WP-52
                       |
Wave 1             WP-41 T1..T8 (merge per task)   WP-42   WP-43   WP-46   WP-51
                       |
Wave 2             WP-45     WP-47 --> WP-48     WP-44     WP-50
                       |
Wave 3             WP-49
                       |
Wave 4 (serial)    WP-53 integration and acceptance
```

- **P3-WAVE-001.** WAVE-002 limits as R-066: 7 package orchestrators, 3 builders and 3 reviewers per package.
  WP-41 gets the first slot, because three critical paths go through it.
- **P3-WAVE-002.** Critical paths:
  - Sharing and recording (longest): WP-40 → WP-41 (T5 to T7) → WP-47 → WP-48 → WP-49 → WP-53.
  - Web client: WP-40 → WP-42 → WP-44 → WP-53. WP-43 (hub) and WP-52 (image) run beside it. WP-49's watch
    page needs WP-44's signed-out route.
  - Notifications: WP-40 → WP-41 (T1 to T4) → WP-45 → WP-50 (the interface on the real hub) → WP-53.
- **P3-WAVE-003.** WP-44, WP-49 and WP-50 start against the mock from the deltas (CON-003, R-066). Each
  rebases onto `phase-3` when its hub package merges.

## 5. Phase 3 acceptance criteria

Drawn from spec Sections 1.3, 6.3.3, 6.4, 6.6, 6.8, 6.9 and 7. "Operator" means a live step that only the
operator can do.

| # | Criterion | Spec IDs | Operator |
| --- | --- | --- | --- |
| 1 | The hub serves the web client at `https://hub.anima.ing/`. The operator signs in with password and TOTP in Safari and Chrome on macOS and in Firefox or Chromium on Linux. The browser holds only an `HttpOnly` cookie: no token is in script-readable storage. | Section 1.3, CLI-001, SEC-001, SEC-002, D-2, D-3 | Yes: browsers on VPN machines |
| 2 | In the web client the device list, the detail view, live status, the logs panel, and SSH and VNC sessions work. The desktop-only controls (updates, log folder, web view tab, pop-out, This Mac, System Settings) do not show, and the updater is never loaded. | CLI-001, UPD-001, LOG-008, RA-001, D-5 | Yes, for the live sessions |
| 3 | In the web client, the Web button opens a new browser tab straight to the device, with `noopener` and `noreferrer`. The pinned fingerprint shows beside the button. | WEB-011, D-5 | No |
| 4 | A cookie-carrying `POST /graphql` or WebSocket upgrade from another origin is refused. The cookie has `__Host-`, `Secure`, `HttpOnly` and `SameSite=Strict`. Sign out clears it and deletes the token. The static files carry the K7 headers. `CLIENT_ORIGINS` is unchanged and the desktop app still signs in. | SEC-002, API-003, D-3, D-4 | No |
| 5 | The hub sends a notification when a check goes Down and when it comes back Up, through webhook, ntfy, email and Apprise. It sends none for a check in maintenance, and no "Up" after Maintenance unless "Down" was sent. | NOT-001, NOT-002, Section 6.5 | Yes: real endpoints (synthetic in tests) |
| 6 | The operator selects channels per check. A channel marked "on by default" is on for each new check. | NOT-003 | No |
| 7 | "Send test" on each channel returns a delivery result within 8 seconds. | NOT-004 | Yes, for each real channel |
| 8 | With a resend interval of N, a check that stays Down sends again after each N Down heartbeats. | NOT-005 | No |
| 9 | The hub sends one expiry notification per threshold (default 21, 14 and 7 days) for each TLS certificate, and for each domain expiry check (D-22). | NOT-006, CHK-007 | No |
| 10 | A template changes the title and body. Only the fixed placeholders are accepted. Values are escaped for the channel: JSON for webhooks, no line breaks in an email subject or a header. | NOT-007, D-21 | No |
| 11 | Every notification connection passes the target rule and the deny list for each resolved address, follows no redirect, and verifies TLS. A channel pointing at a deny-listed address fails with a fixed message. Channel secrets are sealed, never returned, and never logged. | P3-003, SEC-003, SEC-005, LOG-007, D-17 | No |
| 12 | `GET /metrics` gives the Prometheus text format with status, response time, uptime ratio and certificate days for each check. Labels hold no value that changes often. It refuses a request with no token. | INT-001, INT-002, D-24 | Optional: a real Prometheus scrape |
| 13 | The operator makes an API key with a scope and an expiry date. The secret shows once, and the hub keeps only its SHA-256. A read key cannot run a mutation. No API key can manage keys, sign in a browser, start a session, open files, share or play a recording. Revoke stops the key at once. Each step has an audit entry. | INT-003, LOG-005, D-23 | No |
| 14 | The operator makes a share link for a live SSH session and a live VNC session. On a second machine, a browser opens the link without sign-in and shows the session read-only. The link stops at expiry, at "Stop sharing", and when the owner's session ends. Each create, join and end has an audit entry. | RA-035, D-7 to D-10 | Yes: a second Mac on the VPN, Screen Sharing on |
| 15 | With a session limit of N on a device, session N+1 is refused with CONFLICT. Viewers and file tunnels do not count. | RA-036 | No |
| 16 | Recording is off by default and set per device. During a recorded session the owner and each viewer see a "Recording" badge. The recording volume holds no plaintext, and a recording file cannot be read without its key in the database. Only output is recorded, never keystrokes. | RA-040, RA-008, D-11, D-13 | Yes, for VNC (synthetic in tests) |
| 17 | The client plays an RDP or VNC recording with `Guacamole.SessionRecording`, and an SSH recording with the asciicast player. Each has play, pause and seek. A recording that did not end (hub crash) plays up to its last verified part and says so. | RA-041, D-12 | Yes, for a real VNC recording |
| 18 | An expired recording is deleted with its key. "Delete" removes the key first, so the file cannot be read again. | RA-040, D-14, D-15 | No |
| 19 | The Settings page has: the hub address and connection test (desktop), the theme (dark, light, system), data retention, notification channels, API keys, default check settings, and in-app reduced motion. New checks use the defaults. A changed retention period applies at the next daily run. | SET-001, Section 7, UI-005, TOK-011, HUB-005, MOT-043, D-25 to D-28 | No |
| 20 | A domain expiry check reads RDAP and reports the days left. It is Degraded below its warning days and Down after expiry. Its connections obey P3-003. | Section 6.4, CHK-009, D-29 | Yes, for one live RDAP query |
| 21 | The hub finds the GPUs of a device at the first collection, every 24 hours and after a restart, and collects GPU metrics (NVIDIA, AMD; Apple without VRAM). | MET-006 to MET-009, R-073, D-30 | Yes: an NVIDIA or AMD device |
| 22 | No log contains a password, key, token, cookie, API key, share token, notification secret, webhook URL, recording key or session content. | LOG-007, SEC-009 | No |
| 23 | Each motion role that Phase 3 uses (theme change, toast, menu, modal, session tab open) has a reduced-motion form. The Phase 3 audit runs in WebKit and Chromium on the desktop and web clients, in both themes. It has no open "layout jump" or "wrong motion". Nothing moves or blurs over a live display, on the watch page or in playback. | MOT-040, MOT-041, MOT-043, Section 5.8, MV-001, MAT-008 | No |
| 24 | Gates pass 12/12, the baseline is empty, K1 has no `@pending`, and Q2-05 to Q2-16 are closed or on issue #1 with a reason. | CQ-010, CON-002, CQ-050 | No |

## 6. Infrastructure changes

Each change on the VPS, Headscale or Cloudflare needs the operator's confirmation in chat (VPS-005, DNS-008).
WP-52 delivers the files and steps. The programme orchestrator asks before it applies them and logs each one in
[operator-changes.md](operator-changes.md).

| Item | Change | Why |
| --- | --- | --- |
| Hub image | A bun stage builds `apps/client` and copies `build/` to `/usr/share/anima/web`. The binary stage is unchanged (R-031). Built on the Mac for amd64, as in Phase 2. | D-2, Section 1.3 |
| Compose, hub | New variables `ANIMA_PUBLIC_ORIGIN=https://hub.anima.ing`, `ANIMA_WEB_ROOT=/usr/share/anima/web`, `ANIMA_RECORDING_ROOT=/var/lib/anima/recordings`, `ANIMA_RECORDING_MAX_BYTES`. A named volume `recordings`, read-write at that path, owned by uid 10001, mode 0700. The root file system stays read-only. | K7, D-13 |
| Compose, deny list | `ANIMA_PROBE_DENY` adds `${ANIMA_HUB_ADDRESS}/32` (the hub's own VPN address). Notifications and RDAP use the same list (P3-003). | R-030, P3-003 |
| Compose, Apprise (D-19) | Optional service `apprise` (pinned `caronc/apprise` tag, stateless mode, no config volume, read-only, `cap_drop: ALL`) on a new network `notify` (172.30.3.0/24) that has a route out. The hub joins `notify`. No published port. | NOT-002, D-19 |
| Egress | No firewall change: the hub's `gateway` network already has a route out, and the `forward` chain accepts it. WP-52 proves with a test from the hub container that ports 443, 465 and 587 reach the internet, because some providers block outbound mail ports. | NOT-002 |
| CORS | No change. `CLIENT_ORIGINS` stays the two Tauri origins. The web client is same-origin, so it needs no entry. Adding the web origin would only widen the rule. | D-2 |
| Cloudflare and DNS | No change. `hub.anima.ing` stays an unproxied A record to the VPN address (DNS-004, DNS-005). No public record is made for the web client. No mail records are needed if D-20 is accepted. | SEC-008, D-20 |
| Headscale ACL | No change for the web client: `operator@` already reaches `tag:hub` on tcp:443. A Prometheus server on a tagged machine needs one grant to `tag:hub` tcp:443 (D-24). Share viewers on a guest user are not in Phase 3 (D-8). | VPN-004 |
| Prometheus | Scrapes `https://hub.anima.ing/metrics` over the VPN with a read API key in `authorization.credentials_file`. Phase 3 does not deploy a Prometheus server. | INT-001, D-24 |
| Backups | `pg_dump` takes the new tables (sealed secrets, sealed recording keys). The `recordings` volume is not in the daily off-site copy (D-15). `deploy/backup/README.md` says so. | VPS-006, DB-007 |
| Disk | Before the first recording, the operator confirms the free space on the VPS. Retention (30 days) and the per-recording limit bound the volume. | D-14 |
| Test machines and accounts | A second Mac on the VPN (share viewer); Safari, Chrome, and Firefox or Chromium on Linux; an SMTP mailbox with an app password; the ntfy app with a topic; one webhook receiver (for example Discord); a device with an NVIDIA or AMD GPU; optional Prometheus. The operator types every real secret into Settings. Agents use synthetic endpoints only. | Criteria 1, 5, 7, 14, 21 |

## 7. Open decisions

| # | Decision | Recommendation | Operator |
| --- | --- | --- | --- |
| D-1 | How CON-002 treats Phase 3 additions | `@pending` as in Phase 2 (P3-002) | No |
| D-2 | Where the web client is served | The hub serves the static build at its own origin (spec 1.3), from `ANIMA_WEB_ROOT`; an empty value turns the web client off. Explicit routes (`/graphql`, the tunnels, `/push/`, `/metrics`, `/health`) win over the static fallback. Not Cloudflare Pages: a public origin would need CORS to a VPN-only hub and would publish the app outside the network (SEC-008) | Yes (image and compose change) |
| D-3 | How the browser signs in. CLI-003 assumes an OS key store, and a browser has none | An `HttpOnly` cookie `__Host-anima` (`Secure`, `SameSite=Strict`, `Path=/`), holding a browser token stored as SHA-256 in `access.api_tokens` with kind `browser`. Lifetime 7 days, with no refresh. The hub accepts the cookie only when `Origin` equals `ANIMA_PUBLIC_ORIGIN`, on HTTP and on the WebSocket upgrade, because device names under `net.anima.ing` are same-site and `SameSite` does not stop them. Rejected: a bearer token in `sessionStorage` (any script injection reads it); memory only (TOTP on every reload). Spec: CLI-003 gains "the web client keeps an `HttpOnly` cookie" | Yes (spec change) |
| D-4 | The browser sign-in operation | A separate mutation `signInBrowser(input: LoginInput!): BrowserSignIn!`, which sets the cookie and never returns the token. It shares the login throttle and TOTP. The API-003 backstop then permits signed-out documents that are one `login`, one `signInBrowser`, or one `joinShare` (D-7). `logout` also clears the cookie | Yes (API-003 reading) |
| D-5 | What the web client leaves out | No pop-out (RA-002 is "on desktop"), no web view tab (WEB-011 instead), no updates (UPD-001), no log folder (LOG-008), no This Mac (R-060), no System Settings button (R-059, R-079 keeps its port test). No hub address field: the web client talks only to the hub that served it | Yes (spec note on RA-002) |
| D-6 | Browsers supported by the web client | The two latest major versions of Safari, Chrome, Edge and Firefox. The motion audit uses WebKit and Chromium (MV-001); Firefox gets a smoke test of sign-in, list and one session | No |
| D-7 | What "sharing a session" means (spec silent) | A share is a link to one live session, `https://hub.anima.ing/shared#<token>`. The token is only in the URL fragment, so it never reaches the server, a log or a `Referer`. The viewer page in the web client sends it once in `joinShare(token, name, display)` and gets a single-use watch ticket (K4 §1 properties) for `/watch`. The viewer is not an operator: it can call nothing else. Spec: SEC-002 gains this exception, as for the push token | Yes (spec change) |
| D-8 | Who may open a share link | Phase 3: only machines already on the VPN with hub access (`operator@` machines, for example the second Mac). A `guest@` Headscale user with only `tag:hub:443` and a hub-side "viewer networks" rule go to issue #1. That keeps the hub's sign-in and API out of a guest's reach | Yes |
| D-9 | Share access and limits | `VIEW` by default; `CONTROL` only after a confirm. A viewer never gets clipboard, file, audio or `argv`, whatever the device policy says. Expiry 5 minutes to 24 hours, default 60 minutes. At most 3 viewers per share (setting up to 10) and 32 shares per hub. The owner sees the viewers and can stop the share | No |
| D-10 | How a late viewer of an SSH session sees the screen | While a share is active, the hub keeps the last 64 KiB of terminal output in memory and sends it to each new viewer first. It is freed when the share ends. It is never written, logged or recorded. Rule that this meets RA-008, because nothing is kept after the share. Without it, a viewer sees a blank screen until the next output | Yes (RA-008 reading) |
| D-11 | How recording is done | The hub tees the stream it already relays: for RDP and VNC, every gateway instruction it sends to the owner; for SSH, the terminal output. Client input is never recorded. guacd `recording-path` stays empty, because guacd writes plaintext to disk and cannot encrypt (RA-040) | No |
| D-12 | The SSH recording format | asciicast v2, played with xterm.js in a small player. RDP and VNC keep the Guacamole format and `Guacamole.SessionRecording`. R-062 named this change. Spec: RA-041 gains "SSH recordings are asciicast v2, played with xterm.js" | Yes (spec change) |
| D-13 | Where recordings are kept, and how they are encrypted | Files in the `recordings` volume, `<recording_id>.anrec`, with XChaCha20-Poly1305 STREAM in 64 KiB parts (K4 §11). Each recording has its own random key, sealed by the vault with context `sessions.recordings:<recording_id>` and kept in `sessions.recordings`. A file without its database row cannot be read; deleting the row destroys the recording. Not PostgreSQL: recordings would swell the database and the daily `pg_dump` | No |
| D-14 | Recording limits and failure | Retention 30 days (setting 1 to 365). At most `ANIMA_RECORDING_MAX_BYTES` per recording (default 2 GiB). Below 1 GiB free space the recording stops. A stop does not end the session: the badge becomes "Recording stopped" and the hub writes a Warning event. If recording is on but the hub cannot record at start, `startSession` refuses with UNAVAILABLE, because the operator asked for a record | Yes |
| D-15 | Recordings in backups, and after device deletion | Not in the off-site backup (size, and the database copy holds the keys). After a device is deleted, its recordings stay until retention ends, like session history. The operator can delete one at any time | Yes |
| D-16 | How notifications learn of status changes | A transactional outbox. `checks` calls `notifications::observe` inside the heartbeat transaction. The pure core in `notifications` decides (transition, resend, maintenance, expiry thresholds) and writes outbox rows. A dispatcher task sends them with retries (1, 5, 15, 60 minutes; then it gives up with a Warning event). Not the event bus: it drops messages when a receiver lags. The scheduler still does not notify (spec 12.1) | No |
| D-17 | Channel secrets and egress | Sealed with context `notifications.channels:<channel_id>`: the whole webhook URL (Discord and Slack keep the credential in the path), webhook header values, the ntfy topic and token, the SMTP password, the Apprise URLs or key. The API returns only the origin or host. Every connection obeys P3-003. Webhook: no redirects, TLS always verified (no "ignore TLS" option), 8 s limit, reply body read up to 4 KiB and discarded. SMTP: implicit TLS (465) or STARTTLS (587) only; no plaintext login | No |
| D-18 | Discord and Telegram (spec Section 7 names them) | Through Apprise, or a webhook with a JSON template. No separate channel kinds; NOT-002 names four | No |
| D-19 | Where the Apprise API runs | The optional compose service of Section 6, stateless mode. If the operator declines, the Apprise channel kind still ships and points at any Apprise API the operator runs | Yes (VPS change) |
| D-20 | SMTP account and sender | The operator's own mailbox with an app password, entered in Settings by the operator. The sender is that mailbox, so `anima.ing` needs no SPF or DKIM records | Yes |
| D-21 | Templates | Placeholder substitution only, no logic: `{device}`, `{check}`, `{status}`, `{previous}`, `{message}`, `{at}`, `{days}`. An unknown placeholder is a VALIDATION error at save. One default template per installation; a channel can override it. Values are cleaned with the `text` leaf (Q2-11) and escaped per channel | No |
| D-22 | Expiry notifications | One global list of thresholds (default 21, 14 and 7 days), one notification per threshold per certificate (keyed by `notAfter`). The same thresholds apply to domain expiry checks. Spec: NOT-006 gains domain expiry | Yes (spec addition) |
| D-23 | API key rules | Format `anima_` plus 43 base64url characters, so secret scanners can find a leaked key. SHA-256 at rest (the D-7 practice for 256-bit random tokens; no slow hash needed). Scope READ or WRITE; an expiry is required and is at most one year. API keys cannot manage API keys, call `signInBrowser`, `startSession`, `openFiles`, `shareSession`, `joinShare` or `openRecording`, or set or clear any device credential. Revoke deletes the row | Yes (INT-003 reading) |
| D-24 | Prometheus authentication, labels and scraper | `/metrics` takes `Authorization: Bearer` with any valid token of READ scope or higher (an API key in practice). Labels: `check_id`, `check_name`, `check_type`, `device_id`, `device_name`, `site`, and `window` with three fixed values. Series in K7. The result is cached for 10 s, so a key holder cannot load the database. The scraper is the operator's own; if it is not an `operator@` machine, VPN-004 needs a change and Headscale needs a grant | Yes, if a grant is needed |
| D-25 | Data retention settings | A `retention` domain owns the periods. `upkeep.rs` reads them daily and passes each to its domain's job. Settable: raw heartbeats and raw metrics (1 to 400 days, default 30), event log (30 to 365, default 90), recordings (1 to 365, default 30), notification deliveries (7 to 365, default 30). Fixed: the aggregates (DB-009) and the audit log (1 year, LOG-006) | Yes (the bounds) |
| D-26 | Default check settings | `checks.defaults`, one row with the schema defaults (DB-011). The client pre-fills the check form from `checkDefaults`. K1 input defaults stay. The hub's default ICMP or TCP check uses the stored defaults | No |
| D-27 | The light theme | The Nord Snow Storm colours (UI-005). Nord aurora on Snow Storm is below 3:1 for status indicators (for example `nord-13` on `nord-6`), so WP-50 task 1 measures it. Option A: Nord values only; every status already has a label (UI-003). Option B (recommended): five derived primitives, each a Nord aurora colour mixed toward `nord-0` until it reaches 3:1 on `nord-6`, used only in light mode. Default theme stays dark; the choice is Dark, Light or System, saved on the device | Yes (UI-001 or UI-005 change) |
| D-28 | In-app reduced motion (MOT-043) | Build it now in the Settings page: "Reduce motion: System, On, Off", saved on the device and read in `lib/motion/` (MOT-041). It costs one row | No |
| D-29 | The domain expiry check (spec 6.4, "Phase 3") | Type `domain_expiry` (CHK-009). RDAP through the IANA bootstrap file (cached 24 hours), HTTPS only, at most 3 redirects, each checked again under P3-003. Interval at least 1 hour; the client proposes 12 hours. Degraded below `warnDays` (default 30), Down after expiry, Unknown when RDAP gives no expiry date | No |
| D-30 | GPU scope (R-073) | NVIDIA (`nvidia-smi` query) and AMD (`/sys/class/drm` files) with VRAM; Apple silicon usage from `ioreg`, with no VRAM (MET-007); Intel usage best effort with a 2 s limit, and Unknown when not permitted. Each is a `FixedCommand` (CHK-012). MET-010 waits for Proxmox (Phase 4) | No |
| D-31 | The names for the Q2-05 split | `ssh/` (link, fixed commands, probe, public key) and `trust/` (host keys, server certificates, their accept mutations). Migration 0920 moves the two tables to schema `trust` with `ALTER TABLE … SET SCHEMA` (metadata only; grants and the R-040 revocation move with them), so the schema-ownership grep gate stays true | No |
| D-32 | Which Phase 2 non-SPEC items Phase 3 takes | Q2-05 to Q2-11 and Q2-14 in WP-41; Q2-12, Q2-13, Q2-15 and Q2-16 in WP-42. They come first, because the new domains and tunnels would otherwise add an eighth error copy, a fourth relay copy and a third target rule. Each one leaves issue #1 when it merges | No |

## 8. Operator checkpoints for Phase 3

1. Approve the contract deltas and the decisions marked "Operator" (before Wave 1).
2. Each VPS, Headscale or Cloudflare change in Section 6: the hub image with the web client, the compose
   variables and the `recordings` volume, the deny list, the optional Apprise service, any Headscale grant.
3. Each spec change from a ruling: CLI-003 (D-3), API-003 (D-4), RA-002 note (D-5), SEC-002 (D-7), RA-008
   reading (D-10), RA-041 (D-12), NOT-006 (D-22), INT-003 (D-23), VPN-004 if needed (D-24), UI-001 or UI-005
   (D-27).
4. Before WP-53: supply the test machines and accounts in Section 6. The operator types each real secret
   (SMTP password, ntfy token, webhook URL, Apprise URLs) into Settings personally. Agents never type a real
   secret.
5. Before the merge of `phase-3` into `main`. If `phase-2` is not yet in `main`, it merges first (Phase 2
   checkpoint 4).

## 9. Phase 2 items that Phase 3 takes

| Item | Source | Package |
| --- | --- | --- |
| GPU discovery and GPU metrics (MET-006 to MET-009); a test that a deny-listed device gets no samples | R-073, WP-28 coulds, WP-34 criterion 11 | WP-51 |
| `sessions/` split into `ssh/` and `trust/` | Q2-05 | WP-41 T5, T6 |
| One error leaf | Q2-06 | WP-41 T1 |
| One audit trail; the web interface `device` field; `AuditTarget::WebInterface` | Q2-07, WP-30 coulds | WP-41 T2 |
| One relay skeleton and quiet timer (hub side) | Q2-08, WP-27 and WP-32 coulds | WP-41 T7 |
| One target rule; the SSH session check reads `ANIMA_PROBE_DENY` | Q2-09, WP-23 could | WP-41 T4 |
| One certificate fingerprint type | Q2-10 | WP-41 T8 |
| One invisible-text rule | Q2-11 | WP-41 T3 |
| Seal fallbacks; the two seal files merged | Q2-14, WP-29 could | WP-41 T8 |
| Client moves, file casing rule, hub import face | Q2-12 | WP-42 |
| One OS-program runner in the shell | Q2-13 | WP-42 |
| Tuple casts | Q2-15 | WP-42 |
| `lib/metrics/charts.ts` rename | Q2-16 | WP-42 |
| Show `Session.clientAddr` in the Sessions tab | WP-33 H-2 | WP-49 |
