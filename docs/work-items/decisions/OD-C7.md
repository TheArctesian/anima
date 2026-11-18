# OD-C7: Decide: Access to ai.anima.ing

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | OD-C7 |
| Kind | Decision |
| Owner | [compute-plan.md](../../plans/compute-plan.md) Section 15 |
| Serves | Work plan Section 4, tier 1 (decisions and approvals) |
| Version | 1.0 |
| Date | 2026-11-16 |
| Origin | GitHub issue #49: made 2026-11-07, last changed 2026-11-08, open at import. Labels: decision, compute, design, priority: foundation. |
| Text | Copied from the issue on 2026-11-16 (R-119). Only the links were changed. Not written in STE yet. "This issue" in the text means this document. |

**State (compute plan version 0.3, 2026-11-18).** Open, with a new question. After R-131 the model address is on the loopback address of each computer, and the operator uses the hosted hub and the link (R-130). The recommendation below (VPN only, Headscale DNS, a DNS-01 certificate) is the text of the import and is no longer the recommendation. The compute plan, Section 15, holds the question, the options and the recommendation.

**Decided (R-165, operator, 2026-11-18).** The model address is only the loopback address of each computer, with no name `ai.anima.ing`, no DNS record and no certificate. The compute plan, Section 15, holds the text.

Recommendation: VPN only, with Headscale DNS and a DNS-01 certificate. Confirm that the operator controls the DNS zone of `anima.ing`. Public access is out of scope (SEC-008). Blocks C1.

**Plan:** [Section 15](../../plans/compute-plan.md)
**Requirements:** OD-C7
