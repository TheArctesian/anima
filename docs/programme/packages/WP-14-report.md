# WP-14 report (package orchestrator)

**Status: done.**
- **Branch:** `claude/wp-14-ssh-tab`, with the latest `phase-1` (d491d67, including WP-16) merged in. The code tip is **10d0b4b**, and this report is committed on top of it.
- **Pushed or merged:** neither.
- **Gates:** `bun run gates` (DATABASE_URL unset) on 10d0b4b: **12/12 pass** (hub 706 + 523 + 80 + others, client 791/791). The earlier run on 32fafe9 failed only the WP-09 ICMP flake (`chk_012`), and gates 4–12 passed on their own; the reviewed code is unchanged since, apart from the phase-1 merge.
- **Plan, decisions, rulings and review log:** [WP-14.md](WP-14.md).

**Not touched:** nothing ran against the VPS, Headscale or any cloud account, and nothing under `contracts/` was edited (K4 §4 was amended on phase-1 by the programme under R-046). Shared files changed are only the handoff edits listed under H-4.

## What is delivered

| Task | Output | Spec |
| --- | --- | --- |
| T1 `tunnel` | `sessionsApi(hub)` (startSession, endSession, testConnection, acceptHostKey). Vendored Apache guacamole-common-js **1.5.5** (R-045, R-046) with a hand-written `.d.ts` of only what is called, an import boundary lint rule and a SHA-256 pin. `openTunnel` / `startTunnel`: the token only in the connect query (D-3); close codes observed through `withObservedSocket` (1.5.5 still maps them to 0x0200); `tunnelUrl` https→wss, http→ws to loopback only. Pure `ending` mapping with fixed texts; the recorded `endReason` wins (D-4). Fakes for tests. | K4 §2–§6, RA-007, SEC-002, LOG-007, R-035 |
| T2 `view` | `SessionView`: display sized to the pane × dpr, DPI and IANA zone, debounced clamped `size` (D-7); the display mounted only when ready and never transformed or blurred (MAT-008). Keyboard bound only while visible, window-focused, ready and DOM-focused; held keys released on leave (D-8). Clipboard (D-9, as changed in WP-14.md). Keys menu (D-10, RA-006). Connection steps; ended panel with Reconnect (RA-007); host key warning modal: Close default, review, second confirmation, accept the shown fingerprint only, then Reconnect (D-6, SEC-007). | RA-003, RA-006, RA-007, SEC-007, MAT-008, MOT-007 |
| T3 `tabs` | Session store (`createSessions`, `openSession`, select, close, move, pop-out port); tab strip with open motion from the origin, close-to-place, reorder drag with velocity projection and snap, drag-out, rubberband without a port; tab menu; roving tablist with Delete and Shift+F10; strip presence that moves the page once, never mid-animation. | RA-001, RA-002, MOT-005, MOT-030–035, UI-009, UI-013 |
| T4 `wire` | The view in every pane (hidden panes mounted and inert). Only the view calls `startSession`; the store wraps it (`startIn`) and ends every started session exactly once: tab close, Reconnect, replacement of a live session, close during an in-flight start, concurrent starts. Window and page focus feed `focused`. Covered pages have zero height and keep their scroll. | RA-001, RA-002, D-4, D-5, D-11, D-13 |

## Rulings and escalations

- **R-044** (E-1, E-2), **R-045** (E-3), **R-046** (E-4) and the SHOULD-1 pop-out ruling: see WP-14.md.
- **E-4 / R-046 (security).** Apache 1.5.0's tunnel parser counts UTF-16 units; the hub counts code points. A remote host could set its terminal title to emoji followed by Guacamole text and inject `clipboard`, `blob`, `end` or a forged `error`. Fixed client-side by Apache 1.5.5 (`Parser.codePointCount`) and hub-side by the U+FFFD guard (`claude/wp-10-bmp-guard-r046`, e15fe1b, merged). 1.5.0 also ran out of memory on one probe.

### Source and signature checks

**E-1 / E-3 source check (R-044, R-045).** The npm `guacamole-common-js@1.5.0` (integrity `sha512-zxztif3G…rgJg==`) was compared with the Apache Guacamole 1.5.0 source release `guacamole-client-1.5.0.tar.gz` from archive.apache.org (SHA-256 `e44d96fd736b2e5c2d73fcc48a9424f3869b8aaa75215ebf00af4afa2d20691c`, equal to Apache's `.sha256`). The 34 files of `guacamole-common-js/src/main/webapp/modules/` were joined in byte order of name (the npm build script's `cat modules/*.js`) and diffed against the npm `dist/esm/guacamole-common.js` without its last line; nothing fetched was executed. Result: 636 differing lines. 31 modules are identical; Client.js, Display.js and Tunnel.js differ in working code. The npm build matches no Apache release (1.5.1–1.5.5 and 1.6.0 differ by 638–1,423 lines); of its 166 added code lines, 144 appear in later Apache releases and 22 in none. Ruling R-045: vendor the Apache modules instead.

**1.5.0 signature.** `guacamole-client-1.5.0.tar.gz.asc` verified with gpg against `https://downloads.apache.org/guacamole/KEYS` in a throwaway GNUPGHOME: `GOODSIG`/`VALIDSIG`, signed 2023-02-14 by key `F467 E54A CC52 F1D2 7788 2686 5B29 77AE E5E4 518F` (Michael Jumper, CODE SIGNING KEY). The key is the only one in KEYS and is not certified by a key we trust, so the trust anchor is the KEYS file served by apache.org.

**R-046 source and signature (1.5.5, the vendored version).** `guacamole-client-1.5.5.tar.gz` from https://archive.apache.org/dist/guacamole/1.5.5/source/, SHA-256 `8251a4dcec11f0b878f836f8223e81aa049a937f70b06410d320e60acb727258`, equal to its `.sha256`. `.asc` verified the same way: `GOODSIG`/`VALIDSIG`, signed 2024-03-30, the same key. Vendored file `apps/client/src/lib/sessions/tunnel/vendor/guacamole.js` = the 34 modules joined in byte order of name plus `export default Guacamole;`; joined-modules SHA-256 `d5c167c0c3c40aae5fa25177dc56465b947ec24ab1a5a7f60e7009684525c9df`, pinned by `vendor.test.ts`. The T1 security re-review downloaded and verified all of this independently and found the file identical. The 1.5.5 source still sets `Guacamole.API_VERSION = "1.5.4"`.

**1.5.0 → 1.5.5, Parser, Tunnel and Client.** Only Client.js, Parser.js, Tunnel.js, SessionRecording.js and Version.js differ. Parser reads element lengths in code points (`codePointCount`, `toInstruction`); WebSocketTunnel and HTTPTunnel use `Guacamole.Parser` instead of their own UTF-16 parsers and send through `toInstruction`; a parse error closes the tunnel; the first internal instruction sets the UUID only with exactly one argument; pings and the Client `nop` keep-alive become rescheduled timeouts. Unchanged: the socket is private and close codes are still mapped to 0x0200 (so `withObservedSocket` stays); client states are private; the Client still does not watch its tunnel.

## Review

Every task was reviewed by agents that did not build it: Opus security reviews for T1, T2 and the package; Sonnet interface reviews measuring WebKit and Chromium per frame for T2, T3 and the package; a Sonnet code review for T1 and T3. Accepts are backed by probes and named mutations. The final package security review accepted with one SHOULD (pop-out), ruled and fixed. The final interface recheck at 32fafe9 accepted (scroll, 360 px width, teardown error, D-8 and regressions measured in WebKit and Chromium). Full log: WP-14.md.

## Handoffs

- **H-1 WP-12:** no device SSH button exists yet. Call `useSessions().openSession({ deviceId, deviceName, protocol: 'SSH', origin: button.getBoundingClientRect() })`; show "SSH" only when SSH is enabled and a host key is accepted (else the connection test; `startSession` refuses with CONFLICT).
- **H-2 WP-16:** pass a `PopOutPort` — `(device: PoppedOutDevice) => Promise<void>`, with `PoppedOutDevice { deviceId, deviceName, protocol }` — as the second argument to `createSessions` in `routes/+layout.svelte`. The window starts its own session with `openSession`; on resolve the store removes the tab and ends its session (`OPERATOR`); on reject the tab stays. WP-16 also edits the `createHub` line in the same file.
- **H-3 WP-17:** end-to-end against the real `/tunnel` (after WP-05 T4 and the WP-10 mount), including a host key change. Also check in the Tauri WebKit shells:
  - `navigator.clipboard.writeText` without a user gesture (gateway copy may be refused);
  - whether WebKitGTK fires `paste` for Ctrl+Shift+V on the non-editable pane (if not, Linux has no paste shortcut, only the Keys menu);
  - the stage height and the 43 px toolbar with the real preloaded fonts;
  - guacd 1.6.0's paste on Ctrl+V after a clipboard stream (confirmed from source, `terminal.c:1446`).
- **H-4 programme:** shared files changed: `eslint.config.js` (vendor boundary, computed `import()`, wider client globs; merged with WP-16's Tauri boundary so every list refuses both, outside their own folders), `quality/generated.txt` (the vendored file), `routes/+layout.svelte` (strip, stage, panes, the sessions port, covered-page scroll, `minmax(0, 1fr)` columns; the `createHub` line untouched), and `lib/nav/Nav.svelte` (optional `onnavigate`). No `package.json` or `bun.lock` change: the library is vendored, not an npm dependency.

## Deferred (for issue #1)

- WebKit prints a console line containing the tunnel URL (with the ticket) when a WebSocket fails to connect. The ticket is single-use and expires in 30 s, but it is visible in devtools.
- Paste guard: guacd 1.6.0 has no bracketed paste, so multi-line clipboard text runs at once. Consider a confirmation for multi-line or large pastes (T2 security C4).
- The vendor boundary lint can still be bypassed by `new URL('…/vendor/guacamole.js', import.meta.url)`, `import.meta.glob` and a directory symlink; add selectors and a no-symlinks quality check.
- No keyboard reorder of session tabs (pointer drag only).
- The terminal pane takes Tab (D-8), so keyboard users leave it only by pointer or by a window shortcut; document or add an escape chord (WCAG 2.1.2) together with WP-16's window shortcuts.
- The strip's first appearance is one planned layout move (Chromium CLS 0.018 on the first open). Reserving the strip height before the first tab would make it 0.
- The nav stays 192 px wide at 360 px (WP-11), so a session pane is only 120 px wide. In it, the steps and ended panels scroll sideways: the Reconnect button (143 px) is clipped by 47 px with its focus ring, and the device name truncates to nothing. Fix with a collapsing nav (WP-11) and a wrapping panel row (UI-009).
- The host key modal is a `div` with a focus trap, not a `<dialog>`; consider `<dialog>` for native inertness.
- RDP (RA-004/005) is Phase 2.
- `api_005_the_hub_schema_batches_sites_devices_credentials_and_host_keys` (WP-06 wiring) failed once under load: 106 statements against a bound of 80 for 163 devices, because a 1 ms DataLoader window splits batches on a busy machine. It needs the same treatment as WP-09's batching tests (a longer injected window, as `devices::batching` does).
- Old PostgreSQL processes from earlier test runs (PIDs 4647 and 51272) were left running on the build machine.

## Side work done during WP-14 (each reported and merged separately)

- `claude/wp-10-privileges-flake` (07665cf): role-setting test connections are detached from the pool (WP-10, WP-05, WP-09 helpers).
- `claude/wp-10-bmp-guard-r046` (e15fe1b): R-046 hub guard.
- `claude/wp-10-tunnel-timing` (90218e0): the D-9 and D-10 tunnel wire tests wait on their conditions against an injected end wait.
