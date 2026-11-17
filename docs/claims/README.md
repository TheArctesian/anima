# Claims

| Item | Value |
| --- | --- |
| Document | How a session says that it builds a task now, so that no second session takes the task (PLN-022) |
| Version | 1.0 |
| Date | 2026-11-17 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved (operator, 2026-11-17: a light commit to the documents on `main` when a session starts the work) |

A claim is one small file in this folder. It says which machine and which coding agent builds one task now. The files in this folder on `origin/main` are the list of the work that is taken. There is no second list.

A claim is not a state. The task table of the plan holds the state of the task (PLN-006). A claim only says who has the task.

## Rules (PLN-022)

- Before a session takes a task, it reads this folder on `origin/main`. If a claim holds the task, the session does not take it.
- To take a task, the session writes one file `docs/claims/<name>.md`, runs `bun run quality:documents`, commits that file only, and pushes the commit to `main`. Then it starts the work. This commit needs no other gate, because it changes no code.
- If `main` moved and the push is refused, the session gets `main` again, reads the claims again, and tries again only if the task is free.
- The machine is not planned. The session that starts the work writes the name of the machine that it runs on.
- The commit that merges the work into `main` deletes the claim and sets the state of the task (PLN-008).
- A session that stops and will not continue deletes its claim and writes in the task row what is built.
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

- `task` is the identifier in the task table or in the index of the work items.
- `plan` is the path of the plan from the root of the repository.
- `branch` is the branch that will hold the work. Push it early, so that a different session can see that the claim is not old.

## Claims now

The files in this folder, other than this one. None on 2026-11-17.
