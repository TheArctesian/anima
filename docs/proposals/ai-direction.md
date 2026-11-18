---
kind: Proposal
state: Draft
evidence: Version 1 (2026-11-17). R-131 to R-134 decide four points of the direction. R-135 approves step 1 (AIS-018 to AIS-024, in the extensions plan) for build. No other step is approved. It defines no requirement; its open decisions are OD-AI1 to OD-AI7.
---

# Proposal: the direction of the AI functions

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

| Item | Value |
| --- | --- |
| Document | AI direction |
| Version | 1 |
| Date | 2026-11-17 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | The header of this document holds the state (PLN-020). History: Draft |
| From | The planning session of 2026-11-17 for the model service. The operator asked for one view of "how we are really going to do AI" |
| Serves | Work plan Section 4: rows 4.5 (model service), 4.7 (jobs and AI agent runs) and 4.8 (the extensions, for AI coding sessions) |
| Sources | The documents of Section 2. The notes "Ideas from T3 Code", "Agent Control Planes Overview" and "Idle Compute Overview" of the business vault; their private parts stay there |

This document obeys the writing rules in Section 0 of the design specification.

## 1. Purpose

Ten documents describe AI functions. Two of them own the same path, three of them run an AI agent
in three different ways, and most of them assume the VPS and the VPN. This document gives the map,
the order and the decisions that make them one design. It owns no requirement (PLN-016): each
requirement stays in the document of Section 2 that defines it. When the operator decides a row of
Section 6, the owner document changes, not this one.

In this document, "AI agent" is a program that uses a model and tools to do a task, for example
Claude Code or Codex. It is not `anima-daemon`.

## 2. The map

The AI functions are three jobs.

| Job | What the operator gets | Parts | Owner |
| --- | --- | --- | --- |
| 1. Agents and usage | See and control each AI agent on each machine, and see the use of each account | The record of sessions and the counts | [extensions-plan.md](../plans/extensions-plan.md) Section 4.12 (AIS-001 to AIS-008) |
| | | Panes that stay alive, with a state and an alert | The same, Section 4.12.1 (AIS-009 to AIS-017) |
| | | Bots on the subscriptions of the operator | [bots.md](bots.md) |
| | | A run as a job on a selected machine | [agent-runs.md](agent-runs.md), [jobs.md](jobs.md) |
| 2. One model address | One address on each computer for each model that the operator can use | Models on the machines of the operator | [compute-plan.md](../plans/compute-plan.md) |
| | | Keys of model providers, and subscriptions sent without change | [extensions-plan.md](../plans/extensions-plan.md) Section 4.15 (the model gateway) |
| 3. More capacity | Work goes to the machine that is free; a rented machine adds capacity | The tool inventory (built) and the placement of a job | [compute-plan.md](../plans/compute-plan.md) Section 3.1, [jobs.md](jobs.md) |
| | | Rented computers and GPUs | [cloud-machines.md](cloud-machines.md) |

The command line and the MCP server ([cli-mcp-plan.md](../plans/cli-mcp-plan.md)) are the way in
for an AI agent to each function. They are built.

## 3. What is decided

- **R-131 (2026-11-17).** A model request goes directly from the computer of the caller to the
  machine with the model. The hub keeps the list and the record of use.
- **R-132 (2026-11-17).** Job 1 is built first. Job 2 follows it.
- **R-133 (2026-11-17).** The first usage function sends counts only, with no text of a session.
- **R-134 (2026-11-17).** The counts include the name of the repository of each session.
- **R-135 (2026-11-17).** Step 1 of Section 5 is approved for build: AIS-018 to AIS-024, tasks AU-1 to AU-5 of the extensions plan.
- The models on the machines of the operator stay near the top (operator, 2026-11-17: "the local
  models like unsloth coudl really help people get into the market"). A person with no paid
  subscription can start with them. Thus step 4 of Section 5 follows job 1 directly and is not
  parked. This is direction, not a ruling.
- The beachhead is the person who runs AI coding agents on more than one machine (operator,
  2026-11-16, business vault).
- Anima does not sell tokens. The operator uses the subscriptions and the keys that the operator
  already has.
- Anima must not read, keep or move the login of a vendor program ([bots.md](bots.md)), and must
  not read the usage limit of an account ([agent-runs.md](agent-runs.md); CAP-025).
- The operator installs Anima and no second program.

## 4. Where the documents do not agree

| Number | Subject | The documents |
| --- | --- | --- |
| 1 | Two owners of the path of a model request | The compute plan has a router at `ai.anima.ing` with one shared key. The model gateway has routes in the hub and identifies the caller by its VPN address. R-131 says that the hub carries no request. |
| 2 | Two records of use | The table of the model gateway, and the record that R-131 gives to the hub for the model service. |
| 3 | Three ways to run an AI agent | An agent run is in a container with no home folder. A bot runs the vendor program as the user of the operator. A pane is a terminal that the daemon keeps. |
| 4 | The daemon | The daemon must not listen on a port and has no work type "run this command" ([jobs.md](jobs.md); MET-020). Panes and bots need a session server in the daemon and a way to attach to it. |
| 5 | The choice of a model | A bot can use "the cheapest model that can do the task". The compute plan and the agent runs approve no automatic choice. |
| 6 | The hosted hub | These parts assume the VPS, a VPN address, Headscale or an operator password: the model gateway, the vault, the tags of bots, the join of a rented computer, and `anima auth login`. A hosted hub has none of them (R-129, R-130). |
| 7 | The path from one device to a different device | R-131, a pane that the operator opens from a second device, and "Add this computer" all need it. The link carries streams from a device to the hub only. |

## 5. Proposed order

Each step gives a function that the operator can use. No step is approved.

| Step | Content | Needs |
| --- | --- | --- |
| 1 | Usage: for each machine, each AI agent program and each day, the sessions, the tokens and the model. Counts only, no text. The hub, the command, the MCP tool, then the page (CMD-030). | OD-AI2, OD-AI3 |
| 2 | The spike for the path from one device to a different device through the link. | Nothing |
| 3 | Panes: start an AI agent on a selected machine, see its state, get an alert, attach to it. | Step 2, OD-AI4, OD-AI5 |
| 4 | The one model address on each computer, with the models of the machines of the operator. | Step 2, OD-AI1, the approval of the compute plan |
| 5 | Keys and subscriptions behind the same address. | Step 4, Secrets and the vault |
| 6 | Bots, runs in a container, rented computers. | Their proposals |

## 6. Open decisions

| Number | Question | Recommendation |
| --- | --- | --- |
| OD-AI1 | Does one document own the model address, the list of models and the record of use? | Yes. The model gateway owns the address on each computer, the list and the record. The compute plan owns the machines: runtimes, pulls, placement. The router of the compute plan and the hub routes of the model gateway are removed. |
| OD-AI2 | Does the first usage function send counts only, read by `anima-daemon`, before the vault exists? | Decided (R-133, operator, 2026-11-17: "Yes, counts only"). The requirements are AIS-018 to AIS-024 in Section 4.12.0 of the extensions plan. The text of sessions (AIS-004, AIS-005) waits for the vault and the collector. |
| OD-AI3 | Does the usage page show the quota that is left on an account? | Decided: no (R-135 approves AIS-022). It shows tokens and an estimate of the cost at the public price of the model. |
| OD-AI4 | Is a pane that runs the vendor program as the user of the operator the first way to run an AI agent? | Yes. It is the way that works with a subscription. A run in a container stays in [agent-runs.md](agent-runs.md) and comes later. The daemon gets one new function, the session server, with its own ruling. |
| OD-AI5 | Does Anima write its own session server, or embed Herdr? | Open decision 1 of Section 4.12.1 of the extensions plan holds this. Recommendation there: not ruled. |
| OD-AI6 | Does the automatic choice of a model wait? | Yes. The operator or the caller names the model. The rule "cheapest model" of a bot waits for its own decision. |
| OD-AI7 | Does job 1 get its own row in Section 4 of the work plan, before row 4.5? | Yes: one row "AI agents: usage and panes", tier 2b, before the model service. Row 4.5 keeps its number. |

## 7. Changes to other documents

OD-AI2: Section 4.12.0 of the extensions plan (AIS-018 to AIS-024, proposed). When a row of Section 6 is decided, the same commit changes the owner
document and gives the ruling its number (PLN-021).
