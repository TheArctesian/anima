# Anima Work Plan

| Item | Value |
| --- | --- |
| Document | Work plan: purpose, categories and priority order |
| Version | 1.0 |
| Date | 2026-11-13 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved. The operator approved the principles and the priority order on 2026-11-13. The proposals stay drafts until each has a ruling (Section 5). |
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

The three types stay different. A project can run on owned compute and on rented compute. A service that holds data about the operator is a source only: nothing runs there, and a project never lists it as its hosting (OD-W11, PRO-006).

Anima is for the operator first. Later, other persons can run their own Anima on a service of their choice, or pay for a hosted one. Each person has their own coordinator and their own data (OD-W2, [other-operators.md](programme/proposals/other-operators.md)).

## 2. Principles

The coordinator is the hub of the design specification. The interface, the specification and the code all say "hub" (OD-W3).

- **ETH-001. One home.** Each person has one coordinator.
- **ETH-002. The coordinator is portable.** It can run on a VPS, at home, or on a host that a service operates. No code must assume one provider.
- **ETH-003. One home, copies by policy.** Each set of data has one home. A policy places its copies on other machines. The operator does not select the machines. The live database has at least two standby copies. Bulk data has at least two copies of each encrypted piece.
- **ETH-004. The operator confirms a new home.** Anima finds a failure and selects the best copy. A copy becomes the home only after the operator confirms. Anima must not elect a new home alone.
- **ETH-005. Spread the work, not the state.** Work that holds no data can run on any machine. Only the home accepts writes. A machine that has no key can hold only encrypted pieces.
- **ETH-006. Own first.** When owned and rented resources can do the same work, Anima shows the owned resource first.
- **ETH-007. Control follows ownership.** Anima can change an owned resource: a machine with `anima-agent` takes work from the coordinator by default, and the operator can set a machine off (USE-004). For a rented resource, the hub only reads; an action uses the CLI tool of the service, on the operator's computer or in a job (OD-W1). Anima only reads a resource that is held about the operator.
- **ETH-008. Private by construction.** The coordinator is not on the public internet (SEC-008). A source uses read-only access unless ETH-007 permits more.
- **ETH-009. Build for use.** Build only for the services and devices that the operator uses (EXT-000, CMP-001). A different person must be able to add a source without a change to the engine.

## 3. Categories

| Category | Meaning | Content | Owner documents |
| --- | --- | --- | --- |
| Foundation | The parts that all other work uses | Coordinator, VPN between sites, `anima-agent` on each machine, the clients, the standby copies, bulk data on the operator's machines | Design specification Sections 3, 4, 5 and 10. [vpn-plan.md](vpn-plan.md). [coordinator-standby.md](programme/proposals/coordinator-standby.md), [bulk-data-placement.md](programme/proposals/bulk-data-placement.md) |
| See | Know the state of all that is yours | Checks, metrics, notifications, Projects (the front door for rented accounts), the Integrations section | Design specification Section 6. Extensions plan Sections 4.2 and 4.5. [projects-and-builds.md](programme/proposals/projects-and-builds.md) |
| Reach | Go into a thing and act on it | SSH, RDP, VNC, web interfaces, Secrets | Design specification Section 6.3. Extensions plan Section 4.4 |
| Remember | Keep a copy of the life of the operator | Health, Money, Map, People, History, Documents, Writing, Reminders, Library, Wardrobe, AI coding sessions | Extensions plan Sections 2 and 4. [new-sources.md](programme/proposals/new-sources.md) |
| Use | Make the machines do work | Model service, builds, jobs, AI agent runs, hosted applications | [compute-plan.md](compute-plan.md). [jobs.md](programme/proposals/jobs.md), [projects-and-builds.md](programme/proposals/projects-and-builds.md), [agent-runs.md](programme/proposals/agent-runs.md), [hosted-applications.md](programme/proposals/hosted-applications.md) |

The command line and the MCP server are not a category. They are a second and a third way to use each function of each category ([cli-mcp-plan.md](cli-mcp-plan.md); design specification Section 6.10). They are built and are on `main`. A new function is built in this order: the hub API, the `anima` command, the MCP tool, the graphical client (CMD-030).

## 4. Priority order

The tiers come from the issue labels `priority: foundation`, `priority: later` and `blocked: device-manager`, read on 2026-11-12. Do the work in this order. A lower row can start earlier only when it does not delay a higher row.

| Tier | Category | Work | Issues | State |
| --- | --- | --- | --- | --- |
| 0 | Foundation, See, Reach | Finish the device manager: the Phase 3 acceptance run (WP-53) and the operator steps before it. Three open defects. | [handoff.md](programme/handoff.md) Sections 3 and 4. #52, #53, #54 | Phases 1 to 3 are built. Acceptance waits for the operator. This tier removes `blocked: device-manager` (PH-E-000, PH-C-000). |
| 1 | Foundation | The procedure to restore a backup on a new machine and to move the coordinator (FND-006) | No issue yet | Proposal. The operator put it here on 2026-11-12: directly after tier 0. |
| 1 | All | Decisions and approvals that the foundation work needs | Extensions: #26, #27, #31. Compute: #41, #42, #43, #44, #47, #49, #50. This plan: Section 7 | Open |
| 2a | See, Remember | The engine (E0), with Writing as its sample extension | #4, #5, #7, #8, #9, #10, #11, #14 | Planned. `priority: foundation` |
| 2b | Use | Level 1 model service: C1 and C2 | #34, #35 | Planned. `priority: foundation`. The issues were given a new scope on 2026-11-07 and are newer than the compute plan. |
| 3a | See | Projects: import each GitHub repository, find where it runs, propose resources (PRO-001 to PRO-008) | #13 | The direction is decided (OD-W11). It changes PRJ-001. It needs the engine (tier 2a). |
| 3a | Use | Jobs, with the build as the first job type. Then AI agent runs. | #40 (C7), #36 (C3). No issue yet for the proposals | #40 is planned. The operator put jobs before the extensions (OD-W8) and the build as a job directly after Projects (PRO-009, OD-W12). A Mac build is a native job (PRO-011). |
| 3b | See, Remember, Reach | The other extensions, in the order of XP-005, one at a time | #12, #15 to #25, #6, #32. Decisions #28, #29, #30 | Planned. `priority: later` |
| 3c | Use | Hosted applications | No issue yet | Proposal. Not approved. |
| 3 | Foundation | Standby copies of the coordinator database (FND-001 to FND-005, FND-007 to FND-012) | No issue yet | Proposal. Its direction is decided (OD-W9). |
| 3 | Foundation | Bulk data in encrypted pieces on the operator's machines | No issue yet. It changes the file store of #8 | Proposal. Its direction is decided (OD-W9). Decide it before E0-5 (#8) starts. |
| 4 | Use | Level 2: one model across machines (C4 to C6) | #37, #38, #39. Decisions #45, #46 | Parked. Start only after Level 1 is in daily use (CMP-004, PH-C-001). |
| 4 | Foundation | A hash chain for the audit log, so that a change to an old entry shows | #2 | The operator made the issue smaller on 2026-11-12: only its phase L0 stays. Its phases L1 to L5 are withdrawn (OD-W9). |
| 4 | Foundation, Reach | iOS client (Phase 5). Site operations and traffic data (Phase 6) | Design specification Section 9. #55 | Planned. #55 puts Phase 6 after the extension foundations and compute. |
| — | All | Deferred items and ideas from other products | #1 | Recorded, not planned |

Notes:

- Tiers 2a and 2b do not depend on each other. They can run at the same time.
- Each extension needs the engine (PH-E-000). The extensions are independent workflows (XP-004).
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
| Rules for all work on machines, and jobs | [jobs.md](programme/proposals/jobs.md) | USE-001 to USE-009, JOB-001 to JOB-010 |
| AI agent runs | [agent-runs.md](programme/proposals/agent-runs.md) | AGN-001 to AGN-008 |
| Hosted applications | [hosted-applications.md](programme/proposals/hosted-applications.md) | HOST-001 to HOST-009 |
| Projects as the front door: import from a forge, resources that Anima finds, builds as jobs, sign-in with a forge | [projects-and-builds.md](programme/proposals/projects-and-builds.md) | PRO-001 to PRO-024 |
| The `anima` command line and the MCP server | [cli-mcp-plan.md](cli-mcp-plan.md). The design specification Section 6.10 gives the summary. | CMD, MCP |
| Who did an action: the actor kinds User, Agent and Hub in the audit log and the event log | [design-spec.md](design-spec.md) Section 3.8.1 | LOG-012 to LOG-020 |
| Installation: the disk image and the AppImage, with the `anima` program inside | [design-spec.md](design-spec.md) Section 3.11 | INS-001 to INS-009 |
| The marketing website with the install links | [design-spec.md](design-spec.md) Section 3.12 | SITE-001 to SITE-012 |
| Cloudflare source for Projects. A new Activity extension for Spotify and Screen Time | [new-sources.md](programme/proposals/new-sources.md) | None until each provider is verified |
| Other persons: self-hosting and a hosted option | [other-operators.md](programme/proposals/other-operators.md) | None yet |
| Rulings and the state of each package | [programme/programme.md](programme/programme.md), [programme/handoff.md](programme/handoff.md) | R, CR |

The eight proposals are drafts. A proposal becomes a plan when the operator approves it with a ruling. Then its work gets an issue with the labels of Section 4.

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
| OD-W1 | Can Anima change a rented resource (redeploy, purge a cache, restart the VPS)? | **Decided (operator, 2026-11-12): yes, with the CLI tool of each service, never from the hub.** The hub stays read-only (PRJ-003). First, the client runs the CLI on the operator's computer with `anima run` (SCR-006). When jobs exist, a CLI can also run as a job. It changes PRJ-010 ("changes happen there, not in Anima"). **Exception (operator, 2026-11-13): GitHub.** The hub makes changes on GitHub through the GitHub App of the operator, each change a named function that is in the audit log (PRJ-017, PRJ-027 to PRJ-029). A different rented service can get the same later, with one ruling for each service. Until then this decision applies to it. |
| OD-W2 | How do other persons use Anima? | **Decided (operator, 2026-11-12): both. A person can host Anima on a service of their choice, and a paid hosted option runs it for them.** [other-operators.md](programme/proposals/other-operators.md) has the operator's words and what this changes. Nothing is built for it now. |
| OD-W3 | The name of the hub in the interface | **Decided (operator, 2026-11-12): keep "hub".** |
| OD-W4 | How do commands get to the agent? | Not a decision of this plan. It is open in #34, #41 and #44. OD-W10 gives its direction: the agent asks the hub for work and listens on no port (USE-005, MET-020). |
| OD-W5 | The runtime for functions that answer a request | **Decided (operator, 2026-11-12): later, as containers that stay warm.** No WebAssembly runtime. It comes after jobs (JOB-006). |
| OD-W6 | The builder for a job image | **Decided (operator, 2026-11-12): keep more than one way.** A job image can come from Railpack, from a Dockerfile or from a Nix flake (JOB-002, JOB-003, JOB-008). Test Railpack later. |
| OD-W7 | The compute plan says that the agent is optional and that metrics use SSH | Not a decision of this plan. #41 and #44 record that R-084 replaces that text. |
| OD-W8 | The position of Use against the extensions in tier 3 | **Decided (operator, 2026-11-12): jobs and AI agent runs first, then the extensions one at a time, then hosted applications.** |
| OD-W9 | One home with copies, or a copy on each node that syncs between nodes when the hub is down (issue #2) | **Decided (operator, 2026-11-12): one home, copies by policy.** The section "Decision" of [coordinator-standby.md](programme/proposals/coordinator-standby.md) has the operator's words. It needs a ruling number in `programme.md`. Issue #2 keeps only its phase L0. |
| OD-W10 | Can the agent do work that the coordinator gives it? R-115 did not take a command channel to the agent. | **Decided (operator, 2026-11-12): yes, one agent, on by default ("it should be on by default").** The section "Decision" of [jobs.md](programme/proposals/jobs.md) has the limits. It needs a ruling number that changes R-115. It also changes CMP-002 and the recommendation of OD-C8 (#50). An agent that is already installed takes work after its update, with no confirmation. |
| OD-W11 | Are projects the front door for rented accounts, and does the builder build each project? | **Decided (operator, 2026-11-13): yes.** Connect GitHub, import each repository, leave out the ones that the operator selects. Anima proposes resources from signs in the repository. The builder is no longer specific to Anima. The three resource types stay different. A different forge must be possible. [projects-and-builds.md](programme/proposals/projects-and-builds.md) has the six points. |
| OD-W12 | The proposals that follow OD-W11 | **Decided (operator, 2026-11-13).** A build is the first job type and comes directly after Projects (PRO-009). Anima schedules builds from a webhook of the GitHub App, through the webhook receiver of the engine (WHK-001 to WHK-010), with a slow poll that repairs a lost webhook. Anima can also start a workflow of GitHub Actions; a machine is not a runner (PRO-010, PRO-020, PRO-021). The receiver runs on the VPS as a separate program (OD-E15); to open its port needs the confirmation of the operator (VPS-005). A Mac build is a native job that runs as a build user, in a new checkout that is deleted, on a Mac that the operator set on for it (PRO-011, PRO-022 to PRO-024). Keys belong to the project (PRO-012). No builds of pull requests from forks (PRO-013). Sign-in with a forge comes later, with the local sign-in as the root (PRO-015, PRO-016). PRJ-017 replaces "read access first" (PRO-014). |

## 8. Changes that this plan needs in other documents

These changes are not made. The operator must approve each one (operator checkpoint 3).

| Document | Change |
| --- | --- |
| Design specification 1.1 | Add the purpose line and the three resource types of Section 1. Issues #26 and #41 already change this section. |
| Design specification 4.2 and OD-9 | Call the Hostinger VPS the reference setup (ETH-002). |
| Design specification 9 | Add a pointer to the priority order of this plan. |
| Extensions plan 4.2 and 9 | Add Cloudflare after verification ([new-sources.md](programme/proposals/new-sources.md)). |
| Extensions plan 4 and 9 | Add Spotify and Screen Time after verification ([new-sources.md](programme/proposals/new-sources.md)). |
| Extensions plan PRJ-010 and Section 4.4 | Permit an action on a rented service through its CLI tool with `anima run` (OD-W1). |
| Extensions plan 1.1 and EXT-000, design specification 1.4 | Anima is no longer for one person only (OD-W2). The operator approved the new wording of Section 1.1 on 2026-11-13: three layers (foundation, extensions, community extensions later), one operator for each hub. The wording is on the branch `claude/spec-cli-distribution`, not yet on `main`. |
| Extensions plan FIL-001 | Keep files on the operator's machines, not in a volume on the hub ([bulk-data-placement.md](programme/proposals/bulk-data-placement.md)). |
| Compute plan CMP-002 and OD-C8 | A machine with the agent takes work by default (OD-W10). Issue #50 has the old recommendation. |
| Compute plan 3.3, 12.1 and 13 | Agree with R-084 and with the new scope of #34 and #35 (#41). Make C7 the second job type of [jobs.md](programme/proposals/jobs.md), after the build (OD-W12). |
| Extensions plan PRJ-001 and Section 4.2 | Import each repository; the operator leaves some out (OD-W11). PRJ-001 says that the operator accepts each project. OD-W11 is the ruling; PRJ-001 takes the new wording. |
| [jobs.md](programme/proposals/jobs.md) JOB-001, JOB-004, USE-001 | Changed on 2026-11-13 (OD-W12): a job is a container job or a native job on a Mac, and the build is the first job type. |
| Design specification BLD-001 to BLD-082 | The builder takes a project as a parameter; the release of Anima becomes one project (OD-W11). |
| `AGENTS.md`, [cli-mcp-plan.md](cli-mcp-plan.md) | Done on the branch `claude/anima-cli`: the command-line-first rule (CMD-030). |
| Design specification 6.10, 7, 9 and 13 | Done (operator, 2026-11-13): the command line and the MCP server are part of the specification. SET-004 names the "MCP" section. |
| Design specification 3.8.1 | Done as text (operator, 2026-11-13): the actor kinds User, Agent and Hub (LOG-012 to LOG-020). Not built. |
| `contracts/schema.graphql` (K1), `contracts/database.md` (K2), `contracts/conventions.md` (K7) | Not made: the additions for the actor kinds, in the table of design specification Section 3.8.1. They need a programme ruling (CON-001, design specification OD-15). |
| Design specification 3.11 | Done as text (operator, 2026-11-13): installation from the disk image or the AppImage, with the `anima` program inside (INS-001 to INS-009). INS-002 to INS-007 are built (branch `fix/bundle-anima`); a real release must confirm OD-16 to OD-18. |
| Design specification 3.12 and 11 | Done as text (operator, 2026-11-13): the marketing website (SITE-001 to SITE-012). Not built. The operator must decide OD-20, OD-21 and OD-22 of the design specification. |
| [cli-mcp-plan.md](cli-mcp-plan.md) Sections 6 and 7 | Not made: `anima key create` takes "made for"; `anima audit list` gets the actor column and `--actor`; `anima mcp` sends the agent declaration (LOG-015, LOG-016, LOG-020). Make them with the build, after the ruling. |
