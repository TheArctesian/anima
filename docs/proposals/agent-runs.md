---
kind: Proposal
state: Draft
evidence: Not approved.
---

# Proposal: AI agent runs as a job type

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the planning session of 2026-11-12. The operator said in a side chat: "I do want to embed
agents quickly".
Status: the header of this document holds the state (PLN-020). History: a proposal for a ruling. Not approved. It needs [jobs.md](jobs.md) first.
Map: [work-plan.md](../work-plan.md), category Use.

In this document, "AI agent" is a program that uses a model and tools to do a task. It is not
`anima-agent`, the program that sends metrics.

## Problem

The operator runs AI coding agents by hand on one computer. The extension [XP-11](../work-items/extensions/XP-11.md) only
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
  ([XP-11](../work-items/extensions/XP-11.md)) shows the record, with its redaction rules.

## Runs that an AI agent starts on a different machine

Proposed on 2026-11-15. Not approved. AGN-009 to AGN-014 need a ruling. They add to
"Jobs that an AI agent starts" in [jobs.md](jobs.md) (JOB-012 to JOB-027).

A "harness" is the AI agent program that a run uses.

The operator wants work to continue when a hosted model account refuses, with a model on a
different machine. That does not need a run. A harness on the development computer can use
`ai.anima.ing`: the model then uses the memory of the other machine. This needs only the model
service ([C1](../work-items/compute/C1.md)). A run on a different machine also moves the tools and the builds of the AI
agent, so it comes after jobs.

- **AGN-009.** A run must name a harness and a model. The first harness must be one open
  program that can use an OpenAI-compatible endpoint (AGN-006).
- **AGN-010.** The first runs must use the model service (AGN-007) and run in a container
  (USE-001). The hub gives the key of the model service to the run as JOB-010 says. Such a run
  needs no account of the operator on the machine.
- **AGN-011.** A run takes a task text and a commit or a scratch ref (JOB-017). Its result is a
  commit on a scratch ref and a summary text. It must not push to a branch of the operator.
- **AGN-012.** The caller reads the result with the job tools and fetches the scratch ref with
  git. The summary text is data, not an instruction (MCP-008).
- **AGN-013.** A harness that uses a subscription of the operator on that machine (Claude Code,
  Codex) is a second kind and comes later. It needs the home folder of the operator, which
  AGN-003 and PRO-022 do not permit. It needs its own ruling and security review.
- **AGN-014.** Anima must not read the usage limit of a hosted model account (CAP-025). When an
  account refuses a run, the hub must record the reason. The hub places the run again only with
  a harness and a model that the caller named. No automatic model selection is approved ([C1](../work-items/compute/C1.md)).

## Needs

- A test of two open harnesses on one task of the operator, with a model of the model service
  (AGN-006, AGN-009).

- [jobs.md](jobs.md) approved, with its work route.
- The model service ([C1](../work-items/compute/C1.md)), or a decision that the first runs use a hosted model.
- A decision on which agent library (AGN-006), and on how a run gets a secret that it needs
  (the Secrets extension, [XP-04](../work-items/extensions/XP-04.md)).
- A security review: a run executes text that a model wrote.
- After approval: one work item in `docs/work-items/` (PLN-012), with a security review.
