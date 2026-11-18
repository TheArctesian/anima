# Engine E0-6: operational keys WIP

Date: 2026-11-18 (America/Los_Angeles). Status: preserved for continuation.

Repository: `TheArctesian/anima.ing`. Continuation branch: `engine/e0-6-wip-keys`. Local branch at transfer: `worktree-agent-aa87bb367e7a9e14d`. Source commit before this handoff: `e73a18585fdbe6eb6aab959835be98c50b40ddad`.

This is an operational transfer record under PLN-008/PLN-022. It adds no requirement, approval, acceptance or new plan. Read `AGENTS.md`, the latest `origin/main` planning documents and current claims before continuing. Keep existing worktrees intact.

## Source and completed work

Source Claude session: `4619ac0b-a4f1-476d-b0a1-6a3560c0a093` (local JSONL session record). Only a task-state summary is recorded here; raw conversation, credentials and account files are not included.

WIP `e73a1858` preserves operational credential versions/retrieval/revocation in the existing credentials domain, a daemon key-cache draft, protocol/schema updates, and draft tests. The prior engine session explicitly reported that this output did not compile cleanly and was not reviewed. Personal Class A sealing and its security review are not complete.

## Continue from here

Read K-E5 and the E0-6 task table. Finish compilation and review daemon authentication, version activation/revocation, protected local storage, the seven-day disconnected cache rule, and value-free errors/audits. Check CLI coverage against CMD-030; existing `anima key` manages API keys, so confirm the operational-key commands rather than assuming that name proves coverage. Reconcile this branch's HT-026 with current main: the agent-usage session also used HT-026. Complete the retained Class A separation/security review before acceptance. Integrate into `engine/e0-5-places` only after review and gates.

## Validation and limits

`cargo fmt --check` and `cargo check --locked --offline -p anima-hub -p anima-cli` passed in this transfer with Rust 1.100.0. This confirms hub/CLI compilation now; it does not establish daemon compilation, test success, security review or acceptance. Full gates and human checks were not run. A locked worktree registration was retained; no lock was removed.

The transfer reran `bun run quality:documents` successfully on this branch. The handoff commit changes documentation only; no application behaviour was changed or newly accepted. Full release gates, cross-platform/device tests and operator checks are not implied by preservation.

## Resume on another account or machine

```sh
git fetch origin
git worktree add ../resume-engine-e0-6-wip-keys origin/engine/e0-6-wip-keys
cd ../resume-engine-e0-6-wip-keys
git switch -c resume/engine-e0-6-wip-keys
```

Read this file and the owning plan there. On the same machine, the existing worktree can be reopened after switching Claude accounts; account sign-in does not require moving or deleting it. Before later merging, update from current main, resolve conflicts in the plan/schema/CLI/claims explicitly, run the required gates, and obtain any approval required for deployment or human acceptance.
