# OD-C5: Decide: Router location

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | OD-C5 |
| Kind | Decision |
| Owner | [compute-plan.md](../../compute-plan.md) Section 15 |
| Serves | Work plan Section 4, tier 1 (decisions and approvals) |
| Version | 1.0 |
| Date | 2026-11-16 |
| Origin | GitHub issue #47: made 2026-11-07, last changed 2026-11-08, open at import. Labels: decision, compute, design, priority: foundation. |
| Text | Copied from the issue on 2026-11-16 (R-119). Only the links were changed. Not written in STE yet. "This issue" in the text means this document. |

The router runs in the hub on the VPS (OD-9), so each request goes caller → VPS → node. Recommendation: measure the added delay in C1; if too high, run a second router at the home site.

**Plan:** [Section 15](../../compute-plan.md)
**Requirements:** OD-C5
