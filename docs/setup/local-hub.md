# The hub on this computer (`anima-local-hub`)

When the hub runs inside the desktop application (one-click plan S4), it runs as a background
program of your user. It starts at login and runs when the application window is closed. The
program is `anima-local-hub`. It starts the database (`anima-database`,
[bundled-database.md](bundled-database.md)), the gateway (`anima-gateway`,
[native-gateway.md](native-gateway.md)) and the hub, in that order, and stops them in the
reverse order.

The Docker installation on a server does not use any of this.

**State of the work:** the program is built and tested. A release of the application does not
carry it yet, and the hub that it starts permits no client yet (plan item S5 sets the access
rules). See `docs/research/one-click/s4-local-hub-tasks.md`, Section 7.

## 1. The folders

Everything is below the hub root, which only your user can open (mode 0700).

| Platform | Hub root |
| --- | --- |
| macOS | `~/Library/Application Support/ing.anima.desktop/hub` |
| Linux | `$XDG_DATA_HOME/anima/hub` (default `~/.local/share/anima/hub`) |

| Path | What it is |
| --- | --- |
| `database/` | The database. See [bundled-database.md](bundled-database.md). |
| `vault.key` | The hub's key (mode 0600). The hub seals the device credentials and the TOTP secrets with it. **Back it up with the database: a backup of the database is of no use without this file.** |
| `programs/current/` | The programs that run: a copy from the application, made at install and at each update. |
| `log/` | `anima-hub.log`, `anima-gateway.log`, `anima-database.log` and `local-hub.log`. Each has one older file (`.1`) and rotates at 8 MB. |
| `state`, `lock` | What the program last reported, and the lock of the one copy that runs. |

The unit file is not below the root:

| Platform | Unit file |
| --- | --- |
| macOS | `~/Library/LaunchAgents/ing.anima.hub.plist` (label `ing.anima.hub`) |
| Linux | `~/.config/systemd/user/anima-hub.service` |

## 2. The commands

The application runs these for you. Each command accepts `--root <folder>` together with
`--socket-folder <folder>` to use other folders.

| Command | What it does |
| --- | --- |
| `anima-local-hub install --tree <folder>` | Copies the programs beside it and the PostgreSQL tree at `<folder>` to `programs/`, writes the unit file and starts the hub. On a hub that runs, it is an update: the hub stops, the programs are replaced, the hub starts. |
| `anima-local-hub remove` | Stops the hub, removes the unit file and `programs/`. **It does not remove the database (with its URL files, which hold passwords), the key or the logs.** To remove those, delete the hub root yourself. On Linux, run `remove` with the same `XDG_DATA_HOME` as `install`: it finds the hub root from it. |
| `anima-local-hub status` | Prints one line of JSON. |
| `anima-local-hub run` | What the unit runs. It prints `ready` when the hub serves and stops on SIGTERM. |

`status` prints, for example:

```json
{"installed":true,"running":true,"state":"ready","hub":"127.0.0.1:8443","recovered":false}
```

| Field | Meaning |
| --- | --- |
| `installed` | The unit file and the programs are there. |
| `running` | A `run` holds the lock now. When it is `false`, `state` is what the last run left. |
| `state` | `starting`, `ready`, `blocked`, `stopping`, `stopped` or `failed`. Absent when the hub never ran. |
| `hub` | With `ready`: the address the hub listens on. |
| `recovered` | With `ready`: the database replayed its log after an unclean stop. Nothing that was saved is lost. |
| `reason` | With `blocked`: see Section 4. |
| `cause` | With `failed`: which program ended. The unit starts the hub again after 10 seconds. |

Exit codes of `run`: 0 stopped on request, 1 a program of the group ended or did not start,
10 another copy runs the hub of this folder.

## 3. Start, stop and a crash

- **Login:** the unit starts `run`.
- **Sign-out, shutdown, `remove`, an update:** the unit sends SIGTERM. `run` stops the hub (it
  ends the open sessions and drains requests for 10 seconds), then the gateway, then the
  database. The unit waits 90 seconds before it kills what is left. A SIGTERM during a start
  does not wait for the start: what did start is stopped in order.
- **A program of the group ends by itself:** `run` stops the others in order and ends with
  code 1. The unit starts the group again after 10 seconds.
- **`run` is killed:** launchd or systemd kills the programs it started. At the next start
  the database recovers from its write-ahead log, and `status` shows `"recovered":true`.

On Linux the hub runs while you are signed in. This version does not turn on lingering
(`loginctl enable-linger`).

## 4. When the state is `blocked`

The hub cannot start until you act. The program stays up and idle, so the unit does not start
it in a loop.

| `reason` | Meaning | What to do |
| --- | --- | --- |
| `key-missing` | `vault.key` is missing and the database is not new. The program makes a key only for a new database: a new key cannot open what the old key sealed. | Put your copy of `vault.key` back (mode 0600), then install again or sign in again. |
| `database-version` | The database is of another PostgreSQL major version than this Anima has. | See [bundled-database.md](bundled-database.md), Section 6. |
| `database-in-use` | Another program runs this database. | Stop the other `anima-database`, then install again or sign in again. |

## 5. What the hub is told

`run` gives the hub the database URL file of its runtime login, the key file, the gateway's
address and `ANIMA_LISTEN`, which is `127.0.0.1:8443` with plain HTTP. It sets no access rule:
with the hub's own default a client on this computer is refused and only `/health` answers.
Plan item S5 sets the rules and the addresses of the application mode.

The gateway listens on a loopback port that the operating system picks. It has no sign-in, so
any program of this computer can ask it to connect to a VNC host
([native-gateway.md](native-gateway.md)). Another user of this computer can also hold all of
its 64 sessions and so stop your remote display sessions. On a computer that other people
use, this is a known limit until plan item S5 closes it.

`install` refuses a hub root or a unit folder that another user can change, or that has such
a folder above it: that user could put a program of their own where the unit runs one at
each login. The message names the folder; run `chmod go-w` on it.

## 6. For plan item S6: what the application calls

The Tauri shell has these commands, for the main window only.

| Command | Result |
| --- | --- |
| `local_hub_carried` | `true` when this build carries the programs. |
| `local_hub_install` | Runs `install`. Also the update: call it after the application updated itself. |
| `local_hub_remove` | Runs `remove`. |
| `local_hub_start`, `local_hub_stop` | Run `start` or `stop`. |
| `local_hub_status` | The JSON object of Section 2. |
| `local_hub_account` | `{ "name", "totp" }` of the operator account, or `null`. |
| `local_hub_totp_new { name }` | `{ "secret", "uri" }`: a new TOTP secret. Nothing is saved. |
| `local_hub_account_create { account: { name, password, secret, code } }` | Makes the account. Permitted only while there is no account, or for 5 minutes after `local_hub_recover`. |
| `local_hub_recover` | Shows a native confirmation. `true` when the person confirmed. |
| `local_hub_key_save` | Shows a native save dialog and copies `vault.key` there (mode 0600). `false` when cancelled. |

An error is `{ "code", "message" }` with the codes `caller`, `unavailable`, `not-carried`,
`busy`, `no-home`, `timed-out`, `failed`, `argument`, `exists`, `refused`, `no-key` and
`destination`. The cause of a `failed` is in the application's log. The account commands log no
output of the program, only a code and the exit code.

A debug build (`bun tauri dev`) carries no programs. To try the hub from a debug build, set
`ANIMA_DEV_PROGRAMS` to a folder that holds `anima-local-hub` and `anima-link`, and
`ANIMA_DEV_TREE` to a PostgreSQL tree. A release build ignores both variables.
