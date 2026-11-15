# S1 task breakdown: the link (`anima-link`)

Date: 2026-11-14. Status: **approved by the operator on 2026-11-14 with every recommended
decision. T1 to T13 are built on branch `claude/s1-link` (Section 8). T14 waits for a second Mac.**

Branch: `claude/s1-link`, cut from `origin/main` at `604fec8d`. (The local `main` is 6 commits
behind `origin/main`; `claude/s2-native-vnc` uses the same base.)
Sources: `docs/one-click-plan.md` Section 4.1 and the spike report
`docs/research/one-click/spike-link.md` (both on branch `claude/spike-link-7d94b5`, commit
`d199b40d`; neither is on `main`). The spike code is reference only. Nothing ships from `spikes/`.

Scope: `anima-link` as a program that is built and tested by itself. The hub, the client, the
Tauri shell and the `anima` command do not change in S1.

Not in scope, and S1 does not wait for them:

- **S5** (the hub permits clients by device key: NET-001, NET-002, BLD-024). It needs a spec
  delta and a security ruling first. Until S5, no hub accepts traffic from the link.
- **S6** (first run: the application starts `anima-link`, shows the invite, keeps the device key).
- **S4** (the hub as a background service) and the packaging of `anima-link` in the application
  bundle and the hub image.

## 1. Design

### 1.1 One program, two roles

`anima-link hub` listens on the link with the hub's saved key and sends each accepted stream to
one loopback TCP port. `anima-link device` gives one loopback TCP port that reaches the hub.
The link carries bytes only. It does not read HTTP.

The library (Tailcat) is used in one package only (`tunnel`). All other packages are plain Go
with no network library: rules for devices, invites and expiry are pure code with the clock and
the random source as parameters.

### 1.2 What the link keeps

A state directory with owner-only access. Each file is written to a temporary file and renamed.
A lock file stops a second `anima-link` on the same directory (spike K-C: a parent that is
killed leaves its child running on macOS).

| File | Hub role | Device role |
| --- | --- | --- |
| Identity | The private key, the pre-shared key and **the whole relay region** (spike 3.1). These three give the address. | The device's private key and its expiry time |
| Hub address | — | The hub's address, and the next address when the hub has announced one (1.6) |
| Devices | Each permitted device: public key, name, permitted time, expiry time | — |
| Invites | For each open invite: the public key that the code gives (1.4), expiry time, used or not. Never the code. | — |

In S1 the link owns the list of permitted devices. In S5 the hub must become the owner, or read
this list; that is S5's decision (D-7 gives the seam).

### 1.3 Permitted devices and revocation (LNK-002, LNK-004)

- Admission: the library asks `AllowClient` for each new key. The answer comes from the devices
  file and the open invites. An empty list refuses every key. A refused device gets no answer.
- Each accepted stream is recorded under the device key that the tunnel authenticated
  (`PeerKey`).
- Revocation, in this order: remove the key from the list; close each stream of that key; wait
  until the close has reached the device; call `DisconnectClient`. The library call alone leaves
  the session open (spike 3.3).
- The same steps run when a key expires (1.5).

### 1.4 Invites (LNK-003)

Problem: a new device has no permitted key, and the hub must refuse each key that it does not
know. Design:

- An invite is the hub address and a code of 128 random bits, in one string.
- The code gives a **temporary key**: both sides derive a private key from the code (HKDF). The
  hub keeps only the public key of it. So the hub never admits a key that it does not know, and
  the stored value is not a secret (as MET-012 keeps only a hash).
- A stream from a temporary key reaches only the enrolment port of `anima-link`. It never reaches
  the hub's port.
- The device connects with the temporary key and sends its own new public key and a name. The
  hub link marks the invite used **before** it answers, adds the device, and disconnects the
  temporary key. A second use fails, also when the first use failed later (as MET-012).
- The invite expires after 15 minutes. The clock is a parameter.

### 1.5 Key expiry and renewal (LNK-005)

- A device key expires 180 days after it was permitted. The hub's key does not expire (as VPN-003).
- Renewal: in the last 30 days, the device link makes a new key, connects with the old key to
  the enrolment port and sends the new public key. The hub link adds the new key and keeps the
  old key until the new key connects once or the old key expires. Then it revokes the old key (1.3).
- A device with an expired key needs a new invite.

### 1.6 Keepalive, path report and relay (LNK-006, LNK-007, proposed LNK-009)

- **Keepalive.** Each 15 s the device link pings the hub. If the ping fails, it registers again
  and retries. This one loop answers three spike faults: no reconnect after a hub restart (3.2),
  the idle stall (3.6), and the path report.
- **Path report.** Device: the result of the last ping (`direct`, `relayed`, `none`, latency).
  Hub: the library's status for each device, with the relay name, and with disconnected devices
  removed. The device cannot know the relay name (spike Section 4).
- **Own relay.** A setting for the relay host. At start, the hub link checks that the relay
  accepts it (a ping to itself through the relay), because a relay that refuses gives no error
  (spike Section 5).
- **Relay change.** The address contains the relay, so a new relay is a new address. The hub
  link first announces the next address to each connected device for a set time, then changes.
  A device keeps both and tries both. A device that was off for the whole time needs the domain
  name (below) or a new invite.
- **Domain name.** A DNS TXT record can hold the hub address (optional, D-6). A name only finds
  the hub; LNK-002 still applies.

### 1.7 Control and logs (LNK-008)

- `anima-link` has a control socket: a Unix socket in the state directory with owner-only
  access. Verbs: devices (list, revoke), invites (make, list, cancel), path, relay (show, set),
  address (show). The commands of `anima-link` itself use this socket.
- No secret is in an argument or an environment variable. The invite and the address are shown
  only as the answer of the control verb that makes or shows them. Logs name a key by a short
  fingerprint. A test reads every log line of a full run and fails if a key, an address, a
  pre-shared key or an invite code is in it.
- `anima-link` stops when its parent closes its standard input (for S4 and S6).

### 1.8 Folder

One Go module in `link/` at the repository root. Packages are operator nouns: `tunnel` (the
only package that imports Tailcat), `identity`, `devices`, `invites`, `relay`, `control`, and
the two roles. `cmd/anima-link/main.go` wires and holds no rules. Errors are typed for each
package (the Go form of the `thiserror` rule). The 300 / 60 / 4 / 15 / 4 / 12 limits apply.

## 2. Decisions for the operator

| ID | Question | Recommendation |
| --- | --- | --- |
| D-1 | **Go gates.** The gates are Rust and TypeScript. Which Go gates, and where do they join? | Five gates in `bun run gates`, after `pinning` and before `install`: `gofmt -l` (must print nothing), `go vet ./...`, lint (`staticcheck`, and `revive` for function length 60, arguments 4, complexity 15, nesting 4), `go test -race ./...`, `go build ./...`. The two lint tools are pinned as `tool` lines in `link/go.mod` and run with `go tool`, so nothing is installed system-wide. File lines, files for each folder and banned names: add `.go` and `link/` to the existing `quality:limits` and `quality:names`. This changes `quality/src/` and the gate list in `AGENTS.md`: it needs a ruling (Section 4). Go 1.27 must be on `PATH` on each machine that runs the gates; `go.mod` pins the toolchain. |
| D-2 | **An unreleased Tailcat commit.** The functions that S1 needs are only on Tailcat's `main` (spike Section 8, item 1). | Pin `main` at `e00cdde6ff78` (2026-11-05) by pseudo-version; `go.sum` fixes the content. Do not vendor (the dependencies are tens of MB). Keep Tailcat in the `tunnel` package only. Add "pinning" tests for each library behaviour that S1 relies on (admission, disconnect, peer key, the address round trip, the status fields), so that an update that changes one fails a test, not a user. Keep `link/UPSTREAM.md`: the pin, the behaviours, the upstream issues that S1 follows. Move to the next release (`v0.8.0`) when it has these functions. Two spike faults (no registration after a server restart; the 5 s reconnect) are candidates for upstream issues: I do not open an issue without the operator's word, because it is public. |
| D-3 | **The relay for the first version.** The Headscale relay is set to refuse link keys. A relay of our own is a VPS change. | First version: Tailscale's shared relays, the library's default. No VPS change in S1. S1 builds and tests the own-relay setting against a relay that the tests start on loopback, so the function is ready. Know the limits: the shared relays have a rate limit, Tailscale can log connection metadata (not content), and a relayed remote display is slow (spike Section 6). A `derper` of our own for the hosted mode is a later decision, after the second-Mac results. Do not set `verify_clients: false` on Headscale: it undoes WP-03 decision D-7. |
| D-4 | **What waits for the second-Mac measurements?** | Nothing in T1 to T12 waits; they are proved by tests on one machine. Three things wait: the acceptance numbers of the keepalive (T6 is built with parameters; the values are set after), the answer to the slow first connection (spike 3.5: if two machines show it, T6 gets one more step, the hub link forgets a device that registers again), and T14 (live checks). The default relay (D-3) and the warning for relayed remote display are confirmed or changed by the same results. |
| D-5 | **Invites by a temporary key** (1.4), or an enrolment port that accepts every key? | The temporary key. The hub then never admits a key that it does not know, also when the address is public (D-6). The security review of T8 must confirm the derivation. |
| D-6 | **A domain name for the address** (LNK-007). The address holds the pre-shared key, and DNS is public. Then the list of permitted keys is the only protection. | Build it, off by default, with this warning in the setting and in the documents. An invite is the normal way. |
| D-7 | **The device key for the hub (the seam for S5).** A loopback stream does not say which device it is from. S5 needs that. Build the sending side in S1? | Yes, as T12, off by default: the hub link puts a PROXY protocol v2 header with the device key at the start of each stream. It is tested against a fake hub. The hub side, and the decision to trust it, are S5. If the operator prefers, T12 moves to S5 with no effect on the other tasks. |
| D-8 | **Where the private key is kept.** A file with owner-only access, or the system key store? | A file in S1, for both roles (the hub role also runs on Linux with no key store). For the device role in the application, S6 can move the key to the key store that the Tauri shell already uses. The security review of T3 gives its opinion. |
| D-9 | **Folder name**: `link/` at the root, program `anima-link`, proposed requirement LNK-009 for the keepalive. | Yes. Spec Section 13 needs one line (Section 4). |

## 3. Tasks

Each task is one or more commits with its tests and gates, and cites its requirement IDs.
"Security" means `reviewer-security` also reviews the task.

| # | Task | Content | Done when | Review |
| --- | --- | --- | --- | --- |
| T1 | Go gates | D-1 as ruled: the gates in `quality/src/gates/`, `.go` in limits and names, the `AGENTS.md` text. An empty `link/` module with one test, so that each gate has something to run. | `bun run gates` runs the Go gates and passes. A file of 301 code lines, a banned name and a `gofmt` fault each fail their gate (shown once, then removed). | reviewer |
| T2 | Tunnel and test bed | `tunnel`: the only use of Tailcat, at the pinned commit (D-2). A test bed that starts a relay on loopback inside the test, so that no test needs the internet. The pinning tests. `UPSTREAM.md`. | A hub-role tunnel and a device-role tunnel exchange bytes in a test, direct and with the relay forced. Each pinning test passes. No test opens a connection outside loopback. | reviewer |
| T3 | Identity and stable address (LNK-001, LNK-008) | `identity`: the state directory, atomic writes, owner-only modes, the lock file, the saved relay region. | After a stop and a start the address is the same, with an own relay and with a region of a recorded relay map. A second process on the same directory is refused. A file with wrong permissions is refused. No key in any error text. | reviewer, security |
| T4 | Control socket and process rules (LNK-008) | `control`: the Unix socket, owner-only, the verb frame. Stop when the parent closes standard input. The log test of 1.7 starts here and each later task extends it. | A second user's connection is refused (socket mode checked). The process stops in 2 s when its standard input closes. The log test passes. | reviewer, security |
| T5 | Permitted devices and the hub role (LNK-002) | `devices`: the list and its rules (pure) and its file. The hub role: admission from the list, each stream sent to the loopback port and recorded by device key. | A key not in the list gets no answer and is not in the peer list. A key added while the hub link runs connects with no restart. HTTP and WebSocket pass through against a test server, with correct byte counts in both directions. | reviewer, security |
| T6 | Device role and keepalive (proposed LNK-009) | The local port, the dial, the 15 s keepalive that registers again. The clock is a parameter. | After a hub-link restart, a device link that stayed running serves a request again with no user action, inside the keepalive time plus a stated margin. After 5 minutes idle (test clock where possible, one real-time test) the next request has no stall. The first request after start is recorded in the test output. | reviewer |
| T7 | Revocation (LNK-004) | The order of 1.3. | A test holds an open WebSocket and revokes the device: the device's session ends in under 1 s, the hub-side stream to the test server is closed, a new request fails, a second device is not affected, and the peer list no longer shows the key. | reviewer, security |
| T8 | Invites (LNK-003) | `invites`: the code, the derivation, the rules (pure), the file, the enrolment port (D-5). | 128 random bits from the given source. Expiry at 15 minutes by the test clock. A second use fails, also when the first use was cut after the hub marked it. A temporary key cannot open a stream to the hub's port. The file holds no code. A wrong code gets no answer. | reviewer, security |
| T9 | Key expiry and renewal (LNK-005) | Expiry in `devices`; the renewal exchange on the enrolment port; the device role renews by itself. | By the test clock: a key connects at day 179 and not at day 181; an open session ends at expiry; a device renews in the last 30 days with no user action and the old key is then refused; a cut renewal leaves one working key. | reviewer, security |
| T10 | Path report (LNK-006) | Both roles, as 1.6. | With the relay forced: both roles say `relayed`, and the hub gives the relay name. Not forced: `direct`. A revoked device is not in the hub's list. | reviewer |
| T11 | Own relay and relay change (LNK-007) | `relay`: the setting, the start check, the announce-then-change steps of 1.6. | A relay that refuses the hub's key gives a clear error at start (not silence). After a relay change with a device connected, the device reaches the hub on the new relay with no new invite. A device that was off gets the documented failure. | reviewer, security |
| T12 | Domain name (LNK-007), and the device key for the hub (D-7) | Two small parts, each only if its decision says yes: the TXT lookup (the resolver is a parameter); the PROXY v2 header. | The device finds the hub by a name from a fake resolver, and a key that is not permitted is still refused. A fake hub reads the header and gets the same key that the tunnel authenticated; with the option off, no byte is added. | reviewer, security |
| T13 | Build and documents | Release builds for macOS and Linux (arm64, x86_64) with Tailcat's build tags. `docs/setup`: how to run a hub link and a device link by hand, the relay limits of D-3, the warning of D-6. | Each binary is under a stated size (the spike: 15 to 17 MB). A person can follow the document from nothing to a request through the link. | reviewer |
| T14 | Live checks | With the operator and a second Mac: the steps of spike report Section 7 with the real program. Sets the keepalive numbers (D-4). | A short report: the direct rate for each network pair, the first-connection times, relayed Screen Sharing usable or not, the firewall prompt. | Operator |

Order: T1, T2, T3, T4, then T5 to T7, then T8 to T12 in any order, then T13. T14 when the second
Mac is available; it does not block T1 to T13.

Not in these tasks: any hub, client, Tauri or `anima` change; the invite screen; UDP through the
link; more than one hub for one device process (S13 starts one device link for each hub).

## 4. Where a ruling is necessary

I stop and report before each of these. None is changed without a ruling.

| Item | Why | When |
| --- | --- | --- |
| `quality/src/gates/gates.ts`, `quality/src/limits.ts`, the names gate, `quality/README.md` | The Go gates and the Go limits (D-1) change `quality/` files. | T1 |
| `AGENTS.md` (gate list, a Go line under "Idiomatic") | Shared by every branch. | T1 |
| A new top-level folder `link/`, and `.gitignore` | Spec Section 13 lists the root folders. Text only. | T1 |
| Spec text: LNK-001 to LNK-008 with the changes of spike Section 8, and the new LNK-009 | Part of OI-2 in the plan. S1 builds to the texts in Section 5 below until the delta is ruled. | Before merge |
| An upstream issue at Tailcat | Public. | D-2 |
| `contracts/`, `quality/baseline.json`, `quality/hub-modules.json`, `main.rs`, `schema.rs`, `Cargo.toml` | **No change is planned.** S1 adds no hub API, no setting and no Rust code. If a task needs one, I stop. | — |
| The control socket as a contract | When S4, S5 or S6 starts to use it, its verbs must be frozen as a contract. Not in S1. | S4 to S6 |
| Any VPS, Headscale or cloud change | None in S1 (D-3). | — |

CMD-030: S1 adds no hub function, so no `anima` command and no MCP tool are due. When S5 gives
the hub its own device list, the commands come with it. By MCP-004, making an invite (it
returns a secret) and permitting or revoking a device (a trust decision) cannot be MCP tools;
the device list and the path report can.

## 5. Requirement texts that S1 builds to

These are the plan's texts with the spike's changes. They are proposals until the spec delta.

- **LNK-001.** As the plan. Added: the hub must save its key, its pre-shared key and its relay region.
- **LNK-002.** As the plan. Added: a refused device gets no answer.
- **LNK-003.** As the plan. Added: the hub must keep no invite code.
- **LNK-004.** As the plan. Added: the open session of the device must end in 1 second.
- **LNK-005.** As the plan. Added: renewal is in the last 30 days and needs no user action.
- **LNK-006.** As the plan. Added: the hub also shows the relay name.
- **LNK-007.** As the plan. Added: the hub link must fail at start if the relay refuses it; a
  relay change must not need a new invite for a connected device; a domain name is off by default.
- **LNK-008.** As the plan.
- **LNK-009 (new).** The device link must keep the link in use and must connect again by itself
  after a hub restart. The times are set after T14.

## 6. Gates and reviews

- Each task: the Go gates for `link/`, then `bun run quality:limits` and `quality:names`. Before
  I report a task as done: the full `bun run gates` with `BASE_REF=origin/main`, and its real output.
- Each task is reviewed by `reviewer`; T3, T4, T5, T7, T8, T9, T11 and T12 also by
  `reviewer-security`. I report their findings as they are, then correct them.
- No key, address or invite in a log, a test file or a commit. Tests make their keys at run time.
- No push and no merge.

## 7. Handoff note

Nothing is built yet. If this session stops: the spike program (`spikes/link/` on
`claude/spike-link-7d94b5`) is the reference for T2 (`serve.go`, `connect.go`: how the library
is called), for T7 (`revoke` in `serve.go`: the order that works) and for T6 (`dialHub` in
`connect.go`: registering again; the keepalive replaces it). `try.sh` and `slowrelay.go` there
give the relayed test set-up for T14. The spike's `state.go` shows the saved relay region.
The macOS firewall list on this Mac has an entry for the spike's `anima-link`; T14 must use a
second Mac to see the prompt for a new program.

## 8. What was built (2026-11-14)

| # | Task | State | Differences from the plan above |
| --- | --- | --- | --- |
| T1 | Go gates | Built | Six gates, not five: `staticcheck` and `revive` are one gate each, so that each gate is one process. The build gate writes to `/dev/null`. |
| T2 | Tunnel and test bed | Built | Secrets are a type of their own (`redacted`): no way of printing, logging or encoding shows a key, a pre-shared key, an address or an invite. |
| T3 | Identity and stable address | Built | The state directory also refuses a symbolic link and a folder above that another user can change, and removes macOS access-control entries. |
| T4 | Control socket and process rules | Built | No check of the caller's user by the socket itself: the directory (mode 700) is the protection. |
| T5 | Permitted devices and the hub role | Built | Added: 128 streams for each device and 1024 in total. |
| T6 | Device role and keepalive | Built | Measured on one machine: after a hub restart the device link works again in about 14 s (1 s keepalive in the test). The idle test is 5 minutes of real time and runs only with `ANIMA_LINK_SLOW_TESTS=1`; it was run once and passed (7 ms). |
| T7 | Revocation | Built | `hublink.Revoke` holds the order. A sweep each 30 s finishes a revocation that was cut. |
| T8 | Invites | Built | The operator gives the device's name when the invite is made. The invite's temporary key and the hub's own key can never become a device key. `join` prints the key fingerprint and `devices.list` shows it, so that the operator can see if another party used the invite. |
| T9 | Key expiry and renewal | Built | Renewal has two steps and one list entry for each device. It completes only on a stream from the new key. **Known limit:** the sessions that are open when a renewal completes end once (about each 150 days for each device). |
| T10 | Path report | Built | — |
| T11 | Own relay and relay change | Built | Two operator steps: `relay.announce`, then `relay.switch` (the hub link stops and starts again). A device follows after about 45 s. |
| T12 | Domain name; device key for the hub | Built | Both off by default. |
| T13 | Build and documents | Built | `link/build.sh` (four programs, 15.1 to 16.7 MB; the limit is 20 MB). `docs/setup/link.md`. |
| T14 | Live checks | **Not done** | Needs the operator and a second Mac. |

Open items for the operator:

1. **T14**: the second-Mac steps (spike report, Section 7). They set the keepalive numbers of
   LNK-009 and show if the slow first connection occurs between two machines.
2. **Builders (BLD-042)** run `bun run gates`. Each builder computer now needs Go 1.27 on `PATH`.
3. **Gate time.** The Go tests add about 100 seconds to `bun run gates`: they start real
   tunnels on loopback, and some wait for time limits of the library. The full gates passed,
   20 of 20, on commit `b316570a` (2026-11-14).
4. **A session ends at a key renewal** (T9). To keep sessions, the hub link must let the old key's
   streams end by themselves. Not built.
5. **Spec delta** (OI-2): the LNK texts of Section 5 and LNK-009, before a merge to `main`.
6. **Upstream issues** at Tailcat for the faults in `link/UPSTREAM.md`: not opened. The draft
   texts are in `docs/research/one-click/tailcat-upstream-issues.md`.
7. **For S5** (from the security review of T12): the device key header is safe only if the hub
   reads it on a port that only the link's process can reach (a Unix socket with owner-only
   access, or a check of the caller), requires it there, and reads it nowhere else. A loopback
   TCP port is open to each local user of the hub computer.

Security findings that changed the design after the breakdown was approved:

| Finding | Change |
| --- | --- |
| A device could give the invite's own temporary key as its key. A used invite would then work with no end. | The temporary key of an open invite, the invite's own key and the hub's key can not become a device key. |
| The tunnel asks its admission question before the sender has proved its key. A renewal that completed there could be forced by the relay. | A renewal completes only on a stream from the new key. Nothing changes on an admission question. |
| A device that looked for the hub by its domain name accepted an address with another hub's keys. | A device moves only to an address with its hub's key and pre-shared key, and only after a stream of the tunnel. |
| A key, an address or an invite could show in structured logs and in JSON. | The type `redacted.Secret`; a test prints each secret in each way. |
| macOS access-control entries can give access to the state files that the mode bits do not show. | The state directory removes them, after it has checked that the directory is the user's own. |
