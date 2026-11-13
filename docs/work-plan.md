# Anima Work Plan

| Item | Value |
| --- | --- |
| Document | Work plan: purpose, categories and priority order |
| Version | 0.4 (draft) |
| Date | 2026-11-12 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | For review. The operator has not approved the principles or the priority order. |
| Related | [design-spec.md](design-spec.md), [extensions-plan.md](extensions-plan.md), [compute-plan.md](compute-plan.md), [vpn-plan.md](vpn-plan.md), [programme/handoff.md](programme/handoff.md), [programme/proposals/](programme/proposals/) |

This document obeys the writing rules in Section 0 of the design specification.

This document is a map. It gives the purpose of Anima, five categories for all planned work, and one priority order. It contains no build requirements. Each requirement is in the plan or the proposal that owns it (Section 5). This document does not replace a plan and does not change a requirement identifier.

GitHub issues hold the state and the priority of each item of work. If this document and an issue do not agree, the issue is correct.

## 1. Purpose

**Everything that computes for you, or holds data about you, in one place that you own.**

Anima joins three types of resource:

| Type | Examples | What Anima does |
| --- | --- | --- |
| Owned | The computers at home and at the offices | Operates them |
| Rented | Hostinger, Vercel, Cloudflare, Neon | Oversees them |
| Held about you | WHOOP, Garmin, Apple Health, Spotify, a bank | Brings a copy home |

Anima is for the operator first. Later, other persons can run their own Anima. Each person has their own coordinator and their own data (OD-W2).

## 2. Principles

The coordinator is the hub of the design specification (OD-W3).

- **ETH-001. One home.** Each person has one coordinator.
- **ETH-002. The coordinator is portable.** It can run on a VPS, at home, or on a host that a service operates. No code must assume one provider.
- **ETH-003. One home, copies by policy.** Each set of data has one home. A policy places its copies on other machines. The operator does not select the machines. The live database has at least two standby copies. Bulk data has at least two copies of each encrypted piece.
- **ETH-004. The operator confirms a new home.** Anima finds a failure and selects the best copy. A copy becomes the home only after the operator confirms. Anima must not elect a new home alone.
- **ETH-005. Spread the work, not the state.** Work that holds no data can run on any machine. Only the home accepts writes. A machine that has no key can hold only encrypted pieces.
- **ETH-006. Own first.** When owned and rented resources can do the same work, Anima shows the owned resource first.
- **ETH-007. Control follows ownership.** Anima can change an owned resource: a machine with `anima-agent` takes work from the coordinator by default, and the operator can set a machine off (USE-004). For a rented resource, see OD-W1. Anima only reads a resource that is held about the operator.
- **ETH-008. Private by construction.** The coordinator is not on the public internet (SEC-008). A source uses read-only access unless ETH-007 permits more.
- **ETH-009. Build for use.** Build only for the services and devices that the operator uses (EXT-000, CMP-001). A different person must be able to add a source without a change to the engine.

## 3. Categories

| Category | Meaning | Content | Owner documents |
| --- | --- | --- | --- |
| Foundation | The parts that all other work uses | Coordinator, VPN between sites, `anima-agent` on each machine, the clients, the standby copies, bulk data on the operator's machines | Design specification Sections 3, 4, 5 and 10. [vpn-plan.md](vpn-plan.md). [coordinator-standby.md](programme/proposals/coordinator-standby.md), [bulk-data-placement.md](programme/proposals/bulk-data-placement.md) |
| See | Know the state of all that is yours | Checks, metrics, notifications, cloud accounts (Projects), the Integrations section | Design specification Section 6. Extensions plan Sections 4.2 and 4.5 |
| Reach | Go into a thing and act on it | SSH, RDP, VNC, web interfaces, Secrets | Design specification Section 6.3. Extensions plan Section 4.4 |
| Remember | Keep a copy of the life of the operator | Health, Money, Map, People, History, Documents, Writing, Reminders, Library, Wardrobe, AI coding sessions | Extensions plan Sections 2 and 4. [new-sources.md](programme/proposals/new-sources.md) |
| Use | Make the machines do work | Model service, jobs, AI agent runs, hosted applications | [compute-plan.md](compute-plan.md). [jobs.md](programme/proposals/jobs.md), [agent-runs.md](programme/proposals/agent-runs.md), [hosted-applications.md](programme/proposals/hosted-applications.md) |

## 4. Priority order

The tiers come from the issue labels `priority: foundation`, `priority: later` and `blocked: device-manager`, read on 2026-11-12. Do the work in this order. A lower row can start earlier only when it does not delay a higher row.

| Tier | Category | Work | Issues | State |
| --- | --- | --- | --- | --- |
| 0 | Foundation, See, Reach | Finish the device manager: the Phase 3 acceptance run (WP-53) and the operator steps before it. Three open defects. | [handoff.md](programme/handoff.md) Sections 3 and 4. #52, #53, #54 | Phases 1 to 3 are built. Acceptance waits for the operator. This tier removes `blocked: device-manager` (PH-E-000, PH-C-000). |
| 1 | All | Decisions and approvals that the foundation work needs | Extensions: #26, #27, #31. Compute: #41, #42, #43, #44, #47, #49, #50. This plan: Section 7 | Open |
| 2a | See, Remember | The engine (E0), with Writing as its sample extension | #4, #5, #7, #8, #9, #10, #11, #14 | Planned. `priority: foundation` |
| 2b | Use | Level 1 model service: C1 and C2 | #34, #35 | Planned. `priority: foundation`. The issues were given a new scope on 2026-11-07 and are newer than the compute plan. |
| 3 | See, Remember, Reach | The extensions, in the order of XP-005. Projects comes early in that order. | #12, #13, #15 to #25, #6, #32. Decisions #28, #29, #30 | Planned. `priority: later` |
| 3 | Use | Jobs, then AI agent runs, then hosted applications | #40 (C7), #36 (C3). No issue yet for the three proposals | #40 is planned. The proposals are not approved. |
| 3 | Foundation | Standby copies of the coordinator database, and the move procedure | No issue yet | Proposal. Its direction is decided (OD-W9). The operator decides its tier. |
| 3 | Foundation | Bulk data in encrypted pieces on the operator's machines | No issue yet. It changes the file store of #8 | Proposal. Its direction is decided (OD-W9). Decide it before E0-5 (#8) starts. |
| 4 | Use | Level 2: one model across machines (C4 to C6) | #37, #38, #39. Decisions #45, #46 | Parked. Start only after Level 1 is in daily use (CMP-004, PH-C-001). |
| 4 | Foundation | Replicated ledger and sync between nodes | #2 | The OD-W9 decision does not agree with its phases L1 to L4. Its phase L0 (a hash chain for the audit log) can stay. The issue is not yet changed. |
| 4 | Foundation, Reach | iOS client (Phase 5). Site operations and traffic data (Phase 6) | Design specification Section 9. #55 | Planned. #55 puts Phase 6 after the extension foundations and compute. |
| — | All | Deferred items and ideas from other products | #1 | Recorded, not planned |

Notes:

- Tiers 2a and 2b do not depend on each other. They can run at the same time.
- Each extension needs the engine (PH-E-000). The extensions are independent workflows (XP-004).
- In tier 3, the order inside Use is the order that the operator gave: jobs first, then AI agent runs soon after, then hosted applications. The position of Use against the extensions is open (OD-W8).
- The side chat of 2026-11-12 put the standby copy directly after tier 0. The issues have no item for it. The operator decides its tier.
- Phase 4 (Proxmox VE) is withdrawn (R-109). Session recording is deferred (R-113). This plan does not include them.

## 5. Where each requirement lives

One document owns each concern. Add a requirement to its owner, not to this document.

| Concern | Owner | Identifiers |
| --- | --- | --- |
| Device manager: hub, clients, sessions, checks, metrics, notifications | [design-spec.md](design-spec.md) | As published there |
| VPN and enrolment | [vpn-plan.md](vpn-plan.md) | As published there |
| Agent push | `contracts/agent-push.md` (K8), [phase-3-agent-delta.md](programme/phase-3-agent-delta.md) | MET-011 to MET-025 |
| Engine and extensions | [extensions-plan.md](extensions-plan.md) | ENG, SRC, COL, VLT, XP, PRJ and the others there |
| Model service and pooled models | [compute-plan.md](compute-plan.md), as changed by #34 and #35 | CMP, CAG, CAP, RTR and the others there |
| Copies of the coordinator database, and the move of the coordinator | [coordinator-standby.md](programme/proposals/coordinator-standby.md) | FND-001 to FND-012 |
| Bulk data on the operator's machines | [bulk-data-placement.md](programme/proposals/bulk-data-placement.md) | STO-001 to STO-011 |
| Rules for all work on machines, and jobs | [jobs.md](programme/proposals/jobs.md) | USE-001 to USE-009, JOB-001 to JOB-007 |
| AI agent runs | [agent-runs.md](programme/proposals/agent-runs.md) | AGN-001 to AGN-008 |
| Hosted applications | [hosted-applications.md](programme/proposals/hosted-applications.md) | HOST-001 to HOST-009 |
| Cloudflare, Spotify and Screen Time sources | [new-sources.md](programme/proposals/new-sources.md) | None until each provider is verified |
| Rulings and the state of each package | [programme/programme.md](programme/programme.md), [programme/handoff.md](programme/handoff.md) | R, CR |

The six proposals are drafts. A proposal becomes a plan when the operator approves it with a ruling. Then its work gets an issue with the labels of Section 4.

## 6. Not planned

Do not build these.

| Item | Reason |
| --- | --- |
| Kubernetes, with each machine as a node | It needs servers that are the same and always on. A Mac cannot be a full node. |
| A new coordinator without the operator's confirmation | Two sites that lose the VPN link can each elect one. Anima selects the copy; the operator confirms (ETH-004, FND-005). |
| A live database that is split across machines, or that more than one machine can write | Each query then needs more than one machine. The database is small (DB-009). Bulk data is spread as encrypted pieces (STO-001). |
| Automatic moves of applications that keep data | HOST-006 applies. |
| Builds from source for hosted applications, a reverse proxy, preview deployments | HOST-009 applies. Use Vercel or Coolify for these. |

Level 2 of the compute plan is not in this table. It stays in the compute plan and it is parked (tier 4).

## 7. Open decisions

Decisions of the other plans stay in their issues (Section 4, tier 1). These are the decisions of this plan.

| Number | Decision | Recommendation |
| --- | --- | --- |
| OD-W1 | Can Anima change a rented resource (redeploy, purge a cache, restart the VPS)? | Stay read-only now (PRJ-003, PRJ-010). Add a short fixed list of actions later, each with a confirmation and an audit entry. |
| OD-W2 | How do other persons use Anima? | Self-host first. A later service can supply the public parts only: VPN coordination, a relay and the OAuth applications. It must not be able to read data. |
| OD-W3 | The name of the hub in the interface | Consider "anima" for the coordinator. Decide before more interface text says "hub". |
| OD-W4 | How do commands get to the agent? | Not a decision of this plan. It is open in #34, #41 and #44. OD-W10 gives its direction: the agent asks the hub for work and listens on no port (USE-005, MET-020). |
| OD-W5 | The runtime for functions that answer a request | Decide after jobs are in use. WebAssembly is one option. |
| OD-W6 | Railpack as the image builder | Test it on one project of the operator before JOB-002 is final. |
| OD-W7 | The compute plan says that the agent is optional and that metrics use SSH | Not a decision of this plan. #41 and #44 record that R-084 replaces that text. |
| OD-W8 | The position of Use against the extensions in tier 3 | The operator decides. The two do not depend on each other. |
| OD-W9 | One home with copies, or a copy on each node that syncs between nodes when the hub is down (issue #2) | **Decided (operator, 2026-11-12): one home, copies by policy.** The section "Decision" of [coordinator-standby.md](programme/proposals/coordinator-standby.md) has the operator's words. It needs a ruling number in `programme.md`. Open: what to do with issue #2. |
| OD-W10 | Can the agent do work that the coordinator gives it? R-115 did not take a command channel to the agent. | **Decided (operator, 2026-11-12): yes, one agent, on by default ("it should be on by default").** The section "Decision" of [jobs.md](programme/proposals/jobs.md) has the limits. It needs a ruling number that changes R-115. It also changes CMP-002 and the recommendation of OD-C8 (#50). |

## 8. Changes that this plan needs in other documents

These changes are not made. The operator must approve each one (operator checkpoint 3).

| Document | Change |
| --- | --- |
| Design specification 1.1 | Add the purpose line and the three resource types of Section 1. Issues #26 and #41 already change this section. |
| Design specification 4.2 and OD-9 | Call the Hostinger VPS the reference setup (ETH-002). |
| Design specification 9 | Add a pointer to the priority order of this plan. |
| Extensions plan 4.2 and 9 | Add Cloudflare after verification ([new-sources.md](programme/proposals/new-sources.md)). |
| Extensions plan 4 and 9 | Add Spotify and Screen Time after verification ([new-sources.md](programme/proposals/new-sources.md)). |
| Extensions plan FIL-001 | Keep files on the operator's machines, not in a volume on the hub ([bulk-data-placement.md](programme/proposals/bulk-data-placement.md)). |
| Compute plan CMP-002 and OD-C8 | A machine with the agent takes work by default (OD-W10). Issue #50 has the old recommendation. |
| Compute plan 3.3, 12.1 and 13 | Agree with R-084 and with the new scope of #34 and #35 (#41). Make C7 the first job type of [jobs.md](programme/proposals/jobs.md). |
