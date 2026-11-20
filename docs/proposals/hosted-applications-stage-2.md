---
kind: Proposal, with a task table
state: Direction decided
evidence: R-209 to R-216 (operator, 2026-11-19, relayed) decide OD-HOST-5 to OD-HOST-12 as recommended. Approved: HOST-027, HOST-028, HOST-031 (no change of the file), HOST-037, HOST-039 and HOST-041, and HOST-008 of the plan; HOST-009 of the plan is replaced (R-213). Section 4 is the text of the container work types (R-209). A security review read its first form (SF-015 to SF-033); R-226 to R-233 decide the eight findings that needed the operator, and R-228 changes R-210: no profile `full`. A second read found the second form corrected only in part (SF-034 to SF-048); R-234 to R-238 decide its five points, and R-246 decides the design for a Mac (the applications program runs as the owner of Docker). Section 4 of version 4 is the third form. A third read by a separate reviewer (2026-11-19) says of the third form: no finding remains that the owner has not decided; the open points are changes of the text (SF-049 to SF-069). Section 4 of version 5 has those changes. The owner of `contracts/work-route.md` reviews the text when it is put in that file. It is the contract only when `contracts/work-route.md` has it as version 0.4; no code of a container work type on a machine is built before that. Not approved: HOST-026, HOST-029, HOST-030, HOST-032 to HOST-036, HOST-038, HOST-040, HOST-042 and HOST-043; Section 9 says what each waits for. The approved requirements and the tasks that the rulings name are in the task table of hosted-applications-plan.md. Version 5 (2026-11-19).
---

# Proposal: hosted applications, stage 2 and the stage after it

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this document (PLN-005).

Version: 4. Date: 2026-11-19. Version 4 has Section 4 in its third form, after the second read (SF-034 to SF-048), R-234 to R-238 and R-246. Version 3 has Section 4 written again after its security review and the answers of the operator to it (R-226 to R-233). Version 1 was the proposal with eight open decisions. Version 2 records the answers of the operator (R-209 to R-216) and adds Section 9, the state of each requirement.
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

## 4. The container work types: the text for version 0.4 of contract K10

This is the text for version 0.4 of [work-route.md](../../contracts/work-route.md), in its third
form. A first review read the first form (SF-015 to SF-033 of
[security-findings.md](../security-findings.md)); the operator decided eight points (R-226 to
R-233). A second read of the second form found it corrected only in part (SF-034 to SF-048); he
decided five points (R-234 to R-238) and the design for a Mac (R-246). A third read by a separate
reviewer (2026-11-19) of the third form says: no finding remains that the owner has not decided,
and the open points are changes of the text (SF-049 to SF-069). This fourth form has those
changes. It is the contract only when `contracts/work-route.md` has it. The earlier
forms are in the history of this file (commits `1c274d68` and `aa364653`).

The owner of `contracts/work-route.md` is the session of
[projects-and-builds-plan.md](../plans/projects-and-builds-plan.md). The sections of K10 that this
text names are those of version 0.3. The new sections get their numbers in K10 when the text is
put there; here they are Sections 4.4 to 4.13 of this document.

### 4.1 The applications program

Container work needs the right to use Docker. A machine has one program of Anima that talks to
Docker: the applications program, `anima-applications`. The daemon, `anima-daemon`, talks to the
hub and gives each container item to the applications program. The applications program makes no
connection to a network and never talks to the hub.

**Two kinds of engine (R-226, R-246).**

| Kind of engine | How the program knows | The applications program runs as | What the Docker right is there |
| --- | --- | --- | --- |
| A system engine | The real socket of the engine, after each link is resolved, belongs to the administrator | The administrator of the machine. The daemon has no right to use Docker and is in no group that has it (R-226) | The right of the administrator of the machine |
| An engine of a user (Docker Desktop on a Mac, a rootless Docker) | The real socket of the engine, after each link is resolved, belongs to a user | That user, as a second unit beside the daemon. It never runs as the administrator there (R-246) | The right of that user over the engine and over what the engine can reach. Each program of that user holds it with no Anima |

- On a machine with an engine of a user, the daemon can reach Docker, because it runs as that
  user. The rules of this text there protect against the hub, an API key and an AI agent, whose
  requests reach Docker only through the applications program. They do not protect against a
  different program of that user, to which Anima gives nothing that it did not have (R-246). A
  fault of the daemon that the hub can reach is not limited there by the settings.
- The administrator records the kind of the engine when he sets "Manage applications" on, with the
  path of its socket and the paths of the `docker` program and of the Compose program. The socket
  is a Unix socket of the machine; an address of a different machine is refused.
- At each start the applications program resolves each link of the socket path and of the two
  program paths. It refuses to start when the owner of the real socket is not the user that it
  runs as, or when the kind is different from the recorded one. It also refuses to start when a
  link, a real file or a folder above one of them can be written by a user who is neither an
  administrator nor the user that it runs as; when it runs as the administrator, when one can be
  written by a user who is not an administrator (SF-035).
- It gives the socket path to each command as an argument. It starts the Compose program directly
  at its recorded path, not through `docker`, and refuses to start when the Compose program is
  older than the lowest version that it knows.
- On each kind of engine the administrator installs the program file and the unit of the
  applications program, and the user cannot write them. An update through the hub (K8 Section 8)
  replaces `anima-daemon` only: it does not replace the applications program and changes no
  setting.
- It starts `docker` with a list of arguments, never through a shell. It gives `docker` an
  environment that holds only what the program sets, and a configuration folder of its own that
  holds no login of a registry (SF-041, SF-042). Each path that `docker` needs is an argument.

**The socket between the two programs.**

- The applications program makes a Unix socket in a folder that only the user that it runs as can
  write. It accepts a connection only from one user: the user that the administrator named when he
  set "Manage applications" on. A computer can have a daemon for each of two accounts (R-206,
  R-208); only the named one is accepted. It reads the user of the peer from the operating system.
  On an engine of a user the accepted user is the owner of the socket of the engine; the
  applications program refuses to start when the two are different, so that Anima gives no account
  the engine of a different account.
- The daemon checks that the socket belongs to the user that the applications program runs as,
  before it sends a file.
- No request on the socket changes a setting of Section 4.2.
- The applications program does not trust the daemon. It checks each member of each request by the
  rules of this text.
- Each program that runs as the user of the daemon can send a request on the socket with no hub.
  It gets what the rules of the machine permit. The rules for actors (Section 4.13) are rules of
  the hub and do not limit such a request. The operator accepts this (R-236). When the key of the
  device exists (task W7), the applications program checks a signature of the hub or of the
  operator on each request.

### 4.2 The settings at the machine

Only the administrator of the machine can set them, on each kind of engine. They are in one file
that only the administrator can write; the applications program reads it. The hub cannot set one,
the daemon cannot, and a request on the socket cannot (SF-007, SF-018).

| Setting | After the install | Meaning |
| --- | --- | --- |
| Manage applications | Off | The machine takes container work (R-210). With it go: the user of the daemon that is accepted, the socket of the engine, and the paths of the Docker programs |
| All containers | Off | The types also reach containers that Anima did not install and that are not of an adopted project (R-227) |
| Open to the network | Off | A file may publish a port on each address of the machine, not only on the machine itself (R-229) |
| The hub may delete volumes | Off | A request of the hub can remove the volumes of an application (R-230) |
| Adopted projects | None | The Compose projects that Anima did not install and may manage (R-211), each with its recorded paths |
| Protected | None | Names of containers and of Compose projects that no item can change |
| Registries with a login | None | The registries for which a pull may use a login, which the owner puts in the configuration folder of the applications program |
| Label prefixes | None | The prefixes of the labels that a file from the hub may set (SF-043) |

- There is one profile for a file from the hub, `contained` (Section 4.5). There is no profile
  `full` (R-228). A file that needs more is started by the owner at the machine and adopted.
- "The hub may delete volumes", off, stops the removal of a volume. It does not protect the data
  of an application: an install or an update from the hub can run a command with the volume of its
  own application and empty it. The protection of data is a copy (HOST-042). The screen that shows
  the setting says both (R-238).
- The screen that sets "Manage applications" on says which kind of engine the machine has and,
  for an engine of a user, the sentence of Section 4.1.
- To set "Manage applications" off does not stop a container. The applications program then shows
  each application that Anima installed and asks if it stops them (SF-030).
- The settings are made for one hub. The daemon sends the origin of its hub and its device
  identifier with each request. The applications program records the first pair in a file of its
  own. When a request has a different pair, it refuses each container item (`applications_off`)
  until the administrator sets "Manage applications" on again. This protects an owner who moves a
  machine to a different hub and forgets the settings. It does not limit a program of the user of
  the daemon, which can send the old pair (R-236). The key of the device (task W7) is the
  correction (SF-039).

### 4.3 Changes to the sections of K10 that exist

**Section 1, Terms.** "Work type": "What an item is. The list is fixed: `build`, `runner`
(JOB-011), and the container types." "Kind" gains `host`: "the applications program of the machine
does the steps with the engine of the machine. No job code runs. Only the container types have
this kind."

**Section 2.** The limit of a request to `POST /work/v1/secret` stays 4 KiB. Its answer is at most
1 MiB, and stays `Cache-Control: no-store`.

**Section 3, the row of the switch.** Added: "This row is for `build` and `runner`. For a
container type the setting is 'Manage applications' and the member of the claim is `applications`."

**Section 3, the row "Which repositories the machine works for".** Added: "The list does not apply
to a container type, which names no repository. For a container type the controls of the machine
are the settings, the adoption and the rules of a file."

**Section 3, the row "The steps of an item".** "The daemon program, or the workflow file, or, for
`application_apply`, the Compose file of the application. For `build` and `runner` the hub names
no program, no command, no argument, no path, no address, no image and no environment variable.
For `application_apply` the hub names, through the Compose file, the image, the command, the
health check and the environment of each container."

**Section 3, a new row.** "If the machine takes container work | The administrator of the machine
| The settings at the machine. The hub cannot set one."

**Section 3, the paragraph "What the fixed list does and does not give".** Added: "For the
container types there is no isolation. On a machine whose setting 'Manage applications' is on, a
person who controls the hub can: start, stop and restart the containers that the types reach, and
read their logs; start the containers of a Compose file of their choice that passes the rules of a
file, with a command of their choice in each, and with the volumes of that application, so that
they can read and empty its data, and fill the disk with them; make the engine get an image from
each address that such a file names, also an address of the local network; bring an application
back to its earlier version; remove an application with its kept files; remove stopped containers
and images that Section 4.11 names; make an adopted project start its files as they are at that
moment, and get its images again. An adopted project has no rule of a file: it runs the file of
the owner (R-232, R-235). The containers of an application can reach each address that the machine
can reach, on the local network and on the network of Anima (R-229). Each setting of the machine
that is on adds what its row says. Each program of the user of the daemon can ask for the same
with no hub (R-236)."

**Section 4, Claim.**

- `Claim` gains the member `applications`: `on` or `off`. A claim with no such member is read as
  `off`.
- The hub gives a `build` or a `runner` item only when `switch` is `on`. It gives a container item
  only when `applications` is `on`. When the two are `off`, the answer is always 204.
- The event of the first claim (USE-008) stays with `switch`.
- The daemon asks each 10 seconds while `switch` is `on` or `applications` is `on`. It also asks
  when it has no free slot and `applications` is `on`; the hub gives no job to a claim whose
  `free.slots` is 0.
- A machine that is `unclean` still claims while `applications` is `on`.
- `types` names the container types when `applications` is `on`; `kinds` then has `host`.
- `free.slots` counts jobs only. For container items the hub counts its own leases: it gives one
  machine at most one item that changes Docker and at most two `container_follow` items at a time.
  One claim gets at most one item. The hub gives, in this order: `container_action` with `stop`;
  each other container item, the oldest first; a job. The state `finishing` of a container item is
  its wait for health; the hub counts an item that changes Docker only in the states `claimed`,
  `preparing` and `running`. While an item of an application or of a project is not final, the hub
  gives no other item of that application or project, apart from `container_action` with `stop`.
  The hub gives one machine at most 6 `container_action` items in a minute for one API key.
- `unclean` is of jobs: the hub gives a container item to a machine that is `unclean`.
- An item of a container type has no member `repository`, `project` is null, and `secrets` names
  only the two purposes of Section 8 below. The machine refuses one that breaks this
  (`member_unknown`).

**Section 5, new causes of refusal.**

| Cause | When |
| --- | --- |
| `applications_off` | The item is of a container type and "Manage applications" is off |
| `file_not_permitted` | The Compose file or the environment file breaks a rule of Section 4.5. The report names the rule by a fixed code and, for a rule of a service, the position of the service in the file as a number; never text of a file |
| `not_adopted` | The item names a container or a project that Section 4.4 does not let it reach |
| `not_at_machine` | The item needs a setting that is off at the machine |

The daemon reports a refusal that it finds before the state `preparing` with the state `refused`,
as for a job. A cause that the machine finds after `preparing`, when it has the two files
(`file_not_permitted`, `program_mismatch`, `limit_disk`), is reported with the state `failed` and
that cause; the diagram of Section 7 does not change. The hub does not put a container item back
in the queue: the item is final, with the state `failed` and the cause as its reason. An
application has one machine, and a container identifier has meaning on one machine.

**Section 6, the first lines.** "The list is fixed in the daemon program and, for the container
types, in the applications program. There is no type that takes a command line from the hub. For
`application_apply` the hub names the command of a container in the Compose file."

**Section 7, Log.** "For a container type other than `container_follow`, the log of the item holds
only lines of the applications program: the step, a fixed code, and the digest of each image. It
holds no output of the Docker program and no text of a file."

**Section 7, the end of a container item.**

- The hub makes no second attempt of a container item. The operator asks again.
- When the lease of a container item ends with no report, the hub does not give the item to a
  different machine and shows "Waiting for the machine". The hub ends the attempt when
  `limits.seconds` and 5 minutes have passed since the claim, as for a job (SF-011); the item is
  then final, `failed`, with the cause `orphan`. `limits.seconds` is at most 1,800 for
  `application_apply` and `adopted_apply`, 120 for `container_action`, and 900 for
  `container_follow`.
- When a claim of the machine does not name the item in `holds`, the hub ends the attempt and the
  item is final, `failed`, with the cause `orphan`. It does not go back to the queue.
- `409 NOT_HOLDER`, `401 TOKEN_REFUSED`, `403 WRONG_DEVICE`, a stop and `expired`: the applications
  program finishes the one command that runs, starts no later step, and writes what it did to the
  log of the machine. It does not undo a start. After a stop the machine reports `stopped` when
  that command has ended. `container_follow` ends at once.
- When the machine cannot reach the hub, a container item finishes the step that it started and
  reports when the hub answers again; `limits.orphanSeconds` has no meaning for the kind `host`.
- The switch of the operator in the hub (`WORK_OFF`): the hub gives the machine no job and no
  container item other than `container_action` with `stop`. To a claim whose `applications` is
  `on` the hub does not answer `403 WORK_OFF`: it answers 204, or that one item. `403 WORK_OFF` to
  a report of a container item is as `409 NOT_HOLDER`. The count of 5 refused or lost items counts
  jobs only.
- `result` of an item of the kind `host` has `cleanup` `none` and these members, each a fixed code
  or a number: `health`; the code of the rule and the position of the service of a refusal; the
  counts and the bytes of a removal; `data_not_restored`.
- `container_action` with `stop` is never refused for `policy` or for `unclean`.

**Section 8, two new purposes.**

| Purpose | Work type | What it is | Life |
| --- | --- | --- | --- |
| `compose_file` | `application_apply` | The Compose file of the application, at most 256 KiB as the hub holds it | The application |
| `environment_file` | `application_apply` | The environment file of the application, at most 64 KiB: the one source of the variables of the Compose file, and a secret of the operator (HOST-022) | The application |

"Each test of this section applies to the two purposes: the holder, the current attempt, the
state, the purpose that the item names, and one time for one attempt. The rule 'the operator
named the device for the project' is, for them, 'the device is the one machine of the
application'. The rules 'no file outside the isolation' and 'deletes the value when the step has
it' do not apply to them: the applications program keeps the two files in the folder of the
application (Section 4.6) for the life of the application, and `application_remove` deletes them.
The rule 'asks for a secret only after the isolation exists' does not apply to the kind `host`.
The rule that no MCP tool, command or query returns a value of this route applies to
`environment_file` only: the editor shows the Compose file. Until the key of the device exists
(task W7), a person with the daemon token of a machine can claim its items and get its environment
file, on each machine, also on one that the operator does not own alone; the operator accepts this
(R-231). The values of an
environment file can also be read by each user of the machine who can use the engine."

**Section 9.** "The machine deletes each secret at the end of an item, except the two purposes of
a container type (Section 8). An item of the kind `host` has no isolation to delete and never
makes a machine `unclean`. For a container type the entry of the log of the machine has: the type;
the container identifier and the action; the application or the project; `fileSha256`,
`environmentSha256` and the SHA-256 of the final form; `pull`, `keep`, `volumes`, `images`, `what`;
the code of a refusal; the digest of each image that was started; the counts of what was removed.
For `container_follow`: the container and the time, never a line. On a system engine the
applications program writes this log where the daemon and the hub cannot change it. On an engine
of a user the log is a file of that user: a program of that user, also the daemon, can change it."

**Section 13.** Point W1-P6 is decided (R-226, R-246): the applications program.

### 4.4 Which containers and projects a type reaches

- A container is reached only when the applications program finds, from its own records and the
  project label of the container, that it is of an application that Anima installed or of an
  adopted project. With "All containers" on, each container is reached. Each other container is
  refused (`not_adopted`).
- Each type refuses an item that names, or would stop, start, make again or remove, a container or
  a project of the list "Protected". The list holds names of containers and names of projects.
- A project whose name starts with `anima-` cannot be adopted.

### 4.5 What a Compose file from the hub may have

**How the file is read (SF-034, SF-036, SF-042).** The applications program reads the file with
its own code, not with the reader of Compose.

1. It refuses a file of more than 256 KiB, a YAML alias or anchor, a tag, the merge key, a second
   document, a key that is not a string, a key that is there two times, a depth of more than 32,
   more than 32 services, more than 16 networks and more than 64 volumes. A name of a service, a
   volume, a network, a secret or a config has 1 to 63 characters of `a-z`, `0-9`, `-` and `_`, and
   starts with a letter or a digit.
2. It applies the list of keys below to the file as it came. A key that is not in the list is
   refused at any level: thus `include`, `extends`, `env_file`, `profiles`, `build` and `name` are
   refused before a variable is filled.
3. It fills the variables itself, in a scalar value after the YAML is read, never in a key. A
   variable is `${NAME}` or `${NAME:-text}`. The result is one scalar of text: it is never read
   again as YAML, and it is not filled a second time. `$$` is one `$`. Each other `$` is refused.
   A name that the environment file does not have, and that has no `:-text`, is refused. The one
   source is the environment file of the item, which holds lines `NAME=value`: a name has the
   characters `A-Z`, `0-9` and `_`, the value is the text after the first `=` with no rule for
   quotes, and a name that is there two times is refused.
4. It applies the list of keys and the rules of values to the result.
5. It writes the final form itself, as JSON, which Compose reads as a Compose file: each string in
   quotes with each control character escaped, each number and each truth value from its own typed
   value, `mode` as a string of octal digits, each service with the image by the digest that it
   pulled, each `$` written `$$`, the top-level `name` equal to the project name, and each path of
   the machine as the real path. Compose reads only this file.
6. It starts the project from the final form only, with the project name and the path of the file
   as arguments. It never gives the environment file to Compose, and the file has a name that
   Compose does not read. Thus the check and the start read one input (SF-017).

No file of the machine other than the two files of the item is opened for this.

**The keys that are permitted.** A key, at any level, that is not here is refused
(`file_not_permitted`).

| Where | Keys, and the rule of each |
| --- | --- |
| The top level | `services`; `volumes`; `networks`; `configs`. There is no `secrets`: a secret of an application is a variable of the environment file |
| A top-level volume | No member |
| A top-level network | `driver` with the value `bridge`; `internal` |
| A top-level config | `content` only |
| A service | `image`; `command`; `entrypoint`; `environment`; `restart`; `depends_on`; `healthcheck`; `user`; `working_dir`; `hostname`; `stop_signal`; `stop_grace_period`; `read_only`; `tmpfs`; `init`; `cap_drop`; `expose`; `mem_limit`; `cpus`; `pids_limit`; `labels`; `logging`; `security_opt`; `ports`; `volumes`; `networks`; `configs` |
| `image` | A reference that starts with `a-z` or `0-9`: a name with the characters that a registry permits, and a tag or a digest. The program gives it to a command after `--`. The applications program starts only an image that it pulled for this application, by the digest of that pull. It never starts an image that was on the machine for a different reason (SF-041) |
| `healthcheck` | `test`, `interval`, `timeout`, `retries`, `start_period`, `start_interval`, `disable` |
| `depends_on` | Names of services of this file; or, for each, `condition`, `restart`, `required` |
| `environment` | Each entry has a name and a value; the form with a name only is refused |
| `user`, `hostname`, `stop_signal`, `restart`, `expose`, `command`, `entrypoint` | A text or a list of texts in the form that Compose gives each; `restart` is `no`, `always`, `unless-stopped` or `on-failure` with a count; `expose` is whole numbers |
| `tmpfs` | An absolute path in the container with no `:` and no `,`. The program sets each option |
| `labels` | A key whose prefix is in "Label prefixes" of the machine. After the install no prefix is named, so no label is permitted (SF-043). A prefix that starts with `com.docker.` cannot be named |
| `logging` | `driver` with the value `json-file` or `local`. The program sets the size and the count of the files |
| `security_opt` | Only `no-new-privileges:true` |
| `ports` | Only the long form: `target`, `published`, `host_ip`, `protocol`. `published` and `target` are one whole number each, not a range; `protocol` is `tcp` or `udp`. `published` is a port from 1024 to 65535. With "Open to the network" off, `host_ip` is `127.0.0.1`; a different address, or none, is refused (`not_at_machine`) |
| `volumes` of a service | Only the long form: `type`, `source`, `target`, `read_only`. `source` is always present. The type `volume` with a volume of this file; or the type `bind` whose source is exactly `binds/<name>` of the application (Section 4.6). No other type and no other member. A `target` is an absolute path with no `..`; it is not `/`; two mounts of one service have different targets |
| `networks` of a service | Names of networks of this file; or, for each, `aliases` |
| `configs` of a service | Only the long form: `source`, `target`, `mode`, with a source of this file and the rule of a `target` above |

**What the program adds, and the limits (SF-029, SF-044).** It adds to each service
`cap_drop: [NET_RAW]`. For `mem_limit`, `pids_limit`, `cpus` and `stop_grace_period` it uses the
lower of the value of the file and its own limit, and its own value when the file has none. A
value of one of them that is zero or below is refused. `healthcheck.interval` is at least 5
seconds. Before a pull
it refuses the item when the free disk is below its own limit (`limit_disk`).

**The check of the hub.** The hub applies the same list to the text of the file before it makes an
item, so that the operator sees a refusal early. It opens no file and no address, and shows no
text of an error of a reader (SF-023). The check of the machine is the control.

### 4.6 Where the files are

The applications program keeps the applications that Anima installed below one folder that only
the user that it runs as can reach. Each application has one folder, named by its UUID.

- `files/` holds the Compose file as it came, the final form, the environment file, and the same
  three of the earlier version when `keep` is 1.
  The final form holds values of the environment file, so only the user that the program runs as
  can read `files/`. No mount can name `files/`.
- `binds/` holds one folder for each bind mount. A bind mount names exactly `binds/<name>`, where
  `<name>` is 1 to 64 characters of `a-z`, `0-9`, `-` and `_`. The program makes that folder
  itself. Directly before the start it checks, without following a link, that it is a real folder
  below a real `binds/` (SF-016).
- On an engine in a virtual machine, the folder must be below a folder that the engine shares.

### 4.7 `container_action` and `application_apply`

`container_action` starts, stops or restarts one container.

```json
{ "container": "4f5c1d2e3a4b…64 hexadecimal characters", "action": "restart" }
```

- `container` is 64 lower-case hexadecimal characters: the full identifier. A shorter value, a name
  and an identifier that the engine does not have are refused.
- `action` is `start`, `stop` or `restart`. There is no `kill`, no `pause` and no `remove`.
- `start` is permitted only for a container of an application that Anima installed, or whose
  restart policy is not `no`.
- A `stop` does not wait for an `application_apply` that waits for health: the wait for health
  does not hold the place of the item that changes Docker.

`application_apply` installs or updates one application that Anima installed.

```json
{ "application": "0192f6b0-7c1e-7a3b-9d2f-4e5a6b7c8d9e", "fileSha256": "…64 hexadecimal characters", "environmentSha256": "…or null", "pull": true, "keep": 1 }
```

- `application` is a UUID in the form of 36 lower-case characters; each other value is refused.
  The project name is `anima-` and the 32 hexadecimal digits of that UUID. The item is refused when
  a project of that name exists with a different folder (SF-028).
- The machine gets the two files with the purposes of Section 8 and refuses a file whose SHA-256
  is different (`program_mismatch`). The rules of Section 4.5 are applied at each apply.
- `pull` true gets the image of each service again. `pull` false uses the digests of the last
  apply of this application, and is refused for a service that has none. A pull uses no login,
  unless the owner named the registry at the machine.
- The start removes the containers of the project that the new file no longer names.
- `keep` is 0 or 1: with 1, the machine keeps the Compose file as it came, the environment file and
  the image digests of the version before this apply.
- The result has `health`: `healthy`, `unhealthy`, `no_check` or `not_waited`.
- A recipe of the catalogue of Anima names each image by a digest (R-233). A file of the operator
  can name a tag.

### 4.8 `adopted_apply`

Brings to its files a Compose project that Anima did not install and that the owner adopted.

```json
{ "project": "tracks", "action": "up", "pull": false }
```

- The owner adopts a project at the machine. At the adoption the applications program records the
  real path of each Compose file of the project and of its folder, as the labels of its containers
  give them at that time, and shows them to the owner with these sentences: "Anima can start, stop
  and update this stack. Anima starts this file as it is at that moment, and does not check if it
  changed. An update gets the newest images that the stack names and starts them. Anima does not
  check what those images do, and this stack runs with the rights that its file gives it."
- `adopted_apply` uses the recorded paths, never the labels at the time of the item. It gives each
  recorded file, the recorded folder as the project folder and the project name as arguments, with
  the environment and the configuration folder of Section 4.1. Compose reads the `.env` file of
  the recorded folder. A variable that the owner had in his shell is not there. It runs as the
  user that the applications program runs as.
- At the adoption the applications program makes the form of the project in this way, and refuses
  the adoption when a service of it has a different image, port or mount than the container that
  runs. The adoption screen also says: "Anima starts this stack with no variable of your shell and
  with no registry login of yours."
- `up` is `up -d --no-build`, with `--pull always` when `pull` is true.
- `action` is `up`, `stop` or `restart`. There is no `down`. `pull` true is permitted only with
  `up`.
- The applications program never changes a file of an adopted project (R-211).
- Anima does not check if a file of the project changed after the adoption: `up` starts the files
  as they are at that moment. A user who is not the administrator and who can write a file of the
  project, a file that it names (its `.env`, an `env_file`, a file of `include` or `extends`) or
  its folder, can thus become the administrator of the machine through Anima on a system engine,
  and get the rights of the owner of the engine on an engine of a user. The operator accepts this
  (R-235).
- The rules of Section 4.5 do not apply to an adopted project. The hub can make it get and start
  new images, and for a project that holds the socket of the engine the publisher of such an image
  then controls the machine. The operator accepts this (R-232).
- The screen and the command show the sentences above again before each `adopted_apply` with `up`.

### 4.9 `application_remove`

```json
{ "application": "0192f6b0-7c1e-7a3b-9d2f-4e5a6b7c8d9e", "volumes": false, "images": false }
```

- It stops and removes the containers and the networks of one application that Anima installed,
  from the kept final form; if the folder is gone, by the project label.
- It deletes `files/` of the application in each case.
- `volumes` true also removes the named volumes of the project and `binds/`. The applications
  program refuses it unless "The hub may delete volumes" is on (`not_at_machine`). When the key of
  the device exists (task W7), the applications program also checks the signature of the operator
  on such an item (R-230).
- `images` true also removes the images that the applications program pulled for this application
  and that no container uses.

### 4.10 `application_back`

```json
{ "application": "0192f6b0-7c1e-7a3b-9d2f-4e5a6b7c8d9e" }
```

- It brings the application back to the kept version. The applications program makes the final
  form again from the kept Compose file and the kept environment file, by the six steps of
  Section 4.5 of today, with the kept image digests and no pull. It sets no tag on an image.
- It goes back one step. A second `application_back` is refused until the next
  `application_apply`.
- It does not bring back data; the result says so with `data_not_restored` when the project has a
  volume.

### 4.11 `docker_prune`

```json
{ "what": ["containers", "images", "networks"] }
```

- `containers`: the stopped containers that have the project label of an application that Anima
  installed. `networks`: the networks of such a project that no container uses. `images`: the
  images that the applications program pulled for an application and that no container uses.
- With "All containers" on, the three members mean what `docker container prune`,
  `docker network prune` and `docker image prune` remove.
- In each case it never removes an image that `keep` holds, and nothing of the list "Protected".
- There is no `volumes` member: a volume is removed only with its application.
- The result has, for each member, the count of what was removed and the bytes that the engine
  says it freed.

### 4.12 `container_follow`

```json
{ "container": "4f5c1d2e3a4b…64 hexadecimal characters", "tail": 200 }
```

- It sends the lines of the log of one container that Section 4.4 lets it reach. `tail` is 0 to
  1,000.
- The lines are the log of the item, with the rules of Section 7 of K10. The hub keeps them only
  while the item runs.
- No setting of the machine is for logs: a person who controls the hub, and a program of the user
  of the daemon, can read the logs of the containers that Section 4.4 permits.

### 4.13 Which actor may ask for each type

The hub applies this table (R-234). The machine cannot see the actor.

| Type | A login or a browser session | An API key or an MCP tool |
| --- | --- | --- |
| `container_action` | Yes | Yes |
| `application_back` | Yes | Yes |
| `application_apply` | Yes | Only the first install of an entry of the tested catalogue of Anima. The hub makes the file from the recipe. The key or the tool gives no Compose file, no environment file and no value of a variable, apart from a value that the recipe names as a choice with a fixed list. Not an update |
| `application_remove` | Yes; with `volumes` true only with the setting at the machine | No |
| `adopted_apply` | Yes | No. A key or a tool starts, stops and restarts one container with `container_action` |
| `docker_prune` | Yes | No |
| `container_follow` | Yes | No. No MCP tool returns a log (MCP-004) |

### 4.14 What is not a container type

- **The processor and memory use of each container (HOST-018).** A change of K8, not a work type.
- **The terminal in a container (HOST-024).** A session of the hub, after its own review (task H12).
- **A scheduled command in a container.** Not proposed (USE-006, JOB-015).
- **A change of the file of an adopted project.** Not in this version (R-211).

## 5. Stage 2, checked against the research

### 5.1 New requirements for stage 2 (proposed, not approved)

- **HOST-026.** Each change of a container, an application or the Docker of a machine is an item of
  the work route with a container type of contract K10. No other path changes a machine for hosted
  applications. Stage 1 stays as it is: it reads through SSH and changes nothing.
- **HOST-027.** A machine takes container work only when its owner set "Manage applications" on at
  the machine. It is off after the install. The hub cannot set it.
- **HOST-028.** A machine applies to each Compose file that the hub gives the rules of the profile
  `contained`: the file may have only what a fixed list permits. There is no profile `full` in the
  first version (R-228, which changes R-210); a file that needs more is started at the machine and
  adopted.
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
| The rules of a Compose file (Section 4.5) as a pure function with tests, for the hub and the applications program | It needs no machine. It is the part that the review will read most closely, so its tests help the review |
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
| HOST-028: the profile of a machine for a Compose file | Approved (R-210, as R-228 changed it: no profile `full`) | The rules are Section 4.5. The rules as a pure function are task H28 |
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
