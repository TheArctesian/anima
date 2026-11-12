# quality

Repository checks that Clippy and ESLint do not cover. Run with bun from `quality/`.

| Command | Spec | Checks |
|---|---|---|
| `bun run limits` | CQ-010, REPO-005 | Code lines per file (300) and measured files per folder (12) |
| `bun run baseline -- --base <ref>` | CQ-011 | `baseline.json` has no new path, new signal or larger value than at `<ref>` (default `phase-2`) |
| `bun run names` | CQ-030 | Banned words in file, folder and type names |
| `bun run imports` | REPO-004 | No hub domain uses another domain's `store`; no domain cycle |

## Local gates

`bun run gates` (from the repository root) is the only gate entry point. There is no CI; gates run on the machine that makes the change.
It runs fourteen gates as separate processes, in this order, and stops at the first failure (exit code of that gate, 1 if killed by a signal):

1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo nextest run --workspace` (through `bin/with-postgres.ts --migrated-template`)
4. `cargo test --workspace --doc`
5. `cargo test --workspace --test pinning`
6. `bun install --frozen-lockfile`
7. `bun run lint`
8. `bun run check`
9. `bun run test`
10. `bun run --cwd apps/client build`
11. `bun run quality:limits`
12. `bun run quality:names`
13. `bun run quality:imports`
14. `bun run quality:baseline -- --base <BASE_REF>`

`BASE_REF` is read from the environment and defaults to `phase-2` (a local branch; no remote needed).
A missing executable is a failure, never a skip (CQ-024). The gate list and runner live in `src/gates/gates.ts`.

### Rust tests: gates 3 to 5 (R-112)

Together they run every test `cargo test` runs, no more and no fewer.
Gate 3 runs the libtest tests with [cargo-nextest](https://nexte.st): one process per test, all test binaries at once.
`.config/nextest.toml` sets no retries (a test that passes only on a second try fails the gate) and fail-fast.
nextest does not run doctests, so gate 4 runs them; the workspace has none today, and the gate keeps a new one from going unrun.
`anima-desktop`'s `pinning` target is not libtest (`harness = false`, R-069) and nextest cannot list it, so
`.config/nextest.toml` leaves it out and gate 5 runs it with cargo (it proves the macOS trust hook; elsewhere it prints `skipped`).
Gates 4 and 5 pass `--workspace` so cargo reuses gate 3's build instead of resolving features for one package.
`bun run gates` also sets `CARGO_PROFILE_DEV_BUILD_OVERRIDE_DEBUG=0` unless you set it: left unset, cargo gives a build
script's or proc macro's dependency the normal debuginfo only when the same crate is a normal dependency of that command's
targets, so gates 4 and 5 (fewer targets) would rebuild sqlx, async-graphql, tauri and the workspace crates (about 70 s cold).
Gate 3 fails with the install command when `cargo-nextest` is not on `PATH`.

### Throwaway PostgreSQL (R-009, R-019)

Gate 3 runs through `bin/with-postgres.ts`, and so do the deploy tests in gate 9. It starts a throwaway PostgreSQL 17 on a Unix socket in a temporary folder, sets `DATABASE_URL`, runs the tests and deletes the folder. Set `DATABASE_URL` yourself only for a server that no other worktree uses at the same time (R-019). If PostgreSQL 17 (`initdb`, `pg_ctl`, `psql`) is not installed, the gate fails; database tests never skip.

With `--migrated-template` before the command (gate 3 passes it), `with-postgres` first migrates `template1` with
`crates/hub/migrations`, exactly as sqlx 0.8.6 would:
sqlx creates each `#[sqlx::test]` database with a bare `create database`, a copy of `template1`, and then skips every
migration already recorded in `_sqlx_migrations` with the same checksum. So each test database starts migrated instead of
running all migrations itself. `src/gates/migrations.ts` reads the folder by sqlx's rules (file `<version>_<description>.sql`,
description with `_` as spaces, checksum SHA-384 of the file bytes) and refuses what it cannot apply the same way
(`-- no-transaction`, `.up.sql`/`.down.sql`, invalid UTF-8, a repeated version). Each migration is sent verbatim with `psql -c`
(no psql variables or meta-commands) with sqlx's session settings, inside one transaction with its `_sqlx_migrations` row.
A file that differs from what the tests were compiled with fails every database test with a checksum mismatch; it cannot pass silently.
psql exits after each migration, so no session holds `template1` when the tests run.
With `DATABASE_URL` set, `with-postgres` starts nothing and leaves that server's `template1` alone. Without the flag the
cluster stays as `initdb` made it: the migrations also create the cluster-wide role `anima_runtime`, and the deploy
tests (`deploy/package.json`) install the roles themselves.

Files come from `git ls-files --cached --others --exclude-standard`, minus the globs in `generated.txt` (CQ-012).

## baseline.json (CQ-011)

```json
{ "crates/hub/src/big.rs": { "file-lines": 340 }, "crates/hub/src/devices": { "folder-files": 14 } }
```

Key: repo-relative file path (`file-lines`) or folder path (`folder-files`). Value: the measured number.
A file or folder over its limit passes only when its entry is at least the measured number.
Entries only shrink. Never add one by hand.

## hub-modules.json (REPO-004, hub-domains contract)

`leaves` may be imported by all and import no domain. `wiring` (`main`, `lib`, `schema`) may import anything.
`domains` maps each domain to the domains it may import. Any other top-level module under `crates/hub/src` is `unlisted-module`.
Signals: `forbidden-edge`, `leaf-imports-domain`, `unlisted-module`, `foreign-store` (even on an allowed edge),
`import-cycle`, `allow-list-cycle`. Changing the list is a contract change.

## Not handled by `imports`

Inline `mod { }` blocks, bare relative `use store::x;`, macro-generated `use`, paths in strings,
glob imports (`use crate::checks::*;` then a bare `store::x`), names re-bound by shadowing or `let`,
and re-exports (`pub use` counts as a use at the re-exporting file only).
Aliases bound by `use` (`use crate::checks as c; c::store::x`) are followed within the same file.

## Machine requirements

Linux machines that run `bun run gates` need the webkit2gtk-4.1 development headers (and libsoup-3.0, librsvg2, libdbus-1) because gates 1-5 compile the Tauri shell (`apps/desktop/src-tauri`). macOS needs nothing extra.

Every machine needs PostgreSQL 17 (`initdb`, `pg_ctl` and `psql` in one folder) and cargo-nextest:

```sh
cargo install cargo-nextest --locked
```

`~/.cargo/bin` (or `$CARGO_HOME/bin`) must be on `PATH`: the gates look for `cargo-nextest` there.
