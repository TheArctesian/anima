# WP-17 report — Integration and acceptance

Package branch `claude/wp-17-acceptance` from `phase-1` at bb86ed8. Head: see the commit that adds this file (its parent f0312ed passed `gates: 12/12 pass`, `env -u DATABASE_URL BASE_REF=bb86ed8 bun run gates`).
Plan, decisions and review log: [WP-17.md](WP-17.md).

## Section 9.1 acceptance criteria

Status: **met** (evidence in the repository), **met, operator to confirm** (tests pass; one live step only the
operator can do), **blocked** (needs an operator action first).

| # | Criterion (short) | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Hub stack starts from compose on the VPS | met | `deploy/test/compose.test.ts:139,149`; live first start recorded in `docs/programme/operator-changes.md` line 30 (2026-11-05 05:20Z). |
| 2 | VPN reaches devices at both sites; hub to device | met in part; **blocked** for site to site | Policy tests `deploy/headscale/tests/policy.test.ts` (ROUTER_ROWS at :28-42); hub to device SSH with a fake server `crates/hub/src/sessions/terminal/tests/wire/session.rs:43`; live TCP from the VPS to the Mac on port 22 (operator-changes line 33). Site to site needs a subnet router enrolled. |
| 3 | Desktop app builds and opens on macOS and Linux | met, operator to confirm (Linux open) | macOS builds. Linux: Ubuntu 24.04 container with the release job's apt packages, Rust 1.99.1 aarch64: `cargo build -p anima-desktop` passes, `cargo test -p anima-desktop` 49 passed, `cargo clippy --all-targets -D warnings` clean (T6). Opening the window on a Linux desktop is not checkable here. |
| 4 | Navigation shell | met | `apps/client/src/lib/nav/shell.test.ts:86,110`. |
| 5 | Settings page | met | Minimal page per SET-002 (R-048). See escalation E-4 (SET-002 against LOG-008). |
| 6 | Add, edit, delete a device | met | Hub and client tests; T7 `crates/hub/tests/it/acceptance.rs` `dev_015_deleting_a_device_removes_its_whole_chain_and_keeps_its_history` (uptime counted by check across the three rollups; heartbeats after `delete_orphan_heartbeats`). |
| 7 | A stopped device shows Down within 2 intervals | met for a refused connection; **spec conflict** for a silent device | `checks/scheduler/tests/retries.rs` `chk_002_a_stopped_device_shows_down_within_2_intervals_with_the_defaults`: Down at exactly 120 s. `decisions.rs` `chk_002_with_the_defaults_...`: a silent device is Down at 122 s (ICMP) or 125 s (TCP), because the last retry waits for its probe timeout. See E-1. |
| 8 | SSH in a terminal from a card | met, operator to confirm | Card SSH test `apps/client/src/lib/devices/screen/page.test.ts:195`; hub terminal tests (WP-18). The SSH tab has not been tried against the real hub (operator-changes line 40). |
| 9 | Design tokens only | met | TOK-003 lint gate passes (`bun run --cwd apps/client lint`). |
| 10 | Live updates without reload; only the expected connections | met, operator to confirm | Inspection: only the GraphQL HTTP request, the graphql-ws socket and the K4 terminal socket; subscription tested `apps/client/src/lib/devices/screen/screen.test.ts:99`. Live check against the real hub is the operator's. |
| 11 | Audit trail of device changes | met | `crates/hub/src/devices/trail.rs:75,89,100`. |
| 12 | No credential in any log (LOG-007) | met | T4 security audit: 31 tracing sites read, redaction mutations M1 to M4 each fail a test; F-1 gap closed by F4 (`lib/sessions/tunnel/terminal/link.test.ts`). |
| 13 | Token gate | met | Same TOK-003 gate as criterion 9; 0 raw values. |
| 14 | Reduced motion for every role; no layout jump or wrong motion | met, with open items named (D-13, D-14) | Every Phase 1 role has a reduced-motion form (MOT-040, MOT-041). Every "layout jump" and "wrong motion" finding a WP-17 fix could close is closed: first audit M-1 to M-3 and the wrong-motion findings (F1 to F4), N-1 modal bottom edge 0 px per frame (F5a), N-2 offline strip springs at most 9.7 px per frame (F5b), N-6 0 px for every body control (F5a round 2), N-7 live-updates notice springs at most 4.3 px per frame wide and 19.1 px at 360–390 px (F6), N-9 logs notice springs at most 9.2 px per frame, was 108 px (F7). Open by ruling, for issue #1: N-8 (Chromium-only CLS 0.065–0.080 from `div.stage` below 768 px; nothing visible moves) and the reduced-motion CLS of the notices and form modal. Lint exit 0; no `@keyframes`, no `startViewTransition`. Measurements: `docs/research/motion/phase-1.md` (audit, re-audit, final re-audit, last re-measure, N-9 re-measure). |
| 15 | Gates pass; quality baseline empty | met | `env -u DATABASE_URL BASE_REF=bb86ed8 bun run gates` on f0312ed: fmt, clippy, cargo-test, install, lint, check, test, build, limits, names, imports, baseline all pass (`gates: 12/12 pass`); `quality/baseline.json` is `{}`. Log: `/private/tmp/claude-501/-Users-stephenokita-Scripts-Personal-anima/44a9516e-29ae-4035-8f15-a42b950ef866/scratchpad/gates-wp17-f0312ed.log`. |
| 16 | Signed updates; a bad update is refused | met for the tested part; **blocked** live | `deploy/test/release-feed.test.ts` (signed-version refusals); `updates/tests/schedule.test.ts:81` (a bad signature fails, the app keeps running and keeps checking). `allowDowngrades` and `requireSignedVersion` are real keys in tauri-plugin-updater 2.14.1. Blocked: `tauri.conf.json` pubkey is `REPLACE_WITH_OPERATOR_PUBLIC_KEY`; the default branch is not `main`; operator steps 0 and a to i in `docs/releases.md` are outstanding. |

Other measurements: DEV-001, 200 devices render in 131–358 ms with no API delay; with a 400 ms delay WebKit
takes about 1.0 s, which is the minimum skeleton hold by design.

## Escalations (spec conflicts and contract requests)

- **E-1 Criterion 7 bound.** "Within 2 intervals" holds for a refused connection (120 s exactly). A silent
  device adds the last probe's timeout (122 s ICMP, 125 s TCP), and the scheduler tick adds about 1–5 s of
  wall-clock slack. Ruling needed: either read the criterion as "2 intervals plus one probe timeout", or
  change the retry schedule.
- **E-2 R-020 and the K1 comment.** See D-5. `contracts/schema.graphql:4` is stale; R-020 should name
  `contracts/schema.graphql` as the generation source, guarded by the CON-002 test.
- **E-3 Section 5.7.3 cross-fade easing.** See D-6; spec names `ease-exit` for the outgoing layer.
- **E-4 SET-002 against LOG-008.** SET-002 says the Settings page shows "only" its rows; LOG-008 adds a
  log-folder row. The page follows LOG-008.
- **E-5 C-7 Modal blur.** Section 5.7.3's Modal row says "blur and scale 0.98 to 1"; MOT-006 allows only
  `transform` and `opacity` to animate. The code animates scale and fade. The spec needs one rule.

## Handoffs (shared files WP-17 does not own)

- **AGENTS.md and spec Section 13** still say `pnpm`; the repository uses `bun` (CQ1-23).
- **MV-008 keyframes gate:** add a `@keyframes` ban to the shared stylelint config (today a grep in the audit).
- **CQ1-02:** the events and audit cursor decoder (`crates/hub/src/events/page.rs:69-75`) has no time-range
  bound, so a crafted cursor likely returns INTERNAL instead of a validation error. Fix first among the CQ-050
  items once the operator accepts it (D-3).

## For issue #1 (deferred)

- N-8: portal `LogsPanel` and `HostKeyModal` out of the page chain to make the compact-strip CLS 0 (MOT-007).
- N-10: reduced-motion CLS of the live notices (0.037–0.139) and of the form modal (up to 0.28), Chromium only.
- C-6 primary button contrast 3.5:1; N-4, N-5; C-2, C-4, C-8a, C-8c.
- The 8 px logs panel handle; NET-004 compact text; velocity carried through reversals; a font preload if the
  typeface changes later.
- The WebKit "General" disclosure close reaches 98.4 px in its first spring frame on a long frame.
- Notice removal is reachable only in unit tests: no screen clears `liveError`.
- Test and code coulds: `contain: layout` on `.disclosure` has no automated test; O(n²) `carried` scan in
  `Disclosure.svelte:96-102`; `moved()` holds two rules; a fixed element mounting mid-flight of the strip FLIP
  sits offset until settle; whole-subtree style scan in `bannerShift.ts`; the logs test finds the spring run by
  its `stiffness` key; comment placement at `LiveNotice.svelte:31`; `gap` prop name no longer fits `padded`.

## Decisions the programme should know

D-5 to D-14 are in [WP-17.md](WP-17.md). D-13 and D-14 are the programme's rulings on the last re-measure.
`LiveNotice` now lives in `apps/client/src/lib/hub/notice/` and is used by the device list, the device page and
the logs panel; `lib/hub/` therefore depends on `$lib/motion`.

## Open risks

- `k4_8_5_a_client_ping_sent_while_the_hub_s_send_waits_keeps_the_session` uses fixed 2.5 s timers and failed
  once under full-gate load; it passed on rerun.
- Every motion measurement is in Playwright's Chromium and WebKit; real WKWebView (macOS) and WebKitGTK (Linux)
  are not measured.

## Read-only live checks for the programme to run

Each command only reads. None changes the VPS, Headscale or Cloudflare.

| Criterion | Command | Expected |
| --- | --- | --- |
| 1 | `ssh <vps> 'cd /opt/anima/deploy && docker compose ps --format "{{.Name}} {{.State}} {{.Health}}"'` | hub, postgres, guacd, certs healthy; migrate exited 0 |
| 2 | `ssh <vps> 'docker compose -f /opt/anima/headscale/compose.yaml exec headscale headscale nodes list'` and the same with `nodes list-routes` | a router at each site with its subnet route approved (today: none, so blocked) |
| 10 | From a device on the VPN: `curl -sS -o /dev/null -w '%{http_code}\n' https://hub.anima.ing/health` | 200 (the hub listens only on 100.64.0.1:443) |
| 16 | `curl -sI https://updates.anima.ing/latest.json` | a redirect to the release feed (today: blocked) |
