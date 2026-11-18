# Changes that the work plan needs in other documents

Part of the work plan: [work-plan.md](../work-plan.md) is the entry point, and its Section 8 is this document. A reference to "work plan Section 8" in a different document means this document.

| Item | Value |
| --- | --- |
| Document | Each change to a different document that the work plan needs, and if it is made |
| Version | 2.0 |
| Date | 2026-11-16 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved text of the work plan, version 1.3. Version 2.0 only moved the text to this document (operator, 2026-11-16). |

## 8. Changes that this plan needs in other documents

Each row gives the state of one change. A row that says "Not made" needs the approval of the operator (operator checkpoint 3).

| Document | Change |
| --- | --- |
| Design specification 1.1 | Add the purpose line and the three resource types of Section 1. [SPEC](../work-items/extensions/SPEC.md) and [SPEC-C](../work-items/compute/SPEC-C.md) already change this section. |
| Design specification 4.2 and OD-9 | Call the Hostinger VPS the reference setup (ETH-002). |
| Design specification 9 | Add a pointer to the priority order of this plan. |
| Extensions plan 4.2 and 9 | Add Cloudflare after verification ([new-sources.md](../proposals/new-sources.md)). |
| Extensions plan 4 and 9 | Add Spotify and Screen Time after verification ([new-sources.md](../proposals/new-sources.md)). |
| Extensions plan PRJ-010 and Section 4.4 | Permit an action on a rented service through its CLI tool with `anima run` (OD-W1). |
| Extensions plan 1.1 and EXT-000, design specification 1.4 | Anima is no longer for one person only (OD-W2). The operator approved the new wording of Section 1.1 on 2026-11-13: three layers (foundation, extensions, community extensions later), one operator for each hub. The wording is in Section 1.1 of the extensions plan. |
| Extensions plan FIL-001 | Keep files on the operator's machines, not in a volume on the hub ([bulk-data-placement.md](../proposals/bulk-data-placement.md)). |
| Compute plan CMP-002 and OD-C8 | A machine with the daemon takes work by default (OD-W10). [OD-C8](../work-items/decisions/OD-C8.md) has the old recommendation. Version 0.3 of the compute plan (2026-11-18) asks OD-C8 as this question; CMP-002 changes when the operator answers it. |
| Compute plan 3.3, 12.1 and 13 | Agree with R-084 and with the new scope of [C1](../work-items/compute/C1.md) and [C2](../work-items/compute/C2.md) ([SPEC-C](../work-items/compute/SPEC-C.md)). Make C7 the second job type of [jobs.md](../proposals/jobs.md), after the build (OD-W12). Version 0.3 of the compute plan (2026-11-18) made the first sentence true for Sections 3.3, 12.1 and 13; it is not approved. The change for C7 is not made. |
| Extensions plan PRJ-001 and Section 4.2 | Import each repository; the operator leaves some out (OD-W11). PRJ-001 says that the operator accepts each project. OD-W11 is the ruling; PRJ-001 takes the new wording. |
| [jobs.md](../proposals/jobs.md) JOB-001, JOB-004, USE-001 | Changed on 2026-11-13 (OD-W12): a job is a container job or a native job on a Mac, and the build is the first job type. |
| Design specification BLD-001 to BLD-082 | The builder takes a project as a parameter; the release of Anima becomes one project (OD-W11). |
| Design specification Section 3.10 and BLD-029 | A machine can be a runner of GitHub Actions (OD-W13). The text "no GitHub runner" must change. |
| Design specification BLD-028, BLD-040 and Section 3.11 | Add Go to the tool list. The clone address has the new repository name. One name for the macOS application (OD-W19). Tasks R1, R3 and R5 of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md). |
| Extensions plan PRJ-019, OD-E14 and `contracts/extensions/sources.md` | The App needs the events `push` and `workflow_job` (PRO-042). The runner functions join the GitHub functions (PRO-049). The contract says 22 write permissions and the manifest has 23. |
| [cli-mcp-plan.md](../plans/cli-mcp-plan.md) Sections 6 and 7 | Not made: the `build` and `runner` commands and tools (PRO-053). Make them with the build. |
| `AGENTS.md`, [cli-mcp-plan.md](../plans/cli-mcp-plan.md) | Done on the branch `claude/anima-cli`: the command-line-first rule (CMD-030). |
| Design specification 6.10, 7, 9 and 13 | Done (operator, 2026-11-13): the command line and the MCP server are part of the specification. SET-004 names the "MCP" section. |
| Design specification 3.8.1 | Done as text (operator, 2026-11-13): the actor kinds User, Agent and Hub (LOG-012 to LOG-020). Built, except the audit list of the client (commit `17357ccd`; [cli-mcp-plan.md](../plans/cli-mcp-plan.md) holds the state). |
| `contracts/schema.graphql` (K1), `contracts/database.md` (K2), `contracts/conventions.md` (K7) | Made: the additions for the actor kinds are in K1 (`enum ActorKind`), K2 and K7 (commit `17357ccd`). The ruling number that OD-15 of the design specification asks for is not recorded. |
| Design specification 3.11 | Done as text (operator, 2026-11-13): installation from the disk image or the AppImage, with the `anima` program inside (INS-001 to INS-009). INS-002 to INS-007 are built (merge `a6ba6b19`); the check of OD-16 to OD-18 on a real release is open (the release record in [releases.md](../procedures/releases.md)). Added (operator, 2026-11-18, R-151, relayed): the command line alone by Homebrew and Cargo (INS-008 changed; INS-010 to INS-013; OD-23 to OD-26), as text; not built (tasks R8a to R8d). |
| Design specification 3.12 and 11 | Done as text (operator, 2026-11-13): the marketing website (SITE-001 to SITE-012). Not built. The operator must decide OD-20, OD-21 and OD-22 of the design specification. |
| [cli-mcp-plan.md](../plans/cli-mcp-plan.md) Sections 6 and 7 | Made (commit `17357ccd`): `anima key create` takes "made for"; `anima audit list` has the actor column and `--actor`; `anima mcp` sends the agent declaration (LOG-015, LOG-016, LOG-020). [cli-mcp-plan.md](../plans/cli-mcp-plan.md) holds the state. |
| Design specification MET-002 and ruling R-115 | Not made, and only after OD-W22 or OD-W23 is approved: the daemon can restart a watched program (WDG-026) and can replace its own program files (ROL-015). R-115 lists the daemon update as later work with no specification. |
| `contracts/daemon-push.md` (K8) | Not made, and it needs a programme ruling (CON-001): the daemon reports its restarts (WDG-031); the hub tells a daemon a version and gives it the files (ROL-008, ROL-009). Made: the daemon reports its version (ROL-051, R-122). |
| Design specification Sections 3.9, 3.10, 6.4, 6.9 and 7 | Not made, and only after approval: a release has the daemon, the hub and a release record (ROL-001 to ROL-005); the check type `watch` (WDG-035); the new calls in the list of INT-003 (OD-WDG-9, OD-ROL-9); the group "Hub and machines" in the Updates section (SET-006). |
