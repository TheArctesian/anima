# Handoffs

This folder is a record, not a plan (PLN-010). It holds the notes that one session writes so that the next session can continue its work. The operator defined it on 2026-11-18 (R-156).

| Rule | Text |
| --- | --- |
| Content | A note says what was done, where the work is (branch, commit), what is next, and which document owns the work. It points to the owner document. |
| No requirement | A note must not add a requirement, an approval, a state or a decision (PLN-010). Put them in the owner document (work plan Section 5) or in [rulings.md](../rulings.md). |
| Name | `YYYY-MM-DD-<branch>.md`, with `/` in the branch name written as `-`. |
| Secrets | A note must not hold a password, a token, a key or the text of a private session. |
| Life | A note stays on its branch. When the branch merges and its work is in the owner document, delete the note in the merge commit. A note that is on `main` is stale after its work is done. |
| Plans | Work that only a note describes is not a plan (PLN-011). Merge the plan to `main` first (R-155). |

There is no list of notes here. The folder holds only the notes of work that is not done.
