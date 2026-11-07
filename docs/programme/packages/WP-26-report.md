# WP-26 report (package orchestrator)

**Status: done.**

- **Branch:** `claude/wp-26-device-list`. The code tip is **b412a6dc**, on phase-2 d4e6a0bc. This report is committed on top of it. Nothing is pushed.
- **Accepted task SHAs (R-067):**
  - T1 a11ac49, merged as 1398c31.
  - T3 e8aca70, merged as 446c883.
  - T2 ba5e799, merged as 207efdf.
  - T4 3ade244e, merged as 5d9ffd70.
  - T5 1e8493ce and T6 b412a6dc, sent for merge.
- **Gates:** every task is client-only, so under R-068 each one ran gates 4–12, and every run exited 0. Gates 1–3 were not run because no Rust changed.
- **Plan, decisions and review log:** [WP-26.md](WP-26.md).

## What is delivered

| Task | Output | Spec |
| --- | --- | --- |
| T1 | The list and detail documents select the K1 device fields the hub serves. Fields the hub lacks are typed by fragments that no document spreads. `lib/web-interfaces/api.ts` holds the five mutations. `lib/forms/hub-pending.ts` is deleted. `devicesApi` moves to `lib/devices/client.ts`. | WP-25 H-1, CON-003, API-008 |
| T2 | SSH, RDP, VNC and Web buttons on the card, the table row ("Connect") and the header. The header always shows SSH and RDP, dashed with "+" when off, which opens Settings. VNC shows only when on. The Web menu uses the Menu role. The RA-020 badge has text and a tooltip. The route openers forward the protocol. | 6.1, DEV-020–022, RA-020, RA-030, WEB-003, MOT-005 |
| T3, then T5 | Metric bars on the card, now WP-33's `MetricBars`. The bars read live `readings` and `metricsAdded`. The empty state has the same geometry. | MET-004, MET-007, MET-008, MOT-007, MOT-008 |
| T4 | The header fits its own width: when one line does not fit, it switches to compact rows, and the skeleton switches with it. | DEV-021, MOT-007 |
| T5 | `WakeButton` in the card actions for a device with a MAC address. | RA-031 |
| T6 | The Web button opens the web tab through the session strip. A `SYSTEM_BROWSER` interface on http goes to the system browser. An https interface always goes to the tab, so the pin is checked. | WEB-003, WEB-004, WEB-008, WEB-011 |

## Files outside the owned paths (all named to the programme)

- **Route import lines** (T1): `routes/devices/+page.svelte`, `[id]/+page.svelte` and `[id]/detail.svelte.ts`. Also fixture fields in `[id]/page.fixture.svelte.ts`.
- **Route functions and props:**
  - `connect` takes the protocol (T2).
  - `wakeApi` and the `wake` prop (T5).
  - `openWeb` and the `onOpenWeb` prop (T6).
  - The `[id]/page.test.ts` cases for the web tab.
- **`lib/forms/**`** (T1, as granted): imports only. `SettingsPanel.svelte`: one import.
- **New `lib/web-interfaces/`** (T1, accepted by the programme).
- **Sessions** (T6, ruling):
  - `lib/sessions/tabs.ts`, `sessions.svelte.ts` and its test.
  - `strip/SessionPanes`, `SessionTab`, `SessionTabs` and `panes.test.ts`.
  - The new `lib/sessions/web/open/` inside WP-31's folder. The programme may move it.

## Contract change requests

None.

## Handoffs

- **Sessions owner (WP-32 or programme): live status in an open web tab.** The tab holds the device status as it was when the tab opened. A device that goes Down while its tab is open does not show the Down page (WEB-010). The fix is a status store keyed by device that `SessionPanes` reads. `overlayOpen` is fixed `false`.
- **Programme or motion owner:** one shared `endTransitionsAtOnce` under `lib/motion/` (WP-33 H-1 item 4).
- **WP-34:** the motion audit covers the Web menu, the header's compact-rows switch, and the background-colour transitions on the buttons and menu choices. Those transitions are the known programme-wide MOT-006 could.

## Open risks

- The DEV-001 200-card jsdom test hit its bound under machine load. The bound is now 60 s (ba5e799). It is on the flake list.
- The header switches to compact rows at 56rem of its own width, so a 1024 px window gets compact rows even for a short button set. This was measured as correct, but it gives up space.

## Open coulds (issue #1)

- The table is 1506 px wide at 1280 px, so the Connect column sits off-screen until the table is scrolled.
- Button borders are 1.36:1 and the bar track 1.17:1 against the card (UI-004).
- No Wake button in the table row or the header.
- `DeviceActions` restyles `WakeButton` through `:global(.status)`.
- The web tab's close label uses operator text.
- The https page test checks only the tab count.
- The warning wording exists twice, once in `lib/devices/actions` and once in `lib/forms/device/remote/warnings.ts`.
