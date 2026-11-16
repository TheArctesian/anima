# Rules for plans and progress

Part of the work plan: [work-plan.md](../work-plan.md) is the entry point, and its Section 10 is is this document. A reference to "work plan Section 10 is" in a different document means this document.

| Item | Value |
| --- | --- |
| Document | The rules PLN-001 to PLN-017 |
| Version | 2.0 |
| Date | 2026-11-16 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved text of the work plan, version 1.3. Version 2.0 only moved the text to this document (operator, 2026-11-16). |

## 10. Rules for plans and progress

These rules apply to each person and each agent (operator, 2026-11-15).

- **PLN-001. One register.** Section 9 is the only list of plans. Do not build from a document that is not in it.
- **PLN-002. No plan without a row.** Do not make a new plan, proposal or task breakdown unless the same commit adds its row to Section 9.
- **PLN-003. Each plan has a parent.** A new plan must name the row of Section 4 that it serves. If no row applies, ask the operator for one first.
- **PLN-004. Change a plan before you make a new one.** If a plan in Section 9 owns the concern (Section 5), change that plan.
- **PLN-005. One standard.** Each plan obeys the writing rules in Section 0 of the design specification. Each plan has a title, a version, a date and a status. The line below its title points to Section 9.
- **PLN-006. Tasks hold their state.** A plan or a task breakdown that has tasks must have a table with one row for each task. The row gives the state and the commits.
- **PLN-007. The words for a state.** A plan is Draft, Approved, In progress, Done, Parked or Withdrawn. A task is Not started, In progress, Done or Cancelled. "Not checked" is permitted only in Section 9.
- **PLN-008. Progress moves with the work.** Change the state of a task in the commit that does the work, or in its merge. Change Section 9 when the state of a plan changes.
- **PLN-009. A state needs evidence.** "Done" must name a commit, an acceptance record or a ruling of the operator. Do not write "Done" for work that waits for a live check.
- **PLN-010. Records are not plans.** A report, a log, a procedure or a review must not add a requirement. Put the requirement in its owner (Section 5).
- **PLN-011. A plan on a branch.** A plan that is only on a branch is not a plan. Merge it with its row before work on it starts.
- **PLN-012. No second store.** Do not keep a plan, a task, a decision, a defect or a deferred item in a GitHub issue, a project board or a label (operator, 2026-11-16, R-119). A new work item or a new defect is a document in [work-items/](../work-items/README.md) with a row in its index. A deferred item is a line in [deferred.md](../deferred.md). An open decision is a row in the "Open decisions" section of its plan. If a person outside the project opens an issue, move its content to a document and close the issue with the path of the document.
- **PLN-013. Plan first.** A session that builds must start from a plan (operator, 2026-11-16: "The coding session should plan for or at least read from the plan and build or update the plan"). Before it changes code, the session must do one of these:
  1. Find the plan that owns the work (Section 5, Section 9), read it, and build from it.
  2. If that plan does not include the work, change the plan first (PLN-004). Then build.
  3. If no plan owns the work, write a proposal first, with its row (PLN-002, PLN-003). A short proposal is sufficient: the goal, the requirements with identifiers, and the open decisions.

  If the build must be different from the plan, change the plan in the same commit. Change the state of the task in the commit that does the work (PLN-008). A correction of a defect that adds no function and changes no requirement needs no plan; it names the requirement that it corrects.
- **PLN-014. One list of operator checks.** A check that only the operator can do (a real machine, a real account, a real network) has a row in [operator-checks.md](../operator-checks.md) (operator, 2026-11-16). The row names what the check blocks and holds its result. The steps stay in the document that owns them. Add the row in the commit that defers the check. A task that waits for a check is not Done until its row says Pass (PLN-009).
- **PLN-015. No line limit for planning documents.** The size limits of `AGENTS.md` apply to code only (operator, 2026-11-16). Do not divide a specification, a plan or a proposal to make it shorter. Divide a document only when it holds two subjects that have different owners, and keep its identifiers and its section numbers.
- **PLN-016. One entry, one job, one owner.** These three rules keep the documents easy to find and hard to misuse (operator, 2026-11-16).
  1. One entry. [work-plan.md](../work-plan.md) is the entry point. A reader must be able to get from it to each planning document through a list: the register, the index of the work items, or a link in this folder.
  2. One job. A document has one job: a map, a specification, a plan, a proposal, a task breakdown, a list or a record (Section 9 gives the kinds). Do not add a second job to a document. Put the text in the document that has that job.
  3. One owner. One document holds each fact. One document defines each requirement identifier (Section 5). The register holds the state of a plan. The index of the work items holds the state of an item. [operator-checks.md](../operator-checks.md) holds the result of a check. A different document points to the owner; it does not copy the fact.
- **PLN-017. The documents gate.** `bun run quality:documents` is one of the gates (`bun run gates`). It fails when a relative link does not resolve, when a plan, a proposal or a task breakdown has no link in the register (PLN-001), when a work item has no link in its index (PLN-012), when a plan has no status and no pointer to the register in its first 20 lines (PLN-005), or when a second document defines a requirement identifier (PLN-016). `quality/duplicate-identifiers.txt` lists the identifiers that had two definitions on 2026-11-16. That list can only get shorter.

A task list that an agent keeps for one session is not a document. These rules do not apply to it.
