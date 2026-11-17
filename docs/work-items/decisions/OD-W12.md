# OD-W12: Decide: the proposals that follow projects-first (builds as jobs, native Mac jobs, forge sign-in)

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | OD-W12 |
| Kind | Decision |
| Owner | [work-plan.md](../../work-plan.md) Section 7 |
| Serves | No tier: the work is closed or the work plan holds it |
| Version | 1.0 |
| Date | 2026-11-16 |
| Origin | GitHub issue #61: made 2026-11-13, last changed 2026-11-13, closed 2026-11-13 at import. Labels: decision, design, priority: later. |
| Text | Copied from the issue on 2026-11-16 (R-119). Only the links were changed. Not written in STE yet. "This issue" in the text means this document. |
| Note | The work plan Section 7 holds the ruling. OD-W13 later changed PRO-010 and PRO-013. |

The operator decided OD-W11 on 2026-11-13: projects are the front door for rented accounts, and the builder builds each project. `docs/projects-and-builds-plan.md` has the six points.

These proposals follow from it. The operator heard them and did not refuse. They are not approved.

- [ ] PRO-009: a build is a job, the first job type. Jobs come directly after Projects.
- [ ] PRO-010: Anima schedules builds (a GitHub App tells the hub). No self-hosted runners of GitHub Actions.
- [ ] PRO-011: native jobs on a Mac come early. A macOS or iOS build cannot run in a container.
- [ ] PRO-012: a signing key and a secret belong to a project, not to a builder.
- [ ] PRO-013: build only the repositories and branches of the operator. Never a pull request from a fork.
- [ ] PRO-014: read access first. Write access comes with the builder.
- [ ] PRO-015, PRO-016: "connect GitHub" is not "sign in with GitHub". Forge sign-in comes later; the local sign-in stays the root.

## Conflicts to settle with this decision

- PRJ-001 says that the operator accepts each project. OD-W11 says: import each repository, leave some out.
- JOB-001 and USE-001 say that a job is a container image. PRO-011 does not agree. A native job has no container around it, so it needs its own limits.

## Comments on the issue

### 2026-11-13, TheArctesian

Decided by the operator on 2026-11-13 and recorded on main (697e35c2 to 9f88f727).

- PRO-009: the build is the first job type, directly after Projects.
- PRO-010, PRO-020, PRO-021: Anima schedules builds from a webhook of the GitHub App through the webhook receiver (on the VPS, OD-E15), with a slow poll as repair; it can also start or re-run an Actions workflow; a machine is never a runner.
- PRO-011, PRO-022 to PRO-024: a Mac build is a native job with a build user, a deleted checkout and a setting on the Mac.
- PRO-012, PRO-013, PRO-015, PRO-016: decided as written. PRJ-017 replaces the first text of PRO-014.
- JOB-010: the hub gives a project secret to one job. OD-E14: the first GitHub write functions are decided.
- GitHub is an exception to OD-W1; a different rented service needs its own ruling.

Not done: ruling numbers in programme.md, the security review of builds, the design of the build user on a Mac, and the open port on the VPS.
