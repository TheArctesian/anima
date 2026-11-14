# S2 task breakdown: native VNC

Date: 2026-11-14. Status: **waits for the operator's approval. No product code is written yet.**

Branch: `claude/s2-native-vnc`, cut from `origin/main` at `604fec8d`.
Sources: `docs/one-click-plan.md` Section 4.2 and the spike report
`docs/research/one-click/spike-display.md` (both on branch `claude/spike-display-f9104d`;
neither is on `main`). The spike code is reference only. Nothing ships from `spikes/`.

Scope: VNC only (operator, 2026-11-14). RDP on IronRDP follows after VNC is accepted.

## 1. Design

### 1.1 The seam

Today the hub does a Guacamole handshake with `guacd` over TCP (`tunnel/guacd.rs`), then
relays instructions between that socket and the client under its policy (`tunnel/relay.rs`,
`gateway.rs`, `client.rs`). The hub never reads a pixel.

The native path keeps this seam. A new program, `anima-gateway`, speaks the same protocol
that `guacd` speaks to the hub: it answers `select`, sends `args`, reads `connect`, sends
`ready`, then sends drawing instructions and reads `key`, `mouse`, `clipboard` and `sync`.
For the hub it is a gateway like `guacd`.

Effects:

- **The hub's session code does not change.** The handshake, the relay rules, the policy,
  the end reasons, the address rule (R-032: the hub resolves and checks the device and
  sends an IP address) and the credential handling stay as they are and keep their tests.
- **K4 does not change**, Section 3 included. The client does not change. (The spike report,
  Section 9 item 9, said that K4 Section 3 needs a note. With this design it does not.)
- **`guacd` stays the fallback** by configuration: `ANIMA_GUACD` names the gateway. A hub
  that points at `guacd` is as today. See 1.3 for each protocol.
- **Sessions run in a worker process.** The gateway never has the database, the vault or the
  hub's keys. It gets one device's password in `connect`, as `guacd` does today.

### 1.2 Process model

`anima-gateway` listens on a loopback TCP port. For each connection it starts a child
process (itself, in worker mode) and gives it the accepted socket. One session is one
process. A parser fault, a panic or a memory limit ends one session only. The listener
reads nothing from the socket, so it never holds a credential.

### 1.3 Fallback for each protocol and platform

| Hub | VNC | RDP (until the RDP work) |
| --- | --- | --- |
| Linux, Docker (today's mode) | `guacd`, or `anima-gateway` when the operator sets it | `guacd` |
| Mac, in the application | `anima-gateway` | Not available. The gateway refuses `select rdp` with a status that the hub maps to `GATEWAY_ERROR`. |

For a hub that wants native VNC and `guacd` RDP at the same time, the gateway takes an
optional upstream address and passes each protocol that it does not serve to that `guacd`,
byte for byte. This keeps one `ANIMA_GUACD` value and needs no hub change.

### 1.4 Decisions for the operator

| ID | Question | Recommendation |
| --- | --- | --- |
| D-1 | The seam: a gateway program with the `guacd` protocol (1.1), or VNC code inside the hub with a worker that the hub starts? | The gateway program. No change to the hub's session code, to K3, K4 or K7. The other way changes the `tunnel` router signature (K3), adds a setting (K7) and touches `lib.rs`. |
| D-2 | The gateway listens on loopback TCP with no authentication, as `guacd` does (GW-003). On a Mac any local program can connect to it. Such a program gets no credential and no session; it can ask the gateway to connect to a VNC host, which it can also do itself. Accept this for the first version? | Yes. A Unix socket with owner-only access is better, but `ANIMA_GUACD` is `host:port` in K7. Ask for that K7 change with the security review of T6. |
| D-3 | A Mac with several displays sends all of them as one screen of about 14 megapixels (spike 4.2). First version: scale the screen down in the gateway to a largest size, or show one display? | Scale down to at most the client's window size (the `size` instruction gives it). It needs no Apple-specific message and also cuts bytes. One display at a time is a later task. |
| D-4 | Name: crate `crates/anima-gateway`, program `anima-gateway`. "Gateway" is the spec's word for this role (Section 4.3). | Yes. |

## 2. Tasks

Order: the encoder first (the spike found it is the largest part and the bandwidth risk).
Each task is one or more commits, with its tests, its gates and its reviews. "Security" in
the Review column means `reviewer-security` also reviews it.

| # | Task | Content | Done when | Review |
| --- | --- | --- | --- | --- |
| T1 | Screen and changes | New crate. `screen/`: the gateway's copy of the remote screen, and which parts changed. It compares new pixels with the copy, so a rectangle that the device sends again with no change costs nothing. Pure code. | Property tests: after any sequence of writes, replaying the reported changes on an old copy gives the new screen. | reviewer |
| T2 | Frame encoder | `frames/`: changed parts to `size`, `img` + `blob` + `end`, `rect` + `cfill`, `copy`, `cursor`, `sync`. Joins rectangles by cost, not by count. PNG for flat content, JPEG for photographic content, chosen for each rectangle. Blob size and stream indexes as `guacd`. Pure code; time and limits are parameters. | Byte budget, measured in a test with recorded update sequences from the spike's two desktops: a text session sends not more than 2 times the bytes that the device sent (the spike's sketch: 17 times). Every instruction decodes with `guac-protocol`. | reviewer |
| T3 | Frame pacing | `frames/`: flow control from the client's `sync` answers. No new frame while the client is behind; changes collect and the next frame holds only the newest pixels. A frame-rate limit. A pure state machine with the clock as a parameter. | Tests: a client that stops answering gets no more frames and memory stays bounded; a slow client gets fewer, larger frames. | reviewer |
| T4 | Gateway handshake | `handshake/`: the gateway side of K4 Section 3: `select`, `args` (the VNC argument names the hub sends: `hostname`, `port`, `username`, `password`, `disable-copy`, `disable-paste`), the client's `size`, `image`, `timezone`, `connect`, `ready` with a new connection ID. `required` for a missing user name or password and the `argv` answer (GW-015). Unknown names are accepted and ignored, as the hub sends an empty value for each. | The hub's own `guac-protocol` `Handshake` completes against it in a test. The password is zeroed after use and is in no log or error (LOG-007). | reviewer, security |
| T5 | VNC client | `vnc/`: RFB 3.8 on tokio. Security types: Apple (30), VNC password (2), none (1). Raw, CopyRect, Zlib, ZRLE, cursor, desktop size. Keys, pointer, clipboard both ways. Typed errors. Limits: rectangle inside the screen, inflated size not above the rectangle's pixels, largest screen, clipboard length, time limits for each read in the sign-in. | A fake RFB server in the tests covers each sign-in method, a refused password, each encoding and each limit. A fuzz-style property test: no server bytes cause a panic or unbounded memory. | reviewer, security |
| T6 | Session and worker process | `session/`: joins T2 to T5. Input: `key` and `mouse` pass with the same numbers. Clipboard obeys `disable-copy` and `disable-paste` (second guard; the hub's relay is the first). Ends: sign-in refused gives `error` 0x0301 (`AUTH_FAILED`), device closed gives `disconnect`, other faults give `error` 0x0200 with the fixed text. `main.rs`: the listener, one child process for each connection, the optional upstream `guacd` (1.3), `select rdp` refused. Logs at the edges only, no device text. | An integration test starts the real program, runs the hub's handshake against it and a full session against the fake RFB server: frames, input, clipboard, each end cause. A killed worker ends one session and the listener stays up. | reviewer, security |
| T7 | Large screens | Scale the screen down to the client's size before it is encoded (D-3), and map the pointer back. The scale changes when the client sends `size`. | A 4104 x 3404 source and a 1440 x 900 client: the frame bytes and the encode time for a whole-screen change, measured in a test, are within a stated budget; pointer positions map back within one pixel. | reviewer |
| T8 | Hub tunnel against the gateway | No hub source change. One hub integration test (`crates/hub/tests/it`) runs the existing `/tunnel` route against a started `anima-gateway` and the fake RFB server, so the pair is proved together, with copy off and paste off. | The test passes in `bun run gates`. | reviewer, security |
| T9 | Live checks | With the operator: macOS Screen Sharing on this Mac (the operator types the password; keys and clipboard this time, with the operator's agreement), and TigerVNC in the spike's container. Figures: bytes to the client against bytes from the device, frame rate, key to pixels on a quiet screen. | The figures are in a short report. Screen Sharing is usable through the real client at the scaled size. | Operator |
| T10 | Packaging and documents | The hub's Docker image and the gates build the new program. `docs/setup` says how to select it. No VPS change. | A hub container with `ANIMA_GUACD` set to the gateway opens a VNC session. | reviewer |
| T11 | Icons (own commit, can be done at any time) | VNC: a screen. RDP: a screen with a lock in it. See Section 4. | See Section 4. | reviewer-interface |

Not in these tasks: RDP, audio, the shared drive, the application bundle for the Mac hub
(plan items S4 and S6 start the gateway there), and the newer Apple sign-in types 33, 35
and 36.

## 3. Where a ruling is necessary

I stop and report before each of these. None is changed without a ruling.

| Item | Why | When |
| --- | --- | --- |
| Root `Cargo.toml`: a new workspace member `crates/anima-gateway` | Not a contract, but it is shared by every branch. | T1 |
| `infra/` (the hub image or a compose file) | T10 adds the program to the deployment. | T10 |
| Spec text: GW-001 says the gateway is the official `guacd` image; GW-010 to GW-015 name `guacd`. A second gateway needs a spec delta. | Part of OI-2 in the plan. Text only. | Before merge |
| K7 `ANIMA_GUACD` as a Unix socket path (D-2) | Contract change. Only if the security review asks for it. | After T6 review |
| `quality/baseline.json`, `quality/hub-modules.json`, `main.rs`, `schema.rs`, `contracts/` | No change is planned. If a task needs one, I stop. | — |

CMD-030 (command and MCP tool for each new hub function): this work adds no hub API and no
GraphQL field, so no command is due. The plan's Section 7 excludes session opening from MCP.

## 4. Icons (T11)

Rules read: UI-014 (Carbon for interface icons, no third library, no image files), UI-015,
UI-016 (name each icon by its Iconify id; `design/bin/icons.ts` copies the shape into
`$lib/icons/shapes.ts`; no SVG markup in a component), UI-026.

Found on `main`:

- `carbon:screen` is already in the icon table. It fits "a screen" for VNC.
- RDP uses `carbon:remote-connection` in the device settings today. A running-row test
  shows an RDP row with `carbon:screen`. I have not yet listed every place that shows a
  protocol icon; that list is the first step of T11.
- I could not check the pinned Carbon set for a "screen with a lock" icon: this worktree has
  no `node_modules`. I know of no such icon in Carbon. `carbon:locked` and `carbon:screen`
  exist as two icons.

Plan: install the packages (`bun install --frozen-lockfile`), search the pinned Carbon set,
and then report one of:

1. A fitting Carbon icon exists: use it for RDP, `carbon:screen` for VNC, in each place that
   shows a protocol (connect buttons, session tabs, running rows, device settings).
2. No fitting icon: I do not draw one without asking. Choices for the operator: a small
   `carbon:locked` badge over `carbon:screen` made by a component from the two table icons
   (no new SVG, but UI-016 must permit a composed icon), or a different Carbon icon for RDP.

## 5. Gates and reviews

- Each task: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
  `cargo nextest run` for the changed crates, then `bun run quality:limits`,
  `quality:names`, `quality:imports`. Before I report a task as done: the full
  `bun run gates` with `BASE_REF=origin/main`, and its real output.
- Each task is reviewed by `reviewer`; T4, T5, T6 and T8 also by `reviewer-security`; T11 by
  `reviewer-interface`. I report their findings as they are, then correct them.
- No push and no merge.

## 6. Handoff note

Nothing is built yet. If this session stops: the spike's VNC code
(`spikes/display/vnc-probe` on the spike branch) is the reference for T5; `guac-frames`
there is the naive version of T2 and shows what not to do with rectangles. The test host
image `anima-spike-display-host` is on this Mac for T9.
