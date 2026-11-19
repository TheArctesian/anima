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
The rule is `choose` in `src/gates/tier.ts`; `tests/tier.test.ts` tests it.

| Changed paths | Gates |
| --- | --- |
| Nothing | None |
| Only `docs/` and `*.md` files of the root | `documents` |
| A crate | `fmt`; `clippy` and `rust-test` with `-p` for each touched crate; `pinning` when the desktop shell is touched; the five quality gates |
| `link/` | The six Go gates; the five quality gates |
| A bun package | `install`; `lint`, `check` and `test` with `--filter` for each touched package; `build` when the client is touched; the five quality gates |
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.config/`, `.cargo/` | The whole Rust group |
| `package.json`, `bun.lock`, the lint and TypeScript settings of the root, `design/` | The whole bun group |
| `contracts/` | The whole Rust group and the whole bun group |
| `.github/` | The tests of `@anima/deploy`, which read the workflow files |
| A path that no rule knows | The full list |

What it does not do: it does not test a package that depends on a touched one, and it runs the doctests
only with the whole Rust group. The full list does that. `--group rust,go,bun,quality` and
`--gate name,name` run a part of the list by name; the jobs of the workflow use them.

### The gates on GitHub

`.github/workflows/gates.yml` runs the full list on `ubuntu-24.04` machines of GitHub for each pushed branch
but `main`, and for a pull request to `dev` from a fork. Four jobs run at the same time: `rust-lint` (`fmt`,
`clippy`), `rust-test` (`rust-test`, `doctest`, `pinning`), `go`, and `bun` (the bun gates and the quality
gates). The job `all` passes when each passed. `deploy/test/gates-workflow.test.ts` checks that the jobs
together run each gate one time, that the file holds no secret and no token that can write, that each
action is pinned to a commit, and that a pull request of a fork gets nothing. A different workflow that
needs the gates of a commit calls this one (`workflow_call`, input `ref`).

A Linux machine does not compile the code that is only for macOS (the Keychain store of the daemon, the
LaunchAgent, the macOS parts of the desktop shell), and `pinning` proves the macOS trust hook only on a Mac.

### Numbers at a merge

`bin/numbers.ts --check` fails when a placeholder of a number (`R-NEW-` and a name; `src/numbers.ts` has the series) is in the repository;
`--assign` gives each its number. `bin/merge.ts <branch>` merges a branch into `dev` with them
(`AGENTS.md`, "Numbers and generated files at a merge"). `merge-rule.json` holds the two rules that
wait for the operator (OD-W-NEW-gaterule, OD-W-NEW-automerge); the two are off.

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

## Machine requirements

Linux machines that run `bun run gates` need the webkit2gtk-4.1 development headers (and libsoup-3.0, librsvg2, libdbus-1) because gates 1-5 compile the Tauri shell (`apps/desktop/src-tauri`). macOS needs nothing extra.

Every machine needs PostgreSQL 17 (`initdb` and `pg_ctl`) and cargo-nextest:

```sh
cargo install cargo-nextest --locked
```

`~/.cargo/bin` (or `$CARGO_HOME/bin`) must be on `PATH`: the gates look for `cargo-nextest` there.
