# The hub on this computer (`anima-local-hub`)

When the hub runs inside the desktop application (one-click plan S4), it runs as a background
program of your user. It starts at login and runs when the application window is closed. The
program is `anima-local-hub`. It starts the database (`anima-database`,
[bundled-database.md](bundled-database.md)), the gateway (`anima-gateway`,
[native-gateway.md](native-gateway.md)), the hub and the link (`anima-link`,
[link.md](link.md)), in that order, and stops them in the reverse order.

The Docker installation on a server does not use any of this.

For the first start of the application, see [first-run.md](first-run.md).

**State of the work:** the program is built and tested. A release of the application does not
carry it yet. See `docs/research/one-click/s4-local-hub-tasks.md`, Section 7, and
`docs/research/one-click/s6-first-run-tasks.md`.

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
| `link/` | The link's state: its key, its devices and invites, and its control socket `control.sock`. **Back it up with the database:** a new link key is a new hub address, and each device must join again. |
| `log/` | `anima-hub.log`, `anima-gateway.log`, `anima-database.log`, `anima-link.log` and `local-hub.log`. Each has one older file (`.1`) and rotates at 8 MB. |
| `state`, `lock` | What the program last reported, and the lock of the one copy that runs. |

The hub's socket for the link, `link.sock`, is in the database's socket folder: on macOS
`~/Library/Application Support/ing.anima.desktop/pg`, on Linux `$XDG_RUNTIME_DIR/anima`. That
folder is your user's, with mode 0700, and its path is short. A socket path can have at most
103 bytes. If `<hub root>/link/control.sock` is longer, the link does not start, the state of
the link is `down`, and `anima-link.log` tells the path and its length.

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
| `anima-local-hub remove` | Stops the hub, removes the unit file and `programs/`. **It does not remove the database (with its URL files, which hold passwords), the key or the logs.** To remove those, delete the hub root yourself. On macOS also delete the socket folder `~/Library/Application Support/ing.anima.desktop/pg`, which keeps a stale `link.sock`. On Linux, run `remove` with the same `XDG_DATA_HOME` as `install`: it finds the hub root from it. |
| `anima-local-hub status` | Prints one line of JSON. |
| `anima-local-hub stop` | Stops the hub now and waits until it stopped. The hub stays installed: it starts again at the next login or with `start`. macOS: `launchctl bootout`; Linux: `systemctl --user stop`. |
| `anima-local-hub start` | Starts the installed hub. On macOS a hub that launchd has loaded already is left as it is. |
| `anima-local-hub account …` | The operator account. See Section 7. |
| `anima-local-hub run` | What the unit runs. It prints `ready` when the hub serves and stops on SIGTERM. |

`status` prints, for example:

```json
{"installed":true,"running":true,"state":"ready","hub":"127.0.0.1:8443","recovered":false,"link":{"state":"up","since":"2026-11-15T10:00:00Z"}}
```

| Field | Meaning |
| --- | --- |
| `installed` | The unit file and the programs are there. |
| `running` | A `run` holds the lock now. When it is `false`, `state` is what the last run left. |
| `state` | `starting`, `ready`, `blocked`, `stopping`, `stopped` or `failed`. Absent when the hub never ran. |
| `hub` | With `ready`: the address the hub listens on. |
| `recovered` | With `ready`: the database replayed its log after an unclean stop. Nothing that was saved is lost. |
| `link` | With `ready`: `state` is `starting`, `up` or `down`, and `since` is the time of that change (RFC 3339, UTC). Absent in a state that an older version wrote. |
| `reason` | With `blocked`: see Section 4. |
| `cause` | With `failed`: which program ended. The unit starts the hub again after 10 seconds. |

Exit codes of `run`: 0 stopped on request, 1 a program of the group ended or did not start,
10 another copy runs the hub of this folder. `start` and `stop`: 0 done, 1 failed.

## 3. Start, stop and a crash

- **Login:** the unit starts `run`.
- **Sign-out, shutdown, `remove`, an update:** the unit sends SIGTERM. `run` stops the hub (it
  ends the open sessions and drains requests for 10 seconds), then the gateway, then the
  database. The unit waits 90 seconds before it kills what is left. A SIGTERM during a start
  does not wait for the start: what did start is stopped in order.
- **A program of the group ends by itself:** `run` stops the others in order and ends with
  code 1. The unit starts the group again after 10 seconds.
- **The link ends, or cannot start:** the link needs its relay, so with no network it ends.
  The hub still serves this computer: the group stays `ready` and the link state is `down`.
  `run` starts the link again alone after 10 seconds, then 20, 40, and then each 60 seconds.
  After the link was up for 5 minutes, the next pause is 10 seconds again. A stop during a
  pause acts at once. The link gets 5 seconds to stop, and it stops before the hub.
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
address, `ANIMA_LISTEN` (`127.0.0.1:8443`, plain HTTP), and the link mode of S5:
`ANIMA_LINK_SOCKET=<socket folder>/link.sock` and `ANIMA_LINK_CONTROL=<hub root>/link/control.sock`.
In the link mode the hub permits this computer and the devices of the link
([link.md](link.md), Section 7). `ANIMA_PERMITTED_NETWORKS` is not set.

`run` starts `anima-link hub -state <hub root>/link -target <socket folder>/link.sock` with an
empty environment. Its ready line is `msg="the hub link is ready"` in its log.

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
| `local_hub_account_create { account: { name, password, secret, code } }` | Makes the account. When an account exists, a native dialog first asks the person at this computer to confirm its replacement, for this request only; script cannot answer it. For 60 seconds after a cancel no dialog is shown and the answer is `busy`. |
| `local_hub_key_save` | Shows a native save dialog and copies `vault.key` there (mode 0600). `false` when cancelled. |

An error is `{ "code", "message" }` with the codes `caller`, `unavailable`, `not-carried`,
`busy`, `no-home`, `timed-out`, `failed`, `argument`, `cancelled` (the replacement was not
confirmed), `refused`, `no-key` and
`destination`. The cause of a `failed` is in the application's log. The account commands log no
output of the program, only a code and the exit code.

A debug build (`bun tauri dev`) carries no programs. To try the hub from a debug build, set
`ANIMA_DEV_PROGRAMS` to a folder that holds `anima-local-hub` and `anima-link`, and
`ANIMA_DEV_TREE` to a PostgreSQL tree. A release build ignores both variables.

## 7. The operator account

The account is made on the hub computer only. No hub API makes it, and a device cannot reset it
(first-run plan, F-4 and F-8). `account` runs the installed `anima-hub` (in `programs/current/bin`)
with an empty environment except `ANIMA_DATABASE_URL_FILE=<hub root>/database/urls/hub.url` and
`ANIMA_KEY_FILE=<hub root>/vault.key`. The hub must be `ready`, because the database runs only then.

| Command | Runs | Input and output |
| --- | --- | --- |
| `anima-local-hub account status` | `anima-hub operator show` | One JSON line: the name and whether TOTP is set, or `null`. |
| `anima-local-hub account totp-new <name>` | `anima-hub operator totp <name> --new` | One JSON line with a new TOTP secret and its `otpauth://` URI. Nothing is saved. |
| `anima-local-hub account create <name>` | `anima-hub operator create <name>` | Three lines on standard input: the password, the secret, a code. It makes or replaces the operator in one step. |

- A name has 1 to 64 letters, digits, `.`, `_` or `-`, and starts with a letter or a digit.
- `account` runs the hub only when the hub root, each folder down to `programs/current/bin`
  and `anima-hub` are your user's, are not symbolic links, and no other user can change them
  (the folders: no access for group or others). Otherwise it ends with code 1 and names the
  path, and nothing runs. It changes no mode.
- The secrets pass on standard input and standard output only. They are never arguments, and
  `account` writes nothing of them to a log. The hub's own error text is not shown: a refusal
  prints a fixed message with the hub's exit code.
- Exit codes: 0 done; 20 the hub is not `ready`; 2 wrong use (for example a wrong name); another
  code is the hub's own refusal.
