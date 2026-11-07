# WP-32 report — Client files, wake and live sessions

Status: **done.** Package branch `claude/wp-32-files-wake-live`. Plan and review log: [WP-32.md](WP-32.md).

## Tasks

| Task | Commit | Base | State |
| --- | --- | --- | --- |
| T4 `wake` | daf0d953 | f225b9cb | Merged into `phase-2` at 115265ed |
| T1 `files-link` | 649a9243 | `phase-2` b1d956ba | Merged into `phase-2` at 41c8252 |
| T3 `live-list` | 4c3c2c25 | `phase-2` b4f5d702 | Merged into `phase-2` at 2d7459c0 |
| T2 `files-panel` | a8310f55 | `phase-2` 2d7459c0 | Accepted, SHA sent |
| T5 `live-opaque` | fbbe120d | T2 (5544cba8) | Accepted, SHA sent |
| T6 `wake-live` | 0daa3692 | `phase-2` 3eddb112 | Accepted, SHA sent (three rounds, R-066 exception) |

Every task passed gates 4–12 (R-068; no Rust, so gates 1–3 were not run). Each had at most two review rounds.
The final package review accepted the package with no must.

## What the client has now

- **Files (RA-032, K4 §9, MOT-009, MAT-008).** `sessions/files/api.ts` (`openFiles`) and `files/link/`:
  - The link has the `anima-files` subprotocol, the `tunnelUrl` rule, and the token only in the constructor.
  - Hub messages are a closed set and the link fails closed.
  - Ping after 5 s; lost after 15 s.
  - One request at a time.
  - Downloads are bounded by the announced size. Uploads send 64 KiB frames, hold back above 1 MiB buffered, and
    stop and send `cancel` on `refused`.
  - Progress is linear.
- **Files panel.** A solid column beside the terminal, toggled by a "Files" button in the SSH toolbar.
  - It has listing, Up, Refresh, download, upload by picker or drop, refusal texts and a two-line status.
  - Device names are text only; `joinPath` refuses unsafe names and `saveName` gives one safe segment.
  - The panel closes with its session and asks for a new ticket for the new session on reconnect.
- **Live sessions (RA-034, D-8).** A nav item with the count opens a popover (Menu role, from the control,
  opaque `surface-raised`, MAT-008) listing device, protocol, operator, client address and start, with Stop
  (`endSession`).
  - It loads with `sessions(live: true)` and `sessionChanged`.
  - A stale re-query is dropped, and pending changes are applied on top of its answer.
- **Wake (RA-031).** `sessions/wake/` with `wakeApi(hub)`, `wakeText` (fixed text per WP-27 code) and `WakeButton`
  (`deviceId`, `api`, `disabled?`). The props were relayed to WP-26.

- **T6 (after WP-22).**
  - **Wake on start.** When `wakeSent` is true, a reserved "Waking <device>" line appears under the four
    connection steps. The Guacamole receive timeout is 120 s while a woken tunnel opens and goes back to 15 s
    once it is open. The terminal link has no opening timer.
  - **Live list.** It follows `sessionChanged`. A subscription error shows "Live updates stopped; the list
    refreshes when you open it."
  - **Certificates.** Close 4003 offers "Check certificate". "Unchanged" and Reconnect appear only when
    `certificateMatched` (CR-3).
  - **Connection tests.** A reachable RDP or VNC test shows the hub's message, not a failed sign-in.
  - **Stream policy.** No client stream breaks the device policy (checked; no change needed).

## Edits outside owned paths (each reported and accepted by the programme)

- `lib/nav/Nav.svelte`: one `<LiveSessions>` item between Devices and the status notice (D-8, NAV-003).
- `lib/sessions/view/session.svelte.ts`: `sessionId` on the `open` stage.
- `lib/sessions/view/SessionView.svelte`:
  - the Files toggle and the `<FilesPanel>` mount, placed before `.pane` with CSS `order`;
  - `.stage` becomes flex;
  - the toggle clears when the view is not ready.
- `lib/sessions/view/view.fixture.svelte.ts` (optional hub), the new `view/filing/` (test and fixture), the toggle
  test moved out of `view.test.ts`.
- `lib/icons/icons.ts`: seven glyphs (`access_point`, `folder`, `file`, `link`, `upload`, `download`, `arrow_up`).
- No-op `request`/`subscribe` in the hand-made hub fakes of `nav/shell.test.ts` and
  `sessions/strip/{strip,panes,keyboard}.test.ts`.
- T6 (programme order; WP-24 and WP-25 are done):
  - `lib/sessions/api.ts`: `wakeSent` and `certificateMatched` added;
  - `lib/sessions/tunnel/`: `wakeSent` on `Started` and on refusals, and the waking receive limit;
  - `lib/sessions/view/`: the steps and the wake line, the ended panel's "Check certificate", the certificate
    modal (with `returnTo`) and `trapFocus` `returnTo`;
  - `lib/forms/device/remote/test/`: the reachable outcome and `certificateMatched`;
  - their fixtures and tests.
- `lib/hub/generated/**`: regenerated, never hand-merged.

## Contract change requests

None.

## Handoffs

- **WP-26.** Mount `WakeButton` where a device with a MAC address is shown, with `disabled` while the device is
  Up. Until then RA-031 has no client path except the hub's automatic wake on `startSession`.
- **WP-24 area (issue #1).** `liveness()` and the ping and silence constants are copied between
  `files/link/link.ts` and `tunnel/terminal/link.ts`. Lift them into `sessions/tunnel/liveness.ts`.
- **WP-34.**
  - A Stop failure shifts the live rows once (MOT-007, programme ruling).
  - Check a real download in the Tauri app. The blob URL is revoked right after `click()`, and wry saves the
    suggested name in `~/Downloads`.
  - The reduced-motion form of the live popover has no test.
- **Hub (WP-27 owner).**
  - Report or refuse non-UTF-8 names instead of replacing them: a replaced name cannot be sent back.
  - A download cancelled after the hub has read every byte, but before the client has all of them, is cancelled
    on the client and audited as done.

## Needs a real device to accept

Criterion 6 (upload and download against a real SSH server, flags, audit entries) and criterion 8 (a sleeping
device with a relay). Criterion 7 is complete only once WP-22 serves `sessionChanged`.

## Open coulds (issue #1)

- T6:
  - a 2 px nudge when the wake icon changes;
  - the ended-panel button widens when its text changes;
  - "You ended the session." on 4003, which may come from the fake hub only;
  - the `.wake` ellipsis is pinned by a test but not re-measured in a browser.

- The files panel does not read `transfer.upload`/`download` up front: a flag that is off shows its reason only
  after the hub refuses.
- The panel's busy guards are pinned only through the link's `busy` refusal. `up` and `refresh` return silently
  while busy.
- Downloads are held in memory as one `Blob`, with no size warning. Hub text messages have no length cap beyond
  10 000 entries.
- Below `bp-compact` the panel takes half the window and rows scroll sideways. At 320 px the toolbar's device name
  is squeezed to 0.
- The name column is about 12 characters at 960–1100 px. The date column could be shorter.
- Constants exported only for tests: `PATH_MAX_BYTES`, `TEXT_MAX_BYTES`, `ENTRIES_MAX`, `isSendablePath`,
  `HOLD_BACK_BYTES`, `DRAIN_POLL_MS`.
- Wake:
  - `wake()` has no `catch`.
  - `FAILURE_TEXT` is keyed by `string`, and its VPN sentence duplicates `hub/error.ts`.
  - It uses `disabled`, not `aria-disabled`, while busy.
  - The status is not clamped to two lines.
  - The jsdom animation stub is copied again.
- Live:
  - The sequence-check mutant survives.
  - `LIVE_LIMIT = 200` truncates silently.
  - The tests count microtask ticks.
  - The `hadFocus` branch in `onOutside` is dead.
  - "Live sessions" wraps to two lines in the wide bar.
- The `saveName` trailing trim after the cut is untested. Stripping `\p{Cf}` splits emoji sequences joined by ZWJ.
  `order: 1` is not testable in jsdom.
- The Files toggle reuses `class="find"` on one long line, to stay under the 300-line limit of `SessionView.svelte`.
- A disabled control looks almost the same as an enabled one (busy state). Safari drops list semantics under
  `list-style: none`.
