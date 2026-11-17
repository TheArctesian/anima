# Plan register

Part of the work plan: [work-plan.md](../work-plan.md) is the entry point, and its Section 9 is is this document. A reference to "work plan Section 9 is" in a different document means this document.

| Item | Value |
| --- | --- |
| Document | The only list of the plans of Anima, with the state of each (PLN-001) |
| Version | 2.0 |
| Date | 2026-11-16 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved text of the work plan, version 1.3. Version 2.0 only moved the text to this document (operator, 2026-11-16). |

## 9. Plan register

This table is the only list of the plans of Anima. A document that is not in this table is not a plan (PLN-001). The states were checked on 2026-11-15 against the documents, the commits on `main` and the issues. "Not checked" means that no person or agent compared the plan with the code.

| Document | Kind | State | Evidence |
| --- | --- | --- | --- |
| [design-spec.md](../design-spec.md) | Specification | Approved | `AGENTS.md` names it the source of truth. |
| [implementation-plan.md](../implementation-plan.md) | Plan: Phase 1 | Done | Section 4, tier 0: Phases 1 to 3 are built. Its header still says "draft". |
| [programme/phase-2-plan.md](../programme/phase-2-plan.md) | Plan: Phase 2 | Done | Adopted (R-065). Section 4, tier 0. |
| [programme/phase-3-plan.md](../programme/phase-3-plan.md) | Plan: Phase 3 | In progress | Adopted (R-081). All packages are done. The acceptance run (WP-53) waits for the operator ([handoff.md](../programme/handoff.md), 2026-11-12). |
| [programme/phase-3-agent-delta.md](../programme/phase-3-agent-delta.md) | Plan: change to Phase 3 | Done | Approved 2026-11-07. WP-54 is done ([handoff.md](../programme/handoff.md)). |
| [vpn-plan.md](../vpn-plan.md) | Plan | Approved. Build state not checked | Accepted by the operator on 2026-11-10. |
| [vpn-build.md](../vpn-build.md) | Build specification of the VPN plan | Approved. Build state not checked | Version 0.2. |
| [cli-mcp-plan.md](../cli-mcp-plan.md) | Plan | In progress | Section 3: the command line and the MCP server are on `main`. The test against a real hub is open ([GH-60](../work-items/device-manager/GH-60.md)). R-120 (operator, 2026-11-16): the MCP server is fully automatic by default (MCP-024 to MCP-026); ruled and not built. |
| [extensions-plan.md](../extensions-plan.md) | Plan | In progress | Its header says "For review". On `main`: E0-1, E0-2, E0-8 without its launch briefs, the Projects extension (PRO-001 to PRO-004) and Connect GitHub (PRJ-039 to PRJ-042) (Section 4, tiers 2a and 3a). E0-3 to E0-7 and the other extensions are not on `main`. The index of the work items holds the state of each item ([work-items/](../work-items/README.md), "Engine", "Extensions" and "Decisions"). |
| [compute-plan.md](../compute-plan.md) | Plan | In progress | Its header says "For review". Its work items were open at the import of 2026-11-16 ([work-items/](../work-items/README.md), "Compute" and "Decisions"). The tool inventory (CAP-020 to CAP-031) is built: the agent, the hub, `anima capability list` and the MCP tool (commits 1cb43b06 to a0cf53e5). Its live checks are open ([need-human-testing.md](../need-human-testing.md), HT-002 to HT-006). Its client section is approved and not started (Section 11.1, CUI-010 to CUI-019). No other part is approved. |
| [mobile-plan.md](../mobile-plan.md) | Plan | Approved | Version 0.2 (2026-11-16). The operator decided OD-M1 to OD-M8 on 2026-11-16. It serves Section 4, tier 4, "iOS client and Android client (Phase 5)", and starts now (OD-M2). MOB-001 to MOB-042. No task is started. Its facts are from a test build on the local branch `spike/mobile`, which is not for merge. |
| [power-health-plan.md](../power-health-plan.md) | Plan | Draft | Version 0.1 (2026-11-14). Its header says: for operator review, nothing in this plan is accepted. `PWR`, `HLT`, `HWI` are proposed. Its `HLT-001` to `HLT-007` have the same identifiers as requirements of the Health extension in the extensions plan (PLN-016): one of the two must get new identifiers. No tier in Section 4 yet. |
| [ui-redesign-direction.md](../ui-redesign-direction.md) | Plan: direction of the redesign | Not checked | Its title says "draft for approval". |
| [ui-redesign-plan.md](../ui-redesign-plan.md) | Plan | Not checked | Version 0.2. |
| [ui-site-language-plan.md](../ui-site-language-plan.md) | Plan | Approved. Build state not checked | Approved for implementation on 2026-11-13. |
| [research/one-click/s2-vnc-tasks.md](../research/one-click/s2-vnc-tasks.md) | Task breakdown: native VNC gateway | In progress | Its Section 6: T1 to T8, T10 and T11 have commits on `main`. T9 waits for the operator. Its parent plan is [one-click-plan.md](../one-click-plan.md). |
| [research/one-click/s2-rdp-tasks.md](../research/one-click/s2-rdp-tasks.md) | Task breakdown: native RDP gateway | In progress | Its Section 5: R1 to R7 are built. Its Section 6 lists what is open for the operator. |
| [research/one-click/s3-database-tasks.md](../research/one-click/s3-database-tasks.md) | Task breakdown: bundled PostgreSQL | In progress | Approved by the operator on 2026-11-14. Its Section 7.5: T1 to T9 and T15 have commits; T10 is in progress; T11 to T14 wait for the operator or other hardware. Its parent plan is [one-click-plan.md](../one-click-plan.md). |
| [research/one-click/s4-local-hub-tasks.md](../research/one-click/s4-local-hub-tasks.md) | Task breakdown: the hub as a background program | In progress | Its Section 2: T1 to T8 have commits; T9 and T10 are not started. No operator ruling on its design. Its parent plan is [one-click-plan.md](../one-click-plan.md). |
| [research/one-click/s3-spec-delta.md](../research/one-click/s3-spec-delta.md) | Proposal: spec delta for the bundled PostgreSQL | Draft | Not approved. It changes no requirement until the operator approves it. |
| [research/one-click/s4-spec-delta.md](../research/one-click/s4-spec-delta.md) | Proposal: spec delta for the hub as a background program | Draft | Not approved. It changes no requirement until the operator approves it. |
| [research/one-click/s6-first-run-tasks.md](../research/one-click/s6-first-run-tasks.md) | Task breakdown: the first run | In progress | Its Section 3 holds the state of each task. Its parent plan is [one-click-plan.md](../one-click-plan.md). |
| [research/one-click/s6-spec-delta.md](../research/one-click/s6-spec-delta.md) | Proposal: spec delta for the first run | Draft | Not approved. It changes no requirement until the operator approves it. |
| [research/one-click/s8-stay-awake-tasks.md](../research/one-click/s8-stay-awake-tasks.md) | Task breakdown: stay awake | In progress | Its Section 5: T1 to T6 have commits. The live checks of its Section 6 wait for the operator. Its parent plan is [one-click-plan.md](../one-click-plan.md). |
| [research/one-click/s1-link-tasks.md](../research/one-click/s1-link-tasks.md) | Task breakdown: the link (`anima-link`) | In progress | T1 to T13 have commits on `main` (merge `f367f819`). T14, the live checks with a second Mac, waits for the operator. Its parent plan is [one-click-plan.md](../one-click-plan.md). |
| [research/one-click/s5-access-tasks.md](../research/one-click/s5-access-tasks.md) | Task breakdown: access rules by device key | In progress | T1 to T9 have commits on `main` (merge `f367f819`); T10, the agent's link origin, has its commit. Open: the K7 row of its Section 6.3 and one test of agent, link and hub together. Its parent plan is [one-click-plan.md](../one-click-plan.md). |
| [research/one-click/s12-packaging-tasks.md](../research/one-click/s12-packaging-tasks.md) | Task breakdown: packaging | In progress | Its Section 3: T1 to T8 have commits. The checks of its Section 6.3 wait for the operator or other hardware. Its parent plan is [one-click-plan.md](../one-click-plan.md). |
| [research/one-click/s12-spec-delta.md](../research/one-click/s12-spec-delta.md) | Proposal: spec delta for packaging | Draft | Not approved. It changes no requirement until the operator approves it. |
| [work-items/README.md](../work-items/README.md) | Task breakdown: the work items that came from the GitHub issues, with the state of each | In progress | R-119 (operator, 2026-11-16). 57 items: engine, extensions, compute, decisions, device manager and defects. Each item names the row of Section 4 that it serves. The states are the states of the issues at the import and are not checked against the code. |
| [programme/proposals/projects-and-builds.md](../programme/proposals/projects-and-builds.md) | Proposal, with the task table for builds and runners | In progress | Decided (OD-W11, OD-W12, OD-W13). Connect GitHub is on `main`. Version 2 (2026-11-16): OD-W14 to OD-W21 are decided; its status line names the requirements that still have no ruling. In its "Tasks", R1 to R4 are Done (commit `e3058276` on `main`). Each other task is Not started. |
| [programme/proposals/jobs.md](../programme/proposals/jobs.md) | Proposal | Draft | Direction decided (OD-W10). No requirement is approved. Version 2 (2026-11-16) adds JOB-011. Its tasks W1 to W6 are in the task table of the row above. |
| [programme/proposals/coordinator-standby.md](../programme/proposals/coordinator-standby.md) | Proposal | Draft | Direction decided (OD-W9). No requirement is approved. |
| [programme/proposals/bulk-data-placement.md](../programme/proposals/bulk-data-placement.md) | Proposal | Draft | Direction decided (OD-W9). No requirement is approved. |
| [programme/proposals/other-operators.md](../programme/proposals/other-operators.md) | Proposal | Draft | Direction decided (OD-W2). It has no requirements. |
| [programme/proposals/agent-runs.md](../programme/proposals/agent-runs.md) | Proposal | Draft | Not approved. |
| [programme/proposals/hosted-applications.md](../programme/proposals/hosted-applications.md) | Proposal | Draft | Not approved. |
| [programme/proposals/new-sources.md](../programme/proposals/new-sources.md) | Proposal | Draft | No provider is verified. |
| [programme/proposals/add-this-machine.md](../programme/proposals/add-this-machine.md) | Proposal | Draft. Build state not checked | Its status line: a proposal for a ruling. |
| [programme/proposals/first-run-setup.md](../programme/proposals/first-run-setup.md) | Proposal | Draft. Build state not checked | Its status line: a proposal for a ruling. |
| [programme/proposals/in-app-enrolment.md](../programme/proposals/in-app-enrolment.md) | Proposal | Draft. Build state not checked | Its status line: a proposal for a ruling. |
| [programme/proposals/ssh-terminal.md](../programme/proposals/ssh-terminal.md) | Proposal | Draft. Build state not checked | Its status line: a proposal for a ruling (R-062). |
| [programme/proposals/shared-pool-hosting.md](../programme/proposals/shared-pool-hosting.md) | Proposal | Draft | Not approved. Launch on the pool is decided (HM-R4, operator, 2026-11-16); the design is not. The AX102 list price was corrected on 2026-11-16 to €259 a month plus €129 setup (about $286). |
| [programme/proposals/shell-prompt.md](../programme/proposals/shell-prompt.md) | Proposal | Draft | Not approved. CMD-032 to CMD-038. INS-009 ruled 2026-11-16. |
| [programme/proposals/hosted-mode.md](../programme/proposals/hosted-mode.md) | Proposal | Draft | Direction decided (HM-R1 to HM-R6, operator, 2026-11-16). HMD-001 to HMD-029 are not approved. HM-R6: a free hub sleeps. |
| [programme/proposals/cloud-machines.md](../programme/proposals/cloud-machines.md) | Proposal | Draft | Not approved. CVM-001 to CVM-024. |
| [programme/proposals/machine-profiles.md](../programme/proposals/machine-profiles.md) | Proposal | Draft | Not approved. REP-001 to REP-025. MET-002, USE-006, INS-009 and VLT-007 ruled 2026-11-16. |
| [programme/proposals/cve-watch-and-network-awareness.md](../programme/proposals/cve-watch-and-network-awareness.md) | Proposal | Draft | Not approved. VUL-001 to VUL-028, ADR-001 to ADR-014. No tier in Section 4 yet. Operator rulings 2026-11-16 (R-121): the feeds and the match run once a day (VUL-006); Arch Linux and Fedora are in the first version (VUL-021 to VUL-024); an agentless SSH scan is an option (VUL-025 to VUL-028). macOS stays "Not covered". |

These documents are records, not plans. They must not add a requirement (PLN-009).

| Documents | Kind |
| --- | --- |
| [programme/programme.md](../programme/programme.md), [programme/handoff.md](../programme/handoff.md), [programme/operator-changes.md](../programme/operator-changes.md) | Rulings and the state of the programme |
| `programme/packages/` | Task breakdown and log of each work package of a phase plan |
| `programme/phase-2-*.md` and `programme/phase-3-*.md`, but not the two plans | Contracts, acceptance records and optional items of a phase |
| `research/`, but not the task breakdowns and spec delta proposals in `research/one-click/` that the register lists | Audits and reports |
| `setup/`, [releases.md](../releases.md), [programme/linux-client-setup.md](../programme/linux-client-setup.md) | Procedures |
| [need-human-testing.md](../need-human-testing.md) | The list of checks that only a person can do, with the state of each (PLN-014) |
| [security-findings.md](../security-findings.md) | The list of security findings, with the state of each |
| [deferred.md](../deferred.md) | The list of deferred work (R-119; it replaces GitHub issue #1 and R-007) |
| [ui-critique-2026-11-11.md](../ui-critique-2026-11-11.md), [ui-inventory-2026-11-11.md](../ui-inventory-2026-11-11.md) | Reviews of the interface |
