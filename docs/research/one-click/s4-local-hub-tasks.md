# S4 task breakdown: the hub as a background program

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version 1.0. Date: 2026-11-15. Status: **In progress.** The planning session started this item after the
operator said "go ahead and build it out" there. Each design choice is the recommendation of the
build session; no operator ruling was asked for. Section 2 holds the state of each task, Section 6
the choices taken while building, Section 7 the state at hand-over and what S6 needs.

Branch: `claude/s4-local-hub`, cut from `claude/s3-database` at `ec0ba639` (which holds
`origin/main` at `4c27cceb`).
Sources: `docs/one-click-plan.md` Section 4.4 (on branch `claude/one-click-plan`),
`s3-database-tasks.md` Section 1.6, `docs/setup/bundled-database.md` Section 7,
`docs/setup/native-gateway.md`, and the agent's unit code `crates/agent/src/units/` (MET-019).

Scope: on macOS and Linux the hub runs when the application window is closed and starts at
login. The Tauri shell installs and removes it. The VPS mode (Docker, Headscale) does not change.

Not in scope, owned by other items:

- **S5:** the hub's access rules (NET-001, NET-002, BLD-024, the agent token path) and the hub's
  listen addresses. S4 sets no access rule. See Section 1.4.
- **S6:** the first-run screens, the operator account, the invite, starting `anima-link`. S4
  leaves one place for the link (Section 1.3) and gives S6 the commands of Section 1.6.
- **S9:** the schedule of backups.

## 1. Design

### 1.1 One program for the group

The hub on a desktop needs three programs today and a fourth with S1: `anima-database`
(PostgreSQL, S3), `anima-gateway` (VNC, S2), `anima-hub`, and later `anima-link`. launchd and
systemd user units can start one program and keep it up; an order between four units with a
one-shot step (`anima-hub migrate`) between them is possible in systemd and not in launchd.

So S4 adds one program, `anima-local-hub`, in a new crate `crates/anima-local-hub`. The
operating system keeps this one program up; the program keeps the group in order. "Local hub" is
the operator's noun for "the hub on this computer", beside "the hub on a server".

| Command | What it does |
| --- | --- |
| `run` | Starts the group in order, prints `ready`, keeps it up, stops it in order on SIGTERM or SIGINT. The unit runs this. |
| `install` | Copies the programs out of the application bundle, writes the LaunchAgent or the systemd user unit and starts it. A second `install` is an update. |
| `remove` | Stops the unit, removes the unit file and the copied programs. It never removes the database, the key or the logs. |
| `status` | Prints one JSON object: installed or not, running or not, the state, the hub's address. No secret. |

The hub, the database program and the gateway do not change. Each stays a program with a ready
line and a stop signal. `anima-local-hub` starts `anima-database serve` as a child and does not
link the database library: one seam (the one of `bundled-database.md` Section 7) for every
member of the group.

### 1.2 Folders

All below the hub root, which only the user can open (mode 0700). The database root of S3 is
`<hub root>/database`, so the hub root is its parent.

| Platform | Hub root |
| --- | --- |
| macOS | `~/Library/Application Support/ing.anima.desktop/hub` |
| Linux | `$XDG_DATA_HOME/anima/hub` (default `~/.local/share/anima/hub`) |

```text
<hub root>/database/           S3: cluster, urls, server.log, lock
<hub root>/programs/current/   bin/anima-local-hub, anima-hub, anima-database, anima-gateway
                               database-tree/  the PostgreSQL tree (bin, lib, share)
<hub root>/programs/next/      a copy in progress; never run
<hub root>/vault.key           the hub's ANIMA_KEY_FILE, mode 0600 (Section 1.5)
<hub root>/log/                hub.log, gateway.log, local-hub.log, each with one older file
<hub root>/lock                held by the one `run` of this root
<hub root>/state               what `run` last reported; `status` reads it
```

`run`, `install`, `remove` and `status` take `--root <folder>` and `--socket-folder <folder>`
for tests and trials. Without them the folders are the ones above.

### 1.3 The order

Start:

1. Take `<hub root>/lock`. A second `run` ends with exit code 10 and changes nothing.
2. Make `vault.key` when the rule of Section 1.5 permits it.
3. `anima-database serve --tree <programs>/database-tree`. Wait for `ready` (up to 330 s: a
   start after a kill replays the write-ahead log).
4. `anima-hub migrate` with `ANIMA_DATABASE_URL_FILE=<database>/urls/owner.url` (R-016). Up to 300 s.
5. `anima-gateway listen 127.0.0.1:0`. Read the port from its line `listening on <address>`.
6. `anima-hub` with `hub.url`, the key file and the gateway's address. Wait for its log line
   `hub started` and read `fields.listen` from it.
7. **Place for S6:** `anima-link` starts here, after the hub, and stops first. The group is a
   list of members; the link is one more entry (`members.rs`).
8. Write the state `ready` and print `ready`.

Stop (SIGTERM or SIGINT): the reverse order. The hub gets SIGTERM and 20 s (it drains requests
for 10 s), the gateway SIGTERM and 5 s, the database SIGTERM and 45 s (its own limit to stop
PostgreSQL is 30 s). After the limit a member is killed. The unit gives the whole program 90 s
(`ExitTimeOut`, `TimeoutStopSec`), which is more than the sum.

A signal during the start acts at once: the wait in progress ends, a migration in progress is
stopped, and the members that did start are stopped in order.

When a member ends by itself: stop the others in order, then

| Cause | What `run` does |
| --- | --- |
| The hub, the gateway or the database ended (database code 1 or 3), or `migrate` failed | Exit code 1. The unit starts the group again after 10 s. |
| Database exit code 10 (another copy) or 11 (other PostgreSQL major version), or the key rule of Section 1.5 refuses | Write the state `blocked` with the reason. Stay up and idle until SIGTERM. The unit does not start it again in a loop, and `status` can tell the operator. |

Each child gets a cleared environment with only what it needs, its own log file, and no
standard input. A child's output is copied to its log file line by line; the log files rotate at
8 MB with one older file, as `server.log` does.

### 1.4 What the hub is told (and what S5 must set)

| Setting | Value | Owner |
| --- | --- | --- |
| `ANIMA_DATABASE_URL_FILE` | `<database>/urls/hub.url` | S4 |
| `ANIMA_KEY_FILE` | `<hub root>/vault.key` | S4 |
| `ANIMA_GUACD` | the gateway's address | S4 |
| `ANIMA_LOG` | `info` | S4 |
| `ANIMA_LISTEN` | `127.0.0.1:8443`, plain HTTP (NET-003 permits it on loopback only) | **S5.** S4 gives the K7 default so that the group can start. |
| `ANIMA_PUBLIC_ORIGIN` | not set: the hub's default, `https://hub.anima.ing` | **S5.** It must name the origin that the client uses. |
| `ANIMA_PERMITTED_NETWORKS` | not set: the hub's default, `100.64.0.0/10` | **S5.** With the default, a client on loopback gets 403; only `/health` answers. S4 does not open the hub. |
| `ANIMA_PROBE_DENY` | not set (the hub warns) | **S5** |
| All others | not set: no TLS, no web client, no Tailscale, no VPN broker, no release publishing | — |

These are in one function, `hub_environment` in `members.rs`, with the S5 rows marked. `run` has
`--listen <address:port>` for tests and trials only.

### 1.5 The hub key

The hub does not start without `ANIMA_KEY_FILE`: 32 random bytes, base64. It seals the device
credentials and the TOTP secrets in the database. On the VPS `deploy/bin/make-secrets` makes it.

Rule: `run` makes `vault.key` (mode 0600, `O_EXCL`, no link followed) **only when the file is
absent and the database is new**: the cluster folder is not there (an error other than "not
found" is not "new"). The key is made before the database, so a crash between the two leaves a
key and no database. When the key is absent and the
database is not new, `run` goes to the state `blocked` with the reason `key-missing` and makes
no key: a new key cannot open what the old one sealed, and the operator must put the key back.

The key is never printed, logged or put on a command line (LOG-007). S9 must back it up with
the database; a backup of one without the other cannot be restored.

### 1.6 Install, update and remove

`install` runs from the copy in the application bundle:

1. Check the carried programs: `anima-hub`, `anima-database` and `anima-gateway` beside
   `anima-local-hub`, and the PostgreSQL tree at `--tree <folder>`.
2. Copy them to `programs/next` (regular files and links that stay inside the tree; all else is
   refused). Folders 0700, programs 0700, other files 0600.
3. Stop the unit if it runs. Wait until `<hub root>/lock` is free.
4. `programs/current` becomes `programs/previous`, `next` becomes `current`, `previous` is removed.
5. Write the unit file. Start the unit.

So a running database never has its tree replaced under it (DBL-013), and an application update
that replaces the bundle does not touch the running programs. The Tauri shell runs `install`
again after an update (S6 decides when).

| Platform | Unit file | Commands |
| --- | --- | --- |
| macOS | `~/Library/LaunchAgents/ing.anima.hub.plist` | `launchctl bootout gui/<uid>/ing.anima.hub` (may fail), `launchctl bootstrap gui/<uid> <file>` |
| Linux | `~/.config/systemd/user/anima-hub.service` | `systemctl --user daemon-reload`, `enable`, `restart`; to stop: `disable --now` |

The unit: start at login, start again after a failure with 10 s between, 90 s to stop, no core
file. On Linux no `loginctl enable-linger`: the hub runs while the user is signed in, as the
agent's user unit does. A hub that must run with nobody signed in is a later item.

`remove`: stop the unit, wait for the lock, remove the unit file and `programs/`.

### 1.7 The Tauri shell

`apps/desktop/src-tauri/src/local_hub/` holds only OS work: find `anima-local-hub` beside the
application's own program and the tree in the bundle's resources, and run it with a cleared
environment and a time limit. Commands: `local_hub_carried`, `local_hub_install`,
`local_hub_remove`, `local_hub_status`. They take no path and no program from the web view
(as `agent_install`, D-10 d). The screens that call them are S6.

### 1.8 Command line and MCP (CMD-030)

CMD-030 is for functions of the hub API. S4 adds no root field to `contracts/schema.graphql`.
`anima-local-hub` is itself the command line of this function. It gets no MCP tool: `install`
and `remove` change what runs on the computer at login, which Section 7 of the CLI plan keeps
out of tools.

## 2. Tasks

| Task | What | State | Commits |
| --- | --- | --- | --- |
| T1 | Crate, the folders, the lock, the key file rule | Done. `reviewer` and `reviewer-security`; findings corrected. | `5b954d7a`, `4521241f` |
| T2 | A child program: sealed environment, ready line, log file, stop with a limit | Done. `reviewer` twice; findings corrected (the second: a member's end is seen also when its own child keeps the output open). | `5b954d7a`, `2e7486b1`, and the commit of this row |
| T3 | The group: order, stop, a member that ends, the state; `run` and `status`. A test with the real programs. | Done. `reviewer`: 1 must-fix (a stop during the start) corrected; second review: closed, 2 test faults corrected. `reviewer-security`: no must-fix. | `5b954d7a`, `47e4cd60` |
| T4 | The unit files, `install` and `remove`, the copy of the programs | Done. `reviewer`: 1 must-fix (the swap) corrected. `reviewer-security`: 2 should-fix corrected. | `5b954d7a`, `4521241f`, `47e4cd60` |
| T5 | The Tauri shell commands | Done. Both reviews: sound. | `2ec288af`, `47e4cd60` |
| T6 | Trial with a real LaunchAgent on this Mac, in a trial folder; removed after | Done. Section 7.3 has the result. | — |
| T7 | Build and tests on Linux in a container | Done: arm64, Debian 12, a user without root; 65 of 65 tests at `2e7486b1`. | — |
| T8 | Documents: `docs/setup/local-hub.md`, the spec delta, what S6 needs | Done | `5b954d7a`, `4521241f`, and the commit of this table |
| T9 | The release builder carries the four programs and the tree (with S3 T11) | Not started. Section 7.2 has the steps. | — |
| T10 | The systemd user unit on a real Linux desktop; the macOS Local Network prompt for a hub that launchd started | Not started: needs the operator and a Linux desktop. | — |

## 3. Where a ruling is necessary

None is asked for before the build. To be confirmed by the operator after it: the requirement
texts in the spec delta (T8), and the choices marked "S4 recommendation" in Section 6.

## 4. Gates and reviews

`BASE_REF=origin/main bun run gates` with a private Rust build folder, on the branch with
`origin/main` merged in. Tests that start PostgreSQL join the nextest group `bundled-postgres`.

## 6. Choices taken while building

- **Name.** `anima-local-hub`, the "local hub". The repository bans `service` in names; the systemd
  file is still `anima-hub.service`, as the agent's is `anima-agent.service`.
- **Children, not a library.** `run` starts `anima-database serve` as a program. One seam for every
  member, and the seam is the one that `bundled-database.md` Section 7 gives.
- **Members stay in the program's process group.** launchd kills what is left of a job's process
  group when the job's program dies; systemd kills the control group. So a `kill -9` of `run` leaves
  no member that holds the database or the hub's port. The trial proved it on launchd. The price:
  Ctrl-C in a terminal reaches every member at once.
- **One pipe for a member's two outputs.** The lines are read in the order written, so the database's
  "recovered" notice is never lost to a race with its `ready` line.
- **`blocked` stays up.** launchd cannot be told "do not start again after exit code 11". The program
  stays up and idle with the state `blocked`, so the unit does not loop and `status` can tell why.
- **The hub key.** Made by `run` before the database, only when no cluster folder is there. A link or
  a folder at its path is refused. Reason for a file and not the Keychain: the unit starts with no
  person there to permit a Keychain read.
- **`ANIMA_PUBLIC_ORIGIN` is not set** (the first design set it). It is S5's with the listen address.
- **The gateway's port** is picked by the operating system and read from the gateway's ready line.
  No fixed port can be taken by another program first.
- **Install checks every folder above the hub root and the unit file** (owner, no write for group or
  others without the sticky bit), as the agent's install does. ACLs are not read.
- **`anima-database`:** `owner_only` is now public and `startup::RECOVERED_NOTICE` is a constant, so
  that the two programs share them. No behaviour changed.
- **Shared file:** `.config/nextest.toml`: this crate's `it` test joins the group `bundled-postgres`.
- **No client TypeScript.** The four shell commands have no caller yet; S6 writes the screens.
- **Linux sandbox:** only `NoNewPrivileges` and `LimitCORE=0`. A tighter unit needs T10 first.
- **Not corrected from the reviews, with the reason:**
  - The gateway has no sign-in, and another user of the computer can hold its 64 sessions. It is
    S2's accepted state on loopback; the fix (a Unix socket or a secret) changes the gateway and the
    hub's `ANIMA_GUACD`, so it is listed for S5 in Section 7.4 and in the guide.
  - A log file that cannot be written does not stop a member, and the fault is not reported.
  - `status` after a `kill -9` of `run` shows the last state with `"running":false`. The guide says
    to read `running` first.
  - SIGHUP is not handled.
  - Tests that the second review asked for and that are not written: a state file that cannot be
    written, a `status` at the instant a `run` starts, the note for a killed member, the 2 GB limit
    of the copy, and a stop during the gateway's and the hub's own wait (the database's and the
    migration's are tested).

## 7. State at hand-over

### 7.1 Done

T1 to T8. The program, its unit files, its install and remove, the shell commands and the
documents are built, reviewed and tested on macOS and in a Linux container.

### 7.2 Open

- **T9, the bundle.** A release does not carry the programs, so `local_hub_carried` answers
  `false` in every build today. Steps, with S3 T11 (its Section 7.2 has the security conditions):
  1. `builder/sidecar.rs`: build `anima-local-hub`, `anima-hub`, `anima-database` and
     `anima-gateway` for each triple as it builds `anima`, and `lipo` them on macOS.
  2. `plan::bundle`: add the four to `externalBin`, and `"resources": {"database": "database"}`
     (Linux: the tree must land at `usr/lib/Anima/database`; `local_hub/carried.rs` expects it).
  3. `artifact::mac_binaries`: accept the four names.
  4. Run `database-tree/build.sh` before the bundle, signed with the Developer ID.
  5. Check each program with `lipo -archs` and `codesign --verify --strict`, with the hardened
     runtime and without `allow-dyld-environment-variables`.
- **T10.** A real Linux desktop: `install`, sign out and in, `kill -9` of `run`, `remove`. A Mac:
  the Local Network prompt when the hub that launchd started first connects to a LAN device.
- **The spec delta** [s4-spec-delta.md](s4-spec-delta.md) (LHB-001 to LHB-011) waits for the operator.
- **The work plan** has no tier for the one-click work (its Section 4, note). The operator must
  give one (PLN-003).
- **PLN-011:** the rule came to `main` on 2026-11-15 while this was built. This document and the
  S3 one are on a branch until the operator approves the merge.

### 7.3 Results

- Gates: see the last commit of the branch.
- macOS, a real LaunchAgent (label `ing.anima.hub`, trial root in `~/Library/Caches`, debug
  programs, the universal PostgreSQL tree of the S3 recipe), 2026-11-15:
  - `install`: the hub was `ready` in about 15 s on a new database; `anima-hub health` passed.
  - `kill -9` of the hub: the group stopped in order and launchd started it again; `ready` in 10 s.
  - `kill -9` of `run`: no member was left; PostgreSQL got a fast shutdown from launchd's signal
    to the process group; `ready` again in 8 s.
  - A second `install` (update): stop in order, programs replaced, `ready` again, 4 s in all. The
    key was kept.
  - `remove`: 1 s. No process, no unit file, no listener. The database, key and logs stayed.
  - The trial folder and the agent file were removed after it.
- Linux, arm64 container (Debian 12, a user without root, the tree of the S3 recipe): build,
  Clippy and 65 of 65 tests, the three with the real programs included. Nothing left running.
  systemd itself was not run.

### 7.4 What S5 and S6 need

**S6 (first run):**

1. Call the shell commands of `docs/setup/local-hub.md` Section 6. `local_hub_install` is also
   the update: call it after each application update, or the hub keeps the old programs and gets
   no security correction. It can take minutes when a hub must stop first.
2. Poll `local_hub_status` and show the states (LHB-010). Read `running` before `state`. For
   `blocked` with `key-missing`, tell the operator to put the key back. Never offer to make a
   new key.
3. The operator account: run `anima-hub operator set <name>` with the password on standard input
   and `ANIMA_DATABASE_URL_FILE=<hub root>/database/urls/hub.url`, as
   `crates/hub/tests/it/listening/bundled.rs` does. `operator totp` needs a terminal; S6 needs
   another way to show the TOTP secret.
4. The link: one more member. Add its `Spec` in `members.rs` and one step at the end of
   `Group::start` (both places are marked). It starts after the hub with
   `anima-link hub -state <hub root>/link -target <hub address>`, a cleared environment and a stop
   limit, and it stops first. `PROGRAMS` in `members.rs` and T9 must then carry `anima-link`.
5. The client accepts only `https:` hub addresses today (`apps/client/src/lib/hub/address.ts`),
   and the Tauri content security policy permits only `https:` and `wss:`.
6. The raw-sample retention of 3 days (S3 decision D-9) is set at the first run.

**S5 (access rules):**

1. Everything in `hub_environment` in `members.rs` that is marked: `ANIMA_LISTEN`,
   `ANIMA_PUBLIC_ORIGIN`, `ANIMA_PERMITTED_NETWORKS`, `ANIMA_PROBE_DENY`.
2. Another user of the computer can take `127.0.0.1:8443` before the hub (the group then fails
   and starts again every 10 s). Before a client trusts the local hub, the hub's endpoint needs a
   proof: a Unix socket in a folder of the user, or a certificate made at install and pinned.
3. To permit `127.0.0.1` in the hub permits every user of the computer; NET-002 is then no gate.
4. The gateway: a Unix socket or a secret, so that only the hub can use it.
