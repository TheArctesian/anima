# Where each requirement lives

Part of the work plan: [work-plan.md](../work-plan.md) is the entry point, and its Section 5 is this document. A reference to "work plan Section 5" in a different document means this document.

| Item | Value |
| --- | --- |
| Document | The one owner document of each concern, with its identifiers |
| Version | 2.0 |
| Date | 2026-11-16 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved text of the work plan, version 1.3. Version 2.0 only moved the text to this document (operator, 2026-11-16). |

## 5. Where each requirement lives

One document owns each concern. Add a requirement to its owner, not to this document.

| Concern | Owner | Identifiers |
| --- | --- | --- |
| Device manager: hub, clients, sessions, checks, metrics, notifications | [design-spec.md](../spec/design-spec.md) | As published there |
| VPN and enrolment | [vpn-plan.md](../plans/vpn-plan.md) | As published there |
| One-click install, many hubs, shares and teams | [one-click-plan.md](../plans/one-click/one-click-plan.md) | `LNK`, `AWK`, `MHB`, `SHR`, `MBR` (proposed; not in the spec yet) |
| Power, hardware health and inventory | [power-health-plan.md](../plans/power-health-plan.md) | `PWR`, `HLT`, `HWI` (proposed; not in the spec yet) |
| Agent push | `contracts/agent-push.md` (K8), [phase-3-agent-delta.md](../plans/phase-3/phase-3-agent-delta.md) | MET-011 to MET-025 |
| Engine and extensions | [extensions-plan.md](../plans/extensions-plan.md) | ENG, SRC, COL, VLT, XP, PRJ and the others there |
| Model service and pooled models | [compute-plan.md](../plans/compute-plan.md), as changed by [C1](../work-items/compute/C1.md) and [C2](../work-items/compute/C2.md) | CMP, CAG, CAP, RTR and the others there |
| Copies of the coordinator database, and the move of the coordinator | [coordinator-standby.md](../proposals/coordinator-standby.md) | FND-001 to FND-012 |
| Bulk data on the operator's machines | [bulk-data-placement.md](../proposals/bulk-data-placement.md) | STO-001 to STO-011 |
| Rules for all work on machines, and jobs | [jobs.md](../proposals/jobs.md) | USE-001 to USE-009, JOB-001 to JOB-027 |
| AI agent runs | [agent-runs.md](../proposals/agent-runs.md) | AGN-001 to AGN-014 |
| Hosted applications | [hosted-applications.md](../proposals/hosted-applications.md) | HOST-001 to HOST-009 |
| Projects as the front door: import from a forge, resources that Anima finds, builds as jobs, owned machines as runners of GitHub Actions, sign-in with a forge | [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) | PRO-001 to PRO-053 |
| The `anima` command line and the MCP server | [cli-mcp-plan.md](../plans/cli-mcp-plan.md). The design specification Section 6.10 gives the summary. | CMD, MCP |
| Who did an action: the actor kinds User, Agent and Hub in the audit log and the event log | [design-spec.md](../spec/design-spec.md) Section 3.8.1 | LOG-012 to LOG-020 |
| Installation: the disk image and the AppImage, with the `anima` program inside | [design-spec.md](../spec/design-spec.md) Section 3.11 | INS-001 to INS-009 |
| The marketing website with the install links | [design-spec.md](../spec/design-spec.md) Section 3.12 | SITE-001 to SITE-012 |
| Stay awake: the desktop application keeps its computer on | [design-spec.md](../spec/design-spec.md) Section 3.13 | AWK-001 to AWK-008 |
| Cloudflare source for Projects. A new Activity extension for Spotify and Screen Time | [new-sources.md](../proposals/new-sources.md) | None until each provider is verified |
| Other persons: self-hosting and a hosted option | [other-operators.md](../proposals/other-operators.md) | None yet |
| Hosted Solo hubs on a shared pool of hosts | [shared-pool-hosting.md](../proposals/shared-pool-hosting.md) | None yet |
| The hosted mode as the default first run: sign-in, the account directory, a hub made for each account, a free hub that sleeps | [hosted-mode.md](../proposals/hosted-mode.md) | HM-R1 to HM-R8, HMD-001 to HMD-031 |
| The Anima.ing shell prompt and related shell commands | [shell-prompt.md](../proposals/shell-prompt.md) | CMD-032 to CMD-038 |
| Rented cloud computers and GPU add-ons sold by Anima.ing | [cloud-machines.md](../proposals/cloud-machines.md) | CVM-001 to CVM-024 |
| Machine profiles: copy a setup to a new computer | [machine-profiles.md](../proposals/machine-profiles.md) | REP-001 to REP-025 |
| CVE watch of the updates that wait on each machine, and network awareness (the address classes of a device, other overlay networks, address changes) | [cve-watch-and-network-awareness.md](../proposals/cve-watch-and-network-awareness.md) | VUL-001 to VUL-028, ADR-001 to ADR-014 |
| The watchdog: watched programs of a machine, their restarts and the reports | [watchdog.md](../proposals/watchdog.md) | WDG-001 to WDG-041 |
| The update of the agent and the hub: the release record, the rollout, the rollback, the automatic update. The update of the desktop application stays in the design specification Section 3.9 (UPD). | [agent-and-hub-update.md](../proposals/agent-and-hub-update.md) | ROL-001 to ROL-052 |
| Rulings and the state of each package | [rulings.md](../rulings.md), [handoff.md](../plans/phase-3/handoff.md) | R, CR |

The proposals are drafts (Section 9 lists them). A proposal becomes a plan when the operator approves it with a ruling. Then its work gets a task table in the plan, or a work item in [work-items/](../work-items/README.md), and a row in Section 4 (PLN-012).
