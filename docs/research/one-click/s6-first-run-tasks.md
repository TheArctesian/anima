# S6 task breakdown: the first run

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version 1.0. Date: 2026-11-15. Status: **In progress.** The S1 to S8 planning session started this
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
    secret (base32) and the `otpauth://` URI. It saves nothing. No terminal is needed.
  - `anima-hub operator totp <name> --confirm`: reads the secret and one code from standard input
    (two lines). It saves the secret (sealed) only when the code is correct now, records the code's
    step so that the code cannot be used again, and removes the operator's tokens, as
    `operator totp` does.
- **F-5.** The first run requires TOTP (OD-2). There is no "skip". The client signs in after the
  confirmation with the next code.
- `anima-local-hub account status|set <name>|totp-new <name>|totp-confirm <name>` runs the
  installed `anima-hub` with the database URL file and the key file of the hub root. The secrets
  pass through standard input and standard output only (LOG-007).

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
  set (the VPS mode) the fields answer with the error `LINK_OFF`.
- K1 fields (planned in `s5-access-tasks.md` Section 7.6):
  - `Query.linkDevices: [LinkDevice!]!` (name, fingerprint, address, path, relay, expiry, streams).
  - `Mutation.createLinkInvite(name: String!): LinkInvite!` (invite text, expiry). Returns a secret.
  - `Mutation.revokeLinkDevice(key: String!): Int!` (closed streams). A trust decision.
  - Both mutations need a login token, not an API key, and write an audit entry. The invite text
    is never logged or audited.
- `anima link devices`, `anima link invite <name>`, `anima link revoke <key>`.
- MCP: one tool, `link_devices` (read). No tool for the invite (MCP-004: it returns a secret) or for
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

- **F-8.** Recovery is local only. A joined device, an API key or a login token cannot reset the
  password or TOTP. Reason: the person at the hub computer's user account can already read the
  database and the key; a remote reset would be a new path to take the hub.
- The creation flow shows where `vault.key` is and that it must be backed up with the database.

### 2.9 Acceptance criterion 4 (no open inbound port on a public interface)

The hub, the gateway and PostgreSQL listen on loopback or on Unix sockets. `anima-link` holds one
UDP socket for WireGuard; it answers only the keys in its list (LNK-002) and serves no TCP port.
The run-through records `lsof -i` for each program.

## 3. Tasks

| Task | What | Proof | Review | State | Commits |
| --- | --- | --- | --- | --- | --- |
| T1 | This document, the spec delta, the register row | — | — | Done | the commit of this row |
| T2 | Hub: `operator show`, `operator totp --new`, `operator totp --confirm` | Tests: a wrong code saves nothing; a used code is refused at sign-in; tokens removed; no secret in logs | reviewer, reviewer-security | Not started | — |
| T3 | Hub: `fleet/link` domain, `ANIMA_LINK_CONTROL`, K1 fields, audit | Tests against a fake control socket; `LINK_OFF` without the variable; API key refused for the mutations | reviewer, reviewer-security | Not started | — |
| T4 | `anima link devices|invite|revoke`; MCP `link_devices`; CMD-031 | CLI tests against the fake hub | reviewer | Not started | — |
| T5 | Local hub: the link member (F-6), hub link-mode environment, `account`, `start`, `stop` | Unit tests with stand-in programs; the existing real-program test | reviewer, reviewer-security | Not started | — |
| T6 | Tauri shell: account, start/stop, carried `anima-link`, device link (join, run, status); CSP | Rust unit tests; caller check on each command | reviewer, reviewer-security | Not started | — |
| T7 | Client: loopback address rule (F-1), QR code, AWK-005 seam (F-3) | Vitest | reviewer | In progress (built, waits for review) | 3ffae7d5 |
| T8 | Client: first-run screens (choices, create, account, TOTP, invite, join, recover) and the two Settings sections | Vitest; `reviewer-interface` | reviewer, reviewer-interface, reviewer-security (account and invite) | Not started | — |
| T9 | Guide `docs/setup/first-run.md`; this document's results | — | — | Not started | — |
| T10 | `origin/main` merged, full gates on that commit | Gate record | — | Not started | — |
| T11 | Run-through on this Mac from a clean state (create a hub, account, invite, a second device link on this Mac, second use of the invite, revocation, `lsof`); LaunchAgent and folders removed after | Section 6 | — | Not started | — |

## 4. Gates and reviews

`BASE_REF=origin/main bun run gates` with `DATABASE_URL` unset and a private Rust build folder.
Under load, re-run alone `ra_031_without_a_wake…`, `sources::controls::src_004…` and the
`provider-github` fixture tests before a failure counts.

## 5. Where a ruling is necessary

| ID | Question | Options | Recommendation |
| --- | --- | --- | --- |
| OR-S6-1 | `vault.key` is lost and the database is present, with no backup | (a) Only "put the key back"; else start a new hub. (b) A "reset sealed data" step: a new key, every sealed value (device credentials, TOTP secrets, other sealed columns) removed, devices, checks and history kept; the operator enters credentials again. (c) A copy of the key in the macOS Keychain or the Secret Service at creation. | (a) now, (b) as an S9 item. (c) is refused: the unit starts with no person there to permit a Keychain read (S4 Section 6). |
| OR-S6-2 | Recovery codes, so that an operator away from the hub computer can sign in without the TOTP device | (a) None: recovery is on the hub computer only (F-8). (b) Ten single-use codes at creation; a K1 change to `login`. | (a). A hub on a laptop is near its operator. (b) adds a remote path past the second factor. |
| OR-S6-3 | K1: `linkDevices`, `createLinkInvite`, `revokeLinkDevice` | Planned by S5 Section 7.6 and CMD-030. | Approve with the spec delta. |
| OR-S6-4 | `ANIMA_LINK_CONTROL` (not in K7, as `ANIMA_LINK_SOCKET`) | Add both rows to K7 | Approve with the spec delta. |
| OR-S6-5 | Raw-sample retention of 3 days in the application mode (S3 D-9) | Set at the first run, or not | Not set by S6 until the DB-009 delta is approved. |

## 6. Results

(Filled in by T9 and T11.)

## 7. What S12 packaging needs

(Filled in by T9.)
