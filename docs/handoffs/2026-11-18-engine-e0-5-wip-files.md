# Engine E0-5: file store and extraction WIP

Date: 2026-11-18 (America/Los_Angeles). Status: preserved for continuation.

Repository: `TheArctesian/anima.ing`. Continuation branch: `engine/e0-5-wip-files`. Local branch at transfer: `worktree-agent-a9002294e50be42b4`. Source commit before this handoff: `c9f530d2a06f283df89ad727ca1a5d90490057ce`.

This is an operational transfer record under PLN-008/PLN-022. It adds no requirement, approval, acceptance or new plan. Read `AGENTS.md`, the latest `origin/main` planning documents and current claims before continuing. Keep existing worktrees intact.

## Source and completed work

Source Claude session: `4619ac0b-a4f1-476d-b0a1-6a3560c0a093` (local JSONL session record). Only a task-state summary is recorded here; raw conversation, credentials and account files are not included.

`3bf91c9a` added the hub data-key foundation. WIP `c9f530d2` preserves the file-store and extraction-proposal implementation, migrations, hub routes, GraphQL operations, CLI/MCP adapters and draft tests. The prior engine session explicitly reported that the builder stopped at its usage limit and that this output did not compile cleanly and was not reviewed.

## Continue from here

The transfer refreshed the lockfile and formatted the two MCP-test calls. Next run targeted hub/CLI compilation, fix the actual diagnostics against K-E4/K-E5, and review upload streaming, encryption, EXIF permission, archive deletion and extraction confirmation. Run the full gates and the required live checks before integrating into `engine/e0-5-places`.

## Validation and limits

The initial `cargo check --locked --offline -p anima-hub -p anima-cli` stopped before compilation because Cargo.lock was stale. The transfer then ran `cargo update --offline --workspace`: the diff adds only the existing `http-body-util` and `tokio-util` dependency edges to anima-cli, with no package/version changes. `cargo metadata --locked --offline --no-deps --format-version 1` now passes. The two calls in `crates/anima-cli/tests/it/mcp/engine_records.rs` received wrapping and an optional trailing comma only; the rest of the non-whitespace source is identical. `cargo fmt --check` now passes. Hub/CLI compilation and full tests were not rerun after this metadata-only repair.

The transfer reran `bun run quality:documents` successfully on this branch. The transfer commits change the handoff, lockfile metadata and test formatting only; no application behaviour was changed or newly accepted. Full release gates, cross-platform/device tests and operator checks are not implied by preservation.

## Resume on another account or machine

```sh
git fetch origin
git worktree add ../resume-engine-e0-5-wip-files origin/engine/e0-5-wip-files
cd ../resume-engine-e0-5-wip-files
git switch -c resume/engine-e0-5-wip-files
```

Read this file and the owning plan there. On the same machine, the existing worktree can be reopened after switching Claude accounts; account sign-in does not require moving or deleting it. Before later merging, update from current main, resolve conflicts in the plan/schema/CLI/claims explicitly, run the required gates, and obtain any approval required for deployment or human acceptance.
