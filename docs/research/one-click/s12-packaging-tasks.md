# S12 task breakdown: packaging

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version 1.0. Date: 2026-11-16. Status: **In progress.** The S1 to S8 planning session started this
item. Each design choice below is the recommendation of the build session; the choices that need
an operator ruling are in Section 5.

Branch: `claude/s12-packaging-3433e4`, cut from `origin/main` at `5c5520c9`.
Sources: `docs/one-click-plan.md` Section 4.9 (S12) and Section 8, criterion 1 (on branch
`claude/power-health-plan`); `s3-database-tasks.md` Section 7.2 (T11, T12); `s4-local-hub-tasks.md`
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

1. The clone is at the job's commit with a clean tree (as today).
2. One `cargo build` for the five Rust programs, for each Rust target. Each is put in `binaries/`
   and checked (universal on macOS, an x86-64 ELF file on Linux).
3. `/bin/sh link/build.sh`. The two macOS programs are joined with `lipo`; the result has the same check.
4. `/bin/sh apps/desktop/src-tauri/database-tree/build.sh` with `SIGN_IDENTITY` set to
   `bundle.macOS.signingIdentity` of the clone's `tauri.conf.json` (macOS). The recipe signs each
   Mach-O file of the tree with the hardened runtime and a secure timestamp. This is the innermost code.
5. `tauri build --no-bundle`, then `tauri bundle`. Tauri signs each external binary, then the
   application (nested code first).
6. The checks of 2.3.
7. Notarize the DMG, staple the DMG and the application, make the updater archive (as today).

### 2.3 The checks on macOS (UPD-011)

For **every Mach-O file** in the application (found by its first bytes, not by its name or folder):

- `lipo -archs` names `x86_64` and `arm64`.
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
| T1 | The builder builds and stages the five Rust programs, `anima-link` and the tree, and gives them to Tauri (2.1, 2.2) | Tests of the planned commands pass; a trial bundle has every file of 2.1 | reviewer, reviewer-security | Not started | — |
| T2 | The macOS checks of 2.3 | Tests with the output of real `codesign` runs pass; an ad hoc signed file fails the check | reviewer-security | Not started | — |
| T3 | The shell finds the tree on Linux under the product name; the tool check names the tools the tree recipe needs on Linux; the recipe's Linux hardening flags (S3 T11 d) | Tests pass | reviewer | Not started | — |
| T4 | `anima-local-hub update` and the digest (2.4) | Tests pass: no change when the copy is the same, an update when one byte differs, the data folder is not touched | reviewer, reviewer-security | Not started | — |
| T5 | The shell runs `update` at each start (2.4) | A test of the decision passes; the run-through of T7 shows the hub on the new programs | reviewer-security | Not started | — |
| T6 | `trial-bundle.sh` (2.5) | The script makes an application on this Mac | reviewer | Not started | — |
| T7 | Run-through on this Mac from a clean state, the update, the sizes | Section 6.1 has the result | — | Not started | — |
| T8 | Documents: `docs/releases.md`, `docs/setup/local-hub.md`, the state of S3 T11 and S4 T9, the spec delta | The documents agree with the build | reviewer | Not started | — |

## 4. Recorded decisions

| ID | Decision | Reason |
| --- | --- | --- |
| D-1 | The tree is built again from the pinned source at each release. The builder removes the recipe's work folder before it runs the recipe. | S3 T11 condition (a): a tree is not trusted because of its stamp. A full build is about 3 minutes; the gates before it take about 35. |
| D-2 | The builder runs the two scripts with `/bin/sh`, never a `sh` found on `PATH`, and only from the clone at the job's commit. | S3 T11 condition (c). The scripts are part of the commit that the gates checked. |
| D-3 | No program of the bundle has an entitlement, and the builder refuses a Mach-O file that has one. | The hardened runtime with library validation is the strongest setting. PostgreSQL loads only libraries of the tree, which have the same Team ID. The Go and Rust programs load no library of their own. |
| D-4 | The checks of 2.3 apply to every Mach-O file of the application, found by its first bytes. | `codesign --verify --deep` does not check a file below `Contents/Resources` as code (S3 T11 condition b). A rule by content cannot miss a file that a later change adds. |
| D-5 | The Team ID is read from the signing identity in the clone's `tauri.conf.json`. The builder reads no keychain item for it. | One source for the identity: the file that Tauri signs with (BLD-083). |
| D-6 | A digest of the files decides whether the programs changed. A version number decides only one thing: `update` never replaces the hub's programs by those of a lower version. | The copy must be the carried programs, and a digest is true for a trial build also. An older copy of the application that is still on the computer must not put older hub programs on a database that a newer hub changed (security review of T4, S2). An explicit `install` still replaces. |
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

### 6.1 Run-through on this Mac

Not run yet.

### 6.2 Sizes

Not measured yet.

### 6.3 Deferred: for the operator or other hardware

Not written yet.

## 7. The seam to S9 (hub update and backup)

- S12 replaces the programs and starts the hub again. It makes no backup first.
- `programs/previous` is removed when the new copy is in place. A hub that does not start on the
  new programs is not put back on the old ones.
- The data folder keeps its PostgreSQL major version. A new major version in the tree gives the
  state `blocked` with `database-version` (S3, S4); the upgrade is S9's.
- OR-S12-2: the step back of the hub to an older release. S12 refuses it in `update`; only an
  explicit `install` does it.
