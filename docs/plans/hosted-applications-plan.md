---
kind: Plan
state: In progress
evidence: Approved by R-149 (operator, 2026-11-18): stage 1 (the containers of a machine, read-only, with a screen in the client) and stage 2 (start, stop, install, update, remove, the catalogue). Stage 2 waits for the work route (task W1). HOST-004 to HOST-009 are marked "Not approved". Version 2 (2026-11-18). All tasks after H0 are Not started. OD-HOST-1 and OD-HOST-2 are open.
---

# Plan: hosted applications (containers that run all the time)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version: 2. Date: 2026-11-18. Version 1 (2026-11-12) was the proposal. Version 2 is the plan: R-149
approves it in two stages and decides the open decisions of version 1.
From: the planning session of 2026-11-12. The operator wants easy self-hosting of published
applications on the owned machines: the container management part of Portainer, not the build
pipeline of Coolify. On 2026-11-18 the operator said: "a portainer clone also does need to be built
into this app if that isn't already there", and "would also like a portainer like UI for running
containers".
Status: the header of this document holds the state (PLN-020). History: a proposal until 2026-11-18;
the operator approved it on 2026-11-18 (R-149). A requirement with the words "Not approved" has no
ruling and is not built (PLN-007).
Map: [work-plan.md](../work-plan.md), category Use, row 4.9.
Identifiers: HOST-001 to HOST-014.

## Problem

The operator runs published applications in containers on different machines and manages each
one with SSH. Anima does not show them, cannot start or update them, and does not know which
machine has the data. Spec Section 1.1 says that a later version may run containers (R-109).

## What exists (2026-11-18)

- The Docker container check: the hub reads the state of one named container through its SSH link
  (CHK-012, `crates/hub/src/fleet/checks/probe/docker.rs`). It keeps nothing of the output.
- The daemon reports the installed versions of `docker` and `podman` (the tool inventory).
- A web interface entry can open the page of a Portainer that the operator installed (WEB-001). The
  mockup shows this link on the device `docker-host`. There is no container screen.
- Nothing lists the containers of a machine, shows a log, or starts, stops, installs, updates or
  removes a container.
- The hub cannot give work to the daemon. The daemon only pushes
  ([daemon-push.md](../../contracts/daemon-push.md)). The work route is task W1 of
  [projects-and-builds-plan.md](projects-and-builds-plan.md), and it waits for a ruling that changes
  R-115 ([jobs.md](../proposals/jobs.md), OD-W10).

## Requirements

A hosted application is a container that runs all the time on one machine. Anima installs
published applications. Anima is not a build platform.

### Stage 1: see (approved, R-149; it can be built now)

- **HOST-001.** The operator must be able to see the containers on each machine, with their
  state and their logs. For each container: the name, the image, the state, the status text, the
  ports and the name of its Compose project, if it has one. The log is the last lines of one
  container; the operator selects the number of lines, to a fixed maximum.
- **HOST-010.** Anima manages Docker only. Podman is not planned.
- **HOST-013.** The client has a container screen for each machine: a table of the containers,
  grouped by Compose project, with the state of each, and the log of the selected container. In
  stage 2 the same screen has the controls of HOST-002 and HOST-003. The hub API, the `anima`
  command and the MCP server give the same list (CMD-030).

How stage 1 reads a machine: the hub runs fixed read-only `docker` commands through its SSH link,
as the Docker check does. This is the "read-only service discovery" and the "log tail over the
hub's SSH" that R-115 lists as later work. It needs no work route and it changes no machine. These
rules apply:

- The command is static text. A container is named by its identifier, and the hub accepts an
  identifier only if it is 12 to 64 hexadecimal characters.
- The hub does not keep the list or the log, and does not write them to its own log (LOG-007). A log
  of a container can hold a secret, so no MCP tool returns it (MCP-004); the list has an MCP tool.
- A machine with no SSH credential in Anima has no list in stage 1. The answer says so. Stage 2 can
  read such a machine through the daemon.

### Stage 2: control (approved, R-149; it waits for task W1)

- **HOST-002.** The operator must be able to start, stop and update a container.
- **HOST-003.** The operator must be able to install an application from a recipe. A recipe is a
  Docker Compose file.
- **HOST-014.** The operator must be able to remove an application. Anima asks before it removes
  the data of the application.
- **HOST-011.** A hosted application runs on a machine of the operator, through the daemon. It does
  not run in the container of a hosted hub: the pool replaces that container, so data there is not
  safe (R-130, R-139).
- **HOST-012.** Anima has a catalogue of recipes. The first entry is OwnTracks Recorder; the second
  is Dawarich. Anima uses the published images and copies no code of them (they are GPL and AGPL).
  The location samples of such an application reach Places through the ingestion contract of the
  engine (R-145, K-E4). This plan does not define that contract.

HOST-002, HOST-003 and HOST-014 change a device. They use the work route of
[jobs.md](../proposals/jobs.md) with fixed work types (USE-006); the hub cannot name a command.

### Not approved

No ruling names these requirements. They are not built (PLN-007).

- **HOST-004.** Not approved. Each hosted application has one home machine (ETH-003).
- **HOST-005.** Not approved. An application that keeps data must have a copy of its data on a
  second machine.
- **HOST-006.** Not approved. The operator must be able to move an application to the machine that
  has the copy. Anima must not move it automatically (ETH-004).
- **HOST-007.** Not approved. An application that keeps no data can start on a different machine
  when its home stops.
- **HOST-008.** Not approved. Each hosted application must get a check (spec Section 6.4) and a web
  interface entry (WEB-001).
- **HOST-009.** Not approved. Anima does not build source code, supply a reverse proxy, or make
  preview deployments. Use Vercel or Coolify for these.

## Work that already exists

- [deferred.md](../deferred.md), "Ideas from Coolify (hub)": the rules that make one status from many containers,
  and the image and package scan.
- [deferred.md](../deferred.md), "Ideas from Coolify Sentinel": how to calculate container metrics.
- Spec Section 6.4: the Docker container check, and CHK-014 for restart loops.

## Reference: Arcane

The operator named Arcane (`https://github.com/getarcaneapp/arcane`) on 2026-11-18 as a product to
learn from for this plan. It is a reference. It adds no requirement and changes no task.

- Licence: BSD 3-Clause. Code can be copied if its copyright notice stays with the copied part.
- Its client is Svelte 5 with SvelteKit, as `apps/client` is. Its styles are Tailwind and
  shadcn-svelte, and its folders are named `components/`. A copied part must use the design tokens
  and the names of this repository (`AGENTS.md`).
- Its server is Go and uses the Docker socket. Stage 1 here is the hub with fixed commands through
  its SSH link. Read its server to learn the behaviour; do not copy it.
- For task H3: the container table and its helpers (`frontend/src/routes/(app)/containers/`), with
  the state of each container, the group of a Compose project, and the log panel.
- For tasks H6 and H7: the screens for a Compose project and for templates
  (`frontend/src/routes/(app)/projects/`).
- Not used: Podman, Swarm, GitOps, image builds and the vulnerability scan (HOST-009, HOST-010).

## Decisions

| Decision | Question | Result |
| --- | --- | --- |
| OD-HOST-A | Is "darawich" Dawarich, the program on the operator's server (R-050)? | Yes (R-149). |
| OD-HOST-B | Where do applications run: the operator's machines, the container of the hosted hub, or both? | The operator's machines only, through the daemon (R-149, HOST-011). |
| OD-HOST-C | What does the first version manage? | Two stages. Stage 1: the list with state and logs. Stage 2: start, stop, install from a Compose file, update and remove (R-149). |
| OD-HOST-D | Docker only, or Docker and Podman? | Docker only (R-149, HOST-010). |
| OD-HOST-E | Which applications are first in the catalogue? | OwnTracks Recorder, then Dawarich (R-149, HOST-012). |
| OD-HOST-F | Does row 4.9 move to a higher tier? | No move. Stage 1 is built now because it needs nothing of a higher row; stage 2 waits for task W1 of row 4.7 (R-149). |

## Open decisions

| Decision | Question | Recommendation |
| --- | --- | --- |
| OD-HOST-1 | How is the data copy of HOST-005 made: a backup of the volume, or replication that the application supplies? | A backup of the volume, kept by [bulk-data-placement.md](../proposals/bulk-data-placement.md). It is asked when HOST-004 to HOST-008 come to approval; it blocks no task of this plan. |
| OD-HOST-2 | Do you approve HOST-004 to HOST-009 (the home machine, the data copy, the move, a check and a web interface for each application, and no build platform)? | Yes, after stage 2 is in use. It blocks no task of this plan. |

## Tasks

The order is hub API, `anima` command, MCP tool, client (CMD-030). A task is built from its row
(PLN-013).

| Task | Content | Needs | State | Commits |
| --- | --- | --- | --- | --- |
| H0 | The decisions of the operator: the approval, the stages, Docker only, where applications run, the catalogue, the tier. | Nothing | Done (R-149, 2026-11-18) | The commit that adds this row |
| H1 | Stage 1, the hub: the containers of a device and the last lines of the log of one container, through the SSH link with fixed commands (HOST-001, HOST-010). The additions to K1 and K3 (R-149, CON-001). | H0 | Not started | |
| H2 | Stage 1, the `anima` command (`anima containers list`, `anima containers logs`) and the MCP tool for the list. No MCP tool returns a log (MCP-004). | H1 | Not started | |
| H3 | Stage 1, the client: the container screen of a machine (HOST-013), and its mockup in the same commit (PLN-019). | H1 | Not started | |
| H4 | The check of stage 1 on a real machine that runs Docker. An entry in [need-human-testing.md](../need-human-testing.md) (PLN-014). It changes no server of the operator: it only reads (VPS-005, R-050). | H1 to H3 | Not started | |
| H5 | Stage 2: start, stop and restart a container (HOST-002), with the controls in the screen of H3. | Task W1 of [projects-and-builds-plan.md](projects-and-builds-plan.md) (the work route); H3 | Not started | |
| H6 | Stage 2: install from a Compose file, update and remove (HOST-002, HOST-003, HOST-014). | H5 | Not started | |
| H7 | The catalogue, with OwnTracks Recorder and then Dawarich (HOST-012). Task T6 of E0-5 in [extensions-plan.md](extensions-plan.md) (R-145) is a quick way to deploy an OwnTracks server; the session that builds H7 and the engine session agree which recipe is the one recipe, so that there are not two. | H6; K-E4 of the engine | Not started | |

## Needs

- The work route of [jobs.md](../proposals/jobs.md) for stage 2 (task W1, row 4.7 of the priority
  order). Stage 1 does not need it.
- The ingestion contract of the engine for the location samples (R-145). The engine owns it.
