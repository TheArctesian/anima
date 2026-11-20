---
kind: Plan
state: In progress
evidence: Approved by R-149 (operator, 2026-11-18): stage 1 (the containers of a machine, read-only, with a screen in the client) and stage 2 (start, stop, install, update, remove, the catalogue). Stage 2 waits for the work route (task W1). HOST-004 to HOST-009 are marked "Not approved". Version 2 (2026-11-18). Version 4 (2026-11-18): R-150 approves HOST-015 to HOST-025, which came from a comparison with Arcane, in three groups (with stage 1, with stage 2, and HOST-024 after a security review); the tasks are H8 to H12. Task H8 is built on 2026-11-18 (HOST-017, HOST-019, HOST-020) and waits for the check HT-038; HOST-021 moved to the new task H13 (version 5), which is built on 2026-11-19 and waits for the check HT-039. Tasks H1 to H3 (stage 1) are built on 2026-11-18 (commits `9fb61504`, `9b5fa08b`, `1bb37a55`) and wait for the check HT-032 of task H4 on a real machine; task H9 (the client for H8 and H13) is built on 2026-11-19 and waits for the check HT-045; tasks H5 to H7 and H10 to H12 are Not started. OD-HOST-1 and OD-HOST-2 are open.
---

# Plan: hosted applications (containers that run all the time)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version: 5. Date: 2026-11-18. Version 5 records the build of task H8 and moves HOST-021 from H8 to the new task H13. Version 1 (2026-11-12) was the proposal. Version 2 is the plan: R-149
approves it in two stages and decides the open decisions of version 1. Version 3 adds the comparison
with Arcane and the proposed requirements HOST-015 to HOST-025 (operator, 2026-11-18: "find more of
their features and styling components that we could use, and expand our plan"). Version 4 records
the approval of those requirements (R-150) and adds the tasks H8 to H12.
From: the planning session of 2026-11-12. The operator wants easy self-hosting of published
applications on the owned machines: the container management part of Portainer, not the build
pipeline of Coolify. On 2026-11-18 the operator said: "a portainer clone also does need to be built
into this app if that isn't already there", and "would also like a portainer like UI for running
containers".
Status: the header of this document holds the state (PLN-020). History: a proposal until 2026-11-18;
the operator approved it on 2026-11-18 (R-149). A requirement with the words "Not approved" has no
ruling and is not built (PLN-007).
Map: [work-plan.md](../work-plan.md), category Use, row 4.9.
Identifiers: HOST-001 to HOST-025.

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
- The log query is closed to every API key, as the file content of a repository is (D-23): a login
  or the browser session reads it. The list is open to any token.

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

### From the comparison with Arcane (approved, R-150)

The operator asked for these on 2026-11-18 as an expansion of the plan and approved them the same
day (R-150). Each one says how it reads or changes a machine, because Arcane uses the Docker socket
and Anima does not. With stage 1: HOST-015, HOST-017, HOST-019, HOST-020 and HOST-021. With stage 2:
HOST-016, HOST-018, HOST-022 and HOST-023. After a security review of the session: HOST-024. With
the catalogue: HOST-025.

- **HOST-015.** The log of a container has a search, a filter by level, a switch for
  line wrap, a switch for timestamps and a download of the lines that are shown. The client does this
  work on the lines that HOST-001 gives. No new hub function.
- **HOST-016.** The log of a container can follow new lines. It needs a stream from
  the machine, so it waits for the daemon path of stage 2. No MCP tool returns it (MCP-004).
- **HOST-017.** The operator can see the details of one container, read-only: the
  command, the restart policy, the health check and its last result, the mounts, the networks with
  the addresses, the labels and the processes. The values of environment variables can hold a
  secret: the hub gives the names only (LOG-007), and no MCP tool returns them.
- **HOST-018.** Each row of the container table can show the processor use and the
  memory use of the container. The daemon measures them and pushes them with its readings; the hub
  starts no command for them. [deferred.md](../deferred.md), "Ideas from Coolify Sentinel", has the
  calculation.
- **HOST-019.** A Compose project has one state that the hub makes from the states of
  its containers, for example "Partly running" when one container of three has stopped. One rule
  gives the same state in the client, the `anima` command and the MCP tool.
  [deferred.md](../deferred.md), "Container status rules", has the order of the states.
- **HOST-020.** The operator can see the images, the volumes and the networks of a
  machine, read-only, with the size of each image and volume, the containers that use it, and the
  space that a clean-up can free.
- **HOST-021.** Anima shows when a newer image exists for a container: the hub
  compares the digest of the image on the machine with the digest of the same tag in its registry.
  It reads public registries only in the first version. It updates nothing by itself (ETH-004); the
  update is HOST-002.
- **HOST-022.** The install of HOST-003 has an editor for the Compose file and for
  its environment file, with a check of the syntax before the install, and a function that changes
  a `docker run` command into a Compose file. The values of the environment file are a secret of
  the operator: the hub keeps them sealed and writes none of them to a log (LOG-007).
- **HOST-023.** The operator can remove stopped containers, unused images, unused
  networks and unused volumes of a machine. Anima shows what each choice removes and how much
  space it frees, and asks before it removes a volume (HOST-014). It is a fixed work type of the
  work route (USE-006).
- **HOST-024.** The operator can open a terminal in a running container. It is a
  session of the hub (design specification Section 6.3), with the same access rules and the same
  record as an SSH session. No MCP tool opens it (cli-mcp-plan.md, Section 7).
- **HOST-025.** The catalogue of HOST-012 reads a list of recipes from an address
  that the operator adds. Each entry has an identifier, a name, a description, a version, an
  author, tags, and the addresses of the Compose file, the environment file, the documentation and
  the icon. This is the form of the template registry of Arcane, so a list that is made for Arcane
  also works in Anima (OD-HOST-4). Anima shows the Compose file before the install and installs
  nothing by itself.

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
learn from for this plan, and asked on the same day for a wider comparison. This section is a
reference. It adds no requirement. The comparison read the commit `40e65a7` (2026-11-18).

- Licence: BSD 3-Clause. Code can be copied if its copyright notice stays with the copied part.
- Its client is Svelte 5 with SvelteKit, as `apps/client` is. Its styles are Tailwind and
  shadcn-svelte, and its folders are named `components/`. A copied part must use the design tokens
  and the names of this repository (`AGENTS.md`).
- Its server is Go and uses the Docker socket. Stage 1 here is the hub with fixed commands through
  its SSH link. Read its server to learn the behaviour; do not copy it.

### Functions

Paths are from `frontend/src/routes/(app)/` of Arcane, unless the row gives a different root.

| Function of Arcane | Where | Use in Anima |
| --- | --- | --- |
| Container table: name, image, state, status text, ports, address, created; rows in groups by Compose project | `containers/components/container-table.svelte`, `containers/container-table.helpers.ts` | Task H3 (HOST-001, HOST-013) |
| Log viewer: tail, search, level filter, wrap, timestamps, download, structured JSON lines, ANSI colours | `frontend/src/lib/components/logs/` | Task H3 for the tail. The other functions are HOST-015 and HOST-016 |
| Container detail: overview, configuration, environment, storage, network, health check, processes, inspect | `containers/components/container-*.svelte` | HOST-017 |
| Processor and memory use in each row and on the detail page | `containers/components/container-stats*.svelte` | HOST-018 |
| Start, stop, restart, pause, redeploy, update, remove, kill | `containers/container-table.actions.ts` | Tasks H5 and H6 (HOST-002, HOST-014). Pause and kill are not planned |
| Compose projects: table, services panel, logs of the project, one state for the project | `projects/` | Task H6 (HOST-003). The one state is HOST-019 |
| Compose editor, environment editor, `docker run` converter | `frontend/src/lib/components/compose/` | HOST-022 |
| Templates: gallery, card, registries that the operator adds, default template, global variables | `projects/templates/`, `projects/variables/`, `backend/internal/template/` | Task H7 (HOST-012). The registry form is HOST-025 |
| Images, volumes, networks, ports: tables and detail pages | `images/`, `volumes/`, `networks/`, `ports/` | HOST-020 |
| Updates: containers and projects that have a newer image | `updates/`, `backend/internal/imageupdate/` | HOST-021 |
| Prune, with a choice of what to remove | `frontend/src/lib/components/prune/` | HOST-023 |
| Terminal in a container | `containers/components/container-shell.svelte` | HOST-024 |
| Dashboard: a list of what needs attention, space that can be freed, the containers that use most memory | `dashboard/` | Not in this plan. The device list and the checks own the overview (design specification Section 6) |
| Events of Docker | `events/` | Not in this plan. The event log of Anima owns events (design specification Section 3.8) |
| Volume file browser and volume backup | `volumes/`, `frontend/src/lib/components/file-browser/` | Not in this plan. HOST-005 and OD-HOST-1 own the copy of data |
| Podman, Swarm, GitOps, image builds, vulnerability scan, hardened images | `podman/`, `swarm/`, `gitops/`, `images/build*`, `security/` | Not used (HOST-009, HOST-010). The CVE proposal owns scans |
| Users, roles, OIDC, API keys, notifications, agents | `system/`, `agents/`, `account/` | Not used. Anima has these functions |

### Parts of the interface

Paths are from `frontend/src/lib/components/` of Arcane. "Anima has" names the part of
`apps/client/src/lib/` that does the same job. Change that part; do not add a second one.

| Part of Arcane | What it does | Anima has | Use |
| --- | --- | --- | --- |
| `arcane-table/` | A table with sort, text filter, filters by value, groups, column choice, pages, and a card layout on a narrow screen | `tables/`, `rows/` | Take the groups and the card layout as a design, if `tables/` has none. Do not take its code: it is built on shadcn |
| `logs/` | Log rows, controls, structured entries | `process-log/`, `sessions/` | Take `log-viewer.utils.ts` (levels, search, ANSI) as the model for HOST-015 |
| `lifecycle-indicator/`, `action-buttons.svelte` | Shows "starting", "stopping", "restarting" on a row while an action runs | `motion/`, `states/` | Take the behaviour for task H5 |
| `badges/port-badge.svelte`, `badges/label-badges.svelte` | A port as a link to its address; labels as small marks | None | Take for task H3 |
| `stat-strip/`, `stat-card.svelte`, `metric-ring.svelte`, `memory-ring.svelte` | Small numbers and rings for use | `charts/`, `metrics/` | Use the charts of Anima for HOST-018. Take no ring |
| `resource-detail/` | Sections and key and value grids of a detail page | `page/`, `rows/` | Take the layout as a design for HOST-017 |
| `network-diagram/` | A drawing of the networks and their containers | None | Later, with HOST-020. Not in the first version |
| `compose/`, `code-editor/`, `resizable-split.svelte`, `editor-tab-strip.svelte` | The Compose editor with a split view | None | Take for HOST-022, after a check of the size of the editor library |
| `terminal/` | xterm with controls | `sessions/` | Not taken. Anima has a terminal |
| `prune/prune-mode-picker.svelte`, `confirm-dialog/`, `operation-watch-dialog.svelte` | The choice of what to prune; a dialog that shows the steps of a long action | `overlay/` | Take the two behaviours for HOST-023 and task H6 |
| `states/empty-state.svelte`, `ui/skeleton`, `virtual-rows.svelte`, `pinned-scroll-region.svelte` | Empty and loading states, long lists | `states/`, `scroll/` | Not taken. Anima has these |
| `lib/themes/*.json`, `accent-color/`, `application-theme/` | 12 colour themes, an accent colour, a theme import | `theme/`, `palette/`, `design/tokens/palettes/` | Not taken. The tokens of Anima own colour (design specification Section 5) |

## Decisions

| Decision | Question | Result |
| --- | --- | --- |
| OD-HOST-A | Is "darawich" Dawarich, the program on the operator's server (R-050)? | Yes (R-149). |
| OD-HOST-B | Where do applications run: the operator's machines, the container of the hosted hub, or both? | The operator's machines only, through the daemon (R-149, HOST-011). |
| OD-HOST-C | What does the first version manage? | Two stages. Stage 1: the list with state and logs. Stage 2: start, stop, install from a Compose file, update and remove (R-149). |
| OD-HOST-D | Docker only, or Docker and Podman? | Docker only (R-149, HOST-010). |
| OD-HOST-E | Which applications are first in the catalogue? | OwnTracks Recorder, then Dawarich (R-149, HOST-012). |
| OD-HOST-3 | Are HOST-015 to HOST-024 approved, and in which stage is each one? | Approved in three groups: with stage 1, HOST-015, HOST-017, HOST-019, HOST-020 and HOST-021; with stage 2, HOST-016, HOST-018, HOST-022 and HOST-023; after a security review of the session, HOST-024 (R-150). |
| OD-HOST-4 | Does the catalogue read the form of the template registry of Arcane (HOST-025)? | Yes, for reading. A list of a different person is off until the operator adds its address (R-150). |
| OD-HOST-F | Does row 4.9 move to a higher tier? | No move. Stage 1 is the first to build, because it needs nothing of a higher row; stage 2 waits for task W1 of row 4.7 (R-149). |

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
| H1 | Stage 1, the hub: the containers of a device and the last lines of the log of one container, through the SSH link with fixed commands (HOST-001, HOST-010). The additions to K1 and K3 (R-149, CON-001). | H0 | Built (2026-11-18). Hub domain `crates/hub/src/work/containers/`; `Query.containers`, `Query.containerLog`. `containerLog` is closed to every API key, as `codeFile` is (D-23). The log has the time of each line, for HOST-015. The check on a real machine is HT-032. | `9fb61504`, `4dcc8045` (corrections of the review) |
| H2 | Stage 1, the `anima` command (`anima containers list`, `anima containers logs`) and the MCP tool for the list. No MCP tool returns a log (MCP-004). | H1 | Built (2026-11-18). `anima containers logs` takes the name of a container or its identifier. MCP tool `anima_container_list` (tool set `devices`). The check is HT-032. | `9b5fa08b`, `4dcc8045` (corrections of the review) |
| H3 | Stage 1, the client: the container screen of a machine (HOST-013), and its mockup in the same commit (PLN-019). | H1 | Built (2026-11-18). The screen is the tab "Containers" of the device page (design specification Section 6.7); mockup `docs/mockup/containers/index.html`, not compared by a person. The phone layout has no special form. The check is HT-032. | `1bb37a55`, `4dcc8045` (corrections of the review) |
| H4 | The check of stage 1 on a real machine that runs Docker. An entry in [need-human-testing.md](../need-human-testing.md) (PLN-014). It changes no server of the operator: it only reads (VPS-005, R-050). | H1 to H3 | The entry is written: [HT-032](../need-human-testing.md), Open. The task is done when the operator sets the entry to Passed (PLN-014). | The commit that adds this text |
| H5 | Stage 2: start, stop and restart a container (HOST-002), with the controls in the screen of H3. | Task W1 of [projects-and-builds-plan.md](projects-and-builds-plan.md) (the work route); H3 | Not started | |
| H6 | Stage 2: install from a Compose file, update and remove (HOST-002, HOST-003, HOST-014). | H5 | Not started | |
| H7 | The catalogue, with OwnTracks Recorder and then Dawarich (HOST-012). The catalogue reads a list of recipes in the form of HOST-025 (R-150). Task T6 of E0-5 in [extensions-plan.md](extensions-plan.md) (R-145) is a quick way to deploy an OwnTracks server; the session that builds H7 and the engine session agree which recipe is the one recipe, so that there are not two. | H6; K-E4 of the engine | Not started | |
| H8 | Stage 1, more to read, in the hub, the `anima` command and the MCP tool (CMD-030), with the additions to K1 and K3 (CON-001): the details of one container, with the names of the environment variables and no values (HOST-017); the one state of a Compose project (HOST-019); the images, the volumes and the networks of a machine (HOST-020). Fixed read-only commands through the SSH link, as H1. No MCP tool returns the details of a container. The newer image (HOST-021) is task H13: it needs a new outbound path of the hub. | H1, H2 | Built (2026-11-18). `Query.containerDetail` (closed to every API key, as `containerLog` is: a command, a label or a health-check command can hold a secret; D-23), `Query.containerStorage`, `Query.containerNetworks`, and the field `Container.composeProjectState` with the enum `ComposeProjectState`, whose description in K1 is the one rule. `anima containers inspect`, `images`, `volumes` and `networks`; `anima containers list` shows the state of the project. MCP tools `anima_container_storage` and `anima_container_networks` (tool set `devices`). A size is the text of Docker: Docker gives no exact number in these lists. The names of the containers of an image come from the name that each container gives; the number is the count of Docker. Limits that are known: a container that ended counts as one that does not run, with any exit status, so a project with a set-up container that finished is "Partly running" (the list of Docker cannot tell it from a stopped service); a Docker older than about 20.10 can refuse the command of `containerStorage`; an independent review by reading found no security defect, and its corrections are in the third commit. The command lines ran on Docker 29.9.3 of the build machine, not through the hub on a device: the check is [HT-038](../need-human-testing.md). | `285e4062`, `58f4f3ed`, and the commit that adds this text (corrections of the review) |
| H9 | Stage 1, the client for H8: the detail of a container, the state of a project in the table, the lists of images, volumes and networks, and the mark for a newer image; and the log functions of HOST-015, which need no hub function. The mockup in the same commit (PLN-019). An entry in [need-human-testing.md](../need-human-testing.md) for a real machine (PLN-014). | H3, H8; H13 for the mark of a newer image | Built (2026-11-19). In the tab "Containers" of the device page (`apps/client/src/lib/containers/`): a first row selects containers, images, volumes or networks; a group header has the state of its project; "Details" of a row shows the details in the place of the log; "Look for newer images" asks the hub, and each row then has its mark; the log has a search, a filter by the level word of a line, a switch for line wrap, a switch for timestamps and a download of the lines that are shown. Each new read starts only when the operator asks. The level of a line is a field that names it (`level=warn`), or else its first level word in capitals, in brackets or before a colon, in its first 120 characters; a line with no level has the level of the line before it, so a stack trace stays with its error. Programs write levels in many forms: the filter reads the common ones, and HT-045 asks which it misses. An arrow key in the two choices of the screen only moves the focus, so it starts no read. An independent review by reading found no path from the text of a device to markup or to a URL; its corrections are in the second commit. The mockup has two new frames; it is not compared by a person. The phone layout has no special form. The check is [HT-045](../need-human-testing.md). | `e1893ba9`, and the commit that adds this text (corrections of the review) |
| H10 | Stage 2, through the daemon: the log that follows new lines (HOST-016) and the processor and memory use of each container (HOST-018), with the change to the daemon contract that they need (K8, CON-001). | H5 | Not started | |
| H11 | Stage 2: the editor for the Compose file and its environment file, the syntax check and the `docker run` function (HOST-022); the clean-up of a machine (HOST-023). | H6 | Not started | |
| H12 | The terminal in a running container (HOST-024). First a security review of the session, with its findings in [security-findings.md](../security-findings.md); the build starts only when the review has no open finding of this task. | H5 | Not started | |
| H13 | Stage 1: a newer image for a container (HOST-021), in the hub, the `anima` command and the MCP tool (CMD-030), with the addition to K1 (CON-001). The hub reads the digest of the image on the machine with a fixed read-only command, and the digest of the same tag at its registry. This is a new outbound path of the hub: it reads only a fixed list of public registries, each hop passes the address policy as the RDAP path does (`fleet/checks/probe/rdap/egress.rs`), it sends no credential, and it updates nothing (ETH-004). An image of a registry that is not in the list has the answer "not known". | H8 | Built (2026-11-19). `Query.containerImageUpdates` gives for each container the state `UP_TO_DATE`, `NEWER_IMAGE` or `NOT_KNOWN` with a reason. The hub reads Docker Hub, ghcr.io, quay.io and mcr.microsoft.com, with the token that each gives to a reader with no account, and asks for the manifest with `HEAD`; it follows no redirect. A repository or a tag reaches a URL only when each character is one that a registry permits. At most 40 different tags for each read, 10 seconds for each request, 25 seconds for each tag. `anima containers updates`; MCP tool `anima_container_updates`. Nothing is kept: each read asks the registries again, and nothing limits how often a caller asks; a short cache and one limit for the whole hub are a later change, wanted before many accounts share the address of a pool (a registry can limit a reader with no account by address). The rules of the pinned connection are a second copy of those of the RDAP path; one shared unit is a later change of both. An independent review by reading found no path from the text of a device to the host or the path of a request; its corrections are in the second commit. The check with real registries and a real device is [HT-039](../need-human-testing.md). | `e71b5fb2`, `ffbc306e` (corrections of the review), `26f2a0ef` |

## Needs

- The work route of [jobs.md](../proposals/jobs.md) for stage 2 (task W1, row 4.7 of the priority
  order). Stage 1 does not need it.
- The ingestion contract of the engine for the location samples (R-145). The engine owns it.

## Council follow-up: correct host and Compose provenance (2026-11-18)

R-152 names the preparation below. It changes no approved requirement or contract. Read current `main` and the claims before taking a task. A source review is not a Passed human check. The task owner records the tested build and the result; unknown results stay unknown.

| Task | Work and acceptance | Depends on | State |
| --- | --- | --- | --- |
| H-C1 | Reconcile H4 with two cases: known-running Docker on the correct host reached through the hosted route, and that same host with the route denied. Distinguish unreachable host from stopped container. Give H4's owner the acceptance gap, or record that the governing acceptance text is missing. For a proposed Compose provenance display, include equal project names on different hosts and unknown provenance. A path on screen does not authorize adopting or updating a deployment. | H1 to H4 owner; hosted TCP/SSH route; operator target for a live check; separate adoption decision before writes to existing external projects | Not started. Preparation only; no new H1 build or live check |
