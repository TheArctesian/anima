# Proposal: AI agent runs as a job type

From: the planning session of 2026-11-12. The operator said in a side chat: "I do want to embed
agents quickly".
Status: a proposal for a ruling. Not approved. It needs [jobs.md](jobs.md) first.
Map: [work-plan.md](../../work-plan.md), category Use.

In this document, "AI agent" is a program that uses a model and tools to do a task. It is not
`anima-agent`, the program that sends metrics.

## Problem

The operator runs AI coding agents by hand on one computer. The extension XP-11 (#22) only
records those sessions. Anima cannot start a run on a selected machine, show it while it runs,
or limit what it can touch.

## Proposal

- **AGN-001.** An AI agent run is a job ([jobs.md](jobs.md)). The rules USE-001 to USE-007 apply.
- **AGN-002.** A run must name its workspace: one machine and one folder. The run must not read
  or write outside the workspace.
- **AGN-003.** A run must not have access to the credentials of the coordinator.
- **AGN-004.** Each run must have an allow list of tools.
- **AGN-005.** The audit log must record each action of a run (LOG-005). It must not record a
  secret (LOG-007).
- **AGN-006.** Do not write a new agent loop. Use an existing agent library.
- **AGN-007.** The run can use a model from the model service or a hosted model. The model
  service comes first (ETH-006).
- **AGN-008.** The client must show a live run and its history. The AI coding sessions extension
  (XP-11, #22) shows the record, with its redaction rules.

## Needs

- [jobs.md](jobs.md) approved, with its OD-W10 decision.
- The model service (C1, #34), or a decision that the first runs use a hosted model.
- A decision on which agent library (AGN-006), and on how a run gets a secret that it needs
  (the Secrets extension, XP-04, #15).
- A security review: a run executes text that a model wrote.
- After approval: one issue with the labels `compute`, `security` and `priority: later`.
