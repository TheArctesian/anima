---
kind: Task breakdown: native RDP gateway
state: In progress
evidence: Its Section 5: R1 to R7 are built. Its Section 6 lists what is open for the operator.
---

# S2 task breakdown: native RDP

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Date: 2026-11-15. Status: **built and reviewed.** Section 5 holds the state of each task.

Branch: `claude/s2-native-rdp`, cut from `origin/main` at `581e1d13`.
Sources: the spike report `docs/plans/one-click/spike-display.md` Sections 5, 7 and 9
(branch `claude/spike-display-f9104d`; not on `main`) and
[s2-vnc-tasks.md](s2-vnc-tasks.md), whose design this work keeps. The spike's `rdp-probe`
is reference only. Nothing ships from `spikes/`.

Request: the operator, 2026-11-15, in the build session: "Start RDP on IronRDP next".

## 1. Design

### 1.1 What stays

RDP goes into the program that serves VNC, `anima-gateway` (`crates/anima-gateway`). The
seam, the worker process for each session, the screen copy, the frame encoder, the pacing,
the scaled view, the prompt and the hub's side are as they are for VNC. The hub's session
code, the client and the contracts K3, K4 and K7 do not change: the gateway answers
`select rdp` as `guacd` does and reads the `connect` values that K4 Section 3 lists.

### 1.2 What is new

| Part | Content |
| --- | --- |
| The RDP protocol | IronRDP (`ironrdp` 0.17 and its part crates, pinned in `Cargo.lock`), with `sspi` for NLA (CredSSP with NTLM). TLS is rustls with `ring`, as in the hub. |
| The certificate | The gateway reads the server certificate after the TLS handshake and before any credential is sent. It goes on only when the SHA-256 is the one in `cert-fingerprints`, or when `ignore-cert` is `true` (SEC-007). The hub has already made the same read (K4 Section 6); this is the second guard, as `guacd`'s is. |
| Keys | Guacamole sends an X11 keysym. RDP wants a scancode of the device's keyboard. The gateway has a table for the US keyboard, the layout `guacd` uses when `server-layout` is empty, which is what the hub sends. A character that the table cannot make with the modifiers that are down goes as a Unicode key event. |
| Pointer | Guacamole's button mask to RDP button and wheel events. |
| Clipboard | Text both ways on the `cliprdr` channel, under `disable-copy` and `disable-paste` (RA-033). |
| Resize | With `resize-method` `display-update`, a new client view is sent to the device on the display control channel, always with a physical size (the spike's xrdp fault). The device then sends "Deactivate All" and the capabilities are exchanged again. A device that does not resize is scaled to the view, as VNC is. |

### 1.3 Not in the first version

| Function | In the gateway | Why |
| --- | --- | --- |
| Audio out, the microphone | The values are read and not used; the session has no sound. | Not built (spike 5.3). |
| The shared drive, upload and download | The values are read and not used; the session has no drive. | Large: all of Guacamole's file transfer (spike 5.3). |
| `security` `rdp` and `vmconnect` | Refused with status 0x0100. | IronRDP has TLS and CredSSP only. |
| `resize-method` `reconnect` | The picture is scaled to the view. | A reconnect inside the gateway is a later task. |
| Keyboard layouts other than US | Unicode key events cover text; shortcuts follow the US table. | One table first (spike Section 9 item 5). |
| Kerberos, a gateway, RemoteApp | Not offered. | The hub has no field for them. |

A hub that needs sound or the drive keeps `guacd` for RDP: `--other-gateway host:port`
with the new option `--pass rdp` sends RDP sessions there and serves VNC itself.

## 2. Tasks

| # | Task | Content | Done when | Review |
| --- | --- | --- | --- | --- |
| R1 | Handshake for RDP | `handshake/`: `select rdp`, the `args` names of K4 Section 3, and the `connect` values as a typed opening: sign-in with domain, security, the certificate rule, the resize method. | The hub's own `Handshake` completes against it for RDP. A `connect` with no pin and no `ignore-cert`, or with `security` `rdp`, is refused. No value in an error (LOG-007). | reviewer, security |
| R2 | Keys and pointer | `rdp/keys.rs`, `rdp/pointer.rs`: pure tables and state. | Tests: each US key, shifted characters, a character outside the table, shortcuts with Control, the wheel, button changes. | reviewer |
| R3 | RDP opening | `rdp/opening.rs`: X.224, TLS, the certificate check, CredSSP, up to the active session. Time limits. Typed errors that tell a refused sign-in from other faults. | Tests against an RDP server in the test (`ironrdp-server`, test only): TLS sign-in works; a wrong pin stops before a credential is sent; a server that is not RDP ends in time. | reviewer, security |
| R4 | RDP session | `rdp/` and `session/rdp/`: updates into the screen copy, frames out, keys, pointer, clipboard, resize with reactivation, the ends. The shared picture code of the VNC session moves to one place. | A full session against the test server through `serve`: frames, keys and pointer arrive, each end cause, copy off and paste off. | reviewer, security |
| R5 | Program and documents | `--pass`, the setup document, this file. | The started program serves RDP and passes it when told to. | reviewer |
| R6 | Hub tunnel against the gateway | One hub test: the real `/tunnel` route, an RDP device record, the gateway and the test server. No hub source change. | The test passes in `bun run gates`. | reviewer |
| R7 | Check against real servers | The spike's container on this Mac: xrdp (TLS) and the FreeRDP shadow server (NLA), with the container's own test account. Sign-in, a frame, keys, clipboard, resize, a wrong password. | The results are in Section 5. | — |

Not in these tasks: what Section 1.3 lists, a GNOME Remote Desktop host (the operator has
one; R-076: no Windows), the deployment files, and the spec text of Section 6.3.2 for a hub
without sound and drive (a spec delta, for the operator).

## 3. Where a ruling is necessary

| Item | Why |
| --- | --- |
| `contracts/`, `quality/baseline.json`, `quality/hub-modules.json`, `main.rs`, `schema.rs` | No change is planned. If a task needs one, I stop and report. |
| `Cargo.lock` grows by the IronRDP and `sspi` crates | Shared by every branch. The spike counted 359 crates for its probe. The operator's request names IronRDP, which is taken as the ruling. |
| Spec Section 6.3.2 | Sound and the drive are not there on a hub with the native gateway. Text only; listed in Section 6. |

CMD-030: no hub API and no GraphQL field is added, so no command is due.

## 4. Gates and reviews

As for VNC: the crate's gates for each task, `reviewer` for each task, `reviewer-security`
for R1, R3 and R4, and the full `BASE_REF=origin/main bun run gates` before the merge.

## 5. Progress and choices made while building

| Task | State | Commits |
| --- | --- | --- |
| R1 Handshake for RDP | Built. | `43e106aa` |
| R2 Keys and pointer | Built. | `d64a2b62` |
| R3 RDP opening | Built, with tests against an RDP device in the test. `reviewer-security`: 2 medium, 1 low, 3 notes; all corrected but the two items in Section 6. Second review of the corrections: all closed, 1 new medium and 2 smaller points, corrected. | `a896283b`, `3bc6cb53` |
| R4 RDP session | Built, with tests of a whole session through `serve`. `reviewer`: fix and re-review, no fault in the keys or the loop, 24 of 39 mutants not caught; its 6 should-fix points corrected. | `9ab323c8` and the commit after R5 |
| R5 Program and documents | Built: `--pass`, the setup document. | `4554a90f` |
| R6 Hub tunnel against the gateway | Built: 2 hub tests pass with a throwaway PostgreSQL. `reviewer` (by reading): fix and re-review; the password and the drawing are now asserted, and a wait with no meaning is gone. | `dd9d67c8` and the commit after it |
| R7 Check against real servers | Done; results below. | — |

Choices:

- **One program.** RDP is a module of `anima-gateway`, not a second program: the handshake,
  the screen, the frames and the worker are shared.
- **The IronRDP versions.** The meta crate `ironrdp` 0.17 cannot be used: its part crates pin
  a pre-release of `picky` that needs `curve25519-dalek` 5.0.0-rc.1, and the workspace has
  5.0.0. The gateway uses the newest part crates directly (`ironrdp-connector` 0.12,
  `ironrdp-session` 0.13, `ironrdp-tokio` 0.12, `ironrdp-pdu` 0.11, `ironrdp-cliprdr` 0.9,
  `ironrdp-displaycontrol` 0.10, `ironrdp-dvc` 0.10, `ironrdp-svc` 0.10,
  `ironrdp-graphics` 0.11, `ironrdp-core` 0.3), with `sspi` 0.23. `Cargo.lock` grows from 828
  to about 890 packages. These versions decode xrdp's short "Deactivate All", so the
  spike's correction for it is not needed.
- **The test device.** `ironrdp-server` has no release for these part crates. The tests have
  their own RDP device on `ironrdp-acceptor` 0.12 (test only), with a certificate made at
  run time. A builder agent wrote it in its own worktree; its commit is `3bc6cb53`.
- **R1, the names offered.** `args` lists the six names of VNC and then `domain`, `security`,
  `ignore-cert`, `cert-fingerprints` and `resize-method`. The names for sound and the drive
  are not offered, so the hub sends no value for them.
- **R1, the certificate rule.** A session opens only with an accepted fingerprint or with
  `ignore-cert` `true`. With both, the fingerprint is checked. One fingerprint is taken, in
  the form the hub sends.
- **R2, the lock keys.** Caps Lock and Num Lock are not sent to the device. The session sets
  Num Lock on and Caps Lock off once at its start, and a capital with no Shift down goes as
  a Unicode key. AltGr (`ISO_Level3_Shift`) is not sent either: its characters go as Unicode.
- **R3, the certificate check.** TLS takes any certificate chain and checks the handshake's
  signatures. The opening compares the SHA-256 after the handshake, before CredSSP and
  before the Client Info, which are the two places an account is sent.
- **R3, a refused sign-in.** A fault of the CredSSP step, or "access denied" from the device,
  ends the session with 0x0301. Every other fault of the opening is 0x0203.
- **R4, the prompt comes before the device is reached,** so the device does not wait with an
  open connection while the operator types. With security `TLS` nothing is asked for: the
  device shows its own sign-in for what is missing, as with `guacd`.
- **R4, waits.** 8 s to connect, then 16 s for the device's offer and its sign-in.
- **R4, the view.** The device is asked for the client's view when the session opens. A
  later view is asked for once it has kept its size for 300 ms, always with a physical
  size. Until the device has changed its screen the picture is scaled.
- **R4, the first frame.** An RDP session sends `size` at once, before the device draws.
- **R4, a flush after each write.** A TLS connection keeps what it could not send at once.
- **R5, `--pass`.** Before this work `--other-gateway` sent RDP to `guacd`, because the
  gateway did not serve it. Now the gateway serves RDP itself, and `--pass rdp` keeps the
  old way.

- **After the security review.** The first message of an opening is not encrypted, and
  IronRDP put the user name into it as a cookie: it now holds the client's name. The parts
  of one virtual channel message are counted before IronRDP joins them (limit: 2 MiB and
  4096 bytes), and packed channel data, which was never agreed, ends the session. The
  device's first screen size is checked before a picture of that size is made. The settings
  IronRDP keeps for a later capability exchange hold no account. A device that sends the
  session to another host ends it.
- **After the review.** The clipboard rule (`rdp::reply`) and the timing of the resize
  request (`rdp::ViewRequests`, with the time as a parameter) are pure code with their own
  tests. A view whose adjusted size is the screen's size asks nothing. After a capability
  exchange every key and button that is down is released on the device, because it does
  not listen during the exchange and a release could be lost. New tests: each kind of key
  and button through a session, the statuses for a device that is not RDP and one that is
  not there, and `--pass`. The clipboard backend is `ClipboardNotes`.
- **After the second security review.** All five corrections closed; one new medium of the
  same kind, corrected: the parts of one drawing update are counted too (limit 8 MiB,
  what the client says it takes), and a packed update ends the session. A channel part
  with neither a first nor a last mark, outside a message, is a whole message and is not
  counted on. With paste off the device is told nothing when the client's clipboard
  changes. The live checks were repeated after the corrections: xrdp (TLS, keys, resize)
  and the FreeRDP server (NLA, a wrong password) gave the same results.
- **Not corrected from the review.** A read error of the device ends the session as a
  close (`disconnect`), not as an error. Every fault of the CredSSP step counts as a
  refused sign-in. `offer_text`, `ask_for_text` and `give_text` take a fault of IronRDP's
  clipboard channel as "no channel". Keys with no mapping: Pause, the keypad's `=`, the
  old keysym ranges for Greek and Cyrillic; Print goes without its second prefix.

- **R6, the test device is a crate of its own.** The operator asked for R6 (2026-11-15). The
  hub reads the device's certificate itself before the session, so the hub's tests need the
  whole RDP device. It moved from the gateway's tests to `crates/rdp-test-device`, a
  workspace member that only tests depend on, so there is one copy. The two hub tests are
  in `tunnel/tests/wire/native/rdp.rs`: a session that signs in with the stored user name, password and domain, draws what the
  device paints and takes a key, and a device with another certificate, which ends
  `SERVER_CERTIFICATE_MISMATCH` and never gets the account. No hub source outside the
  tests changed.

### R7. Check against real servers (2026-11-15)

The spike's container on this Mac, with a password made for the run and kept in no file of
the repository. A script did the hub's side of the Guacamole protocol against the started
program.

| Check | Server | Result |
| --- | --- | --- |
| Sign-in over TLS | xrdp 0.10.1 | Worked. First picture 0.05 to 0.2 s after `connect`. |
| Frames | xrdp | About 21 frames each second with two moving windows; `copy`, `rect` and `img` all in use. |
| Keys | xrdp | Worked. `touch /tmp/typed-rdp-A_1!` typed with no Shift key made the file, so scancodes and Unicode keys both arrive. |
| Pointer | xrdp | A click gave the terminal the focus for those keys. |
| Clipboard to the device | xrdp | Worked (`xclip -o` gave the text). |
| Clipboard from the device | xrdp | Worked (the client got the text). |
| Resize 1024 x 768 to 1280 x 720 | xrdp | Worked: scaled picture after 0.4 s, the device's new screen after 2.5 s. |
| Sign-in with NLA | FreeRDP shadow | Worked. The device's own size (1280 x 800) came by a new capability exchange. |
| A wrong password with NLA | FreeRDP shadow | `error` 0x0301. |
| `tls` asked of a device that requires NLA | FreeRDP shadow | `error` 0x0203, "does not offer the security". |
| `any` | FreeRDP shadow | Worked with NLA. |
| `nla` asked of a device without it | xrdp | `error` 0x0203, "does not offer the security". |

Not checked: a GNOME Remote Desktop host, and the certificate pin against a real server
(the tests check it against the test device).

## 6. Open for the operator

Results: [need-human-testing.md](../../need-human-testing.md), section "Checks that other documents hold", holds the result of each check below (PLN-014).

| Item | Why it is open |
| --- | --- |
| Spec Section 6.3.2 and the device settings | On a hub with the native gateway an RDP device has no sound, no microphone and no shared drive. The settings still show these switches. |
| A GNOME Remote Desktop host | The operator has one; R-076 excludes Windows. The NLA path worked against FreeRDP's server. |
| The account in memory | IronRDP and `sspi` do not zero the user name and the password when they drop them after the opening. The gateway zeroes its own copies, and IronRDP holds none after the opening. A correction needs a change in IronRDP. |
| The device's keyboard layout | US only. The hub has no setting for a layout (`server-layout` is never sent). |
| The clipboard and the resize request on the wire | The test device has no clipboard channel and no display control channel. The rule and the timing are tested as pure code, and both worked against xrdp (R7). A test on the wire needs `ironrdp-cliprdr`'s server side in the test device. |
| Dynamic channel data, and a memory limit for each worker | The count of channel parts covers a static channel message. Inside the dynamic channel a device can still declare a long message (up to 4 GiB, by reading IronRDP). A memory limit for the worker process covers every such path; it was already open from the VNC work, and macOS does not enforce one. |
| Panic texts | A panic in IronRDP or `sspi` is printed by the worker to the gateway's log. The security review found none that prints a credential. A panic hook with a fixed line would make this certain. |
