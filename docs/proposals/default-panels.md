---
kind: Proposal
state: Draft
evidence: Version 1.1 (2026-11-18): R-196 withdrew the native agent panes (AIS-009 to AIS-017); PNL-002, the table of Section 2 and OD-PNL5 now point to the Herdr tool extension. Version 1 (2026-11-17). Founder direction of 2026-11-17, relayed by the Anima Competitor Bot; no ruling. PNL-001 to PNL-008 and OD-PNL1 to OD-PNL6 are proposed, not ruled. No requirement is approved. It has no tier in Section 4 of the work plan.
---

# Proposal: the default panels (Devices, Agents, Projects)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this proposal (PLN-005).

| Item | Value |
| --- | --- |
| Document | Proposal: the default panels of the desktop client |
| Version | 1 |
| Date | 2026-11-17 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | The header of this document holds the state (PLN-020). History: Draft. Founder direction, not a ruling. No requirement is approved. |
| Serves | Work plan Section 4, row 4.4 (the engine and the extension switches, EXT-001, EXT-008). It also touches row 4.6 (Projects) and the AI agents work of [ai-direction.md](ai-direction.md) |
| Related | [extensions-plan.md](../plans/extensions-plan.md) Sections 2.8 and 3.3, [bots.md](bots.md) (BOT-039, BOT-040), [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) (PRO-001, PRO-027), the design specification Section 5.3 |

This document obeys the writing rules in Section 0 of the design specification.

## 1. The direction

The founder said on 2026-11-17 (relayed by the Anima Competitor Bot; words as relayed, spelling kept):
"Also we should probably have the devices page, agent and a project pannel pannel by default so we can
add runners on easily. With the Bookshelf, Maps, etc as extensions".

This is founder direction. It is not a ruling: it answers no question about these requirements, and an
agent's relay is not an approval (PLN-021). No `R-nnn` number is written for it. When the operator
answers the open decisions of Section 4, the answer gets its ruling number and this document changes.

I read it as: a person who installs Anima.ing sees three panels with no setup: Devices, Agents and
Projects. A runner is easy to add from each of them. Bookshelf and Maps are extensions that the
person turns on.

## 2. What the documents say today

| Subject | Today | Source |
| --- | --- | --- |
| Devices page | The start page. It is the foundation. | NAV-004, design specification Section 5.3 |
| Projects | An extension, off by default. The operator ruled that it is not a part of the foundation. The foundation must work with all extensions off. | EXT-001, EXT-005, OD-E13 (operator, 2026-11-13) |
| Projects pages | The list and the detail page are on `main`. A project is one repository, or more than one. The import comes from Connect GitHub. | PRO-001 to PRO-004, PRJ-001, PRJ-039 to PRJ-042 |
| Agents | There is no page of this name. The page that listed every pane on every device was AIS-011; R-196 withdrew AIS-009 to AIS-017, and Anima.ing hosts Herdr as a tool extension in their place ([agent-tools-as-extensions.md](agent-tools-as-extensions.md); version 1 leaves the state of each agent to the screen of Herdr, OD-AXT10). The usage page is AU-4 (approved, not started). Both belong to the extension "AI coding sessions", which is off by default. | AIS-009 to AIS-017 (withdrawn, R-196), AIS-018 to AIS-024 (R-135), extensions plan Section 4.12; [agent-tools-as-extensions.md](agent-tools-as-extensions.md) (AXT-020, AXT-026) |
| Runners | Two kinds exist in the documents. An agent runner is an agent in a pane on a device (BOT-039, proposed). A build runner is an owned machine that is a runner of GitHub Actions for a project (PRO-027, PRO-039). | [bots.md](bots.md), [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) |
| Bookshelf | The Library extension (Section 4.14), off by default. E0-8 calls it "Library/bookshelf". | EXT-001, [E0-8](../work-items/engine/E0-8.md) |
| Maps | The Map extension (Section 4.1), off by default. The map canvas component `$lib/places/` is shared with Devices. | EXT-001, extensions plan Section 3.2 |
| First start | The client asks what Anima must do and offers sets of extensions. A hosted first run is only the sign-in. | EXT-008, HMD-037 (R-128) |
| Navigation groups | Infrastructure: Devices, Projects, Model gateway. Each group shows only the extensions that are on. | Extensions plan Section 3.3, NAV-014 |

Bookshelf and Maps are extensions today, and they are off by default. This proposal moves no feature out
of the foundation for them (OD-PNL3). The real change is that Agents and Projects are on by default, and
that Agents is a page of its own.

## 3. Proposed requirements

None of these is approved. Build nothing from them (PLN-007).

- **PNL-001.** After the first run, the navigation bar must show Devices, Agents and Projects. The
  operator must not need to set an extension on first. Devices stays the start page (NAV-004).
- **PNL-002.** The Agents page must show, for each device, the agents that the daemon found and one
  state for each (BOT-024). When Herdr is installed and granted on a device (the Herdr tool extension of
  [agent-tools-as-extensions.md](agent-tools-as-extensions.md), R-196; AIS-009 to AIS-013 are withdrawn), it must link to the Herdr pane
  of that device. R-242 decides that the Agents page lists the agents of all devices with their states, Herdr panes included
  (AXT-046 to AXT-052 of [agent-tools-as-extensions.md](agent-tools-as-extensions.md); OD-AXT21 asks if it replaces this list). It must show the usage page (AU-4, AIS-020). With no agent
  it must show an empty state with the action "Add agent" (UI-030).
- **PNL-003.** "Add agent" must open the list of the agent catalog (BOT-040) and the custom entry
  (BOT-039). The terms gate of BOT-041 applies.
- **PNL-004.** The Projects page must show, for each project, its machines that are build runners
  (PRO-027) and the agents that work in its repository (AIS-002, PRJ-014). With no project it must show
  an empty state with the action "Connect GitHub" (PRJ-039).
- **PNL-005.** Each device in the Devices page must have the action "Add runner". It must open one
  choice between the kinds of runner of Section 2, and then the flow of that kind (PNL-003 for an agent;
  PRO-027 for a build runner). It needs no new hub function. Each flow keeps its own requirements.
- **PNL-006.** Agents and Projects must be extensions that are on in a new installation, and the operator
  can set each one off in Settings (EXT-002). The foundation must still work with all extensions off
  (EXT-005). Every other extension stays off by default (EXT-001), including Library and Map.
- **PNL-007.** The navigation bar must keep the groups of Section 3.3 of the extensions plan: Agents
  goes in the group Infrastructure, after Devices and before Projects. A group with no active item must
  not show (NAV-014).
- **PNL-008.** The hosted first run stays only the sign-in (HMD-037). The three panels show after the
  sign-in, with no question about extensions in the first run (OD-PNL4). The command line and the MCP
  server get no new command: each flow uses an existing function (CMD-030).

## 4. Open decisions

Each row is proposed, not ruled. Only the operator can rule it.

| Decision | Question | Recommendation |
| --- | --- | --- |
| OD-PNL1 | Are Agents and Projects a part of the foundation, or extensions that are on by default? OD-E13 (operator, 2026-11-13) ruled that Projects is not a part of the foundation, so the first choice reverses a ruling. | Proposed, not ruled: extensions that are on by default (PNL-006). It keeps EXT-005 and OD-E13, and the operator can still turn them off. Only EXT-001 and EXT-008 change, and only for these two. |
| OD-PNL2 | What does "runner" mean in "add runners on easily": an agent runner (BOT-039), a build runner of GitHub Actions (PRO-027), or both? | Proposed, not ruled: both, in one choice on "Add runner" (PNL-005). The two flows stay separate. |
| OD-PNL3 | Which features move out of the foundation as extensions? | Proposed, not ruled: none. Bookshelf (Library, Section 4.14), Maps (Map, Section 4.1) and the others are extensions already. The map canvas component stays shared with Devices. Confirm whether "Maps" means the Map extension, or the project map (XP-14). |
| OD-PNL4 | Does the first-start question of EXT-008 stay, with Devices, Agents and Projects already selected, or is it removed? | Proposed, not ruled: keep it for a local and a self-hosted first run, and leave it out of the hosted first run (HMD-037). |
| OD-PNL5 | Is the Agents page a new extension, or the extension "AI coding sessions" with its panes and its usage? The text of sessions (AIS-001 to AIS-008) needs the vault and is a larger step. | Proposed, not ruled: one default-on extension "Agents" with the agent tools (Herdr and Hermes Agent, [agent-tools-as-extensions.md](agent-tools-as-extensions.md); AIS-009 to AIS-017 and the native bot runtime are withdrawn by R-196), the usage (AIS-018 to AIS-024) and the agent states, catalogue and install of bots.md (BOT-019 to BOT-024, BOT-039 to BOT-046, BOT-058 to BOT-078). The session text stays in "AI coding sessions", off by default. |
| OD-PNL6 | Does the phone layout show the same three panels? It has four destinations: Devices, Sessions, Logs and Settings (MOB-010). | Proposed, not ruled: not in the first version. Show Agents and Projects on the phone later. |

## 5. Changes to other documents

None yet. When the operator rules OD-PNL1 and OD-PNL4, the same commit changes the owner documents and
gives the ruling its number (PLN-021): EXT-001, EXT-005 and EXT-008 and OD-E13 of the extensions plan,
the table of Section 3.3 of the extensions plan, NAV-001 of the design specification, and the mockups of
the navigation bar (PLN-019). This proposal changes no mockup now, because it changes no screen.
