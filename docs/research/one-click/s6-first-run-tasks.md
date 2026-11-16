# S6 task breakdown: the first run

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version 1.0. Date: 2026-11-15. Status: **Built; waits for the operator.** The S1 to S8 planning session started this
item. Each design choice below is the recommendation of the build session; the choices that need
an operator ruling are in Section 5.

Branch: `claude/s6-first-run-e456e7`, cut from `origin/main` at `a8274e6b`.
Sources: `docs/one-click-plan.md` Sections 1.2, 4.6 and 8 (on branch `claude/one-click-plan`), and the
hand-over sections of `s1-link-tasks.md` (8), `s4-local-hub-tasks.md` (7.4), `s5-access-tasks.md` (7)
and `s8-stay-awake-tasks.md`. Spec delta: [s6-spec-delta.md](s6-spec-delta.md).

Scope: one application build shows three choices at the first start. "Create a hub on this computer"
installs and starts the local hub with its database, gateway and link, makes the operator account
with a password and TOTP, and shows an invite. "Join a hub" takes an invite, joins this device to
the hub over the link and connects the client. "Sign in" (hosted) is shown as not yet available.
The VPS mode (Docker, Headscale) does not change (acceptance criterion 6).

Not in scope: the release bundle that carries the programs (S12, S4 T9), the backup (S9), the
relay and domain settings (S11), many hubs (S13), the metrics agent over the link (K8 ruling).

## 1. What exists

| Part | State on `main` at `a8274e6b` |
| --- | --- |
| Link (S1, S5) | `anima-link hub`, `device`, `join`, `ctl`. Invites, device keys, revocation, renewal. The hub's link mode (`ANIMA_LINK_SOCKET`) names each device by a link address. No hub code talks to the link's control socket. |
| Local hub (S4) | `anima-local-hub run/install/remove/status`, a LaunchAgent or systemd user unit; members database, migration, gateway, hub. A marked place for the link. Four shell commands; no client calls them. The hub gets `ANIMA_LISTEN=127.0.0.1:8443` and no access rule, so it refuses every client. |
| Account | `anima-hub operator set` (password on standard input) and `operator totp` (needs a terminal, saves at once). TOTP is optional at sign-in. No setup state and no recovery path in the API. |
| Client | No first-run state: with no saved address the desktop uses `https://hub.anima.ing`. Hub addresses must be `https:`; the Tauri CSP permits only `https:` and `wss:`. No QR code. |
| Stay awake (S8) | `hubIsThisComputer(origin)` is true for a loopback hub address. |

## 2. Design

### 2.1 How the client reaches a hub in the application mode

| Computer | Hub address in the client | Reaches |
| --- | --- | --- |
| The hub computer | `http://127.0.0.1:8443` | The hub's loopback port (NET-003 permits plain HTTP on loopback). |
| A joined device | `http://127.0.0.1:<port>` | The local port of `anima-link device`, which the application starts. |

- **F-1.** The client and the CSP accept `http:` and `ws:` for loopback hosts only, the same rule
  as `anima` (`crates/anima-cli/src/hub/origin.rs`). Every other address stays `https:`.
- **F-2.** The device link uses one local port for each hub, chosen at the join and saved in the
  device's link folder, because the client keys its tokens by origin. A port that is taken at a
  later start is an error that the client shows; it is not changed in silence.
- **F-3.** "This computer is the hub" is true only when the local hub is installed and the
  client's hub address is the local hub's address. A loopback address alone is not enough: a
  joined device also uses a loopback address (correction to the S8 seam, AWK-005).

### 2.2 Create a hub on this computer

1. The application runs `local_hub_install` (S4) and polls `local_hub_status` until `ready`,
   `blocked` or `failed`.
2. The local hub's group now has five members: database, migration, gateway, hub, link (2.4).
3. Account (2.3), then the client signs in with password and a new TOTP code.
4. The client calls `createLinkInvite` and shows the invite as a QR code and as text, with its
   expiry (15 minutes, LNK-003).
5. When Stay awake is off, the last step and Settings tell the operator that checks stop
   during sleep (AWK-005).

### 2.3 The operator account is made on the hub computer only

- **F-4.** No hub API makes the account. The application runs the hub's own commands on the hub
  computer, through `anima-local-hub account`, with each secret on standard input. Reason: an
  API that makes the first account is open to the first caller (`first-run-setup.md`, "Why not a
  sign-up page"). On the hub computer the user that runs the hub can already read the database
  and `vault.key`, so a local command gives no new power.
- New hub commands (security tag):
  - `anima-hub operator show`: one JSON line, `{"name":…,"totp":true|false}` or `null`.
  - `anima-hub operator totp <name> --new`: makes a secret and prints one JSON line with the
    secret (base32) and the `otpauth://` URI. It saves nothing. No terminal is needed. The
    operator need not exist: the name is only the label in the URI.
  - `anima-hub operator create <name>`: reads the password, the secret and one code from
    standard input (three lines). Only when the password passes the rules of `operator set` and
    the code is correct now, one transaction makes or replaces the operator with the password
    and the sealed secret, records the code's step so that the code cannot be used again, and
    removes the operator's tokens and login attempts. So the account never exists without its
    second factor (security review M3). The first run and the recovery (2.8) use this command.
  - In the link mode, `login` and `signInBrowser` refuse an operator with no TOTP secret, with
    the answer of a wrong password (SEC-002). In the link mode a request must also name a
    loopback host (`127.0.0.1`, `[::1]` or `localhost`, any port) and carry no `Origin` or a
    client origin, else 403 (security review S1: DNS rebinding).
- **F-5.** The first run requires TOTP (OD-2). There is no "skip". The client signs in after the
  creation with the next code. In the link mode the hub refuses sign-in for an operator with no
  TOTP secret (M3), so `operator set` and `operator totp --disable` cannot open a hub on the link.
- `anima-local-hub account status|totp-new <name>|create <name>` runs the installed `anima-hub`
  with the database URL file and the key file of the hub root. The secrets pass through standard
  input and standard output only (LOG-007).
- **F-9 (M1).** Script in the main web view must not be able to take the account. The shell
  creates at once only while `account status` is `null` (the first run). When an account
  exists, the same `local_hub_account_create` command first shows a native confirmation dialog
  from Rust, which script cannot answer, and replaces the account only on "Replace". Each
  replacement needs its own confirmation; the shell keeps no permission for later (T6 review SF1).
  After a cancel the command answers `busy` for 60 s and shows no dialog, so script cannot repeat
  the dialog until the person clicks through it. A user-presence check (macOS
  LocalAuthentication) is a later addition (Section 5, OR-S6-6).
- **F-10 (M2).** Before the client uses a loopback hub address, the shell checks that a process of
  the same user listens on that loopback port (macOS: `lsof` as the user, which sees only the
  user's own processes; Linux: the owner uid in `/proc/net/tcp`). For a joined device the port
  must also be the one that the shell's own `anima-link device` child reported. Otherwise the
  client sends nothing (no token, no password) and shows the error. Later: a Unix socket or a
  pinned certificate (S4 Section 7.4 item 2).
  The client keeps an "own" answer for 5 s for HTTP requests. The socket asks the shell again before each
  connection, because it connects again by itself after a hub stops. The client forgets the answer when
  the connection stops being live, and before and after Start and Stop (security re-review MF-1).
  Remaining risk: another program can take the port in the short time between the shell's check and the
  send. Only a Unix socket or a pinned certificate removes this risk. A second, smaller window
  stays for HTTP requests: if the hub stops again less than 5 s after an "own" answer while the
  socket waits for its next try, a request in those seconds uses the kept answer. That needs the
  hub to stop twice in seconds and a local program to take the port in between (security
  confirmation of MF-1, Low; the same later fix removes it).
- **F-11 (S1).** In the link mode the hub refuses a request whose `Host` is not a loopback literal,
  and a request whose `Origin` is present and not a client origin of the application. A page in a
  browser (DNS rebinding) then cannot drive sign-in failures into the throttle of `127.0.0.1`.

### 2.4 The link is a member of the local hub

- `anima-link hub -state <hub root>/link -target <socket folder>/link.sock` starts after the hub
  and stops first. The hub gets `ANIMA_LINK_SOCKET=<socket folder>/link.sock` (link mode, S5) and
  `ANIMA_LINK_CONTROL=<hub root>/link/control.sock` (2.5).
- **F-6.** The link needs the relay at its start; with no network it ends. The hub must still serve
  this computer. So the link is the one member that the group starts again **alone**, after
  10 s, then 20 s, up to 60 s, and the group stays `ready`. The state has a `link` field:
  `starting`, `up` or `down` with the time of the last try.
- `PROGRAMS` gets `anima-link`. The release bundle must carry it (S12).

### 2.5 Invites and devices in the hub API (CMD-030)

- New hub domain `fleet/link/`. It talks to the link's control socket (`ANIMA_LINK_CONTROL`) with
  the verbs `invites.make`, `devices.list`, `devices.revoke`, `status`. When the variable is not
  set (the VPS mode) the fields answer `UNAVAILABLE` with a fixed message (K7 permits no new error code).
- K1 fields (planned in `s5-access-tasks.md` Section 7.6):
  - `Query.linkDevices: [LinkDevice!]!` (name, fingerprint, address, path, relay, expiry, streams).
  - `Mutation.createLinkInvite(name: String!): LinkInvite!` (invite text, expiry). Returns a secret.
  - `Mutation.revokeLinkDevice(key: String!): Int!` (closed streams). A trust decision.
  - Both mutations need a login token, not an API key, and write an audit entry. The invite text
    is never logged or audited.
- `anima link devices`, `anima link invite <name>`, `anima link revoke <key>`.
- MCP: one tool, `anima_link_devices` (read). No tool for the invite (MCP-004: it returns a secret) or for
  revocation (MCP-005: a trust decision).

### 2.6 Join a hub

- The operator pastes the invite. The Tauri shell checks its form (`anima-invite.1.` and the
  invite alphabet, at most 4 KiB), and runs `anima-link join -state <link folder>.next` with the
  invite on standard input.
- **F-7.** Each join makes a new key in a new folder. On success the new folder replaces the old
  one. So a device that the hub revoked (`key_retired`) joins again with a new invite and no
  other step (S5 Section 7.8).
- The shell then runs `anima-link device -state <link folder> -listen 127.0.0.1:<port>` while the
  application runs, reads the `local=` address from the ready line and starts it again after an
  end (10 s). The client saves `http://127.0.0.1:<port>` as the hub address and shows the sign-in.
- **S3 of the review.** A successful join removes the saved token of that origin, so a token of
  an earlier hub is never sent to a new one.
- The join screen shows this device's new key. The operator finds the same key in the hub's
  "Devices on the link" list, so that a join with a false invite (another hub) shows as a key that
  the operator's hub does not list. The link reports no hub identity at the join, so the screen
  cannot show one (T8).
- If the join succeeds but its finish fails (the key store or the port), the screen keeps the
  joined state and offers "Retry", which repeats only the finish. The invite is used once, so the
  screen never asks for it again.
- A second use of the invite gets no answer from the hub (LNK-003); the client says that the
  invite is used, expired or wrong, or that the hub is off.

### 2.7 State and control in the client

- Settings, section "This computer's hub" (hub computer only): the local hub's state (starting,
  ready, blocked with the reason, failed with the program), the link's state, "Stop" and "Start".
  `anima-local-hub stop` stops the unit until "Start" or the next sign-in to the computer;
  `start` starts it.
- Settings, section "Devices on the link" (signed in to a link-mode hub): the device list, "Invite
  a device", and "Revoke" with a confirmation.
- A joined device shows the path of its link (direct or relayed, LNK-006).

### 2.8 Recovery

| Loss | What the operator does | Built in S6 |
| --- | --- | --- |
| The TOTP device | On the hub computer: "Recover the account" sets a new password and a new TOTP secret through `anima-local-hub account`. Every token is removed. An audit entry records it. | Yes |
| The password | The same step. | Yes |
| `vault.key` (the hub key that seals credentials and TOTP secrets), database present | The hub is `blocked` with `key-missing` (S4). The client says to put the key back and shows where. | The message and "Show the folder". The other options need a ruling (OR-S6-1). |
| The link key (`<hub root>/link`) | The link makes a new key and a new hub address at its next start. Each device joins again with a new invite (F-7). What the hub bound to the old link addresses stays unused (E-10). | Yes, by F-7. |
| The hub computer | Restore from a backup (S9). | No |

- **F-8.** Recovery is local only, behind the native confirmation of F-9. A joined device, an API key or a login token cannot reset the
  password or TOTP. Reason: the person at the hub computer's user account can already read the
  database and the key; a remote reset would be a new path to take the hub.
- The creation flow offers "Save a copy of the hub key": a native save dialog of the shell writes
  `vault.key` to a file that the operator chooses (mode 0600). The web view never sees the key.
- After recovery the operator removes a device that an attacker joined with a stolen token in
  Settings, "Devices on the link" ("Revoke"). The recovery screen does not repeat the list (T8);
  the guide gives this step.
- The guide says: a Time Machine backup holds the database, `vault.key` and the link key together
  (FileVault and an encrypted backup disk protect them); a stolen hub computer is protected only
  by FileVault; an operator who loses the TOTP device away from the hub computer must wait until
  they are at it (OR-S6-2).

### 2.9 Acceptance criterion 4 (no open inbound port on a public interface)

The hub, the gateway and PostgreSQL listen on loopback or on Unix sockets. `anima-link` holds two
UDP sockets for WireGuard on every interface (IPv4 and IPv6, seen in T11); it answers only the keys in its list (LNK-002) and
serves no TCP port. So the criterion holds as "no TCP listener off loopback, and UDP sockets that
answer only permitted keys" (security review S5). The library's port mapper (UPnP, NAT-PMP, PCP)
can ask the router to open a port; the run-through checks whether the Tailcat build turns it off,
and records `lsof -i` for each program.

## 3. Tasks

| Task | What | Proof | Review | State | Commits |
| --- | --- | --- | --- | --- | --- |
| T1 | This document, the spec delta, the register row | — | — | Done | the commit of this row |
| T2 | Hub: `operator show`, `operator totp --new`, `operator create`; TOTP required and loopback names in the link mode | Tests: a wrong code saves nothing; a used code is refused at sign-in; tokens removed; no secret in logs | reviewer, reviewer-security | Done. Review fixes: audit entries for `create` and `totp --disable`, no terminal for `create`, loopback `Host` for the health probe, atomic `create`. `operator set` writes no audit entry (unchanged; existing tests need an empty trail) | c4c383b4, dceb43af, 379869c5, 5170ee2a |
| T3 | Hub: `fleet/link` domain, `ANIMA_LINK_CONTROL`, K1 fields, audit | Tests against a fake control socket; `UNAVAILABLE` without the variable; API key refused for the mutations | reviewer, reviewer-security | Done. With no control socket the fields answer `UNAVAILABLE` with a fixed message: K7 allows no new code. A change whose audit write fails is stated as `INTERNAL`, and an unaudited invite is cancelled | 4ad2d659, 4ab256a0 |
| T4 | `anima link devices|invite|revoke`; MCP `link_devices`; CMD-031 | CLI tests against the fake hub | reviewer | Done. The tool is `anima_link_devices` (MCP-011) | 88b6dea3 |
| T5 | Local hub: the link member (F-6), hub link-mode environment, `account`, `start`, `stop` | Unit tests with stand-in programs; the existing real-program test | reviewer, reviewer-security | Done. `account` has `status`, `totp-new`, `create` (M3) | a82f38e6, a1b528bb |
| T6 | Tauri shell: account, start/stop, carried `anima-link`, device link (join, run, status); CSP | Rust unit tests; caller check on each command | reviewer, reviewer-security | Done. No separate recovery command: one dialog for each replacement, `busy` for 60 s after a cancel; a failed link swap restores the old link; `127.0.0.1` only (no `localhost`, no `[::1]`: the CSP cannot name an IPv6 literal) | 210a0fa4, 33f8c269 |
| T7 | Client: loopback address rule (F-1), QR code, AWK-005 seam (F-3) | Vitest | reviewer | Done | 3ffae7d5, ed6f7490 |
| T8 | Client: first-run screens (choices, create, account, TOTP, invite, join, recover) and the two Settings sections | Vitest; `reviewer-interface` | reviewer, reviewer-interface, reviewer-security | Done after two fix rounds: F-10 for a typed address, a pending check and each socket connection (MF-1); no secret in a live region; steps hold their height; join keeps the invite and offers Retry. The QR size comes from a space token, because K5 has no QR size and is frozen | 3f536a5c, a9a688b3 to ff67a2bf, 6a256238, 604295d3, bb9d80d7 |
| T9 | Guide `docs/setup/first-run.md`; this document's results | — | — | Done (Section 6 is completed by T10 and T11) | f598e176 and the commit of this row |
| T10 | `origin/main` merged, full gates on that commit | Gate record | — | Done (Section 6.1) | 3abe87d0 |
| T11 | Run-through on this Mac from a clean state (create a hub, account, invite, a second device link on this Mac, second use of the invite, revocation, `lsof`); LaunchAgent and folders removed after | Section 6 | — | Done (Section 6.2) | 3abe87d0 |

## 4. Gates and reviews

`BASE_REF=origin/main bun run gates` with `DATABASE_URL` unset and a private Rust build folder.
Under load, re-run alone `ra_031_without_a_wake…`, `sources::controls::src_004…` and the
`provider-github` fixture tests before a failure counts.

## 5. Where a ruling is necessary

| ID | Question | Options | Recommendation |
| --- | --- | --- | --- |
| OR-S6-1 | `vault.key` is lost and the database is present, with no backup | (a) Only "put the key back"; else start a new hub. (b) A "reset sealed data" step: a new key, every sealed value (device credentials, TOTP secrets, other sealed columns) removed, devices, checks and history kept; the operator enters credentials again. (c) A copy of the key in the macOS Keychain or the Secret Service at creation. | (a) now, (b) as an S9 item. (c) only if it is never synchronized (no iCloud Keychain); the security review prefers the "save a copy" step of 2.8, which S6 builds. |
| OR-S6-2 | Recovery codes, so that an operator away from the hub computer can sign in without the TOTP device | (a) None: recovery is on the hub computer only (F-8). (b) Ten single-use codes at creation; a K1 change to `login`. | (a). A hub on a laptop is near its operator. (b) adds a remote path past the second factor. |
| OR-S6-3 | K1: `linkDevices`, `createLinkInvite`, `revokeLinkDevice` | Planned by S5 Section 7.6 and CMD-030. | Approve with the spec delta. |
| OR-S6-4 | `ANIMA_LINK_CONTROL` (not in K7, as `ANIMA_LINK_SOCKET`) | Add both rows to K7 | Approve with the spec delta. |
| OR-S6-6 | A user-presence check (macOS LocalAuthentication, Linux polkit) before recovery, in addition to the native dialog of F-9 | Now, or later | Later: the native dialog already stops script; the check adds a password prompt for a person at the unlocked computer. |
| OR-S6-7 | `connect-src` permits every loopback port (the device link's port is chosen at the join) | Accept, or one fixed port range | Accept, with F-10; the gateway's missing sign-in (S4 Section 7.4 item 4) is the real fault and stays open. |
| OR-S6-5 | Raw-sample retention of 3 days in the application mode (S3 D-9) | Set at the first run, or not | Not set by S6 until the DB-009 delta is approved. |

## 6. Results

### 6.1 Gates (T10)

`origin/main` (eef4e7b1) merged at 3abe87d0. `BASE_REF=origin/main bun run gates` with
`DATABASE_URL` unset and a private Rust build folder: 20 of 20 gates pass. The Rust tests ran
5587 tests: 5587 pass, 1 skipped. No known timing test failed. The gate must use the default
`PATH` (Homebrew cargo first): rustfmt 1.11 of `~/.cargo/bin` formats
`crates/providers/exchange/src/parts/text.rs` differently from rustfmt 1.10, and `origin/main`
fails `cargo fmt --check` under 1.11.

### 6.2 Run-through on this Mac (T11, 2026-11-16, 3abe87d0)

The steps use the programs and arguments that the shell uses (debug build, `anima-link` from
`link/build.sh`, PostgreSQL 17.12). The desktop window was not driven; Section 6.3 lists the
screen checks.

| Step | What was seen | Result |
| --- | --- | --- |
| Clean state | No `ing.anima.hub` LaunchAgent, no hub root, nothing on 8443. | Pass |
| Create | `install` 27 s; `ready` at 50 s, link `up` 1 s later. Folder modes as in `local-hub.md` Section 1. | Pass |
| Account | `account status` `null`. A wrong code: refused, nothing saved. The right code: `Operator … is set, with TOTP.`; status `totp: true`. | Pass |
| Sign-in (F-5, F-11) | No code: `UNAUTHENTICATED`. `Host: localhost.evil` and a foreign `Origin`: 403. Password and code with `Origin: tauri://localhost`: token. | Pass |
| Invite | `createLinkInvite`: `Cache-Control: no-store`, expiry 15 minutes; the audit entry holds no invite text. | Pass |
| Join | `anima-link join` 0.9 s; the device link ready 0.07 s after its start. Through its port: `linkDevices` shows the device, path `DIRECT`; sign-in works; the audit records the link address. | Pass |
| Second use | `the invite is used, expired or wrong, or the hub is off`, after 10 s. | Pass |
| Revoke | `Streams closed 1`; the open connection is closed at once; `linkDevices` empty; audit `delete link`. | Pass |
| Criterion 4 | TCP listeners: hub 127.0.0.1:8443, gateway 127.0.0.1, device link 127.0.0.1 only. PostgreSQL: Unix socket only. Each link: two UDP sockets on `*` (WireGuard), one outgoing TCP to its relay. No port-mapping lines; `ts_omit_portmapper` in the build tags. | Pass |
| Stop, start, F-6 | `stop` 6 s, nothing left running; `start` ready in 2.4 s. The link killed alone: group stays `ready`, link back `up` in 14.7 s. | Pass |
| Clean-up | `remove`, then the hub root, `pg/` and the device folder deleted; only `builds/` stays. | Pass |

No secret was found in any hub, device or join log (search for the password, TOTP secret, invite
and token: 0 matches).

Seen and not yet handled (follow-ups, not S6 requirements):

- A revoked device's link stays up and logs nothing; new requests through its port are reset
  after about 15 s, so the client sees a slow failure, not "revoked".
- The `login` answer carries a token and no `Cache-Control: no-store`.
- `anima-link join` prints its next command with an unquoted path; under Application Support the
  path has a space.
- A second use of an invite fails only after 10 s.

### 6.3 Deferred live checks (need the operator or another computer)

1. **The desktop window, on this Mac.** Build with `bun run --cwd apps/desktop tauri build --debug`
   and start it with `ANIMA_DEV_PROGRAMS=<folder with anima-local-hub, anima-hub, anima-database,
   anima-gateway, anima-link>` and `ANIMA_DEV_TREE=/opt/homebrew/opt/postgresql@17`. Choose
   "Create a hub on this computer"; scan the TOTP QR code with a phone; finish the steps; save the
   key copy; make an invite and check that its QR code is hidden after 15 minutes. In Settings,
   "Replace the account": the native dialog shows; "Cancel" gives a 60 s wait. With Stay awake off,
   the AWK-005 notice shows. Remove the hub after (Section 6.2, clean-up).
2. **Join from a second Mac.** Make an invite on the first Mac, paste it on the second Mac
   ("Join a hub"), compare the key on the join screen with "Devices on the link" on the first Mac,
   sign in, revoke from the first Mac, and check that the second Mac loses the hub.
3. **Linux.** Repeat check 1 on Linux with the systemd user unit; check F-10 with a listener of
   another user on the hub port (the client must send nothing).
4. **A real router.** On the hub computer, check the router's UPnP or NAT-PMP table: no mapping
   for the hub, and from outside the network no TCP port of the hub answers.
5. **A relayed path.** Join from a network where UDP is blocked; the path must show `RELAYED`.

## 7. What S12 packaging needs

1. **Carried programs.** A release build finds `anima-local-hub` and `anima-link` beside its own
   executable only (`apps/desktop/src-tauri/src/carried.rs`). `anima-local-hub` in turn needs
   `anima-hub`, `anima-database`, `anima-gateway` and a PostgreSQL 17 tree. The bundle must carry
   all of them for each target (macOS arm64 and x86-64, Linux x86-64 and arm64). A debug build
   reads `ANIMA_DEV_PROGRAMS` and `ANIMA_DEV_TREE` instead; a release build never does.
2. **`anima-link` build.** Build it with `link/build.sh` only, which sets the build tags with
   `ts_omit_portmapper`. A plain `go build` includes the port mapper (UPnP, NAT-PMP, PCP), which
   can open a port on the router and break acceptance criterion 4.
3. **Signing.** Sign every carried executable with the application's identity and the hardened
   runtime, and notarize the bundle (macOS). `anima-local-hub account` and the shell check the
   owner and mode of the hub root and of each program before they run it, so the bundle must
   install them owned by the user and not writable by the group or others.
4. **Key store.** Release builds use the service `ing.anima.desktop`; debug builds use
   `ing.anima.desktop.dev`. The first release start finds no debug entries.
5. **Origins.** The hub accepts the client origins `tauri://localhost` (macOS) and
   `http://tauri.localhost` (Linux) in the link mode (F-11). A change of the bundle identifier or
   the custom protocol must change `CLIENT_ORIGINS` in `crates/hub/src/lib.rs`.
6. **CSP and capabilities.** The release `tauri.conf.json` keeps `connect-src` with
   `http://127.0.0.1:*` and `ws://127.0.0.1:*` (OR-S6-7) and the `dialog` plugin (F-9).
7. **Linux.** The local hub uses a systemd user unit. F-10 reads `/proc/net/tcp` and
   `/proc/net/tcp6`; no `lsof` is needed. Check on a Linux computer (Section 6, deferred checks).
