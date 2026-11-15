# S3 task breakdown: the bundled PostgreSQL

Date: 2026-11-14. Status: **approved by the operator on 2026-11-14 with the recommendation of each
decision D-1 to D-10. The build is in progress; Section 6 has the choices taken on the way.**

Branch: `claude/s3-database`, cut from `origin/main` at `604fec8d` (local `main` is 6 commits
behind it; the S2 branch uses the same base).
Sources: `docs/one-click-plan.md` Section 4.3 and ruling OR-6, and the spike report
`docs/research/one-click/spike-database.md` (both on branch `claude/spike-database-2819c1`;
neither is on `main`). The spike code in `spikes/database/` is reference only. Nothing ships
from `spikes/`.

Scope: the hub in the desktop application on macOS and Linux gets a PostgreSQL that the
application carries. The VPS mode (Docker) does not change.

## 1. Design

### 1.1 The seam

The hub reads one file name from `ANIMA_DATABASE_URL_FILE`, reads a URL from that file and
connects (`config.rs`, `main.rs`; K7 in `contracts/conventions.md`). It applies migrations
only in the one-shot `migrate` command, with the owner role's URL (R-016).

S3 keeps this seam. A new program, `anima-database`, owns the PostgreSQL server and writes two
URL files: one for the owner role and one for the hub role. For the hub it is a database
address like the Docker one.

Effects:

- **The hub does not change.** No hub source, no `main.rs`, no `schema.rs`, no
  `quality/hub-modules.json`, no K7 setting, no SQL. The hub cannot tell whether its
  PostgreSQL is in Docker or in the application. The spike proved that `sqlx` accepts the
  socket URL as it is, with a space in the path.
- **K2 does not change.** The roles are the roles of `deploy/postgres/roles.sql`: an owner
  for migrations, `anima_runtime`, and a hub login that owns nothing.
- **The VPS mode does not change.** `deploy/` is not touched.
- **The spike report said "a supervisor in the hub (a new hub domain)". This design replaces
  that.** A hub domain needs `main.rs`, `hub-modules.json` and a new K7 setting, and the hub
  would know its database is bundled. A program needs none of these.

### 1.2 The program

A new crate `crates/anima-database` builds the program `anima-database`. Its commands:

| Command | What it does |
| --- | --- |
| `serve` | Takes the lock. Stops a server that an earlier copy left. Makes the cluster at the first run. Starts `postgres` as its child. Makes the roles at the first run. Writes the URL files. Reports ready. Watches the child. On SIGTERM: fast shutdown, then exit. If the server ends by itself: exit with a failure, so the service starts it again. |
| `backup <file>` | `pg_dump -Fc` of the `anima` database to a file with mode 0600, as `deploy/backup/backup.sh` does on the VPS. |
| `restore <file>` | Restores a backup into an empty cluster. |
| `verify` | Runs `pg_amcheck` and reports damage. |
| `status` | Prints the PostgreSQL version of the cluster and of the tree, the folder size and if a server runs. No secret. |

Folders, all below one root that the user owns:

```text
<root>/cluster/      the PostgreSQL data folder (initdb makes it, mode 0700)
<root>/urls/         owner.url and hub.url, mode 0600; the hub's ANIMA_DATABASE_URL_FILE names hub.url
<root>/server.log    the server's log; statement logging off
<root>/lock          the flock that permits one copy
<socket folder>/     .s.PGSQL.5432, mode 0700 (see D-5 for where)
<trees>/<version>/   bin/, lib/, share/ of one PostgreSQL version (see 1.4)
```

macOS root: `~/Library/Application Support/ing.anima.desktop/hub/database`. Linux root:
`$XDG_DATA_HOME/anima/hub/database` (default `~/.local/share`). The folder word is `cluster`,
PostgreSQL's own word, because `data` is a banned name (AGENTS.md).

Modules of the crate (operator nouns; pure code apart from the edges):

| Module | Purpose |
| --- | --- |
| `layout` | The paths above from a root, and the socket path rule. Pure. |
| `tree` | One PostgreSQL tree: find its programs, read its version. |
| `cluster` | Make the cluster with `initdb`; read `PG_VERSION`; compare it with the tree. |
| `server` | Start, prove ready, watch and stop the `postgres` child. |
| `roles` | Make the K2 roles and the database; write the URL files. |
| `backup` | `backup`, `restore`, `verify`. |

Fixed choices that cannot change later without a dump (all tested in the spike): UTF-8, the
built-in `C.UTF-8` locale provider, data checksums on, `scram-sha-256`.

### 1.3 Start, stop, two copies, a kill

- **Start.** `postgres` is a direct child, with its settings in the command line: no TCP
  listener, the socket folder, `max_connections`, `shared_buffers`, statement logging off.
  Ready means two facts: the process ID in `postmaster.pid` is the child's, and a login works.
  The spike showed why the first fact is necessary: a new copy took an old server's socket for
  its own.
- **Stop.** SIGINT to the child (fast shutdown), then wait. 0.4 s at most in the spike.
- **Two copies.** `flock` on `<root>/lock` before anything else. The second copy ends with a
  typed error, "another copy runs this database". Without the lock the spike lost a new
  cluster: two `initdb` runs removed each other's folder.
- **Supervisor killed.** The server keeps running (parent process 1). The next `serve` stops
  it with `pg_ctl stop -m fast` and starts its own. On Linux the child also gets
  `PR_SET_PDEATHSIG`, so the server ends with its parent.
- **Server killed, or power lost.** PostgreSQL recovers at the next start (0.7 s in the
  spike; no acknowledged row was lost). `serve` reports "recovered after an unclean stop" in
  its own log, so a support report shows it.

### 1.4 The binaries

- **Built from the official source in our release build**, pinned by version and SHA-256.
  The spike showed that no prebuilt package fits (macOS 15 or glibc 2.36 only, or no
  `pg_dump`, or `fsync` off).
- Configure options: `--without-icu --without-readline --without-openssl --without-llvm
  --disable-nls`. The tree links only system libraries.
- The tree: `postgres`, `initdb`, `pg_ctl`, `pg_dump`, `pg_restore`, `pg_upgrade`,
  `pg_isready`, `pg_amcheck`, `pg_controldata`, `libpq`, the server modules, `amcheck`,
  `pg_stat_statements`, and `share/`. `psql` is not shipped: the program speaks to the server
  through `sqlx`.
- **macOS:** two builds (arm64, x86_64) with deployment target 14.2, joined with `lipo`,
  library paths set relative to the tree. Each Mach-O file is signed with the Developer ID,
  the hardened runtime and a secure timestamp **before** `tauri build`. 32 MB in the bundle.
- **Linux:** one build for `x86_64-unknown-linux-gnu` on the release builder, the same
  computer that builds the application, so the tree needs no newer glibc than the application
  itself. About 21 MB.
- **In the bundle:** the tree and `anima-database` go in as a resource folder, so the tree
  stays together (`postgres` finds `../lib` and `../share`). macOS:
  `Anima.app/Contents/Resources/database/`. Linux AppImage: `usr/lib/Anima/database/`.
- **Where it runs from:** not from the bundle. An AppImage is mounted only while the
  application window runs, and a macOS update replaces the bundle under a running server.
  When the hub service is installed or updated (S4), the tree is copied to
  `<trees>/<postgresql version>/`. This is what MET-019 does with the agent program. A minor
  version gets a new folder; the old one is removed after the new one has started. The tree
  of the last major version stays, for the upgrade.

### 1.5 Versions and upgrade

- **Minor version** (17.x to 17.y): the same cluster, a new tree. No step.
- **Each start** compares `cluster/PG_VERSION` with the tree's major version.
  - Equal: start.
  - Cluster older, and the old tree is present: the upgrade path (below).
  - Cluster newer (the operator installed an older application; `allowDowngrades` is true):
    **refuse to start** with a typed error that names both versions. Never touch the cluster.
- **Major upgrade path:** backup with the old tree, `pg_upgrade` into a new folder
  (`--clone` where the file system has it), `verify`, then swap. The old cluster stays until
  the hub has run on the new one. The spike ran this from 17.12 to 18.7.
- PostgreSQL 17 has community support to November 2029, so the first major change is years
  away. See D-6 for what is built now.

### 1.6 What S4 must supply

S4 (the hub as a background service) is not designed yet. S3 needs these from it:

1. **One service for each user**, which starts `anima-database serve` and no second copy.
2. **Order.** Start: `anima-database serve`, wait for ready, `anima-hub migrate` with
   `owner.url`, then the hub with `hub.url`. Stop: the hub first, then SIGTERM to
   `anima-database`. S1 (`anima-link`) and S2 (`anima-gateway`) are also child programs of
   the Mac hub, so S4 needs one launcher for all of them. S3 does not build that launcher.
3. **A stop time of 30 s or more** before SIGKILL (`ExitTimeOut`, `TimeoutStopSec`).
4. **The copy of the tree** to `<trees>/<version>/` at install and update (1.4), and the
   path of the tree and of the root given to `anima-database`.
5. **Restart** of the whole group when `anima-database` exits with a failure.
6. **A place for the unclean-stop and refusal messages** in the hub status that the
   operator sees (S6).

`crates/agent/src/units/` already plans, renders and installs LaunchAgent and systemd user
units for the agent. S4 can use the same pattern.

### 1.7 Decisions for the operator

| ID | Question | Recommendation |
| --- | --- | --- |
| D-1 | The seam: a program `anima-database` beside the hub (1.1), or a `database` domain inside the hub? | The program. No change to the hub, to K7, to `main.rs` or to `hub-modules.json`, and the hub stays unaware of where its database is. |
| D-2 | Name: crate `crates/anima-database`, program `anima-database`, folder word `cluster`. | Yes. |
| D-3 | The password and where it is kept. (a) Generated 192-bit passwords, SCRAM, kept in the two URL files with mode 0600 below the root. This is DB-002's "file" and the spike tested it. (b) No password: `peer` authentication on the socket, where PostgreSQL asks the kernel which user connects. Nothing to leak, but not tested, and a later TCP listener would need passwords after all. (c) The macOS Keychain: the hub reads a file, so a file would still exist while the hub runs. | (a). The files are in a folder with mode 0700 and never leave it; `backup` and `status` never print them. The same question exists for the hub's key file (`ANIMA_KEY_FILE`) in S4; give both the same answer. |
| D-4 | Durability on macOS: `wal_sync_method=fsync_writethrough`? It makes a commit 3.7 ms in place of 45 microseconds (spike 10.2). Without it a power cut can damage the cluster. | Yes, on macOS. A Mac mini has no battery. The hub's write rate can carry it. Linux keeps its default, which is already safe. |
| D-5 | The socket path limit is 103 bytes. With the socket in `~/Library/Application Support/ing.anima.desktop/pg/` a home folder path of 40 bytes or less fits (`/Users/` and a 31-character name is 38). For a longer home folder: (a) refuse to start with a clear message, or (b) a loopback TCP listener with the same SCRAM passwords. | (a) for the first version. (b) is a second connection mode to build, test and review for a case that may never occur. On Linux the socket is in `$XDG_RUNTIME_DIR/anima/`, which is short. |
| D-6 | The major-version upgrade (1.5). Build the full `pg_upgrade` driver now, or only the parts that protect data? | Now: the version check, the refusal of a newer cluster, the versioned tree folders and `backup`. Later, in the release that brings PostgreSQL 18: the `pg_upgrade` driver, with both trees in that release. The gates have only PostgreSQL 17, so a driver built now has no second version to test against. Nothing is lost by waiting: the release that needs the driver carries it. |
| D-7 | Backup. S3 gives the `backup`, `restore` and `verify` commands. The schedule, the place, how many are kept and the command in the client and CLI are S9. Agree? | Yes. S3 also takes one backup itself before a major upgrade. |
| D-8 | Time Machine copies the cluster while the server runs. Such a copy restores, at best, as a crash recovery. Exclude `<root>/cluster` from Time Machine (`tmutil addexclusion`, no administrator rights) and tell the operator that the backups folder is the thing to keep? | Yes. |
| D-9 | Raw sample retention in the application mode: 30 days needs about 4.9 GB for 10 devices with an agent (spike 10.2). The charts do not read raw samples. | 3 days in the application mode. This is a hub setting (`retention.periods`) and a DB-009 delta, not S3 code. It waits from the last report. |
| D-10 | The oldest Linux that the application supports. The tree is built on the release builder, so it follows the application. | State the builder's distribution in the release notes; no extra build container for the first version. |

## 2. Tasks

Order: the pure parts first, then the server, then the bundle. Each task is one or more
commits with its tests, its gates and its reviews. "Security" in the Review column means
`reviewer-security` also reviews it. Tests that need a server use the PostgreSQL 17 that the
gates already require (`quality/bin/with-postgres.ts` finds it); they never skip (CQ-024).

| # | Task | Content | Done when | Review |
| --- | --- | --- | --- | --- |
| T1 | Layout and socket rule | New crate. `layout`: every path from a root; the socket folder choice and the 103-byte check (D-5). Typed errors. Pure. | Unit tests: paths for macOS and Linux roots; a root with a space; a path of 103 bytes passes and 104 fails with the typed error. | reviewer |
| T2 | Tree and cluster | `tree`: find the programs of a tree, read its version. `cluster`: `initdb` with the fixed choices (1.2); the superuser password through a 0600 file that is removed; read `PG_VERSION`; the three outcomes of 1.5 as a pure decision (start, upgrade, refuse). | Tests: a new cluster has checksums on, UTF-8 and the built-in provider (read back with `pg_controldata` and SQL); the folder modes are 0700; no password file is left. The version decision has a table test, the newer-cluster refusal included. | reviewer, security |
| T3 | Server | `server`: start the child, the two-fact readiness proof (1.3), watch, fast shutdown, the orphan stop, the lock, `PR_SET_PDEATHSIG` on Linux. Time limits are parameters. | Integration tests with a real server: start and stop; a second copy gets the typed error; SIGKILL to the server during commits, then a start finds every acknowledged row; SIGKILL to the supervisor, then a start stops the old server and owns a new one; readiness is not reported for a server that is not the child. | reviewer, security |
| T4 | Roles and URL files | `roles`: the roles and the database of `roles.sql`, random passwords, statement logging off for that session, URL files with mode 0600, written atomically. Errors carry the SQLSTATE only. | Tests: the hub role cannot create a table and cannot connect to `postgres`; the owner can; the role rights equal the ones `deploy/postgres/roles.sql` gives (compared from `pg_roles`); the server log and the program output contain no password (LOG-007). | reviewer, security |
| T5 | The program | `main.rs`: `serve` and `status` over T1 to T4. Exit codes for each typed error. Logs at the edges only. The durability setting of D-4. | An integration test starts the real program: ready, SIGTERM, clean exit; a second start keeps the cluster and the URL files. | reviewer |
| T6 | The hub against the bundled database | No hub source change. One hub integration test (`crates/hub/tests/it`) starts `anima-database serve`, runs `anima-hub migrate` with `owner.url` and starts the hub with `hub.url`: sign-in and one write work. The same test proves the seam: the hub's settings are only the two file names. | The test passes in `bun run gates`. | reviewer, security |
| T7 | Backup, restore, verify | `backup`: the three commands (D-7). The backup file has mode 0600; it holds the hub's data, so it is treated as a secret file in logs. | Tests: backup, a new empty cluster, restore: the same row counts and the same migration records, and the hub starts on it. `verify` passes on a good cluster and reports a damaged index (a test damages one page). | reviewer, security |
| T8 | Version guard | The 1.5 decisions in `serve`: equal starts; a newer cluster is refused and not touched; an older cluster with no upgrade driver is refused with a message that names the release to install (D-6). The Time Machine exclusion (D-8). | Tests with a cluster whose `PG_VERSION` is edited: each outcome, and the cluster's files are byte-equal after a refusal. | reviewer, security |
| T9 | The tree recipe, macOS | The pinned source (version and SHA-256 in one file in the repository), the two builds, `lipo`, relative library paths, the kept files of 1.4, the signature of each Mach-O file. Checks after the build: `lipo -archs` gives both on every program, no link leaves the tree apart from system libraries, the minimum macOS is 14.2. A cache keyed by the pin, so a release does not build PostgreSQL each time. | On this Mac: the recipe gives a tree, the checks pass, and the T3 tests pass against that tree for arm64 and, through Rosetta, for x86_64. | reviewer, security |
| T10 | The tree recipe, Linux | The same pin and options for `x86_64-unknown-linux-gnu`, library path relative to the tree. Check: the tree links only `libc`, `libm`, `libz` and its own `libpq`. | On the Linux builder: the recipe gives a tree and the T3 tests pass against it. **The spike did not test x86_64 Linux.** This task needs the Linux build computer. | reviewer; operator check |
| T11 | In the bundle | The release configuration puts `anima-database` and the tree in the bundle as a folder (1.4). The builder runs T9 or T10 before `tauri build`. UPD-011's checks get `lipo -archs` and `codesign --verify --strict` for `postgres`. | A release build on this Mac has the tree in `Contents/Resources/database/`; `codesign --verify --deep --strict` passes on the application. | reviewer, security |
| T12 | Signature and notary check | **The spike did not test this.** One real build with the Developer ID: every Mach-O file of the tree has the Team ID, the hardened runtime and a timestamp; no file has the library-validation entitlement; the notary service accepts the DMG; on a second Mac (or a new user account) the downloaded application starts the server from the copied tree. | The notary answer is Accepted and the server starts on the second Mac. | Operator (uses the signing identity and the Apple account) |
| T13 | Power cut | **The spike did not test this.** With D-4 set, on a Mac that can lose power (a spare, or a Mac mini on a switched socket): a loop of commits, the power is cut, then a start. Run it 5 times. | Each time the cluster recovers, `verify` passes and no acknowledged row is lost. If no spare computer exists, the operator accepts D-4 on PostgreSQL's documented behaviour and this task is closed as not run. | Operator |
| T14 | Linux desktop check | **The spike ran Linux only in a container.** On a real Linux desktop as a normal user: T3's kill tests, the socket in `$XDG_RUNTIME_DIR`, a stop through a systemd user unit. | The tests pass; a short note with the figures. | Operator, or the Linux-machine session |
| T15 | Documents | `docs/setup`: the folders, what to back up, how to read `status`, what a refusal message means. The spec delta text for DB-001, DB-002 and DB-009 (D-9) as a proposal for OI-2. | The documents are in the branch. | reviewer |

Not in these tasks: the service and its launcher (S4), the first-run screen (S6), the backup
schedule and its commands in the client and CLI (S9), export and import between modes (S10),
the `pg_upgrade` driver (D-6), a TCP listener (D-5).

## 3. Where a ruling is necessary

I stop and report before each of these. None is changed without a ruling.

| Item | Why | When |
| --- | --- | --- |
| Root `Cargo.toml`: a new workspace member `crates/anima-database` | Not a contract, but every branch shares it. S2 adds `crates/anima-gateway` to the same line, so the two branches will conflict there. | T1 |
| `apps/desktop/src-tauri/tauri.release.conf.json` and the release builder (`apps/desktop/src-tauri/src/builder/`) | T11 changes how a release is built. The `builder/` folder has 16 entries and the limit is 12, so the recipe goes in a new sub-folder, not in new files beside the old ones. | T9, T11 |
| The pinned PostgreSQL source (a download from `ftp.postgresql.org` in the release build) | A new outside input to the release. | T9 |
| `crates/hub/tests/it`: one new test file (T6) | A hub test, no hub source. If the test needs a hub change, I stop. | T6 |
| Spec text: DB-001 and DB-002 name Docker; DB-009 (D-9). K2's text gets the rule "a migration may use only a module that is in the official image and in the bundle" (spike 10.3). | Part of OI-2. Text only; K2's rules do not change for the VPS. | Before merge |
| `quality/baseline.json`, `quality/hub-modules.json`, `main.rs`, `schema.rs`, `contracts/`, `deploy/` | No change is planned. If a task needs one, I stop. | — |

CMD-030 (a command and an MCP tool for each new hub function): S3 adds no hub API and no
GraphQL field, so no `anima` command is due. `anima-database` is a program of the service,
not an operator command. The operator's backup command belongs to S9 and goes through the hub.

No VPS, Headscale or cloud change. T12 uses the Apple notary service with the operator's
account; the operator runs or confirms that step.

## 4. Gates and reviews

- Each task: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
  `cargo nextest run` for the changed crates, then `bun run quality:limits`, `quality:names`,
  `quality:imports`. Before I report a task as done: the full `bun run gates` with
  `BASE_REF=origin/main`, and its real output.
- Each task is reviewed by `reviewer`. T2, T3, T4, T6, T7, T8, T9 and T11 are also reviewed
  by `reviewer-security`. I report their findings as they are, then correct them.
- Commits name their requirement IDs (DB-001 to DB-003, DB-007, LOG-007, UPD-011, R-016).
- No push and no merge.

## 5. Handoff note

Nothing is built yet. If this session stops:

- `spikes/database/src/` on the spike branch is the reference for T2 to T5. Its faults are
  known: it trusts any server that answers on the socket (T3's readiness proof corrects
  this), it has `String` errors, and it calls `pg_ctl` for the orphan stop with no check that
  the process is a PostgreSQL server.
- `spikes/database/bundle-macos.sh` is the reference for T9. Its Linux counterpart was a
  one-time script; Section 5.1 of the spike report has its options (`patchelf` for the
  library path; PostgreSQL 17 needs `bison` and `flex`).
- `spikes/database/trials.sh` holds the kill and two-copy trials that T3 turns into tests.
  `upgrade.sh` holds both upgrade ways for the later driver (D-6).
- The decisions D-1 to D-10 wait for the operator. D-1, D-3 and D-6 change the size of the
  work; the others change a setting or a message.
- The S2 branch (`claude/s2-native-vnc`) and S4 share two things with this work: the root
  `Cargo.toml` line, and the launcher that starts the hub's child programs.

## 6. Choices taken while building

Each is the option I recommend, with its reason. None changes a contract.

| Task | Choice | Reason |
| --- | --- | --- |
| T2 | A third URL file, `urls/superuser.url`, for the cluster's superuser `anima_super`. The hub is never given it. | The roles (T4), `verify` (T7, `amcheck` needs a superuser) and a later upgrade need a superuser login after the first run. It is the same kind of file as D-3 approved, in the same 0700 folder. |
| T2 | The read-back of the encoding and the locale provider with SQL is in T3's tests, not T2's. | It needs a running server. T2 proves the checksums with `pg_controldata`, the folder modes, SCRAM in `pg_hba.conf` and that no password file is left. |
| T3 | No `PR_SET_PDEATHSIG` on Linux. | It needs `pre_exec`, which is `unsafe`; the workspace forbids `unsafe` code. The stop of a server left by an earlier copy covers the case on both platforms. |
| T3 | The shutdown signal uses `nix`, the lock uses the standard library's `File::try_lock`. | `nix` is what the hub and the desktop shell already use; no new crate for the lock. |
| T3 | "Ready" is read from `postmaster.pid`: it names the child, it was written after the child started, and its status line is `ready`. Section 1.3 said "and a login works". | This is the signal `pg_ctl` itself uses. It needs no database client in the server module, and it cannot be fooled by another server's socket. The roles step (T4) signs in at once after it, and the tests sign in with the password. |
| T3 | A stale `postmaster.pid` is removed when no server answers on the socket and the process it names is not a `postgres`. | `postgres` refuses to start while that number is a live process of the same user, which can happen after a crash and a restart of the computer. A signal goes only to a process that is PostgreSQL. |
| T3 | A `Server` value that is dropped stops its server. | A panic or an early return must not leave a server running. |
| T2 review | A database folder that is a symbolic link, or that another user owns, is refused; an existing folder is set to 0700; a password file left by a crash is removed before the new one is made; only `initdb`'s own `initdb: error:` line is passed on in an error. | Findings of the two reviews of T2. |
| All | The full gates for a task run in a second, detached worktree at the task's commit (`.claude/worktrees/s3-gates`; not below `/tmp`, where the desktop tests refuse to run), while the next task is written in this one. | A gates run takes about 35 minutes and must test one fixed tree. |
