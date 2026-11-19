# K10 — Work route: claim, report, log, stop and a secret for one job

Status: **proposed, not accepted** (task W1 of [projects-and-builds-plan.md](../docs/plans/projects-and-builds-plan.md); version 0.2, 2026-11-18). Version 0.1 was the text that the security review read; version 0.2 has the changes that the review asked for, and each is marked with its finding (SF-003 to SF-014). No ruling accepts this text. Build nothing from it (PLN-007, CON-001). R-160 (operator, 2026-11-18) confirmed the work route and its default; it approved no requirement one by one and no contract. The security review of this text is in [security-findings.md](../docs/security-findings.md), entries SF-003 to SF-014; each is open until the operator closes it, and four of them name a risk that only the operator can accept (SF-003, SF-004, SF-006, SF-013). The sentence with which the operator accepts the contract is at the end, in Section 14.

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

## 2. Transport

- The transport, the times and the form of a body are those of K8 Section 1: HTTPS to the origin of the hub, or the local port of the link; UTF-8 JSON; camelCase; RFC 3339 UTC. The daemon sends no cookie, follows no redirect, ignores proxy variables and listens on no port (MET-020, USE-005).
- The credential of each route is the daemon token of K8 Section 2, `Authorization: Bearer`. The token proves the device. There is no second token for work.
- The daemon asks. The hub never opens a connection to a machine. A machine gets work only as the answer to its own request.
- A machine whose daemon reaches the hub through the local port of the link (K8 Section 1) takes no item: it sends no claim with the switch on. K8 says that a program of a local user that listens on that port when the link does not would receive the daemon token, and a job is such a program. This rule goes when K8 has its correction, a Unix socket between the daemon and the link, with a ruling (SF-004).
- The hub ignores a member that it does not know. The daemon does not: Section 5 says what a daemon does with an item that it does not fully know.
- A breaking change adds `/work/v2/`. The hub keeps `v1` while one enrolled machine last used it (as ROL-042).

| Route | Body limit | Success |
| --- | --- | --- |
| `POST /work/v1/claim` | 8 KiB | 204, empty; or 200, `Item` |
| `POST /work/v1/report` | 96 KiB | 200, `Reported` |
| `POST /work/v1/secret` | 4 KiB | 200, `Secret` |

The limits on requests in progress of K8 Section 6 apply to these routes: each address, each device, the database. A device has at most one claim in progress.

A hub that does not have this contract answers 404 to each route. The daemon then asks again one time in each hour. A daemon that does not have this contract never asks; the hub shows that machine as "Needs an update" for work (USE-008).

## 3. What the machine decides, and what the hub decides

| Subject | Who decides | Rule |
| --- | --- | --- |
| If the machine takes work | The machine | The switch at the machine (USE-004). The hub cannot set it on. A machine whose switch is off sends `"switch": "off"`. If the hub answers such a claim with an `Item`, the daemon drops the answer and starts nothing; the same when the device policy does not permit work. The machine keeps the switch where no job and no other user can write it, and an update of the daemon does not change it (SF-007). |
| If a Mac takes a `native` item | The machine | The setting of PRO-024, made at the Mac. The hub cannot set it on. The rules of the switch apply to it (SF-007). |
| Which repositories the machine works for | The machine, if its owner sets it | The machine can hold a list of owners and repositories. With a list, the daemon refuses an item of a different repository (`repository_not_permitted`). With no list, each repository of the hub is permitted: that is the default of R-160 (SF-003). |
| Which machines get a secret of a project | The operator | An item that names a secret goes only to a device that the operator named for that project. The placement and the secret route each check it (SF-006). |
| The work types | The daemon program | The fixed list of Section 6. The hub cannot add a type. |
| The steps of an item | The daemon program, or the workflow file | The hub names no program, no command, no argument, no path, no address, no image and no environment variable. |
| The isolation | The machine | The design of task W2. An item cannot ask for less isolation. |
| The limits of an item | Both | The machine uses the lower of the limit that the hub gives and its own limit. |
| Which machine gets an item | The hub | The placement function (PRO-051). |
| When work runs on a machine | The machine | The device policy (USE-009): mains power, quiet hours, an active operator. |

What the fixed list does and does not give (SF-003). The hub names no command. But the hub names the repository and the commit of a `build`, and for a `runner` it gives a JIT configuration that the machine cannot read into its parts with certainty. The code of that repository, or the steps of a workflow, then run on the machine. Thus a person who controls the hub can run code of their choice on each machine that takes work, inside the isolation of task W2 and not outside it. The isolation is the limit, not the list. The operator accepted this cost with OD-W10 and R-160 ([jobs.md](../docs/proposals/jobs.md), "Decision"); the list of the row above is the one control that the owner of a machine has against it.

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
- `unclean` is true when the last clean-up of this machine did not delete each thing (Section 9). The hub gives no item to such a machine, and shows it.
- The hub answers 204 when it has no item for this machine. It always answers 204 to a claim whose `switch` is `off`.
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

## 5. What the daemon does with an item

Before it starts an item, the daemon checks it. It refuses the item when one of these is true. A refusal is a report with the state `refused` and a fixed cause (Section 7). It is not an attempt: the hub gives the item to a different machine and adds nothing to its attempts.

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
| `repository_not_permitted` | The machine has a list of repositories, and the repository of the item is not in it (Section 3) |
| `runner_too_old` | `runnerVersion` is lower than the lowest version that this daemon accepts (Section 6.2) |

A daemon refuses a member that it does not know because a newer hub can send a member that makes an item more narrow. A daemon that ignored it would run the item with less limit than the hub asked.

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

- When a lease ends with no report, the hub ends the attempt (BLD-006). For a `build`, the item goes back to the queue with its attempts plus 1; after 3 attempts it is `failed` with the reason "machine stopped 3 times". The placement prefers a different machine (USE-007).
- For a `runner`, the hub removes the registration of that runner at GitHub and makes no second attempt of the item: a JIT configuration is for one use. Warm mode makes a new item (PRO-042).
- A result counts only from the holder of the current attempt.

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

## 9. Clean-up

- When an item ends, in each final state, the machine deletes the isolation, the checkout, the work folder, the runner program copy and its configuration, the secrets and, on a Mac, the keychain of the job (PRO-023, PRO-029, PRO-039).
- The machine does the same at the start of the daemon for each item that a stop of the daemon or of the machine left.
- The result says what was deleted (`cleanup`). If one thing cannot be deleted, the machine sets itself `unclean`, claims no item, and says so in each claim. Only a clean-up that succeeds, or the operator at the machine, sets it back.
- The machine keeps its own log of the work that it took. It writes the entry of an item before the state `preparing`, and it also writes each item that it refused: the id, the type, the kind, the project, the repository, the commit and the recipe or the version and the SHA-256 of the runner program, the purposes of the secrets, the times and the outcome. It holds no secret. The log is outside the isolation; a job and the build user cannot write it, and the hub cannot delete it (SF-012).
- A job that makes the clean-up fail takes its machine out of work until the owner acts. With a warm runner on a public repository, one pull request from a fork can do that for each machine. This is accepted in this version: a machine that is not clean must not take the next job. The controls are the isolation of task W2 and the approval rule of GitHub (F18) (SF-013).

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
| 403 | `WORK_OFF` | The operator set this device off for work in the hub (USE-004), or the device refused or lost 5 items in 1 hour and the hub gives it none for 1 hour (SF-011) | Ends each item that it holds; claims again in 1 hour |
| 409 | `NOT_HOLDER` | Section 7 | Ends the job, deletes each thing, sends no later report of that attempt |
| 409 | `SECRET_REFUSED` | Section 8 | Fails the attempt with `secret_failed` |
| 413 | `TOO_LARGE` | The body exceeds its limit | Sends a smaller log part |
| 503 | `UNAVAILABLE` | The hub cannot store now, or is busy | Tries again with the waits of K8 Section 7 |

## 12. With the update of machines (K8 Section 8)

- A machine that holds an item sends `busy: true` in the `update` member of K8 (ROL-028).
- A `runner` item that waits for a job would keep a machine busy with no end. Thus the hub stops each waiting `runner` item of a machine before the step of a rollout that includes it (`stop.reason` is `update`), and gives that machine no item until its update ends.

## 13. Open points of this text

| Point | Question | Proposal |
| --- | --- | --- |
| W1-P1 | The runner program and the checkout come from GitHub. MET-020 says that the daemon connects only to the hub | The job gets them inside the isolation, with the network of the job. The part of the daemon that speaks to the hub makes no request to GitHub. Task W2 must show it |
| W1-P2 | How the runner program takes the JIT configuration with no command line that a different user can read | Task B1 must prove one way (a file inside the isolation, or the environment of the one process) |
| W1-P3 | One contract for results that the model work shares ([jobs.md](../docs/proposals/jobs.md), "Needs") | Not in this version. `result` has only fixed codes, so a later contract can add members |
| W1-P4 | A work token that is not the daemon token | Not proposed: one token proves one device. The security review (SF-006) proposes a different thing: a key of the device that signs each claim and each request for a secret, so that a stolen token alone gets no secret. Not in this version; the operator decides with SF-006 |
| W1-P5 | The work types of the model service (OD-C9, R-165) | Each is one more type of the fixed list, added with a ruling. Not in this version |

## 14. Acceptance

This contract is a proposal until the operator accepts it with a ruling (CON-001). The sentence:

> I accept the work route contract K10, version 0.2, as written in `contracts/work-route.md`, and for each open finding of its security review I accept the state that `docs/security-findings.md` gives.

If the operator wants a finding corrected first, the correction changes this text, raises its version, and the sentence then names the new version. After the ruling: the status line of this file changes, the table "Contracts" of [rulings.md](../docs/rulings.md) is not changed (it is the state of Phase 1), K8 gets its one changed sentence, and tasks W3 and W4 can start when task W2 is also accepted.
