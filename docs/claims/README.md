# Claims

| Item | Value |
| --- | --- |
| Document | How a session says that it builds a task now, so that no second session takes the task (PLN-022) |
| Version | 1.0 |
| Date | 2026-11-17 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved (operator, 2026-11-17: a light commit to the documents on `main` when a session starts the work) |

Claims are on the branch `dev`, the branch of the daily work (R-197, AGENTS.md); where a sentence below says `main`, read `dev`. A claim is one small file in this folder. It says which machine and which coding agent builds one task now. The files in this folder on `origin/dev` are the list of the work that is taken. There is no second list.

A claim is not a state. The task table of the plan holds the state of the task (PLN-006). A claim only says who has the task.

## Rules (PLN-022)

- Before a session takes a task, it reads this folder on `origin/dev`. If a claim holds the task, the session does not take it.
- To take a task, the session makes the claim in a small worktree of its own, so that the commit never carries other changes: `git fetch origin`; `git worktree add --detach ../claim-<task> origin/dev` (for example `../claim-e0-5`); in it, write one file `docs/claims/<name>.md`; `bun run quality:documents`; commit; `git push origin HEAD:dev`; and, when the push has succeeded, `git worktree remove ../claim-<task>`. If that folder already exists, a session of this computer has or had this claim in progress. Do not use the folder. Look at it and ask the operator. Then the session starts its branch from `origin/dev` (`git switch -c <branch> origin/dev`), or merges `origin/dev` into a branch that exists, so that the branch holds the claim and the merge of the work can delete it. The claim commit holds the claim file. If the task is new, it also holds the row of the task in its plan, with the register and the queue that `bun run quality:register` writes. It needs no other gate, because it changes no code.
- If `main` moved and the push is refused, the session runs `git fetch origin` and `git reset --hard origin/dev` in the same small worktree, and reads the claims again. The reset removes the claim commit. If the task is free, the session writes the claim file again, commits, and pushes again. Then it runs `git worktree remove ../claim-<task>`. If the task is not free, the session gives it up and runs `git worktree remove ../claim-<task>`.
- The machine is not planned. The session that starts the work writes the name of the machine that it runs on.
- The commit that merges the work into `main` deletes the claim and sets the state of the task (PLN-008).
- A session that stops and will not continue deletes its claim on `main` with the same steps (a small worktree `../claim-<task>`, a commit that deletes the file, a push), writes in the task row what is built, and pushes its branch.
- A claim is old when its branch has no new commit for 3 days. Do not take the task of an old claim without a check: look at the branch, ask Anima if the machine is connected (`anima device list`), or ask the operator. Then delete the old claim in the commit that makes the new one.
- The gate fails when a claim does not give each field, when its plan does not exist, or when two claims hold the same task.

## Format

The name of the file is the task in lower case, for example `e0-5.md` or `one-click-s4-t9.md`. The file starts with these six lines between two `---` lines. Text below them is optional.

```text
---
task: E0-5
plan: docs/plans/extensions-plan.md
machine: the name of the machine in Anima, or its host name if Anima does not know it
agent: Claude Code, Codex, or the name of a different coding agent
branch: engine/e0-5-places
started: 2026-11-17
---
```

- `task` is the identifier in the task table or in the index of the work items. The gate compares it without case, and the plan must name it. For a correction of a defect that has no task row, `task` is the identifier of the defect or of the requirement, and `plan` is the document that names it.
- `plan` is the path of the plan from the root of the repository.
- `branch` is the branch that will hold the work. Push it early, so that a different session can see that the claim is not old.

## Claims now

The files in this folder, other than this one. This document keeps no list of them.
