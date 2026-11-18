# Mobile M3-2 merge candidate

Date: 2026-11-18 (America/Los_Angeles).
Branch: `handoff/2026-11-18-mobile-merge`.
Original base: `2bcdca0dc6224e77ede7c55c67f0fa5ebfeaeda8`.

## State and continuation

The detached merge-m3-2 checkout is preserved at this base. It contains the merge of mobile/m1-shell for Paste invite and Joining screens. This is a candidate snapshot; this checkpoint does not establish that all 21 gates passed. Read docs/plans/mobile-plan.md and the mobile/m1-shell handoff. Inspect the previous gate output if it still exists, then run gates on a current-main integration worktree before considering a merge. Source sessions: 6ad73a45-1a24-4908-af6c-bc41b73e26dc and 2838f139-2835-4c4e-8dfe-7d84192c78da.

## Checkpoint validation

Original checkout was clean. This commit adds only this operational handoff.
`git diff --cached --check` passed. Full product gates were not run for this documentation snapshot.
No feature, merge, release, cloud operation or deployment was performed.
