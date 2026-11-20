# quality

Repository checks that Clippy and ESLint do not cover. Run with bun from `quality/`.

| Command | Spec | Checks |
|---|---|---|
| `bun run limits` | CQ-010, REPO-005 | Code lines per file (300) and measured files per folder (12) |
| `bun run baseline -- --base <ref>` | CQ-011 | `baseline.json` has no new path, new signal or larger value than at `<ref>` (default `phase-2`) |
| `bun run names` | CQ-030 | Banned words in file, folder and type names |
| `bun run imports` | REPO-004 | No hub domain uses another domain's `store`; no domain cycle |
| `bun run documents` | PLN-016, PLN-017, PLN-019 to PLN-023 | Document links resolve; each plan is in the register, each work item in its index and each mockup in its index (PLN-019); each plan gives its kind and state in its header and the register agrees (PLN-020; `bun run register` writes it); a number of the rulings log names one ruling (PLN-021); a claim is complete and one claim holds a task (PLN-022; `claim-not-complete`, `task-claimed-twice`); a mockup in state Built names the commit it was compared at, and no layout file of its screens has a later commit (PLN-019; `mockup-not-compared`, `mockup-commit-not-found`, `mockup-without-screens`, `screen-changed-after-mockup`); the operator queue agrees with the documents and a task for an agent links to a document whose state permits a build (PLN-023, PLN-007; `queue-out-of-date`, `queue-without-list`, `agent-task-without-link`, `agent-task-in-unapproved-document`); the register has its generated list and each plan has evidence (`register-without-list`, `plan-without-evidence`); a known duplicate still has two owners (`known-duplicate-has-one-owner`); one document defines each requirement identifier (`duplicate-identifiers.txt` is the shrink-only list of the known ones) |

## Local gates

`bun run gates` (from the repository root) is the gate entry point. The gates run on the machine that makes the change; the workflow `gates` runs the same list on GitHub (see "The gates on GitHub").
It runs twenty-one gates as separate processes, in this order, and stops at the first failure (exit code of that gate, 1 if killed by a signal):

1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo nextest run --workspace` (through `bin/with-postgres.ts`)
4. `cargo test --workspace --doc`
5. `cargo test --workspace --test pinning`
6. `bun quality/bin/go-format.ts` (`gofmt -l` on each Go file; any output fails)
7. `go -C link vet ./...`
8. `go -C link tool staticcheck ./...`
9. `go -C link tool revive -config revive.toml -set_exit_status ./...`
10. `go -C link test -race ./...`
11. `go -C link build -o /dev/null ./...`
12. `bun install --frozen-lockfile`
13. `bun run lint`
14. `bun run check`
15. `bun run test`
16. `bun run --cwd apps/client build`
17. `bun run quality:limits`
18. `bun run quality:names`
19. `bun run quality:imports`
20. `bun run quality:documents`
21. `bun run quality:baseline -- --base <BASE_REF>`

Gates 6 to 11 are for the Go module `link/`. Go must be on `PATH` (`brew install go`). `staticcheck` and
`revive` are `tool` lines in `link/go.mod` and run with `go tool`, so they are not installed system-wide.
`link/revive.toml` holds the function limits (60 lines, 4 parameters, complexity 15, nesting 4). revive counts
every line of a function, blanks and comments included, so it is a little stricter than "code lines". Complexity
is cognitive complexity, the same measure that Clippy uses for Rust. The names gate reads `type Name` and
`type ( ... )` groups. The limits
and names gates also measure `.go` files.

`BASE_REF` is read from the environment and defaults to `phase-2` (a local branch; no remote needed).
A missing executable is a failure, never a skip (CQ-024). The gate list and runner live in `src/gates/gates.ts`.

### The fast tier: `bun run gates:fast`

`bun quality/bin/gates.ts --fast` runs a part of the list, chosen by the paths that are different from the
merge base with `origin/dev` (`FAST_BASE` names a different ref): committed, staged, not staged and new files.
The rule is `choose` in `src/gates/tier.ts`; `tests/gates/tier.test.ts` tests it.

| Changed paths | Gates |
| --- | --- |
| Nothing | None |
| Only `docs/`, `*.md` files of the root, `.gitattributes`, `LICENSE` | `documents` |
| A crate | `fmt`; `clippy` and `rust-test` with `-p` for each touched crate; `pinning` when the desktop shell is touched; the five quality gates |
| `link/` | The six Go gates; the five quality gates |
| A bun package | `install`; `lint`, `check` and `test` with `--filter` for each touched package; `build` when the client is touched; the five quality gates |
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.config/`, `.cargo/` | The whole Rust group |
| `package.json`, `bun.lock`, the lint and TypeScript settings of the root, `design/` | The whole bun group |
| `contracts/` | The whole Rust group and the whole bun group |
| `.github/` | The tests of `@anima/deploy`, which read the workflow files |
| A path that no rule knows | The full list |

Measured on 2026-11-19 on the Mac of the operator, at a load of 8 to 12 (the full list takes about 13
minutes on the same Mac when it is quiet: Rust tests 7, bun tests 3.5, Go tests 2):

| Change | Time of `bun run gates:fast` |
| --- | --- |
| One document | Less than 1 s |
| One line in the daemon crate, in a new worktree | 33 s |
| One more line in the same crate | 14 s |
| One line in the bun package `@anima/dns` | 4 s |
| One line in `link/` | 142 s (the Go tests with the race detector are 2 minutes of it) |

What it does not do: it does not test a package that depends on a touched one, and it runs the doctests
only with the whole Rust group. The full list does that. `--group rust,go,bun,quality` and
`--gate name,name` run a part of the list by name; the jobs of the workflow use them.

### The gates on GitHub

`.github/workflows/gates.yml` runs the full list on `ubuntu-24.04` machines of GitHub for each pushed branch
but `main` (R-254). No pull request starts it. The job `base` reads the commit of `dev`; then four
jobs run at the same time, each on the branch as merged with that commit (`gates-merge.sh`): `rust-lint`
(`fmt`, `clippy`), `rust-test` (`rust-test`, `doctest`, `pinning`), `go`, and `bun` (the bun gates and the
quality gates). The job `passed` passes when each passed. A run for `dev`, and a run that a workflow calls
with `ref`, tests the commit as it is. `deploy/test/gates-workflow.test.ts` checks that the jobs
together run each gate one time, that the file holds no secret and no token that can write, that each
action is pinned to a commit, and that no pull request starts it. A different workflow that needs the
gates of a commit calls this one (`workflow_call`, inputs `ref` and `base_ref`).

A Linux machine does not run these, so a Mac must (`bun run gates:fast` there): the tests of
`check-mach-o.sh` and one test of `trial-bundle.sh --marks`, which skip off macOS; the Keychain store of the
daemon and its LaunchAgent; the access-control tests of `link/statedir`; the macOS parts of the desktop
shell, with the trust hook that `pinning` proves. No job on GitHub runs macOS: the trial of the release
workflow measured 82 minutes for the gates on `macos-15`.

Measured on 2026-11-19, the first run in which each job passed (run 44652644005, commit `152773ff`): 31.5
minutes from the start of `base` to the end of `passed`. `go` 2 minutes, `bun` with the quality gates 6
minutes, `rust-lint` 12 minutes, `rust-test` 31 minutes (the Rust tests on 4 processors are the longest
part). An earlier run with a full build cache had `rust-lint` at 2 minutes. Five runs before it failed:
two for the set-up of the machine (a wrapper script that `sh` of Ubuntu broke; PostgreSQL 17 for the tests
of `deploy`), one for three tests of the daemon that run only on Linux and were stale, and two because
`dev` had moved and the branch did not merge with it.

### The merge into `dev`: the merge train (R-252)

A branch does not merge alone. `quality/bin/train.ts` is the conductor; the workflow `merge-to-dev` runs one
step of it at each start, and `bun quality/bin/train.ts --plan` prints, in any worktree and with no change,
each train with its state and the branches of the next one.

- **The batch.** Each branch on origin whose tip passed its own run of `gates` (a push, tested as merged with
  the `dev` of that moment) and that is not in `dev`; the oldest pass first, then the name; at most 12.
- **The train.** The conductor merges the batch in that order on the `dev` of the moment, each branch with
  the steps of `bin/merge.ts` (the generated files written again, the numbers, the merge commit, the document
  gates on the commit). A branch with a conflict, or whose document gates fail, is left out with a line that
  says why. The result is pushed as the branch `train/<commit of dev>-<time>`, and the gates are started for
  it one time, on the commit as it is.
- **The landing.** When those gates pass and `dev` is still the commit that the train stands on, `dev` moves
  to the commit of the train in one push that is not forced. The tree that was tested is the tree of `dev`,
  so no run of the gates follows for `dev`. If `dev` moved (a claim, a merge by hand), the gates of the
  train do not count: its run is cancelled and the next train holds the same branches on the new `dev`.
- **A move of `dev` in records only.** Off today (OD-W-NEW-trainrecords). With `"carryRecords": true` in
  `quality/merge-rule.json`, a train is not given up when `dev` moved only in `docs/claims/`, `docs/ideas/`
  or the two generated files: the conductor makes it again on the new `dev`, proves that each other file is
  the one that the gates tested, and lands it. Not covered by a test: the calls to git and GitHub of this step.
- **A failure.** The next train holds the first half of the failed one, in the same order. A branch that
  fails alone is refused: a note on its tip (`refs/notes/train`) keeps it out until the branch moves or its
  own gates pass again (`gh run rerun`). A cancelled run is no verdict: the train is built again.
- **What no longer happens.** A branch does not run again because `dev` moved. With n branches that passed,
  the old workflow needed about n runs after each merge, n squared in all; the train needs one run for the
  batch, and about log2(n) more when one branch of it fails.

A push of a commit to a branch stops the older run of that branch (`concurrency` in `gates.yml`). The pure
rules are `src/merge/train.ts`, the git work is `src/merge/stack.ts`, the end of one merge is
`src/merge/finish.ts` and `src/merge/commit.ts`: the merge commit comes first and the document gates run on
it, because the mockup check counts the layout files of a branch as changed while its merge is not committed.

What works from where. GitHub takes `merge-to-dev.yml` from the default branch, `main`; the checkout is
`dev`, so the conductor and `merge-to-dev.sh` are those of `dev`. With the old file on `main` the train works,
but the workflow starts only when the gates of a pushed branch pass: a train lands, or is split, at the next
such start, so the last train of a quiet hour waits. The new file also starts at the end of the gates of a
train and each 20 minutes. GitHub has a merge queue for this work only for a repository of an organization;
this repository belongs to a personal account.

Seen on GitHub on 2026-11-20, after this was in `dev`: the conductor made three trains (the third with nine
branches and three left out for a conflict in a document), and gave up the first two because pushes of ideas
moved `dev`; no train landed yet when this was written. The tests cover the rules and the
merges of a train in a real repository; the calls to GitHub are not covered.

The limit of GitHub (https://docs.github.com/en/actions/reference/limits, read 2026-11-20): an account
runs at most 20 jobs of standard runners at the same time on the plan Free, 40 on Pro, 60 on Team. One run
of `gates` has four jobs at the same time, so 5 runs fill the limit of Free and 10 that of Pro; each run
after them waits. A concurrency group keeps one run and one that waits.

### Settings of GitHub

No session of the coding agents makes these; the orchestrating session or the operator does. `OWNER/REPO`
is this repository.

```sh
# 1. Actions can run, and a workflow can ask for a token that writes (merge-to-dev asks for
#    contents: write in its own file; the default stays read).
gh api -X PUT repos/OWNER/REPO/actions/permissions -F enabled=true -f allowed_actions=all
gh api -X PUT repos/OWNER/REPO/actions/permissions/workflow -f default_workflow_permissions=read -F can_approve_pull_request_reviews=false
# 2. A pull request of a fork needs an approval before a workflow runs (it is so today; make sure).
gh api -X PUT repos/OWNER/REPO/actions/permissions/fork-pr-contributor-approval -f approval_policy=all_external_contributors
# 3. Read the rules of dev and main. If a rule of `dev` refuses a push of GitHub Actions, the
#    workflow merge-to-dev cannot push: permit it for `dev` only. Never for `main`.
gh api repos/OWNER/REPO/rulesets
gh api repos/OWNER/REPO/branches/dev/protection
gh api repos/OWNER/REPO/branches/main/protection
# 4. Optional, when dev has a rule: name the check of the gates as required for `dev`.
#    The name of the check is `passed`.
```

The file `merge-to-dev.yml` must be on `main` before it runs: that is a merge of `dev` into `main`, a step
of the operator (R-198).

### Numbers at a merge

`bin/numbers.ts --check` fails when a placeholder of a number (`R-NEW-` and a name; `src/merge/numbers.ts` has the series) is in the repository;
`--assign` gives each its number. `bin/merge.ts <branch>` merges a branch into `dev` with them
(`AGENTS.md`, "Numbers and generated files at a merge"). `merge-rule.json` holds the two rules
of R-254 and R-252; the two are on.

### Rust tests: gates 3 to 5 (R-112)

Together they run every test `cargo test` runs, no more and no fewer.
Gate 3 runs the libtest tests with [cargo-nextest](https://nexte.st): one process per test, all test binaries at once.
`.config/nextest.toml` sets no retries (a test that passes only on a second try fails the gate) and fail-fast.
nextest does not run doctests, so gate 4 runs them; the workspace has none today, and the gate keeps a new one from going unrun.
`anima-app`'s `pinning` target is not libtest (`harness = false`, R-069) and nextest cannot list it, so
`.config/nextest.toml` leaves it out and gate 5 runs it with cargo (it proves the macOS trust hook; elsewhere it prints `skipped`).
Gates 4 and 5 pass `--workspace` so cargo reuses gate 3's build instead of resolving features for one package.
The root `Cargo.toml` sets `[profile.dev.build-override] debug = 0`: left at cargo's default, a build script's or proc
macro's dependency gets the normal debuginfo only when the same crate is a normal dependency of that command's targets,
so gates 4 and 5 (fewer targets) would rebuild sqlx, async-graphql, tauri and the workspace crates (about 70 s cold).
Gate 3 fails with the install command when `cargo-nextest` is not on `PATH`.

### Throwaway PostgreSQL (R-009, R-019)

Gate 3 runs through `bin/with-postgres.ts`, and so do the deploy tests in gate 9. It starts a throwaway PostgreSQL 17 on a Unix socket in a temporary folder, sets `DATABASE_URL`, runs the tests and deletes the folder. Set `DATABASE_URL` yourself only for a server that no other worktree uses at the same time (R-019). If PostgreSQL 17 is not installed, the gate fails; database tests never skip.

Files come from `git ls-files --cached --others --exclude-standard`, minus the globs in `generated.txt` (CQ-012).

## baseline.json (CQ-011)

```json
{ "crates/hub/src/big.rs": { "file-lines": 340 }, "crates/hub/src/devices": { "folder-files": 14 } }
```

Key: repo-relative file path (`file-lines`) or folder path (`folder-files`). Value: the measured number.
A file or folder over its limit passes only when its entry is at least the measured number.
Entries only shrink. Never add one by hand.

## hub-modules.json (REPO-004, hub-domains contract)

`groups` names the parent-noun folders that only group modules (`fleet`, REPO-E-001, OD-E5): a module inside a group
counts as a top-level module, so `crates/hub/src/fleet/devices/store.rs` is in the domain `devices`, and
`crate::fleet::devices::store` is that domain's store. A group may not share a name with a module.
`leaves` may be imported by all and import no domain. `wiring` (`main`, `lib`, `schema`) may import anything.
`domains` maps each domain to the domains it may import. Any other top-level module under `crates/hub/src` or inside a group is `unlisted-module`.
Signals: `forbidden-edge`, `leaf-imports-domain`, `unlisted-module`, `foreign-store` (even on an allowed edge),
`import-cycle`, `allow-list-cycle`. Changing the list is a contract change.

## Not handled by `imports`

Inline `mod { }` blocks, bare relative `use store::x;`, macro-generated `use`, paths in strings,
glob imports (`use crate::checks::*;` then a bare `store::x`), names re-bound by shadowing or `let`,
and re-exports (`pub use` counts as a use at the re-exporting file only).
Aliases bound by `use` (`use crate::checks as c; c::store::x`) are followed within the same file.

## The build cache

`.cargo/config.toml` gives each cargo command of each worktree the wrapper `quality/bin/rustc-cache.sh`:
sccache when it is on `PATH`, the compiler alone when it is not. It also sets the size of the cache to
40 GiB; the default of 10 GiB was full. The gates add `CARGO_INCREMENTAL=0` (`src/gates/cache.ts`); a
cargo command of a session keeps incremental builds for the crates of the workspace, and sccache then
caches only the dependencies. `RUSTC_WRAPPER=` (empty) runs a command without the cache.

Measured on 2026-11-19 on the Mac of the operator (18 cores), while other sessions built (the load was
12 to 29, so each time is too high and only the comparison counts). The command was
`cargo check -p anima-hub -p anima-daemon -p anima-cli --all-targets`, 624 Rust compile steps, with a
cache of its own so that other sessions did not change the counts.

| Case | Time | What it shows |
| --- | --- | --- |
| New worktree, no cache | 170 s (114 s at a load of 12) | The start of today for a cargo command of a session |
| New worktree, sccache, a cache that a different worktree filled | 122 s to 140 s; 438 steps from the cache, 186 compiled | The dependencies come from the cache; the crates of the workspace and what depends on their path do not |
| The same with `SCCACHE_BASEDIRS` | The same 438 and 186 | That setting does not help Rust; it is not used |
| The same folder again, cache full for that path | 66 s; 624 steps from the cache | What a second run in one worktree gets after `cargo clean` |
| One `CARGO_TARGET_DIR` for two worktrees, one after the other | 127 s, then 1 s | The fastest start, and the reason below not to use it |
| The same, two worktrees at the same time | Each build waited for the lock | Two sessions do not build at the same time |
| The same, a second worktree with a fault in a source file that is older than the build | `cargo check` passed in 0.3 s; with its own `target/` it fails | cargo compares times, not contents: a session can pass with the build of a different branch |
| One changed line in the hub, incremental builds on | 22 s | The work loop of a session |
| One changed line in the hub, sccache with incremental builds off | 71 s | Why a session keeps incremental builds on |

Thus: sccache for each cargo command, incremental builds on outside the gates, one `target/` for each
worktree, and no shared `CARGO_TARGET_DIR`. When two sessions build at the same time, each compiles in its
own `target/`; they share only the cache, which serves the two with no lock that a build waits for. If
the two compile the same dependency in the same minute, each compiles it one time and the second write
replaces the first with the same content.

## Machine requirements

Linux machines that run `bun run gates` need the webkit2gtk-4.1 development headers (and libsoup-3.0, librsvg2, libdbus-1) because gates 1-5 compile the Tauri shell (`apps/desktop/src-tauri`). macOS needs nothing extra.

Every machine needs PostgreSQL 17 (`initdb` and `pg_ctl`) and cargo-nextest:

```sh
cargo install cargo-nextest --locked
```

`~/.cargo/bin` (or `$CARGO_HOME/bin`) must be on `PATH`: the gates look for `cargo-nextest` there.
