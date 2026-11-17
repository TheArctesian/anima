---
kind: Proposal
state: Draft
evidence: Direction decided (OD-W10). No requirement is approved. Version 2 (2026-11-16) adds JOB-011. Its tasks W1 to W6 are in the task table of the row above.
---

# Proposal: jobs (work that the coordinator gives to a machine)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the planning session of 2026-11-12. The operator agreed the form in a side chat: a job is
a container image, models run on the machine directly, jobs call the model service.
Version: 3. Date: 2026-11-16.
Status: Draft. The operator decided OD-W10 on 2026-11-12 (see "Decision").
The requirements are not yet approved one by one. The decision changes R-115. Version 2 adds
JOB-011, a proposal that follows OD-W13. Version 3 adds
JOB-012 to JOB-027, jobs that an AI agent starts.
Map: [work-plan.md](../work-plan.md), category Use, decisions OD-W5, OD-W6, OD-W10 and OD-W13.
Identifiers: USE-001 to USE-009, JOB-001 to JOB-027.

## Problem

The compute plan lets a machine answer model requests. It has one other type of work: C7,
fine-tuning ([C7](../work-items/compute/C7.md)). The operator also wants to run other work on the machines: a script, a batch,
an AI agent run. There is no common form for that work, so each type would get its own path to
the agent.

[C1](../work-items/compute/C1.md) and [C2](../work-items/compute/C2.md) already say that some model work needs "asynchronous job, progress and
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
  PRO-022 to PRO-024 of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) give its limits: a
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

- **JOB-011.** Proposed, no ruling. The fixed list of USE-006 starts with two work types:
  `build` and `runner`. A `build` runs a recipe of Anima (PRO-032). A `runner` runs the runner
  program of GitHub for one job of a workflow (PRO-027, PRO-039; OD-W13, operator, 2026-11-16).
  The hub cannot name a command in a `runner` item. The workflow file of the repository names
  the steps. Thus a `runner` item must have the isolation of PRO-029.

The build of a project is the first job type (PRO-009, OD-W12). Fine-tuning (compute plan C7,
[C7](../work-items/compute/C7.md)) is the second. Its "done when" text in [C7](../work-items/compute/C7.md) stays. A `runner` is a work type and not a job
type: GitHub schedules it (PRO-047).

The tasks for the work route are W1 to W6 in "Tasks" of
[projects-and-builds-plan.md](../plans/projects-and-builds-plan.md). That table holds their state (PLN-006).

## Jobs that an AI agent starts

Proposed on 2026-11-15. JOB-017 and JOB-018 are approved for build as the command
`anima snapshot` (operator, 2026-11-15: "lets build the two buildable ones now"; ruling number
to be assigned). The other requirements of JOB-012 to JOB-027 are not approved. The operator said: "if
the agent can't access or can't put extra resources on this machine it can branch out to other
that is coding agent agnostic using all the resources it can get".

The operator develops on one computer, and that computer often has no free memory or processor
time. An AI agent on that computer, for example Claude Code or Codex, must be able to give a
build or a test to a different machine. It uses the `anima` command or the MCP server, so the
function does not depend on one AI agent program. The compute plan uses "offload" for the layers
of a model (its Section 5.2); this proposal does not use that word for jobs.

| Term | Meaning |
| --- | --- |
| Requirement set | The tools, the build target and the resources that a job needs |
| Scratch ref | A git ref under `refs/anima/wip/` that holds a snapshot of a working tree |
| Artifact | A file that a job makes and keeps, for example a binary or an application bundle |

### Placement from the tool inventory

- **JOB-012.** A job must state a requirement set: the tools with a minimum version, the build
  target, the minimum free memory and the minimum free disk.
- **JOB-013.** The placement must be a pure function of the requirement set, the capabilities
  (compute plan CAP-001 to CAP-031), the measurements and the device policy (USE-003, USE-009).
- **JOB-014.** When no machine fits, the hub must refuse the job at once. The refusal must name
  each requirement that no machine meets. Example: "No machine has the target
  `aarch64-unknown-linux-gnu`".
- **JOB-015.** No command and no MCP tool takes a command line for a machine (USE-006, MCP-006).
  A caller names a job type from the fixed list and gives the parameters of that type.
- **JOB-016.** The build of a project and the test of a project are the first two job types
  that a caller can start. The steps come from the configuration of the project (PRO-009), not
  from the caller.

### The working tree

A job builds a commit. The working tree of the operator has changes that are not committed.

- **JOB-017.** `anima job start` must be able to send a snapshot of the working tree. It must
  make a commit of the tree with no change to the index, the branch or the stash. It must push
  that commit to a scratch ref on the forge of the project.
- **JOB-018.** The snapshot must obey the ignore rules of the repository. It must refuse a tree
  that contains a file that matches the secret patterns of the project (`.env*`, key files).
  Until jobs exist, the command is `anima snapshot push`. `anima snapshot list` and
  `anima snapshot delete` let the operator remove a scratch ref, because no hub job does it
  (JOB-019). The secret patterns are a fixed list in the command until a project has its own.
- **JOB-019.** The machine must build the scratch ref in a new checkout and delete the checkout
  when the job ends (PRO-023). The hub must delete each scratch ref when its job ends, and each
  scratch ref that is older than 7 days.
- **JOB-020.** A job from a scratch ref must not get a project secret (JOB-010) and must not
  publish a release. BLD-041 stays for releases.

### Artifacts

- **JOB-021.** A job type can name the files that it keeps. Each of them is an artifact. The
  artifact record is part of the one contract for job results (see "Needs").
- **JOB-022.** The hub must record each artifact with its job, its project, its commit, its
  build target, its size and its SHA-256.
- **JOB-023.** Artifacts are bulk data. They must use the store of
  [bulk-data-placement.md](bulk-data-placement.md). The hub must not keep them on its own disk.
- **JOB-024.** Until that store exists, the machine that made an artifact keeps it. When a
  request for it comes, the machine uploads the file through the hub. The agent must not listen
  on a port for this (MET-020).
- **JOB-025.** `anima artifact fetch` writes a file to a path that the caller gives and verifies
  the SHA-256. The MCP tool returns the path, not the content.
- **JOB-026.** An artifact from a scratch ref must carry the mark "not a release". The updater
  and the Releases page must never offer it.
- **JOB-027.** The Data retention section of the Settings page (SET-004) must get one period for
  artifacts. The default is 14 days.

### Decisions for the operator

| Question | Recommendation |
| --- | --- |
| How does the working tree get to the other machine? | A scratch ref on the forge (JOB-017). A file sync is a second transport. |
| Does MCP-006 stay? | Yes (JOB-015). Open point 1 of [cli-mcp-plan.md](../plans/cli-mcp-plan.md) stays open. |
| Which token can start a job? | A write API key, with the MCP tool `off` until the operator allows it (MCP-002). |

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
| Programs | One: `anima-agent` (as [OD-C2](../work-items/decisions/OD-C2.md) says). No second binary. |
| Default | A machine with the agent takes work (USE-004). The operator can set a machine off. |
| R-115 | Its "not taken: a command channel" line is replaced. The other lines of R-115 stay. |
| MET-020 | Not changed: the agent listens on no port and connects only to the hub (USE-005). |
| Compute plan CMP-002, [OD-C8](../work-items/decisions/OD-C8.md) | They say "off by default". They must change to agree. |

The cost that the operator accepts: a person who controls the hub can run work on each machine
that has the agent. R-084 selected the agent so that the hub needs no SSH login on each device
(OD-9); this decision gives the hub a second way in. These rules limit it, and none needs a step
from the operator:

- The agent accepts only the work types in a fixed list (USE-006). There is no "run this
  command" type. After OD-W13 the type `runner` runs the steps that a workflow file names
  (JOB-011). The hub still cannot name a command.
- Jobs run in containers (USE-001), with no access to the agent token or the home folder. A
  native job on a Mac has the same two limits through its build user (PRO-022).
- A machine that is set off at the machine cannot be set on from the hub (USE-004).
- Each item of work is in the audit log (LOG-005), and the machine keeps its own log of the work
  that it took.

This decision comes before [C1](../work-items/compute/C1.md), not only before jobs.

## Needs

- A ruling number in `programme.md` that changes R-115, then the transport decision in [C1](../work-items/compute/C1.md) and
  [SPEC-C](../work-items/compute/SPEC-C.md), and the new default in [OD-C8](../work-items/decisions/OD-C8.md).
- A new contract for the work route. K8 is frozen for Phase 3 (CON-001), so this is a new
  contract or `/agent/v2/`, not a change to K8.
- A security review of the work route (a `security` task for the security reviewer).
- One contract for job state, progress and results, shared with [C1](../work-items/compute/C1.md) and [C2](../work-items/compute/C2.md).
- A test of Railpack on one project of the operator (JOB-002).
- A design for how the hub gives a secret to one job run and how the machine deletes it
  (JOB-010), with the security review of the work route.
- A security review of JOB-017 to JOB-020: a scratch ref lets a caller select the code that a
  machine builds. Build scripts are code.
- The tool inventory of the compute plan (CAP-020 to CAP-031) before JOB-012 to JOB-014.
- After approval: change the title of [C7](../work-items/compute/C7.md) or add one work item for jobs in
  `docs/work-items/` (PLN-012).
