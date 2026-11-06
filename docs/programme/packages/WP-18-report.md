# WP-18 report (package orchestrator)

**Status: done.**

- **Branch:** `claude/wp-18-ssh-terminal`, from `phase-1` fa49e57, with `phase-1` 9e5a18c (WP-19) merged in. The code tip is **8ee79a1**; this report is committed on top of it.
- **Pushed or merged into phase-1:** neither.
- **Gates:** `env -u DATABASE_URL BASE_REF=fa49e57 bun run gates`, **12/12 pass**. They ran on ca8e9e2 (the package after the phase-1 merge) and again on 8ee79a1 (after the final review fixes).
- **Plan, decisions and review log:** [WP-18.md](WP-18.md).
- **Not touched:** the VPS, Headscale, any cloud account and `deploy/` (including `deploy/headscale`). No secret files were read.

## What is delivered

| Task | Output | Spec |
| --- | --- | --- |
| T1 `contract` | K4: SSH uses a new terminal tunnel (Section 8: request, opening, host key, messages, limits, ends, client terminal rules); `tunnelPath` is `/terminal`; a ticket is valid only on its path; Section 7 tests. K3: `terminal_router`. Spec text: RA-010, RA-011, UI-011, GW-001, GW-015, Section 6.3, the glossary, the gateway scope, PVE-033 and the Section 13 tree. | R-062 |
| T2 `hub` | `sessions/terminal/` serves `/terminal` (subprotocol `anima-terminal`). It opens SSH with `russh` to an address checked by R-032 and pins the accepted key: no authentication request is sent unless the key matches. It signs in with the stored credential (password; keyboard-interactive with at most 4 rounds of 4 prompts; public key). It opens a PTY (`xterm-256color`) at the size from the first `resize`, then the login shell. Binary frames carry bytes both ways; text frames are a closed JSON set (`resize`, `ping`, `end`). Limits: 64 KiB per message and 256 B per text message; 5 s pings; 15 s silence and stall limits; a 4-message channel buffer for backpressure. Channels the device opens are refused. Ends follow K4 §8.6, with the row, audit entry and event written once. Shared code was moved, not copied: `sessions/ssh/`, `sessions/end/`, `sessions/admission.rs`. `main.rs` mounts `/terminal` and no longer mounts `/tunnel`. | SEC-005/006/007, API-009, NET-002, R-032, R-034, RA-008, LOG-007, OD-9 |
| T3 `client` | The SSH tab draws with xterm.js 6.0.0, with fit, WebGL (DOM renderer fallback), search, web-links and Unicode 11, all pinned exactly. It uses the Nord theme from tokens, OpenDyslexicM Nerd Font Mono at 12 px, 5000 lines of scroll-back and a minimum contrast of 4.5:1. Terminal rules: OSC 52 is swallowed; links are http/https only and open after a confirmation that shows the host and the full URL (links with userinfo or over 2048 characters are refused); titles have controls stripped and are cut to 80 characters, shown next to the device name at a fixed tab width. Paste goes through one sanitiser, so an embedded `ESC[201~` cannot end a bracketed paste. Above 16 MB of undrawn output, further output is skipped with one notice and the shell keeps running. There is a Find bar, a Keys menu that sends bytes, and notices that overlay the toolbar so the pane never moves. The wire client is in `tunnel/terminal/`; xterm sits in `screen/` behind an ESLint boundary. The Guacamole client code is kept for RDP and VNC and is off the SSH import path. | RA-001–003, RA-006, RA-007, RA-010, RA-011, SEC-007, LOG-007, MAT-008, MOT-007 |

**Acceptance point:** OSC 7 and OSC 133 (fish's `file://…` and `;click_events=1`) render nothing. The interface review measured this in WebKit and Chromium with a fish-like stream; see the review log. The operator's own fish prompt, in the real app against the real hub, still needs checking after deploy (see "To try it").

## Review verdicts

| Task | Reviewers and rounds | Verdict |
| --- | --- | --- |
| T1 contract | reviewer (Sonnet): 3 rounds | accept |
| T2 hub | reviewer-security (Opus): 3 rounds; reviewer (Sonnet): 3 rounds, then a narrow mutation round run by the builder under the programme's ruling | accept |
| T3 client | reviewer-interface (WebKit and Chromium): 4 rounds; reviewer-security (Opus): 2; reviewer (Sonnet): 2 | accept |
| Package | reviewer-security (Opus), whole diff | accept; its should and one could were fixed (8ee79a1) |

## Decisions (from WP-18.md)

- **D-1:** the Guacamole tunnel is kept but not mounted. Only SSH exists, so nothing uses `/tunnel`. The module, its tests and the vendored `guacamole-common-js` stay for RDP and VNC. The programme may rule to delete them instead.
- **D-2:** the hub opens and signs in before the upgrade, and starts the PTY after the first `resize`.
- **D-3:** a device with no stored credential ends `AUTH_FAILED` (see E-1).
- **D-4:** xterm.js 6 has no canvas renderer, so the fallback is the DOM renderer.
- **D-5:** keyboard-interactive is answered with the stored password. This now also applies to `testConnection`, which still pins the key first.
- **D-6:** shared pieces were moved, not copied.

## Contract and spec text changed (under R-062)

- **`contracts/tunnel.md` (K4):**
  - Header: two tunnels.
  - §1: `tunnelPath` is `/terminal` for SSH, a ticket is valid only on its path, and a start needs an accepted host key.
  - §2–§4: marked as the Guacamole tunnel (RDP and VNC); the §3 SSH table is marked historical.
  - §5: a note that §8.6 lists the terminal tunnel's causes.
  - §6: terminal pinning.
  - §7: terminal tests.
  - **§8, new.**
- **`contracts/hub-domains.md` (K3):** `terminal_router` added; `tunnel_router` is not mounted until RDP or VNC uses it.
- **`contracts/tokens.md`:** `guac_color_scheme` is used only by the Guacamole tunnel.
- **`docs/design-spec.md`:** RA-010, RA-011, UI-011, GW-001, GW-015, the glossary entry for Gateway, the gateway scope paragraph, the Section 6.3 procedure, the Section 6.3.1 note, the Section 5.5 output row, PVE-033 and the Section 13 tree.
- **For the programme:**
  - R-062 names only K4 §8 and RA-011. The K3 line, RA-010, GW-015, the `tokens.md` line and the edits to K4 §1–§6 follow from them; please amend R-062 to cover them.
  - K1's `PROTOCOL_ERROR` doc comment still says "instruction that does not decode (GW-011)". It is left for the next K1 opening.

## Escalations and requests

- **E-1:** GW-015 (a prompt for a missing credential) does not apply to SSH any more. guacd used to prompt inside the terminal; the hub does not. A hub-side prompt (read the typed line, no echo, never kept) is proposed for issue #1. Please add it to the escalation list or issue #1 (R-007).
- **K4 §8.5 close code:** two over-cap tests accept a 1002 close or a connection reset after `end PROTOCOL_ERROR`, because the OS reset can overtake the close frame. You ruled this acceptable. Should §8.5 say "1002 or reset"? The Guacamole tunnel likely behaves the same and should be checked.

## Handoffs

- **H-1 (hub wiring, WP-06/WP-10):** `crates/hub/tests/it/wiring/sessions_schema.rs` now expects `tunnelPath` `/terminal` (one line).
- **H-2 (shared files):**
  - `eslint.config.js`: an xterm import boundary. `@xterm/*` is allowed only under `sessions/screen/`, and every other folder bans it.
  - `apps/client/package.json` and `bun.lock`: the six `@xterm/*` packages.
  - `quality/generated.txt`: no change.
- **H-3 (Tauri shell, WP-16):** confirmed links do nothing on desktop. `window.open` returns nil because `apps/desktop/src-tauri` sets no new-window handler. Open confirmed http/https links through the opener plugin or a shell command scoped to http and https. Never answer `NewWindowResponse::Allow`: the new view would share the opener's configuration and expose Tauri IPC to the remote page. Until then, links can only be copied.
- **H-4 (shared Modal):** in WebKit, the shared `Modal`'s focus trap lets Tab skip buttons. Keyboard users can still use the link confirm, but the trap should be fixed where `Modal` lives.
- **H-5 (WP-17):** at 360 px the page scrolls sideways because of the Devices page cells, not this package.

## For issue #1 (R-007)

- E-1: an in-terminal prompt for a missing credential.
- A flow-control acknowledgement in K4 §8. A browser WebSocket gives no receive backpressure, so the client skips output above 16 MB undrawn. K4 §8.5 should also record that skip.
- WebKit prints a console line that contains the ticket when a WebSocket fails, as noted in WP-14. Release builds have no devtools.
- A dynamic `import()` gets past every ESLint import boundary (motion, guacamole, Tauri, xterm). Add a `no-restricted-syntax` rule in one change.
- `ANIMA_GUACD` is still required at start, although nothing uses it until RDP or VNC.
- Bare plan IDs (`D-3`, `D-5`) collide between WP-10 and WP-18 inside `sessions/`. Replace them with K4 section references in one change.
- A device with a certificate host key gets the host-key warning, but its `testConnection` fails, so the operator cannot act on it.
- Offer only the accepted key's algorithm in the key exchange (T1 round-1 S1).
- `screenReaderMode` for xterm, a "refused link" notice for schemes xterm filters, and the ended panel covering the last output.
- In the web build (Phase 2), hidden-tab timer throttling may delay the 5 s client ping.
- Keyboard reorder of tabs and the Tab escape chord (from WP-14) remain.

## Open risks

- **Timers in a background window.** The client pings from plain timers. Ping cadence measured 5 s in WebKit and Chromium with the page visible. A hidden or App-Napped Tauri window could not be measured headless. If macOS suspends the webview for more than 15 s, the hub ends the session `TIMEOUT`.
- **Certificate host keys** are tested as a unit only: the fake server cannot negotiate certificate algorithms.
- **Russh's own buffers** hold copies of the password and of typed bytes that the hub cannot wipe. This is the same limit as the guac-protocol note.

## To try it (the operator)

1. **Redeploy the hub** from the merged branch. There is no new environment variable and no change to `compose.yaml` or the gateway image: the hub already reaches devices over SSH on the `gateway` network for `testConnection`. `ANIMA_GUACD` must still be set, as today.
2. **Rebuild the desktop app**, which gets the new client with the xterm dependencies (`bun install` first).
3. Open an SSH tab to the Mac (`device-stephens-mac`). The fish prompt should show no `ile://…` and no `;click_events=1`. Also try typing, resizing the window, Find (Cmd+F), copy (Cmd+C on a selection), paste and the Keys menu.
4. A link in the terminal opens the confirmation, but "Open link" does nothing on desktop until H-3 is done.
