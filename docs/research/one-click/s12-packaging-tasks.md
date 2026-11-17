# S12 task breakdown: packaging

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version 1.0. Date: 2026-11-16. Status: **Built; waits for the operator.** The S1 to S8 planning session started this
item. Each design choice below is the recommendation of the build session; the choices that need
an operator ruling are in Section 5.

Branch: `claude/s12-packaging-3433e4`, cut from `origin/main` at `5c5520c9`.
Sources: [one-click-plan.md](../../one-click-plan.md) Section 4.9 (S12) and Section 8, criterion 1;
`s3-database-tasks.md` Section 7.2 (T11, T12); `s4-local-hub-tasks.md`
Section 7.2 (T9); `s6-first-run-tasks.md` Section 7; `s1-link-tasks.md`; UPD-011, BLD-043 to
BLD-047, BLD-083 to BLD-085, INS-002 and INS-007 of the design spec.

Scope: one built application contains all that "Create a hub on this computer" needs. The release
builder puts the hub's programs and the PostgreSQL tree into the macOS application and into the
Linux AppImage, checks each file, and notarizes the macOS result. An application update replaces
the hub's programs and starts the hub again on the new programs.

Not in scope: the backup before an update, the upgrade of the data folder between PostgreSQL
major versions and the step back to an older release (all S9); a Linux package other than the
AppImage; the run with the operator's Developer ID (Section 6).

## 1. What exists

| Part | State on `main` at `5c5520c9` |
| --- | --- |
| Release builder | `apps/desktop/src-tauri/src/builder/`. The bundle step builds `anima`, puts it in `binaries/`, and gives `bundle.externalBin: ["binaries/anima"]` to `tauri build` and `tauri bundle`. On macOS it checks the application (`codesign --verify --deep --strict`), the two programs in `Contents/MacOS` (`lipo -archs`, `codesign --verify --strict`), notarizes the DMG and staples it and the application. Every step runs fixed programs in a scrubbed environment (BLD-044). |
| Shell | `local_hub/carried.rs` looks for `anima-local-hub` beside the application's executable and for the tree in `Contents/Resources/database` (macOS) or `usr/lib/Anima/database` (Linux). No release has them, so `local_hub_carried` is `false`. |
| `anima-local-hub install` | Copies `anima-local-hub`, `anima-hub`, `anima-database`, `anima-gateway` and `anima-link` from the folder of its own program, and the tree that `--tree` names, to `<hub root>/programs/`. A second `install` is an update. Only the Create flow of the client calls it. |
| PostgreSQL tree | `database-tree/build.sh` builds it from the pinned source. No builder step runs it. |
| `anima-link` | `link/build.sh` builds it for four platforms into `link/bin/`. No builder step runs it. |

## 2. Design

### 2.1 What the bundle carries

| File | Source | macOS | Linux AppImage |
| --- | --- | --- | --- |
| `anima` | `cargo build -p anima-cli` | `Contents/MacOS/anima` | `usr/bin/anima` |
| `anima-local-hub` | `cargo build -p anima-local-hub` | `Contents/MacOS/` | `usr/bin/` |
| `anima-hub` | `cargo build -p anima-hub` | `Contents/MacOS/` | `usr/bin/` |
| `anima-database` | `cargo build -p anima-database` | `Contents/MacOS/` | `usr/bin/` |
| `anima-gateway` | `cargo build -p anima-gateway` | `Contents/MacOS/` | `usr/bin/` |
| `anima-link` | `link/build.sh` (Go) | `Contents/MacOS/` | `usr/bin/` |
| PostgreSQL tree | `database-tree/build.sh` | `Contents/Resources/database/` | `usr/lib/<productName>/database/` |

- The six programs are Tauri external binaries (`bundle.externalBin`), as `anima` is (OD-16). On
  macOS each is universal: the two halves are joined with `lipo`.
- The tree is a Tauri resource (`bundle.resources: {"database": "database"}`).
- The updater file is the archive of the application (macOS) or the AppImage (Linux), so an
  update carries every file (INS-007).

### 2.2 The order of the bundle step

1. The clone is at the job's commit. The step rebuilds git's index from the commit before it
   asks git for changes, so a build cannot hide a changed file (security review, M-1).
2. The signing identity (macOS): the one that the builder application itself was built with.
   The clone's `tauri.conf.json` must name the same one (D-5).
3. The tree: `/bin/sh apps/desktop/src-tauri/database-tree/build.sh` with `SIGN_IDENTITY`. The
   recipe signs each Mach-O file of the tree with the hardened runtime and a secure timestamp.
   This is the innermost code.
4. The link: `go mod verify`, then `/bin/sh link/build.sh`, with a fixed Go environment. The two
   macOS programs are joined with `lipo`.
5. One `cargo build` for the five Rust programs, for each Rust target. Each is put in `binaries/`
   and checked (exactly `x86_64` and `arm64` on macOS, an x86-64 ELF program on Linux).
6. `tauri build --no-bundle`, then `tauri bundle`, with the identity of step 2 in `--config`.
   Tauri signs each external binary, then the application (nested code first).
7. The checks of 2.3.
8. Notarize the DMG, staple the DMG and the application, make the updater archive (as today).

Directly before each use, the two scripts, `postgresql.pin`, `link/go.mod`, `link/go.sum` and
`tauri.conf.json` must be the blobs of the commit. Steps 3 and 4 run before step 5, because
`cargo build` runs the build scripts of the commit (security review, M-2).

### 2.3 The checks on macOS (UPD-011)

For **every Mach-O file** in the application (found by its first bytes, not by its name or folder):

- `lipo -archs` names `x86_64` and `arm64` and no other architecture.
- `codesign --verify --strict` passes with the requirement that the leaf certificate is a
  Developer ID Application certificate of the Team ID that the signing identity names.
- `codesign -d --verbose=4` shows the `runtime` flag and a `Timestamp=` line.
- `codesign -d --entitlements -` prints nothing: no program of the bundle needs an entitlement.

`Contents/MacOS` must hold exactly the application's program and the six programs of 2.1.
`codesign --verify --deep --strict` on the application stays. The notary service then checks the
DMG; a refusal fails the job (BLD-085).

### 2.4 The update path

- `anima-local-hub install` writes `programs/current/digest`: a SHA-256 over the names, the modes
  and the bytes of the copied programs and tree.
- New command `anima-local-hub update --tree <folder>`. When the hub is installed and the digest
  of what the application carries is not the digest of the copy, it does what `install` does:
  the hub stops in order, the programs are replaced, the hub starts. It prints
  `{"updated":true}` or `{"updated":false}`. It never touches the database, the key or the link's state.
- `update` does not replace programs by those of a lower version (D-6). A hub that the operator
  stopped with `stop` gets the new programs and stays stopped.
- The Tauri shell runs `update` once at each start, in the background, when the build carries the
  programs. So the hub gets the new programs at the first start after "Restart to update".

### 2.5 A trial bundle

`apps/desktop/src-tauri/trial-bundle.sh` makes an application with an ad hoc signature for a trial
on the computer that built it. It runs the same commands as the bundle step, with no notary
step. It is never a release.

## 3. Tasks

| Task | What | Done when | Review | State | Commits |
| --- | --- | --- | --- | --- | --- |
| T1 | The builder builds and stages the five Rust programs, `anima-link` and the tree, and gives them to Tauri (2.1, 2.2) | Tests of the planned commands pass; a trial bundle has every file of 2.1 | reviewer, reviewer-security | Done | `f24f4c54`, `aec3c400`, `45af6bb5` |
| T2 | The macOS checks of 2.3 | Tests with the output of real `codesign` runs pass; an ad hoc signed file fails the check | reviewer, reviewer-security | Done; the run with a Developer ID is deferred (6.3) | `f24f4c54`, `aec3c400`, `45af6bb5` |
| T3 | The shell finds the tree on Linux under the product name; the tool check names the tools the tree recipe needs on Linux; the recipe's Linux hardening flags (S3 T11 d) | Tests pass; the recipe passes in a Linux container | reviewer | Done | `228ea35c`, `f24f4c54`, `bae8d42e`, `bce8ccb9` |
| T4 | `anima-local-hub update` and the digest (2.4) | Tests pass: no change when the copy is the same, an update when one byte differs, the data folder is not touched | reviewer, reviewer-security (two rounds) | Done | `b6d6cf14`, `b39df9ef`, `ca714130`, `1cdf8628` |
| T5 | The shell runs `update` at each start (2.4) | A test of the decision passes; the run-through of T7 shows the hub on the new programs | reviewer, reviewer-security | Done; not seen in a running window (6.1) | `9eaf3ee0`, `b90e9fef`, `6f939623` |
| T6 | `trial-bundle.sh` (2.5) | The script makes an application on this Mac | reviewer | Done | `35ff91c5`, `720ae267`, `6f939623` |
| T7 | Run-through on this Mac from a clean state, the update, the sizes | Section 6.1 has the result | — | Done | this commit |
| T8 | Documents: `docs/releases.md`, `docs/setup/local-hub.md`, the state of S3 T11 and S4 T9, the spec delta | The documents agree with the build | — | Done | `73e735d4`, this commit |

Reviews: each finding and its correction is in the commit messages above. The security reviews
of T1, T2 (M-1, M-2, S-1 to S-8) and of T4, T5 (S1 to S5, then R1 to R4) have no open must-fix
finding. What they left open is in Section 8.

## 4. Recorded decisions

| ID | Decision | Reason |
| --- | --- | --- |
| D-1 | The tree is built again from the pinned source at each release. The builder removes the recipe's work folder before it runs the recipe. | S3 T11 condition (a): a tree is not trusted because of its stamp. A full build is about 3 minutes; the gates before it take about 35. |
| D-2 | The builder runs the two scripts with `/bin/sh`, never a `sh` found on `PATH`, and only from the clone at the job's commit. | S3 T11 condition (c). The scripts are part of the commit that the gates checked. |
| D-3 | No program of the bundle has an entitlement, and the builder refuses a Mach-O file that has one. | The hardened runtime with library validation is the strongest setting. PostgreSQL loads only libraries of the tree, which have the same Team ID. The Go and Rust programs load no library of their own. |
| D-4 | The checks of 2.3 apply to every Mach-O file of the application, found by its first bytes. | `codesign --verify --deep` does not check a file below `Contents/Resources` as code (S3 T11 condition b). A rule by content cannot miss a file that a later change adds. |
| D-5 | The signing identity and its Team ID are those of the builder application's own build (the `tauri.conf.json` it was compiled from). The clone must name the same identity, or the bundle step fails. The builder gives this identity to Tauri in `--config` and to the tree recipe. | The commit must not choose the identity that signs it, or the team that the checks require (security review of T1, S-2). The tool check (BLD-083) tests the same identity. A change of the identity therefore needs a new builder application first. |
| D-6 | A digest of the files decides whether the programs changed. A version number decides only one thing: `update` never replaces the hub's programs by those of a lower version. | The copy must be the carried programs, and a digest is true for a trial build also. An older copy of the application that is still on the computer must not put older hub programs on a database that a newer hub changed (security review of T4, S2). An explicit `install` still replaces. |
| D-11 | `link/build.sh` fails when Tailcat's build tags are not found or do not leave out the port mapper. | With an empty Go module cache the script built `anima-link` with no tags (found in the Linux container trial). A link with the port mapper can open a port on the router (acceptance criterion 4). |
| D-9 | `install`, `update`, `remove`, `start` and `stop` hold one lock in the hub root; a second one fails with "another change of the hub runs". | The application has no single-instance rule, so two copies can start an update at one time (security review of T4, S3). |
| D-10 | When the programs are the same, the hub is not running and the operator did not stop it, `update` starts the unit. | A start that failed after the copy was in place must be tried again at the next start of the application (security review of T4, S1). |
| D-7 | The shell starts the update; the client does not. | It is OS work (REPO-003), and it must run when no window loads. The client sees the hub's states through `local_hub_status` as today. |
| D-8 | On Linux the tree is found below `usr/lib/<productName>/`. The product name is `Anima.ing` (NAV-010); a test compares the shell's path with `tauri.conf.json`. | Tauri puts resources there. The path in `local_hub/carried.rs` still had the old name. |

## 5. Choices that need an operator ruling

| ID | Question | Recommendation |
| --- | --- | --- |
| OR-S12-1 | The hub stops for some seconds at the first start after an application update. Sessions of other devices through this hub end. | Accept. The operator selected "Restart to update" (UPD-003). S9 can add a prompt. |
| OR-S12-2 | A step back to an older release (`allowDowngrades`) gives an older application with a newer hub: `update` does not put older hub programs on the database (D-6). The older client then talks to the newer hub. | Accept until S9 gives a rule for the step back of the hub (a backup before each update, then a restore). |
| OR-S12-3 | The tree's source archive is downloaded from `ftp.postgresql.org` at each release and checked against the pin. | Accept. A builder with no network can set `ANIMA_POSTGRESQL_SOURCE`; the builder does not pass it today. |

## 6. Results

### 6.1 Run-through on this Mac (T7, 2026-11-16)

Two trial bundles from commit `720ae267` (`trial-bundle.sh 0.1.0` and `0.1.1`; ad hoc
signature). A copy of the application was put in `~/Applications/S12Trial/`, a folder that did
not exist before. Each step ran the program inside that copy with the arguments and the
environment that the shell gives it (`env -i HOME=…`), and the hub's API with the origin of
the application (`tauri://localhost`).

**The window was not driven.** The operator's own Anima application was running on this Mac
with the same bundle identifier. A second copy would share its saved hub address and its
key-store entries. So the screens of the Create flow and the shell's own call of `update` at
its start are proved by their tests only; Section 6.3 has the check on a second Mac.

| Step | What was seen | Result |
| --- | --- | --- |
| Clean state | No `ing.anima.hub` LaunchAgent, no hub root, nothing on 8443. | Pass |
| The bundle | `Contents/MacOS` holds `anima-desktop` and the six programs; `Contents/Resources/database` holds the tree. `codesign --verify --deep --strict` passes. Each program has `x86_64 arm64` and the `runtime` flag; `anima-desktop` has no entitlement. | Pass |
| `update` with no hub | `{"updated":false}`; no folder is made. | Pass |
| Create: `install --tree <app>/Contents/Resources/database` | 28 s. The unit `ing.anima.hub` runs `programs/current/bin/anima-local-hub`. `ready` with the link `up` 77 s after the start of the install. `programs/current/version` is `0.1.0`; the digest file is there; the hub root is 0700 and `vault.key` 0600. | Pass |
| Processes and listeners | `anima-local-hub run`, `anima-database serve`, `postgres`, `anima-gateway`, `anima-hub`, `anima-link hub`, all from `programs/current`. Listeners: the hub on `127.0.0.1:8443`, the gateway on a loopback port. | Pass |
| Account | `account status`: `null`. `totp-new`, then `create` with a password, the secret and a code: `Operator ada is set, with TOTP.`; status `{"name":"ada","totp":true}`. | Pass |
| Sign-in | `login` with the password and a code gives a token; `me` answers `ada`, `totpEnabled: true`. | Pass |
| Invite | `createLinkInvite` gives an invite (`anima-invite.1.…`, 270 characters) that expires after 15 minutes. | Pass |
| `update` with the same application | `{"updated":false}`; the hub's process is the same. | Pass |
| Update: the copy replaced by 0.1.1, then `update` | `{"updated":true}` in 14 s; `ready` 3 s later, the link `up` 2 s after that. `programs/current/version` is `0.1.1`; the hub has a new process; `programs/previous` is gone. | Pass |
| The data after the update | `vault.key` has the same SHA-256. `account status` shows `ada` with TOTP. Sign-in with the same password and secret works. | Pass |
| A second start of 0.1.1 | `{"updated":false}`. | Pass |
| An older application (0.1.0) | `update`: `{"updated":false,"older":true}`, exit 0, the hub keeps 0.1.1. `install`: refused with "this application (version 0.1.0) is older than the hub's programs (version 0.1.1)", exit 1. | Pass |
| `stop`, `update`, `start` | `stop`: `stopped`, the mark `stopped-by-operator` is there. `update`: the hub stays stopped. `start`: `ready` in 3 s. | Pass |
| Secrets in the logs | The password and the TOTP secret: 0 matches in the five logs. | Pass |
| Clean-up | `remove`; then the hub root, `pg/` and the copy of the application were deleted. No unit, no process, nothing on 8443; `ing.anima.desktop` holds only `builds/`, as before. | Done |

An earlier trial bundle (commit `35ff91c5`) gave the same result for the Create steps.

**Linux, arm64 container (Debian 12, 2026-11-16, commit `228ea35c`).** `trial-bundle.sh` made
`Anima.ing_0.1.0_aarch64.AppImage` (142 MiB). The six programs are in `usr/bin/` and the tree is
in `usr/lib/Anima.ing/database/`. A copy of them in the installed layout ran
`anima-local-hub run`: `ready` in 4 s, the link `up`, `/health` answers, and no process was left
after SIGTERM. Three findings:

- With an empty Go module cache `link/build.sh` built the link with no build tags. Corrected (D-11).
- `libpq.so.5` kept the build folder as its library search path. Corrected in the recipe, with a
  check; the recipe then passed again in the container (22 MB, `initdb` runs).
- The AppImage tool (linuxdeploy) changes each ELF file of the tree and the five Rust programs: it
  adds `$ORIGIN` to the search path. They still run and still pass the recipe's checks. The files
  in the AppImage are therefore not the bytes that were checked before the bundle (Section 8).

systemd was not run, and no x86-64 build was made (6.3).

### 6.2 Sizes (macOS, universal, version 0.1.0 of the run-through)

| Part | Size |
| --- | --- |
| `anima-hub` | 101.4 MB |
| `anima-desktop` (the application itself) | 57.1 MB |
| `anima` | 46.9 MB |
| `anima-link` | 30.3 MB |
| `anima-gateway` | 21.8 MB |
| `anima-database` | 5.2 MB |
| `anima-local-hub` | 4.8 MB |
| PostgreSQL tree | 31.2 MB (`bin` 19.8, `lib` 7.5, `share` 3.9) |
| **The application** | **300 MB** on disk |
| **The DMG** | **113.5 MB** |
| The hub's copy (`programs/`) after install | 207 MB |
| A new database with one account | 50 MB |

Before S12 the application held `anima-desktop` and `anima`: about 104 MB. Each program has two
architectures. Linux, arm64: the AppImage is 142 MiB; `usr/bin` is 143 MB and the tree 25 MB.
`anima-hub` is a third of the application; nothing was done to make it smaller.

### 6.3 Deferred: for the operator or other hardware

None of these blocks the merge.

1. **A release with the Developer ID (T2, S3 T12).** On the Mac builder, with the identity of
   `tauri.conf.json` in the login keychain and the notary profile `anima-notary` (as
   `docs/releases.md`, "Once, by the operator", step 3):
   1. Install a builder application that was built from a commit with S12 (D-5: the builder
      signs with the identity of its own build).
   2. Start a release in the Builds view. Watch the bundle step: the tree recipe, `link/build.sh`,
      `cargo build`, Tauri, then the check of each Mach-O file, then the notary service.
   3. Done when the notary answer is `Accepted` and the job is published. If the check of a
      Mach-O file fails, the step's log has the `codesign` output of that file. Two things are
      not proved without this run: that Tauri gives no program an entitlement when it signs
      with a real identity, and that `codesign -d` prints the lines that the checks read for
      each file of a real release.
   4. `spctl --assess --type execute -vv /Applications/Anima.ing.app` says `accepted` and
      `source=Notarized Developer ID`.
2. **A second Mac, or a new user account on this one.** Install the DMG of step 1. Then:
   1. Start the application: the first run shows "Create a hub on this computer" as available.
   2. Create the hub, the account and an invite in the window (`docs/setup/first-run.md`).
      The first connection of the hub to a LAN device shows the Local Network prompt of macOS.
   3. Publish a second release (a higher version). In the first application select "Restart to
      update". Then `cat ~/Library/Application\ Support/ing.anima.desktop/hub/programs/current/version`
      shows the new version, the application's log has "the hub on this computer got the
      programs of this version", and the sign-in still works.
3. **An x86-64 Linux build.** On the Linux builder install `patchelf`, `bison`, `flex`,
   `binutils` (`readelf`), `make`, `curl` and the zlib headers; the tool check in Settings then
   shows no "tree tools". Run `apps/desktop/src-tauri/trial-bundle.sh 0.1.0`, or a release job.
   Done when the AppImage is made and `./Anima*.AppImage --appimage-extract` shows the six
   programs in `squashfs-root/usr/bin/` and `squashfs-root/usr/lib/Anima.ing/database/bin/postgres`.
4. **A real Linux desktop.** Run the AppImage of step 3, create a hub in the window, sign out
   and in (the systemd user unit starts the hub), run a newer AppImage and check the version
   file as in step 2.3. `kill -9` of `anima-local-hub run`: the unit starts it again (S4 T10).
5. **A builder in the background keeps its job (BLD-005, b361eb5f).** On the Mac builder, with
   an application of 0.2.1 or later:
   1. Run `defaults delete ing.anima.desktop NSAppSleepDisabled`, then quit and open the
      application.
   2. Start a release, then minimise the window and leave it minimised.
   3. Done when the job is published. A job that fails with "builder stopped 3 times" fails
      this check.

### 6.4 The first release with S12: 0.2.0 (2026-11-17)

Step 1 of 6.3 was run. The operator's builder application was replaced by one built from `main` at
408bbbab (ad hoc, then signed by the operator with the Developer ID). The hub was deployed at
408bbbab first, because the hub before it had no `releasePublishing` (operator-changes.md,
2026-11-17 00:27Z).

| Try | Commit | Result |
| --- | --- | --- |
| 1 | 11f62068 | Gates failed: one hub test that counts SQL statements (`api_005_status_uptime_checks_and_ip_cost_a_constant_few_statements…`), with other sessions' builds on the Mac. |
| 2 | 11f62068 | "builder stopped 3 times" during the Rust tests. The builder's reports run on web view timers; the window was in the background. App Nap was then turned off for the application (`NSAppSleepDisabled`) and the window kept visible. |
| 3 | 11f62068 | Gates 20/20. The bundle step failed: the tree recipe took the first word of each `otool -L` line, and the builder's folder is under `Application Support`. Corrected in 43291afc and proved with a work folder whose path has spaces. |
| 4 | 43291afc | Published. Notary answer `Accepted`, stapled. |

Checked on the published DMG (`https://releases.anima.ing/darwin-universal/0.2.0/Anima_0.2.0_universal.dmg`,
114.4 MiB): `stapler validate` passes; `spctl` accepts the DMG and the application as
`Notarized Developer ID`; the application has the seven programs in `Contents/MacOS` and the tree in
`Contents/Resources/database`; `codesign --verify --deep --strict` passes; the carried programs and
the tree have Team ID 7GDLG48PV6, the hardened runtime and a secure timestamp. The feeds of
`darwin-aarch64` and `darwin-x86_64` give 0.2.0. Steps 2 to 4 of 6.3 stay open.

0.2.1 (b361eb5f, 2026-11-17): the first two releases failed with "builder stopped 3 times". The
builder application was older than b361eb5f, and its window was behind other windows. The attempts
stopped at different places (one at test 965 of 5777, two before the tests ran), so the cause
was time and not a report the hub refused. WebKit slows the timers of a covered window as it
does for a minimised one. The third release ran with the window in front and uncovered, and was
published at 06:07Z. `stapler validate` passes on its DMG, `spctl` gives
`Notarized Developer ID`, and both feeds give 0.2.1.

## 7. The seam to S9 (hub update and backup)

- S12 replaces the programs and starts the hub again. It makes no backup first.
- `programs/previous` is removed when the new copy is in place. A hub that does not start on the
  new programs is not put back on the old ones.
- The data folder keeps its PostgreSQL major version. A new major version in the tree gives the
  state `blocked` with `database-version` (S3, S4); the upgrade is S9's.
- OR-S12-2: the step back of the hub to an older release. S12 refuses it in `update`; only an
  explicit `install` does it.

## 8. Open after the reviews

Each row was found by a review of S12 and is not corrected. None was made by S12, except the
last two.

| Item | What | Proposed owner |
| --- | --- | --- |
| The DMG is not bound to the checked application | The checks run on the application folder. The DMG that Tauri made is checked only by the notary service, and its hash is not recorded at the bundle step (security review of T1, S-3). | The builder (BLD); a row of issue #1 |
| The updater archive is not checked after `tar` | A process that outlived a step on macOS could change the application between the check and the archive (S-4). macOS has no reaper for such a process. | The builder; with the step sandbox of issue #1 |
| The AppImage is not checked after `tauri bundle` | The AppImage tool is downloaded by Tauri and changes the carried ELF files (6.1). Nothing compares the result with what was staged. | The builder, with the x86-64 Linux build |
| One `target/` for the gates and the bundle | Build scripts that the gates run can leave outputs that the release build uses. | The builder |
| Ignored files in the clone | A file that the repository's `.gitignore` covers does not show in the clean check; the checkout removes it, but code of the gates step can write it again before the bundle. | The builder |
| A copy of the application that was changed on disk | `update` compares the application with the digest that was written at the last copy. It does not read the hub's copy again, so it does not repair a copy that another program of the same user changed. | Accepted: that user can already change the unit. |
| The window on a clean computer | 6.1: not driven on this Mac. | 6.3, step 2 |
| A builder in the background loses its jobs | The builder reports from timers of the web view. With the window minimised, the hub saw no poll from the idle builder for 7 minutes (2026-11-17 04:32 to 04:39Z), and a job is taken back after 90 s with no report (6.4, try 2). | Corrected: the main window has `backgroundThrottling: "disabled"` (WebKit's inactive scheduling policy, macOS 14 and later). Built in b361eb5f; not proved yet: the check is step 5 of 6.3. Linux: WebKitGTK has no such setting; not seen on the Linux builder. |
