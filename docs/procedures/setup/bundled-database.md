# The database that the application carries (`anima-database`)

When the hub runs inside the desktop application (one-click plan S3), the application starts
its own PostgreSQL. The operator installs nothing and needs no administrator rights. The
program is `anima-database`. The hub does not know it: the hub reads a database URL from a
file, as it does with the Docker installation (DB-002).

This page is for the person who operates such a hub, and for the work that starts the program
(plan item S4). The Docker installation on a VPS does not use any of this.

## 1. The folders

Everything is below one root that only your user can read (mode 0700).

| Platform | Root |
| --- | --- |
| macOS | `~/Library/Application Support/ing.anima.desktop/hub/database` |
| Linux | `$XDG_DATA_HOME/anima/hub/database` (default `~/.local/share/anima/hub/database`) |

| Path | What it is |
| --- | --- |
| `cluster/` | The PostgreSQL data folder. Do not copy it while the database runs, and never change a file in it. |
| `urls/owner.url`, `urls/hub.url` | One connection URL for each login, with its password (mode 0600). The hub's `ANIMA_DATABASE_URL_FILE` names `hub.url`; `anima-hub migrate` is given `owner.url` (R-016). |
| `server.log`, `server.log.1` | The server's log: starts, stops, recovery and refused logins. It holds no statement and no error of a session (LOG-007). |
| `lock` | Held by the one copy that runs the database. |

The Unix socket is not below the root. macOS: `~/Library/Application Support/ing.anima.desktop/pg/`.
Linux: `$XDG_RUNTIME_DIR/anima/`. The server has no network listener (DB-001).

A socket path can have 103 bytes. On macOS this permits a home folder path of 40 bytes or
less. With a longer path the program stops with a message that says so.

## 2. Who can sign in

| Login | How | For |
| --- | --- | --- |
| `anima_hub` | Password in `urls/hub.url` | The hub. It owns nothing. |
| `anima_owner` | Password in `urls/owner.url` | Migrations. It owns every object. |
| `anima_super` | No password. Only the operating-system user who runs the database, through the socket. | `anima-database` itself: the logins, backups, checks. |

The logins and their rights are the ones `deploy/postgres/roles.sql` makes for Docker (K2).
If a URL file is lost, the next start makes new passwords for both logins and writes both files.

## 3. The commands

Each command takes `--tree <folder>`: the PostgreSQL tree (`bin/`, `lib/`, `share/`). Without
`--root` and `--socket-folder` it uses the folders of Section 1.

| Command | What it does | Needs a running `serve` |
| --- | --- | --- |
| `serve` | Brings the database up, prints `ready`, and stops the server on SIGTERM or SIGINT. | — |
| `status` | Prints the PostgreSQL version of the tree and of the cluster, and if a server accepts connections. | No |
| `backup <file>` | Writes a backup of the hub's database to `<file>` (mode 0600). | Yes |
| `restore <file>` | Restores a backup into the hub's database, which must be empty. | Yes |
| `verify` | Checks every table and index for damage. | Yes |

Exit codes:

| Code | Meaning |
| --- | --- |
| 0 | Done. For `serve`: stopped on request. |
| 1 | A fault. The message on standard error says which. |
| 3 | `serve`: the server ended by itself. Start it again; if it repeats, read `server.log`. |
| 4 | `verify` found damage and printed `damaged: <database.schema.name>` for each table or index. |
| 10 | Another copy already runs this database. |
| 11 | The cluster is of another PostgreSQL major version. Nothing was changed. See Section 6. |

## 4. What to back up

Keep the files that `backup` writes. They hold everything the hub knows, so treat a backup as
a secret file. A copy of `cluster/` made while the database runs is **not** a backup: at best
it restores as a database that crashed.

On macOS the program excludes `cluster/` from Time Machine when it makes the cluster. If it
cannot, `serve` says so on standard error and the database still works.

To restore: start `serve` on a new, empty root, then run `restore <file>`. The restore puts
back all of the backup or nothing. It signs in as `anima_owner`, not as the superuser: a
backup file holds statements, and a file that somebody changed must not get more rights than
a migration has. `backup` refuses a folder that other users can write. Then start the hub; `anima-hub migrate` applies any
migration that is newer than the backup.

The schedule of backups and the place they are kept are not part of this program (plan item S9).

## 5. After a crash

- **The application or the program was killed.** The server can keep running by itself. The
  next `serve` stops it and starts its own. Nothing is lost.
- **The server was killed, or the power was cut.** The next `serve` recovers from the
  write-ahead log and says `recovered after an unclean stop`. Each change that the hub was
  told is saved is there. On macOS the server asks the drive to empty its cache at each commit
  (`fsync_writethrough`), so that this also holds after a power cut.
- After a recovery you can run `verify`. Exit code 0 and `sound` mean that no table or index
  is damaged.

## 6. A message about the PostgreSQL version

- *"the database was last run by a newer Anima"*: you installed an older Anima over a newer
  one. This Anima does not touch the database. Install the newer Anima again.
- *"Install the Anima release that upgrades the database"*: the database is from an older
  PostgreSQL major version than this Anima has. This release has no upgrade step. Install the
  release that the message names; it carries both versions and upgrades the database.

A minor PostgreSQL update (17.x to 17.y) needs no step.

## 7. For plan item S4: how to start it

1. One service for each user starts `anima-database serve` and no second copy. Give it a
   cleared environment; it needs no variable.
2. Wait for the line `ready` on its standard output.
3. Run `anima-hub migrate` with `ANIMA_DATABASE_URL_FILE=<root>/urls/owner.url`.
4. Start the hub with `ANIMA_DATABASE_URL_FILE=<root>/urls/hub.url`.
5. To stop: stop the hub first, then send SIGTERM to `anima-database`. Give it 30 seconds or
   more before SIGKILL (`ExitTimeOut`, `TimeoutStopSec`). A SIGTERM during a start is kept and
   acts when the start is done; a start after a kill can take minutes.
6. When `anima-database` ends with a code other than 0, stop the hub and start the group again,
   but not for codes 10 and 11: tell the operator.
7. Run the tree from a folder that an application update does not replace under a running
   server: copy it from the bundle to a folder named for its PostgreSQL version.

## 8. The PostgreSQL tree

`apps/desktop/src-tauri/database-tree/build.sh` builds the tree from the official PostgreSQL
source. `postgresql.pin` beside it holds the version and the SHA-256 of the source archive.
The tree links only system libraries. It holds nine programs, `libpq`, the server's modules
and two standard extensions (`amcheck` for `verify`, and `pg_stat_statements`, which nothing
loads). A migration may use only an extension that is in the official `postgres` Docker image
and in this tree.

```sh
# macOS: both architectures, minimum macOS 14.2. With no SIGN_IDENTITY the signature is ad hoc:
# for a trial on this computer only.
apps/desktop/src-tauri/database-tree/build.sh darwin-universal <work folder> <out folder>
# Linux: the architecture of the computer. Needs bison, flex, patchelf and the zlib headers.
apps/desktop/src-tauri/database-tree/build.sh linux <work folder> <out folder>
```

For a release on macOS each file of the tree must have the Developer ID signature, the
hardened runtime and a secure timestamp (`SIGN_IDENTITY`), and the tree must be signed before
the application bundle is made. The release builder runs this script in its bundle step, from
the pinned source at each release, and checks each file of the tree in the finished
application (`docs/plans/one-click/s12-packaging-tasks.md`, Sections 2.2 and 2.3).
