# OD-C2: Align shared metrics/compute agent with separate personal collector

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | OD-C2 |
| Kind | Decision |
| Owner | [compute-plan.md](../../plans/compute-plan.md) Section 15 |
| Serves | Work plan Section 4, tier 1 (decisions and approvals) |
| Version | 1.0 |
| Date | 2026-11-16 |
| Origin | GitHub issue #44: made 2026-11-07, last changed 2026-11-08, open at import. Labels: decision, compute, design, priority: foundation. |
| Text | Copied from the issue on 2026-11-16 (R-119). Only the links were changed. Not written in STE yet. "This issue" in the text means this document. |

**State (compute plan version 0.3, 2026-11-18).** Decided for the compute plan by R-084 (one daemon for the metrics and the compute work; R-126 gives its name). The separation from `anima-collector` is OD-E2 of the extensions plan. What the daemon is permitted to do for a model is OD-C9 of the compute plan, Section 15. The text below is the text of the import.

**Ownership / priority clarification (2026-11-07):**

[R-084](../../plans/phase-3/phase-3-daemon-delta.md) already establishes one metrics agent (`agent`, `agent-core`) that C1 [C1](../compute/C1.md) later extends for compute. Do not create another metrics/compute binary. The retained boundary is separation from personal-source `anima-collector` [E0-3](../engine/E0-3.md) and any justified shared enrolment library. Align compute command transport/privileges in [C1](../compute/C1.md)/[SPEC-C](../compute/SPEC-C.md); that decision remains open.

---

Recommendation: keep `anima-agent` separate from `anima-collector`; share enrolment code in a library crate.

**Plan:** [Section 15](../../plans/compute-plan.md)
**Requirements:** OD-C2
