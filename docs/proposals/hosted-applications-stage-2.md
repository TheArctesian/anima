---
kind: Proposal, with a task table
state: Direction decided
evidence: R-209 to R-216 (operator, 2026-11-19, relayed) decide OD-HOST-5 to OD-HOST-12 as recommended. Approved: HOST-027, HOST-028, HOST-031 (no change of the file), HOST-037, HOST-039 and HOST-041, and HOST-008 of the plan; HOST-009 of the plan is replaced (R-213). Section 4 is the text of the container work types for a security review (R-209): it is not part of contract K10, and no code of a container work type on a machine is built before it is version 0.4. Not approved: HOST-026, HOST-029, HOST-030, HOST-032 to HOST-036, HOST-038, HOST-040, HOST-042 and HOST-043; Section 9 says what each waits for. The approved requirements and the tasks that the rulings name are in the task table of hosted-applications-plan.md. Version 2 (2026-11-19).
---

# Proposal: hosted applications, stage 2 and the stage after it

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this document (PLN-005).

Version: 2. Date: 2026-11-19. Version 1 was the proposal with eight open decisions. Version 2 records the answers of the operator (R-209 to R-216) and adds Section 9, the state of each requirement.
From: the operator, 2026-11-19, relayed by the orchestrating session and not first-hand in the session that wrote this: the container side should be "fleshed out more via the competitor research", and "the coolify/dockploy and the portainer things are still competitors and things I want to emulate ish".
Status: the header of this document holds the state (PLN-020).
Map: [work-plan.md](../work-plan.md), category Use, row 4.9 of the [priority order](../plan/priorities.md). The plan that this proposal changes when it is approved is [hosted-applications-plan.md](../plans/hosted-applications-plan.md). The contract that it changes is K10, [work-route.md](../../contracts/work-route.md).
Identifiers: HOST-026 to HOST-043; open decisions OD-HOST-5 to OD-HOST-12; tasks H14 to H27.

## 1. What this document is for

Stage 1 of hosted applications is built: Anima reads the containers of a machine. Stage 2, which
changes a machine, is approved (R-149, R-150) and has not started. It waits for the work route
(task W1 of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md)).

The research found that stage 2 waits for more than task W1. The work route accepts only the work
types of a fixed list, and the list has two types: `build` and `runner`. Container work is not in
it. Each type is added with a ruling (contract K10, point W1-P5; CON-001). Thus no stage 2 task can
start when the operator accepts the contract as it is today.

This document gives:

- the container work types as proposed contract text (Section 4), so that the operator can accept
  the contract and approve these types in one decision;
- the stage 2 tasks, checked against the research (Section 5);
- new tasks for stacks that already run and for a check of each application (Section 5);
- a stage 3 for domains and certificates, deploys from git, databases, backups and rollback
  (Section 6);
- the questions that only the operator can answer, in the order to ask them (Section 7);
- what an agent can build on the day of the first answer (Section 8).

## 2. The research, and what was checked again

The research is a file outside the repository, written on 2026-11-19 by a research agent from
pages that it read through a tool that summarises a page. This proposal treats each statement of
the research as a claim. It builds a requirement on a claim only when the claim was read again at
its source, or when the requirement does not depend on it.

| Claim | Checked again | Result |
| --- | --- | --- |
| Portainer 3 has no Community Edition; CE 2.45 is the last free release | The announcement of Portainer of 2026-11-06, read again on 2026-11-19 | Confirmed. The page says that Portainer 3 "has no Community Edition", that "CE 2.45 is the last version of CE", and that it gets no fixes "after August 2027". It also says that Docker, Swarm and Podman environments in Portainer 3 "keep receiving bug fixes and security fixes" and "won't receive new features". |
| The work route contract lists only `build` and `runner` | [work-route.md](../../contracts/work-route.md), Sections 1 and 6 | Confirmed. |
| Docker writes the path of the Compose file of a project in a label of its containers | Not checked on a machine in this session | Not verified. Task H15 starts with this check. |
| Coolify and Dokploy run commands as root over SSH; counts of their security advisories | Not checked again | Not used for a requirement. |
| Prices, version numbers and counts of catalogue entries of the competitors | Not checked again | Not used for a requirement. |

The five largest gaps that the research names, and where this proposal puts each:

| Gap | Where |
| --- | --- |
| A stack that already runs is managed in place | Stage 2: tasks H15 and H16, decision OD-HOST-7 |
| Rollback, and an update that is judged by a health check | Stage 2: task H19 |
| A reverse proxy with domains and certificates | Stage 3: tasks H21 and H22, decision OD-HOST-9. HOST-009 says no today. |
| A Compose file from a git repository, then a build from source | Stage 3: tasks H23 and H26, decision OD-HOST-10 |
| Databases as a kind, with backups and restore | Stage 3: tasks H24 and H25, decision OD-HOST-12 |

Two things that look like advantages of Anima and are not, by the research: an MCP server (Coolify
and Dokploy each have one), and a hosted hub (both sell a hosted control panel). This proposal does
not build on either.

## 3. The main fact: container work is not a job

The two work types of K10 run a job: code that is not trusted, inside an isolation that task W2
designs. The isolation is the limit of what a person who controls the hub can do to a machine
(K10 Section 3; SF-003).

Container work is different in three ways. Each one changes the contract.

1. **No isolation.** To start a container, the daemon uses the Docker of the machine itself. A
   program that can use Docker can become the administrator of the machine: it can start a
   container that mounts the disk of the machine. Thus the right "the daemon may use Docker" is
   the right "the daemon may do all on this machine".
2. **The hub must name things.** K10 says that the hub names no image, no command, no path and no
   environment variable. A Compose file is a list of images, commands, paths and environment
   variables. To install an application, the machine must get that file from the hub.
3. **Thus the file is the risk.** A person who controls the hub can send a Compose file that
   starts a privileged container. On a hosted hub that person can be someone who is not the owner
   of the machine (SF-003 says the same of a `build`). For a `build` the isolation is the limit.
   For a Compose file there is no isolation, so the limit must be a rule that the machine applies
   to the file, and a switch that only the owner of the machine can set.

The competitors do not have this problem, because they do not try to solve it: Coolify and Dokploy
keep an SSH key of each server and run commands with it (the research, Section 3.7). The slower
path of Anima is the difference that the research names as real. It is only real if the contract
text keeps it. Section 4 is written for that.

## 4. Proposed contract text: the container work types

This is the text for a version 0.4 of [work-route.md](../../contracts/work-route.md). It is a
proposal. No security review has read it. The review of version 0.1 found 12 faults (SF-003 to
SF-014); a text that gives a machine to Docker deserves the same review before it is accepted.
Decision OD-HOST-5 asks the operator how to join the two.

### 4.1 Changes to the sections that exist

**Section 1, Terms.** The row "Work type" becomes: "What an item is. The list is fixed: `build`,
`runner` (JOB-011), and the container types of Section 6.4." The row "Kind" gains a third value:
"`host`: the daemon does the steps itself with the Docker of the machine. No job code runs in the
daemon. Only the container types have this kind."

**Section 3, the row "The steps of an item".** It becomes: "The daemon program, or the workflow
file, or, for `application_apply` only, the Compose file of the application. For `build` and
`runner` the hub names no program, no command, no argument, no path, no address, no image and no
environment variable. For the container types Section 6.4 says what the hub names."

**Section 3, two new rows.**

| Subject | Who decides | Rule |
| --- | --- | --- |
| If the machine takes container work | The owner of the machine | A second switch at the machine, "Manage applications". It is off after the install of the daemon. The hub cannot set it on. The rules of the switch of USE-004 apply to it (SF-007). A machine whose switch is off does not put a container type in `types` of its claim, and refuses each such item (`applications_off`). |
| What a Compose file may ask for | The owner of the machine | The profile of the machine: `contained` or `full` (Section 6.4.3). It is `contained` after the switch goes on. The owner sets `full` at the machine; the hub cannot. |

**Section 3, the paragraph "What the fixed list does and does not give".** One paragraph is added:
"For the container types there is no isolation. A person who controls the hub can stop each
container of a machine whose switch 'Manage applications' is on, and can start the containers of a
Compose file of their choice on it. On a machine with the profile `contained`, the rules of
Section 6.4.3 are the limit: such a file cannot ask for the rights of the machine. On a machine
with the profile `full` there is no limit, and that person can become the administrator of the
machine. The switch and the profile are the two controls that the owner of a machine has."

**Section 5, three new causes of refusal.**

| Cause | When |
| --- | --- |
| `applications_off` | The item is of a container type and the switch "Manage applications" is off |
| `file_not_permitted` | The Compose file breaks a rule of the profile of this machine (Section 6.4.3). The report names the rule by a fixed code, never text of the file |
| `not_adopted` | The item names a Compose project that the owner of the machine did not adopt (Section 6.4.5) |

**Section 8, two new purposes.**

| Purpose | Work type | What it is | Life |
| --- | --- | --- | --- |
| `compose_file` | `application_apply` | The Compose file of the application, at most 256 KiB. It is not a secret, but it reaches the machine only this way, for one item, to its holder | The item |
| `environment_file` | `application_apply` | The environment file of the application, at most 64 KiB. It is a secret of the operator (HOST-022) | The item |

The rule "the operator named the device for the project of the item" (SF-006) applies with one
change: an application has one machine (HOST-011), and the hub gives the two files only to the
holder that is that machine.

**Section 9, Clean-up.** One sentence is added: "An item of the kind `host` has no isolation to
delete. The machine deletes the copy of the environment file that it made for the step, if it made
one outside the folder of the application. A container that the item started is not deleted: it is
the result of the item."

**Section 13, a new open point, W1-P6.** "How the daemon gets the right to use Docker, and only
when the switch is on. Proposal: the daemon does not use Docker until the owner sets 'Manage
applications' on at the machine; that step needs the administrator of the machine and gives the
right; setting the switch off takes the right back. Task W2 must show one way on Linux and say
what is possible on a Mac and on Windows."

### 4.2 Section 6.4, Container work types (new)

The container types change the Docker of a machine. Each has the kind `host`. Each names a
container or a project by a value that the daemon checks against the Docker of the machine; no
type takes a command line, a path, or text that the daemon gives to a shell. The daemon starts
`docker` with a list of arguments, never through a shell.

No container type runs a command inside a container, opens a terminal, reads a file of a volume,
or changes a file outside the folders that Section 6.4.4 names. No container type exists for a
machine whose Docker is not the Docker of that machine: a remote Docker address is refused.

#### 6.4.1 `container_action`

Starts, stops or restarts one container that exists.

```json
{ "container": "4f5c1d2e3a4b…64 hexadecimal characters", "action": "restart" }
```

- `container` is 64 hexadecimal characters: the full identifier. The daemon refuses a shorter
  value and a name. It refuses an identifier that the Docker of the machine does not have.
- `action` is `start`, `stop` or `restart`. There is no `kill`, no `pause` and no `remove`.
- It names no secret.

#### 6.4.2 `application_apply`

Installs or updates one application that Anima installed: it writes the Compose file of the
application and brings the project to that file.

```json
{ "application": "0192f6b0-…", "fileSha256": "…64 hexadecimal characters", "environmentSha256": "…or null", "pull": true, "keep": 2 }
```

- `application` is a UUID that the hub makes. The project name of Docker is `anima-` and the first
  12 characters of that UUID; the hub does not choose the name.
- The machine gets the Compose file with the purpose `compose_file`, and the environment file, if
  `environmentSha256` is not null, with the purpose `environment_file`. It refuses a file whose
  SHA-256 is different (`program_mismatch`).
- The machine checks the file against its profile before it writes anything (Section 6.4.3).
- `pull` true makes the machine get the image of each service again before the start. That is an
  update (HOST-002).
- `keep` is how many earlier images of each service the machine keeps for a way back
  (Section 6.4.6), from 0 to 5.
- The result has, besides the fixed codes of Section 7, `health`: `healthy`, `unhealthy`,
  `no_check` or `not_waited`. The machine waits for the health checks of the project for at most
  `limits.seconds`.

#### 6.4.3 What a Compose file may ask for

The daemon reads the file with the same reader that the Compose program uses to give the final
form of a project. It applies the rules to that final form, so a rule cannot be passed with an
include, an extension, an anchor or a variable.

With the profile `contained` the daemon refuses a file in which a service:

| Code | The service asks for |
| --- | --- |
| `privileged` | `privileged`, or an added capability, or a device of the machine |
| `host_namespace` | The network, the process list, the user list or the IPC of the machine |
| `docker_socket` | A mount of the socket or of a folder of Docker |
| `bind_outside` | A bind mount whose path on the machine is not below the folder of the application (Section 6.4.4) |
| `security_off` | A security option that turns a protection off (`seccomp`, `apparmor`, `no-new-privileges:false`) |
| `build` | A `build` section: Anima installs published images in stage 2 (HOST-009) |
| `file_outside` | An environment file, a secret file or a configuration file outside the folder of the application |

A named volume, a published port and a network of the project are permitted.

With the profile `full` the daemon applies no rule of this table. The log of the machine
(Section 9) records the profile with which each file was applied.

The hub applies the same rules before it makes an item, so that the operator sees a refusal
before the machine does. The check of the machine is the control; the check of the hub is a
courtesy (as BLD-049 for the redaction).

#### 6.4.4 Where the files are

The daemon keeps the files of the applications that Anima installed below one folder, with one
folder for each application, named by the UUID. The path is fixed in the daemon program for each
operating system; the hub does not name it. The folder holds the Compose file, the environment
file, and the bind mounts of the application. Only the daemon and the administrator of the machine
can read the environment file.

An ordinary `docker compose` command in that folder works: the files are plain files. Anima keeps
no second copy on the machine and no private form.

#### 6.4.5 `adopted_apply`

Brings to its file a Compose project that Anima did not install and that the owner of the machine
adopted.

```json
{ "project": "tracks", "action": "up", "pull": false }
```

- The owner of the machine adopts a project at the machine: the daemon holds a list of adopted
  project names, with the rules of the switch (SF-007). The hub cannot write the list.
- The daemon finds the file of the project from the labels that Docker keeps on its containers.
  The hub names no path. The daemon refuses a project that is not in its list (`not_adopted`) and
  a project whose file it cannot find or cannot read.
- `action` is `up`, `stop` or `restart`. There is no `down`: Anima does not remove a project that
  it did not install.
- The daemon does not change the file. A change of the file of an adopted project is a second
  step, with its own type, and is not in this version (decision OD-HOST-7).
- The profile of the machine does not apply: the file is the file of the owner, on the machine of
  the owner.

#### 6.4.6 `application_remove`

Stops and removes the containers and the networks of one application that Anima installed.

```json
{ "application": "0192f6b0-…", "volumes": false, "images": false }
```

- `volumes` true also removes the named volumes of the project, and the folder of the
  application. The hub makes such an item only for an act of the operator with a login or a
  browser session (the actor kind `User`, LOG-012): never for an API key and never for an MCP tool
  (HOST-014).
- `images` true also removes the images that only this application used.

#### 6.4.7 `application_back`

Brings one application back to the images that it ran before its last `application_apply`.

```json
{ "application": "0192f6b0-…" }
```

- The machine kept the identifiers of the earlier images (`keep`). It starts the project with
  those images and the Compose file and environment file of that time, which it kept in the folder
  of the application.
- It does not bring back data. The result says so with the fixed code `data_not_restored` when the
  project has a volume.

#### 6.4.8 `docker_prune`

Removes things of Docker that nothing uses.

```json
{ "what": ["containers", "images", "networks"] }
```

- `what` is a list from `containers` (stopped), `images` (not used), `networks` (not used) and
  `volumes` (not used).
- `volumes` has the rule of `application_remove`: only for an act of the operator with a login or
  a browser session.
- The result has, for each member, the count of what was removed and the bytes that Docker says
  it freed.

#### 6.4.9 `container_follow`

Sends the new lines of the log of one container for a limited time (HOST-016).

```json
{ "container": "4f5c1d2e3a4b…64 hexadecimal characters", "tail": 200 }
```

- The lines are the log of the item (Section 7), with its rules: the redaction, the removal of
  control characters, the limit of a line and the limit of 2 MiB.
- `limits.seconds` is at most 900. The client asks again to follow longer.
- The log of a container can hold a secret. The hub gives the log of such an item only to a login
  or a browser session, as `containerLog` (D-23), and no MCP tool returns it (MCP-004).
- The hub keeps the lines of such an item only while the item runs, and deletes them when it ends.
  It is the one item whose log is not kept (LOG-007).

### 4.3 What is not a container type

- **The processor and memory use of each container (HOST-018).** The daemon measures it and pushes
  it with its readings (contract K8). It is a change of K8, not a work type.
- **The terminal in a container (HOST-024).** It is a session of the hub, after its own security
  review (task H12).
- **A scheduled command in a container.** It is "run this text", which the work route forbids
  (USE-006, JOB-015). It is not proposed.

### 4.4 How the text fits the findings and the rulings

| Finding or ruling | What it says | For the container types |
| --- | --- | --- |
| SF-003, R-185: the hub selects the code; the machine has a list of repositories | The control is a list at the machine | The same form of control: a switch and a profile at the machine (Section 3 rows). No list of repositories applies: a container type names no repository. A new finding is expected here, of the same kind as SF-003; the operator must accept its risk as he did with R-185. |
| SF-004, R-172: a job can take the daemon token through the port of the link | A machine behind the link takes no item until the socket of task W0 exists | No job runs, so no job can take the token. But a container that the item starts is a program of a local user. The rule of K10 Section 2 stays: no container type on a machine behind the link before task W0. Task W0 is built and merged (2026-11-19); its check on a machine is open. |
| SF-005: a machine keeps a job after its lease ended | A `build` ends when the lease ends | A container type is short and has no job to end. `application_apply` in progress is finished or left as it is; the result says which. `container_follow` ends with its lease. |
| SF-006, R-186: each device can get the secrets of a project; the device list now, the key later | The operator names the devices | An application has one machine; the two files go only to it. The device key of task W7 applies when it exists. |
| SF-007: the switch depended on the hub | The switch is where no job and no other user can write it | The switch "Manage applications", the profile and the list of adopted projects have the same rule. |
| SF-008: a registration stays after its item | The hub makes each secret useless | No registration is made. The environment file stays on the machine while the application is installed: it is the configuration of the application, not a secret of one job. `application_remove` with `volumes` deletes it. |
| SF-009: a project secret did not depend on the code | `origin` | Not applicable: no code of a repository runs. |
| SF-010: the log of a job carries text | Rules for each line | `container_follow` has the rules, and one more: the hub does not keep the lines. |
| SF-011: a device can hold the queue | Limits | The limits apply. `container_follow` has a short limit of its own. |
| SF-012: the log of the machine could not check the hub | The machine writes each item before it starts | The entry of a container type also has: the container or the project, the action, the SHA-256 of the two files, the profile, and the code of a refusal. |
| SF-013, R-187: a job can take its machine out of work | Accepted | A container type makes no machine `unclean`: it has no isolation that can fail to be deleted. |
| SF-014: the hub can name an old runner program | The daemon holds the lowest version | Not applicable to the daemon. A Compose file can name an old image; that is the choice of the operator, and Anima shows a newer image (HOST-021). |

### 4.5 What the acceptance would say

If the operator selects option B of OD-HOST-5, the sentence of K10 Section 14 gains a second part:

> I accept the work route contract K10, version 0.3, as written in `contracts/work-route.md`, and for each open finding of its security review I accept the state that `docs/security-findings.md` gives. I approve Section 4 of `docs/proposals/hosted-applications-stage-2.md` as the text of the container work types for a security review. It becomes version 0.4 of the contract when that review has no finding that I have not decided.

## 5. Stage 2, checked against the research

### 5.1 New requirements for stage 2 (proposed, not approved)

- **HOST-026.** Each change of a container, an application or the Docker of a machine is an item of
  the work route with a container type of contract K10. No other path changes a machine for hosted
  applications. Stage 1 stays as it is: it reads through SSH and changes nothing.
- **HOST-027.** A machine takes container work only when its owner set "Manage applications" on at
  the machine. It is off after the install. The hub cannot set it.
- **HOST-028.** A machine applies to each Compose file that the hub gives the rules of its
  profile. The profile `contained` refuses a file that asks for the rights of the machine. The
  owner sets the profile `full` at the machine.
- **HOST-029.** The files of an application that Anima installed are plain files in one folder of
  the machine, and an ordinary `docker compose` command works with them. Anima keeps no private
  form.
- **HOST-030.** Anima shows, for each Compose project that it did not install, where its file is
  and how it was started, as Docker says. This is read-only and gives no right to change the
  project (task H-C1 of the plan).
- **HOST-031.** The operator can manage a Compose project that Anima did not install, in place,
  after the owner of the machine adopted it at the machine. Anima does not move its file and does
  not remove it.
- **HOST-032.** An update keeps the earlier images of the application. The operator can bring the
  application back to them. Anima says that this does not bring back data.
- **HOST-033.** After an install or an update, Anima waits for the health checks of the
  application and shows the result. It does not go back by itself (ETH-004).
- **HOST-034.** The operator can see the containers of all machines in one list. A machine that
  did not answer is in the list as a machine that did not answer.
- **HOST-035.** The editor of HOST-022 shows the limits of memory and processor of each service as
  fields.
- **HOST-036.** The result of each item of container work is on the page of the application, with
  its time and its actor, and a failed install or update makes a notification (NOT-001).

### 5.2 The tasks of stage 2 that exist

| Task | The research says | Change |
| --- | --- | --- |
| H5: start, stop, restart | Portainer also has kill, pause, and "re-create with a fresh pull" | No kill and no pause: they are not asked for, and each is one more thing a machine must permit. "Re-create with a fresh pull" is the update of H6. H5 now needs task H14 and task W3, not only W1. |
| H6: install from a Compose file, update, remove | Each competitor has it. Rollback is expected (Coolify, Dokploy, CapRover) | H6 keeps the earlier images (`keep`), so that task H19 can go back. The files are plain files in one folder (HOST-029), which is what users of Dockge chose it for. |
| H7: the catalogue | Coolify "360+", Dokploy "+600" entries; Portainer reads a list by address | No change to the task. The size of the catalogue is decision OD-HOST-11. |
| H10: the log that follows; processor and memory | Each competitor has both | The log is the work type `container_follow`; the use is a change of K8. They are two changes of two contracts and can be two tasks. |
| H11: the editor, the syntax check, `docker run`; the clean-up | Each competitor has an editor. Limits of memory and processor are fields in all three | Add the limits as fields (HOST-035). The syntax check and the `docker run` function need no machine: they can be built before the work route (Section 8). The clean-up is the work type `docker_prune`. |
| H12: the terminal in a container | Each competitor has it | No change. It stays behind its own security review. |
| H-C1: the correct host and the provenance of Compose | Portainer is known for stacks that it calls "limited" | H-C1 is preparation only. Tasks H15 and H16 are the build that it prepares. |

### 5.3 New tasks for stage 2 (proposed)

A task is built from its row only after a ruling names it (PLN-007, PLN-013). The order is hub
API, `anima` command, MCP tool, client (CMD-030).

| Task | Content | Needs |
| --- | --- | --- |
| H14 | The container work types in contract K10: Section 4 of this document becomes version 0.4, after a security review of that text with its findings in [security-findings.md](../security-findings.md). | OD-HOST-5, OD-HOST-6 |
| H15 | Read-only: for each Compose project of a machine, the path of its file and its working folder as Docker says, and whether Anima installed it (HOST-030). Through the SSH link, as stage 1. First step: check on a machine with Docker that the labels hold what this task needs. | Approval of HOST-030. No work route. |
| H16 | Adopt a project at the machine, and `adopted_apply` (HOST-031). | H14, H5, H15, OD-HOST-7 |
| H17 | A check for each installed application, made at the install, and a web interface entry for an application that publishes a port (HOST-008). | H6; approval of HOST-008 (OD-HOST-8) |
| H18 | One list of the containers of all machines (HOST-034). Read-only, through the reads of stage 1. | Approval of HOST-034. No work route. |
| H19 | The way back to the earlier images (`application_back`, HOST-032), and the wait for health after an install or an update (HOST-033). | H6 |
| H20 | The result of each item on the page of the application, and a notification for a failed install or update (HOST-036). | H6; task W6 |

Not proposed for stage 2, with the reason:

- **A private registry.** The machine needs a credential that the hub keeps sealed. It is task H27
  of stage 3, because it adds a secret of a new kind.
- **Tools for AI agents that change things.** CMD-030 already says that each function gets its MCP
  tool unless it takes a secret, makes a trust decision or opens a session. Thus `container_action`
  and `application_apply` get tools; a removal of data and `container_follow` do not. No new
  decision is needed.
- **More than one person for a hub, Swarm, Kubernetes, Podman.** Standing decisions say no
  (design specification Section 1.4; HOST-010). The research gives no reason to change them.

## 6. Stage 3 (proposed): the part that Coolify and Dokploy have

Stage 3 starts after stage 2 is in use. Each requirement here is a direction, written so that the
operator can say yes or no to it. None has contract text yet.

- **HOST-037.** Anima can install a reverse proxy on a machine as an application that the operator
  can see, only when the operator asks for it on that machine. It refuses clearly when ports 80 or
  443 are in use. This replaces the words "supply a reverse proxy" of HOST-009.
- **HOST-038.** The operator can give an application a domain name. Anima asks for a certificate
  only after it has checked that the name points at the machine. Anima already warns before a
  certificate ends (CHK-007, NOT-006).
- **HOST-039.** The Compose file of an application can be a file in a repository of a project. The
  hub reads the file again at intervals; it applies a change only when the operator set that
  application to follow the repository.
- **HOST-040.** Anima can build an image from a Dockerfile of a repository with the work type
  `build`, on a machine that the operator selects, and install it. This replaces the words "build
  source code" of HOST-009.
- **HOST-041.** A database is a catalogue entry with more knowledge: Anima makes its password,
  shows its connection text one time, and knows how to copy it while it runs. PostgreSQL is first.
- **HOST-042.** Anima makes copies of a database, and of the volumes of an application, at
  intervals, and can bring one back. A copy function is built only with its way back and a test at
  intervals that the way back works. Where the copies go is a decision of that time (HOST-005,
  OD-HOST-1).
- **HOST-043.** The operator can give a machine the credential of a private registry. The hub keeps
  it sealed; only that machine uses it.

| Task | Content | Needs |
| --- | --- | --- |
| H21 | The reverse proxy as an application (HOST-037). First a small trial of two proxy programs on one machine, with a record. | Stage 2; OD-HOST-9 |
| H22 | Domain names and certificates (HOST-038). | H21 |
| H23 | The Compose file from a repository (HOST-039). | H6; the Projects plan; OD-HOST-10 |
| H24 | Databases as catalogue entries with more knowledge (HOST-041). | H7; OD-HOST-12 |
| H25 | Copies with a way back (HOST-042). | H24; OD-HOST-1 |
| H26 | A build from a Dockerfile, then an install (HOST-040). | Tasks W3 and W4 with `build` proven; OD-HOST-10 |
| H27 | The credential of a private registry (HOST-043). | H6 |

Not proposed, with the reason: preview deployments (they need H21, H23 and H26, and both
competitors warn about them for public repositories); build packs (small after H26, of no use
before it); a scheduled command in a container (Section 4.3).

## 7. Decisions

The operator answered each question on 2026-11-19 (R-209 to R-216), with the recommended option in
each case. The text below the table is the question as it was put to him, with its options.

| Decision | Question | Result |
| --- | --- | --- |
| OD-HOST-5 | Do you accept the work route contract, and how do the container work types join it? | B (R-209): contract K10 version 0.3 is accepted; Section 4 is the text for a security review and becomes version 0.4 when the review has no finding that he has not decided. |
| OD-HOST-6 | What may a Compose file from the hub do on a machine? | B (R-210): a switch at the machine that is off after the install; the profile `contained`; `full` only when the owner sets it at the machine. |
| OD-HOST-7 | May Anima manage a stack that it did not install? | B (R-211): yes, after the owner adopts it at the machine; start, stop, restart and update in place; Anima never changes its file in the first version. |
| OD-HOST-8 | Do you approve HOST-008: each installed application gets a check and a web interface entry? | Yes (R-212). |
| OD-HOST-9 | The reverse proxy: does Anima run one, manage one that exists, or stay out? | A (R-213), in stage 3: Anima installs its own, only where he asks, and refuses when ports 80 or 443 are in use. HOST-009 is replaced. |
| OD-HOST-10 | Deploys from git: a Compose file from a repository first, or a build from source first? | A (R-214): the Compose file first; a build from source later. |
| OD-HOST-11 | Does a catalogue ship with the product? | A (R-215): a small list that is tested, and the lists by address. |
| OD-HOST-12 | Are databases a kind of their own? | A (R-216): yes, in stage 3, PostgreSQL first. |

### OD-HOST-5: the work route contract and the container work types

Task W1 waits for your acceptance of contract K10, version 0.3. That acceptance alone starts no
stage 2 task: the contract has no container work type.

| Option | For | Against |
| --- | --- | --- |
| A. Accept version 0.3 only. Decide the container types later | The smallest decision. Builds and runners can start | Stage 2 of hosted applications stays blocked, and you are asked a second time |
| B. Accept version 0.3, and approve Section 4 as the text for a security review | One decision. The queue of the hub (task W3) and the review can start the same day. The types join the contract only after a review, as the first two did | The container types are not usable until the review ends and you have decided its findings |
| C. Accept version 0.3 with Section 4 as it is, with no review | Stage 2 can start at once | A text that gives Docker of a machine to the hub is accepted with no review. The review of the first text found 12 faults |

Recommendation: B.

### OD-HOST-6: what a Compose file from the hub may do on a machine

A program that can use Docker can become the administrator of the machine. On a hosted hub, the
person who controls the hub is not you.

| Option | For | Against |
| --- | --- | --- |
| A. No limit. A machine that takes work also takes each Compose file | The simplest. It is what Coolify, Dokploy and Portainer do | A person who controls the hub is the administrator of each machine. The main difference from the competitors is gone |
| B. A switch "Manage applications" at the machine, off after the install; with it on, the profile `contained`; the owner sets `full` at the machine | The owner of each machine decides two times. Most published applications run with `contained` | Some applications need more (a media server that reads a disk, a tool that reads the Docker socket). For them the owner must set `full`, and `full` has no limit |
| C. As B, with no `full` profile at all | The strongest limit | Anima cannot install some common applications |

Recommendation: B.

### OD-HOST-7: a stack that Anima did not install

A server that exists has stacks that were started by hand. The research names this as the top wish
of people who leave Portainer and Dockge.

| Option | For | Against |
| --- | --- | --- |
| A. Full: after adoption Anima also changes the file, with a copy of the old file kept | No second tool is needed | Anima writes a file that it did not make. A fault damages a set-up that works |
| B. In place, no change of the file: start, stop, restart and update (get the images again), after the owner adopts the project at the machine | Useful on the first day. Anima cannot damage the file | To change the file, the operator uses SSH or the files function |
| C. Never: only what Anima installed | The simplest | Anima shows a server that exists and can do nothing with it |

Recommendation: B first; A is a later task when B is in use.

### OD-HOST-8: a check and a web interface entry for each application

HOST-008 is not approved. The parts exist: the Docker container check, the rule for a restart loop
(CHK-014), notifications, and web interface entries (WEB-001). The missing part is to make them at
the install.

Recommendation: approve HOST-008. It is small, and a failed application is then seen with no extra
step.

### OD-HOST-9: the reverse proxy

HOST-009 says that Anima supplies no reverse proxy. It is the main reason that people choose
Coolify or Dokploy.

| Option | For | Against |
| --- | --- | --- |
| A. Anima installs its own proxy on a machine, as an application that you can see | One program to support. It is how Coolify, Dokploy and CapRover work | It can clash with a proxy that is there. Anima answers for a site that is down |
| B. Anima writes the settings of a proxy that you run | No clash | Many programs and versions to support |
| C. Stay out. Anima shows the open ports and checks the certificates | No new risk. Mostly built | The main reason stays unanswered |

Recommendation: A, in stage 3, after a small trial of two proxy programs. Do not choose the program
from this document.

### OD-HOST-10: deploys from git

| Option | For | Against |
| --- | --- | --- |
| A. A Compose file from a repository first | Medium size. It works with the images that your runners already build | No build inside Anima |
| B. A build from a Dockerfile at the same time | The experience of Coolify | It needs a place for the image and more of the work route |
| C. Neither | The least work | The repository is not the source of the truth |

Recommendation: A. Add B when the work type `build` is proven for the releases of Anima.

### OD-HOST-11: the catalogue

Today the plan has two entries, and reads a list in the form of Arcane from an address that you add
(R-150).

| Option | For | Against |
| --- | --- | --- |
| A. A small list that Anima tests, 10 to 30 entries | Each entry works, and can say where its data is and how to copy it | It looks small beside "+600" |
| B. A large list from a community source | It looks complete | Nobody tests 600 recipes. A recipe that is broken or not safe becomes a fault of Anima. With the profile `contained`, many will be refused |
| C. None; only the lists that you add | No upkeep | An empty screen at the first use |

Recommendation: A, with the lists by address that exist.

### OD-HOST-12: databases

| Option | For | Against |
| --- | --- | --- |
| A. Yes: "New PostgreSQL" with a password, the connection text, and a copy that is correct for the engine | It matches Coolify and Dokploy. A copy can be correct because Anima knows the engine | A recipe and a copy method to keep for each engine |
| B. No: a database is an ordinary catalogue entry | Nothing new | A copy of the volume of a database that runs is not safe |

Recommendation: A, in stage 3, for PostgreSQL only at first.

## 8. What an agent can build on the day of the first answer

With option B of OD-HOST-5, and no other answer:

| Work | Why it can start |
| --- | --- |
| The security review of Section 4, with its findings in [security-findings.md](../security-findings.md) | The answer asks for it |
| Task W3, the queue of the hub (the owner is [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md)) | It waits only for the acceptance of the contract |
| The rules of a Compose file (Section 4.2, 6.4.3) as a pure function with tests, for the hub and the daemon | It needs no machine. It is the part that the review will read most closely, so its tests help the review |
| The syntax check and the `docker run` function of task H11 (HOST-022, approved by R-150) | They need no machine and no work route |

With OD-HOST-6 also answered: the hub side of tasks H5 and H6 against the queue of task W3 (the
items, the `anima` commands, the MCP tools), and the daemon side when task W4 has the daemon take
an item.

With no answer at all, if the operator approves HOST-030 and HOST-034: tasks H15 and H18. Both
only read, through the SSH link of stage 1.

Not possible on that day: a container that starts from Anima on a real machine. That needs the
review of Section 4, your decisions on its findings, task W3, and the daemon part of task W4.

## 9. The state of each requirement after the rulings

| Requirement | State | What it waits for |
| --- | --- | --- |
| HOST-026: each change goes through a container work type | Not approved | The review of Section 4 and version 0.4 of the contract (R-209, task H14) |
| HOST-027: the switch "Manage applications" at the machine | Approved (R-210) | Its code waits for version 0.4 and for task W4 |
| HOST-028: the profile of a machine for a Compose file | Approved (R-210) | The exact rules are text of Section 4, under review. The rules as a pure function can be built and can change with the review |
| HOST-029: the files of an application are plain files in one folder | Not approved | The review of Section 4 (its Section 6.4.4) |
| HOST-030: Anima shows where the file of a project is | Not approved as a function of its own | R-211 names task H15, the read that task H16 needs. The approval of HOST-030 as a thing that the operator sees is one question for him |
| HOST-031: manage an adopted project in place | Approved (R-211), with no change of its file | Version 0.4 of the contract, task H5 |
| HOST-032: the way back to the earlier images | Not approved | A question for the operator |
| HOST-033: the wait for health after an install or an update | Not approved | A question for the operator |
| HOST-034: the containers of all machines in one list | Not approved | A question for the operator. No answer of 2026-11-19 covers it |
| HOST-035: the limits of memory and processor as fields | Not approved | A question for the operator |
| HOST-036: the result of each item on the page of the application | Not approved | A question for the operator |
| HOST-037: the reverse proxy | Approved for stage 3 (R-213) | Stage 2 in use; the trial of two programs |
| HOST-038: domain names and certificates | Not approved | A question for the operator, with task H21 |
| HOST-039: the Compose file from a repository | Approved for stage 3 (R-214) | Task H6 |
| HOST-040: a build from a Dockerfile | Not approved (R-214: later) | The work type `build` proven on the releases of Anima |
| HOST-041: databases as a kind | Approved for stage 3 (R-216) | Task H7 |
| HOST-042: copies with a way back | Not approved | OD-HOST-1 of the plan |
| HOST-043: the credential of a private registry | Not approved | A question for the operator |

One question remains for the operator from this table: "Do you approve HOST-030 and HOST-032 to
HOST-036 for stage 2?" It is decision OD-HOST-13 of
[hosted-applications-plan.md](../plans/hosted-applications-plan.md).
