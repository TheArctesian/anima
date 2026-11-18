# Handoff: branch `offline-copy` (task F2)

Date: 2026-11-18. This note is a record (PLN-010, R-156). The owner of the work is task F2 in
[coordinator-standby.md](../proposals/coordinator-standby.md); its state is in that row. Delete this note in the
commit that merges the branch.

## What was done

- Task F2 (FND-007) is built in 53af82a1 and corrected after a review in 3b46dfef. The code is in
  `apps/client/src/lib/hub/recent/`.
- The branch holds `origin/main` of 2026-11-18. In the merge, two numbers changed because `main` used them first:
  the ruling of F2 is R-157 (it was R-149 on the branch), and its human check is HT-030 (it was HT-026).
- afb1e6a0 corrects one line of `crates/daemon/src/readers/agent_sessions/files.rs` that came from `main`: clippy
  of Rust 1.100.0 refuses it, so the gates stop there on `main` also.

## What is next

- The operator does the check HT-030 in [need-human-testing.md](../need-human-testing.md).
- R-155: a ruling merges to `main` before the code that depends on it. R-157 is only on this branch.
- On macOS 27.3 the gate `go-test` fails in `link/statedir` and `link/cmd/anima-link` on `main` and on this branch:
  `/bin/chmod -R -N` exits 1 when the folder holds a socket file. This branch does not change `link/`.
- The claim [f2.md](../claims/f2.md) is deleted in the commit that merges the branch (PLN-022).
