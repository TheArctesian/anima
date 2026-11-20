# Agent multiplexer and routing: one view of which agent runs on which device, and MCP that picks the machine

- From: [the operator], 2026-11-19 (16:36 PT), in the chat of the Anima Product Planner. The Planner relayed his words to the idea stage. They are relayed here, not first-hand in this record.
- Source: the vault note "Agent Multiplexer and Routing"
- Status: Framed
- Serves: not confirmed. Closest rows: 4.21 Agent tools as extensions (parts a and b) and 4.7 Jobs, builds and AI agent runs (part c). The Planner or the operator must name the row (question 1).
- Owner if it becomes work: Open. Part (a) is already owned by [agent-tools-as-extensions.md](../../proposals/agent-tools-as-extensions.md) (AXT-001 to AXT-045). Part (b) is a deferred item of the same proposal. Part (c) has no owner; its neighbours are [jobs.md](../../proposals/jobs.md) (USE-003, JOB-012 to JOB-015) and [agent-runs.md](../../proposals/agent-runs.md) (AGN-001, AGN-002).
- Effort (estimates): (a) S, only a change to a decided proposal; (b) M; (c) M to L, after a ruling on MCP-006 and a security review. Recommendation: should, but merge it. Two of its three parts are already in a plan or on the deferred list; one part is new.

## The idea

The operator tried the agent multiplexer Herdr and found that it is "just a multiplexer". He wants Anima.ing to embed the multiplexer and add a layer on top: it shows which agent runs on which device, and it does this by itself. He also wants the MCP layer to route the work: if this machine is not the right one to run a task, the MCP layer sends it to the machine that is.

## Evidence

- The operator's words, relayed on 2026-11-19 (16:36 PT) by the Anima Product Planner (full text under "Source text"). The spelling "Herder" is read as Herdr, the agent multiplexer that the plans already name.
- The vault note on Herdr (checked 2026-11-18, from competitor research of 2026-11-16, not re-read at the source for this brief): Herdr is Apache-2.0, keeps agents running in terminals held by a background server, shows each as working, blocked or done, and has one sidebar for local and SSH machines. It has no MCP server and no machine health. It starts agents on other machines only through your own SSH or Tailscale. Its desktop application and its terminal program do not share agents. Not verified: the current Herdr documents and its release state; the GitHub page of the project did not load for the idea stage on 2026-11-19.
- Repository, checked 2026-11-19 at commit `05a4bb62` on `dev`. See the overlap check.

## Overlap check

Files read for this check: [agent-tools-as-extensions.md](../../proposals/agent-tools-as-extensions.md), [extensions-plan.md](../../plans/extensions-plan.md) Section 4.12.1, [default-panels.md](../../proposals/default-panels.md), [bots.md](../../proposals/bots.md), [jobs.md](../../proposals/jobs.md), [agent-runs.md](../../proposals/agent-runs.md), [cli-mcp-plan.md](../../plans/cli-mcp-plan.md) (MCP-001 to MCP-006, Section 9 and Section 11), [compute-plan.md](../../plans/compute-plan.md) (Sections 6.3, 7 and 8.1), [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md), [plan/priorities.md](../../plan/priorities.md), [deferred.md](../../deferred.md), and the five ideas `agent-execution-on-managed-machine`, `per-agent-resource-attribution`, `needs-you-inbox`, `agent-state-hooks` and `remote-mcp-over-http`. This is a keyword search plus a read of the named sections, not a full read of each document.

| Part | Already covered by | What is new |
| --- | --- | --- |
| (a) Embed an agent multiplexer | Covered. R-196 (OD-AXT2) withdrew the native panes AIS-009 to AIS-017 and decided to host Herdr as a terminal tool extension, as a pane in the client. AXT-020 puts Herdr in the version 1 catalogue. AXT-014 opens it in the SSH terminal of the device. AXT-024 gives its install recipe. AXT-026 says that BOT-069 starts an agent in a Herdr pane when Herdr is installed on the device. AXT-030 says Anima.ing does not fork or rebrand a tool. AXT-025 leaves the Herdr desktop and phone applications out of version 1. Row 4.21, R-196 | Nothing, if "embed" means host Herdr as it is. A different meaning (build the open Herdr runtime into the daemon) would reverse OD-AXT2 and AXT-030 (question 3) |
| (b) Show which agent runs on which device, automatically | Partly covered, partly deferred. Version 1 leaves the state of each agent to Herdr's own screen, with no Anima.ing page of all agents and no alerts (OD-AXT10, R-196). The page of every pane on every device (AIS-011) and `anima agents` across devices (AIS-014) are withdrawn. [deferred.md](../../deferred.md) holds "A page of all agents on all devices, with alerts: after version 1" (from the ideas `agent-state-hooks` and `needs-you-inbox`). PNL-002 (default-panels.md) has an Agents page with the agents that the daemon found on each device, one state each (BOT-024), and a link to the Herdr pane; its open clarification is whether the page also lists the Herdr panes. Usage by device is built (AIS-018 to AIS-024). The `anima tui` screen lists open sessions (cli-mcp-plan.md Section 11) | A decision to move the deferred page into scope and to feed it from the Herdr state protocol. The "automatic" part is cheap: the hub already knows the device of each agent, so no one tags a pane by hand |
| (c) MCP routing to the right machine | Not covered, and in conflict with several rules. MCP-001 and MCP-002: `anima mcp` is one server for one hub, with a state (`allow`, `ask`, `off`) for each tool. MCP-006: no tool opens a session or runs a command on a device (cli-mcp-plan.md Section 9, open point 1: it needs an operator decision and a security review). JOB-015: no MCP tool takes a command line for a machine. AXT-012 and AXT-019: an MCP tool must not open a pane or start, stop or install a tool; one read tool lists states. AXT-021 and CHAT-041: an action on a machine needs the click of the operator. Neighbours that already choose a machine: placement as a pure function of the requirements, the inventory and the device policy (USE-003, JOB-012 to JOB-014, CMP-011) with failover (USE-007); a run names one machine and one folder (AGN-002); the withdrawn rule of BOT-018 (the operator's order of machines, the next as fallback); model routing (RTR-030, and RTR-014 and RTR-036: a named node is never replaced silently); SEL-004 (Anima.ing does not choose a model) | Choosing the machine for an agent task, and giving that choice to an AI agent through MCP. Its inputs would be the tool inventory (CAP-023), device policy (USE-004), health and free memory. It needs a ruling on MCP-006, and a rule for what "right machine" means |

- `register.md`: [agent-tools-as-extensions.md](../../proposals/agent-tools-as-extensions.md) (row 4.21), [jobs.md](../../proposals/jobs.md) and [agent-runs.md](../../proposals/agent-runs.md) (row 4.7) are the owners that apply. No plan or proposal owns routing of agent work. `ownership.md` names no owner for it.
- The chat plan (compute-plan.md Section 8.1, CHAT-008 to CHAT-045): only CHAT-041 and CHAT-044 touch this. CHAT-044 asks whether the Chat page reuses the MCP tools and grants of Hermes Agent. Nothing in the chat plan routes agent work.
- Ideas folder: `agent-execution-on-managed-machine` (an agent here, commands on a managed machine) is the nearest in intent for part (c) but is about where commands run, not which machine is chosen. `agent-state-hooks` and `needs-you-inbox` feed part (b). `per-agent-resource-attribution` shows the CPU and memory of each agent and would be an input to "right machine". `remote-mcp-over-http` is the same MCP surface for cloud assistants.
- Result: mostly an existing proposal. Part (a) is decided and part (b) is deferred in AXT. Only part (c) is new. The recommendation is to merge (a) and (b) into AXT and to bring (c) to the jobs and agent-runs work.
- Not checked: the full text of each plan; whether Herdr can be told by a manifest to use the host's own device name; the current Herdr documents.

## Pieces and effort

Effort is an estimate (S about a week, M one to three weeks, L more than three weeks, one engineer).

| Piece | Effort (estimate) | Recommendation |
| --- | --- | --- |
| (a) Herdr in a pane | S (an amendment; the AXT tasks hold the work) | Already decided; do not repeat |
| (b) The page of all agents on all devices, with state | M (after the state protocol of `agent-state-hooks`) | should |
| (c) A suggestion of the right machine, read-only | M | should, after (b) |
| (c) Automatic start on the chosen machine | L, after a ruling on MCP-006 and a security review | could, later |

## Narrowest version that proves it

A read-only answer to "where should this agent task run": for a task that needs a tool and a repository, list the machines that have them, are healthy and take work (USE-004), in a fixed order that Anima.ing shows. Nothing starts. The operator or the agent then picks. Measure: in one week of use, the suggested machine is the one the operator would have chosen.

## Stop if

The operator drops it if the Herdr-based version 1 of AXT shows that panes per device are enough, or if a security review says that an MCP tool that starts work on a chosen machine cannot be made safe.

## Open questions

1. Where does this idea go? (one choice: A, fold it into what exists: parts (a) and (b) become changes to agent-tools-as-extensions.md on row 4.21, and part (c) goes to the jobs and agent-runs work on row 4.7; B, one new proposal with its own new row; C, all three parts into agent-tools-as-extensions.md on row 4.21. Recommended: A.)
2. When the MCP layer picks a machine, does it act by itself? (one choice: suggest the machine and ask the operator first; act by itself for read-only work only; act by itself for all work. Recommended: suggest and ask first. It follows MCP-002 `ask`, AXT-021 and CHAT-041, and needs no change of MCP-006 for the first version.)
3. Does "embed" mean host Herdr as it is, in a pane (decided in R-196)? (yes/no. Recommended: yes. A "no" means building the open Herdr runtime into the daemon, which reverses OD-AXT2 and AXT-030 and is a larger project.)

## Reviews

- 2026-11-19: round 1. The idea stage framed the idea from the relayed words and an overlap check. No answers yet. The Serves row is not confirmed, so the status is Framed.

## Source text

Relayed by the Anima Product Planner, 2026-11-19, from the operator's words in its chat on 2026-11-19 at 16:36 PT, with no change except that the operator's name is replaced by "[the operator]":

> I actually just tried Herder, and it is just a multiplexer, like an agent multiplexer. And we can embed the agent multiplexer and add on the layer of adding in this agent is running on this device, this agent is running on this device, and it automatically does that. And then also the MCP layer just automatically does the routing for you. Like if this machine is not the right machine to run it, then the other machine is the right machine to run it.

The Planner added: check the overlap with the agent tools as extensions work (AXT) and the chat plan before marking it Ready, and this is not part of Observability Suite.
