# Proposal: SSH in a real terminal (hub SSH client, xterm.js in the app)

From: the orchestrator. The operator chose this in chat on 2026-11-06 ("I like option 2"), after
the SSH tab showed `ile:/…;click_events=1` in front of the prompt.
Status: a proposal for a ruling (R-062) and for issue #1 (R-007). Not Phase 1 scope: it changes
the frozen K4 tunnel contract (CON-001) and spec rules RA-011, RA-040 and RA-041.

## Problem

SSH goes through guacd, which runs the terminal emulator on the server and streams the screen to
the app as images (RA-011). Its emulator is small:

- It prints the payload of any OSC sequence it does not know. Shells send these all the time:
  OSC 7 (current folder; fish, zsh and bash with vte.sh) and OSC 133 (prompt marks; fish 4,
  many zsh themes). guacd 1.6.0 shows `ile://host/path` and `;click_events=1`. Upstream fixed
  this (GUACAMOLE-2213, April 2026) but no release has the fix yet.
- The text is a picture: selection, copy, search and links depend on guacd's own handling;
  fonts blur at fractional zoom; the font must be installed in the gateway image.
- Each keystroke and each repaint crosses hub → guacd → hub → app as image updates.

## Proposal

SSH sessions use a terminal in the app. The hub runs the SSH connection. guacd stays for RDP and
VNC only.

1. **Hub SSH client.** A new part of the `sessions` domain opens the SSH connection with `russh`,
   which the hub already uses for Test connection and the session host-key check (K4 §6, R-034).
   Same target rule (R-032), same host-key pinning: the session connects only when the key
   equals the accepted key, else it ends with `HOST_KEY_MISMATCH`. The stored credential is
   decrypted in the hub and used only for authentication; it never reaches the client (OD-9).
   The hub asks for a PTY with `TERM=xterm-256color` and the client's size, and a shell.
2. **Terminal tunnel.** `startSession` returns a ticket as today (K4 §1: 32 bytes, 30 s, single
   use, bound to operator, session and client address). For SSH, `tunnelPath` is `/terminal`.
   The WebSocket (subprotocol `anima-terminal`) carries:
   - binary frames: terminal bytes, both ways, as they are;
   - text frames: JSON control messages, a closed set: `resize {cols, rows}`, `ping`, and from
     the hub `end {reason}`.
   Limits as in K4 §4: the hub pings after 5 s of silence and ends after 15 s without the
   client; a frame size cap; backpressure, so a slow client stalls the SSH channel instead of
   growing a buffer. End reasons and events as in K4 §5.
3. **Client.** `@xterm/xterm` with the fit, WebGL (canvas fallback), search, web-links and
   Unicode 11 add-ons. Theme from the design tokens; the OpenDyslexicM Nerd Font Mono the client
   already ships; the existing tab, toolbar and "Keys" menu stay.
4. **What the terminal may do.** xterm.js parses OSC itself, so the junk is gone. It also
   supports things a remote host could misuse, so each is set on purpose:
   - OSC 52 (write to the clipboard): off. A remote host must not fill the operator's
     clipboard unseen.
   - OSC 8 (hyperlinks) and detected links: open only after a confirm that shows the full URL.
   - Window title (OSC 0/2): shown in the tab after stripping control characters, at most 80
     characters.
   - No sixel or image protocols in the first version.
5. **What stays on guacd.** RDP and VNC, unchanged. GW-001 holds (official image plus fonts);
   the terminal font can leave the gateway image once no SSH session uses guacd.

## Spec and contract changes

- **RA-011** becomes: the client renders SSH with xterm.js; the gateway no longer needs the
  terminal font for SSH.
- **K4** gains Section 8, the terminal tunnel (`/terminal`, frames, limits, ends). Section 6
  (host keys) applies unchanged; the gateway handshake parts apply to RDP and VNC only.
- **RA-040 / RA-041 (recording, Phase 3):** SSH recordings become asciicast v2 files, encrypted
  like the other recordings, played with xterm.js. RDP and VNC keep Guacamole recordings.
- **Session sharing (Phase 3):** the hub fans the terminal stream out to joined viewers, instead
  of guacd's join.
- **RA-008** (no content recorded unless recording is on) is unchanged: the hub relays bytes and
  keeps nothing.

## Risks and answers

- **More attack surface in the hub.** The hub now parses SSH, not only the Guacamole protocol.
  `russh` is already in the hub for probes; the session path adds channels and PTY. It needs the
  security review (reviewer-security) and fuzzing of the control-frame parser.
- **Terminal escapes from a hostile host.** Handled by item 4; xterm.js has no code-execution
  sequences.
- **Two remote-access paths.** RDP/VNC (guacd) and SSH (terminal) share tickets, session rows,
  end reasons and events, so the session list and audit stay one.

## Until then

SSH stays on guacd, and shells that send OSC 7 or OSC 133 show their payload in front of the
prompt. The guacd patch (GUACAMOLE-2213 on 1.6.0's terminal library) was not chosen; guacd 1.6.1
will carry the fix for RDP and VNC users of the terminal anyway.

## Needs

- Ruling R-062 (made) and an issue #1 entry (R-007).
- The spec and K4 changes above, after Phase 1 is accepted.
- A work package: hub terminal session and tunnel (builder-heavy, security review), client
  terminal tab (builder, interface review), tests per K4 §7 plus a fake SSH server.
