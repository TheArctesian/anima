# Motion audit: Phase 3 redesign (nav, device page, Settings, Add device)

Audit of the client on branch `claude/rebase-rerun-anima-214537`, based on `066327c5`, including the uncommitted redesign of the
navigation bar (NAV-006, NAV-007), the device page (DEV-020, DEV-021), Settings and Add device. All data is synthetic (Section
5.8): the browser-test synthetic hub in `apps/client/browser/` (K1 schema over the K6 fixtures), inside Playwright, with every
other host blocked. No hosted app was opened. Evidence marks: **local** (measured in a browser on the production web build),
**static** (read from code only).

## 1. Storyboard of the reel

Reel: `motion-reel.html` beside this file (a simulation on synthetic content, not shipped behaviour). 120 BPM, one beat 0.5 s,
6 bars, 12 s loop.

| Bar | Beat | Time | Moment | Proposal |
| --- | --- | --- | --- | --- |
| 1 | 1–2 | 0.0–1.0 s | Cursor travels to the sidebar toggle, presses | M-1 |
| 1 | 3–4 | 1.0–2.0 s | Labels fade out; the bar snaps to icons; the page glides left from where it was | M-1 |
| 2 | 1–2 | 2.0–3.0 s | Cursor presses the toggle again; the page glides right; labels fade back in | M-1 |
| 2 | 3–4 | 3.0–4.0 s | Cursor travels to a device card name and presses | M-5 |
| 3 | 1–3 | 4.0–5.5 s | The name and status pill fly to the header at full strength while the pages cross-fade | M-5 |
| 3 | 4 | 5.5–6.0 s | A live update: the 24 h uptime figure cross-fades 99.58 % → 99.61 % (never counts) | M-4 |
| 4 | 1–2 | 6.0–7.0 s | Cursor presses Copy; the copy icon cross-fades to a check, "Copied" fades in | M-6 |
| 4 | 3–4 | 7.0–8.0 s | "Copied" fades out after the highlight time; the icon returns | M-6 |
| 5 | 1–2 | 8.0–9.0 s | Cursor presses Save on a Settings row; after 300 ms the label cross-fades to "Saving" | M-3 |
| 5 | 3–4 | 9.0–10.0 s | The hub answers; the label cross-fades to a check and "Saved" | M-3 |
| 6 | 1–4 | 10.0–12.0 s | "Saved" returns to "Save"; cursor goes back to rest; the frame returns to its first state | M-3 |

The Copy moment (M-6) and the Save moment (M-3) are on separate bars for legibility; in the product they are unrelated.

## 2. Top findings

Score = frequency × visibility × meaning (each 1–3) ÷ cost (S = 1, M = 2, L = 3).

| ID | Title | Class | Route | Impact | Cost | Score |
| --- | --- | --- | --- | --- | --- | --- |
| M-4 | Overview readings change on one frame | Numbers and charts that jump | `/devices/[id]` Overview | 2 × 2 × 3 | S | 12.0 |
| M-1 | Collapsing the sidebar moves the whole page 124 px in one frame | Layout jump | every page | 3 × 3 × 2 | M | 9.0 |
| M-3 | Settings Save buttons show nothing while the hub answers, and nothing when it succeeds | Missing feedback | `/settings` | 2 × 3 × 3 | M | 9.0 |
| M-5 | The card-to-detail flight starts as a ghost | Broken continuity | `/devices` → `/devices/[id]` | 2 × 3 × 3 | M | 9.0 |
| M-6 | Copy host key: "Copied." appears on one frame and the icon never changes | Missing feedback | `/devices/[id]` Overview | 1 × 2 × 2 | S | 4.0 |
| M-2 | The navigation bar cannot be resized by hand (5.7.4) | Missing direct manipulation | every page | 1 × 2 × 2 | L | 1.3 |

Only six findings survived measurement. Page loads were clean (Section 4), and the tab indicator, theme change, modal open
and close, and menu roles measured as specified.

## 3. Baseline

- **Vocabulary** (`design/tokens/`, generated into `lib/motion/tokens.ts`): springs `default` (ζ 1, 0.35 s), `snappy` (ζ 1,
  0.25 s), `move` (ζ 1, 0.4 s), `momentum` (ζ 0.8, 0.3 s); durations `press` 100, `fast` 150, `base` 200, `highlight` 1500 ms;
  `ease-enter` and `ease-exit`; `stagger-list` 20 ms × 8; thresholds `drag` 10 px, `loading` 300 ms, `loading-min` 500 ms.
- **Roles** (`lib/motion/roles/`): `press`, `crossFade`, `fade`, `popover`, `modal`, `listRow`, `flip`/`measure`,
  `reorderMotion`; `sharedItem` (card to detail); `view-transition/cross-fade` for the theme change.
- **Rules held** (spec 5.7.1): feedback on pointer-down under 100 ms; interruptible from the current value; springs for anything
  touchable; no bounce without momentum; leave on the path you came in on; only `transform` and `opacity` (individual
  `translate`/`scale`); CLS 0; readings cross-fade, never count; honest progress; no ambient loop except Live and the cursor.
- **Traversal**: page change is a 200 ms cross-fade of the keyed `.page` in `routes/+layout.svelte`; shared items fly with
  `spring-move` on top of it. Under reduced motion only the cross-fade runs.
- **Reduced motion**: `motionPreference()` folds every spring and duration to the reduced token set (thresholds never fold,
  MOT-021); the theme view transition uses its own CSS. The CSS `*` rule does not reach JS springs, which is why every role
  takes the preference as a parameter (MOT-041).
- **Gates** today: stylelint's token rule (no raw durations), unit tests per role, and the browser layout-shift spec
  (`browser/acceptance/layout-shift.spec.ts`, Chromium only). No gate looks at the collapse or at async buttons.
- **Other platforms**: no motion branches or sessions are active for native platforms (`git branch -a | grep -i motion` shows
  only the Phase 1 branches). The desktop app is the same web build in WKWebView and WebKitGTK.

## 4. Surface inventory

Measured on the production web build with the synthetic hub, Chromium 158 headless shell (v1254) and WebKit 27.2 (v2388).
Widths 390, 900 and 1280; height 1600 for loads; GraphQL delayed 400 ms for loads and 1500 ms for mutations.

| Route | State | Current motion (measured) | What was seen | Mark |
| --- | --- | --- | --- | --- |
| `/devices` | First load, 3 widths, normal and reduced | Skeleton, then cross-fade | CLS 0 in all six runs; toolbar now starts the page (heading is screen-reader only) | local |
| `/devices/[id]` | First load, 3 widths, normal and reduced | Header skeleton cross-fades over the reserved rows; overview skeleton draws the real cards | CLS 0 in all six runs | local |
| `/settings` | First load, 3 widths, normal and reduced | Rows keep reserved message lines | CLS 0 in all six runs | local |
| every page | Collapse sidebar | None: nav 192 → 68 px and every page element 124 px left on one frame (t 25 → 65 ms); labels vanish on the same frame; only the toggle's 150 ms colour transition runs | Chromium and WebKit identical | local |
| every page | Expand sidebar | Same in reverse on one frame | | local |
| `/devices` → detail | Card to detail | Page cross-fade 200 ms; name and pill fly on `spring-move`, settle about 500 ms | At 60 ms the travelling name is barely visible over the card it left: it rides the incoming page's opacity (0 → 1 over 200 ms) while its counterpart on the outgoing page is hidden | local |
| detail | Tab switch Overview → Checks | Indicator `translate` 0 → 128 px and `scale` 7.75 → 6.44 on `spring-snappy`, settles about 330 ms; panel swaps on one frame | As specified (5.7.3 Tab selection names only the indicator) | local |
| detail Overview | Copy host key | "Copied." appears on one frame beside the button; icon unchanged | | local |
| detail Overview | Uptime, check counts, status, maintenance change | Text replaced on one frame (no keyed cross-fade in `overview/facts/OverviewFacts.svelte`; `StatusDot.svelte` likewise) | MOT-008 asks readings to cross-fade, as `lib/metrics/bars/MetricBars.svelte` does | static |
| `/settings` | Retention Save, 1500 ms hub, pressed twice 60 ms apart | Press scale only; 1 mutation sent (the `busy` guard works); nothing changes for 1.5 s; no success state | `aria-disabled` is set but `FormButton` has no visible pending or done state | local |
| `/settings` | Theme Dark → Light | Root view transition, 200 ms fade-out and fade-in | As specified | local |
| `/devices` | Add device open | Surface scale 0.98 → 1 with fade, settles about 500 ms; scrim fades | As specified | local |
| `/devices` | Add device close (Escape) | Surface fades out in about 170 ms | As specified | local |
| `/devices` | Add device section toggle | SSH section header height 84 → 40 px between two frames | Followers not sampled: see Unknowns | local |

## 5. Opportunities

### M-4 Overview readings change on one frame (S)

- **Location**: `routes/devices/[id]/overview/facts/OverviewFacts.svelte` (uptime figures, check counts, maintenance) and
  `facts/StatusDot.svelte` (status word).
- **Today**: text replaced on one frame when a live update arrives (static; the same component draws the skeleton).
- **Proposal**: each value in a grid-stacked slot, keyed on its text, with `transition:crossFade` (opacity only,
  `duration-base`, `ease-enter`/`ease-exit`), exactly the `MetricBars` pattern. The slot keeps its size, so nothing moves.
- **Communicates**: this reading changed; it does not pretend to pass through values in between (MOT-008).
- **Reduced motion**: the same cross-fade (opacity only).
- **Other platform**: none yet; iOS mirrors it when built.
- **Verify**: a unit test that changes the uptime and finds two layers during the fade; browser: push a device update and
  sample opacities (sum ≈ 1).

### M-1 Collapsing the sidebar moves the whole page in one frame (M)

- **Location**: `routes/+layout.svelte` (`toggleCollapsed`), `lib/nav/Nav.svelte`.
- **Today**: nav width 192 → 68 px and every element of the page 124 px left on one frame; the labels vanish (local, both
  engines).
- **Proposal**: the width still changes at once (MOT-006 forbids animating it). Before the change, measure `main`; after it,
  `flip()` `main` from the old x to the new on `spring-default` with the individual `translate` property. Labels fade out over
  `duration-fast` with `ease-exit` when collapsing and fade in over `duration-base` with `ease-enter` when expanding. A second
  press mid-flight starts from the current offset (MOT-002).
- **Communicates**: the page is the same page; the sidebar made room.
- **Reduced motion**: instant move; label fade only.
- **Other platform**: iOS has a bottom bar; nothing to mirror.
- **Verify**: browser: sample `main` x per frame — continuous, no step over 40 px; two presses 60 ms apart end in place.

### M-3 Settings Save buttons show no pending or done state (M)

- **Location**: `lib/forms/FormButton.svelte` and its users in `lib/retention/RetentionRow.svelte`,
  `lib/checks/defaults/DefaultsSection.svelte`, `lib/access/SignOutRow.svelte`.
- **Today**: 1.5 s of nothing after a press; one request (guard works); no success state.
- **Proposal**: an async state on `FormButton` (`idle`, `pending`, `done`). The label and the state faces sit in one grid cell,
  so the button never changes size. `pending` shows only after `threshold-loading` (300 ms): the label cross-fades to
  "Saving". `done` cross-fades to a check icon and "Saved", held for `duration-highlight`, then back to the label. A failure
  returns to the label (the error text is the row's, and is not animated, MOT-010). No spinner: repeating motion is reserved
  (MOT-011). The button keeps refusing presses while pending (`aria-disabled`).
- **Communicates**: the press was heard, the hub is working, it finished.
- **Reduced motion**: the same cross-fades.
- **Verify**: unit tests with a fake clock: no "Saving" before 300 ms, "Saved" after success, label back after the hold;
  browser: press twice with a 1.5 s hub, count 1 mutation, read the frames.

### M-5 The card-to-detail flight starts as a ghost (M)

- **Location**: `lib/motion/shared/shared.ts`.
- **Today**: the arriving name and pill fly inside the incoming page, which fades 0 → 1 over 200 ms; the departing copy is
  hidden. At 60 ms the name is barely visible over the card it left (local, screenshot `flight-060`).
- **Proposal**: fly a clone in a fixed layer above both pages at full opacity, from the departing rect to the arriving rect, on
  `spring-move` with `translate` and `scale`; the arriving element stays hidden until the clone lands, then shows and the clone
  is removed. Back mid-flight removes the clone and shows the original (no orphan).
- **Communicates**: this is the same device.
- **Reduced motion**: no clone; the page cross-fade only (unchanged).
- **Verify**: browser: at 60 ms the clone's opacity is 1 and its rect lies between the two ends; Back at 100 ms leaves no clone.

### M-6 Copy host key gives a flat confirmation (S)

- **Location**: `routes/devices/[id]/overview/facts/HostKeyCard.svelte`.
- **Today**: "Copied." appears on one frame; the icon never changes.
- **Proposal**: the icon cross-fades `content_copy` → `check` and "Copied" fades in (`duration-base`); after
  `duration-highlight` both cross-fade back. The live region keeps announcing.
- **Reduced motion**: same cross-fades.
- **Verify**: unit test for the icon and text sequence with a fake clock.

### M-2 The navigation bar cannot be resized by hand (L)

- **Location**: `lib/nav/Nav.svelte`; spec 5.7.4 "Resize the navigation bar".
- **Today**: only the toggle exists (static).
- **Proposal**: a resize edge on the bar. The width follows the pointer 1:1 from the grab offset (MOT-030) between the icon
  width and the full width, with rubber-band resistance beyond them (MOT-033). On release the projected position (MOT-032) and
  the velocity direction (MOT-034) choose collapsed or expanded; the choice is stored as with the toggle and the page moves with
  M-1's FLIP. During the drag the width is direct manipulation, not an animation.
- **Reduced motion**: same drag; the release lands instantly.
- **Verify**: unit tests for the snap choice; browser: drag 60 px with a fast flick collapses.

## 6. Vocabulary proposals

- **V-1 Async button** (new role "Async action"): pending after `threshold-loading`, label cross-fade, done for
  `duration-highlight`. Replaces ad-hoc `busy` flags with no visual. Reduced: same. Tests: new FormButton tests. iOS mirrors it.
- **V-2 Layout follow** (new role "Navigation bar collapse"): instant width, `flip()` of the page on `spring-default`, label
  fade. Reduced: instant. No other platform.
- **V-3 Shared item layer**: the card-to-detail clone flies in a fixed layer above the pages. Replaces in-page flight. Reduced:
  unchanged.

## 7. Do not animate

- The navigation bar's width (MOT-006): only the page's `translate` follows it.
- Tab panel content on tab change: the indicator carries the change; panels swap at once.
- The SSH fingerprint, error text in Settings rows, and focus rings (MOT-010).
- No spinner on Save (MOT-011); no count-up of uptime or check counts (MOT-008).

## 8. Unknowns

- Add device section followers were not sampled after the redesign; re-measured in the build (Section 9).
- The desktop shell's overlay title bar (NAV-007) exists only in Tauri; the browser build cannot show it.
- Headless frame times are the loop's ceiling, not device frame times.
- WebKit has no `layout-shift` entries; WebKit loads were not scored for CLS here (Chromium was).

## 9. Built (same day)

All six findings were built in this worktree and re-measured with the local probe (Chromium and WebKit, synthetic hub).

| ID | Built as | Measured after |
| --- | --- | --- |
| M-4 | `overview/facts/Reading.svelte`: a keyed `crossFade` slot around each Overview reading (status, maintenance, uptime, check counts) | unit tests; no layout shift |
| M-1 | `shiftShell()` in `lib/nav/banner-shift.ts` (the banner's FLIP, now shared) runs the collapse; `lib/nav/collapse/words.ts` fades the names in on expand | toolbar 216 → 92 px over about 450 ms on `spring-default` in both engines; names 0 → 1 over 200 ms |
| M-3 | `lib/forms/feedback/` (pure `actionFeedback` plus the `useActionFeedback` rune); `FormButton` takes `feedback`, `pendingLabel`, `doneLabel` | Retention Save: "Saving" at about 300 ms, check and "Saved" when the hub answers, label back after 1500 ms; width constant; one mutation on a double press |
| M-5 | `lib/motion/shared/shared.ts` flies a copy in a fixed layer (`--layer-popover`) at opacity 1; the header item shows when it lands | copy opacity 1 from the first frame to landing in both engines |
| M-6 | Copy uses the same async feedback with `doneLabel="Copied"`; "Copied." stays in the live region for screen readers only | unit test |
| M-2 | `lib/nav/collapse/ResizeEdge.svelte` and `resize.ts`: the edge follows the pointer with rubber-band past either width; release snaps to the projected width; arrow keys collapse and expand | unit tests for the snap and the band; not driven by a real pointer in the browser |

Deviation: spec 5.7.4 named 280 px as the upper limit. The bar has two widths, so the band starts at the full width; the spec row
was updated to match.

New observation, not fixed: the logs panel is fixed at `inset-inline: var(--nav-inline-size) 0`, so its left edge still steps on
collapse while the page glides. It is closed by default; a glide for it needs the panel's own placement code.
