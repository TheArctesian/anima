# Proposal: jobs (work that the coordinator gives to a machine)

From: the planning session of 2026-11-12. The operator agreed the form in a side chat: a job is
a container image, models run on the machine directly, jobs call the model service.
Status: a proposal for a ruling. The operator decided OD-W10 on 2026-11-12 (see "Decision").
The requirements are not yet approved one by one. The decision changes R-115.
Map: [work-plan.md](../../work-plan.md), category Use, decisions OD-W5, OD-W6 and OD-W10.
Identifiers: USE-001 to USE-009, JOB-001 to JOB-010.

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

- **USE-001.** Models run on the machine directly. Jobs run in containers. The one exception is
  a native job on a Mac (JOB-004).
- **USE-002.** A job must use the model service for inference. Thus a job on a Linux machine can
  use a model on a Mac.
- **USE-003.** The coordinator decides where work runs. The placement must be a pure function, as
  in the compute plan (CMP-011).
- **USE-004.** A machine with `anima-agent` takes work by default. The operator must be able to
  set this off for each machine, at the machine and in the client. A machine that is set off at
  the machine must stay off; the hub must not be able to set it on.
- **USE-008.** The install command and the Device page must say that the agent takes work from
  the hub (MET-019, MET-024). An agent that was installed before this function exists takes work
  after its update, with no confirmation (operator, 2026-11-12: "On after update"). The hub must
  write one event for each machine when this starts.
- **USE-009.** "Takes work" is the permission. When a machine takes work is the device policy of
  the compute plan (CAP-010 to CAP-013): mains power, quiet hours, an active operator.
- **USE-005.** The agent must ask the coordinator for work. The agent must not listen on a port
  (MET-020).
- **USE-006.** The agent must accept only the work types in a fixed list. It must refuse all
  other work.
- **USE-007.** If a machine stops during a job, the coordinator gives the job to a different
  machine.

## Jobs

A job runs to its end on one machine. It is a container image, or it is a native job on a Mac
(OD-W12).

- **JOB-001.** A job is a container image, unless it is a native job (JOB-004).
- **JOB-002.** The operator can make the image from a folder of source code with Railpack.
  Nixpacks is in maintenance, so do not use it. Test Railpack on one project of the operator
  before this requirement is final (OD-W6).
- **JOB-003.** The operator can make the image from a Nix recipe (a flake).
- **JOB-004.** A native job runs on a Mac without a container. Use it only for work that needs
  macOS directly, for example a macOS or iOS build. It comes with the first job type, not in a
  later version, and it is not only a Nix recipe: Nix does not supply Xcode or the macOS SDK.
  PRO-022 to PRO-024 of [projects-and-builds.md](projects-and-builds.md) give its limits: a
  build user, a new checkout that the machine deletes, and a setting on the Mac. (OD-W12,
  operator, 2026-11-13)
- **JOB-005.** One machine builds an image. The other machines pull it.
- **JOB-006.** Queued jobs come first. A function that answers a request in milliseconds comes
  later (JOB-009).
- **JOB-007.** The client must show each job with its machine, its state, its log and a stop
  control.
- **JOB-008.** The operator can make the image from a Dockerfile. No builder is the only way
  (OD-W6, operator, 2026-11-12: "leave the option to do both").
- **JOB-009.** A later function that answers a request must be a container that stays warm, not
  a WebAssembly runtime (OD-W5, operator, 2026-11-12).
- **JOB-010.** The CLI tool of a rented service can run as a job (OD-W1). The hub opens the
  secret of the project and gives it to the machine for that one job (VLT-007, OD-E12). The hub
  must record the release in the audit log, and the machine must delete the secret when the job
  ends. Thus a build that a push starts needs no client. (operator, 2026-11-13; the first text
  said that a client releases it.)

The build of a project is the first job type (PRO-009, OD-W12). Fine-tuning (compute plan C7,
#40) is the second. Its "done when" text in #40 stays.

## Decision (OD-W10, operator, 2026-11-12)

R-115 (operator, 2026-11-12) listed as **not taken**: "a command channel from the agent to the
hub, because it would make the agent a remote-execution endpoint (MET-013, MET-020); anything
that changes a device". K8 says that the agent makes no request other than enrol, push and renew.
A job is remote execution. So are CAG-003 of the compute plan (install a runtime, start a
server), HOST-002 and the store machines of [bulk-data-placement.md](bulk-data-placement.md).

The options were: one agent with work off by default (the recommendation), a second program
for compute machines, or R-115 kept as it is. The operator selected one agent and changed the
default: **"it should be on by default"**.

What this means:

| Subject | Decided |
| --- | --- |
| Programs | One: `anima-agent` (as #44 says). No second binary. |
| Default | A machine with the agent takes work (USE-004). The operator can set a machine off. |
| R-115 | Its "not taken: a command channel" line is replaced. The other lines of R-115 stay. |
| MET-020 | Not changed: the agent listens on no port and connects only to the hub (USE-005). |
| Compute plan CMP-002, OD-C8 (#50) | They say "off by default". They must change to agree. |

The cost that the operator accepts: a person who controls the hub can run work on each machine
that has the agent. R-084 selected the agent so that the hub needs no SSH login on each device
(OD-9); this decision gives the hub a second way in. These rules limit it, and none needs a step
from the operator:

- The agent accepts only the work types in a fixed list (USE-006). There is no "run this
  command" type.
- Jobs run in containers (USE-001), with no access to the agent token or the home folder. A
  native job on a Mac has the same two limits through its build user (PRO-022).
- A machine that is set off at the machine cannot be set on from the hub (USE-004).
- Each item of work is in the audit log (LOG-005), and the machine keeps its own log of the work
  that it took.

This decision comes before C1 (#34), not only before jobs.

## Needs

- A ruling number in `programme.md` that changes R-115, then the transport decision in #34 and
  #41, and the new default in #50.
- A new contract for the work route. K8 is frozen for Phase 3 (CON-001), so this is a new
  contract or `/agent/v2/`, not a change to K8.
- A security review of the work route (a `security` task for the security reviewer).
- One contract for job state, progress and results, shared with #34 and #35.
- A test of Railpack on one project of the operator (JOB-002).
- A design for how the hub gives a secret to one job run and how the machine deletes it
  (JOB-010), with the security review of the work route.
- After approval: change the title of #40 or add one issue for jobs, with the labels `compute`
  and `priority: later`.
