# WP-24 report (package orchestrator)

**Status: done.**

- **Branch:** `claude/wp-24-rdp-vnc-tabs`. The code tip is **a12e7b5**, on phase-2 23c4287; this report is committed on top of it.
- **Merged into phase-2 (R-067):** d7005d3 (T1, T1b, T4) as b666c76; a75b7e7 (T3, T2) as 3ad2c24; a12e7b5 (T5) sent for merge. Nothing pushed.
- **Gates:** client-only package, so under R-068 each task ran gates 4–12 one at a time (no change in `crates/`, `Cargo.*`, `contracts/`). T1 and T1b also passed all 12 before R-068. The programme orchestrator runs 12/12 on phase-2 after each merge.
- **Plan, decisions and review log:** [WP-24.md](WP-24.md).
- **Not touched:** `contracts/`, `routes/`, `lib/devices/`, `lib/hub/`, the VPS, Headscale, cloud accounts and secret files.

## What is delivered

| Task | Output | Spec |
| --- | --- | --- |
| T1 `protocols`, T1b `generated` | `startSession` takes RDP and VNC; the device's transfer policy is read at each start and fails closed; close 4003 and `SERVER_CERTIFICATE_MISMATCH` have their own text; the new calls (`setCredential`, `acceptServerCertificate`, `testConnection` for RDP) on generated K1 documents; `sessionStart` serves both tunnels without a change to the root layout | CON-003, RA-033, K4 §1, §5, LOG-007 |
| T2 `remote-view` | RDP and VNC tab over the Guacamole tunnel: display never animated, keyboard bound only while visible, focused and connected, Keys menu with Ctrl+Alt+Delete, text clipboard both ways under the policy, RDP resize after 150 ms, VNC scaled to fit, ended panel with Reconnect | RA-001, RA-002, RA-004–007, RA-030, RA-033, MAT-008 |
| T3 `prompts` | Credential prompt for the gateway's `required` (only the protocol's own names; password saved only with "Save password"); RDP certificate change modal that accepts exactly the fingerprint shown | GW-015, SEC-007 |
| T4 `terminal-policy` | SSH copy off and paste off enforced in the client on every path, including the Linux primary selection | RA-033, K4 §8.7 |
| T5 `wire` | Prompt and certificate modal in the remote view (visible tab only, keyboard released, early `required` kept); fixed-width protocol label on RDP and VNC tabs; existing tab-open motion | GW-015, MOT-007, Section 5.7 |

## Contract change requests

None.

## Handoff notes

- **WP-26 / WP-33 (device page and header, `routes/devices/`):** the RDP and VNC buttons call `useSessions().openSession({ deviceId, deviceName, protocol: 'RDP' | 'VNC', origin: button.getBoundingClientRect() })`. Until then no page opens an RDP or VNC tab.
- **WP-22 (hub):** the client expects K4 as written: `tunnelPath` `/tunnel` for RDP and VNC, close 4003 for a changed certificate, `required` with K4 §4 names, `clipboard` only when the policy allows.
- **WP-34:** background-color transitions on the ended panel, the modals and the tab (MOT-006) are programme-wide; decision IDs in session code comments to swap for spec or K4 IDs (COM-002).

## Open risks

- **RA-005 on WebKit:** the device's clipboard text is written to the system clipboard from a socket message, not a user gesture; WebKit may refuse it in the macOS app. Not provable in jsdom; check in the end-to-end run (WP-22, WP-34).
- RDP and VNC run end to end only once WP-22 serves `/tunnel` for them; the interface was measured in a throwaway harness on the fake socket.

## Open coulds (issue #1)

Microphone input and the shared drive (D-6); the `onclient` test seam in `RemoteView`; `KEYSYM` and `PASTE_STROKES` exported but used only by tests; no test cites RA-030; one redundant certificate `read()` guard; the clipboard text WebKit fix above.
