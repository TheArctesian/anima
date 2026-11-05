# WP-15 report (package orchestrator)

Status: **done**. Branch `claude/wp-15-settings`, with the latest `phase-1` (`c1a48b7`) merged in. Not pushed, not merged.
`bun run gates` on the tip: **gates 1–2 and 4–12 pass, and gate 3 passes with three time-of-day WP-09 tests skipped** (at `c45915a`, 00:06 UTC; the later merge of `c1a48b7` changes four doc lines only). Full runs stopped at gate 3 because of the clock, never because of WP-15 code:
  - 23:13 UTC: `db_005_a_failed_daily_run_is_retried_within_the_hour` failed. Its `now + 1h` tick crosses midnight.
  - 00:05 UTC: two `checks::tests::heartbeat_cost::api_005_*` tests failed. They insert 6000 s of history before `now()` but create only today's partition, so they fail from 00:00 to 01:40 UTC.

  WP-15 changes no Rust. A full 12/12 run passes between 01:40 and 23:00 UTC; see H-4.. Plan, decisions D-1..D-5, handoffs and review log: [WP-15.md](WP-15.md).

## What is delivered

| Output | Path | Spec |
| --- | --- | --- |
| Settings page | `routes/settings/+page.svelte`:<br>- the h1 "Settings", with its height reserved from tokens (28.797 px before and after the font swap)<br>- the "Signed in as <name>" row and "Sign out" (`SignOutRow`)<br>- a marked place for WP-04 stage B's desktop Updates row | SET-001, SET-002 (R-048), H-7 |
| Sign-in page | `routes/sign-in/`:<br>- three fields: user name, password (hidden), TOTP code (optional; six digits when given)<br>- the hub address field: `storeHubAddress`, then reload, and an invalid address is not stored<br>- pure form rules and fixed outcome messages in `form.ts`, and the flow in `sign-in.ts`<br>- the token is written with `hub.address` only<br>- password and code are emptied on every submit<br>- outside the desktop app nothing is sent<br>- a key store that cannot be read shows its own message, never "wrong password"<br>- `aria-invalid` is set and focus goes to the first invalid field | API-003, CLI-002, CLI-003, NET-005, OD-2, DEV-011, LOG-007 |
| Access domain | `lib/access/`:<br>- `accessApi(hub)` (`login`, `logout`, `me`)<br>- the pure `startGuard` and `signInNeeded`<br>- `navigation.ts` (the browser edge)<br>- `signOut`: `logout`, then `clearAccessToken(hub.address)` even when `logout` fails or rejects, then go to `/sign-in`; the text says whether the hub confirmed<br>- `SignOutRow.svelte` (busy flag) | CLI-003, SET-002, H-5 |
| Start guard | one line in `routes/+layout.svelte`: `guardStart(page.url.pathname, hub.address)` sends the operator to `/sign-in` when no token is stored | API-003, H-5 |
| Desktop check | `keyStoreAvailable()` in `lib/desktop/token.ts`: an additive change on WP-16's merged path, because the lint boundary keeps the Tauri import there | REPO-003 |

Tests: 841 client tests in total. The new ones cover the Settings page, the sign-in page, form rules, the access domain, and the layout guard in `shell.test.ts`. The interface review used WebKit 2370 and Chromium 1248, with Playwright measurements and a delayed font.

## Review findings (summary; detail in WP-15.md)

- **T1 page:** 1 round, accepted with fixes. The SET-001 link-to-title test, an exact SET-002 test and a shorter comment were folded into T2. H-7 was measured in the interface check.
- **T2 sign-in:** code review and security review, 3 rounds.
  - Code-review must: nothing tested that the layout calls the guard.
  - Security shoulds:
    - The tests used `DEFAULT_HUB_ADDRESS`, so writing the token with the wrong address went unnoticed.
    - A key store that cannot be read showed "wrong password" while the login never left the client.
    - The sign-out text claimed the hub had confirmed when it had not.
    - A `logout` that rejects skipped the clear.
  - Security coulds:
    - A browser sign-in left a live token on the hub that was then dropped; it is now refused before sending.
    - The Sign out button had no busy flag.
  - Probes that held:
    - An address edited but not saved does not change which hub gets the password.
    - The token is never in `$state`.
    - The password never reaches the URL or history.
    - A typed hostile hub gets no old token.
    - The guard cannot loop.
    - A hub cannot fake the key-store message, because the check is by identity.
  - Mutations caught: round 2 code review 7/10, security review 10/12; round 3 builder 4/4.
- **T3 interface:** 3 rounds.
  - Must: the Sign out button moved 137 px when the name arrived.
  - Shoulds:
    - Error text contrast was 3.05:1; it is now 10.84:1, with a status bar beside it.
    - A two-line message pushed the hub form down.
    - Invalid fields had no `aria-invalid` and focus did not move.
    - Labels grew at the font swap, moving the form 40–58 px.
  - Coulds: WebKit Tab skipped the buttons; control height changed at the font swap.
  - Final: no element moves on `/sign-in` or `/settings` from load through the font swap, in both engines.
- **Final package review:** accepted with fixes. The fixes were doc-only: empty handoffs and the D-3 wording. The phase-1 merge only added lines.

## Handoffs (PKG-001)

- **H-1 WP-12, WP-13 (pages):** on a failed hub result, call `signInNeeded(error, goToSignIn)` from `$lib/access/guard` and `$lib/access/navigation`. When it returns true it has already navigated, so show no error.
- **H-2 WP-04 stage B:** mount the desktop Updates row at the marked comment in `routes/settings/+page.svelte`, after `<SignOutRow />`. Use `keyStoreAvailable()` for the desktop check.
- **H-3 WP-16 owners (`lib/desktop`):** `keyStoreAvailable()` was added. Sign-in relies on `AccessTokenError` `not-desktop` and on `TOKEN_UNREADABLE` (compared by identity), so keep both stable.
- **H-4 programme (WP-09):** three hub tests depend on the time of day, so gate 3 fails at some hours:
  - `checks::scheduler::tests::daily::db_005_a_failed_daily_run_is_retried_within_the_hour` fails from 23:00 to 00:00 UTC, because its `now + 1h` tick crosses midnight.
  - `checks::tests::heartbeat_cost::api_005_a_page_of_heartbeats_reads_the_page_not_the_history` and `…api_005_the_latest_heartbeat_reads_a_few_rows_not_the_history` fail from 00:00 to 01:40 UTC. They write 6000 s of heartbeats before `now()` but call `ensure_day_partition(now())` only, so yesterday has no partition.

  Fix: take a fixed time, or create the partitions for every day the test writes.

## Contract change requests (CON-001)

None.

## Deferred (for issue #1, R-007)

- The browser (web client) cannot sign in: there is no token provider, so sign-in is refused before sending (WP-16 C2). The start guard does nothing outside the desktop app.
- The page overflows horizontally at 360 px, because of the fixed 192 px nav (WP-11 deferral). The Tauri `minWidth` is 960.
- `--border` on `--bg` is 1.69:1, so input borders are faint (WP-02 tokens).
- There is no control-height token. Controls use `--space-8` for `block-size`.
- Hub field messages are shown as the hub sends them (escaped). They could map to fixed text.
- Replace the empty per-page `<script lang="ts">` blocks with an ambient `*.svelte` declaration.
- A late `me` result after unmount only sets local state; cancel it.
- The real `login` success and failure paths were exercised only in jsdom with a fake hub, not against a running hub in the desktop app.

## Notes

- T1's review fixes were folded into T2, because both edited the same files.
- The phase-1 merge (`570a17c`) regenerated `lib/hub/generated/*` from the documents on both sides.
- To make room for the gates I deleted the build output of this package's own finished builders (about 21 GiB). No other session's worktree was touched.
