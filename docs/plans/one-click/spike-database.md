# Spike K-C: a PostgreSQL that the desktop application carries

Status: **report, 2026-11-14**. Spike of `docs/plans/one-click/one-click-plan.md` Section 5, for item S3 and
ruling OR-6. The code is in `spikes/database/`. It is throwaway code: it changes no contract,
no hub code and no live system.

## 1. Verdict

**A bundled PostgreSQL is practical. The stop condition is not met. SQLite needs no new ruling.**

- With no administrator rights, the spike made a data folder below the user's home, started
  PostgreSQL 17.12 on a Unix socket with generated passwords, applied the 62 real hub
  migrations, stopped the server, started it again and found the same data. This passed on
  macOS (arm64, and x86_64 through Rosetta) and on Linux (a container, as a normal user).
- The hub needs no change. It reads a URL from `ANIMA_DATABASE_URL_FILE`, and a URL that names
  a Unix socket works with `sqlx` as it is.
- The cost is small: **32 MB** in the macOS universal bundle (12 MB compressed) and 21 MB on
  Linux, a start of
  **about 20 to 50 ms**, and **about 18 MB** of memory with no client.
- A major-version upgrade of the data folder passed in two ways (17.12 to 18.7).

Two conditions come with the verdict:

1. **Build PostgreSQL from source in our own release build.** No prebuilt package fits
   (Section 4). The build is short and has no third-party library.
2. **Upgrades between major versions are the main open risk** (Section 7). The mechanism
   works. The policy and the release steps are S3 work.

## 2. What the hub needs

| Question | Answer | Source |
| --- | --- | --- |
| PostgreSQL version | 17. The VPS image is `postgres:17.11-bookworm`. The gates use major 17. The spike used 17.12. | `deploy/compose.yaml`, `quality/src/gates/postgres.ts` |
| Extensions | None. No migration has `CREATE EXTENSION`. The migrations use only core functions: range partitions, `jsonb`, PL/pgSQL, and one exclusion constraint with `gist (subnet inet_ops)`, which is in core. | `crates/hub/migrations/` (62 files) |
| Database address | `ANIMA_DATABASE_URL_FILE` names a file. The hub reads the URL from it and calls `PgPool::connect`. The default is `/run/secrets/database_url`. | `crates/hub/src/config.rs`, `crates/hub/src/main.rs` |
| Roles | An owner role applies migrations. The hub signs in with a role that is a member of `anima_runtime` and owns nothing. | `contracts/database.md` (K2 "Roles"), `deploy/postgres/roles.sql` |
| Collation | The gates start PostgreSQL with `--locale=C`, so the hub tests already pass with C ordering. Four queries set `COLLATE "C"` themselves. | `quality/bin/with-postgres.ts` |

The URL that the spike wrote and `sqlx` 0.8.6 accepted is
`postgres://<role>:<password>@localhost:5432/anima?host=<socket folder, percent-encoded>`.
The folder had a space in its name (`Application Support`).

## 3. What the spike does

`spikes/database/src/` is a small supervisor in Rust. For one root folder it does this:

1. Takes an exclusive lock (`flock`) on `<root>/supervisor.lock`.
2. If `<root>/data/postmaster.pid` is there, stops that server with `pg_ctl stop -m fast`.
3. First run only: runs `initdb` with `scram-sha-256`, UTF-8, the built-in `C.UTF-8` locale
   provider and data checksums. The superuser password goes through a file with mode 0600,
   which is then removed.
4. Starts `postgres` as a direct child process. The server has no TCP listener
   (`listen_addresses=''`). The socket is in `<root>/run` with mode 0700.
5. First run only: makes `anima_runtime`, `anima_owner`, `anima_hub` and the database, as
   `roles.sql` does, with 192-bit random passwords. Statement logging is off for that session.
6. Applies `crates/hub/migrations` with `sqlx::migrate!` as the owner role.
7. On SIGTERM, sends SIGINT to the server (a fast shutdown) and waits for it.

The URL files are `<root>/secrets/<role>.url` with mode 0600. No password is in a command
line, in the environment, in the server log or in the spike output (LOG-007).

## 4. Where the binaries come from

| Source | macOS | Linux | Size (one architecture) | Result |
| --- | --- | --- | --- | --- |
| **Our own build from the official source** | Universal. Minimum macOS 14.2 (the application's own minimum). Links only `libSystem` and `libz` from macOS. | Built on Ubuntu 22.04 (Section 5.1). Links only glibc and `libz`. | 32 MB universal on macOS, 21 MB on Linux | **Recommended.** |
| `theseus-rs/postgresql-binaries` 17.12.0 (GitHub releases; the default of the `postgresql_embedded` crate) | Needs **macOS 15.0**. The application supports 14.2. One architecture for each archive. | Needs **glibc 2.36**. Fails on Ubuntu 22.04 (`GLIBC_2.36 not found`). | 261 MB unpacked, 96 MB archive (118 MB is LLVM, 29 MB Python). 71 MB after removal of LLVM, Python and static libraries. | Not usable as it is. |
| `zonky` embedded-postgres-binaries 17.11.0 (Maven) | Universal, minimum macOS 12. | Not examined. | 297 MB unpacked, 62 MB archive. | Has only `postgres`, `initdb` and `pg_ctl`. No `pg_dump`, no `pg_upgrade`: no backup (S9) and no upgrade path. |
| `postgresql_embedded` crate 0.22.0 | Uses the theseus archives (downloaded at first run to `~/.theseus`, or put inside the executable with the `bundled` feature). | Same. | Adds 237 crates to the lock file. | **Not recommended.** See below. |
| Homebrew, a distribution package, Postgres.app | Not relocatable or not ours to ship; a system-wide install needs the operator and often administrator rights. | Same. | — | Against the purpose of the plan. |

The `postgresql_embedded` crate was run against the same binaries
(`spikes/database/embedded-crate/`). It is made for tests, not for a durable database:

- It starts the server with `-F`: **`fsync=off`** (confirmed with `show fsync`). A power cut
  can destroy the data folder.
- It uses the `password` method (clear-text password on the connection), keeps the password in
  a file with mode 0644, and always opens a TCP listener on `localhost`.
- It failed to start when the socket folder had a space in its path
  (`postgres: invalid argument: "Support/…"`). `~/Library/Application Support` has a space.
- It starts the server through `pg_ctl`, so the server is not a child of the application.

Our own supervisor is about 250 lines and has none of these faults. Write it in S3.

### The build

`spikes/database/bundle-macos.sh` is the recipe. It builds 17.12 two times
(`--without-icu --without-readline --without-openssl --without-llvm --disable-nls`, deployment
target 14.2), joins the two halves with `lipo`, sets the path of `libpq` relative to the tree,
keeps 12 programs, `libpq`, the 29 server modules and `share/`, and signs each file. On this
computer the arm64 build took 78 s and the x86_64 build 142 s.

- No ICU: the data folder uses the built-in `C.UTF-8` provider (PostgreSQL 17 and later).
- No OpenSSL: the server has no network listener, so it needs no TLS. SCRAM and random numbers
  use code inside PostgreSQL.
- A data folder that the theseus build made was opened by our build with no fault.

## 5. Measurements

Computer: Apple silicon Mac, macOS 27.0, 10 days up, load average about 20 (other sessions ran
builds at the same time). Linux: Debian 12 in a Docker container on the same computer, arm64,
a user with no root rights. Times are from five runs of `trials.sh timings`; each run uses a
new root. Treat them as an order of size, not as a benchmark.

| Measure | macOS, our universal build | macOS, x86_64 through Rosetta | Linux container (theseus build) | How |
| --- | --- | --- | --- | --- |
| Size in the bundle | **32 MB** (programs 21, libraries 7.2, `share` 3.8); 11.8 MB as `tar.gz`, 8.4 MB as `zstd -19` | in the same 32 MB | see below | `du -sh` on the tree; `tar` piped to `wc -c` |
| First run: `initdb` | 330 to 1400 ms. The first use of a new tree took 3.7 to 7.4 s once. | 860 to 1180 ms (4.8 s once) | 915 to 1113 ms | `Instant` around the `initdb` process |
| Cold start (data folder exists) | **18 to 42 ms** | 60 to 117 ms | 46 to 60 ms | From `spawn` of `postgres` to the first login that succeeds (5 ms poll) |
| Roles + 62 migrations, first run | 50 to 265 ms + 79 to 708 ms | 103 to 192 ms + 216 to 278 ms | 29 to 39 ms + 91 to 162 ms | `Instant` around each step |
| Clean stop | 2 to 6 ms idle; 76 to 214 ms after the migrations (checkpoint) | 11 to 210 ms | 8 to 405 ms | From SIGINT to process exit |
| Start after SIGKILL (recovery) | 715 ms | not measured | 558 ms | Same as cold start |
| Idle memory, no client | **18.3 MB** footprint (26.6 MB sum of RSS), 6 processes | not measured | 19.4 MB PSS (61.4 MB sum of RSS) | 30 s after the migrations. macOS: sum of `phys_footprint` from `footprint`. Linux: sum of `Pss` from `smaps_rollup`. RSS counts shared memory once for each process, so its sum is too large. |
| Idle memory, 10 sessions that each ran one query | 80.6 MB footprint, 16 processes | not measured | 36.1 MB PSS | Same, with ten `psql` sessions held open as the hub role |
| Data folder with the schema, no rows | 51 to 66 MB, of which 16 to 32 MB is write-ahead log | — | — | `du -sh` |

Settings for these numbers: `shared_buffers=32MB`, `max_connections=30`. The hub's pool size
decides the second memory number: each open session costs about 6 MB on macOS.

The Linux column uses the theseus tree because the full set of trials ran on it. Section 5.1
has our own Linux build.

### 5.1 Linux, our own build

The same configure options, built in an `ubuntu:22.04` container (arm64), with the library
path set relative to the tree by `patchelf`. PostgreSQL 17 needs `bison` and `flex` for a
build from the source archive.

- Size: **21 MB** for the same 12 programs, `libpq`, the modules and `share/`; 6.6 MB as `tar.gz`.
- It links only `libc`, `libm` and `libz` from the system and needs **glibc 2.34** or later.
  The theseus tree, trimmed in the same way, is 79 to 85 MB and needs glibc 2.36.
- The lifecycle passed in a Debian 12 container as a user with no root rights: `initdb`
  1235 ms, cold start 40 ms, 62 migrations in 150 ms, the same data after the restart.
- Not done: an x86_64 build (the release target is `x86_64-unknown-linux-gnu`), and a build on
  an older glibc. The package install failed on `debian:bullseye` (its security archive has
  moved), so the spike did not go below Ubuntu 22.04. S3 must set the oldest Linux that the
  application supports and build on it.

## 6. Trials

| Trial | Result |
| --- | --- |
| **Server killed** (SIGKILL to `postgres` while a loop committed rows one at a time) | Tested, macOS and Linux. All acknowledged rows were there after the restart (102 of 102; 119 of 119). The child processes ended by themselves. The stale `postmaster.pid` did not stop the next start. The log shows `automatic recovery in progress`, then `redo done`. The supervisor must watch its child: the first version of the spike did not see the death and waited for ever. |
| **Supervisor killed** (SIGKILL to the spike; the application crash case) | Tested, macOS and Linux. **The server keeps running** with parent process 1 and holds the data folder. A plain second start fails (`lock file "postmaster.pid" already exists`). Worse, the first version of the spike took the old server's socket for its own new server and reported "ready". With the lock and step 2 of Section 3, the next start stops the old server and starts a new one (36 ms). |
| **Two copies at the same time, new root** | Tested. Without a lock, both ran `initdb`, both failed, and `initdb` removed the half-made data folder. With the `flock` lock, one copy runs and the other ends with "another copy already runs this database". |
| **Two copies, existing root** | PostgreSQL itself refuses the second server through `postmaster.pid`. The lock gives a clear message before that. |
| **Major-version upgrade, `pg_upgrade`** | Tested, 17.12 to 18.7, macOS: schema, 62 migration records, roles, passwords and 20 000 marker rows arrived; `sqlx` found nothing to apply; the hub role could sign in. 9.0 s. Needs the old and the new tree on disk together. Used `--clone` (APFS). Both data folders need the same checksum setting; PostgreSQL 18 turns checksums on by default, so the spike turns them on in 17 too. |
| **Major-version upgrade, dump and restore** | Tested, same versions: `pg_dumpall --no-role-passwords` with the old tree (0.8 s, 900 KB), `initdb` and restore with the new tree, role passwords set again from the URL files. Same checks passed. 5.4 s. The old tree is needed only for the dump. |
| **Hub migrations on PostgreSQL 18** | Tested: all 62 apply on a new 18.7 data folder. The hub's queries were not run on 18. |
| **Minor-version change** | Not tested across two minors (17.11 to 17.12). PostgreSQL keeps the data format inside a major version; only the tree changes. A folder made by one 17.12 build opened with another. |
| **Power cut** | Not tested. See risk R3. |
| **Socket path limit** | Found by accident, two times: a Unix socket path can have 103 bytes. `~/Library/Application Support/ing.anima.desktop/hub/run/.s.PGSQL.5432` fits only when the home folder path is 37 bytes or less. |

## 7. Signing and notarization (macOS)

Tested here:

- The universal tree, signed **ad hoc with the hardened runtime**, ran the full lifecycle. So
  PostgreSQL needs no entitlement for its own work (no JIT, no unsigned memory).
- With the hardened runtime, `initdb` refused `libpq` when the files had no common Team ID:
  `mapping process and mapped file (non-platform) have different Team IDs`. An ad hoc
  signature has no Team ID, so the spike added `com.apple.security.cs.disable-library-validation`
  for its own test only.

Thus, for the release:

1. **Sign each Mach-O file of the tree** (12 programs, `libpq.5.dylib`, 29 modules in
   `lib/postgresql/`) with the Developer ID identity, `--options runtime` and `--timestamp`.
   The modules count: the server loads `plpgsql.dylib` and `dict_snowball.dylib` at run time.
   With one Team ID on all files, no entitlement is necessary. Do not ship the
   library-validation entitlement.
2. **Keep the tree together.** `postgres` finds `../lib` and `../share` relative to itself.
   Tauri's `externalBin` puts one renamed file in `Contents/MacOS`, which breaks this. Put the
   tree in the bundle as a folder (`bundle.resources` or `bundle.macOS.files`).
3. **Sign before `tauri build`.** Each signature is inside its file, so it stays valid when
   the bundler copies the tree. `bundle-macos.sh` takes `SIGN_IDENTITY` for this. Do not rely
   on the bundler to sign code below `Resources`.
4. UPD-011 already runs `codesign --verify --deep --strict` and `lipo -archs` on the bundle,
   and BLD-084 sends it to the notary service. Add a `lipo -archs` check for `postgres`.

Not tested here, because it needs the operator's signing identity and Apple account: a real
Developer ID signature on the tree, the `--deep --strict` check of an application that has the
tree, and a notary answer. This is the first task of S3. It is one build.

Other facts:

- The first run of a new program is slow on macOS (the 3.7 to 7.4 s `initdb` in Section 5).
  The first-run screen (S6) must show progress for a few seconds.
- The App Sandbox is not in use. PostgreSQL needs System V shared memory and semaphores, which
  the sandbox blocks. If the application is ever put in the sandbox, this design stops.
- Linux needs no signature. The AppImage carries the tree as plain files.

## 8. Open risks

| ID | Risk | Answer |
| --- | --- | --- |
| R1 | **Major-version upgrade.** A new application version with PostgreSQL 18 finds a version 17 data folder. `pg_upgrade` needs the 17 tree, and the update has just replaced it. An operator can also skip versions, or replace the application by hand. | Keep one major version for a long time: PostgreSQL 17 has community support to November 2029. The release that changes the major version carries **both trees** (about 32 MB more) and runs `pg_upgrade`; keep the old tree for several releases after it. Before the upgrade, write a backup (S9). Keep the old data folder until the hub has run on the new one. Dump and restore is the second way when `pg_upgrade` fails. |
| R2 | **Downgrade.** `tauri.conf.json` has `allowDowngrades: true`. An older application cannot read a data folder that a newer major version has upgraded. | The hub must compare `data/PG_VERSION` with its tree before the start, and refuse with a clear message. Keep the old data folder and the backup for this case. |
| R3 | **Power cut on macOS.** By default PostgreSQL on macOS does not ask the drive to empty its cache. A power cut can lose the last commits or damage the folder. A laptop battery reduces the risk; a Mac mini has none. | Decide in S3: `wal_sync_method=fsync_writethrough`. Measured in Section 10.2: 3.7 ms for each commit in place of 45 microseconds, which the hub's write rate can carry. |
| R4 | **Server left running** after an application crash, or two hubs on one folder. | One owner for the server: the hub service of S4. The lock and the orphan stop of Section 3. The readiness check must prove that the server is its own child (compare the child's process ID with `postmaster.pid`). |
| R5 | **Socket path longer than 103 bytes** (long user name, home folder on another volume). | Check the length before `initdb`. Use a short folder for the socket (Linux: `$XDG_RUNTIME_DIR/anima`). If none fits, use loopback TCP with the same SCRAM passwords. |
| R6 | **We now ship PostgreSQL.** Each PostgreSQL security release (four each year) needs an application release. | A minor update only replaces the tree. Add the PostgreSQL version to the release checklist. Because the server has no network listener, most server faults cannot be reached from outside the computer. |
| R7 | **Ordering differs from the VPS.** The VPS database uses the image's `en_US.utf8`; the bundled one uses `C.UTF-8`. Lists sorted by name in SQL can differ in order for accents and case. | Accepted for the spike: the gates already run with `C`. A move from a VPS hub to an application hub must use dump and restore, not a copy of the folder. |
| R8 | **Memory grows with the pool.** 10 open sessions cost about 80 MB on macOS. | Give the application hub a small pool (for example 5) and an idle timeout. |
| R9 | **Backup tools.** `pg_dump` is in the tree, so S9 can use it. A copy of the data folder while the server runs is not a backup. | S9 uses `pg_dump`. Exclude the data folder from Time Machine or tell the operator; a snapshot of a running folder restores as a crash recovery at best. |
| R11 | **Disk space.** Raw samples for about 10 devices with a daemon need about 4.9 GB in 30 days (Section 10.2). | Shorter raw retention in the application mode (Section 10.4). Show the data folder size in the hub status. |
| R10 | The Linux trials ran in a container, not on a desktop with systemd user units and an AppImage. | Repeat `trials.sh` on a real Linux desktop in S3. |

## 9. Changes for the plan

### S3 (Section 4.3)

Add to "The work is":

1. **Build PostgreSQL from source in the release build** (macOS universal with deployment
   target 14.2; Linux on an old glibc). No prebuilt package and no `postgresql_embedded`.
   Pin the source archive by its SHA-256.
2. **A supervisor in the hub** (a new hub domain, for example `database/`), not in the Tauri
   shell, because the hub must run with the window closed (S4). It does the seven steps of
   Section 3.
3. **A second way to give the hub its URL is not necessary.** The supervisor writes the URL
   files; the hub reads them through `ANIMA_DATABASE_URL_FILE`. The migration step runs with
   the owner URL, the hub serves with the hub URL, as on the VPS (R-016).
4. **Spec delta for DB-001 and DB-002.** Their text names Docker. In the application mode:
   the server has no TCP listener; the socket folder has mode 0700; each role's URL is in a
   file with mode 0600 below the user's folder.
5. **Fixed choices for `initdb`**, because they cannot change later without a dump: UTF-8,
   the built-in `C.UTF-8` provider, data checksums on.
6. **The upgrade policy of R1 and R2** as requirements, with a test that takes a data folder
   of the previous major version through the upgrade.
7. The durability decision of R3, the socket rule of R5 and the pool size of R8.

Proposed requirement prefix: `DBL` (database, local), if free on `main`.

### S4 (Section 4.4)

1. **The hub service owns PostgreSQL.** The service starts the server as its child and stops
   it with a fast shutdown when the service stops. The window, the CLI and the MCP tool are
   clients of the hub; none of them starts a server.
2. **Stop time.** `launchd` and `systemd` send SIGTERM and then SIGKILL. A fast shutdown took
   up to 0.4 s here; give the unit a stop timeout of at least 30 s
   (`ExitTimeOut`, `TimeoutStopSec`). A SIGKILL is safe for the data (Section 6) but costs a
   recovery at the next start.
3. **Where the tree runs from.** The service must run a tree that an application update does
   not replace under it. Either the update stops the service first and starts it after, or
   the Tauri shell copies the hub and the tree to a versioned folder (as MET-019 copies the
   daemon). Decide this in S4; R1 depends on it, because a versioned folder can keep the old
   tree for `pg_upgrade`.
4. **One service for each user.** The lock of Section 3 makes a second copy end with a clear
   message. Two users on one computer have two roots and two sockets, which works.
5. On Linux the supervisor can ask the kernel to end the server when the hub dies
   (`PR_SET_PDEATHSIG`). macOS has no such call, so the orphan stop of Section 3 stays.

## 10. Extensions

Question from the operator after the first report: would a PostgreSQL extension make the
bundled database faster or fit the use better? **Answer: no extension is worth a change to
the hub's SQL. Carry two small standard modules for checks and support. The real gains are in
settings, not in extensions.** `spikes/database/extensions.sh` made the numbers below.

### 10.1 What the hub does with the database

- It already does in its own code what the common extensions offer: day partitions and their
  removal (DB-004, DB-005), minute, hour and day aggregates (DB-008), UUID version 7 made in
  Rust, work queues with `SKIP LOCKED`, and its own scheduler.
- The only text search is `ILIKE` on device names: tens or hundreds of rows.
- The charts read the `minute` and `hour` tables, never the raw samples
  (`fleet/metrics/store/chart.rs`).
- The large data is raw metric samples and heartbeats, kept for 30 days (DB-009). The metrics
  daemon sends one point for each series each 10 seconds (MET-014).

### 10.2 Measured

| Measure | Result |
| --- | --- |
| One raw metric sample (`metrics.samples`, with its primary key) | **132 bytes** (table 74 MB + key 89 MB for 1 296 000 rows) |
| One heartbeat (`checks.heartbeats`) | 130 bytes |
| 150 series (about 10 devices with a daemon) for one day | 163 MB |
| The same for the 30 days of DB-009 | **about 4.9 GB** |
| One commit, default settings on macOS (`open_datasync`) | 45 microseconds |
| One commit with `wal_sync_method=fsync_writethrough` (risk R3) | **3.7 ms** (82 times slower; about 270 commits each second in one session) |
| One commit with `synchronous_commit=off` | 20 microseconds |

Each number is from one run on a loaded computer, with random values. Treat it as an order of size.

So the use case has one database problem that an extension could touch: **disk space for raw
samples on a laptop**. Speed is not a problem at this size.

### 10.3 The candidates

| Extension | What it would give | Verdict |
| --- | --- | --- |
| TimescaleDB (compression) | Smaller raw samples. Its documents claim 90 % or more for data of this kind. **Not measured**: the build needs `cmake`, a system-wide tool that the spike did not install. | **No.** The hub would have to make hypertables in place of its own partitions, so the hub SQL changes (against OR-6 and K2). The compression part has the Timescale License, not an open-source licence; shipping it in our bundle needs a legal check. It needs `shared_preload_libraries`, a universal build and signature of its own, and a custom image on the VPS. It makes R1 worse: a PostgreSQL major upgrade must wait for a TimescaleDB release that supports it. |
| `pg_partman`, `pg_cron` | Partition care and scheduled jobs. | No. The hub does both, with tests. Third-party builds for nothing. |
| `pg_trgm` | Fast `ILIKE` and fuzzy search. | Not now. The device list is too small to gain. It is a standard module and the owner role can create it (tested), so it is cheap to add when a search over logs or many rows comes. |
| `btree_gist` | Exclusion constraints that mix scalar and range columns. | Not needed. The one exclusion constraint (`sites_subnet_no_overlap`) uses `inet_ops` from core. |
| `pgcrypto`, `uuid-ossp` | Encryption and UUIDs in SQL. | No. The hub seals secrets and makes UUIDs in Rust. Our build has no OpenSSL. |
| `pgvector`, PostGIS, `pg_prewarm` | — | No use in this product; the database fits in memory. |
| **`amcheck`** (with the `pg_amcheck` program) | Checks that tables and indexes are not damaged. | **Yes, carry it.** Run it after a major upgrade (R1) and after a recovery from a crash (R3), before the old data folder is removed. Tested: 322 B-tree indexes of the migrated schema verified. About 80 KB for each architecture. Only the superuser can create it; the supervisor does that, not a migration. |
| **`pg_stat_statements`** | Which statements cost the time. | **Carry it, off by default.** It needs `shared_preload_libraries`, so the supervisor turns it on only for a support report. Tested: loads and records. About 80 KB. |
| `auto_explain` | Plans of slow statements in the log. | Optional, same rule as `pg_stat_statements`. Plans can show data values, so LOG-007 applies. |

A rule for S3: **a migration may use only a module that is in the official `postgres` image
and in our bundle.** The standard ("contrib") modules meet this at no cost. A third-party
extension means a custom VPS image and a second build for each platform, so it needs a ruling.

### 10.4 What helps more than an extension

1. **Shorter raw retention in the application mode.** The charts do not read raw samples, and
   `retention.periods` already takes 1 to 400 days for `metrics` and `heartbeats`. With 3 days
   in place of 30, the 4.9 GB above is about 0.5 GB. This is a setting and a spec delta to
   DB-009, not code. It needs an operator ruling: what else reads raw samples older than a few days?
2. **The durability setting (R3).** `fsync_writethrough` costs 3.7 ms for each commit. That is
   acceptable for the hub's write rate (a few commits each second), and it is the safe choice
   for a Mac with no battery. If writes grow, group them in one transaction for each batch
   before `synchronous_commit=off` is considered, because that setting can lose an audit row
   that the hub has already reported as written.
3. **The pool size (R8)** and `shared_buffers=32MB`, as measured in Section 5.

### 10.5 Changes for S3

- Build `amcheck`, `pg_amcheck` and `pg_stat_statements` with the tree (`make -C contrib/<name> install`).
  Size effect: below 0.5 MB in the universal bundle.
- Add the module rule of 10.3 to the K2 text when S3 makes its contract delta.
- Put the raw-retention question of 10.4 to the operator with OI-2.

## 11. What the spike left on this computer

Nothing runs. The spike removed its folder in `~/Library/Application Support`. It installed
no system-wide tool. Downloaded archives and build output are in the session's scratch folder
only. Docker has five pulled images (`debian:bookworm-slim`, `debian:bullseye`, `ubuntu:22.04`,
`ubuntu:20.04`, `rust:1-bookworm`) that the operator can remove.
