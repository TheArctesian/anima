# WP-22 report — Hub RDP and VNC sessions, live list

Status: **done.** Package branch `claude/wp-22-rdp-vnc-hub`. Plan, decisions and every review round are in
[WP-22.md](WP-22.md). Accepted tasks went to the programme one at a time (R-067):

| Merge sent | Contents |
| --- | --- |
| 18b57ab | T1: `k4_8_5` deterministic (test only) |
| df71493 | T2: guacd 1.6.0 `args` fixtures for `rdp` and `vnc` |
| 2152393 | T3: migration 0610, `Protocol` RDP/VNC, `SERVER_CERTIFICATE_MISMATCH`, `ServerCertificate`, `Session.clientAddr` |
| 2e6b5c4 | T7 deny list, T4 certificate reader and `acceptServerCertificate`, T5 RDP/VNC on `/tunnel` |
| 9ef8b21 | T8 live feed, T9 certificate pin (4003), T6 relay policy |
| 22dedda | T10 wake before the session |

## What the hub does now

- RDP and VNC run through guacd on `/tunnel`, which is mounted in `lib.rs::tunnels()` behind NET-002. SSH no
  longer uses guacd; `/terminal` is unchanged except for the deny list.
- `startSession` checks each protocol: on, and for RDP an accepted certificate unless security is `RDP` or
  `ignoreCertificate` is on.
- The K4 §3 values are tested against recorded guacd 1.6.0 `args`. `drive-path` is
  `<ANIMA_GUACD_DRIVE_ROOT>/<device_id>` (RA-021).
- RDP certificate (D-5): the hub reads it itself (X.224, then TLS, never CredSSP). `testConnection` reports
  it under WP-21 D-3, and `acceptServerCertificate` pins it with an audit entry.
- Before the handshake the hub reads the certificate again. A change ends the session: 4003,
  `SERVER_CERTIFICATE_MISMATCH`, Error event.
- `ANY` goes to guacd as the negotiated TLS mode (CR-2).
- Copy, paste, microphone and drive flags are enforced in the relay and in guacd (D-6).
- `ANIMA_PROBE_DENY` applies to every session connection (P2-004, D-7). This closes R-032 E-7 and WP-23 H-4.
- `sessionChanged` (D-15, RA-034) is published after each commit. Files relays join the shutdown set
  (WP-27 handoff b).
- Wake-on-LAN runs before a session to a Down device (K4 §1, R-074).
- K1 has no `@pending` left.

## k4_8_5

The cause was the test's own timing, not a relay race. The client drained the flood and then sent nothing
for longer than the 2 s idle limit, so `TIMEOUT` was correct. The test now waits for the hub to read its
ping (a WebSocket ping and pong) and stops the flood after 800 ms of quiet. With the peek guard removed, it
fails every run. About 0.9 s of margin is left before the hub's legitimate 2 s send limit.

## Contract changes

- **CR-1** (accepted): `tunnel_router(db, vault, bus, clock, guacd, drive_root, live)`.
- **CR-2** (accepted): the K4 §3 `security` wording, now in `contracts/tunnel.md`.
- Stale K4/K3 text about `/tunnel`, §6 and `drive_root`: fixed by the programme on phase-2.

## Handoffs

See WP-22.md "Handoffs". Edits outside owned paths: `sessions.rs` face, `lib.rs` `tunnels()`, `main.rs`
(one policy), one `schema.rs` loader line and one Subscription line, `devices/graphql/remote/rdp.rs`,
`guac-protocol` `ConnectParams.audio`, `sessions/link/connect.rs`, `targets.rs`.

**To WP-24 (client):**
- Handle `SERVER_CERTIFICATE_MISMATCH` (close 4003): show the warning modal and offer a new test, not
  "Reconnect".
- `ConnectionTestResult.serverCertificate` is null when it equals the accepted certificate.
- `authenticated` is always false for RDP and VNC tests: the hub sends no credential in a test.
- `wakeSent` arrives on the ticket. A woken opening can take up to 120 s.
- When paste is off, a dropped clipboard does not end the session. Refused `audio`, `file`, `get` or `put`
  close the tunnel with 1002, so the client must not send them when the policy is off.
- Folder listings still work when download is off.
- `live.svelte.ts` comments still say the hub may not serve `sessionChanged` (WP-32).

## Needs a real machine to accept

- RDP to Windows with NLA: resolution follows the window (RA-004), clipboard (RA-005), Ctrl+Alt+Delete
  (RA-006), the certificate pinned at first use and a change ending 4003. Check the `sha256:` form of
  `cert-fingerprints`: it was reasoned from the FreeRDP source, not checked against a live server.
- VNC to macOS Screen Sharing (username required) and to a Linux desktop.
- Wake-on-LAN through a real site relay.
- Hyper-V (`VMCONNECT`): the hub's plain X.224 read has no preconnection blob, so it may fail closed with 503.

Live gateway steps: none beyond WP-23's. The Headscale grant for the Windows device (tcp:3389) is WP-23's
template.

## Open coulds (for issue #1)

- `ArgValues` copies the password into a `String` that is not zeroed (already on issue #1).
- A woken redeemed opening has no cap of its own; the ticket cap applies before the wake.
- The terminal `Timeout` text is vague after a 120 s wake.
- The certificate tests for nla, nla-ext and tls could be one table-driven test.
- `vnc.rs` banner check has no property test.
- Lagged branch of the `sessionChanged` stream is untested.
- Old `R-0xx` and `WP-08 H-n` references in comments that predate this package.
