---
kind: Proposal, with a task table
state: Direction decided
evidence: Version 4 (2026-11-19): R-234 (operator, first-hand, 16:41 to 16:50 PT) adds automatic routing of work to the right machine for all work (ROU-001 to ROU-014, not approved; tasks RT0 to RT8) and changes JOB-015 and MCP-006. Direction decided (OD-W10; R-160, 2026-11-18, confirms it also for a hosted hub and replaces the "command channel" item of R-115). No requirement is approved. Version 2 (2026-11-16) adds JOB-011. Its tasks W1 to W6 are in the task table of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md).
---

# Proposal: jobs (work that the coordinator gives to a machine)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the planning session of 2026-11-12. The operator agreed the form in a side chat: a job is
a container image, models run on the machine directly, jobs call the model service.
Version: 4. Date: 2026-11-19.
Status: the header of this document holds the state (PLN-020). History: Draft. The operator decided OD-W10 on 2026-11-12 (see "Decision").
The requirements are not yet approved one by one. The decision changes R-115. Version 2 adds
JOB-011, a proposal that follows OD-W13. Version 3 adds
JOB-012 to JOB-027, jobs that an AI agent starts.
Map: [work-plan.md](../work-plan.md), category Use, decisions OD-W5, OD-W6, OD-W10 and OD-W13.
Identifiers: USE-001 to USE-009, JOB-001 to JOB-027, ROU-001 to ROU-014, OD-ROU1 to OD-ROU6, tasks RT0 to RT8.

## Problem

The compute plan lets a machine answer model requests. It has one other type of work: C7,
fine-tuning ([C7](../work-items/compute/C7.md)). The operator also wants to run other work on the machines: a script, a batch,
an AI agent run. There is no common form for that work, so each type would get its own path to
the daemon.

[C1](../work-items/compute/C1.md) and [C2](../work-items/compute/C2.md) already say that some model work needs "asynchronous job, progress and
artifact/result contracts". This proposal must use the same contracts, not a second set.

## Rules for all work on machines

These rules apply to jobs, to AI agent runs ([agent-runs.md](agent-runs.md)) and to hosted
applications ([hosted-applications-plan.md](../plans/hosted-applications-plan.md)).

- **USE-001.** Models run on the machine directly. Jobs run in containers. The one exception is
  a native job on a Mac (JOB-004).
- **USE-002.** A job must use the model service for inference. Thus a job on a Linux machine can
  use a model on a Mac.
- **USE-003.** The coordinator decides where work runs. The placement must be a pure function, as
  in the compute plan (CMP-011).
- **USE-004.** A machine with `anima-daemon` takes work by default. The operator must be able to
  set this off for each machine, at the machine and in the client. A machine that is set off at
  the machine must stay off; the hub must not be able to set it on.
- **USE-008.** The install command and the Device page must say that the daemon takes work from
  the hub (MET-019, MET-024). A daemon that was installed before this function exists takes work
  after its update, with no confirmation (operator, 2026-11-12: "On after update"). The hub must
  write one event for each machine when this starts.
- **USE-009.** "Takes work" is the permission. When a machine takes work is the device policy of
  the compute plan (CAP-010 to CAP-013): mains power, quiet hours, an active operator.
- **USE-005.** The daemon must ask the coordinator for work. The daemon must not listen on a port
  (MET-020).
- **USE-006.** The daemon must accept only the work types in a fixed list. It must refuse all
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
the daemon can't access or can't put extra resources on this machine it can branch out to other
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
  A caller names a job type from the fixed list and gives the parameters of that type. When the
  caller names no machine, routing picks it and starts the work with no question (ROU-001, R-234).
  Routing does not change this rule: it never adds a command line or a work type.
- **JOB-016.** The build of a project and the test of a project are the first two job types
  that a caller can start. The steps come from the configuration of the project (PRO-009), not
  from the caller.

### Automatic routing to the right machine (R-234)

The operator, first-hand (R-234, 2026-11-19): "And then also the MCP layer just automatically does the routing for you. Like if this machine is not the right machine to run it, then the other machine is the right machine to run it." (16:36 PT), then "Multiplexer Q1: ... send MCP machine routing to the jobs and agent-runs work on row 4.7." and "Multiplexer Q2: automatic for all work." The Planner had recommended that routing suggests a machine and asks first. These requirements are not approved (task RT1). The security review RT7 comes before any routed start ships. Placement stays one pure function (USE-003, JOB-013, CMP-011, PRO-051); routing is the use of that function for work that an AI agent or a person starts with no machine named.

- **ROU-001.** When a caller (the `anima` command, an MCP tool, or a run) starts work and names no machine, the hub must pick the machine with the placement function and start the work there, with no question to the operator. "Work" is a job of a type in the fixed list (USE-006, JOB-015), an agent run ([agent-runs.md](agent-runs.md)), and an agent task that BOT-069 starts in a Herdr pane (AXT-012, AXT-026). [NEEDS CLARIFICATION: OD-ROU1, whether "all work" also means an interactive session or a command line on a device; the recommendation is no.]
- **ROU-002.** The right machine is the result of the placement function over: the requirement set (JOB-012: tools and versions from the inventory, CAP-023; the build target; free memory; free disk), the device policy (USE-004, USE-009), the state of the machine (online; no check of the machine Down), the work that runs on it now, and whether it holds the local copy of the repository. Among machines that fit, the order is [NEEDS CLARIFICATION: OD-ROU2]. The function takes the time as a parameter (CQ-021).
- **ROU-003.** Routing never widens access. It picks only from the machines that the caller's token may use (MCP-003, INT-003), that take work (USE-004), and where the needed tool is installed and granted (AXT grants for each device). It never installs a tool (AXT-021), never sets "takes work" on, never moves a secret to a machine that the work could not get without routing (JOB-010, JOB-020), and never uses a machine that the operator excluded (ROU-006).
- **ROU-004.** The result of a routed start must name the machine and give the reason in one line, for example "studio: has cargo 1.83 and 22 GB free; laptop refused: 2 GB free". The caller sees it in the reply of the command or of the MCP tool. The text is data (MCP-008).
- **ROU-005.** Each routing decision must write one audit entry (LOG-005, LOG-012 to LOG-020): the actor kind and the caller, the work type, the requirement set, each candidate with the reason it was refused, the machine picked, and the time. It holds no secret and no prompt text (LOG-007). `anima routing log` and the MCP read tool `anima_routing_log` read these entries.
- **ROU-006.** Pins. The operator can pin a machine for a project, for an agent or for one start, and can exclude a machine from routing (it still takes work that names it). A caller can name a machine; then routing does not run. A pinned or named machine that does not fit is refused with the reasons (JOB-014); routing must not pick a different machine in its place (as RTR-014: a named node is never replaced silently). [NEEDS CLARIFICATION: OD-ROU3.]
- **ROU-007.** When no machine fits, the hub must refuse the work at once and name each requirement that no machine meets (JOB-014). [NEEDS CLARIFICATION: OD-ROU4, whether the work waits in the queue for a time, or the client offers a VM ([cloud-machines.md](cloud-machines.md)).]
- **ROU-008.** If a machine stops during routed work, USE-007 applies: the hub routes the work again, and that is a new decision with its own audit entry. A run that wrote to a scratch ref starts again from its last commit (AGN-011).
- **ROU-009.** The tools that start routed work follow the MCP states (MCP-002): with nothing saved they are `allow` (R-120) and route with no question; `ask` makes each start a question; `off` hides them. Routing never changes a state.
- **ROU-010.** A limit: at most 30 routed starts for each caller in each hour, and at most the free slots of the machines. [NEEDS CLARIFICATION: OD-ROU5; the numbers are a recommendation.]
- **ROU-011.** The client shows the machine and the reason on each routed item of the list of what runs (PRO-052) and on the Agents page (AXT-051).
- **ROU-012.** The command line has `anima work start <type>` with the parameters of the type and an optional `--on <machine>`; MCP has the same as one change tool. [NEEDS CLARIFICATION: the names are a suggestion for the interface review.]
- **ROU-013.** A routed start from the Chat page follows ROU-009 and needs no click (CHAT-041, R-234). [NEEDS CLARIFICATION: OD-ROU6.] Each other tool of the Chat page that acts on a machine keeps the click.
- **ROU-014.** No routed start ships before the security review RT7 passes with no open finding of severity high. The review covers: a prompt injection that makes an agent start work on many machines; the reach of a write key; a secret on a machine that routing picked; the limits of ROU-010.

#### Tasks of routing

One pull request for each task. All are Not started. They follow W3 (the queue and the placement function, PRO-051) and the agent runs. None starts before RT1.

| Task | Wave | Content | Needs | State | Commits |
| --- | --- | --- | --- | --- | --- |
| RT0 | 0 | A record: the inputs of ROU-002 that exist (inventory CAP-023, free memory MET, checks) and a threat model of automatic starts for RT7 | — | Not started | — |
| RT1 | 1 | The operator approves ROU-001 to ROU-014 and decides OD-ROU1 to OD-ROU6 | The operator | Not started | — |
| RT2 | 1 | Docs: the contract text for the routed start and the audit entry (K1 schema; K10 if the claim carries the reason) | RT1 | Not started | — |
| RT3 | 2 | Hub: routing over the placement function, the reasons, the audit entry (ROU-002 to ROU-005, ROU-007) | RT2; W3 | Not started | — |
| RT4 | 2 | Hub: pins and exclusions (ROU-006), the limit (ROU-010), failover (ROU-008) | RT3 | Not started | — |
| RT5 | 3 | `anima work start`, `anima routing log`; the MCP change tool and `anima_routing_log` (ROU-009, ROU-012) | RT3 | Not started | — |
| RT6 | 4 | Client: the machine and the reason on routed items (ROU-011, AXT-051); the pin settings; mockup `agent-routing` frame 2 compared | RT5; AX19 | Not started | — |
| RT7 | 5 | The security review of ROU-014; entries in `need-human-testing.md`. No routed start ships before it passes | RT3 to RT6 | Not started | — |
| RT8 | 6 | Human check: an agent on the laptop starts a build with no machine named; it runs on the machine that fits, and the log gives the reason | RT7 | Not started | — |

| Task | Pass or fail | Check commands | Out of scope |
| --- | --- | --- | --- |
| RT0 | Pass when the record lists each input with its source and the threats with a proposed control; fail otherwise | `bun run quality:documents` | Code |
| RT1, RT2 | Pass when the ruling or the documents exist and the gates pass; fail otherwise | `bun run quality:register`, `bun run quality:documents` | Code |
| RT3 | Pass when the function is pure (a table test with fixed inventory and time), a machine without the grant or the tool never fits, no fit refuses with each reason, and each decision writes one audit entry with no secret; fail otherwise | `cargo nextest run -p anima-hub` | Pins |
| RT4 | Pass when a pinned machine that does not fit is refused and never replaced, an excluded machine is never picked, the 31st start in an hour is refused, and a stopped machine leads to a new decision; fail otherwise | `cargo nextest run -p anima-hub` | — |
| RT5 | Pass when the commands follow the output rules, the MCP tool follows `allow`, `ask` and `off`, and both pass their JSON-RPC tests (MCP-017); fail otherwise | `cargo nextest run -p anima-cli` | Sessions (OD-ROU1) |
| RT6 | Pass when the client shows the machine and the reason, and the mockup row names the commit; fail otherwise | `bun run --cwd apps/client test`, `bun run gates` | — |
| RT7 | Pass when no finding of severity high is open and the `HT-nnn` entries exist; fail otherwise | `bun run quality:documents` | Fixes outside the findings |
| RT8 | Pass when the operator sets the entry to Passed; fail otherwise | — | — |

Rules for each task:

- Always: work in a worktree from `origin/dev` and merge to `dev`; claim the task (PLN-022); keep CMD-030 order; run `bun run quality:register` and `bun run quality:documents` before each commit; write "Not verified" for a step you did not run.
- Ask first: a contract change (K1, K10); a new work type; a change to a limit of ROU-010.
- Never: a command line or a session from a routed start; a machine that the caller's token, the grants or USE-004 do not permit; an install of a tool by routing; a secret or a prompt in the audit entry; a fact of the business vault.

Prompt for each task: "Follow AGENTS.md 'Start of a session'. Claim and build task RTn of docs/proposals/jobs.md, section 'Automatic routing to the right machine'. Read that section, USE-001 to USE-009 and JOB-012 to JOB-015 first. Build only what the row of RTn says. Run the check commands of the row, then bun run quality:documents and bun run gates."

#### Open decisions of routing

| Number | Question | Recommendation, with strengths and weaknesses | State |
| --- | --- | --- | --- |
| OD-ROU1 | Does "automatic for all work" include an interactive session or a command line on a device (open point 1 of cli-mcp-plan.md, MCP-006)? (yes/no) | No: work of the fixed types and agent runs only. Strength: keeps JOB-015 and USE-006, so a prompt injection cannot run a command on each machine. Weakness: an agent cannot open a shell on another machine. | Open |
| OD-ROU2 | The order among machines that fit (one choice: the one with the local copy, then the most free memory; the most free memory only; the operator's fixed order of machines) | Local copy first, then most free memory. Strength: no transfer of the tree, and a fast start. Weakness: one machine can get most of the work. | Open |
| OD-ROU3 | Who can pin a machine? (one choice: the operator only, in settings; also a caller with `--on`) | Both. Strength: an agent that knows a reason can name a machine, and a named machine is never replaced. Weakness: a caller can always pick the strongest machine. | Open |
| OD-ROU4 | When no machine fits (one choice: refuse at once with the reasons; wait in the queue for up to 15 minutes; offer a VM) | Refuse at once. Strength: JOB-014 as it is; the agent can act on the reason. Weakness: work that would fit in a minute is refused. | Open |
| OD-ROU5 | The limit of ROU-010: 30 routed starts for each caller in each hour? (yes/no) | Yes, change later from the audit log. Strength: stops a loop or an injection from filling all machines. Weakness: a real burst can hit it. | Open |
| OD-ROU6 | Does a routed start from the Chat page also need no click (CHAT-041)? (yes/no) | Yes, as R-234 says "all work"; the MCP state `ask` stays available. Strength: one rule for routing. Weakness: the Chat page is weaker than the click rule of Hermes for this one tool. | Open |

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
  request for it comes, the machine uploads the file through the hub. The daemon must not listen
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
| Does MCP-006 stay? | Changed by R-234: an MCP tool may start work of a fixed type, or an agent run, on the machine that routing picks (ROU-001). No tool takes a command line (JOB-015). Open point 1 of [cli-mcp-plan.md](../plans/cli-mcp-plan.md) (a session for an agent) stays open (OD-ROU1). |
| Which token can start a job? | A write API key, with the MCP tool `off` until the operator allows it (MCP-002). |

## Decision (OD-W10, operator, 2026-11-12)

R-115 (operator, 2026-11-12) listed as **not taken**: "a command channel from the daemon to the
hub, because it would make the daemon a remote-execution endpoint (MET-013, MET-020); anything
that changes a device". K8 says that the daemon makes no request other than enrol, push and renew.
A job is remote execution. So are CAG-003 of the compute plan (install a runtime, start a
server), HOST-002 and the store machines of [bulk-data-placement.md](bulk-data-placement.md).

The options were: one daemon with work off by default (the recommendation), a second program
for compute machines, or R-115 kept as it is. The operator selected one daemon and changed the
default: **"it should be on by default"**.

What this means:

| Subject | Decided |
| --- | --- |
| Programs | One: `anima-daemon` (as [OD-C2](../work-items/decisions/OD-C2.md) says). No second binary. |
| Default | A machine with the daemon takes work (USE-004). The operator can set a machine off. |
| R-115 | Its "not taken: a command channel" line is replaced. The other lines of R-115 stay. R-160 (2026-11-18) is the ruling that does this. |
| MET-020 | Not changed: the daemon listens on no port and connects only to the hub (USE-005). |
| Compute plan CMP-002, [OD-C8](../work-items/decisions/OD-C8.md) | They say "off by default". They must change to agree. |

The cost that the operator accepts: a person who controls the hub can run work on each machine
that has the daemon. R-084 selected the daemon so that the hub needs no SSH login on each device
(OD-9); this decision gives the hub a second way in. These rules limit it, and none needs a step
from the operator:

- The daemon accepts only the work types in a fixed list (USE-006). There is no "run this
  command" type. After OD-W13 the type `runner` runs the steps that a workflow file names
  (JOB-011). The hub still cannot name a command.
- Jobs run in containers (USE-001), with no access to the daemon token or the home folder. A
  native job on a Mac has the same two limits through its build user (PRO-022).
- A machine that is set off at the machine cannot be set on from the hub (USE-004).
- Each item of work is in the audit log (LOG-005), and the machine keeps its own log of the work
  that it took.

This decision comes before [C1](../work-items/compute/C1.md), not only before jobs.

R-160 (operator, 2026-11-18: "A: on everywhere") confirms this decision also for a hosted hub on the pool: the default is on for each hub. The limits above stay.

## Needs

- Done: the ruling that changes R-115 is R-160 (2026-11-18). Still open: the transport decision in [C1](../work-items/compute/C1.md) and
  [SPEC-C](../work-items/compute/SPEC-C.md), and the new default in [OD-C8](../work-items/decisions/OD-C8.md).
- A new contract for the work route. K8 is frozen for Phase 3 (CON-001), so this is a new
  contract or `/daemon/v2/`, not a change to K8.
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
