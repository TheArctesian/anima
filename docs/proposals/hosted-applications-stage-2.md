---
kind: Proposal, with a task table
state: Direction decided
evidence: R-209 to R-216 (operator, 2026-11-19, relayed) decide OD-HOST-5 to OD-HOST-12 as recommended. Approved: HOST-027, HOST-028, HOST-031 (no change of the file), HOST-037, HOST-039 and HOST-041, and HOST-008 of the plan; HOST-009 of the plan is replaced (R-213). Section 4 is the text of the container work types (R-209). A security review read its first form (SF-015 to SF-033); R-226 to R-233 decide the eight findings that needed the operator, and R-228 changes R-210: no profile `full`. A second read found the second form corrected only in part (SF-034 to SF-048); R-234 to R-238 decide its five points, and R-246 decides the design for a Mac (the applications program runs as the owner of Docker). Section 4 of version 4 is the third form. A third read by a separate reviewer (2026-11-19) says of the third form: no finding remains that the owner has not decided; the open points are changes of the text (SF-049 to SF-069). Section 4 of version 5 has those changes. The owner of `contracts/work-route.md` reviews the text when it is put in that file. It is the contract only when `contracts/work-route.md` has it as version 0.4; no code of a container work type on a machine is built before that. Not approved: HOST-026, HOST-029, HOST-030, HOST-032 to HOST-036, HOST-038, HOST-040, HOST-042 and HOST-043; Section 9 says what each waits for. The approved requirements and the tasks that the rulings name are in the task table of hosted-applications-plan.md. Version 5 (2026-11-19). Version 6 (2026-11-19): Section 10 is the proposed exception of R-247 for an entry that holds the socket of the engine; it is not in the contract and waits for a security review and for OD-HOST-28. Version 7 (2026-11-20): R-302, R-300 and R-301 give stage 3 its direction (HOST-046 to HOST-048); Section 12 is the design of the routing on the hub, Section 13 the recipe format of the catalogue, Section 14 the design of the PostgreSQL entry. Version 8 (2026-11-20): Section 13 has the fields and the codes of a database (`kind`, `server`, `database`), and Section 14.6 says what task H24 built and where it differs from the design.
---

# Proposal: hosted applications, stage 2 and the stage after it

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this document (PLN-005).

Version: 4. Date: 2026-11-19. Version 4 has Section 4 in its third form, after the second read (SF-034 to SF-048), R-234 to R-238 and R-246. Version 3 has Section 4 written again after its security review and the answers of the operator to it (R-226 to R-233). Version 1 was the proposal with eight open decisions. Version 2 records the answers of the operator (R-209 to R-216) and adds Section 9, the state of each requirement.
From: the operator, 2026-11-19, relayed by the orchestrating session and not first-hand in the session that wrote this: the container side should be "fleshed out more via the competitor research", and "the coolify/dockploy and the portainer things are still competitors and things I want to emulate ish".
Status: the header of this document holds the state (PLN-020).
Map: [work-plan.md](../work-plan.md), category Use, row 4.9 of the [priority order](../plan/priorities.md). The plan that this proposal changes when it is approved is [hosted-applications-plan.md](../plans/hosted-applications-plan.md). The contract that it changes is K10, [work-route.md](../../contracts/work-route.md).
Identifiers: HOST-026 to HOST-050; open decisions OD-HOST-5 to OD-HOST-12; tasks H14 to H27.

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
changes. It is the contract only when `contracts/work-route.md` has it. That file is the current text from its version 0.4: its owner asked for eight more exact forms there (the two causes of `WORK_OFF`, the members of `result` for the kind `host`, the limits of a container item, the update of machines), which are in that file and not in this section. The earlier
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

The hub applies this table (R-234). The machine cannot see the actor. The hub makes no container item with no actor in this version: each item is the act of a person, an API key or an MCP tool (ETH-004). An update that the hub starts by itself (R-242 of [extensions-plan.md](../plans/extensions-plan.md), for two stacks of that plan) is not in this version; it needs its own row here, with a ruling.

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

- **HOST-037.** Anima can install a reverse proxy on the hub as an application that the operator
  can see, only when the operator asks for it. It refuses clearly when ports 80 or 443 are in use.
  This replaces the words "supply a reverse proxy" of HOST-009. R-302 changed "on a
  machine" to "on the hub": Anima installs no reverse proxy on a different machine.
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
- **HOST-044.** Proposed (R-249: later, as a general rule). The operator can turn on, for an
  application of the catalogue, updates to a digest that Anima tested. It needs the copies of
  HOST-042 first, a setting at the machine, and the actor "the hub by itself" in the contract.
- **HOST-045.** Proposed (R-250). On a hosted hub, the link carries the port of an application to
  the hub when the person turns that port on at the machine. The design is made with the owner of
  the open ports of the link (HMD-049 of [hosted-mode.md](hosted-mode.md)).
- **HOST-046.** Direction decided (R-302). The hub is the front door of the public
  traffic of the applications of its machines. A request for the domain of an application comes to
  the hub, and the hub sends it to the port of the application on its machine. A hosted hub does
  this behind the front door of the cloud service; a hub that a person hosts does it on its own
  ports 80 and 443. Section 12 is the design.
- **HOST-047.** Direction decided (R-300). The catalogue that ships with the product
  is a large list of popular self-hosted applications. Each entry has a test that a machine runs,
  each image is named by a digest, a schedule pins new digests and runs the test again, each entry
  shows the date of its last test, and an entry that fails leaves the list. An entry that needs
  more than the default rules has a mark. Section 13 is the format.
- **HOST-048.** Proposed (R-301, OD-HOST-38 of the plan). A database has one of
  three levels: one server with copies to a second machine (the default); a primary and a replica
  that the operator selects, with a switch to the copy only by his action; no sharding.
- **HOST-049.** Direction decided (R-319). The images of LinuxServer.io are the base of
  the catalogue: each image that fits the rules is an entry, generated from the data of the
  publisher, in a tier that says whether Anima tested it. Section 15 is the design.
- **HOST-050.** Direction decided (R-319). The hub is the proxy manager: one place for
  each route of a name or a port to an application on a machine, with certificates, for web
  routes and for streams of TCP and UDP. Section 16 is the design.

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
| HOST-037: the reverse proxy | Approved for stage 3 (R-213); on the hub only (R-302) | The design of Section 12 and its review; the trial of two programs |
| HOST-038: domain names and certificates | Not approved; the direction is decided (R-302 asks for the design) | Section 12 and its review, then a question for the operator |
| HOST-039: the Compose file from a repository | Approved for stage 3 (R-214) | Task H6 |
| HOST-040: a build from a Dockerfile | Not approved (R-214: later) | The work type `build` proven on the releases of Anima |
| HOST-041: databases as a kind | Approved for stage 3 (R-216) | Task H7 |
| HOST-042: copies with a way back | Not approved | OD-HOST-1 of the plan |
| HOST-043: the credential of a private registry | Not approved | A question for the operator |
| HOST-044: updates to a digest that Anima tested | Not approved; the direction is decided (R-249: later) | HOST-042; the actor in the contract, after a security review |
| HOST-045: the link carries the port of an application on a hosted hub | Not approved; the direction is decided (R-250) | The design with the owner of HMD-049 |
| HOST-046: the hub is the front door of the traffic of applications | Not approved; the direction is decided (R-302) | Section 12, its review, and the answers of the cloud service |
| HOST-047: a large catalogue that stays tested | Not approved; the direction is decided (R-300) | Section 13; the test on a machine needs task H6 |
| HOST-048: the levels of a database | Not approved | OD-HOST-38 of the plan |
| HOST-049: the images of LinuxServer.io as the base of the catalogue | Not approved; the direction is decided (R-319) | Section 15, its security read, OD-HOST-50 to OD-HOST-53 |
| HOST-050: the hub as the proxy manager, with streams | Not approved; the direction is decided (R-319) | Section 16, its security read, OD-HOST-46 to OD-HOST-49 |

One question remains for the operator from this table: "Do you approve HOST-030 and HOST-032 to
HOST-036 for stage 2?" It is decision OD-HOST-13 of
[hosted-applications-plan.md](../plans/hosted-applications-plan.md).

## 10. A proposed exception to the rules of a file: an entry that may hold the socket of the engine (R-247)

This section is a proposal. It is not in contract K10 and no reviewer has read it. R-247 (operator,
2026-11-19) says that Arcane is an entry of the tested catalogue with an exception to the profile
`contained`, so that it may mount the socket of Docker, and that the exact exception goes through a
security review before any code. Nothing that installs Arcane is merged until the operator has
decided the findings of that review.

What the exception gives, in plain words: a program that holds the socket of the engine is the
administrator of the machine on a system engine, and has the rights of the owner of the engine on
an engine of a user. The settings "All containers", "Protected" and "The hub may delete volumes"
limit Anima. They do not limit a person or a key inside such a program.

### 10.1 The proposed text, for Section 6.4.4 of K10

"**Excepted entries.** The applications program holds a list of excepted entries. The list is fixed
in the program: the hub cannot add an entry and cannot change one. Each entry of the list has a
name, the repository of one image and one digest of that image. A release of the applications
program changes the list; nothing else does.

An excepted entry is a service of a file whose `image` is exactly the repository and the digest of
an entry of the list. For such a service, and for no other service of the file, one more mount is
permitted: the type `bind`, the source `engine-socket`, and the target `/var/run/docker.sock`. The
applications program writes the real path of the socket of the engine in the place of the source.
Each other rule of this section applies to the service with no change: its ports, its other
mounts, its limits, its labels, and each key that is not in the list of keys.

The applications program refuses the mount (`not_at_machine`) unless the owner named the entry in
the setting 'Programs that control Docker' of the machine. The setting is empty after the install.
The screen that sets it says: 'This program controls Docker on this machine, and so can do all on
this machine. What a person does inside it is outside the rules of Anima.'

An excepted entry is never updated by `pull`: its image is the digest of the list. A new version
comes with a new list. `application_back` applies the rule again with the list of today.

The first entry of the list is the manager of Arcane. The agent of Arcane is a second entry."

And for Section 3 of K10, in the paragraph of what a person who controls the hub can do: "On a
machine whose setting 'Programs that control Docker' names an entry, that person can install that
entry, and thus a program that is the administrator of the machine."

### 10.2 What this text adds to the answer of the operator

Two things in the text are controls that his answer did not name. The reviewer and he must see
them as proposals, not as his words:

- The list is in the program, with a digest. Thus neither the hub nor the catalogue can name a
  different image as "Arcane".
- The setting "Programs that control Docker" at the machine. Without it, the consequence that he
  was told holds on each machine whose "Manage applications" is on. With it, it holds only on a
  machine where the owner named the entry.

### 10.3 The question that his answer left open (OD-HOST-28)

R-234 lets an API key or an AI agent make the first install of an entry of the tested catalogue.
Does that apply to an excepted entry?

| Option | For | Against |
| --- | --- | --- |
| A. Yes: an excepted entry is installed as each other entry | One rule for the catalogue | An agent, which can be led by text that it reads, can install a program that is the administrator of a machine |
| B. No: only a person with a login or a browser session installs an excepted entry | The administrator of a machine is never installed by an agent | One more row in the table of actors |

Recommendation: B. With the setting of Section 10.1 at the machine, an agent could install the
entry only on a machine where the owner already named it; B closes that also.

### 10.4 Before the review: the test by hand (task AR0)

R-247 asks for the test of Arcane by hand first. It needs a container that holds the socket of an
engine, so it gives the image of a different publisher the control of that engine. This session
did not run it on the build machine: it was told to install nothing and change no setting there.
It is a step for the operator, or for a session on a machine that he names for it.

## 11. The port of an application through the link: design options (task H35, R-250)

This section is a design with options. It adds no approved requirement. Section 14.6 says what
task H24 built of it. The facts about the link are from the session that built task HM-T12 of
[hosted-mode.md](hosted-mode.md) (HMD-048, HMD-049; tasks HM-T13 and HM-T14 are not started;
the unit of the link is HMD-058, task HM-T24). This session did not read that code again:
each fact of Section 11.1 is a claim of that session.

### 11.1 What the link does today

- **The list.** One file `open-ports` in the state folder of the link on the computer: a list
  of port numbers. A fixed rule in the program permits only 22, 5900, and 3389 on Windows; a
  different port in the file is ignored. The link reads the file at each request.
- **Who can write it.** The user that runs the link. Since R-240 that is the logged-in user,
  the same user as the daemon. So "the hub cannot change the list" holds against the network.
  It does not hold against a program of that user, the daemon included.
- **What it dials.** Only `127.0.0.1:<port>` of its own computer, and only for a port of the
  list. That is the address at which an installed application listens (R-229).
- **Who can ask at the hub.** Each process of the user of the hub can open the socket of the
  link, and one hub function connects for SSH, the checks and the sessions alike. The link
  does not know who asked. Today a TCP check to an open port passes for each caller that may
  make a check.
- **What protects the bytes.** The tunnel of the link, between the computer and the hub. In
  the hub they are plain. No certificate of the application is checked: there is none.

### 11.2 The proposed design

1. **A second kind of entry.** The list gains entries "port of an application". Each names the
   identifier of an application and the port that the applications program recorded for it at
   its install. The fixed rule becomes: 22, 5900, 3389 on Windows, or a port that the record
   of the applications program names at the time of the request.
2. **Checked at each request.** The link refuses a port that the record does not name now.
   An application that is removed takes its entry with it, so a different program that takes
   the port later is not reachable. "Manage applications" off removes each such entry.
3. **Where the entry is kept** is the question OD-HOST-29 below.
4. **The limit is in the hub, not in the link.** An open entry says only "the hub may reach
   this port". Who may use it is a rule of the hub function that opens it. The proposed rule:
   the web page of an application opens only for a person with a login or a browser session;
   no API key and no MCP tool gets bytes of it; a check of the health of an application
   (HOST-008) reads only "answers" or "does not answer" and returns no byte of the answer.
   A hub function that sends to an application by itself (events to an analytics stack) is a
   named function with its own requirement, not a general path.
5. **The page of a different publisher is not served under the origin of the hub.** Served
   under the origin of the hub, such a page can use the session of the person at the hub.
   With one gateway for all hosted hubs (R-239) it could reach each hub of that person. The
   proposed rule: each application gets an origin of its own (a name of its own below a
   domain that holds no session of a hub), and the hub sends none of its cookies to it. How
   that name is made is the question OD-HOST-30.
6. **The switch** is on the page of the application, with one store behind it and the screen
   of the open ports (task HM-T13), which shows such an entry with the name of the
   application and does not change it.

### 11.3 What this design does not protect

- A person who controls the hub reaches each port that is open at a machine. That is what
  "open" means, and it is the same as for port 22.
- Until the limit of point 4 is built and tested in the hub, an open application port is
  reachable by each hub caller that can make a TCP check. Thus the entry of the second kind
  must not be built before the hub rule.
- An application with the socket of the engine (Section 10) behind an open port: a person who
  reaches its web interface and can sign in to it is the administrator of that machine.

### 11.4 Questions for the operator

- **OD-HOST-29: who can turn on the port of an application at the machine?**
  A: only the administrator, in the file of the machine settings that the applications
  program reads (as "Manage applications"); the link asks that record and does not use its
  own file for such an entry. B: the user, in the file of the link, as for port 22.
  Recommendation: A. With B, the daemon, which runs as the same user and takes requests from
  the hub, could open the port of each application by itself; with A, a hub that controls
  the daemon cannot. Against A: on a Mac he is asked for the administrator one time more for
  each application whose page he wants in the hub.
- **OD-HOST-30: under which name does the hub serve the page of an application?**
  A: a name of its own for each application below a separate domain that holds no session of
  a hub (needs a domain or a sub-domain with a wildcard certificate; a change of DNS that
  only he can make). B: no page in the hub in the first version; the hub shows the state and
  the health of the application, and he opens its page in a session to the machine. C: under
  the origin of the hub with the cookies removed. Recommendation: B first, then A. C is not
  safe enough: a script of the page still runs in the origin of the hub.

A security review by a separate reviewer reads this section before it becomes a plan.

## 12. The routing of the domains of applications on the hub (R-302, task H21)

This section is a design with options. It adds no approved requirement. Section 14.6 says what
task H24 built of it. It builds on Section 11 (the port of an application through the link, task H35) and on the
path of task HM-T12 of [hosted-mode.md](hosted-mode.md). The front door at `hub.anima.ing` is a
part of the cloud service; its plan is in a private repository, and this section does not change
it. Each sentence about what the door does is a request to that plan, not a fact: Section 12.9 lists
the questions. The proxy is an application that the operator can see, and the hub installs it only
when he asks for it (HOST-037, R-302 (3)). A security review by a separate reviewer reads
this section before any code (R-302).

### 12.1 What the ruling decides, and what it changes

- The reverse proxy for the domains of applications runs on the hub, and only there. The hub is
  the front door of the public traffic of the applications of its machines.
- A hosted hub has the proxy behind the front door of the cloud service. A hub that a person
  hosts has it on its own ports 80 and 443.
- It replaces "a reverse proxy on a machine of the operator" of R-213 and HOST-037. Anima installs
  no proxy on a different machine. The trial of two proxy programs stays, on one hub.
- Does the condition of task H21, "stage 2 in use", still hold? Not for the design and its review:
  they start now. It holds for the trial and the build in a changed form: the proxy needs an
  application to send traffic to (task H6, or an adopted project of task H16) and the path to the
  port of an application (task H35). Thus no code of this section is built before those.

### 12.2 The path of one request on a hosted hub

1. A person's browser asks DNS for the name of an application, for example `photos.example.com`.
   The owner of that domain has set the name to point at the front door (a `CNAME` to a name
   that the cloud service gives, or the address of the door).
2. The browser opens a TLS connection to the door, port 443, with the name in its SNI field.
3. The door (requested of the cloud plan) finds the hub that owns that name in its table of
   names (Section 12.4), and sends the connection to that hub with no change: the door does not
   open the TLS (option R1 of Section 12.10). If no hub owns the name, the door closes the
   connection. The door gives the address of the client with the PROXY protocol.
4. The proxy of the hub opens the TLS with the certificate of the name, reads the request, and
   finds the application of that name: one device, one application, one recorded port.
5. The proxy gives the identifier of the application to the hub. The hub finds the device and the
   port in its own record and connects through its one connect function (task HM-T12). On a
   computer of the link the link dials `127.0.0.1:<port>` only for a port that the record of the
   applications program names at that moment (Section 11.2). The answer goes back the same way.
6. The `Host` header must be the name of the SNI field; if not, the proxy answers 421. On port 80
   the proxy answers only the ACME challenge and a redirect to `https://`; it sends no content of
   an application on port 80.

On a hub that a person hosts, steps 2 and 3 are gone: the name points at the hub, and the proxy
of the hub listens on ports 80 and 443 itself. Step 5 is the same, or the hub reaches the port on
the network of the machine when the machine has "Open to the network" on (R-229).

### 12.3 The rules of the proxy

- **Only the ports of applications.** The proxy can connect only to a port that a record of an
  application names. It cannot name a device and a port by itself. The function that it calls
  takes the identifier of an application, not a port. The port of an application is never 22,
  5900, 3389, or a port of the link or of the daemon: the hub refuses such a record, and the link
  refuses such an entry.
- **A process of its own, with no other exit.** The proxy holds the private keys of each name and
  reads each request from the internet. It runs as a program apart from the hub process, with a
  user of its own. That user cannot open the dial socket of the link, the control socket of the
  link, the listener of the hub, or the database, and has no network route but its one interface
  to the hub and the outbound ACME and DNS. A test shows that the user of the proxy cannot open
  those sockets. A fault in the proxy must not give the database, the sessions of the hub or port
  22 of a machine.
- **Who may make an application public.** Only a person with a login or a browser session, after a
  second factor (as HM-D14 asks before a port is turned on); no API key and no MCP tool. The
  consent of the machine for the port (OD-HOST-29) says "the hub may reach this port", not "the
  internet may reach it": OD-HOST-43 asks whether a second consent at the machine is needed. An
  application that holds the socket of the engine (Section 10) is not made public.
- **The target is fixed.** The device and the port of a route are fixed when the name is made
  public. On a hub that a person hosts, which reaches the port on the network of the machine, the
  address is fixed then too, and the hub refuses its own loopback addresses and ports, link-local
  addresses and the metadata address `169.254.169.254`. A change of the target needs the same
  rights as making it public.
- **Its own origin for each application (AC-D45).** An application is served only under its own
  name. It is never served under the name of a hub or a path of it (`hub.anima.ing/app/...`), and
  never under a name below a domain that holds a cookie of a hub. AC-D45, a finding of the review of
  the door, says that one origin serves all hubs; the same risk is larger here, because the page of
  an application is code of a different publisher. A name that Anima gives (for an application with
  no domain of the person) is below a domain of its own, separate from `anima.ing`, that is on the
  Public Suffix List (OD-HOST-30, option A).
- **No header of the hub, and no header of the client is trusted.** The proxy removes from each
  request each `Forwarded`, `X-Forwarded-*` and `X-Real-IP` header and each header that a hub or the
  door uses to name a person, then sets `X-Forwarded-For` and `X-Forwarded-Proto` again from what it
  knows. It reads the PROXY protocol only on a connection from the door, never on a hub that a
  person hosts. It sends no cookie of a hub.
- **The hub keeps no content.** TLS ends in the proxy, so content passes through it; it keeps
  none. Its log has only the name, the bytes, the status and the time: never the path, the query,
  a header or a cookie, because they carry tokens of the application (LOG-007). It counts bytes and
  requests for each name (Section 12.7).
- **A hub on the same site.** When a hub that a person hosts and an application share a registered
  domain (`hub.example.com` and `photos.example.com`), the cookie attribute SameSite does not stop a
  request from the page of the application. The hub warns, and keeps its cookie `__Host-` and an
  exact check of `Origin`.
- **API keys and agents.** No API key and no MCP tool reaches the page of an application through
  the hub (Section 11.2, point 4). The public route of this section is for the public: the
  application does its own sign-in.

### 12.4 Who owns a name

All hosted hubs share the address of the door. Thus "the name points at the address of the door"
does not prove which hub owns it: each hub could claim each name that points at the door, also a
name of a different person. The proof:

1. The person adds the name to an application in his hub. The cloud service makes the token:
   random, of at least 128 bits, bound to the pair of the hub and the name, and kept by the cloud
   service. The identifier of a hub is never used again.
2. The person writes a `TXT` record `_anima.<name>` with that token, and points the name at the
   door.
3. The hub asks the cloud service to add the name. The cloud service reads the `TXT` record
   itself, with its own resolver, and checks that the token is that of this hub. Only then does
   it add the name to the table of the door. A name that a different hub already owns is refused,
   until the `TXT` record names the new hub.
4. The cloud service reads the record again at intervals, with a resolver that checks DNSSEC
   where a zone has it. When the token is gone or names a different hub in several reads in a row,
   it removes the name from the table, and the hub shows that the name is not proven. One failed
   read removes nothing.
5. A name with a wildcard (`*.example.com`) is refused in the first version.
6. A name that Anima gives is proven by its allocation, not by a `TXT` record. It is never given
   again to a different hub, and it leaves the table of the door in the same step that deletes the
   hub or the account.
7. The door routes the ACME challenges (`TLS-ALPN-01` on 443, `HTTP-01` on 80) only by its table of
   names. When a name moves to a different hub, the old hub deletes the certificate and the key.

A hub that a person hosts makes the same check of HOST-038 against its own address, with no
token: it is the only hub at that address.

### 12.5 Certificates

- **Who asks.** The hub holds the ACME account and its key, not the proxy; the proxy runs the
  challenge and gets the certificate from the hub. The hub asks for the certificate of each name with ACME, from Let's
  Encrypt at first. Because the door does not open the TLS (option R1), the challenge
  `TLS-ALPN-01` reaches the hub with no help of the door. `HTTP-01` needs the door to send plain
  HTTP on port 80 for that name to the hub too; the design asks for that as well, because the
  same route sends a browser from `http://` to `https://`. `DNS-01` is not used: the hub would need
  a credential of the DNS of the person.
- **When.** Only after the proof of Section 12.4, and only after the name points at the door
  (HOST-038).
- **CAA.** Before the first request the hub reads the `CAA` records of the name and of each
  domain above it. If a record exists that does not permit the authority, the hub does not ask and
  says which record stops it.
- **Limits of the authority.** Let's Encrypt limits the certificates for each registered domain
  each week, the new orders for each account, and the new accounts for each address. All hosted
  hubs share the outbound address of the pool, so the limit for each address is shared. The design:
  one ACME account for each hub, made at the first name; a new order only for a name that has
  none, or a certificate with less than 30 days; failures wait longer each time. The exact numbers
  of the limits are not in this text: they change, and the build reads the current page of the
  authority. A wildcard certificate is not used: its key would serve more than one hub.
  OD-HOST-44 asks for one ACME account for the pool or one for each hub.
- **Where they are kept.** A hosted hub runs in a container that the pool can replace (R-130,
  R-139). The certificates and the ACME account are kept in the data of the hub that the pool
  backs up, not in the container. The private keys are sealed as the other secrets of the hub. That
  seal protects them only if the key of the seal is not in the same backup; where that key is kept
  on a hub of the pool is a question for the cloud plan.
- **The warning before the end.** The check of a certificate exists (CHK-007, NOT-006); each
  name gets one.

### 12.6 When a free hub sleeps

A free hub sleeps when no device of the account is connected (HM-R6, R-164). A machine that runs
an application and is connected keeps the hub awake. Thus a free hub sleeps only when each
machine of the account is offline, and then each application is down anyway. Two cases stay:

- The machine is online and the hub sleeps for a different reason (a fault, an update of the
  pool). The door holds the connection while the cloud service wakes the hub, for a time limit,
  and then closes it. It wakes a hub only when the cloud service knows that a machine of the
  account is online (whether it can know this is a question for the cloud plan), and only a
  limited number of times each hour.
- Each machine is offline. The door answers no request for those names. It does not wake the
  hub for a request from the internet: then any person on the internet could keep a free hub
  awake.

OD-HOST-39 asks whether public names are a function of the paid plans only.

### 12.7 Traffic and abuse

Each byte of the public traffic of each hosted customer goes through the pool: in at the door,
through the hub, through the link, and out the same way. The design:

- **Limits for each plan.** The door counts bytes and connections for each hub and each name. Each
  plan has a limit of bytes for each month, of connections at one time and of new connections each
  second for each name. The numbers are a decision of the operator with the prices (OD-HOST-40);
  this text gives none.
- **Over a limit.** The door slows the name, then closes new connections of that name. With option
  R1 the door cannot answer a page inside TLS; the proxy of the hub answers the page when the hub
  is awake.
- **The link of a machine.** Public traffic and the sessions of the operator (SSH, VNC) share the
  tunnel of the link to that machine. The hub limits the streams and the bandwidth of public
  routes for each link and gives the sessions priority. A flood can still slow a session; the
  design makes it less likely, it does not stop it.
- **Abuse.** A page for phishing or malware on a name of the door damages the address of the pool
  for each customer. The design: the terms of use forbid it; an address for reports of abuse; a
  switch of the operator that removes one name from the door at once, with a record that the
  customer sees, a message to the customer, and a way to ask for a review; the name that Anima
  gives (Section 12.3) is below a domain apart from `anima.ing`, so a block of that domain does not
  stop the hubs, but it stops each name below that domain. The legal duties for content are not
  known to this text. HM-R8 (no access of the staff to hubs) stays: the switch acts on the table of
  the door, not in the hub.
- **Attacks on the door.** A flood of connections to one name must not stop the door for each
  hub. The limit of new connections for each name and for each address of a client is at the door.

### 12.8 What a hub that a person hosts needs

- A public address, and ports 80 and 443 of that address that reach the hub. The proxy refuses
  clearly when a different program uses port 80 or 443 (R-213).
- The names of the applications point at that address.
- A path from the hub to the port of the application: the link (Section 11), or the network of
  the machine when the machine has "Open to the network" on (R-229). A port on `127.0.0.1` of a
  machine that has neither is not reachable.
- A hub with no public address (behind a router with no forward of the ports) is not in the first
  version. The screen says so.

### 12.9 Questions for the plan of the cloud service

These questions belong to the private plan of the cloud service. This session wrote them in a
note for the orchestrating session; it did not change that plan.

1. Can the door route TLS by the name in SNI to one hub, with no TLS of its own (option R1), and
   plain HTTP on port 80 by the `Host` header?
2. Which network route from the door to each hub does R-239 give, and does it carry raw TCP, or
   only HTTP?
3. Can the door keep a table of names to hubs, with the `TXT` proof of Section 12.4 made by the
   cloud service itself?
4. Can the door hold a connection while the hub wakes, and for how long?
5. Can the door count bytes and connections for each hub and each name, and slow or stop one name?
6. Which outbound address do the hubs of the pool share, and is the limit of Let's Encrypt for
   each address a problem at the expected number of hubs?
7. Which domain apart from `anima.ing` holds the names that Anima gives, and who enters it on the
   Public Suffix List?
8. Does AC-D45 have a correction for the door that also covers the names of applications?

### 12.10 Options for the operator

- **OD-HOST-30** (exists) decides whether the hub serves the page of an application at all in the
  first version, and under which name. This section adds the public route; the two answers of
  OD-HOST-30 apply to it too.
- **R1 or R2: where TLS ends on a hosted hub.** R1: at the proxy of the hub; the door sees only the
  name in SNI. R2: at the door; the door holds each certificate and sees each request in plain
  text. Recommendation: R1. With R2 a fault of the door shows the traffic of each customer, and the
  door becomes the place of each certificate. Against R1: the door cannot answer a page of its own
  for a name over its limit inside TLS; it can only close the connection. This is a question for
  the review and the cloud plan, not yet for the operator.
- **OD-HOST-39: public names on a free hub?** A: yes, with the limits of the free plan. B: only on a
  paid plan. Recommendation: B. All public traffic goes through the pool, and an always-on route is
  the paid step (HM-R6).
- **OD-HOST-40: the numbers of the limits of each plan** (bytes each month, connections). A
  decision with the prices. No recommendation in this text.
- **OD-HOST-43: a second consent at the machine before an application is public?** A: yes; the
  administrator turns on "public" for the port at the machine, beside the consent of OD-HOST-29.
  B: no; the consent of OD-HOST-29 and the second factor at the hub are enough.
  Recommendation: A. A port that he opened for a look in the hub can be an administration page
  that must never be on the internet.
- **OD-HOST-44: one ACME account for the pool, or one for each hub?** A: one for the pool, held
  by the cloud service. B: one for each hub, held by that hub, with the limit of new accounts for
  each address accepted. Recommendation: B with a limit of new hubs each hour, because with A the
  cloud service holds a key that asks for a certificate of each customer's name. No name that
  Anima gives is offered before its domain is on the Public Suffix List.

The security review of this section by a separate reviewer (2026-11-20) is in
[security-findings.md](../security-findings.md), SF-227 to SF-234. The
corrections of the text are in this version.

## 13. The recipe format of the catalogue (R-300, task H36)

This section is the format of task H36. The crate `crates/catalogue` reads it and runs the static
test. The recipes are data files in `crates/catalogue/entries/`, one folder for each recipe.
Section 13 of the design specification has no folder for data of this kind. The recipes stay in the
crate that reads them, so that one change holds a recipe and its test. The hub carries them: the
build of the crate puts the text of each recipe folder in the binary (task H38), and the hub lists
only the recipes that pass the static test.

### 13.1 One recipe folder

A recipe is one folder. The name of the folder is the identifier of the entry. The folder has three
files and no other file.

| File | Content |
| --- | --- |
| `entry.json` | The entry: the fields of Arcane and the fields of Anima |
| `compose.yaml` | The Compose file. It follows Section 4.5 with no exception |
| `environment.example` | The example environment file. It holds no secret value |

The example file does not start with `.env`. Thus no tool and no agent takes it for a file of
secrets, and no rule of `.gitignore` hides it.

### 13.2 The fields of the entry

The top level of `entry.json` has the fields of the template registry of Arcane (HOST-025). A list
that is made for Arcane has the same names, so a reader of Arcane reads each entry. The fields of
Anima are below the one key `anima`. A reader of Arcane does not read that key. A key that is not
in the tables makes the entry unreadable.

| Field (Arcane) | Rule |
| --- | --- |
| `id` | A name of Section 4.5. It is the name of the folder |
| `name`, `description`, `version`, `author` | Not empty. `version` is the version of the application |
| `tags` | A list with at least one tag. No tag is empty |
| `compose_url` | `compose.yaml` in the folder. The list that ships writes the full address |
| `env_url` | `environment.example` in the folder |
| `documentation_url` | An address that starts with `https://` |
| `icon_url` | An address that starts with `https://`, or `null` |

| Field (`anima`) | Rule |
| --- | --- |
| `kind` | `application` (the default) or `database`: a database server for the applications of the machine (Section 14, task H24) |
| `server` | For an entry of kind `database` only: `engine` (`postgresql`), `major` (the major version of the image) and `extensions` (each extension that the image has and that an application may name) |
| `database` | For an application that needs a database only: `engine` (`postgresql`), `lowest_major`, `extensions`, and `variables`: the name of the variable for each connection value that the application wants, of `url`, `host`, `port`, `name`, `user` and `password`, at least one. The install writes each of them, so each is also in `generated` (Section 14.2) |
| `images` | For each service of the Compose file: `service`, `image`, `tag` and `resolved`. `image` is `repository@sha256:` and 64 hexadecimal digits, with no tag (R-233). `tag` is the tag that was read from the registry to get the digest. `resolved` is the day of that read |
| `marks` | What the recipe needs beyond the default rules (Section 13.3). Empty for most recipes |
| `exceptions` | The exceptions of a fixed list to the rights of a service (Section 13.3), each as `exception` and the `services` that it applies to. Empty for most recipes |
| `class` | `application` (the default) or `shell`: an entry that is a shell (Section 15.7, OD-HOST-54). A `shell` has at least one variable in `generated`, its credential, and no `network-port` mark |
| `origin` | None for an entry that a person wrote. For a generated entry (tier 2, Section 15.4): `from` (`linuxserver`) and `read`, the day of the data of the publisher |
| `streams` | Each published port other than the port of the test, as `port` (in the container) and `protocols`: the streams that a route of task H40 can carry. Each is a `ports` entry of the file with each protocol |
| `optional` | The parts that are off because no rule of a file permits them yet, each with a setting of the machine: `{"part": "device", "path": "/dev/dri"}`. The path starts with `/dev/`. The Compose file does not have the part |
| `volumes` | Each top-level volume of the Compose file, with the kind of copy that it needs (Section 13.4) |
| `generated` | The variables that the install makes, for example a password. The example file has no value for them |
| `test` | The test of a machine (Section 13.5). Task H37 runs it |
| `pinned` | The day of the last pin of the digests |
| `last_test` | The last test on a machine: `not-tested`, or `passed` or `failed` with the day |

Each day is written `YYYY-MM-DD`.

### 13.3 The marks

A mark says that a recipe needs more than the default rules of Section 4.5. The static test runs the
rules of task H28 with the settings that exactly the marks permit. A mark that the file does not need
is an error. A need that has no mark is an error.

| Mark | What it needs | State |
| --- | --- | --- |
| `network-port` | A port on each address of the machine. The mark has the port and its protocols (`tcp`, `udp`). The machine needs "Open to the network" (R-229) | Permitted. The ports on an address other than `127.0.0.1` must be exactly the ports of the marks |
| `engine-socket` | The socket of the engine (R-247) | Refused until the exception of Section 10 exists. Nothing installs such an entry |
| `host-network` | The network of the machine itself, for example for the discovery of devices | Refused. No rule permits it |
| `device` | A device of the machine, for example a USB stick or a graphics processor | Refused. No rule permits it |

Each service of a recipe has `security_opt: [no-new-privileges:true]` and `cap_drop: [ALL]`,
and no setting makes it root or changes its code at the start (SF-224). The static
test reads these values in the file as its author wrote it, before the variables are filled, and
refuses with `privilege_not_permitted` and one reason:

| Reason | What the file has |
| --- | --- |
| `image_mods` | The variable `DOCKER_MODS`: the image gets and runs more code at the start |
| `install_packages` | The variable `INSTALL_PACKAGES` |
| `install_pip_packages` | The variable `INSTALL_PIP_PACKAGES` |
| `version_not_fixed` | The variable `VERSION` with a value other than `docker`: the image gets a different version at the start, which no digest names |
| `root_identity` | `PUID` or `PGID` that is not a whole number above 0 |
| `root_user` | `user` that is `0` or `root`, as the user or as the group, or a variable |
| `new_privileges` | A service with no `security_opt: [no-new-privileges:true]` |
| `capabilities_kept` | A service whose `cap_drop` is not `[ALL]` and not the drop of an exception of the entry |
| `exception_not_needed` | An exception names a service that does not use it, or no service |
| `entrypoint` | A service has `entrypoint`, and no exception `reviewed-command` names it |
| `shell_command` | The `command` or the test of the `healthcheck` of a service runs a shell (`sh`, `bash`, `ash`, `dash`, `zsh`, `busybox`, `env`, `-c`, the forms `CMD-SHELL` and one text), and no exception `reviewed-command` names it |
| `generated_exception` | An entry with an `origin` names `reviewed-command`, which only a person gives |

The list of exceptions is fixed. Each exception names the services that it applies to, and no
other service has it (SF-251). `init-capabilities`: an init that starts
as root, gives the files to a different user and changes to it (the s6 init of the LinuxServer
images, `gosu`, `su-exec`). Such a service keeps exactly `CHOWN`, `DAC_OVERRIDE`, `FOWNER`,
`SETGID` and `SETUID`. `reviewed-command`: a person read the `entrypoint`, the `command` and the
health check of the service, which run a shell; a generated entry never has it. The variables
`DOCKER_MODS`, `INSTALL_PACKAGES` and `INSTALL_PIP_PACKAGES` are refused also in a word of a
command, with the exception or without it. Contract K10 has no `cap_add`, so the service drops each other capability
of the default set of the engine by name: `AUDIT_WRITE`, `FSETID`, `KILL`, `MKNOD`,
`NET_BIND_SERVICE`, `NET_RAW`, `SETFCAP`, `SETPCAP` and `SYS_CHROOT`.

The image, a published port, its `host_ip`, the `command`, the `entrypoint` and the `hostname`
are written in the file, never as a variable: the install could change a variable after the
static test (`variable_not_permitted`, SF-226, SF-251). A word of a
command or of a health check that gives a value to a name that marks a secret (`--password=x`,
`--token x`, `API_KEY=x`) is refused as `secret_in_file`; a name that ends with `FILE` names a
path.

The rules of Section 4.5 permit a published port from 1024 only. "Open to the network" does not
change that. Thus a DNS server on port 53 (Pi-hole, AdGuard Home) is refused today, also with its
mark. The recipe of Pi-hole is in the folder with its mark. Its test gives the refusal
`port_not_permitted`, and it is not listed. A question for the operator: does a `network-port` mark
permit a port below 1024, and with which setting of the machine?

### 13.4 The data of a recipe

Each top-level volume has one kind of copy. Task H25 (HOST-042) uses it.

| Kind | Meaning |
| --- | --- |
| `files` | Plain files. A copy of the files is good at any time |
| `sqlite` | An SQLite database. A copy needs the backup function of SQLite or a stopped service |
| `postgresql` | A PostgreSQL server. A copy needs `pg_dump` or a copy of the stopped server |
| `cache` | Data that the application makes again. No copy is needed |

### 13.5 The test of a machine

The test is data only. Task H37 runs it on a throwaway machine after the install.

| Field | Rule |
| --- | --- |
| `service` | A service of the Compose file |
| `port` | The port in the container. A `ports` entry of the service publishes it. For an entry of kind `database`, an `expose` entry of the service names it: a server publishes no port (Section 14.1), and the test reaches it on the network of the project |
| `reach` | `http` or `tcp` |
| `path` | For `http`: a path that starts with `/`. For `tcp`: none |
| `statuses` | For `http`: each HTTP status that counts as healthy, from 100 to 599. For `tcp`: none |
| `body` | For `http`: a text that the body of a healthy answer contains, or none. For `tcp`: none |
| `limit_seconds` | The most seconds from the start of the install to a healthy answer, from 1 to 900 |

### 13.6 The example environment file

The example file has lines `NAME=value` and comments. It holds no secret value.

- The line of a variable in `generated` has no value. The line directly above it is
  `# generated at install`. The install makes the value. Anima never writes it to the catalogue.
- A line with no value directly below that comment names a variable of `generated`.
- A variable whose name has the part `PASS` or `KEY`, or has the text `PASSWORD`, `PASSWD`,
  `SECRET`, `TOKEN`, `APIKEY`, `CREDENTIAL` or `PRIVATE_KEY` in capital or small letters, has no
  value (SF-225).
- In the Compose file, a variable of `environment` with such a name has exactly one variable as
  its value, `${NAME}`, with no default (`secret_in_file`).

The static test fills the variables of the Compose file from this file, with the rules of
Section 4.5. Thus each variable of the file has a line, or a default with `:-`.

### 13.7 The static test and its codes

The static test reads each recipe folder. It installs nothing, starts no container and reaches no
machine. Each failure is a fixed code with the identifier of the recipe. No code holds text of a
file. One recipe can have more than one code.

| Code | Meaning |
| --- | --- |
| `entry_not_readable` | The entry file is not there, is not JSON, or has a key or a form that is not permitted |
| `id_not_permitted` | The identifier is not a name, or is not the name of the folder |
| `field_empty` | A text field or the list of tags is empty |
| `address_not_permitted` | An address is not `https://`, or a file address is not the fixed name in the folder |
| `file_not_found` | The Compose file or the example file is not in the folder |
| `date_not_permitted` | A day is not `YYYY-MM-DD` |
| `image_not_pinned` | An image is not named by a digest |
| `image_not_listed` | A service has an image that the entry does not give for it |
| `image_not_used` | The entry gives an image that no service has |
| `compose_refused` | The rules of task H28 refuse the file. The second part is the code of the rule, or `not_at_machine` |
| `mark_not_needed` | The file passes the default rules, so the mark is not needed |
| `mark_not_matched` | The ports on the network are not the ports of the `network-port` marks |
| `mark_not_permitted` | A mark that no rule permits yet. The second part is the mark |
| `test_not_complete` | The test of a machine is not complete (Section 13.5) |
| `volume_not_described` | A top-level volume has no kind of copy, or a described volume is not in the file |
| `secret_in_example` | The example file has a value for a secret |
| `generated_not_marked` | A generated variable is not marked, or a marked line is not in `generated` |
| `machine_test_failed` | The last test on a machine failed. The entry leaves the list |
| `database_not_described` | The fields about a database are not complete: an entry of kind `database` has no `server` or has a `database`; an application has a `server`; or a need has no variable, a variable twice, a variable that is not in `generated`, a lowest major of 0 or an extension that is not a name |
| `database_port_published` | An entry of kind `database` publishes a port on the machine (Section 14.1) |
| `variable_not_permitted` | The image, a published port or its `host_ip` is a variable (Section 13.3) |
| `secret_in_file` | The Compose file gives a variable whose name marks a secret a value that is not one variable (Section 13.6) |
| `privilege_not_permitted` | A service keeps rights that it does not need, or changes its code at the start. The second part is the reason (Section 13.3) |
| `stream_not_published` | A stream of the entry is not a published port of the file with each of its protocols |
| `optional_not_permitted` | An optional part is not a device below `/dev/` |
| `shell_not_contained` | An entry of the class `shell` has no generated credential, or a `network-port` mark (OD-HOST-54) |

A test of the crate reads the folder `entries/` and requires each recipe to pass, with one known
refusal: Pi-hole (Section 13.3). Thus `bun run gates` runs the static test of each recipe.

### 13.8 How a large list stays tested

The static test proves the form only. A list of many entries stays correct only with tests on a
machine.

1. Task H37 installs each entry on a throwaway machine, waits for health, runs the test of the
   entry, removes the entry, and records the day and the result in `last_test`.
2. A schedule reads the registry for the tag of each image, pins the new digest, and runs the test
   of a machine again. The pin and the test go in one change. `pinned` and `resolved` show the day.
3. The catalogue shows the day of the last test of each entry. An entry that is not tested on a
   machine says so.
4. An entry whose last test failed leaves the list (`machine_test_failed`). It comes back when a
   test passes.
5. An entry with a code of the static test is not listed.

No entry of task H36 has been tested on a machine. Each says `not-tested`.

### 13.9 The first entries (task H36)

The digests were read on 2026-11-20 from the public registry of Docker Hub with the HTTP API of the
registry: a request for the manifest, with no pull and no login. Each digest is the digest of the
index of all platforms of the tag.

| Entry | Images (tag) | Marks | Data | Static test |
| --- | --- | --- | --- | --- |
| OwnTracks Recorder | `owntracks/recorder` (1.0.7) | None | `files` | Passes |
| Dawarich | `freikin/dawarich` (1.23.0), `postgis/postgis` (17-3.7-alpine), `redis` (7.4.12-alpine) | None | `files`, `postgresql`, `cache` | Passes |
| Pi-hole | `pihole/pihole` (2026.11.0) | `network-port` 53, `udp` and `tcp` | `sqlite` | Refused: `port_not_permitted` |
| Uptime Kuma | `louislam/uptime-kuma` (2.5.12) | None | `sqlite` | Passes |
| Vaultwarden | `vaultwarden/server` (1.37.5) | None | `sqlite` | Passes |
| Gitea | `gitea/gitea` (28.4.0) | None | `sqlite` | Passes |
| PostgreSQL (kind `database`, task H24) | `postgres` (18.7) | None | `postgresql` | Passes |

Each service has `no-new-privileges:true` and drops each capability (Section 13.3). The services
whose image starts as root and changes to a different user have the exception
`init-capabilities`: the database and the cache of Dawarich, Pi-hole, Uptime Kuma, Gitea and
PostgreSQL. Each published port is written in the file, not as a variable (SF-226).
No test on a machine has shown that each image starts with these rights; task H37 shows it. The
server of OpenSSH in the image of Gitea possibly needs `SYS_CHROOT`, which no exception keeps.

Dawarich passes the default rules. Its Compose file is not the file of the project: it has no
`container_name`, no `stdin_open`, no `tty`, no `shm_size`, no options of the log driver and no
`deploy` key, because Section 4.5 refuses each of them. PostgreSQL thus has the default shared
memory of the engine (64 MiB). The test on a machine must show that this is enough.

### 13.10 A proposed list of the first thirty entries

The order follows what people self-host most. "Open to the network" is the `network-port` mark.
An entry with a refused mark is not listed until a rule permits its mark.

| # | Entry | Marks | Database |
| --- | --- | --- | --- |
| 1 | OwnTracks Recorder | None | None |
| 2 | Dawarich | None | PostgreSQL with PostGIS; Redis |
| 3 | Pi-hole | `network-port` 53 `udp` and `tcp` (below 1024: refused today) | SQLite |
| 4 | AdGuard Home | `network-port` 53 `udp` and `tcp` (below 1024: refused today) | None |
| 5 | Uptime Kuma | None | SQLite |
| 6 | Vaultwarden | None | SQLite |
| 7 | Gitea | None | SQLite |
| 8 | Forgejo | None | SQLite |
| 9 | Nextcloud | None | PostgreSQL; Redis |
| 10 | Immich | None (`device` only for hardware decoding) | PostgreSQL with its vector extension; Redis |
| 11 | Jellyfin | None (`device` only for hardware decoding) | SQLite |
| 12 | Home Assistant | `host-network` for the discovery of devices; `device` for a radio stick | SQLite |
| 13 | Paperless-ngx | None | PostgreSQL; Redis |
| 14 | Audiobookshelf | None | SQLite |
| 15 | Navidrome | None | SQLite |
| 16 | Syncthing | `network-port` 22000 `tcp` and `udp`, 21027 `udp` | None |
| 17 | Arcane | `engine-socket` (R-247) | SQLite |
| 18 | Portainer | `engine-socket` (R-247) | Files |
| 19 | Dozzle | `engine-socket` (R-247) | None |
| 20 | Homepage | None (`engine-socket` only for the state of containers; left out) | None |
| 21 | FreshRSS | None | SQLite |
| 22 | Miniflux | None | PostgreSQL |
| 23 | Mealie | None | SQLite |
| 24 | Actual Budget | None | Files |
| 25 | Stirling-PDF | None | None |
| 26 | Linkding | None | SQLite |
| 27 | Grafana | None | SQLite |
| 28 | n8n | None | SQLite |
| 29 | Mosquitto | `network-port` 1883 `tcp` (for the phones of OwnTracks) | None |
| 30 | Open WebUI with Ollama | None (`device` only for a graphics processor) | SQLite |

Not proposed: Watchtower, because it updates images by itself (ETH-004, HOST-044); a VPN server
such as WireGuard, because it needs a capability of the kernel that no mark names.

## 14. The PostgreSQL entry (R-216, R-301, task H24)

This section is a design with options. It adds no approved requirement. Section 14.6 says what
task H24 built of it. HOST-041 is approved (R-216); the levels of HOST-048 are OD-HOST-38 of the plan. The design is
the same for each level: level 2 adds a replica to the server of level 1.

### 14.1 What the entry is

- **A server of PostgreSQL on a machine.** It is an entry of the catalogue of kind `database`,
  installed with the work type `application_apply` as each entry. Its image is the official image
  `postgres` of one major version, named by a digest (R-233). A variant with an extension (for
  example PostGIS, which Dawarich needs) is a second entry of the same kind with its own image.
- **One server for many applications of the machine.** He said "letting one server be the db".
  Thus the default is one server on a machine, and each application that needs PostgreSQL gets a
  database and a user of its own on it. OD-HOST-41 asks whether a server of its own for each
  application is better.
- **No published port.** The server publishes no port on the machine. An application reaches it
  on a network of Docker that is only for that pair: `anima-db-<application>`. The server joins
  the network of each application that has a database on it; two applications never share a
  network, so one application cannot reach the containers of a different one.
- **Change of the contract.** Section 6.4.4 of K10 does not permit a file from the hub to join a
  network of a different project. The applications program must make and join these networks
  itself, as a fixed step of the install of an application that names a database. That is a change
  of the contract, for its owner and a security review (CON-001). No code before it.

### 14.2 A database for an application

1. The recipe of the application says that it needs PostgreSQL, the lowest major version, the
   extensions, and the names of the variables in which it wants the connection values (for example
   `DATABASE_URL`, or `POSTGRES_HOST` and four more). Section 13 has the field.
2. At the install the operator selects a server on the same machine, or Anima installs one.
3. The applications program makes the database and the user with fixed statements in the server
   (no text of the hub in a statement). The names are made from the identifier of the application
   and must match `[a-z][a-z0-9_]{0,62}`. The user owns only its database and has no other right.
4. The password of the user is made on the machine by the applications program, written only into
   the environment file of the application (R-231 for its keep), and never sent to the hub.
   OD-HOST-42 asks whether the operator must see it.
5. The applications program writes the connection values into the variables that the recipe
   names, and starts the application.
6. At the removal of the application, the database and the user are deleted only with the setting
   "The hub may delete volumes" at the machine (R-230), as the volumes of each application.

### 14.3 Copies and the way back (level 1)

HOST-042 (copies with a way back) is not approved; OD-HOST-1 is open. This is the design that its
approval would build for PostgreSQL:

- **The copy.** `pg_dump` in the custom format for each database, at an interval that the operator
  sets, run by the applications program in the server container. A copy of the volume of a server
  that runs is not used: it is not safe (OD-HOST-12).
- **Where it goes.** First into a folder of the applications program on the machine, then to a
  second machine that the operator selects. The path to the second machine is the question of
  OD-HOST-1.
- **The way back.** `pg_restore` into a new database, then a switch of the application to it, at
  the request of the operator only (ETH-004). The old database stays until the operator deletes it.
- **The test of the way back.** At intervals the program restores the last copy into a database
  for the test, checks that each table of the copy is there, and deletes the test database. A copy
  that fails this test makes a notification. A copy function is built only with its way back and
  this test (HOST-042).

### 14.4 A primary and a replica (level 2)

Only when OD-HOST-38 says B. The second server is on a machine that the operator selects, and
follows the first by the streaming replication of PostgreSQL over the network between the two
machines. A switch to the replica is an action of the operator only (ETH-004, HOST-006): Anima
never switches by itself. The replica needs a path between two machines that this plan does not
have yet; the design of that path comes with the decision.

### 14.5 Versions

- A new digest of the same major version is pinned and tested as each entry (Section 13).
- A new major version needs a dump and a restore, or `pg_upgrade`. Anima never changes the major
  version by itself: the operator starts it, and Anima makes a copy first.

### 14.6 What task H24 built, and where it differs from this design

Task H24 built the part that needs no machine (2026-11-20, branch `hosted/apps-h38`):

- **The entry.** `crates/catalogue/entries/postgresql`: kind `database`, the official image
  `postgres` with the tag 18.7, named by the digest of the index of all platforms of that tag,
  read on 2026-11-20 from the registry of Docker Hub with a request for the manifest (no pull,
  no login). The image is the Debian variant, not Alpine, so that the collations of text are
  those of the C library of most servers. The password of the superuser is generated at the
  install. The server has `expose` 5432 and no `ports` entry.
- **The format.** The fields `kind`, `server` and `database` and the codes
  `database_not_described` and `database_port_published` of Section 13.
- **The pure functions.** `catalogue::postgresql`: the names of an application, the statements
  of the setup and of the removal, the connection values, and the argument lists of the copy,
  the way back and its test, with their tests. Nothing runs them yet.

Where the build differs from Sections 14.1 to 14.3:

- **The names.** Section 14.2 says that the names are made from the identifier. The identifier of
  an entry may have `-`, which a name of PostgreSQL without quotes may not have: each `-`
  becomes `_`. An identifier with `_` or with `--` is refused, so that two identifiers never
  get the same names (`my-app` and `my_app`) and no name of an application has `__`
  (SF-223). The names `postgres`, `template0`, `template1`, `public` and each name
  that starts with `pg_` are refused.
- **The extensions.** An extension is one of a fixed list, not a pattern: `vector`, `pg_trgm`,
  `uuid-ossp`, `citext`, `hstore`, `postgis`, and the contributed extensions that the entry
  `postgresql` names (`btree_gin`, `btree_gist`, `cube`, `earthdistance`, `fuzzystrmatch`,
  `pgcrypto`, `unaccent`). No extension of the list reaches a file, a program or a different
  server (SF-223).
- **The password in a statement.** The password is an input value; the crate never makes one
  (OD-HOST-42 A). It is 22 to 128 letters and digits of ASCII (at least 128 bits), so that the
  quoted literal of the `CREATE ROLE` statement is the same text with each setting of the
  server, and no `$`, backtick, `#` or `=` changes it in an environment file, a shell or a
  connection address (SF-223). The program must give that statement to `psql` on its standard input, never as
  an argument. `Debug` of a statement or of a connection value does not show it.
- **The test of a machine.** It reaches the port that the server exposes on the network of its
  project, not a published port (Section 13.5).
- **The way back.** The new database is `<database>__back_<number>`, the database of the test is
  `<database>__check`. No name of an application has `__`, so neither is the name of a different
  application. A name longer than 63 bytes is refused, not cut.
- **The test of the way back** compares the tables of `pg_restore --list` with the tables of the
  restored database. A table whose name has a space is reported as missing: the list does not
  quote names. That is a false failure, never a false pass.

Not built: the networks `anima-db-<application>` and the change of contract K10 that they need
(Section 14.1, CON-001), each part on a machine (task H6), and a replica (OD-HOST-38).

### 14.7 Questions for the operator

- **OD-HOST-41: one server for each machine, or one for each application?** A: one server on a
  machine, a database and a user for each application (his words). B: a server of its own in the
  project of each application, as most Compose files of applications do now. Recommendation: A.
  It matches his words, uses less memory, and level 2 makes one replica of one server, not one for
  each application. Against A: the change of the contract of Section 14.1, and one server that
  stops stops each application on it.
- **OD-HOST-42: must the operator see the password of a database?** A: no; it stays on the
  machine, in the environment file of the application. B: the hub shows the connection text one
  time, to a person with a login (closed to each API key and MCP tool, as the log of a container).
  Recommendation: A first. HOST-041 says "shows its connection text one time"; with A that text is
  shown only at the machine, so the hub never holds it. B is an addition when a tool outside the
  machine needs the database.

## 15. The images of LinuxServer.io as the base of the catalogue (R-319, HOST-049)

This section is a design with options. It adds no approved requirement and approves no tier and
no list. A security read by a separate reviewer reads it. The facts are from a read of the fleet
list of LinuxServer.io on 2026-11-20
(`https://api.linuxserver.io/api/v1/images?include_config=true&include_deprecated=true`, updated
2026-11-17) and of Docker Hub; the counts were made by a script from that list. A number marked
"estimate" is a judgement.

### 15.1 How this widens R-215, and what he accepts

R-215 said: a small list that Anima tests. R-300 widened it to a large list in which
each entry is tested. R-319 makes the images of one publisher, about 200 of them, the
base. Thus he accepts, if he selects the tiers of Section 15.4:

- Entries that Anima did not test on a machine are in the catalogue, with a mark that says so.
- Tier 2 (Section 15.4) changes R-300 (2) and (3), which keep "each entry is tested
  by Anima": it needs a new ruling, not only an answer to OD-HOST-50.
- The images come from one publisher, LinuxServer.io, which Anima does not control. Anima pins
  each image by a digest (R-233). The image is pinned; code that the image downloads when it
  starts is not (Section 15.9).
- These images start as `root` and then change to the user of `PUID` and `PGID`; they need the
  default capabilities of Docker for that. The rules of a file from the hub permit this today;
  Section 15.9 adds what contains them.
- A pinned digest is about one week old before the publisher makes a newer one (Section 15.5).

### 15.2 The list and its shape

- **202 images** are current (224 in the list, 22 marked as no longer kept). Docker Hub shows 293
  repositories under `linuxserver`, old ones included.
- Nearly each image takes `PUID`, `PGID` and `TZ`, keeps its state in `/config`, and publishes one
  or a few ports. 156 need only `/config`. 160 run on x86_64 and arm64; 42 on x86_64 only.
- The list gives, for each image, a machine-readable `config`: the variables, the volumes, the
  ports (with `/udp`), the devices, the capabilities, `security_opt`, `shm_size` and `sysctls`, the
  host network and `privileged`, each with a mark "optional". The same data is in
  `readme-vars.yml` of each repository. Thus an entry can be **generated** from it.
- Gaps of that data: 4 images have no `config`; the device `/dev/dri` of Plex and Jellyfin is only
  in the text of their documentation; a path of the host is a placeholder.

### 15.3 How many fit the rules of a file from the hub

| Group | Count | What it is |
| --- | --- | --- |
| Fit as they are | 70 | `/config` and data as named volumes, ports from 1024, nothing that the rules refuse |
| Fit when `shm_size` is a permitted key | 85 | Most of the 94 desktop images in a browser (KasmVNC, Selkies), which ask for `shm_size: 1gb`; the count of desktops among the 70 is not made yet. `shm_size` is not a key of the list of Section 4.5 today (OD-HOST-51) |
| Fit when the port of the host is changed | 20 | The default port of the host is 80 or 443 (or 514 and 601 for `syslog-ng`); the entry uses a port from 1024 |
| Fit with an optional part off | 9 | For example Emby and tvheadend with no `/dev/dri`; Jellyfin is here in practice (no hardware transcoding, no discovery on the local network) (estimate) |
| Do not fit | 18 | `privileged` (`kasm`, `qemu-static`); the host network (`plex`, `homeassistant`, `duckdns`, `fail2ban`, `wireshark`); `NET_ADMIN` (`wireguard`, `swag`, `mullvad-browser`); `IPC_LOCK` (`vscode`, `vscodium`, `github-desktop`, `mysql-workbench`); devices that are required (`minisatip`, `oscam`); the socket of the engine (`sealskin`, `socket-proxy`) |

The 18 that do not fit stay "install at the machine and adopt" (R-228): the screen shows the
reason and the Compose file to start at the machine. Plex is the sixth image by pulls; in a bridge
network with port 32400 it can work with no discovery on the local network (estimate; it needs a
test). No current image of the list needs port 53, so OD-HOST-45 does not block this list.

Nine images need UDP ports (`deluge`, `qbittorrent`, `transmission`, `syncthing`, `luanti`,
`unifi-network-application`, `syslog-ng`, `minisatip`, `wireguard`). A UDP port on the machine is
in the rules; a UDP route through the hub is Section 16.

### 15.4 Three tiers

- **Tier 1, tested by Anima.** The test of task H37 passed on this exact digest: the entry starts, is
  healthy within its time, its port answers (the list gives `CI_PORT` for its own tests), it stops
  and is removed with nothing left outside its volumes. "Tested" means this, not that each
  function of the application works, and it is not a security review; the screen says so. One
  click; an API key or an agent may ask for its install (R-234) only by the entry and its digest,
  and the hub refuses unless that digest passed the test at the time of the request. An entry of
  the class of Section 15.7 is installed by a person only, in each tier.
- **Tier 2, generated from the data of the publisher.** The static test of task H36 passed, the
  machine test did not run or is older than the digest. The screen says "Generated from the data of
  LinuxServer.io on <date>; not tested by Anima". Only a person with a login or a browser session
  installs it, after a confirmation; no API key and no agent (R-234 covers the tested catalogue
  only).
- **Tier 3, install at the machine.** The rules refuse it. The screen gives the reason and the file.

The generator reads the list of the publisher, writes the entry of Section 13 for each image (only
the parts that are not optional, a port of the host from 1024, named volumes), resolves the digest
of the multi-arch index of the tag `X.Y.Z-lsN` from the registry (not from the metadata), runs the
static test, and puts the entry in tier 2 or tier 3. A person reading about 200 changed digests
proves nothing, and the merge train needs no person. Thus the generator itself fails, and writes
nothing, for each change that is not a new digest or a new version: a new key, port, variable,
volume or capability, or a change of tier. Such a change needs a named reviewer, and the merge rule
refuses a branch of the generator without one (Section 15.9).

### 15.5 Keeping about 200 pins current

- The publisher builds about each week: 191 of 202 images had a new build in the 7 days before
  2026-11-17, mostly with the same version of the application. An old tag stays available.
- Thus a new pin for each build is about 200 changes each week, and a machine test for each is
  about 30 each day. The proposal: a new pin when the version of the application changes, when an
  advisory names the image, and else each month. Each new pin of a tier-1 entry runs the machine
  test again; until it passes, the entry is tier 2.
- The schedule runs as a workflow of the repository that writes a branch; the merge train lands
  it. It changes no server.
- Who runs the machine tests (a test machine of the operator, or a machine of the cloud) is a
  decision with a cost (OD-HOST-53).

### 15.6 What the screen shows

A list with a filter by tier, a search, the marks (`network-port`, UDP, `desktop`, a database),
for each entry its tier, its version, the date of its pin and of its last test; the page of an entry
with its description, its ports, its volumes, the names of its variables, the Compose file, and
for tier 3 the reason. The install button of tier 2 asks for the confirmation; of tier 3 it is
replaced by "Install at the machine".

### 15.7 A risk of its own: entries that are a shell

94 images are a desktop in a browser (KasmVNC or Selkies); their user `abc` has `sudo` with no
password, and by default the desktop asks for no password (estimate from their documentation;
not checked). The same class holds `code-server` and `openvscode-server` (no password when the
variable is not set), `openssh-server` (port 2222, which goes around the rule of port 22 of Section
12.3), `rdesktop` (3389), and web pages with no sign-in by default (Syncthing, Transmission). Who
reaches one is root in a container that can reach the local network and the network of Anima.
The proposed rule, the class `shell`:

- The class is given by a fixed list in the repository and by the base image, never by a field
  of the data of the publisher.
- The machine makes a credential that the entry must use; without one the entry is refused.
- No public route and no stream; a private route only if OD-HOST-54 says so; a person installs it,
  never an API key or an agent, in each tier. The same rule applies to "Open to the network".

### 15.8 Options for the operator

- **OD-HOST-54: the class `shell` (Section 15.7)?** A: as written, with no route at all (only a
  session through the hub or "Open to the network" with the made credential). B: a private route
  is permitted with the made credential. Recommendation: A first. "Private" is each node of the
  network of Anima, the containers of each machine included (Section 16.7).

- **OD-HOST-50: the three tiers?** A: tiers 1, 2 and 3 as above; this changes R-300
  (2) and (3) and needs a ruling of its own. B: only tier 1 is shown; the others appear when their
  machine test passes. Recommendation: A. With B the list is nearly empty until a test machine runs
  for weeks.
- **OD-HOST-51: `shm_size` as a permitted key?** It lets 85 more images in, most of them of the
  class `shell`. A: yes, a size in the form of Section 4.5, not more than the memory limit of the
  service, with `ipc` still refused. B: no. Recommendation: A, with the rule of Section 15.7. The
  shared memory is counted in the memory of the container and gives no right on the machine while
  `ipc` stays refused. It is a change of contract K10
  Section 6.4.4, for its owner.
- **OD-HOST-52: how often a new pin?** A: on a new version of the application, on an advisory, and
  else each month. B: each build of the publisher (about each week). Recommendation: A.
- **OD-HOST-53: where the machine tests run?** A: a test machine of yours, each night, a fixed
  number of entries. B: a machine of the cloud for each run (a cost each month). Recommendation: A
  first; it needs a machine that you name and is a step of yours. The test machine runs images
  that are not trusted: it is not on the network of Anima or your local network, its outbound
  traffic is recorded, and it is wiped after each run.

### 15.9 Rules from the security read

- **Code that an image downloads.** The init of LinuxServer.io reads `DOCKER_MODS` and downloads
  and runs other images at the start; `INSTALL_PACKAGES` and `INSTALL_PIP_PACKAGES` install
  packages; some images (Plex with `VERSION=latest`) download the application. The generator and
  the applications program refuse these variables and each `VERSION` that is not `docker` or a
  fixed version. The test of task H37 records the outbound connections at the start; one that is
  not on the recorded list fails the test.
- **The workflow of the generator.** It reads the metadata with a safe reader, runs nothing that
  it fetched, and has a token that can push only its own branch.
- **Root.** `PUID` or `PGID` of 0 is refused. Each generated entry gets `no-new-privileges:true`
  and drops `MKNOD`, `SETFCAP`, `SYS_CHROOT` and `AUDIT_WRITE`; the test shows that the image still
  works, and an exception is listed by name.
- **The tier at the hub.** The applications program cannot tell a tier. The hub checks the tier and
  the digest at the moment of each request; a new pin puts an entry back in tier 2 until its test.
- Whether a release carries the data of the catalogue to hosted hubs with no further step is a
  question for the cloud plan.

### 15.10 What the rest still needs (task H39)

The generator of task H39 (`crates/catalogue/src/bin/generate.rs`) ran the form of
`catalogue::linuxserver` over the fleet list of 2026-11-17 (202 current images, read on
2026-11-21). The form keeps only the parts that are not optional, each volume of a placeholder
path as a named volume, a port of the machine below 1024 moved up by 8000, each port on
`127.0.0.1` with its protocol, `PUID` and `PGID` of 1000, the exception `init-capabilities` for
its one service, and each secret or credential as a variable that the install makes. Ten images
are entries now, in `crates/catalogue/entries/` with an `origin`, pinned by the digest of the
index of each tag read from `lscr.io` on 2026-11-21: `calibre-web`, `code-server` (class `shell`,
its `PASSWORD` made at the install), `duplicati`, `freshrss`, `heimdall`, `jellyfin` (the device
`/dev/dri` as an optional part that is off), `nextcloud`, `qbittorrent` (the stream 6881, TCP and
UDP), `radarr` (three volumes) and `sonarr`. Totals: **81 fit** (55 as they are, 16 with a port
moved, 10 with an optional part off) and **121 do not fit**. The counts differ from Section 15.3
because the form also refuses a variable with no value, an image with no TCP port, and a shell
with no credential.

| Place or reason | Count | What it still needs | Images |
| --- | --- | --- | --- |
| fits | 55 | A pin and a test on a machine (task H37) for tier 1. Ten are entries now | `adguardhome-sync`, `apprise-api`, `audiosilo`, `babybuddy`, `bazarr`, `beets`, `bitcoin-knots`, `calibre-web`, `changedetection.io`, `code-server`, `deluge`, `duplicati`, `faster-whisper`, `flexget`, `grocy`, `habridge`, `hedgedoc`, `hishtory-server`, `htpcmanager`, `jackett`, `kavita`, `lazylibrarian`, `ldap-auth`, `lidarr`, `limnoria`, `mariadb`, `medusa`, `mstream`, `mylar3`, `ngircd`, `nzbget`, `nzbhydra2`, `ombi`, `pairdrop`, `piper`, `planka`, `prowlarr`, `pwndrop`, `pydio-cells`, `pyload-ng`, `radarr`, `raneto`, `resilio-sync`, `sabnzbd`, `sickgear`, `sonarr`, `synclounge`, `tautulli`, `thelounge`, `transmission`, `ubooquity`, `unifi-network-application`, `vscodium-web`, `wikijs`, `znc` |
| fits with a port moved | 16 | The same; the person reaches the port of the machine from 1024 (80 is 8080, 443 is 8444 because 8443 is the port of the hub of Anima, SF-252; a port that a different entry of the catalogue takes moves to the next free port, so heimdall publishes 8081 and 8445, SF-254) | `cops`, `diskover`, `dokuwiki`, `freshrss`, `grav`, `heimdall`, `kimai`, `librespeed`, `nextcloud`, `nginx`, `phpmyadmin`, `piwigo`, `projectsend`, `smokeping`, `syslog-ng`, `xbackbone` |
| fits with an optional part off | 10 | The same; the part (a device, a capability, a key of Compose) needs a setting of the machine and a change of contract K10 | `airsonic-advanced`, `emby`, `jellyfin`, `kdenlive`, `lm-studio`, `orcaslicer`, `ppsspp`, `qbittorrent`, `scummvm`, `tvheadend` |
| shared memory | 84 | The key `shm_size` in contract K10 Section 6.4.4 and in `crates/compose-rules` (OD-HOST-51 A, for the owner of the contract); then most are desktops of the class `shell` and need their credential (`PASSWORD`) in the fixed list of the form | `altus`, `ardour`, `audacity`, `azahar`, `bambustudio`, `blade-of-agony`, `blender`, `boinc`, `brave`, `calibre`, `calligra`, `chrome`, `chromium`, `craft-apps`, `cura`, `darktable`, `digikam`, `dogwalk`, `dolphin`, `dosbox-staging`, `doublecommander`, `duckstation`, `eden`, `ferdium`, `filezilla`, `firefox`, `flycast`, `freecad`, `gimp`, `gitqlient`, `gzdoom`, `handbrake`, `helium`, `inkscape`, `intellij-idea`, `joplin`, `kali-linux`, `keepassxc`, `kicad`, `krita`, `libreoffice`, `librewolf`, `lollypop`, `mame`, `mediaelch`, `melonds`, `modrinth`, `msedge`, `nomacs`, `obsidian`, `onlyoffice`, `openshot`, `opera`, `pcsx2`, `pelorus`, `pidgin`, `pycharm`, `qdirstat`, `rawtherapee`, `remmina`, `retroarch`, `rpcs3`, `rustdesk`, `shadps4`, `shotcut`, `signal`, `spotube`, `sqlitebrowser`, `steam`, `telegram`, `thunderbird`, `ungoogled-chromium`, `vivaldi`, `vlc`, `webcord`, `webstation`, `webtop`, `weixin`, `winegui`, `wps-office`, `xemu`, `yaak`, `zen`, `zotero` |
| value needed | 8 | A value that only the person or a different service gives (the address of a database, the address of the application, a key of a form): a database entry of the machine (MariaDB) and a form for the values of the person at the install | `bookstack`, `healthchecks`, `lychee`, `manyfold`, `mastodon`, `netbox`, `speedtest-tracker`, `your_spotify` |
| no port | 7 | A test of a machine for a program with no TCP port (a program that ends, a UDP server) in task H37 | `ddclient`, `doplarr_rs`, `foldingathome`, `kometa`, `luanti`, `rsnapshot`, `webgrabplus` |
| host network | 5 | Tier 3: install at the machine and adopt (R-228) | `duckdns`, `fail2ban`, `homeassistant`, `plex`, `wireshark` |
| no config | 4 | Tier 3, or an entry that a person writes: the publisher gives no data | `ffmpeg`, `modmanager`, `qemu-static`, `socket-proxy` |
| capability IPC_LOCK | 4 | Tier 3; or a test that the application works with no keyring | `github-desktop`, `mysql-workbench`, `vscode`, `vscodium` |
| capability NET_ADMIN | 3 | Tier 3 | `mullvad-browser`, `swag`, `wireguard` |
| device | 2 | Tier 3 until a setting of the machine permits a device | `minisatip`, `oscam` |
| shell without credential | 2 | A credential that the install can make, for each image (the user and password of `openssh-server`, the user of the web page of Syncthing) | `openssh-server`, `syncthing` |
| privileged | 1 | Tier 3 | `kasm` |
| engine socket | 1 | Tier 3 until the exception of R-247 (task H34) | `sealskin` |

The ten entries are tier 2: no test on a machine ran (HT-114 is the check by a person).
The device of hardware decoding of Jellyfin is an optional part that is off; the streams of
qBittorrent (6881, TCP and UDP) and of Heimdall and Nextcloud (their second port) wait for the
routes of task H40. The hub lists the generated entries with the others; it does not show the
tier, the class or the optional parts yet, and its install stays refused (task H6).


## 16. The hub as the proxy manager, with streams (R-319, HOST-050)

This section is a design with options. It adds no approved requirement. It widens Section 12 (the
routing of names on the hub) from HTTPS names to the routes that a proxy manager has, streams of
TCP and UDP included. A security read by a separate reviewer reads it; each part that opens a port
of a machine or of the hub to a network waits for that read and for his decisions.

### 16.1 What he asked for

His words: "I used to use nginx proxy manager and I like it as the hub as the proxy manager don't
need to use nginx but I want it to be networked liket hat with support for more then just https".
Thus: one place in the hub that lists each route ("this name, or this port, goes to this
application on this machine"), with its certificate, and with more kinds than HTTPS. The program
that carries the bytes is the proxy of the hub of Section 12 (R-302): not nginx, and
not a proxy on each machine.

### 16.2 What users of such tools expect

From the public documentation of each tool, as the writer of this section knows it (not read
again for this text; the build checks each point):

| Tool | How routes are made | Kinds of route | Certificates | Access |
| --- | --- | --- | --- | --- |
| Nginx Proxy Manager | A screen: "proxy hosts", "redirection hosts", "streams", "404 hosts" | HTTP(S) by name; streams of TCP and UDP by port | Let's Encrypt, with HTTP or DNS challenge | Access lists: addresses, basic sign-in |
| Traefik | Labels on containers, read from the socket of Docker, or files | HTTP by name and path; TCP by SNI or port; UDP by port | ACME, automatic | Middleware: sign-in in front, addresses, limits |
| Caddy | One file, or its API | HTTP(S) by name; TCP and UDP only with a plug-in (layer 4) | Automatic HTTPS, the strongest default | Plug-ins |
| Pangolin | A screen on a central server; a tunnel program on each site | HTTP(S) by name; raw TCP and UDP | ACME on the central server | Sign-in in front of each route (identity-aware), rules |

Pangolin is closest to Anima: one central server, a tunnel from each site, the routes on the
central server. Nginx Proxy Manager is closest to what he names: a screen of routes.

What the hub of Anima takes from them:

- A screen of routes for the hub, with a list of each route of each application: kind, name or
  port, target (application and machine), certificate, access, state, counts.
- The kinds: **web** (HTTPS by name, Section 12), **redirect** (a name to an `https://` address,
  only on a name that the person proved, with the rights of a publish of Section 12.3: a redirect
  on a name that Anima gives would be an open redirect for phishing),
  **TCP stream** (a port of the hub to a port of an application), **UDP stream** (the same for
  UDP). No path rules or custom files of nginx in the first version.
- The access of a route: **public** (the internet), **private** (only his devices on the network
  of Anima, the VPN of [vpn-plan.md](../plans/vpn-plan.md)), and later **sign-in in front** (the
  hub asks for the sign-in of Anima before it passes a request, as Pangolin does).
- A route is made from the page of the application too: an entry of the catalogue says which of its
  ports is web and which is a stream (Section 13, Section 15), so one click proposes the route.
- Not taken: a route from labels on a container (Traefik). It needs the socket of the engine on
  the hub, which R-226 forbids, and it lets a file of a different publisher make its own route.

### 16.3 Where the bytes go

| From where | To a route of kind web | To a stream of TCP | To a stream of UDP |
| --- | --- | --- | --- |
| The internet, hosted hub | The door by SNI (Section 12) | Needs an address of its own for each hub at the door (planned by the cloud service, not built); none before it | The same, and the link must carry datagrams |
| The internet, hub that a person hosts | Ports 80 and 443 of the hub | A port of the hub that he selects | A port of the hub, and a path that carries UDP to the machine |
| His devices on the network of Anima (private) | The address of the hub on that network | The same | The same |
| His local network | A hub that a person hosts on that network serves it as above. A hosted hub does not: the bytes would go to the pool and back. The machine's own port with "Open to the network" (R-229) is the local path, with no proxy | The same | The same |

The path from the hub to the machine is always the path of Section 12.2 point 5: the hub takes
the identifier of an application and connects through its one connect function; on a computer of
the link, the link dials `127.0.0.1:<port>`. Two limits of today:

- **The link carries TCP only** (task HM-T12). A stream of UDP to a computer of the link needs a
  change of the contract of the link (datagrams on the tunnel) and its own security read.
- **The door of a hosted hub has one address for all hubs** and routes by the name in SNI. A
  stream has no name, so a stream on a hosted hub needs an address (or a range of ports) of its own
  for each hub at the door: a question for the cloud plan.

### 16.4 The rules of a stream

The rules of Section 12.3 apply to each route. For a stream also:

- **Port of the hub.** A stream listens on a port of a range that the operator sets for streams,
  and never on 22, 53, 3478, 5432, the ports of the VPN and of WireGuard, or a port that the hub,
  the gateway (`guacd`), the link or the proxy is set to use. Ports 80 and 443 belong to the web
  routes. The hub tests that it can bind the port before it saves the stream. A private stream
  listens on the address of the hub on the network of Anima; a public one on the public address
  that the operator selects.
- **No look inside, so a narrow list.** A stream carries the bytes of the protocol of the
  application, with no TLS of the hub and no check of a name. A warning is not a control: a public
  stream is permitted only for a port that the entry marks `stream-public` (a protocol with its own
  sign-in and TLS), never for a database, MQTT, Redis or an entry of the class `shell`.
  OD-HOST-43 (a consent at the machine) covers streams too. The application sees the address of
  the hub for each client, so its own limits for each address do not work: the hub applies the
  limits for each source.
- **Limits for each stream.** Connections at one time, new connections each second, bytes. For UDP
  also packets each second, a limit of flows for each stream and a time after which an idle flow
  ends. A source address of UDP can be false, so until a source has proven that it gets the
  answers, the hub sends to it at most as many bytes as it received from it. No public UDP stream
  for syslog (false log lines).
- **Who makes it.** As a public web route (Section 12.3): a person with a login or a browser
  session, after a second factor; no API key and no MCP tool; OD-HOST-43 for the consent at the
  machine.
- **Private by default.** A new route is private. He makes it public by a separate step.

### 16.7 What "private" is

- A private route listens only on the address of the hub on the network of Anima, and the proxy
  checks the interface on which a connection came, not only its name. A private name is never in
  the table of the door.
- "Private" means each node of the network of Anima: his devices, and also each machine and each
  container on it that can reach that network. An entry that holds the socket of the engine
  (Section 10) or of the class `shell` (Section 15.7) gets no route (OD-HOST-54).
- A private route is made with the same rights as a public one (Section 12.3).
- Whether a hosted hub is on a network of its own customer, with its own rules, or on a shared one,
  is a question for the cloud plan.
- On a hub that a person hosts, UDP to a machine on its network needs "Open to the network" on
  that machine, which also opens the port to the whole local network.

### 16.5 What is built in which order

1. The table of routes of task H21 gets the kinds web and redirect, and the access public or
   private. (Task H21 builds web routes now.)
2. TCP streams on a hub that a person hosts, private and public.
3. TCP streams on a hosted hub, after the door gives an address for each hub.
4. UDP streams: on a hub that a person hosts that reaches the machine on its network first; through
   the link after the change of the link contract.
5. Sign-in in front of a route, after a design of its own origin (Section 12.3, AC-D45).

### 16.6 Options for the operator

- **OD-HOST-46: where do the bytes go for his local network on a hosted hub?** A: no proxy for the
  local network; the machine's own port with "Open to the network" is the local path, and the
  routes of the hub are for the internet and his devices on the network of Anima. B: a small
  forwarder on each machine for the local network, which changes R-302 ("only on the
  hub"). Recommendation: A. It keeps one proxy, and a hub that he hosts on his local network gives
  him the names there.
- **OD-HOST-47: streams on a hosted hub before the door has an address for each hub?** A: no;
  streams only on a hub that a person hosts until then. B: the door gives each hub a range of
  ports on the shared address. Recommendation: A. A range of ports on one address is hard to keep
  apart and to limit, and it is the next work of the cloud plan anyway.
- **OD-HOST-48: UDP through the link?** A: TCP streams first; UDP after the change of the link
  contract and its security read. B: no UDP through the link; UDP only where the hub reaches the
  machine on its network. Recommendation: A.
- **OD-HOST-49: which access does a route have in the first version?** A: public and private. B:
  also sign-in in front. Recommendation: A, then B after its design.

A separate reviewer read Sections 15 and 16 on 2026-11-20: SF-243 to SF-250 in
[security-findings.md](../security-findings.md). The text above has the corrections.
