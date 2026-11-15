# S4 task breakdown: the hub as a background program

Date: 2026-11-15. Status: **in build. The planning session started this item after the operator
said "go ahead and build it out" there. Each design choice below is the recommendation of this
session; no operator ruling was asked for. Section 6 records the choices taken while building,
Section 7 the state at hand-over.**

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
2. `anima-database serve --tree <programs>/database-tree`. Wait for `ready` (up to 330 s: a
   start after a kill replays the write-ahead log).
3. Make `vault.key` when the rule of Section 1.5 permits it.
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

A signal during the start is kept and acts when the step in progress is done.

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
| `ANIMA_PUBLIC_ORIGIN` | `http://127.0.0.1:8443` | **S5** |
| `ANIMA_PERMITTED_NETWORKS` | not set: the hub's default, `100.64.0.0/10` | **S5.** With the default, a client on loopback gets 403; only `/health` answers. S4 does not open the hub. |
| `ANIMA_PROBE_DENY` | not set (the hub warns) | **S5** |
| All others | not set: no TLS, no web client, no Tailscale, no VPN broker, no release publishing | — |

These are in one function, `hub_environment` in `members.rs`, with the S5 rows marked. `run` has
`--listen <address:port>` for tests and trials only.

### 1.5 The hub key

The hub does not start without `ANIMA_KEY_FILE`: 32 random bytes, base64. It seals the device
credentials and the TOTP secrets in the database. On the VPS `deploy/bin/make-secrets` makes it.

Rule: `run` makes `vault.key` (mode 0600, `O_EXCL`, no link followed) **only when the file is
absent and the database is new**: `anima-hub migrate` has not yet applied a migration, which
`run` knows because the cluster folder was absent before step 2. When the key is absent and the
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

| Task | What | Review |
| --- | --- | --- |
| T1 | Crate, the folders, the lock, the key file rule | reviewer, security |
| T2 | A child program: sealed environment, ready line, log file, stop with a limit | reviewer |
| T3 | The group: order, stop, a member that ends, the state; `run` and `status`. A test with the real programs. | reviewer, security |
| T4 | The unit files, `install` and `remove`, the copy of the programs | reviewer, security |
| T5 | The Tauri shell commands | reviewer, security |
| T6 | Trial with a real LaunchAgent on this Mac, with `--root` in a trial folder; removed after | — |
| T7 | Build and tests on Linux in a container | — |
| T8 | Documents: `docs/setup/local-hub.md`, the spec delta, what S6 needs | — |
| T9 | Deferred: the release builder carries the four programs and the tree (with S3 T11) | operator |
| T10 | Deferred: systemd user unit on a real Linux desktop; macOS Local Network prompt for a hub that launchd started | operator |

## 3. Where a ruling is necessary

None is asked for before the build. To be confirmed by the operator after it: the requirement
texts in the spec delta (T8), and the choices marked "S4 recommendation" in Section 6.

## 4. Gates and reviews

`BASE_REF=origin/main bun run gates` with a private Rust build folder, on the branch with
`origin/main` merged in. Tests that start PostgreSQL join the nextest group `bundled-postgres`.

## 6. Choices taken while building

(Filled in during the build.)

## 7. State at hand-over

(Filled in at the end.)
