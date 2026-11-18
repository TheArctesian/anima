# OD-C8: Decide: Device policy default for client computers

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | OD-C8 |
| Kind | Decision |
| Owner | [compute-plan.md](../../plans/compute-plan.md) Section 15 |
| Serves | Work plan Section 4, tier 1 (decisions and approvals) |
| Version | 1.0 |
| Date | 2026-11-16 |
| Origin | GitHub issue #50: made 2026-11-07, last changed 2026-11-08, open at import. Labels: decision, compute, design, priority: foundation. |
| Text | Copied from the issue on 2026-11-16 (R-119). Only the links were changed. Not written in STE yet. "This issue" in the text means this document. |

**State (compute plan version 0.3, 2026-11-18).** Open, with a new question: one switch "takes work" (USE-004), or a second switch for compute. The recommendation below is the text of the import and is no longer the recommendation. The compute plan, Section 15, holds the question, the options and the recommendation.

**Decided (R-165, operator, 2026-11-18; R-160).** One switch, "takes work" (USE-004), on by default on each hub; no second switch for compute. The compute plan, Section 15 and CMP-002, holds the text.

Recommendation: off by default; the operator sets each client computer on.

Note of 2026-11-18: OD-W10 and R-160 ("A: on everywhere") say that a machine with the daemon takes work by default, on each hub. [jobs.md](../../proposals/jobs.md), "Decision", says that this recommendation and CMP-002 must change to agree. The owner, the compute plan Section 15, made that change with R-165.

**Plan:** [Section 15](../../plans/compute-plan.md)
**Requirements:** OD-C8
