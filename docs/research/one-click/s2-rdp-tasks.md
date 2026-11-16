# S2 task breakdown: native RDP

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Date: 2026-11-15. Status: **in progress.** Section 5 holds the state of each task.

Branch: `claude/s2-native-rdp`, cut from `origin/main` at `581e1d13`.
Sources: the spike report `docs/research/one-click/spike-display.md` Sections 5, 7 and 9
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
| R1 Handshake for RDP | Open | — |
| R2 Keys and pointer | Open | — |
| R3 RDP opening | Open | — |
| R4 RDP session | Open | — |
| R5 Program and documents | Open | — |
| R6 Hub tunnel against the gateway | Open | — |
| R7 Check against real servers | Open | — |

Choices:

- **One program.** RDP is a module of `anima-gateway`, not a second program: the handshake,
  the screen, the frames and the worker are shared.

## 6. Open for the operator

| Item | Why it is open |
| --- | --- |
| Spec Section 6.3.2 and the device settings | On a hub with the native gateway an RDP device has no sound, no microphone and no shared drive. The settings still show these switches. |
