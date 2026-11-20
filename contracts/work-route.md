# K10 — Work route: claim, report, log, stop and a secret for one job

Status: **accepted** (R-209, operator, 2026-11-19: "I accept the work route contract K10, version 0.3, as written in `contracts/work-route.md`, and for each open finding of its security review I accept the state that `docs/security-findings.md` gives."; task W1 of [projects-and-builds-plan.md](../docs/plans/projects-and-builds-plan.md); version 0.3, 2026-11-18; version 0.4, 2026-11-19). It changes only by a ruling (CON-001). Version 0.4 adds the container work types (Section 6.4, and the paragraphs "Container types (version 0.4)" in Sections 1 to 9 and 13). It is the second part of R-209: "I approve Section 4 of `docs/proposals/hosted-applications-stage-2.md` as the text of the container work types for a security review. It becomes version 0.4 of the contract when that review has no finding that I have not decided." Three separate reviewers read the text (SF-015 to SF-069 of [security-findings.md](../docs/security-findings.md)); the operator decided each point that needed him (R-226 to R-238, R-246); the third reviewer wrote: "No finding remains that the owner has not decided; the open points are changes of the text." Those changes are in this text; no reviewer of the security review read the text after them. Version 0.4 changes no rule of `build` and `runner`, apart from the sentences that say so. Version 0.3 has the answers of the operator to three findings (R-185: SF-003, a machine with no list of repositories refuses each item; R-186: SF-006, the device list now and the device key later; R-187: SF-013, accepted). Version 0.1 was the text that the security review read; version 0.2 has the changes that the review asked for, and each is marked with its finding (SF-003 to SF-014). R-160 (operator, 2026-11-18) confirmed the work route and its default. The security review of this text is in [security-findings.md](../docs/security-findings.md), entries SF-003 to SF-014; the acceptance closes none of them: each keeps the state that the document gives, and each that is Open stays Open until code exists.

Requirements: USE-001 to USE-009 and JOB-010, JOB-011 of [jobs.md](../docs/proposals/jobs.md); PRO-006, PRO-022 to PRO-024, PRO-029, PRO-030, PRO-034, PRO-039 to PRO-043, PRO-050 of the plan above; MET-013, MET-020, LOG-005, LOG-007, LOG-012 of the design specification. The number K9 is reserved for the link ([s6-spec-delta.md](../docs/plans/one-click/s6-spec-delta.md)).

This contract is the interface by which the hub gives an item of work to `anima-daemon` on a machine. It is a new contract, not a change to K8 ([daemon-push.md](daemon-push.md)), as [jobs.md](../docs/proposals/jobs.md), "Needs", says. K8 has one sentence that the ruling for K10 must change: "The daemon makes no other request." With K10 the daemon makes the requests of K8 and of K10 and no other.

What this contract does not hold: the placement of work (PRO-051), the isolation on the machine (task W2), the files that a build gives back to the hub (task A2; today BLD-060 to BLD-066), the calls to GitHub for a runner (task B2, K-E1), and one contract for results that the model work shares ([jobs.md](../docs/proposals/jobs.md), "Needs").

## 1. Terms

| Term | Meaning |
| --- | --- |
| Item | One unit of work that the hub keeps in its queue (PRO-050). |
| Work type | What an item is. The list is fixed: `build` and `runner` (JOB-011). |
| Kind | How the machine isolates the item: `container`, or `native` on a Mac (PRO-011). |
| Attempt | One try of an item on one machine. It has a number, from 1. |
| Lease | The time until which the hub lets one machine hold an item. |
| Holder | The device that holds the lease of the current attempt. |
| Job | What runs inside the isolation. Its code is not trusted (PRO-029). |

**Container types (version 0.4).** *Terms.* "Work type": "What an item is. The list is fixed: `build`, `runner`
(JOB-011), and the container types." "Kind" gains `host`: "the applications program of the machine
does the steps with the engine of the machine. No job code runs. Only the container types have
this kind."

## 2. Transport

- The transport, the times and the form of a body are those of K8 Section 1: HTTPS to the origin of the hub, or the local port of the link; UTF-8 JSON; camelCase; RFC 3339 UTC. The daemon sends no cookie, follows no redirect, ignores proxy variables and listens on no port (MET-020, USE-005).
- The credential of each route is the daemon token of K8 Section 2, `Authorization: Bearer`. The token proves the device. There is no second token for work.
- The daemon asks. The hub never opens a connection to a machine. A machine gets work only as the answer to its own request.
- A machine whose daemon reaches the hub through the local port of the link (K8 Section 1) takes no item: it sends no claim with the switch on. K8 says that a program of a local user that listens on that port when the link does not would receive the daemon token, and a job is such a program. This rule goes when K8 has its correction, a Unix socket between the daemon and the link. R-172 (operator, 2026-11-18) rules that correction; task W0 builds it (SF-004).
- The hub ignores a member that it does not know. The daemon does not: Section 5 says what a daemon does with an item that it does not fully know.
- A breaking change adds `/work/v2/`. The hub keeps `v1` while one enrolled machine last used it (as ROL-042).

| Route | Body limit | Success |
| --- | --- | --- |
| `POST /work/v1/claim` | 8 KiB | 204, empty; or 200, `Item` |
| `POST /work/v1/report` | 96 KiB | 200, `Reported` |
| `POST /work/v1/secret` | 4 KiB | 200, `Secret` |

The limits on requests in progress of K8 Section 6 apply to these routes: each address, each device, the database. A device has at most one claim in progress.

A hub that does not have this contract answers 404 to each route. The daemon then asks again one time in each hour. A daemon that does not have this contract never asks; the hub shows that machine as "Needs an update" for work (USE-008).

**Container types (version 0.4).** The limit of a request to `POST /work/v1/secret` stays 4 KiB. Its answer is at most
1 MiB, and stays `Cache-Control: no-store`.

## 3. What the machine decides, and what the hub decides

| Subject | Who decides | Rule |
| --- | --- | --- |
| If the machine takes work | The machine | The switch at the machine (USE-004). The hub cannot set it on. A machine whose switch is off sends `"switch": "off"`. If the hub answers such a claim with an `Item`, the daemon drops the answer and starts nothing; the same when the device policy does not permit work. The machine keeps the switch where no job and no other user can write it, and an update of the daemon does not change it (SF-007). (for a container type: the paragraph at the end of this section) |
| If a Mac takes a `native` item | The machine | The setting of PRO-024, made at the Mac. The hub cannot set it on. The rules of the switch apply to it (SF-007). |
| Which repositories the machine works for | The owner of the machine | The machine holds a list of the owners and the repositories of GitHub that it builds. The owner of the machine sets it at the machine; the hub cannot write it, and the rules of the switch apply to it (SF-007). The daemon refuses an item of a repository that is not in the list (`repository_not_permitted`). A machine with no list refuses each item that names a repository. The work route stays on by default (R-160); the list is the control of R-185 (SF-003). |
| Which machines get a secret of a project | The operator | An item that names a secret goes only to a device that the operator named for that project. The placement and the secret route each check it (SF-006). |
| The work types | The daemon program | The fixed list of Section 6. The hub cannot add a type. |
| The steps of an item | The daemon program, or the workflow file | The hub names no program, no command, no argument, no path, no address, no image and no environment variable. |
| The isolation | The machine | The design of task W2. An item cannot ask for less isolation. |
| The limits of an item | Both | The machine uses the lower of the limit that the hub gives and its own limit. |
| Which machine gets an item | The hub | The placement function (PRO-051). |
| When work runs on a machine | The machine | The device policy (USE-009): mains power, quiet hours, an active operator. |

What the fixed list does and does not give (SF-003). The hub names no command. But the hub names the repository and the commit of a `build`, and for a `runner` it gives a JIT configuration that the machine cannot read into its parts with certainty. The code of that repository, or the steps of a workflow, then run on the machine. Thus a person who controls the hub can run code of their choice on each machine that takes work, inside the isolation of task W2 and not outside it. The isolation is the limit, not the list. The operator accepted this cost with OD-W10 and R-160 ([jobs.md](../docs/proposals/jobs.md), "Decision"); the list of the row above is the one control that the owner of a machine has against it, and R-185 (operator, 2026-11-18: "A: allow-list at machine") makes it a condition: no list, no work for a repository.

**Container types (version 0.4).** *The row of the switch.* Added: "This row is for `build` and `runner`. For a
container type the setting is 'Manage applications' and the member of the claim is `applications`."

*The row "Which repositories the machine works for".* Added: "The list does not apply
to a container type, which names no repository. For a container type the controls of the machine
are the settings, the adoption and the rules of a file."

*The row "The steps of an item".* "The daemon program, or the workflow file, or, for
`application_apply`, the Compose file of the application. For `build` and `runner` the hub names
no program, no command, no argument, no path, no address, no image and no environment variable.
For `application_apply` the hub names, through the Compose file, the image, the command, the
health check and the environment of each container."

*A new row.* "If the machine takes container work | The administrator of the machine
| The settings at the machine. The hub cannot set one."

*The paragraph "What the fixed list does and does not give".* Added: "For the
container types there is no isolation. On a machine whose setting 'Manage applications' is on, a
person who controls the hub can: start, stop and restart the containers that the types reach, and
read their logs; start the containers of a Compose file of their choice that passes the rules of a
file, with a command of their choice in each, and with the volumes of that application, so that
they can read and empty its data, and fill the disk with them; make the engine get an image from
each address that such a file names, also an address of the local network; bring an application
back to its earlier version; remove an application with its kept files; remove stopped containers
and images that Section 6.4.10 names; make an adopted project start its files as they are at that
moment, and get its images again. An adopted project has no rule of a file: it runs the file of
the owner (R-232, R-235). The containers of an application can reach each address that the machine
can reach, on the local network and on the network of Anima (R-229). Each setting of the machine
that is on adds what its row says. Each program of the user of the daemon can ask for the same
with no hub (R-236)."

## 4. Claim

The daemon asks for work each 10 seconds while its switch is on and it has a free slot. A machine has one slot (BLD-029); a later version of the daemon can have more. While its switch is off, it sends one claim at its start and then one in each hour, so that the hub knows the state.

`Claim`:

```json
{
  "device": "0192f5c4-…",
  "switch": "on",
  "types": ["build", "runner"],
  "kinds": ["container"],
  "os": "linux",
  "architecture": "x86_64",
  "free": { "slots": 1, "memoryBytes": 17179869184, "diskBytes": 85899345920 },
  "holds": [ { "id": "0192f6aa-…", "attempt": 1 } ],
  "unclean": false
}
```

- `device` must be the device of the token (`WRONG_DEVICE`).
- `types` and `kinds` are what this daemon can do now. `native` is in `kinds` only on a Mac that is set on for native jobs and has its build user (PRO-022, PRO-024).
- `holds` names each item that the machine runs now. When the hub has a lease of this device for an item that `holds` does not name, the answer to the claim was lost: the hub ends that attempt and does not add to the attempts of the item (SF-011).
- `unclean` is true when the last clean-up of this machine did not delete each thing (Section 9). The hub gives no item to such a machine, and shows it (for a container type: the paragraph at the end of this section).
- The hub answers 204 when it has no item for this machine. It always answers 204 to a claim whose `switch` is `off` (for a container type: the paragraph at the end of this section).
- A claim is atomic: two machines never get one attempt (BLD-004). The claim that gets an item starts its lease.
- The first claim of a device with the switch on makes one event in the hub (USE-008).

`Item`:

```json
{
  "id": "0192f6aa-…",
  "type": "runner",
  "kind": "container",
  "attempt": 1,
  "lease": { "until": "2026-11-18T10:01:30Z" },
  "limits": { "seconds": 21600, "idleSeconds": 3600, "memoryBytes": 8589934592, "diskBytes": 42949672960, "orphanSeconds": 600 },
  "project": "0192f000-…",
  "repository": { "forge": "github", "owner": "example", "name": "app" },
  "secrets": ["jit_config"],
  "parameters": { }
}
```

- `id` is a UUID that the hub makes. `attempt` starts at 1.
- `repository` names a repository of a project of this hub (PRO-008, PRO-028). `forge` is from a fixed list in the daemon; today the list is `github`. The daemon makes each address from the forge and the two names with a fixed pattern. `owner` and `name` have the characters that the forge permits and no other; a value that breaks the rule makes the daemon refuse the item.
- `secrets` names the secrets that the item needs, by purpose (Section 8). It never holds a value.
- `parameters` is the object of the work type (Section 6).
- An `Item` never holds a command, an argument, a path, an address, an image name or an environment variable.

**Container types (version 0.4).** *Claim.* - `Claim` gains the member `applications`: `on` or `off`. A claim with no such member is read as
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
  only the two purposes of Section 8. The machine refuses one that breaks this
  (`member_unknown`).

## 5. What the daemon does with an item

Before it starts an item, the daemon checks it. It refuses the item when one of these is true. A refusal is a report with the state `refused` and a fixed cause (Section 7). It is not an attempt (for a container type: the paragraph at the end of this section): the hub gives the item to a different machine and adds nothing to its attempts.

| Cause | When |
| --- | --- |
| `type_unknown` | `type` is not in the list of this daemon (USE-006) |
| `kind_not_permitted` | `kind` is not permitted at this machine (PRO-024) |
| `member_unknown` | The item, its `parameters` or its `limits` has a member that this daemon does not know, or a value that breaks a rule of this contract |
| `secret_unknown` | `secrets` names a purpose that this daemon does not know for this type |
| `switch_off` | The switch went off after the claim |
| `policy` | The device policy does not permit work now (USE-009) |
| `no_isolation` | The machine cannot make the isolation now: no container runtime, no build user |
| `unclean` | The machine is not clean (Section 9) |
| `repository_not_permitted` | The repository of the item is not in the list of the machine, or the machine has no list (Section 3; R-185) |
| `runner_too_old` | `runnerVersion` is lower than the lowest version that this daemon accepts (Section 6.2) |

A daemon refuses a member that it does not know because a newer hub can send a member that makes an item more narrow. A daemon that ignored it would run the item with less limit than the hub asked.

**Container types (version 0.4).** *New causes of refusal.* | Cause | When |
| --- | --- |
| `applications_off` | The item is of a container type and "Manage applications" is off |
| `file_not_permitted` | The Compose file or the environment file breaks a rule of Section 6.4.4. The report names the rule by a fixed code and, for a rule of a service, the position of the service in the file as a number; never text of a file |
| `not_adopted` | The item names a container or a project that Section 6.4.3 does not let it reach |
| `not_at_machine` | The item needs a setting that is off at the machine |

The daemon reports a refusal that it finds before the state `preparing` with the state `refused`,
as for a job. A cause that the machine finds after `preparing`, when it has the two files
(`file_not_permitted`, `program_mismatch`, `limit_disk`), is reported with the state `failed` and
that cause; the diagram of Section 7 does not change. The hub does not put a container item back
in the queue: the item is final, with the state `failed` and the cause as its reason. An
application has one machine, and a container identifier has meaning on one machine. `container_action` with `stop` is never refused for `policy` or for `unclean`.

## 6. Work types

The list is fixed in the daemon program. There is no type that runs a command which the hub names (USE-006, JOB-015).

### 6.1 `build`

A `build` runs a recipe of Anima on one commit of a repository of a project (PRO-031, PRO-032).

```json
{ "recipe": "desktop_release", "commit": "0123456789abcdef0123456789abcdef01234567", "target": "linux-x86_64", "version": "0.3.0", "trigger": "operator", "origin": "default_branch" }
```

- `recipe` is from a fixed list in the daemon. The steps of a recipe are code of the daemon program. Today the list has `desktop_release`.
- `commit` is 40 hexadecimal characters. The job makes a new checkout of that commit (PRO-023, PRO-035). A recipe that publishes checks that the commit is an ancestor of the default branch (BLD-041).
- `target` and `version` have the rules of the release (BLD-002, BLD-003). `trigger` is `operator` or `push`.
- `origin` is `default_branch` or `other`. The hub writes `default_branch` only when it has verified that `commit` is an ancestor of the default branch of the repository. A pull request from a fork, a scratch ref (JOB-020) and each commit that the hub did not verify are `other` (SF-009).
- Secrets that a `build` can name: `repository_token`, `project_secret`. An item whose `origin` is `other` names no `project_secret`, unless the operator set the exception of PRO-030 for the project. The daemon refuses an item that breaks this rule (`member_unknown`). A `build` of a private repository gets the `repository_token` with each `origin`: the token reads that one repository only.
- The files that a build gives back to the hub are not in this contract (task A2).

### 6.2 `runner`

A `runner` runs the runner program of GitHub Actions for one job of a workflow (PRO-027, PRO-039). GitHub selects the job and holds its log and its secrets (PRO-047). The workflow file of the repository names the steps. Thus the hub cannot name a command in a `runner` item, and the machine must treat each `runner` item as code that it does not trust (JOB-011, PRO-029).

```json
{ "runnerVersion": "2.330.0", "runnerSha256": "9f0c…64 hexadecimal characters", "labels": ["self-hosted", "anima", "linux", "x64"] }
```

- `runnerVersion` and `runnerSha256` name the runner program for the operating system and the architecture of this machine (PRO-043). The file comes from the releases of `actions/runner`, at an address that the daemon makes from a fixed pattern and the version. The request is made inside the isolation, with the network of the job; the part of the daemon that speaks to the hub makes no request to GitHub (MET-020, W1-P1). The machine refuses a file whose SHA-256 is different. Each item starts from a clean copy of the program. The daemon holds the lowest version that it accepts and refuses an older one (`runner_too_old`), so that the hub cannot name an old program with a known fault (SF-014).
- If task B1 shows that a JIT configuration can be read into its parts, the daemon reads it before the start and refuses the item when its addresses are not addresses of GitHub from a fixed list, or when it is for a different repository than `repository` (SF-003).
- `labels` are the labels that the hub registered (PRO-041). They are for the log of the machine only: the JIT configuration holds the registration.
- The one secret of a `runner` item is `jit_config` (PRO-040). The hub gives a `runner` item no project secret and no signing key (PRO-030).
- The runner does one job at most. Then the machine deletes the runner, its work folder and its configuration (PRO-039). The machine never starts the same JIT configuration a second time.
- The hub makes the JIT configuration at GitHub when the holder asks for the secret, not before (Section 8). Thus no JIT configuration waits in the database.
- When the attempt of a `runner` item ends, in each final state and when its lease ends, and its `jit_config` was given, the hub removes that registration at GitHub. It does not trust the report that the runner is deleted (SF-008).
- A `runner` item that waits for a job holds its lease by its reports. `limits.idleSeconds` is the longest time that it waits; then the machine stops it and reports `done` with the result `idle`.

### 6.3 Data that is held about the operator (PRO-006)

A job reads the local copy that Anima holds. It does not call the service.

- No purpose in the list of Section 8 is a credential of a service that holds data about the operator (a read-only source of Remember). The hub must never give such a credential to a machine for a job.
- A later work type that needs such data gets it as a named local copy. The item will then name the copy, and the machine will give it to the job to read only. This version has no such member: `build` and `runner` read no data that is held about the operator.
- A job gets no token of Anima: no daemon token, no hub token, no API key. A job has no way to ask the hub for data.

**Container types (version 0.4).** *The first lines.* "The list is fixed in the daemon program and, for the container
types, in the applications program. There is no type that takes a command line from the hub. For
`application_apply` the hub names the command of a container in the Compose file."

### 6.4 Container work types (version 0.4)

The container types change the Docker of a machine. Each has the kind `host`. They are for hosted applications ([hosted-applications-plan.md](../docs/plans/hosted-applications-plan.md), stage 2). Three reads by separate reviewers are in [security-findings.md](../docs/security-findings.md), entries SF-015 to SF-069; the rulings R-226 to R-238 and R-246 decide the points that needed the operator.

#### 6.4.1 The applications program

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
- No request on the socket changes a setting of Section 6.4.2.
- The applications program does not trust the daemon. It checks each member of each request by the
  rules of this text.
- Each program that runs as the user of the daemon can send a request on the socket with no hub.
  It gets what the rules of the machine permit. The rules for actors (Section 6.4.12) are rules of
  the hub and do not limit such a request. The operator accepts this (R-236). When the key of the
  device exists (task W7), the applications program checks a signature of the hub or of the
  operator on each request.

#### 6.4.2 The settings at the machine

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

- There is one profile for a file from the hub, `contained` (Section 6.4.4). There is no profile
  `full` (R-228). A file that needs more is started by the owner at the machine and adopted.
- "The hub may delete volumes", off, stops the removal of a volume. It does not protect the data
  of an application: an install or an update from the hub can run a command with the volume of its
  own application and empty it. The protection of data is a copy (HOST-042). The screen that shows
  the setting says both (R-238).
- The screen that sets "Manage applications" on says which kind of engine the machine has and,
  for an engine of a user, the sentence of Section 6.4.1.
- To set "Manage applications" off does not stop a container. The applications program then shows
  each application that Anima installed and asks if it stops them (SF-030).
- The settings are made for one hub. The daemon sends the origin of its hub and its device
  identifier with each request. The applications program records the first pair in a file of its
  own. When a request has a different pair, it refuses each container item (`applications_off`)
  until the administrator sets "Manage applications" on again. This protects an owner who moves a
  machine to a different hub and forgets the settings. It does not limit a program of the user of
  the daemon, which can send the old pair (R-236). The key of the device (task W7) is the
  correction (SF-039).

#### 6.4.3 Which containers and projects a type reaches

- A container is reached only when the applications program finds, from its own records and the
  project label of the container, that it is of an application that Anima installed or of an
  adopted project. With "All containers" on, each container is reached. Each other container is
  refused (`not_adopted`).
- Each type refuses an item that names, or would stop, start, make again or remove, a container or
  a project of the list "Protected". The list holds names of containers and names of projects.
- A project whose name starts with `anima-` cannot be adopted.

#### 6.4.4 What a Compose file from the hub may have

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
| `volumes` of a service | Only the long form: `type`, `source`, `target`, `read_only`. `source` is always present. The type `volume` with a volume of this file; or the type `bind` whose source is exactly `binds/<name>` of the application (Section 6.4.5). No other type and no other member. A `target` is an absolute path with no `..`; it is not `/`; two mounts of one service have different targets |
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

#### 6.4.5 Where the files are

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

#### 6.4.6 `container_action` and `application_apply`

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
  is different (`program_mismatch`). The rules of Section 6.4.4 are applied at each apply.
- `pull` true gets the image of each service again. `pull` false uses the digests of the last
  apply of this application, and is refused for a service that has none. A pull uses no login,
  unless the owner named the registry at the machine.
- The start removes the containers of the project that the new file no longer names.
- `keep` is 0 or 1: with 1, the machine keeps the Compose file as it came, the environment file and
  the image digests of the version before this apply.
- The result has `health`: `healthy`, `unhealthy`, `no_check` or `not_waited`.
- A recipe of the catalogue of Anima names each image by a digest (R-233). A file of the operator
  can name a tag.

#### 6.4.7 `adopted_apply`

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
  the environment and the configuration folder of Section 6.4.1. Compose reads the `.env` file of
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
- The rules of Section 6.4.4 do not apply to an adopted project. The hub can make it get and start
  new images, and for a project that holds the socket of the engine the publisher of such an image
  then controls the machine. The operator accepts this (R-232).
- The screen and the command show the sentences above again before each `adopted_apply` with `up`.

#### 6.4.8 `application_remove`

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

#### 6.4.9 `application_back`

```json
{ "application": "0192f6b0-7c1e-7a3b-9d2f-4e5a6b7c8d9e" }
```

- It brings the application back to the kept version. The applications program makes the final
  form again from the kept Compose file and the kept environment file, by the six steps of
  Section 6.4.4 of today, with the kept image digests and no pull. It sets no tag on an image.
- It goes back one step. A second `application_back` is refused until the next
  `application_apply`.
- It does not bring back data; the result says so with `data_not_restored` when the project has a
  volume.

#### 6.4.10 `docker_prune`

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

#### 6.4.11 `container_follow`

```json
{ "container": "4f5c1d2e3a4b…64 hexadecimal characters", "tail": 200 }
```

- It sends the lines of the log of one container that Section 6.4.3 lets it reach. `tail` is 0 to
  1,000.
- The lines are the log of the item, with the rules of Section 7. The hub keeps them only
  while the item runs.
- No setting of the machine is for logs: a person who controls the hub, and a program of the user
  of the daemon, can read the logs of the containers that Section 6.4.3 permits.

#### 6.4.12 Which actor may ask for each type

The hub applies this table (R-234). The machine cannot see the actor. The hub makes no container item with no actor in this version: each item is the act of a person, an API key or an MCP tool (ETH-004). An update that the hub starts by itself (R-242 of [extensions-plan.md](../docs/plans/extensions-plan.md), for two stacks of that plan) is not in this version; it needs its own row here, with a ruling.

| Type | A login or a browser session | An API key or an MCP tool |
| --- | --- | --- |
| `container_action` | Yes | Yes |
| `application_back` | Yes | Yes |
| `application_apply` | Yes | Only the first install of an entry of the tested catalogue of Anima. The hub makes the file from the recipe. The key or the tool gives no Compose file, no environment file and no value of a variable, apart from a value that the recipe names as a choice with a fixed list. Not an update |
| `application_remove` | Yes; with `volumes` true only with the setting at the machine | No |
| `adopted_apply` | Yes | No. A key or a tool starts, stops and restarts one container with `container_action` |
| `docker_prune` | Yes | No |
| `container_follow` | Yes | No. No MCP tool returns a log (MCP-004) |

#### 6.4.13 What is not a container type

- **The processor and memory use of each container (HOST-018).** A change of K8, not a work type.
- **The terminal in a container (HOST-024).** A session of the hub, after its own review (task H12).
- **A scheduled command in a container.** Not proposed (USE-006, JOB-015).
- **A change of the file of an adopted project.** Not in this version (R-211).

## 7. Report, log and stop

The holder reports at least each 15 seconds while it holds an item, and at once at each change of state (BLD-005, BLD-010).

`Report`:

```json
{
  "id": "0192f6aa-…",
  "attempt": 1,
  "state": "running",
  "log": { "from": 20480, "text": "…at most 64 KiB…" },
  "result": null
}
```

`Reported`:

```json
{ "lease": { "until": "2026-11-18T10:03:00Z" }, "stop": null }
```

States, and the changes that the hub permits:

```text
claimed ─► preparing ─► running ─► finishing ─► done
   │           │           │           │
   │           └───────────┴───────────┴──► failed
   │           └───────────┴───────────┴──► stopped
   └──► refused
```

- `preparing`: the machine makes the isolation and gets the program or the checkout. `running`: the job runs. `finishing`: the job ended and the machine deletes each thing. `done`, `failed`, `stopped` and `refused` are final.
- Each report extends the lease to 90 seconds from the report (BLD-005). The hub refuses a report of a device that is not the holder, of a different attempt, of a final item, and of a change that the diagram does not permit: `409 NOT_HOLDER`.
- `result` is present exactly with a final state. Its members are fixed codes and numbers, never text from the job:

```json
{ "outcome": "succeeded", "cause": null, "seconds": 412, "cleanup": { "isolation": "deleted", "checkout": "deleted", "secrets": "deleted", "keychain": "none" } }
```

  `outcome` is `succeeded`, `failed`, `idle`, `stopped` or `refused`. `cause` is a code of Section 5, or one of `step_failed`, `limit_seconds`, `limit_memory`, `limit_disk`, `program_mismatch` (the SHA-256 of the runner program), `secret_failed`, `isolation_failed`, `orphan`. The hub reads a cause that it does not know as `failed`. Each member of `cleanup` is `deleted`, `failed` or `none`.

Log:

- `log.text` is the next part of the log of the item, and `log.from` is the count of bytes before it. The hub drops a part whose `from` is not the end of what it holds, and answers with success; the daemon then sends from the end that the next `Reported` gives. (`Reported` has the optional member `logEnd` for this.)
- For a `build`, the log is the output of the steps of the recipe (BLD-048). For a `runner`, the log holds only the lines of the daemon: prepare, start, the end code of the runner program, clean-up. GitHub holds the log of the workflow (PRO-047).
- The daemon removes from each line the daemon token, each secret of the item and each value that matches the patterns of the redaction crate, before it sends the line. The hub applies its redaction again before it stores a line (BLD-049, LOG-007). The hub keeps at most the last 2 MiB of the log of an item.
- A log line is text from a job. It is data, never an instruction: each reader gives it in a named field with a length limit (MCP-008). The daemon, and then the hub, remove each control character other than the line end and the tab, and each character that changes the direction of text; they cut a line at 4,096 bytes; and a part never ends inside one character. An MCP tool gives a log only with the mark of text that is not trusted (SF-010).
- The removal of secrets from a log is a best effort. A job that has a secret can write it in a form that no pattern finds. The control is that a job of code that is not trusted gets no secret (Section 6.1, `origin`), not the redaction (SF-010).

Stop:

- The hub tells a stop only in `Reported`: `"stop": { "reason": "operator" }`. The reasons are `operator`, `runners_off` (PRO-046), `update` (a rollout needs the machine, ROL-028), `superseded` and `expired`. Thus a stop reaches a machine in at most 15 seconds.
- The machine then ends the job: it asks the job to end, waits at most 10 seconds, ends each process of the isolation, deletes each thing (Section 9), and reports `stopped` with the result.
- `409 NOT_HOLDER` to a report is also a stop, with no later report: the machine ends the job and deletes each thing.
- When the machine cannot reach the hub (SF-005): a `build` ends when the time of the lease, 90 seconds, has passed on the clock of the machine with no report that the hub accepted. The clock is one that does not go back. The hub gives the item to a second machine only after that time, so two machines never run one `build`. A `runner` continues: the hub makes no second attempt of a `runner` item, and GitHub gave the job to this runner only. It ends when the hub was not reached for `limits.orphanSeconds`. In each case the daemon keeps at most 2 MiB of log, deletes each thing at the end, and reports `failed` with the cause `orphan` when the hub answers again.
- `401 TOKEN_REFUSED`, `403 WRONG_DEVICE` and `403 WORK_OFF` to a report end the job at once, for each type: the machine is no longer permitted to hold it (SF-005).
- The hub ends an attempt that is not final when `limits.seconds` and 5 minutes have passed since its claim, with the stop reason `expired`. An attempt can thus not hold an item with no end by its reports (SF-011).

The lease:

- When a lease ends with no report, the hub ends the attempt (BLD-006) (for a container type: the paragraph at the end of this section). For a `build`, the item goes back to the queue with its attempts plus 1; after 3 attempts it is `failed` with the reason "machine stopped 3 times". The placement prefers a different machine (USE-007).
- For a `runner`, the hub removes the registration of that runner at GitHub and makes no second attempt of the item: a JIT configuration is for one use. Warm mode makes a new item (PRO-042).
- A result counts only from the holder of the current attempt.

**Container types (version 0.4).** *Log.* "For a container type other than `container_follow`, the log of the item holds
only lines of the applications program: the step, a fixed code, and the digest of each image. It
holds no output of the Docker program and no text of a file."

*The end of a container item.* - The hub makes no second attempt of a container item. The operator asks again.
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
  reports when the hub answers again. The hub can have ended the attempt in that time
  (`limits.seconds` and 5 minutes); the report then gets `409 NOT_HOLDER`, and the machine writes
  the end to its own log only.
- `WORK_OFF` has two causes (Section 11). The switch of the operator in the hub: the hub gives the
  machine no job and no container item other than `container_action` with `stop`. The hour after 5
  refused or lost items: the hub gives the machine no job for that hour and gives container items
  as usual; the count of 5 counts jobs only. In each case the hub does not answer `403 WORK_OFF`
  to a claim whose `applications` is `on`: it answers 204, or an item that this rule permits. The
  hub does not answer `WORK_OFF` to a report of a container item. A machine that gets
  `403 WORK_OFF` to such a report from an older hub ends the item as for `409 NOT_HOLDER`.
- `result` of an item of the kind `host` has the four members of Section 7 and, for some types,
  one member more. `outcome`, `cause` and `seconds` are as for a job. Each of the four members of
  `cleanup` is `none`. A hub keeps a member of `result` as the machine sent it.

  `application_apply` and `adopted_apply` add `health`, one of `healthy`, `unhealthy`, `no_check`,
  `not_waited`:

  ```json
  { "outcome": "succeeded", "cause": null, "seconds": 48, "cleanup": { "isolation": "none", "checkout": "none", "secrets": "none", "keychain": "none" }, "health": "healthy" }
  ```

  A refusal of a file adds `rule`, a fixed code of Section 6.4.4, and `service`, the position of
  the service in the file as a whole number from 0, or null for a rule of the top level:

  ```json
  { "outcome": "failed", "cause": "file_not_permitted", "seconds": 1, "cleanup": { "isolation": "none", "checkout": "none", "secrets": "none", "keychain": "none" }, "rule": "key_not_permitted", "service": 2 }
  ```

  The codes of `rule` are: `key_not_permitted`, `value_not_permitted`, `variable_not_permitted`,
  `name_not_permitted`, `too_large`, `yaml_not_permitted`, `image_not_permitted`,
  `port_not_permitted`, `mount_not_permitted`, `label_not_permitted`, `limit_not_permitted`.

  `application_back` adds `health` and `dataNotRestored`, a truth value:

  ```json
  { "outcome": "succeeded", "cause": null, "seconds": 20, "cleanup": { "isolation": "none", "checkout": "none", "secrets": "none", "keychain": "none" }, "health": "no_check", "dataNotRestored": true }
  ```

  `application_remove` and `docker_prune` add `removed`: for each of `containers`, `images`,
  `networks` and `volumes` that the item named, the count and the bytes that the engine says it
  freed, each a whole number:

  ```json
  { "outcome": "succeeded", "cause": null, "seconds": 3, "cleanup": { "isolation": "none", "checkout": "none", "secrets": "none", "keychain": "none" }, "removed": { "containers": { "count": 2, "bytes": 0 }, "images": { "count": 1, "bytes": 470400000 } } }
  ```

  `container_action` and `container_follow` add no member.
- `limits` of a container item has the five members of Section 4, so that a daemon that knows
  the form does not refuse it. `seconds` has the meaning and the maximum of this paragraph.
  `idleSeconds`, `memoryBytes`, `diskBytes` and `orphanSeconds` have no meaning for the kind `host`:
  the hub sends 0 for each, and the machine ignores them. The placement does not compare
  `free.memoryBytes` and `free.diskBytes` with them for the kind `host`.

## 8. A secret for one job

A secret reaches a machine only through this route, for one item, to its holder (JOB-010, PRO-012).

Request:

```json
{ "id": "0192f6aa-…", "attempt": 1, "purpose": "jit_config" }
```

`Secret`:

```json
{ "value": "…", "expiresAt": "2026-11-18T11:00:00Z" }
```

| Purpose | Work type | What it is | Life |
| --- | --- | --- | --- |
| `jit_config` | `runner` | The JIT configuration of one runner (PRO-040) | One use |
| `repository_token` | `build` | A token of the forge for that one repository, with read access to its content only (PRO-034, fact F26) | 1 hour at most |
| `project_secret` | `build` | One secret of the project that the recipe names (PRO-012, JOB-010). Only for an item whose `origin` is `default_branch`, or with the exception of PRO-030 (SF-009) | The item |

- The purposes are a fixed list. The signing identity of a Mac for one job is not in the list of this version: task W2 designs it, and it is added with a ruling.
- The hub gives a secret only when each of these is true: the device is the holder; the attempt is the current one; the state is `preparing` or `running`; `secrets` of the item names the purpose; the operator named the device for the project of the item (Section 3, SF-006); and the secret was not given for this attempt. Each other request is `409 SECRET_REFUSED`, with no word on which test failed.
- One purpose is given one time for one attempt. If the answer is lost, the attempt fails with the cause `secret_failed`, and a new attempt gets a new secret.
- When an attempt ends, the hub makes each secret of it useless where it can: it removes the registration of a `jit_config` (Section 6.2) and revokes the `repository_token` at the forge (SF-008).
- The hub writes each release of a secret to the audit log: the item, the project, the device, the purpose and the time, never the value (JOB-010, LOG-005).
- The hub keeps no secret of this route in the database as plain text and in no log (PRO-040, LOG-007). The answer has `Cache-Control: no-store`. The Rust types that hold a value implement neither `Debug`, `Display` nor `Clone`, and wipe on drop (as K8 Section 2).
- The machine asks for a secret only after the isolation exists, directly before the step that needs it. It gives the value to that step only. It writes the value to no file outside the isolation, to no log and to no command line that a different user of the machine can read. It deletes the value when the step has it, and each copy inside the isolation goes with the isolation.
- An MCP tool, a command and a query never return a value of this route (MCP-004).

**Container types (version 0.4).** *Two new purposes.* | Purpose | Work type | What it is | Life |
| --- | --- | --- | --- |
| `compose_file` | `application_apply` | The Compose file of the application, at most 256 KiB as the hub holds it | The application |
| `environment_file` | `application_apply` | The environment file of the application, at most 64 KiB: the one source of the variables of the Compose file, and a secret of the operator (HOST-022) | The application |

"Each test of this section applies to the two purposes: the holder, the current attempt, the
state, the purpose that the item names, and one time for one attempt. The rule 'the operator
named the device for the project' is, for them, 'the device is the one machine of the
application'. The rules 'no file outside the isolation' and 'deletes the value when the step has
it' do not apply to them: the applications program keeps the two files in the folder of the
application (Section 6.4.5) for the life of the application, and `application_remove` deletes them.
The rule 'asks for a secret only after the isolation exists' does not apply to the kind `host`.
The rule that no MCP tool, command or query returns a value of this route applies to
`environment_file` only: the editor shows the Compose file. Until the key of the device exists
(task W7), a person with the daemon token of a machine can claim its items and get its environment
file, on each machine, also on one that the operator does not own alone; the operator accepts this
(R-231). The values of an
environment file can also be read by each user of the machine who can use the engine."

## 9. Clean-up

- When an item ends, in each final state, the machine deletes the isolation, the checkout, the work folder, the runner program copy and its configuration, the secrets and, on a Mac, the keychain of the job (PRO-023, PRO-029, PRO-039).
- The machine does the same at the start of the daemon for each item that a stop of the daemon or of the machine left.
- The result says what was deleted (`cleanup`). If one thing cannot be deleted, the machine sets itself `unclean`, claims no item, and says so in each claim. Only a clean-up that succeeds, or the operator at the machine, sets it back.
- The machine keeps its own log of the work that it took. It writes the entry of an item before the state `preparing`, and it also writes each item that it refused: the id, the type, the kind, the project, the repository, the commit and the recipe or the version and the SHA-256 of the runner program, the purposes of the secrets, the times and the outcome. It holds no secret. The log is outside the isolation; a job and the build user cannot write it, and the hub cannot delete it (SF-012).
- A job that makes the clean-up fail takes its machine out of work until the owner acts. With a warm runner on a public repository, one pull request from a fork can do that for each machine. The operator accepts this (R-187, 2026-11-18: "A: accept"): a machine that is not clean must not take the next job. The controls are the isolation of task W2 and the approval rule of GitHub (F18) (SF-013).

**Container types (version 0.4).** "The machine deletes each secret at the end of an item, except the two purposes of
a container type (Section 8). An item of the kind `host` has no isolation to delete and never
makes a machine `unclean`. For a container type the entry of the log of the machine has: the type;
the container identifier and the action; the application or the project; `fileSha256`,
`environmentSha256` and the SHA-256 of the final form; `pull`, `keep`, `volumes`, `images`, `what`;
the code of a refusal; the digest of each image that was started; the counts of what was removed.
For `container_follow`: the container and the time, never a line. On a system engine the
applications program writes this log where the daemon and the hub cannot change it. On an engine
of a user the log is a file of that user: a program of that user, also the daemon, can change it."

## 10. What the hub records

- Each item, each attempt with its device, each stop with its reason and each final state is in the audit log (LOG-005) with the actor kind: `User` for an act of the operator, `Hub` for the hub, `Agent` for a call through an API key (LOG-012).
- Each release of a secret is in the audit log (Section 8).
- Each refusal of a route is in the event log with its code and the device, never with the token, at most once a minute for each device (as K8 Section 6).

## 11. Refusals

The body of each refusal is `{ "error": "<CODE>" }` and nothing else.

| Status | Code | When | Daemon does |
| --- | --- | --- | --- |
| 400 | `MALFORMED` | The body is not valid by this contract | Logs the code; for a report, sends the state again without the log part |
| 401 | `TOKEN_REFUSED` | As K8 Section 6 | As K8. To a report: ends the job at once (Section 7) |
| 403 | `WRONG_DEVICE` | `device` is not the device of the token | As `TOKEN_REFUSED` |
| 403 | `WORK_OFF` | The operator set this device off for work in the hub (USE-004), or the device refused or lost 5 items in 1 hour and the hub gives it none for 1 hour (SF-011). To a claim whose `applications` is `on` the hub answers 204 or an item in place of this code (Section 7) | Ends each item that it holds; claims again in 1 hour |
| 409 | `NOT_HOLDER` | Section 7 | Ends the job, deletes each thing, sends no later report of that attempt |
| 409 | `SECRET_REFUSED` | Section 8 | Fails the attempt with `secret_failed` |
| 413 | `TOO_LARGE` | The body exceeds its limit | Sends a smaller log part |
| 503 | `UNAVAILABLE` | The hub cannot store now, or is busy | Tries again with the waits of K8 Section 7 |

## 12. With the update of machines (K8 Section 8)

- A machine that holds an item sends `busy: true` in the `update` member of K8 (ROL-028). For the container types (version 0.4): a machine is busy while it holds an item that changes Docker, and not for a `container_follow` item alone. The hub stops each `container_follow` item of a machine before the step of a rollout that includes it (`stop.reason` is `update`), as for a waiting `runner`, and gives that machine no container item other than `container_action` with `stop` until its update ends.
- A `runner` item that waits for a job would keep a machine busy with no end. Thus the hub stops each waiting `runner` item of a machine before the step of a rollout that includes it (`stop.reason` is `update`), and gives that machine no item until its update ends.

## 13. Open points of this text

| Point | Question | Proposal |
| --- | --- | --- |
| W1-P1 | The runner program and the checkout come from GitHub. MET-020 says that the daemon connects only to the hub | The job gets them inside the isolation, with the network of the job. The part of the daemon that speaks to the hub makes no request to GitHub. Task W2 must show it |
| W1-P2 | How the runner program takes the JIT configuration with no command line that a different user can read | Task B1 must prove one way (a file inside the isolation, or the environment of the one process) |
| W1-P3 | One contract for results that the model work shares ([jobs.md](../docs/proposals/jobs.md), "Needs") | Not in this version. `result` has only fixed codes, so a later contract can add members |
| W1-P4 | A work token that is not the daemon token | Not proposed: one token proves one device. The security review (SF-006) proposes a different thing: a key of the device that signs each claim and each request for a secret, so that a stolen token alone gets no secret. Not in this version. The operator decided (R-186, 2026-11-18: "A: device list, key later"): the device list now; the key is task W7 of the plan, before a machine that he does not own alone takes work, and it will change this contract with a ruling |
| W1-P5 | The work types of the model service (OD-C9, R-165) | Each is one more type of the fixed list, added with a ruling. Not in this version |

**Container types (version 0.4).** Point W1-P6 is decided (R-226, R-246): the applications program.

## 14. Acceptance

The operator accepted this contract, version 0.3, on 2026-11-19 (R-209; CON-001). The sentence that he selected:

> I accept the work route contract K10, version 0.3, as written in `contracts/work-route.md`, and for each open finding of its security review I accept the state that `docs/security-findings.md` gives.

Before the ruling this section said what comes after it; that is done or stands as follows. The status line of this file is changed. The table "Contracts" of [rulings.md](../docs/rulings.md) is not changed (it is the state of Phase 1). K8 has its one changed sentence. The proposal said "tasks W3 and W4 can start when task W2 is also accepted": the chain of [projects-and-builds-plan.md](../docs/plans/projects-and-builds-plan.md) asks that only of task W4, which runs the job on the machine; task W3, the queue of the hub, needs this contract and no isolation, and it starts now. The rule of Section 2 for a machine whose daemon reaches the hub through the local port of the link said that it goes when K8 has its correction: K8 has it since task W0 (R-172; the check is HT-037), and a daemon sends no request to that port, so the rule applies to no machine. A change of this text needs a ruling and raises the version.

Version 0.4 (2026-11-19) is the second part of the same ruling, R-209, with the condition that it gives; the status line says how the condition was met. The rulings that decide its points are R-226 to R-238 and R-246. Two of them change an earlier ruling: R-228 changes R-210 (no profile `full`), and R-246 changes R-226 for a machine whose Docker belongs to a user. No code of a container work type exists; tasks H28, H31 and H32 of [hosted-applications-plan.md](../docs/plans/hosted-applications-plan.md) build it, after the queue of task W3 and the daemon part of task W4.
