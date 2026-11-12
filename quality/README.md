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

### Compile cache (R-112, R-101)

When `sccache` is on `PATH` and `RUSTC_WRAPPER` is not set, `bun run gates` sets `RUSTC_WRAPPER` to `bin/compile-cache.ts`
for every gate (only cargo reads it). Without sccache the gates compile with plain rustc and print `compile cache: off`;
the cache only saves time. A `RUSTC_WRAPPER` you set, even empty, is kept. If sccache is installed but its server
cannot start, the compile fails and so does the gate; run `sccache --stop-server`, or set `RUSTC_WRAPPER=` to run without it.

Keep `CARGO_TARGET_DIR=<checkout>/target` and `CARGO_INCREMENTAL=0` (R-101). sccache keys each rustc call on the compiler,
the arguments, the contents of every source file and dependency it reads, the `CARGO_*` variables and the working folder,
and writes its output into the asking checkout's own target folder. `CARGO_TARGET_DIR` is a `CARGO_*` variable that names
the checkout, so `bin/compile-cache.ts` removes it from rustc's environment only (rustc never reads it) and checkouts can
share dependency crates. A workspace crate's key still holds its checkout's path (`CARGO_MANIFEST_DIR`, the working folder),
so checkouts never share their own crates. sccache caches only library crates: never a test binary, a build script,
a proc macro or a failed compile. Clippy's runs on workspace crates are not cached at all, so a cached result cannot hide a
new warning. The wrapper is TypeScript, not `sh`: dash drops variables whose names hold a hyphen (`CARGO_BIN_EXE_anima-hub`).

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
cargo install sccache --locked   # optional: the compile cache
```

`~/.cargo/bin` (or `$CARGO_HOME/bin`) must be on `PATH`: the gates look for `cargo-nextest` and `sccache` there.
