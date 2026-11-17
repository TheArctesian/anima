---
kind: Task breakdown: the work items that came from the GitHub issues, with the state of each
state: In progress
evidence: R-119 (operator, 2026-11-16). 57 items: engine, extensions, compute, decisions, device manager and defects. Each item names the row of Section 4 that it serves. The states are the states of the issues at the import and are not checked against the code.
---

# Work items

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the row of this index (PLN-005).

| Item | Value |
| --- | --- |
| Document | Work items: the index and the state of each item |
| Version | 1.0 |
| Date | 2026-11-16 |
| Status | The header of this document holds the state (PLN-020). History: In progress. The items came from GitHub on 2026-11-16 (R-119). The state of each item is the state of its issue at the import, not a check against the code. |

The operator decided on 2026-11-16 that GitHub issues are not a store (R-119). The planning documents are the one source of truth. This folder holds each issue that was a work item, a decision, an approval, a checklist or a defect. The deferred list of issue #1 is [deferred.md](../deferred.md).

Rules:

- This table holds the state of each item (PLN-006). Change the row in the commit that does the work (PLN-008).
- An item must not add a requirement that its owner plan does not have. Where an item has newer text than its plan, the Note says so, and the plan must change.
- An older document that names an issue number (`#N`) means the item in the column "Was issue". `#1` is [deferred.md](../deferred.md).
- "Open at import" means: the issue was open, and no person or agent compared the item with the code.
- The issues were read again on 2026-11-16 before the merge into `main`: only the text of E0-4 had changed, and its document has the new text.

## Engine (E0)

| Identifier | Title | Kind | State | Tier (work plan Section 4) | Owner | Was issue |
| --- | --- | --- | --- | --- | --- | --- |
| [E0-1](engine/E0-1.md) | Regroup hub folders into fleet/engine/vault/life/work | Work item | On `main` (merge `78df5bcb`, 2026-11-13). Not compared with the text of the item | 2a (the engine) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.1 | #4 |
| [E0-2](engine/E0-2.md) | Shared source connections and retry-safe imports (Integrations UI later) | Work item | On `main`: the sources framework and Settings > Integrations (merges `c70a3e32`, `8c98653d`, 2026-11-13). Not compared with the text of the item | 2a (the engine) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.1 | #5 |
| [E0-3](engine/E0-3.md) | Reuse SSH/SFTP for demand-driven imports and redaction (later) | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.1 | #6 |
| [E0-4](engine/E0-4.md) | Timeline and record links | Work item | On `main` since 2026-11-17 (merge of the branch `engine/e0-4-timeline-links`). The extensions plan, "Tasks of E0-4", holds the state of each task | 2a (the engine) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.1 | #7 |
| [E0-5](engine/E0-5.md) | Places, location ingestion and event-time context; attachment/extraction contracts | Work item | Open at import | 2a (the engine) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.1 | #8 |
| [E0-6](engine/E0-6.md) | Operational key vault, cache lifecycle and personal-record security review | Work item | Open at import | 2a (the engine) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.1 | #9 |
| [E0-7](engine/E0-7.md) | Reminder engine and ntfy delivery | Work item | Open at import | 2a (the engine) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.1 | #10 |
| [E0-8](engine/E0-8.md) | Extension registry, nav groups and launch briefs | Work item | In progress. On `main`: the extension registry and its switches (merges `3750eb87`, `efcf2fe5`). The launch briefs are not written (ENG-012) | 2a (the engine) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.1 | #11 |

## Extensions

| Identifier | Title | Kind | State | Tier (work plan Section 4) | Owner | Was issue |
| --- | --- | --- | --- | --- | --- | --- |
| [XP-01](extensions/XP-01.md) | Map | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #12 |
| [XP-02](extensions/XP-02.md) | Projects: deployments and analytics | Work item | In progress | 3a (Projects) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #13 |
| [XP-03](extensions/XP-03.md) | Writing | Work item | Open at import | 2a (the engine) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #14 |
| [XP-04](extensions/XP-04.md) | Secrets | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #15 |
| [XP-05](extensions/XP-05.md) | People directory | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #16 |
| [XP-06](extensions/XP-06.md) | Money | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #17 |
| [XP-07](extensions/XP-07.md) | History | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #18 |
| [XP-08](extensions/XP-08.md) | Health and the Garmin bridge | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #19 |
| [XP-09](extensions/XP-09.md) | Documents (legal and identity) | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #20 |
| [XP-10](extensions/XP-10.md) | Reminders page | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #21 |
| [XP-11](extensions/XP-11.md) | AI coding sessions | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #22 |
| [XP-12](extensions/XP-12.md) | Wardrobe | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #23 |
| [XP-13](extensions/XP-13.md) | Library (books and films) | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #24 |
| [XP-14](extensions/XP-14.md) | Project design and map | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.2 | #25 |
| [SPEC](extensions/SPEC.md) | Approve the design spec changes for extensions | Approval | Open at import | 1 (decisions and approvals) | [extensions-plan.md](../plans/extensions-plan.md) Section 1.2 | #26 |
| [KEYS](extensions/KEYS.md) | Accounts, keys and setup checks | Checklist | Open at import | 1 (decisions and approvals) | [extensions-plan.md](../plans/extensions-plan.md) Section 9 | #31 |
| [SEAMS](extensions/SEAMS.md) | Seams between finished extensions | Work item | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 5.4 | #32 |

## Compute

| Identifier | Title | Kind | State | Tier (work plan Section 4) | Owner | Was issue |
| --- | --- | --- | --- | --- | --- | --- |
| [C1](compute/C1.md) | Per-machine model setup and unified ai.anima.ing serving | Work item | Open at import | 2b (Level 1 model service) | [compute-plan.md](../plans/compute-plan.md) Section 12.1 | #34 |
| [C2](compute/C2.md) | Configurable model loading, memory scheduling and request readiness (UI later) | Work item | Open at import | 2b (Level 1 model service) | [compute-plan.md](../plans/compute-plan.md) Section 12.1 | #35 |
| [C3](compute/C3.md) | Single-node offload and KTransformers | Work item | Open at import | 3a (jobs) | [compute-plan.md](../plans/compute-plan.md) Section 12.1 | #36 |
| [C4](compute/C4.md) | Level 2 spikes and decision record | Work item | Open at import | 4 (Level 2) | [compute-plan.md](../plans/compute-plan.md) Section 12.1 | #37 |
| [C5](compute/C5.md) | Single-site pool with llama.cpp RPC | Work item | Open at import | 4 (Level 2) | [compute-plan.md](../plans/compute-plan.md) Section 12.1 | #38 |
| [C6](compute/C6.md) | Cross-runtime ring (the Anima framework) | Work item | Open at import | 4 (Level 2) | [compute-plan.md](../plans/compute-plan.md) Section 12.1 | #39 |
| [C7](compute/C7.md) | Fine-tuning jobs | Work item | Open at import | 3a (jobs) | [compute-plan.md](../plans/compute-plan.md) Section 12.1 | #40 |
| [SPEC-C](compute/SPEC-C.md) | Approve the design spec changes for compute | Approval | Open at import | 1 (decisions and approvals) | [compute-plan.md](../plans/compute-plan.md) Section 1.4 | #41 |
| [KEYS-C](compute/KEYS-C.md) | Compute accounts, keys and setup checks | Checklist | Open at import | 1 (decisions and approvals) | [compute-plan.md](../plans/compute-plan.md) Section 9 | #42 |

## Decisions

| Identifier | Title | Kind | State | Tier (work plan Section 4) | Owner | Was issue |
| --- | --- | --- | --- | --- | --- | --- |
| [OD-E1](decisions/OD-E1.md) | Decide: Where health, money and other Class B data lives | Decision | Open at import | 1 (decisions and approvals) | [extensions-plan.md](../plans/extensions-plan.md) Section 7 | #27 |
| [OD-E4](decisions/OD-E4.md) | Optional hosted extraction permission (local vision owned by C1 #34) | Decision | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 7 | #28 |
| [OD-E10](decisions/OD-E10.md) | Decide: Price source for holdings the source does not price | Decision | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 7 | #29 |
| [OD-E3](decisions/OD-E3.md) | Decide: Check SimpleFIN Bridge coverage for the US accounts | Decision | Open at import | 3b (the other extensions) | [extensions-plan.md](../plans/extensions-plan.md) Section 7 | #30 |
| [OD-C1](decisions/OD-C1.md) | Decide: Hardware of each compute node | Decision | Open at import | 1 (decisions and approvals) | [compute-plan.md](../plans/compute-plan.md) Section 15 | #43 |
| [OD-C2](decisions/OD-C2.md) | Align shared metrics/compute agent with separate personal collector | Decision | Open at import | 1 (decisions and approvals) | [compute-plan.md](../plans/compute-plan.md) Section 15 | #44 |
| [OD-C3](decisions/OD-C3.md) | Decide: Conformance tolerances | Decision | Open at import | 4 (Level 2) | [compute-plan.md](../plans/compute-plan.md) Section 15 | #45 |
| [OD-C4](decisions/OD-C4.md) | Decide: Where the block adapters live | Decision | Open at import | 4 (Level 2) | [compute-plan.md](../plans/compute-plan.md) Section 15 | #46 |
| [OD-C5](decisions/OD-C5.md) | Decide: Router location | Decision | Open at import | 1 (decisions and approvals) | [compute-plan.md](../plans/compute-plan.md) Section 15 | #47 |
| [OD-C6](decisions/OD-C6.md) | Decide: Where conversations are kept | Decision | Open at import | 4 (Level 2) | [compute-plan.md](../plans/compute-plan.md) Section 15 | #48 |
| [OD-C7](decisions/OD-C7.md) | Decide: Access to ai.anima.ing | Decision | Open at import | 1 (decisions and approvals) | [compute-plan.md](../plans/compute-plan.md) Section 15 | #49 |
| [OD-C8](decisions/OD-C8.md) | Decide: Device policy default for client computers | Decision | Open at import | 1 (decisions and approvals) | [compute-plan.md](../plans/compute-plan.md) Section 15 | #50 |
| [OD-W12](decisions/OD-W12.md) | Decide: the proposals that follow projects-first (builds as jobs, native Mac jobs, forge sign-in) | Decision | Closed 2026-11-13 (see its comments) | — | [work-plan.md](../work-plan.md) Section 7 | #61 |

## Device manager

| Identifier | Title | Kind | State | Tier (work plan Section 4) | Owner | Was issue |
| --- | --- | --- | --- | --- | --- | --- |
| [GH-2](device-manager/GH-2.md) | Tamper-evident audit log: a hash chain so a change to an old entry shows | Work item | Open at import | 4 (hash chain for the audit log) | [design-spec.md](../spec/design-spec.md) Section 3.8 (LOG-005, LOG-006) | #2 |
| [GH-55](device-manager/GH-55.md) | Defer Phase 6 site operations and optional traffic metadata | Work item | Open at import | 4 (Phase 6) | [design-spec.md](../spec/design-spec.md) Section 9 (Phase 6) | #55 |
| [GH-56](device-manager/GH-56.md) | Audit log: record who did it — User, Agent or Hub | Work item | Closed 2026-11-13 (see its comments) | — | [design-spec.md](../spec/design-spec.md) Section 3.8.1 (LOG-012 to LOG-020) | #56 |
| [GH-57](device-manager/GH-57.md) | Put the anima command line in the .dmg and the AppImage | Work item | Closed 2026-11-13 (see its comments) | — | [design-spec.md](../spec/design-spec.md) Section 3.11 (INS-001 to INS-009) | #57 |
| [GH-58](device-manager/GH-58.md) | Marketing website with the install links | Work item | Closed 2026-11-13 (see its comments) | — | [design-spec.md](../spec/design-spec.md) Section 3.12 (SITE-001 to SITE-012) | #58 |
| [GH-60](device-manager/GH-60.md) | Try the anima command line and the MCP server against a real hub and a real client | Checklist | Open at import | — (register row of the command line plan) | [cli-mcp-plan.md](../plans/cli-mcp-plan.md) Section 3 | #60 |
| [DMN-1](device-manager/DMN-1.md) | Rename the program `anima-agent` to `anima-daemon` | Work item | In progress (claimed 2026-11-17). Its table holds the state of each task | 0 (finish the device manager) | [design-spec.md](../spec/design-spec.md) Section 6.9 and `contracts/agent-push.md` (K8) | — (R-126) |

## Defects

| Identifier | Title | Kind | State | Tier (work plan Section 4) | Owner | Was issue |
| --- | --- | --- | --- | --- | --- | --- |
| [GH-52](defects/GH-52.md) | Linux desktop build fails: unfulfilled dead_code expectation in web_view/event.rs | Defect | Open at import | 0 (finish the device manager) | [design-spec.md](../spec/design-spec.md) OD-7 | #52 |
| [GH-53](defects/GH-53.md) | VNC login failure shows only 'The gateway reported an error' (user name mismatch with wayvnc) | Defect | Open at import | 0 (finish the device manager) | [design-spec.md](../spec/design-spec.md) GW-015 | #53 |
| [GH-54](defects/GH-54.md) | Linux desktop app: VNC session picture lags behind the remote screen | Defect | Open at import | 0 (finish the device manager) | [design-spec.md](../spec/design-spec.md) RA-030 | #54 |
| [GH-59](defects/GH-59.md) | Hub keeps SNMP secrets when the host of a device changes | Defect | Closed 2026-11-13 (see its comments) | — | [design-spec.md](../spec/design-spec.md) SEC-005, SEC-010, SEC-011 | #59 |
