# WP-12 report (package orchestrator)

Status: **done**. Branch `claude/wp-12-devices-page`, with the latest `phase-1` (`b264fe6`: WP-09 wiring, WP-14, WP-16) merged in. Not pushed, not merged.
`bun run gates` on the code tip `c8839e7`, with `DATABASE_URL` unset: **12/12 pass** (run started 2026-11-05 01:42 UTC, outside both WP-09 clock windows below; every test passed, none skipped: hub 708 + 528 + 80 + others, 0 failed). Plan, decisions D-1..D-13, escalations and the review log: [WP-12.md](WP-12.md).

## What is delivered

| Output | Path | Spec |
| --- | --- | --- |
| Devices route | `apps/client/src/routes/devices/+page.svelte`. It replaces the WP-11 placeholder. Page state lives in the URL, set with `replaceState`. "Add device" goes to `/devices/new` (H-1). The SSH button opens a session tab through `useSessions().openSession` (WP-14 H-1). | DEV-001–005, NAV-004 |
| Devices model | `lib/devices/`: `devicesApi(hub)` (list, tags, `deviceStatusChanged`); URL query parse and write; pure list state (page merge, live status apply, the E-1 client order and filter); the `fakeHub` fixture | DEV-002–004, API-006, CQ-045 |
| Device list UI | `lib/devices/{card,toolbar,screen,live}/`: the card (Section 6.1: name, host and IP, status pill and edge, 24 h uptime, 50-cell bar, SSH when enabled with an accepted host key); grid and table; toolbar with name, status, tag and sort menus and a grid/table toggle; empty and no-match states; staged skeleton; Load more; live status coalescing; reorder, leave and status-change motion | DEV-001–005, Section 6.1; press, hover, menu, reorder, status change and loading roles |
| Heartbeats | `lib/checks/`: `checksApi(hub).watchHeartbeats`; a pure merge of each check's heartbeats into the device's last 50 (E-2 option 1, R-044); `HeartbeatBar.svelte` with the new-heartbeat role; uptime text | Section 6.1, new-heartbeat role |
| Event log | `lib/events/`: `eventsApi(hub)` (events, `eventAdded`); pure log state (filter to variables, live match, prepend, 500-row cap, older-page merge) | LOG-009, LOG-010 |
| Logs panel | `lib/events/{panel,resize}/`: the panel is fixed beside the nav and moved only by `translate`. It has time, severity, source and device filters, live rows, Load older, a disclosure and the new-log-row role. Resizing: the edge follows the pointer 1:1 with rubber-band, the snap comes from the projected position, the release uses `spring-momentum` with the pointer velocity, and keys reach all three stops. | LOG-009, LOG-010, Section 5.7.4, MOT-030–034, MOT-040–042 |
| Drag helpers | `lib/motion/drag/`: the pointer-capture drag attachment and a velocity estimate from event timestamps. This is a subfolder, per WP-11 H-7. | MOT-030–032 |

Tests: about 650 client tests in the full suite. They cover pure logic, a fake hub (request and subscription counts), jsdom components and mutation-checked fix tests. The interface review ran in WebKit 2370 and Chromium 1248 against a Playwright fake hub, for every role and its reduced form, interruption, and layout shift on every frame.

## Escalations and rulings

- **E-1 → R-044:** the client's live order for `DeviceSort.STATUS` (worst first) and `UPTIME` (lowest first, null last), with ties by name, is now K1 doc text.
- **E-2 → R-044:** Phase 1 reads `checks { heartbeats(first: 50) }` per device and merges them on the client in a pure function. The page size stays 200 (DEV-001). WP-17 measures a 200-device page.

## Review findings (summary; detail in WP-12.md)

- **T1 devices-model:** accepted after 2 rounds.
- **T2 events-model:** accepted after 2 rounds. The 500-row cap now ends paging.
- **T3 checks-heartbeats:** accepted after 2 rounds.
- **T4 device-list:** code review took 3 rounds, interface review 3 rounds.
  - Musts fixed: the leave FLIP jumped (704 px in one frame); the name link was 17 px high; browser scroll anchoring jumped after a re-sort (`overflow-anchor: none`).
- **T5 logs-panel:** code review took 3 rounds.
  - Musts fixed: one filter change sent 3 requests; the release spring used `snappy` (D-10 corrected to `momentum`).
- **Final package review:** accepted, with 4 fixes made in `95c3a58`.
- **Final interface review:** rejected once, then accepted.
  - Placement musts fixed: the panel was in the page flow, so it moved with the page height and jumped 472 px on Expand; a long device name widened the page.
  - After the fix, the panel holds its place from the first frame in both engines, layout shift is 0 at every stop, and nothing scrolls sideways at 390 px.
- **Phase-1 merge:** `routes/+layout.svelte` takes WP-14's version. WP-14 H-1 is wired in `c8839e7`, and two mutations show its tests catch a broken wiring.

## Handoffs (PKG-001)

- **H-1 WP-13:** "Add device" calls `onAddDevice()`, which goes to `/devices/new` today; replace it with the modal or serve the route. Card names link to `/devices/{id}`. The name and pill carry `data-device-id` for the card-to-detail shared element. Escape is handled on the document without stopping propagation (N5); check it against WP-13's modal.
- **H-2 WP-14 (closed):** the SSH button calls `openSession` as WP-14 H-1 asks, and shows only with `ssh.enabled` and an accepted `hostKey`.
- **H-3 programme:** `lib/motion/drag/` is a new subfolder under WP-11's `lib/motion/`.
- **H-4 WP-09:** the client order for live reorder follows E-1 (R-044); the hub's `DeviceSort` must keep the same order.
- **H-5 WP-11, WP-02:** `--nav-inline-size` was added to `styles/global.css` (hand-written) and is used by `lib/nav/Nav.svelte`. The logs panel's left edge reads it. The nav resize gesture must set this variable. WP-14 made the same `min-inline-size: 0` change in `routes/+layout.svelte`, so WP-12 adds nothing there.
- **H-6 WP-17:**
  - Measure a 200-device page with real heartbeats (E-2).
  - Check panel frame time and backdrop blur in real WKWebView and WebKitGTK.
  - Re-check the held-pointer window resize in WKWebView: headless WebKit moved `scrollY`.
  - The DEV-001 200-card jsdom test has a 30 s timeout so it survives a loaded gate run.

## Contract change requests (CON-001)

None open. E-1 and E-2 are ruled (R-044).

## Deferred (for issue #1, R-007)

- `Device.heartbeats` in K1, to replace the per-check heartbeat read (E-2).
- Folder cycle: devices screen → events panel → devices toolbar (`Menu`, `toggled`).
- `lib/events/panel/` is at the 12-file limit.
- Compact nav at narrow widths (N2; with WP-11 and WP-16). At 390 px the card is 150 px wide and the panel filters stack in five rows.
- Chromium load CLS of 0.0005 to 0.002, from control widths at the font swap (WP-11 H-8 proposes `font-display: block`).
- Status changes are not announced to screen readers (C6). Status dot contrast relies on the label (C8).
- Heartbeats:
  - the O(n²) dedupe in `latestBeats`;
  - a blank cell mid-spring;
  - `responseTimeMs` is fetched but unused.
- Menu material on the panel in real WebKit (S4).
- Logs panel:
  - the 8 px edge strip has no grip and is under UI-013's 24 px;
  - a truncated device name has no `title`;
  - the empty page scrolls 89 px of clearance;
  - the `innerHeight` initial value is redundant;
  - `LogsPanel.svelte` has stale indentation.
- Un-exported constants that tests repeat (`PAGE_SIZE`, `VELOCITY_WINDOW_MS`).

## Gate notes

- Hub tests flaked under parallel load during this package, and each passed when run alone. None of them is in WP-12's diff:
  - `wiring::devices_schema::api_005`
  - `sessions::tunnel::…::sec_002_d_10`
  - `access::tests::timing::sec_002_d_8`
  - `sessions::privileges::ra_008` (permission denied for table `sessions`)
  - `log_007_redaction_of_pathological_text_takes_linear_time`
- **Phase-1 defects (WP-09, `crates/hub/src/checks/`), found here.** Both tests read the database's wall clock and assume the test runs inside one UTC day. WP-12 has no Rust, SQL or quality-script difference from phase-1 `b264fe6`, so they fail on phase-1 the same way.
  1. `checks::scheduler::tests::daily::db_005_a_failed_daily_run_is_retried_within_the_hour` (`checks/scheduler/tests/daily.rs`) fails from 23:00 to 24:00 UTC.
     - Cause: the test takes `now` from the database, then ticks the scheduler at `now + 1 h`. The scheduler passes that instant as the retention cutoff, but `checks.drop_day_partitions_before` clamps it to `least(instant, pg_catalog.now())`. This is the real clock, by design, so a caller cannot drop data early.
     - When `now + 1 h` is the next UTC day, the cutoff is one day past what the function allows. The drop is refused and the 40-day-old partition stays.
     - Seen failing at 23:21 UTC, both in the full run and alone, and passing alone at 00:02 UTC.
     - Predictable: the test's time and the clamp's time come from two clocks.
     - Fix (WP-09): pick a start whose `+1 h` stays inside the same UTC day, for example `now - 1 h` when `now` is in the last hour. Or make the test assert the clamp.
  2. `checks::tests::heartbeat_cost::api_005_the_latest_heartbeat_reads_a_few_rows_not_the_history` and `checks::tests::heartbeat_cost::api_005_a_page_of_heartbeats_reads_the_page_not_the_history` (`checks/tests/heartbeat_cost.rs`) fail from 00:00 to 01:40 UTC.
     - Cause: `check_with_history` creates only the partition for `now()`, then inserts 6000 beats at `now() - n s`, which is 100 minutes back.
     - Before 01:40 UTC some beats fall on the previous UTC day, which has no partition, so the insert fails with 23514 "no partition of relation heartbeats found for row".
     - Seen at 00:0x UTC; this matches WP-03's 00:00 to 01:40 window.
     - Fix (WP-09): also create the partition for `now() - 6000 s`, or space the beats inside the current day.
- The disk filled to 100% during the reviews; a gate run then failed with `ENOSPC`. A reviewer freed 5 GiB by deleting `node_modules` from its own review worktrees.

## Notes

- Agent scratch stayed in session scratchpads (R-037).
- Nothing touched the VPS, Headscale or any cloud account, and nothing under `contracts/` was edited.
