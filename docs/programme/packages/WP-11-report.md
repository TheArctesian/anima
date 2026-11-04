# WP-11 report (package orchestrator)

Status: **done**. Branch `claude/wp-11-client-shell-cfd923`, with the latest `phase-1` (`3c43a6e`, R-041) merged in. Not pushed, not merged.
`bun run gates` on the tip: **12/12 pass**. Plan, decisions D-1..D-8 and the review log: [WP-11.md](WP-11.md).

## What is delivered

| Output | Path | Spec |
| --- | --- | --- |
| Motion helpers (only importer of `motion` 14.4.0) | `apps/client/src/lib/motion/` — `animate` (translate, scale, opacity only), `springMotion`/`tweenMotion`, `motionPreference()`, `press()` attachment, `crossFade`/`popover`/`modal`/`listRow` transitions (one `transition:` function each, so they reverse mid-flight), `measure`/`flip`, gesture math (`project`, `rubberband`, `nearestSnap`), `ambientMotionAllowed` | MOT-001–011, MOT-020/021, MOT-032/033 (math), MOT-040–042 |
| Hub client (transport only) | `apps/client/src/lib/hub/` — `createHub` with `request` (HTTP), `subscribe` (graphql-ws), `status`, `close()`; K7 typed errors; status reducer and `dataAge`; `provideHub`/`useHub`; socket in `socket/` connects at creation and keeps probing (backoff 1 s doubling, cap 30 s; 10 s connect deadline) | API-002, API-008, CQ-045, NET-004, NAV-003, R-020 |
| Generated types | `apps/client/src/lib/hub/generated/` from `contracts/schema.graphql` (GraphQL Code Generator client preset); `bun run --cwd apps/client graphql` writes, the client `lint` script fails when stale | API-008 |
| Icons | `apps/client/src/lib/icons/` — one `nf-md-*` table checked against the bundled Propo font's cmap, `Icon.svelte` (`aria-hidden`) | UI-014–016 |
| Shell | `routes/+layout.svelte` (global.css once, one `createHub` call, nav, keyed page area with `crossFade`, hub closed on teardown), `routes/+page.ts` redirect `/` → `/devices`, placeholder `devices` and `settings` pages | NAV-004, CQ-044, Section 5.5 page change |
| Navigation bar | `apps/client/src/lib/nav/` — Devices, NET-004 banner, divider, "Connected"/"Disconnected", Settings at the bottom; chrome material; fixed width; sticky; `press()`; focus ring; data-age clock | NAV-001–005, NET-004, MAT-004/005, MOT-001, MOT-007 |
| Lint boundaries | `eslint.config.js` — `motion` only in `lib/motion`, `graphql-ws`, `fetch`, `WebSocket`, `EventSource`, `XMLHttpRequest` only in `lib/hub`; `.svelte.ts` parsed; generated folder ignored | MOT-020, MV-008, CQ-043, CQ-045 |

Tests: client suite about 350 (pure logic, fake socket, jsdom layout and nav, lint-rule tests, cmap check). Interface review in WebKit 2370 and Chromium 1248 with Playwright measurements.

## Review findings (summary; detail in WP-11.md)

- **T3 icons:** accepted in round 1.
- **T2 hub:** 2 rounds, then T2b (connect deadline) and T2c (eager connect, found by the T4 interface review: without it the NET-004 banner could never appear). T2c round 1 should: after a 4403 nothing probed again → probes continue on the backoff.
- **T1 motion:** 2 rounds. Round 1 found a stale style after a stopped flip and that split `in:`/`out:` transitions cannot reverse in Svelte 5.
- **T5 lint:** 2 rounds.
- **T4 shell:** code review 2 rounds; interface review 3 rounds. Musts fixed: the banner widened the nav (CLS 0.096), font swap shifted the nav, 8 px body margin. Shoulds fixed: status label contrast (MAT-005), WebKit Tab skipping links (NAV-005), no reduced-motion press colour (MOT-040), close warning logged the transport's message (LOG-007).
- **Final package review:** accept; added handoffs H-5 to H-8.

## Handoffs (PKG-001)

- **H-1 WP-16:** `--spring-*` CSS uses `linear()`; minimum web view WKWebView 17.2+ (macOS 14.2) and WebKitGTK 2.44+.
- **H-2 WP-16:** supply the key-store token provider (CLI-003) and the stored hub address (CLI-002) to the one `createHub` call in `routes/+layout.svelte`.
- **H-3 WP-12, WP-15:** replace the placeholder pages; use `useHub()` (component init only) with a `<domain>Api(hub)` client in `$lib/<domain>/api.ts`; run `bun run --cwd apps/client graphql` after adding a document.
- **H-4 programme:** `apps/client/package.json`, `bun.lock`, `eslint.config.js` (rules appended per WP-02 H-3) and one line in `quality/generated.txt` changed in this package.
- **H-5 WP-15, WP-16:** an unreadable token is a typed `UNAUTHENTICATED` and leaves the status unchanged; the "sign in needed" surface belongs with login.
- **H-6 WP-16:** until the token provider is real, the hub answers every socket with 4403, which counts as contact, so the nav shows "Connected" while not signed in; the socket re-probes at most every 30 s. Nav links carry `tabindex="0"` because WebKit skips links on Tab: verify NAV-005 in the real WKWebView and WebKitGTK.
- **H-7 WP-12, WP-15:** page headings change height at the font swap; reserve heading height. Ambient motion must call `ambientMotionAllowed` with `document.visibilityState` and the motion preference. `lib/motion/` is at 12 files; new helpers go in a subfolder.
- **H-8 programme (proposal for WP-02's file):** set `font-display: block` in `apps/client/src/styles/fonts.css`. The fonts are bundled and local, so blocking is short, and it removes the first-paint swap shift (MOT-007) on every page. A preload from the root layout was tried and removed: it is added after JS boot and both engines fetched the font twice.

## Contract change requests (CON-001)

None open. Token transport and the TS type source were confirmed as R-020.

## Deferred (for issue #1, R-007)

- Lint does not refuse a dynamic `import()` of `motion` or `graphql-ws`.
- No test drives a non-instant motion to natural arrival and checks `finished` settles without a timer (confirmed with the real engine).
- Nav-bar resize gesture (5.7.4, MOT-030+) and a compact nav below `bp-compact` (23 px overflow at 360 px).
- Modal backdrop blur (MOT-006 ambiguity; documented in `modal`).
- A 502 HTML answer sends no status signal (F7, kept: the hub address answers).
- Hub socket: pin the 4000/4999 fatal-close boundaries in tests (G1); a `subscribe` during a retry wait after a 4403 waits up to 30 s (G3); re-probe after a fatal 1011 close (G4).
- The hub address in the banner wraps mid-word in the 192 px nav.
- `lib/hub/socket/fake.ts` imports vitest beside production code (D7; nothing outside tests imports it).

## Notes

- A merge commit `75c4be9` on the package branch kept conflict markers in `bun.lock` because a commit was chained after a conflicted merge. Fixed forward in `1572962` (lockfile regenerated); no history rewrite. Every later merge checks for unmerged files before committing.
- `:global(body) { margin: 0 }` lives in the root layout; it could move into `global.css` (WP-02) later.
