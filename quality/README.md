# quality

Repository checks that Clippy and ESLint do not cover. Run with bun from `quality/`.

| Command | Spec | Checks |
|---|---|---|
| `bun run limits` | CQ-010, REPO-005 | Code lines per file (300) and measured files per folder (12) |
| `bun run baseline -- --base <ref>` | CQ-011 | `baseline.json` has no new path, new signal or larger value than at `<ref>` (default `phase-1`) |
| `bun run names` | CQ-030 | Banned words in file, folder and type names |
| `bun run imports` | REPO-004 | No hub domain uses another domain's `store`; no domain cycle |

## Local gates

`bun run gates` (from the repository root) is the only gate entry point. There is no CI; gates run on the machine that makes the change.
It runs twelve gates as separate processes, in this order, and stops at the first failure (exit code of that gate, 1 if killed by a signal):

1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`
4. `bun install --frozen-lockfile`
5. `bun run lint`
6. `bun run check`
7. `bun run test`
8. `bun run --cwd apps/client build`
9. `bun run quality:limits`
10. `bun run quality:names`
11. `bun run quality:imports`
12. `bun run quality:baseline -- --base <BASE_REF>`

`BASE_REF` is read from the environment and defaults to `phase-1` (a local branch; no remote needed).
A missing executable is a failure, never a skip (CQ-024). The gate list and runner live in `src/gates.ts`.

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
