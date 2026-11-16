# Spike K-B: remote display without `guacd`

Date: 2026-11-14. Plan: [one-click-plan.md](../../one-click-plan.md) Sections 4.2 and 5.
Code: [spikes/display/](../../../spikes/display/README.md) (throwaway, its own Cargo workspace).

## 1. Verdict

1. **The native path works. The stop condition is not met.** A hub can connect to VNC and RDP
   devices itself, in Rust, and no `guacd` is necessary on macOS.
2. **Ship VNC first on the Mac hub.** RDP follows, on IronRDP.
3. **Apple's authentication works.** The probe signed in to Screen Sharing on macOS 27.0.1
   with security type 30, decoded the screen correctly and moved the pointer. See Section 4.2.
4. **The largest work in S2 is not a protocol client.** It is the part that turns a changing
   screen into Guacamole instructions. The simple version in this spike sends 3 to 17 times
   more bytes to the client than the VNC server sent to the hub. For a Mac with three
   displays that was 116 Mbit/s. See Section 7.

## 2. What the spec requires

Sources: spec Sections 4.3 (GW-*), 6.3 (RA-*), K4 Sections 3 to 5, and the fields that the hub
stores (`devices/remote.rs`, `tunnel/arguments.rs`, `tunnel/policy.rs`).

| Feature | RDP | VNC | Source |
| --- | --- | --- | --- |
| Sign-in with a stored user name and password | Yes, with domain | Yes; the user name is optional (macOS needs it) | K4 Section 3 |
| Prompt for a missing credential | Yes | Yes | GW-015 |
| Security mode | `nla` (default), `nla-ext`, `tls`, `any`, `rdp`, `vmconnect` | No setting | Spec 6.3.2 |
| Server certificate pin, checked before a credential is sent | Yes | No | SEC-007, K4 Section 6 |
| "Ignore certificate" | Yes | No | Spec 6.3.2 |
| Warning badge | `ignore-cert`, `security=rdp` | VNC without TLS | RA-020 |
| Resize when the window changes | Yes: `display-update` or `reconnect` | Not required | RA-004 |
| Clipboard, text | Yes | Follows the copy and paste switches | RA-005, RA-033 |
| Copy off and paste off for each device | Yes | Yes | RA-033 |
| Keys, mouse, special key combinations | Yes | Yes | RA-006, K4 Section 4 |
| Audio out | Yes, default on | No | Spec 6.3.2 |
| Microphone | Yes, default off | No | Spec 6.3.2 |
| Shared drive with upload and download switches | Yes, default off | No | RA-021 |
| Gateway, RemoteApp | In the spec table; the hub has no field for them | No | Spec 6.3.2 |
| End reasons (`AUTH_FAILED`, `REMOTE_CLOSED`, certificate mismatch) | Yes | Yes | K4 Section 5 |
| No session content stored | Yes | Yes | RA-008 |

VNC has a short list. Most of the RDP list is RDP only.

## 3. What was built and how it was tested

| Part | Content |
| --- | --- |
| `vnc-probe` | A direct RFB client, about 830 lines with its measurements. No VNC crate. VNC authentication (type 2), Apple authentication (type 30), Raw, CopyRect, Zlib and ZRLE, cursor and desktop-size messages, keys, pointer, clipboard, resize request. |
| `rdp-probe` | An RDP client on IronRDP (`ironrdp` 0.17.0: connector 0.10, session 0.11, cliprdr 0.7, displaycontrol 0.8; `sspi` 0.21 for CredSSP). TLS with rustls and `ring`, as the hub uses. |
| `guac-frames` | The sketch of the re-encoding: a screen, its changed rectangles, and the `size`, `img`, `blob`, `end`, `rect`, `cfill`, `copy` and `sync` instructions. It uses the hub's `guac-protocol` crate to make the wire text. |
| `testhost` | A container with a Linux desktop: TigerVNC 5901 (type 2), xrdp 0.10.1 on 3389 (TLS), FreeRDP 3 shadow server on 3390 (NLA). |

Why a direct VNC client and not a crate: the two candidates are `vnc-rs` 0.6 and `vnc` 0.4.
Apple authentication is the part that matters for the operator's devices, and the protocol is
small. The direct client is under the hub's control. Its dependency tree has 55 crates
(cryptography, zlib and the PNG and JPEG encoders of the sketch) and its binary is 1.2 MB.
The RDP probe has 359 crates and is 10 MB.

Test computer: this Mac (Apple silicon, macOS 27.0.1). The test host ran in Docker on the same
Mac, so each figure is for a loopback network. The test desktop is 1280 x 800 with two
terminals and a turning polyhedron: one terminal prints the time 50 times each second.

No system setting was changed. Screen Sharing was already on. No tool was installed on the Mac.

## 4. VNC

### 4.1 Against TigerVNC on Linux: all worked

| Step | Result |
| --- | --- |
| Sign-in, VNC authentication | Worked. 14 to 20 ms from connect to `ServerInit`. |
| Frames | Worked with ZRLE, Zlib, Raw and CopyRect. A saved screen was read and is correct. |
| Keys | Worked. The probe typed `touch /tmp/typed-vnc`; the file is there. |
| Pointer | Worked. `xdotool getmouselocation` in the host gave the position sent. |
| Clipboard to the device | Worked. `xclip -o` in the host gave the text sent. |
| Clipboard from the device | Worked. `xclip -i` in the host gave a `ServerCutText` of the right length. |
| Resize | Worked (`SetDesktopSize`, 1280 x 800 to 1024 x 768). The spec does not require it. |

One server detail cost time. TigerVNC answers a full update request with the screen layout
only, and that answer uses the request. The pixels come with the next request.

### 4.2 Against macOS Screen Sharing: worked

Target: Screen Sharing on this Mac (macOS 27.0.1), over loopback. The operator typed the
account password at the probe's prompt. The Mac has three displays.

| Step | Result |
| --- | --- |
| Offer | Version `RFB 003.889`. Security types `30, 33, 36, 35`. |
| Sign-in, Apple authentication (type 30) | Worked with the account name and password. About 0.6 s, most of it the Diffie-Hellman arithmetic. |
| Pixel format | The server accepted the probe's 32-bit format. |
| Frames | Worked with ZRLE. The saved screen was read once and is correct, then deleted. |
| Pointer | Worked: the probe moved the pointer and the screen followed. No key was sent. |
| Clipboard, keys | Not tested on the Mac. They worked on Linux with the same messages. |

Details that matter for S2:

- **All displays are one screen.** The server gave one framebuffer of 4104 x 3404 (14
  megapixels) for the three displays. Each update was one rectangle that covers one display's
  changed area. A session needs 56 MB for its copy of this screen.
- **The first full screen took 1.9 s.** Later full requests took 5 to 86 ms for each display.
- **The stream is heavy.** With a busy desktop and the pointer moving: 43.5 updates each
  second and 38 Mbit/s from the server. See Section 6.
- Types 33, 35 and 36 are newer Apple methods. The probe does not have them. Type 2 (a VNC
  password) is offered only when the operator turns it on; it is off on this Mac.
- `guacd` uses type 30 too, through libvncclient.

To repeat the test:

```sh
spikes/display/target/release/vnc-probe session 127.0.0.1:5900 --seconds 10 --wiggle \
  --cursor-in-screen --latency-pointer 20 --guac png
```

### 4.3 What VNC does not give

- No encryption with type 30 or type 2. Only the sign-in is protected. RA-020 already shows a
  badge for "VNC without TLS". This is the same with `guacd` today.
- No audio, no file transfer, no resize on macOS. The spec asks for none of these on VNC.
- The Tight encoding is not built. TigerVNC and macOS do not need it. A server that offers
  only Tight and Raw would use Raw (80 Mbit/s for the test desktop; see Section 6).

## 5. RDP

### 5.1 What worked

| Step | Server | Result |
| --- | --- | --- |
| Sign-in over TLS | xrdp | Worked. 17 to 76 ms to the active session. |
| Sign-in with NLA (CredSSP, NTLM) | FreeRDP shadow | Worked. 30 ms. |
| A wrong password with NLA | FreeRDP shadow | Refused in CredSSP, before the session. Maps to `AUTH_FAILED`. |
| TLS only against a server that requires NLA | FreeRDP shadow | Refused in the negotiation, with a clear error. |
| Certificate fingerprint and pin | xrdp | Worked. The probe reads the SHA-256 after the TLS handshake and before CredSSP. A different pin stops the opening and no credential is sent (SEC-007). |
| Frames | both | Worked. A saved screen was read and is correct. |
| Keys | xrdp | Worked with scancodes. The probe typed `touch /tmp/typed-rdp`; the file is there. |
| Pointer | xrdp | Worked (`xdotool` gave the position sent). |
| Clipboard text to the device | xrdp | Worked (`xclip -o` gave the text). |
| Clipboard text from the device | xrdp | Worked (the text arrived with the right length). |
| Resize, `display-update` | xrdp | Worked after two corrections; see 5.2. |

### 5.2 What failed

1. **IronRDP does not decode xrdp's "Deactivate All".** After a resize, xrdp sends this PDU
   as a 6-byte header. `ironrdp-pdu` 0.9 wants 10 bytes and returns a decode error. The probe
   takes such a PDU as "Deactivate All" and continues: the screen then changes size and the
   session goes on. The hub needs a correction in IronRDP or the same step.
2. **xrdp 0.10.1 stops (SIGSEGV) on a resize request with no physical size.** With a physical
   size and a scale factor in the request, it does not stop. This is a server fault, but the
   hub must always send both values.
3. **Keys need a table.** Guacamole's `key` carries an X11 keysym. RDP wants a scancode of the
   remote keyboard layout. `guacd` has keymap tables for this; the hub must have its own. The
   probe has a table for a few US keys only. Unicode key events are a fallback for text.

### 5.3 Not tested

- Audio out and the microphone. The crates `ironrdp-rdpsnd` and `ironrdp-rdpsnd-native` exist.
  The hub would also have to send Guacamole `audio` streams.
- The shared drive. The crate `ironrdp-rdpdr` exists. The hub would have to do all of
  Guacamole's file transfer (`file`, `get`, `put`, folder lists) itself. This is large.
- `security=rdp` (standard RDP security). IronRDP's configuration has only TLS and CredSSP.
- `nla-ext`, `vmconnect`, Kerberos, a gateway, RemoteApp.
- GNOME Remote Desktop, the RDP server of a current Linux desktop. It requires NLA; the NLA
  path worked against FreeRDP's server. The operator's test:
  `rdp-probe HOST:3389 --nla --guac png --type-text 'touch /tmp/typed-rdp' --resize 1024x768`.
- The graphics pipeline with H.264. `ironrdp-session` 0.11 does not use it (the crate
  `ironrdp-egfx` is not connected to it). The session used bitmap updates.

## 6. Figures

All on loopback, the 1280 x 800 test desktop, release builds, one run of 10 seconds each
unless a row says more. "Update" for VNC is one `FramebufferUpdate` message. "Frame" for RDP
is a 16 ms period in which the screen changed, because RDP has no frame message here.

| Figure | VNC (TigerVNC, ZRLE) | RDP (xrdp, TLS) | How it was taken |
| --- | --- | --- | --- |
| Sign-in | 14 to 20 ms | 17 to 76 ms | Connect to the first server message after sign-in. |
| Frame rate while the desktop moves | 61.9 updates/s | 28.7 frames/s | Count over 10 s. Each server sets its own limit: TigerVNC 60, xrdp about 30. |
| Whole-screen refresh | 15 ms median (15 to 45 ms in other runs) | 7 to 28 ms to first pixels | VNC: full request to decoded pixels, 5 times. RDP: sign-in to the first update. |
| Key to pixels | 11.4 ms median (9.7 to 15.7) | 6.6 ms median (2.7 to 7.3) | 30 times: type `x` into a terminal on a quiet screen, time the next update with pixels. |
| Bytes from the device | 0.33 Mbit/s | 1.84 Mbit/s | Counted at the socket. VNC with Raw only: 80.8 Mbit/s. |
| Bytes to the client as Guacamole, PNG | 5.76 Mbit/s | 6.67 Mbit/s | The `guac-frames` sketch; see Section 7. |
| Bytes to the client as Guacamole, JPEG 80 | 10.4 Mbit/s | 5.02 Mbit/s | Same. |
| Encode time, PNG | 0.36 ms for each update, 2.2% of one core | 1.14 ms for each frame, 3.3% | Timer around the sketch. |
| Encode time, JPEG 80 | 1.08 ms, 6.7% | 3.52 ms, 10.1% | Same. |

NLA against the FreeRDP shadow server gave 12 frames/s and 4.1 Mbit/s. That server sends the
whole screen each time, so the figure describes the server, not IronRDP.

Limits of these figures: one computer, loopback, a small desktop with flat colours.

The same run against Screen Sharing on this Mac (4104 x 3404, three displays, a busy desktop,
the pointer moving, 10 seconds):

| Figure | macOS Screen Sharing (ZRLE) |
| --- | --- |
| Sign-in | About 0.6 s |
| Frame rate | 43.5 updates/s, 426 000 pixels in an average update |
| Full request, one display | 35 ms median (5 to 86 ms); 1.9 s for the first |
| Pointer to pixels | Not valid: the screen was never quiet for 150 ms |
| Bytes from the device | 38.4 Mbit/s |
| Bytes to the client as Guacamole, PNG | 116 Mbit/s |
| Encode time, PNG | 3.67 ms for each update, 16% of one core |

## 7. Re-encoding into Guacamole instructions

### 7.1 The sketch

The client stays as it is: it gets Guacamole instructions on `/tunnel` (K4, GW-011). The hub
then does what `guacd` does today:

```text
VNC rectangle or RDP update
  -> decode into the hub's copy of the screen (RGBA), mark the rectangle as changed
  -> at most once each 16 ms: take the changed rectangles
       one colour          -> rect + cfill
       moved on the screen -> copy            (VNC CopyRect)
       other               -> img, blob ..., end   (PNG or JPEG, base64, 6048 bytes for each blob)
  -> sync,<timestamp>
```

Other things the hub must send or do, which `guacd` did before:

- `size` at the start and after each remote resize; `cursor` for the pointer image.
- `clipboard` streams both ways, with the copy and paste switches applied (RA-033). The relay
  was the first guard and `guacd` the second; the native path has one guard.
- `required` when a credential is missing, and the read of the client's `argv` answer (GW-015).
- `error` with the status that K4 Section 5 maps to an end reason, and `disconnect`.
- Flow control: the client answers each `sync`. The hub must not encode a new frame for a
  client that has not answered the last ones. Without this a slow link fills with old frames.
- Input: `key` and `mouse` go to VNC with no change (same keysym, same button mask). RDP needs
  the keymap of Section 5.2 and a small mapping for buttons and the wheel.
- The client's `size`: RDP sends a resize; VNC ignores it and the client scales.

### 7.2 The cost

Measured with `encode-cost` on the saved 1280 x 800 test screen, one core:

| Changed part | PNG (fast) | JPEG 80 |
| --- | --- | --- |
| Whole screen | 2.1 ms, 40 kB | 7.5 ms, 55 kB |
| A quarter | 0.5 ms, 8 kB | 1.8 ms, 12 kB |
| 256 x 256 | 0.13 ms, 2.4 kB | 0.50 ms, 3.5 kB |
| Rate | about 490 megapixels/s | about 135 megapixels/s |

- **Processor: small for normal use.** A moving 1280 x 800 desktop took 2% to 10% of one core.
- **A real Mac screen costs more.** `encode-cost` on the saved 4104 x 3404 Mac screen: the
  whole screen takes 80 ms as PNG (4.8 MB) and 164 ms as JPEG (1.6 MB); the rates fall to
  about 175 and 85 megapixels/s, because the content is not flat. A change of one whole
  display will not reach 30 frames each second on one core. Text work, where small parts
  change, is far from this limit: the live session used 16% of one core.
- **Bytes: the simple sketch is poor.** On the test desktop the VNC server sent 0.33 Mbit/s
  and the sketch sent 5.76 Mbit/s to the client, 17 times more. On the Mac the server sent
  38 Mbit/s and the sketch 116 Mbit/s, 3 times more. No relayed link carries that. The causes: each rectangle is a new PNG with its
  own header and base64 (plus 33%); more than 16 rectangles become one large rectangle that
  holds parts with no change; a ZRLE stream keeps its zlib history for the whole session and a
  PNG does not. `guacd` has the same structure but compares each update with its copy of the
  screen, joins rectangles by cost and selects the codec for each rectangle. The hub needs
  the same care, because the link to the client can be a relay with a rate limit (plan
  Section 7, last risk).
- **No lossy WebP.** `guacd` uses it when the client lists `image/webp`. There is no Rust
  encoder for lossy WebP without the C library. PNG and JPEG are enough; the client accepts
  both.
- **Memory:** one RGBA copy of the screen for each session: 4 MB at 1280 x 800, 56 MB for
  the Mac with three displays. IronRDP keeps its own copy too, so RDP needs twice that, or the frame writer
  must read IronRDP's image directly.

## 8. Recommendation

**VNC ships first on the Mac hub. RDP is second, on IronRDP.**

| Reason | VNC | RDP |
| --- | --- | --- |
| The operator's devices | Most are Macs. Screen Sharing is VNC. | Linux hosts with xrdp or GNOME only (R-076). |
| Spec list to cover (Section 2) | Short. All of it worked on Linux. | Long. Sign-in, pin, clipboard and resize worked. Audio and the shared drive are not built. |
| Code the hub owns | About 830 lines, no protocol crate. 55 crates in the tree. | IronRDP and `sspi`. 359 crates in the tree. |
| Input mapping | None: Guacamole already uses keysyms. | A keymap table for each layout. |
| Faults found | One server detail (TigerVNC). | One IronRDP decode fault, one xrdp crash. |
| Speed | 60 updates/s, 11 ms key to pixels. | 29 frames/s (server limit), 7 ms key to pixels. |

The condition for this order was the Screen Sharing sign-in. It worked (Section 4.2).

One caution: a Mac is the heaviest VNC target found here. Its stream to the client must be
reduced before a relayed session is usable (Section 9, items 2 and 10).

RDP is not blocked by anything found here. It is more work and has less use on the first day.

## 9. Changes for plan Section 4.2 (S2)

1. Write the order: VNC first, then RDP. Remove "the one that spike K-B shows to work best".
2. Add the frame writer as its own work item, shared by both protocols: the screen copy,
   changed-rectangle logic, codec choice, `sync` flow control. It is the largest part of S2
   and it decides how a relayed session feels. Give it a byte budget as an acceptance
   criterion (for example: not more than 2 times the bytes that the device sent, for a text
   session).
3. List what moves from `guacd` to the hub (Section 7.1): the `required` prompt (GW-015),
   `error` statuses for K4 Section 5, clipboard policy with one guard, cursor, `size`.
4. RDP on the Mac hub starts without the shared drive and without the microphone, and
   perhaps without audio out. The client must show these as not available on such a hub, or
   the device settings must refuse them. This needs a spec delta for Section 6.3.2. A Linux hub
   keeps `guacd` for them, as the plan already says.
5. RDP needs a keymap work item (keysym to scancode for each layout the operator uses).
6. Pin the IronRDP versions. Correct or work around the "Deactivate All" decode fault. Always
   send a physical size in a resize request.
7. Add to the risks: Apple can remove type 30. Types 33, 35 and 36 are not built here, and
   libvncclient (so `guacd`) does not have them either, to my knowledge. Check this before S2
   starts.
8. The worker process: both decoders are safe Rust, so a parser fault is a panic or high use
   of memory or processor, not memory corruption. Limits on rectangle size and inflated size,
   and one task for each session, cover most of this. Let the security review of S2 decide if
   a separate process is still necessary. `sspi` uses pre-release `picky` crates
   (`7.0.0-rc`); the review must look at this.
9. Contracts and spec: K4 Section 3 describes the handshake with `guacd`. For the native
   path it needs a note that the hub makes the instructions itself; Sections 4 and 5 stay as
   they are for the client. GW-001 to GW-010 do not apply to a hub with no gateway. Both need
   a programme ruling (CON-001) and are part of OI-2.

10. Add a work item for large Mac screens: show one display and not all, or scale the screen
    down in the hub before it is encoded, and limit the frame rate to what the client
    answers with `sync`. Without this a Mac with more than one display sends about
    100 Mbit/s to the client. Check if Apple's server can send one display only.

## 10. Open points

| Point | Owner |
| --- | --- |
| Clipboard and keys against Screen Sharing (not sent in this spike, by choice) | S2 |
| A valid pointer-to-pixels figure for a Mac, on a quiet screen | S2 |
| RDP against a real GNOME Remote Desktop host | Operator, before the RDP task of S2 |
| Does a current macOS still accept type 30 from a non-Apple client on each version the operator has? | S2 |
