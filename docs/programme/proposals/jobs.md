# Proposal: jobs (work that the coordinator gives to a machine)

From: the planning session of 2026-11-12. The operator agreed the form in a side chat: a job is
a container image, models run on the machine directly, jobs call the model service.
Status: a proposal for a ruling. Not approved. It conflicts with R-115 (see below).
Map: [work-plan.md](../../work-plan.md), category Use, decisions OD-W5, OD-W6 and OD-W10.

## Problem

The compute plan lets a machine answer model requests. It has one other type of work: C7,
fine-tuning (#40). The operator also wants to run other work on the machines: a script, a batch,
an AI agent run. There is no common form for that work, so each type would get its own path to
the agent.

Issues #34 and #35 already say that some model work needs "asynchronous job, progress and
artifact/result contracts". This proposal must use the same contracts, not a second set.

## Rules for all work on machines

These rules apply to jobs, to AI agent runs ([agent-runs.md](agent-runs.md)) and to hosted
applications ([hosted-applications.md](hosted-applications.md)).

- **USE-001.** Models run on the machine directly. Jobs run in containers.
- **USE-002.** A job must use the model service for inference. Thus a job on a Linux machine can
  use a model on a Mac.
- **USE-003.** The coordinator decides where work runs. The placement must be a pure function, as
  in the compute plan (CMP-011).
- **USE-004.** A machine takes work only when the operator sets this on for that machine
  (CMP-002).
- **USE-005.** The agent must ask the coordinator for work. The agent must not listen on a port
  (MET-020).
- **USE-006.** The agent must accept only the work types in a fixed list. It must refuse all
  other work.
- **USE-007.** If a machine stops during a job, the coordinator gives the job to a different
  machine.

## Jobs

A job is one container image that runs to its end on one machine.

- **JOB-001.** A job is a container image.
- **JOB-002.** The operator can make the image from a folder of source code with Railpack.
  Nixpacks is in maintenance, so do not use it (OD-W6).
- **JOB-003.** The operator can make the image from a Nix recipe (a flake).
- **JOB-004.** A later version can run a Nix recipe on a Mac without a container. Use this for a
  job that needs the Mac hardware directly.
- **JOB-005.** One machine builds an image. The other machines pull it.
- **JOB-006.** Queued jobs come first. A function that answers a request in milliseconds comes
  later (OD-W5).
- **JOB-007.** The client must show each job with its machine, its state, its log and a stop
  control.

Fine-tuning (compute plan C7, #40) becomes the first job type. Its "done when" text in #40 stays.

## Conflict with R-115

R-115 (operator, 2026-11-12) lists as **not taken**: "a command channel from the agent to the
hub, because it would make the agent a remote-execution endpoint (MET-013, MET-020); anything
that changes a device". K8 says that the agent makes no request other than enrol, push and renew.

A job is remote execution. So are CAG-003 of the compute plan (install a runtime, start a server)
and HOST-002. USE-005 and USE-006 limit the risk but do not remove it. Issues #34, #41 and #44
already record that the command transport is open against MET-020.

The operator must select one of these (OD-W10):

1. **One agent, work off by default.** `anima-agent` gets a work route in a new contract. A
   machine takes work only after the operator sets it on at that machine, not from the hub
   (USE-004). The metrics agent on all other machines stays as R-115 describes it.
2. **Two programs.** `anima-agent` stays metrics only, as R-115 says. A second program takes
   work, and the operator installs it only on compute machines. This is against #44, which says
   not to make a second binary.
3. **Keep R-115 as it is.** No jobs, no hosted applications, and the model service needs a
   different design.

This decision comes before C1 (#34), not only before jobs.

## Needs

- A ruling on OD-W10, then the transport decision in #34 and #41.
- A security review of the work route (a `security` task for the security reviewer).
- One contract for job state, progress and results, shared with #34 and #35.
- A test of Railpack on one project of the operator (OD-W6).
- After approval: change the title of #40 or add one issue for jobs, with the labels `compute`
  and `priority: later`.
