# Engine E0: contracts and integration

Date: 2026-11-18 (America/Los_Angeles). Status: preserved for continuation.

Repository: `TheArctesian/anima.ing`. Continuation branch: `engine/e0-5-places`. Local branch at transfer: `engine/e0-5-places`. Source commit before this handoff: `51e6874449583afc43e56d5aab107d24135f8628`.

This is an operational transfer record under PLN-008/PLN-022. It adds no requirement, approval, acceptance or new plan. Read `AGENTS.md`, the latest `origin/main` planning documents and current claims before continuing. Keep existing worktrees intact.

## Source and completed work

Source Claude session: `4619ac0b-a4f1-476d-b0a1-6a3560c0a093` (local JSONL session record). Only a task-state summary is recorded here; raw conversation, credentials and account files are not included.

Contracts are recorded in `contracts/extensions/shared-records.md` (K-E4), `contracts/extensions/vault.md` (K-E5), and `contracts/extensions/reminders.md` (K-E6). The OwnTracks deployment files are in `deploy/owntracks/`. The task rows name `0ce1e09b`, `41add3e3`, `b14a7fa9` and `965d77d6`. The branch is an integration starting point; it does not contain the three unfinished builder outputs.

## Continue from here

Read `docs/plans/extensions-plan.md`, its E0-5/E0-6/E0-7 task tables, the three contracts, and current claims on `origin/main`. Resume the three builder branches below, review each against its contract and finish its checks before integrating it here. Then build location ingestion and transfer context, Class A sealed records, and the reminder engine in the plan order. CLI and MCP coverage follow CMD-030. Live checks and the vault security review remain open. Do not infer acceptance from a pushed WIP commit.

## Validation and limits

`engine/e0-5-wip-places` starts at `406e6d82`; `engine/e0-5-wip-files` starts at `c9f530d2`; `engine/e0-6-wip-keys` starts at `e73a1858`. OD-E16, OD-E17, OD-E19 and OD-E20 remain open; consult their owning plan rather than treating this note as a ruling.

The transfer reran `bun run quality:documents` successfully on this branch. The handoff commit changes documentation only; no application behaviour was changed or newly accepted. Full release gates, cross-platform/device tests and operator checks are not implied by preservation.

## Resume on another account or machine

```sh
git fetch origin
git worktree add ../resume-engine-e0-5-places origin/engine/e0-5-places
cd ../resume-engine-e0-5-places
git switch -c resume/engine-e0-5-places
```

Read this file and the owning plan there. On the same machine, the existing worktree can be reopened after switching Claude accounts; account sign-in does not require moving or deleting it. Before later merging, update from current main, resolve conflicts in the plan/schema/CLI/claims explicitly, run the required gates, and obtain any approval required for deployment or human acceptance.
