---
kind: Plan
state: Approved
evidence: R-152 (operator, 2026-11-18) names PE-C1 and PE-C2 as read-only preparation. No parser, claim, gate or acceptance policy changes are approved.
---

# Plan: evidence that a task owner can use

Serves row 4.3 of [priorities.md](../plan/priorities.md). The rules stay in [rules.md](../plan/rules.md); this document owns only the tasks below.

## Tasks

| Task | Work and acceptance | Depends on | State |
| --- | --- | --- | --- |
| PE-C1 | Reconcile the human-check index, detail headings, priority rows and generated queue by subject and owner. Record duplicated numbers, omitted entries and missing Results. A missing Result stays missing. Exclude the new report itself from repeated searches. Give each discrepancy to its check owner; do not renumber another branch or invent a pass. | Current main; [need-human-testing.md](../need-human-testing.md); the check owners | Not started. Read-only preparation (R-152) |
| PE-C2 | Prepare one interrupted-task handoff example: immutable branch commit, exact input main commit, changed files, task states, evidence paths, check result on that tree, remaining step and claim owner. Distinguish a past gate duration from current merge proof. List automatable prerequisites of manual checks separately from the operator's actual assertions. Do not add a probe, scheduling lock or gate-cache policy without a separate task and decision. | Current task owners; rules PLN-008, PLN-009 and PLN-022 | Not started. Read-only preparation (R-152) |

## Open decisions

| Decision | Question | Recommendation | State |
| --- | --- | --- | --- |
| PE-D1 | After PE-C1, who resolves check identity before two branches merge? | The integration owner and both check owners compare subjects and evidence on current main; take a fresh number only at integration. | Open. The existing numbering rule is unchanged |
| PE-D2 | Does the owner want a scripted preflight or coordinated gate scheduling after PE-C2? | First measure the missing prerequisite and contention; approve a small interface separately. Do not treat a rerun under lower load as evidence that a failure was harmless. | Open. No new runner or fixture is approved |
