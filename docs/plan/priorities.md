# Categories, priority order and what is not planned

Part of the work plan: [work-plan.md](../work-plan.md) is the entry point, and its Sections 3, 4 and 6 are this document. A reference to "work plan Sections 3, 4 and 6" in a different document means this document.

| Item | Value |
| --- | --- |
| Document | The five categories, the one priority order, and the work that must not be built |
| Version | 2.0 |
| Date | 2026-11-17 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved text of the work plan, version 1.3. Version 2.0 only moved the text to this document (operator, 2026-11-16). |

## 3. Categories

| Category | Meaning | Content | Owner documents |
| --- | --- | --- | --- |
| Foundation | The parts that all other work uses | Coordinator, VPN between sites, `anima-daemon` on each machine, the clients, the standby copies, bulk data on the operator's machines, the update of the daemon and the hub | Design specification Sections 3, 4, 5 and 10. [vpn-plan.md](../plans/vpn-plan.md). [coordinator-standby.md](../proposals/coordinator-standby.md), [bulk-data-placement.md](../proposals/bulk-data-placement.md), [daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md) |
| See | Know the state of all that is yours | Checks, metrics, notifications, Projects (the front door for rented accounts), the Integrations section, the watchdog (it also restarts a program, which is Use) | Design specification Section 6. Extensions plan Sections 4.2 and 4.5. [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md), [watchdog.md](../proposals/watchdog.md) |
| Reach | Go into a thing and act on it | SSH, RDP, VNC, web interfaces, Secrets | Design specification Section 6.3. Extensions plan Section 4.4 |
| Remember | Keep a copy of the life of the operator | Health, Money, Map, People, History, Documents, Writing, Reminders, Library, Wardrobe, AI coding sessions | Extensions plan Sections 2 and 4. [new-sources.md](../proposals/new-sources.md) |
| Use | Make the machines do work | Model service, builds, jobs, AI agent runs, hosted applications | [compute-plan.md](../plans/compute-plan.md). [jobs.md](../proposals/jobs.md), [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md), [agent-runs.md](../proposals/agent-runs.md), [hosted-applications-plan.md](../plans/hosted-applications-plan.md) |

The command line and the MCP server are not a category. They are a second and a third way to use each function of each category ([cli-mcp-plan.md](../plans/cli-mcp-plan.md); design specification Section 6.10). They are built and are on `main`, except the rulings that the header of [cli-mcp-plan.md](../plans/cli-mcp-plan.md) names as not built (R-120). A new function is built in this order: the hub API, the `anima` command, the MCP tool, the graphical client (CMD-030).

## 4. Priority order

The tiers first came from the labels of the GitHub issues, read on 2026-11-12. The labels are no longer in use; this table holds the order (R-119). Do the work in this order. A lower row can start earlier only when it does not delay a higher row.

The table gives the order. The section of each row below the table gives the work, the work items and the full state. The column "Waits for" names who must act next. "Operator:" names a decision, an approval or a check. "Agent:" names one task with a link to the document that holds it; that document must be a plan or a work item, not a proposal (the gate checks this). A row with no "Agent:" task has no work for an agent: do not take a task from it. The column is not a ruling: before you build, read the task row and its plan, and stop if the task is not "Not started" or a decision blocks it. Change the column in the commit that changes the state (PLN-008). [operator-queue.md](../operator-queue.md) lists what waits for the operator.

| Row | Tier | Work | Category | State, in short | Waits for |
| --- | --- | --- | --- | --- | --- |
| 4.1 | 0 | Finish the device manager | Foundation, See, Reach | Phases 1 to 3 are built. | Operator: OD-W25 (what the acceptance run WP-53 is on a hosted hub; R-169 defers eight of its checks); the Linux release and the check that an installed application updates (R7); the check of the rename to `anima-daemon` (HT-018, with new steps for a hosted hub; HT-019 is deferred, R-169); the Linux checks of the defects GH-52 and GH-54 (HT-020, HT-021). Agent: nothing; GH-53 is corrected |
| 4.16 | 1 | One-click install | Foundation, Reach | In progress. Built; the live checks wait for the operator. | Operator: the live checks ([need-human-testing.md](../need-human-testing.md), "One-click") |
| 4.17 | 1 | The hosted mode and the shared pool | Foundation | In progress. The first real sign-in is Passed (HT-022, 2026-11-17). The computer as a device, the hub as a device, the check of the presence on the link and the TCP path from the hub to a computer are built and merged (HM-T8, HM-T9, HM-T11, HM-T12); none has its live check yet. | Operator: HT-027, HT-028, HT-029 and HT-031 in [need-human-testing.md](../need-human-testing.md), which need a hub of the pool from a new hub image and an application with the new link program; the proposed requirement HMD-051 of [hosted-mode.md](../proposals/hosted-mode.md); HM-D2, HM-D8, HM-D11 and HM-D13. Agent: the tasks HM-T10 and HM-T13 to HM-T15 in Section 8 of [hosted-mode.md](../proposals/hosted-mode.md) |
| 4.18 | 1 | Update the daemon and the hub | Foundation | Plan, in progress (R-139). Tasks U1 and U4 are done. | Agent: task U5 in [daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md). Operator: one Linux machine and one Mac for task U3, the release server of OD-ROL-15 (R-158, task U12), and a release for each target for the tasks that need one |
| 4.2 | 1 | Restore a backup and move the coordinator | Foundation | Proposal. FND-006 is approved for a hub on a server (R-141); task F1 is built. The operator starts fresh on the hosted hub (R-140). | Nothing now. R-169 defers the check HT-024 for the self-host path: it is not necessary now. Task F1 stays built and not checked, and is not Done |
| 4.3 | 1 | Decisions and approvals for the foundation | All | Open | Operator: each decision and approval in the row. Agent: PE-C1, then PE-C2 of [planning-evidence-plan.md](../plans/planning-evidence-plan.md), read-only preparation (R-152) |
| 4.4 | 2a | The engine (E0) | See, Remember | In progress. | Agent: E0-5 T3 of [extensions-plan.md](../plans/extensions-plan.md) Section 5.1 ("Tasks of E0-5, E0-6 and E0-7"). Operator: OD-E23 (the noun of the operational key commands, E0-6 T3), HT-034 and HT-035. OD-E16 and OD-E17 stay open and block no task of E0-5 to E0-7. The launch briefs of XP-01 to XP-14 wait for E0-7 (R-156) |
| 4.5 | 2b | Level 1 model service | Use | Approved (R-165, 2026-11-18): Level 1 of the compute plan, with the tasks L1-1 to L1-21 of its Section 12.4. No task is started. The tool inventory is built. | Operator: no Level 1 task starts until the macOS application 0.2.2 is published and the hosted live checks HT-027, HT-028, HT-029 and HT-031 in [need-human-testing.md](../need-human-testing.md) have a result (the condition of R-165); then the missing values of OD-C1 after task L1-1, and OD-AI1 of [ai-direction.md](../proposals/ai-direction.md). Agent: nothing before the condition is met |
| 4.6 | 3a | Projects | See | In progress. | Nothing now. The parts that are not built wait for later plans: a hosting provider as a source (PRO-005, Q1), the Operations area of the project page (PRO-006, Q2), task W1 (PRO-006, Q3) and a second forge (PRO-007, Q4); see [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md), "Results of task P0" |
| 4.7 | 3a | Jobs, builds and AI agent runs | Use | [C7](../work-items/compute/C7.md) is planned. | Agent: task W1, the contract of the work route, in [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) (R-160 is the ruling that changes R-115). Operator: the acceptance of the contract (W1) and of the design (W2); the security review of the work route comes before code |
| 4.8 | 3b | The other extensions | See, Remember, Reach | Planned. The shared redaction crate of E0-3 is built (R-166). | Row 4.4 |
| 4.9 | 3c | Hosted applications | Use | Plan, in progress (R-149, R-150). Stage 1 is built: tasks H1 to H3 (the hub, the `anima` command and the MCP tool, the tab Containers of the device page). It is not checked on a real machine: task H4 waits for HT-032. H8 and H9 can be built now. Stage 2 waits for task W1 of row 4.7, which no longer waits for a ruling (R-160). | Operator: the check HT-032 on a machine that runs Docker. Agent: [hosted-applications-plan.md](../plans/hosted-applications-plan.md), task H8 (Not started, no claim) |
| 4.10 | 3 | Standby copies of the coordinator database | Foundation | Parked until after the hosted launch (R-161). The copy in the application on a computer (FND-007) is not parked: it is approved (R-157), and task F2 is built. | Operator: the check HT-030 in [need-human-testing.md](../need-human-testing.md). The standbys: the hosted launch. Nothing for an agent |
| 4.11 | 3 | Bulk data on the operator's machines | Foundation | Parked until after the hosted launch (R-161). | The hosted launch. Nothing for an agent. E0-5 does not wait for it (R-145) |
| 4.19 | 3 | The watchdog | See, Use | Proposal, direction decided (R-162). After jobs (row 4.7). | Operator: approval of the requirements WDG-001 to WDG-041. Agent: nothing; no ruling names a task |
| 4.20 | 3 | The terminal screen (`anima tui`) | See, Reach | Direction decided (R-168). The requirements are not approved. Starts after R8a to R8d (row 4.6); the measurement TU1 may run before. | Operator: OD-TUI6 and OD-TUI7 after TU1, then the approval of TUI-001 to TUI-083. Agent: task TU1 in [cli-mcp-plan.md](../plans/cli-mcp-plan.md) Section 11.8 (a record only); TU2 to TU10 wait |
| 4.12 | 4 | Level 2: one model across machines | Use | Parked. | Row 4.5 in daily use |
| 4.13 | 4 | A hash chain for the audit log | Foundation | The operator made the work item smaller on 2026-11-12 | Operator: HT-023 on the real hub. Agent: nothing |
| 4.14 | 4 | Mobile clients (Phase 5) and site operations (Phase 6) | Foundation, Reach | In progress (mobile). Planned (Phase 6). | Agent: M1-3 of [mobile-plan.md](../plans/mobile-plan.md) Section 5. No session holds its tasks in progress (claims deleted, 2026-11-18); each row says what is built. Phase 6 waits for rows 4.4 to 4.8 |
| 4.15 | — | Deferred items and ideas | All | Recorded, not planned | Nothing |

### 4.1 Finish the device manager

- Tier: 0
- Category: Foundation, See, Reach
- Work: Finish the device manager: the Phase 3 acceptance run (WP-53) and the operator steps before it. Three open defects ([GH-52](../work-items/defects/GH-52.md), [GH-53](../work-items/defects/GH-53.md), [GH-54](../work-items/defects/GH-54.md)). The correction of the release of Anima: tasks R1 to R7 in "Tasks" of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) (OD-W21).
- Work items: [handoff.md](../plans/phase-3/handoff.md) Sections 3 and 4. [GH-52](../work-items/defects/GH-52.md), [GH-53](../work-items/defects/GH-53.md), [GH-54](../work-items/defects/GH-54.md). R1 to R7 are in the task table of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md)
- State: Phases 1 to 3 are built. Acceptance waits for the operator. This tier removes `blocked: device-manager` (PH-E-000, PH-C-000). R1 to R4 are done and are on `main` (commit `e3058276`, 2026-11-15). R5 and R6 are done. R7, the first release, is in progress: the release record in [releases.md](../procedures/releases.md) holds what is published.

### 4.16 One-click install

- Tier: 1
- Category: Foundation, Reach
- Work: A person installs the application and installs nothing else ([one-click-plan.md](../plans/one-click/one-click-plan.md)). The link (S1), the native VNC and RDP gateway (S2), the bundled PostgreSQL (S3), the hub as a background program (S4), the access rules by device key (S5), the first run (S6), the stay-awake control (S8) and packaging (S12).
- Work items: The task breakdowns hold the tasks: [s1-link-tasks.md](../plans/one-click/s1-link-tasks.md), [s2-vnc-tasks.md](../plans/one-click/s2-vnc-tasks.md), [s2-rdp-tasks.md](../plans/one-click/s2-rdp-tasks.md), [s3-database-tasks.md](../plans/one-click/s3-database-tasks.md), [s4-local-hub-tasks.md](../plans/one-click/s4-local-hub-tasks.md), [s5-access-tasks.md](../plans/one-click/s5-access-tasks.md), [s6-first-run-tasks.md](../plans/one-click/s6-first-run-tasks.md), [s8-stay-awake-tasks.md](../plans/one-click/s8-stay-awake-tasks.md), [s12-packaging-tasks.md](../plans/one-click/s12-packaging-tasks.md)
- State: In progress. The operator put it here on 2026-11-16: directly after tier 0, before the engine. Most tasks have commits on `main`. The live checks of each breakdown wait for the operator. [register.md](register.md) holds the state of each breakdown.

### 4.17 The hosted mode and the shared pool

- Tier: 1
- Category: Foundation
- Work: Sign-in is the default first run and makes a hosted hub for the account. The self-host mode is the second choice (HM-R1; the operator confirmed it on 2026-11-16). The link on the server stack, the sleep of a free hub, the shared pool with its isolation and its security review ([hosted-mode.md](../proposals/hosted-mode.md), [shared-pool-hosting.md](../proposals/shared-pool-hosting.md)).
- Work items: The tasks HM-T1 to HM-T15 in Section 8 of [hosted-mode.md](../proposals/hosted-mode.md). No work item document yet
- State: In progress. The operator put it here on 2026-11-16: directly after tier 0, before the engine. The direction is decided (HM-R1 to HM-R9). The rulings R-127 to R-130 name the tasks HM-T1 to HM-T7 and approve HMD-037 to HMD-041 and HMD-043. HM-T1 to HM-T3, HM-T6 and HM-T7 are built and merged to `main` (2026-11-17); HM-T4 is built in the private repositories and its site half is deployed. No device has signed in with an account yet: each task waits for HT-022. R-163 (2026-11-18) approves the requirements HMD-001 to HMD-036 in their revised text; 16 of them are not built, and HMD-051 is proposed. R-159 (2026-11-18) decides the second sign-in factor (HM-D4: required before the first device with remote access) and the security review (HM-D7: none by a person from outside). Open: "Add this computer" for a computer of the link only (HM-D11), the form of the WorkOS requests (HM-D13), the usage line (HM-D2) and the numbers of the sleep (HM-D8). R-164 decides a free hub that is not used (HM-D15: it sleeps, and it is deleted after 60 days with no sign-in). R-163 decides the meaning of "remote access" (HM-D14). The operator selected WorkOS AuthKit for the sign-in on 2026-11-16 (HM-R9); its requirements are HMD-032 to HMD-036. On 2026-11-17 the operator said that he will use only the hosted version and that his self-hosted hub can go away later (R-130); no row of this table has changed its tier for that.

### 4.18 Update the daemon and the hub

- Tier: 1
- Category: Foundation
- Work: Update the daemon and the hub on each machine with one press, and automatically: signed releases, a rollout in steps, a rollback, machines that are not connected (ROL-001 to ROL-052)
- Work items: No work item yet. The tasks are in "Tasks" of [daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md)
- State: Plan, in progress. The operator put it here on 2026-11-16: directly after the first release (OD-W23), and approved stages A and B on 2026-11-17 (R-139). Stage C (the hub of a server) is deferred: the pool updates a hosted hub (R-130). Task U4 (the contract) is done. Task U1 (the daemon reports its version, R-122) is done. Task U3 needs no release. The other tasks need a release for their target (the release record in [releases.md](../procedures/releases.md)), new files from the builder and a change to K8.

### 4.2 Restore a backup and move the coordinator

- Tier: 1
- Category: Foundation
- Work: The procedure to restore a backup on a new machine and to move the coordinator (FND-006)
- Work items: No work item yet. The requirement is in [coordinator-standby.md](../proposals/coordinator-standby.md), which is a draft
- State: Proposal. The operator put it here on 2026-11-12: directly after tier 0. R-141 approves FND-006 for a hub on a server; task F1 of [coordinator-standby.md](../proposals/coordinator-standby.md) is built and waits for HT-024. R-169 (operator, 2026-11-18: "Approve the sorting") defers HT-024 for the self-host path. That is the ruling that this row asked for: the check is not necessary now. F1 is not Done (PLN-009); the check goes back to Open when a hub on a server is in work again.

### 4.3 Decisions and approvals for the foundation

- Tier: 1
- Category: All
- Work: Decisions and approvals that the foundation work needs
- Work items: Extensions: [SPEC](../work-items/extensions/SPEC.md), [KEYS](../work-items/extensions/KEYS.md). Compute: [SPEC-C](../work-items/compute/SPEC-C.md), [KEYS-C](../work-items/compute/KEYS-C.md), [OD-C1](../work-items/decisions/OD-C1.md), [OD-C2](../work-items/decisions/OD-C2.md), [OD-C5](../work-items/decisions/OD-C5.md), [OD-C7](../work-items/decisions/OD-C7.md), [OD-C8](../work-items/decisions/OD-C8.md). This plan: Section 7, where OD-W22 is open
- State: Open

### 4.4 The engine (E0)

- Tier: 2a
- Category: See, Remember
- Work: The engine (E0), with Writing as its sample extension
- Work items: [E0-1](../work-items/engine/E0-1.md), [E0-2](../work-items/engine/E0-2.md), [E0-4](../work-items/engine/E0-4.md), [E0-5](../work-items/engine/E0-5.md), [E0-6](../work-items/engine/E0-6.md), [E0-7](../work-items/engine/E0-7.md), [E0-8](../work-items/engine/E0-8.md), [XP-03](../work-items/extensions/XP-03.md)
- State: In progress. On `main` (2026-11-13): E0-1, the hub folder regroup (merge `78df5bcb`); E0-2, the sources framework and Settings > Integrations (merges `c70a3e32`, `8c98653d`); E0-8, the extension registry and its switches (merges `3750eb87`, `efcf2fe5`). The launch briefs of E0-8 are not written (ENG-012). On `main` in part since the merge of the branch `engine/e0-5-places` (2026-11-18): E0-5 (places, the file store and extraction proposals with their commands and MCP tools) and E0-6 (the operational keys of the hub and the key cache of the daemon), and the contracts K-E4 Sections 8 to 13, K-E5 and K-E6; the task table of the extensions plan says what is not built. Not on `main`: E0-3, E0-7 after its contract and Writing. E0-4 is on `main` since 2026-11-17 (merge of the branch `engine/e0-4-timeline-links`): the hub, the `anima timeline` and `anima record-link` commands and their MCP tools. Its task T7, the client component, is later work (extensions plan, "Tasks of E0-4"). The index of the work items holds the state of E0-1, E0-2 and E0-8 ([work-items/](../work-items/README.md)).

### 4.5 Level 1 model service

- Tier: 2b
- Category: Use
- Work: Level 1 model service: C1 and C2
- Work items: [C1](../work-items/compute/C1.md), [C2](../work-items/compute/C2.md)
- State: Approved (R-165, operator, 2026-11-18), not started. The compute plan owns the requirements and the tasks L1-1 to L1-21 (its Section 12.4); version 0.3 took in the scope of 2026-11-07 of the two work items, R-131 and the link. Condition of R-165: no task of Level 1 starts until the macOS application 0.2.2 is published and the hosted live checks HT-027, HT-028, HT-029 and HT-031 have a result. When it is met, the first task is L1-1. The record of the comparison is [plan-reconcile-2026-11-18.md](../research/compute/plan-reconcile-2026-11-18.md).

### 4.6 Projects

- Tier: 3a
- Category: See
- Work: Projects: import each GitHub repository, find where it runs, propose resources (PRO-001 to PRO-008)
- Work items: [XP-02](../work-items/extensions/XP-02.md)
- State: In progress. On `main` (2026-11-13): the Projects domain in the hub, the `anima project` commands, the MCP tools, and the list and detail pages of the client (PRO-001 to PRO-004; merges `566238d2`, `3f9e9536`, `cf6d59bb`, `cc0490bf`). Connect GitHub (PRJ-039 to PRJ-042) is on `main` (2026-11-14). PRO-005 to PRO-007 were compared with the code (task P0, 2026-11-17): PRO-005 is not built, PRO-006 is partly built and PRO-007 is built differently; the results and the decisions Q1 to Q4 (operator, 2026-11-17) are in the plan. PRO-008 was not compared with the code. The direction is decided (OD-W11). It changes PRJ-001. The parts of the engine that it needs are on `main` (tier 2a).

### 4.7 Jobs, builds and AI agent runs

- Tier: 3a
- Category: Use
- Work: Jobs, with the build as the first job type. Then AI agent runs. Anima has two build models: its own builds, and owned machines as runners of GitHub Actions (PRO-010, PRO-027, OD-W13).
- Work items: [C7](../work-items/compute/C7.md), [C3](../work-items/compute/C3.md). No work item yet for the proposals
- State: [C7](../work-items/compute/C7.md) is planned. R-160 (operator, 2026-11-18: "A: on everywhere") is the ruling that changes R-115: the hub can send work to the daemon, on by default on each hub, a hosted hub included. Task W1 no longer waits for a ruling; the contract and the security review come before code. The operator put jobs before the extensions (OD-W8) and the build as a job directly after Projects (PRO-009, OD-W12). A Mac build is a native job (PRO-011). The tasks are planned and not started: groups W, B and A in "Tasks" of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) (2026-11-16). Their order is W, then B, then A (OD-W21).

### 4.8 The other extensions

- Tier: 3b
- Category: See, Remember, Reach
- Work: The other extensions, in the order of XP-005, one at a time
- Work items: [XP-01](../work-items/extensions/XP-01.md), [XP-04](../work-items/extensions/XP-04.md) to [XP-14](../work-items/extensions/XP-14.md), [E0-3](../work-items/engine/E0-3.md), [SEAMS](../work-items/extensions/SEAMS.md). Decisions [OD-E4](../work-items/decisions/OD-E4.md), [OD-E10](../work-items/decisions/OD-E10.md), [OD-E3](../work-items/decisions/OD-E3.md)
- State: Planned. One part is built: the shared redaction crate, tasks T1 and T2 of E0-3 (R-166, 2026-11-18, commit `f03429d1`); their rows in [extensions-plan.md](../plans/extensions-plan.md) Section 5.1 hold the state. The collector of E0-3 (T3, T4) and the other extensions stay later.

### 4.9 Hosted applications

- Tier: 3c
- Category: Use
- Work: Hosted applications
- Work items: none. The tasks H0 to H12 are in [hosted-applications-plan.md](../plans/hosted-applications-plan.md)
- State: Plan, in progress (R-149, 2026-11-18). Stage 1 (see the containers of a machine) is built: tasks H1 to H3. It waits for the check HT-032 on a real machine (task H4). Stage 2 (control, install, the catalogue) waits for task W1 of row 4.7; the ruling that W1 waited for is R-160. The tier is not changed.

### 4.10 Standby copies of the coordinator database

- Tier: 3
- Category: Foundation
- Work: Standby copies of the coordinator database (FND-001 to FND-005, FND-007 to FND-012)
- Work items: No work item yet
- State: Parked until after the hosted launch (R-161, operator, 2026-11-18: "B: park both"). No requirement of the standbys is approved. Its direction is decided (OD-W9). FND-006 is row 4.2. FND-007 is not part of the parking: it is approved for the application on a computer (R-157), and task F2 is built and waits for HT-030.

### 4.11 Bulk data on the operator's machines

- Tier: 3
- Category: Foundation
- Work: Bulk data in encrypted pieces on the operator's machines
- Work items: No work item yet. It changes the file store of [E0-5](../work-items/engine/E0-5.md)
- State: Parked until after the hosted launch (R-161, operator, 2026-11-18: "B: park both"). No requirement is approved. Its direction is decided (OD-W9). Open, a note of the orchestrating session: no document says what happens when the 20 GiB disk of a hosted hub is full after the file store of E0-5 is in use. E0-5 builds the file store in a folder of the hub and does not wait for it (R-145).

### 4.12 Level 2: one model across machines

- Tier: 4
- Category: Use
- Work: Level 2: one model across machines (C4 to C6)
- Work items: [C4](../work-items/compute/C4.md), [C5](../work-items/compute/C5.md), [C6](../work-items/compute/C6.md). Decisions [OD-C3](../work-items/decisions/OD-C3.md), [OD-C4](../work-items/decisions/OD-C4.md)
- State: Parked. Start only after Level 1 is in daily use (CMP-004, PH-C-001).

### 4.13 A hash chain for the audit log

- Tier: 4
- Category: Foundation
- Work: A hash chain for the audit log, so that a change to an old entry shows
- Work items: [GH-2](../work-items/device-manager/GH-2.md)
- State: The operator made the work item smaller on 2026-11-12: only its phase L0 stays. Its phases L1 to L5 are withdrawn (OD-W9). The operator asked for the build of L0 on 2026-11-17 (R-138). The operator approved the requirements LOG-021 to LOG-028 (design specification Section 3.8.2) the same day.

### 4.14 Mobile clients (Phase 5) and site operations (Phase 6)

- Tier: 4
- Category: Foundation, Reach
- Work: iOS client and Android client (Phase 5). Site operations and traffic data (Phase 6)
- Work items: Design specification Section 9. [GH-55](../work-items/device-manager/GH-55.md). [mobile-plan.md](../plans/mobile-plan.md)
- State: In progress for the mobile clients ([mobile-plan.md](../plans/mobile-plan.md) Section 5 holds each task). Planned for Phase 6. [GH-55](../work-items/device-manager/GH-55.md) puts Phase 6 after the extension foundations and compute. The mobile clients start now, beside tier 0 (operator, 2026-11-16, OD-M2 of the mobile plan).

### 4.15 Deferred items and ideas

- Tier: —
- Category: All
- Work: Deferred items and ideas from other products
- Work items: [deferred.md](../deferred.md)
- State: Recorded, not planned

### 4.19 The watchdog

- Tier: 3, after jobs (R-162)
- Category: See, Use
- Work: The watchdog: `anima-daemon` restarts the programs that the operator marked, and reports each restart (WDG-001 to WDG-041)
- Work items: No work item yet. The tasks are in "Tasks" of [watchdog.md](../proposals/watchdog.md)
- State: Proposal, direction decided (R-162, operator, 2026-11-18: "A: approve, tier 3"). The decisions OD-WDG-1 to OD-WDG-13 take the recommendations of the proposal. The requirements WDG-001 to WDG-041 are not approved one by one, and no ruling names a task. R-160 is the ruling for OD-W10. It needs a change to K8; its second stage needs the work route (group W).

### 4.20 The terminal screen

- Tier: 3, after R8a to R8d of row 4.6 (R-151). The operator decided the direction in R-168 (first-hand, 2026-11-18). Anima Product Ideas had relayed the first answers.
- Category: See, Reach
- Work: `anima tui`, a read-only full-screen view of the hub in a terminal (devices with live status, events, sessions) that hands the terminal to `anima ssh` (TUI-001 to TUI-083)
- Work items: No work item yet. The tasks TU1 to TU10 are in [cli-mcp-plan.md](../plans/cli-mcp-plan.md) Section 11.8
- State: Direction decided (R-168, operator, 2026-11-18: "OD-TUI1: yes, I confirm."). OD-TUI1 to OD-TUI5 and OD-TUI8 are decided. The requirements TUI-001 to TUI-083 are not approved. Task TU1 (a measurement, a record only) may run before R8d (OD-TUI8); TU2 to TU10 wait for TU1, R8d and the approval of the requirements. OD-TUI6 and OD-TUI7 are open until TU1. Windows is out of scope. The design specification Section 2 and Section 6.10 are changed.

### Notes on the order

Notes:

- Tiers 2a and 2b do not depend on each other. They can run at the same time.
- Each extension needs the engine (PH-E-000). The extensions are independent workflows (XP-004).
- The rows 4.16, 4.17 and 4.18 are tier 1 (operator, 2026-11-16). They come before the engine, the model service and the extensions. Their numbers follow 4.15 because a published number does not change.
- Row 4.19 is tier 3, after jobs (R-162). It keeps its number.
- Row 4.20 is tier 3, after R8a to R8d of row 4.6. It is a new row, so it follows 4.19. The operator chose a row of its own for the terminal screen (R-168). It is not a category: the command line and the MCP server stay a way to use each category.
- The states in this table were read on 2026-11-12, except where a row gives a later date. The rows of tier 0, tier 2a and Projects in tier 3a were compared with the commits on `main` on 2026-11-16.
- Phase 4 (Proxmox VE) is withdrawn (R-109). Session recording is deferred (R-113). This plan does not include them.

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
