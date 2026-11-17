# KEYS-C: Compute accounts, keys and setup checks

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | KEYS-C |
| Kind | Checklist |
| Owner | [compute-plan.md](../../plans/compute-plan.md) Section 9 |
| Serves | Work plan Section 4, tier 1 (decisions and approvals) |
| Version | 1.0 |
| Date | 2026-11-16 |
| Origin | GitHub issue #42: made 2026-11-07, last changed 2026-11-08, open at import. Labels: security, compute, infrastructure, priority: foundation. |
| Text | Copied from the issue on 2026-11-16 (R-119). Only the links were changed. Not written in STE yet. "This issue" in the text means this document. |

**Ownership / priority clarification (2026-11-07):**

Provision per phase: C1 [C1](C1.md), later C5/C6 [C5](C5.md)/[C6](C6.md). The historical hub-to-agent listener ACL below must be reconciled with [R-084/MET-020](../../plans/phase-3/phase-3-agent-delta.md) before implementation; compute transport remains open. Do not treat a metrics-only agent token as compute-command permission. DNS/access decision: [OD-C7](../decisions/OD-C7.md). Each actual cloud/network write still needs operator confirmation (VPS-005).

---

Create each item when its phase starts. Never paste a value in this issue. Each VPS, Headscale or DNS change needs the operator's confirmation in chat (VPS-005).

**Keys**
- [ ] Hugging Face read token, held in the credentials domain (C1)
- [ ] DNS API token for the DNS-01 certificate of `ai.anima.ing`, as a Docker secret (C1)

**Network**
- [ ] Headscale DNS record for `ai.anima.ing` to the hub's VPN address; no public record (C1)
- [ ] ACL: `tag:compute`, hub to agent, nodes to Hugging Face hosts (C1)
- [ ] ACL: ring stages and `rpc-server` pool host (C5, C6)

**Checks not yet verified**
- [ ] The operator controls the DNS zone of `anima.ing` (OD-C7)
- [ ] Added delay of caller → VPS → node routing is acceptable (OD-C5)

**Plan:** [Section 6.1, 9](../../plans/compute-plan.md)
**Requirements:** RTR-001–005, SEC-C-003, SEC-C-004
