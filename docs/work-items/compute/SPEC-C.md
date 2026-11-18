# SPEC-C: Approve the design spec changes for compute

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | SPEC-C |
| Kind | Approval |
| Owner | [compute-plan.md](../../plans/compute-plan.md) Section 1.4 |
| Serves | Work plan Section 4, tier 1 (decisions and approvals) |
| Version | 1.0 |
| Date | 2026-11-16 |
| Origin | GitHub issue #41: made 2026-11-07, last changed 2026-11-08, open at import. Labels: documentation, decision, compute, design, priority: foundation. |
| Text | Copied from the issue on 2026-11-16 (R-119). Only the links were changed. Not written in STE yet. "This issue" in the text means this document. |

**State (compute plan version 0.3, 2026-11-18).** Open. Section 1.4 of the compute plan holds the list of the changes, and it changed: no DNS name and no ACL rule for a hub on the link, model work in `anima-daemon` (MET-002, MET-020; OD-C9), and no router in the hub (R-131). The checklist below is the text of the import; where it does not agree with Section 1.4, Section 1.4 applies.

**Ownership / priority clarification (2026-11-07):**

Before C1 [C1](C1.md), align the older checklist with [approved R-084](../../plans/phase-3/phase-3-daemon-delta.md): metrics are now agent-first with SSH fallback, and compute reuses `agent`/`agent-core` from WP-51/WP-54. The “collector stays agentless” line below is historical. Compute command transport/authorization still needs alignment with MET-020. Include shared private vision in C1, consumed through [E0-5](../engine/E0-5.md) by separate Wardrobe/Library [XP-12](../extensions/XP-12.md)/[XP-13](../extensions/XP-13.md). Prioritize infrastructure approval; navigation/new-screen delivery stays later. Hosted permission [OD-E4](../decisions/OD-E4.md) remains optional; this organization pass grants no blanket spec approval.

---

The compute plan changes parts of `docs/spec/design-spec.md`. Approve each change before C1 starts (operator checkpoint 3).

- [ ] 1.1 Purpose: add a private model service on the operator's devices
- [ ] 3.1 Overview: add the compute agent and the router
- [ ] 4.1 NET-005: add the DNS name `ai.anima.ing`, VPN only
- [ ] 4.2 VPN-004: add ACL rules for compute traffic; a compute node can be a client computer
- [ ] 5.3 Navigation bar: add the Models, Chat and Compute pages
- [ ] 6.6 Metrics: the collector stays agentless; the compute agent is separate and optional
- [ ] 9 Delivery phases: add the compute phases
- [ ] 13 Repository structure: add the compute domain, agent crates and block adapters

**Plan:** [Section 1.4](../../plans/compute-plan.md)
**Requirements:** —
