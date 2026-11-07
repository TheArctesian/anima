# WP-27 report — Hub SSH link, SFTP and Wake-on-LAN

Status: **done.** Package branch `claude/wp-27-link-files-wake`. Plan and review log: [WP-27.md](WP-27.md).

## Tasks

| Task | Commit | Base | State |
| --- | --- | --- | --- |
| T1 `link` | 7a91d75 | 8398255 | Merged into `phase-2` at 531a91b (R-070 names the error `LinkError`) |
| T3 `files` | 631c3ad7 | `phase-2` f6614b22 | Merged into `phase-2` at e527c6e |
| T2 `wake` | e73c2261 | T3 | Merged into `phase-2` at 50fb013; gates 12/12 |
| T4 `follow-ups` (round 2 of T2, T3) | e8ff3b9d (gated as fee12396, 12/12) | 50fb013 | Accepted, SHA sent |

Each task had an Opus security review (with mutations on the link, the wake command and the SFTP relay) and a
Sonnet review, and none had must findings. The final package review accepted the package.

## What the hub has now

- **Link (P2-003, P2-004, SEC-007, CHK-012, MET-002).** `sessions::{link, SshLink, FixedCommand, CommandOutput,
  LinkError}`.
  - The link resolves the host once, and every address must pass the deny list and the target rule.
  - It pins the accepted key before any credential is sent.
  - It runs one `exec` per command, and a command holds only `&'static str` and the MAC the hub renders.
  - Standard output is capped at 4 MiB. The opening has 10 s, with 3 s for each address.
- **Wake-on-LAN (RA-031, VPN-006, D-2 provisional).**
  - `wake` runs `wakeonlan aa:bb:cc:dd:ee:ff` on the device's site relay, with a 10 s limit.
  - `wakeDevice` returns `WakeResult`.
  - Each wake writes an Info event (Error on a failure) and the audit entry `device.wake`.
- **Files (RA-032, K4 §9, D-14).** `openFiles` and `/files` (subprotocol `anima-files`) through the link, with
  only the `sftp` subsystem.
  - The transfer flags are read at each request, and an upload uses create-exclusive.
  - Each transfer writes the audit entry `file.upload` or `file.download` with path and size, and a fixed event.

## Contract change requests

- **CR-1 (K3).** `files_router` gains `policy: AddressPolicy` (accepted by the programme under PKG-001). If WP-22 T7 puts the deny list into `Live`, this
  parameter can go.

## Needs a real LAN device to accept

- **Criterion 8.** A sleeping device with a MAC, and a relay at its site with SSH, an accepted key and
  `wakeonlan` installed.
- **Criterion 6.** SFTP upload and download against a real SSH server.

## Handoffs

- **WP-28 and WP-29.** Use `sessions::link` and `SshLink::run`.
  - A run may be repeated on one link.
  - A non-zero exit status is `Ok`. `cut` means the 4 MiB cap was reached.
  - Standard error is dropped (LOG-007).
  - Static arguments go through `FixedCommand::arg`.
  - In-crate tests reach loopback with `AddressPolicy::default().allowing_loopback()`.
- **WP-22.**
  - `startSession` calls `wake(db, vault, policy, bus, device, now)` when the device is Down. `wake` writes its
    own event; the policy is in `Dependencies.policy`.
  - Register files relays in `Live`'s shutdown closing set (end.rs), so that an in-flight transfer's audit
    entry survives a shutdown.
  - Optional: one `LinkError` to GraphQL-code mapping.
- **WP-32.**
  - `wakeDevice` errors: NOT_FOUND, VALIDATION, CONFLICT (no MAC, no relay, the device is its own relay, the
    relay's SSH is off or it has no key), UNAVAILABLE (the relay cannot be reached, or `wakeonlan` failed),
    INTERNAL.
  - The files client sends `ping` every 5 s, also during a download.
  - During a transfer, only `cancel` and `ping` are allowed.
  - After `refused` on an upload, send `cancel`.
  - `done` ends a cancel.
  - Close codes: 1000, 1001, 1002, 1011 and 4002.
  - `size` and `modifiedAt` can be null.
  - A listing stops at 10 000 entries without a marker.
- **Programme.** Add a `clippy.toml` `disallowed-methods` rule for `Box::leak`, `String::leak` and `Vec::leak`
  (CHK-012).

## Open coulds (issue #1)

- No DNS-rebinding test, and no test for the per-address bound (there is no cheap loopback blackhole).
- If a create is still pending past `CLOSE_WAIT`, the file can stay on the device.
- `russh-sftp` decodes non-UTF-8 names lossily.
- A listing can be about 60 MB.
- `MAX_PENDING` for files tickets is 32 across the whole hub.
- `cancel` is ignored while the file is still opening.
- The `TimeLimit` sentence repeats "10 seconds".
- The host-key-changed wording differs between wake, files and the terminal.
- The terminal could open through the link.
