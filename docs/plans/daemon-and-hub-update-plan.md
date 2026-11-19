---
kind: Plan
state: In progress
evidence: Approved by R-139 (operator, 2026-11-17): the update of machines, stages A and B. Stage C (the hub of a server) is deferred and its requirements are marked "Not approved". Version 2 (2026-11-17). Tasks U1 (R-122) and U4 are Done. The other tasks are Not started. OD-ROL-15 is decided (R-158): a release server signs. Version 2.1 (2026-11-18) adds OD-ROL-16: the method of the update of a hosted hub. Version 2.2 (2026-11-18): OD-ROL-16 is decided (R-189: a volume, a copy, a replacement); ROL-053, ROL-054 and task U13.
---

# Plan: update the daemon and the hub on each machine (one press, and automatic)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version: 2.2. Date: 2026-11-18. Version 2.2 records R-189 (OD-ROL-16 is decided: option B with three conditions), adds ROL-053 and ROL-054 and task U13, and says what the pool plan must take. Version 2.1 adds the section "The update of a hosted hub" and the open decision OD-ROL-16; it changes no requirement. Version 2 is of 2026-11-17. Version 1 (2026-11-16) was the proposal. Version 2 is the plan: R-139 approves stages A and B, defers stage C, and decides OD-ROL-1 to OD-ROL-14. It adds the section "The hub that the pool runs" and OD-ROL-15.
From: the operator's request of 2026-11-16 for two proposals: a watchdog
([watchdog.md](../proposals/watchdog.md)), and the update of the daemon and the hub.
Status: the header of this document holds the state (PLN-020). History: a proposal until 2026-11-17. The
operator decided its tier (OD-W23) and OD-ROL-8 on 2026-11-16, and approved it on 2026-11-17 (R-139).
A requirement with the words "Not approved" has no ruling and is not built (PLN-007).
Map: [work-plan.md](../work-plan.md), category Foundation, decision OD-W23.
Identifiers: ROL-001 to ROL-054. The prefix `UPD` belongs to the desktop application (design
specification Section 3.9) and is not changed here.

## Problem

The desktop application updates itself (UPD-001 to UPD-012). No other program of Anima does.

- To update `anima-daemon`, a person builds it from source on each machine and installs it again.
- To update the hub, a person builds its image on the VPS with Docker Compose.
- The hub does not know the version of a daemon. The daemon does not report it.

Other plans depend on an update that does not exist. USE-008 says that a daemon "takes work
after its update". The watchdog reaches a machine only in a new daemon. R-115 lists "signed daemon
self-update with rollback after 3 failed starts" as later work with no specification. This
proposal is that specification.

## Terms

| Term | Meaning |
| --- | --- |
| Release | One version of each program of Anima, built from one commit. |
| Release record | The signed list of the files of a release for one target. |
| Rollout | The update of the hub and the machines to one release, in steps. |
| Step | The machines that update at the same time in a rollout. |
| Previous version | The program files that a machine had before its update. The machine keeps them. |

## What exists

- A release starts in the Builds view. Builders build the desktop application for `linux-x86_64`
  and `darwin-universal`. The hub checks the files, signs the updater file with the updater key
  and writes the feed at `https://releases.anima.ing` (BLD-045, BLD-064 to BLD-066, R-117).
- `crates/updater-signature` signs and verifies. It is pure and has no file access.
- The release record in [releases.md](../procedures/releases.md) holds what is published. Tasks R5 and R6 of [projects-and-builds-plan.md](projects-and-builds-plan.md)
  are done; R7 is in progress. The update of a real installed application is not verified (OD-16 to OD-18).
- A release build of the desktop application carries `anima-daemon` for its own target, and
  installs it on its own computer (MET-019). No daemon file is published alone.
- The Linux system unit of the daemon runs with no capability and `ProtectSystem=strict`. Thus the
  daemon cannot replace its own program file. The rule MET-002 says the same.
- The daemon connects only to the hub (MET-020). It makes three requests: enrol, push and renew
  (K8). The push answer is empty. The hub cannot tell a daemon which version to get.
- The hub on the VPS is the image `anima-hub:local`, built there from source. A one-shot
  `migrate` job runs before the hub. The way back from a migration is a restore of the backup.
- The hub as a background program on the operator's computer (S4) gets its programs from the
  desktop application. A second `install` is its update.

## The hub that the pool runs (R-130, R-139)

The operator will use only the hosted version (R-130). A hosted hub is a container that the pool
service makes from an image ([hosted-mode.md](../proposals/hosted-mode.md)). This changes the plan:

- The pool replaces the container of a hosted hub. That work belongs to the pool service in the
  private repository. This plan does not update a hosted hub. The method is decided (R-189): a volume,
  a copy, a replacement; see "The update of a hosted hub".
- A hub on the operator's computer updates with the desktop application (ROL-020). That stays.
- A hub on a server that a person runs (the self-hosted setup of `deploy/`) is updated by hand, as
  today. Stage C, which gave that work to a daemon on the server, is deferred (R-139). Its
  requirements stay in this document with the words "Not approved".
- Thus a rollout updates machines only. It brings each machine to the version of its hub.
- The pool updates a hub while its machines have the previous daemon. Thus ROL-034 and ROL-037 and
  their gates (task U9) are necessary for the pool also.
- The hub of the operator on the VPS signs each release today (R-117). If that hub goes away, a
  different place must sign. Decision OD-ROL-15 is decided (R-158, 2026-11-18): a small server that
  only makes releases signs, and the VPS hub signs until that server has published one release.
  Task U12 is the move. Task U2 does not wait for the move: the record has the same form in the two places.

## The update of a hosted hub (OD-ROL-16: decided, R-189)

The operator, 2026-11-18, on the fact that a hosted hub has no update path and must be deleted and made again for a new image: "this also needs a better method". His words were typed in a side chat of the orchestrating session; they are relayed here. He selected no method that day. On a question card of the orchestrating session he then selected "B: volume, copy, replace" (R-189, 2026-11-18; relayed). This section gives the options, what the code of this repository shows, and what the decision adds.

Who owns what. The pool service in the private repository makes, replaces and deletes the container of a hosted hub; its method is its own plan. This repository owns what the hub program must permit: where a hub keeps its state, what a newer program does with an older database, and how a copy of the state is made. OD-ROL-16 is the decision for this repository. The pool plan takes its result.

What the code shows (`main` at `aae8831e`, read on 2026-11-18):

| Fact | Where | Effect |
| --- | --- | --- |
| A hub keeps each file of its state below one root: the database cluster, the vault key, the sealing keys (`data-keys/`), the sealed files, the state of the link with its key and its devices, the logs | `crates/anima-local-hub/src/folders.rs` | One volume at that root holds each thing that must stay. The pool gives the root with `--root` and `--socket-folder` (`arguments.rs`) |
| The programs of an installed hub are below the same root, in `programs/current` | The same file; `programs.rs` | With a volume at the root, a new image does not replace the programs unless the pool starts `run` from the programs of the image, or runs `install` at each start. The pool plan must say which |
| `install` on a hub that runs is an update: stop, replace the programs, start. It keeps the replaced programs. It refuses programs of a lower version unless `--step-back` is given | [local-hub.md](../procedures/setup/local-hub.md), `programs.rs` | The step that option A needs exists for a hub on a computer. It is not built for a container |
| The migrations go one way. The hub has 73 migration files and no file that undoes one. `anima-hub migrate` applies them at the start, as the owner role (R-016) | `crates/hub/migrations/`, `crates/hub/src/main.rs` (`migrate`) | After a new hub has started on a database, the way back to the older hub is a copy that was made before, or the rule ROL-034 |
| ROL-034 says that a migration only adds, so that the hub before it can use the database. No gate checks it: task U9 is not started | This plan | Today nothing proves that an older hub runs on a newer database. A way back that only starts the older image is not proved |
| A backup is `pg_dump` of the database, made while the hub serves. `restore` needs an empty database. The code says that a copy of the cluster folder while the server runs is not a backup | `crates/anima-database/src/backup.rs`, `main.rs` | A snapshot of the volume is a correct copy only when the hub is stopped, or when the provider makes it atomic and the database replays its log at the next start (`recovered` in `status`). A `pg_dump` beside it is the copy that this repository can check |
| The backup holds the database only. The vault key, the sealing keys, the sealed files and the key of the link are files beside it | `folders.rs` | A copy for a way back must hold the whole root, not only the dump. An export of option D that holds only the dump loses the keys, and a database without its vault key cannot be read |
| A hub refuses a database cluster of a different PostgreSQL major version, older or newer, before it reads it | `crates/anima-database/src/cluster.rs`, `startup.rs` | An image with a new PostgreSQL major version cannot start on the old volume. That one case needs a dump and a restore, with each method |
| The path `/var/lib/anima/hub` is not in this repository | `git grep` | It is the root that the pool gives. This repository does not fix it |

The options:

| Option | What it is | For | Against |
| --- | --- | --- | --- |
| A | The pool puts new programs into the container that runs: stop the hub, replace `programs/current`, start | The step exists for a hub on a computer (`install`). No new container, no new volume | The container is no longer what its image says: two hubs of one image can run different programs. A change of the base image, of a system library or of PostgreSQL does not arrive. The way back is the replaced programs on the same disk, and the database has already changed |
| B | The state of a hub is on a volume of its own. The pool makes a copy of the volume, replaces the container with the new image on the same volume, and checks the hub. It does this in steps: one hub, then the others | The container is always its image. The copy is the way back and does not depend on ROL-034. The steps are the rollout of this plan (ROL-023, ROL-026, ROL-027) at the pool. A hub keeps its link key, so its devices stay joined | The pool must hold one volume for each hub and one copy for each update. A copy must be of a stopped hub, or atomic. The first move of a hub that has no volume of its own is a migration of its own. The PostgreSQL case above stays |
| C | The hub updates itself from the release feed, as the desktop application does | No work of the pool for each update | The hub in a container must then replace its own programs: it needs a right that ROL-015 keeps away from the part that speaks to the network, and the container must never get the Docker socket (ROL-019). No image of the hub is published (OD-ROL-3 is deferred). Each hub of each person updates when it wants, with no steps and no copy first |
| D | Delete and make again, with an export before and an import after | No volume to keep. It is the method of today with one more step | The hub is away for the whole time. The import needs an empty database and each key file of the root. A fault in the middle leaves no hub. The address of the hub and its place on the link can change, so each device joins again unless the export holds the link key |

The operator selected option B (R-189). The check against the code: the facts agree with B, with three conditions, and R-189 carries them.

1. The copy is of the whole root, made while the hub is stopped or as an atomic snapshot, with a `pg_dump` in it that `anima-database` can check. The code does not accept a copy of a cluster that runs as a backup.
2. The way back is the copy. It is not the older image on the new database: the migrations cannot be undone, and no gate proves ROL-034 (task U9). When U9 exists, the older image on the same volume becomes a second, faster way back.
3. The programs run from the image, not from `programs/current` on the volume. If not, option B becomes option A without the pool knowing.

Not decided by R-189 (a recommendation of the session that wrote this section): build task U9 before the first update of a hub that holds the data of a person who is not the operator. Options A, C and D are not selected.

Until option B exists: the operator deletes his own hub by hand and the pool makes it again at his next sign-in (R-189). That is acceptable for his hub only: it is his own hub and he deletes it himself. It is not a method for the hub of a different person.

What R-189 adds to this plan: the requirements ROL-053 and ROL-054 (in "Requirements", "The copy of a hub before its replacement") and task U13. Stage C (a hub on a server that a person runs) stays deferred (R-139); the same three conditions would apply to it.

What the pool plan must take. The pool service is in the private repository, and no session of this repository changes it. Its plan must take these points from this section:

1. One volume for each hosted hub, at the root that the pool gives with `--root`. Each thing that must stay is below that root: the database cluster, the vault key, the sealing keys, the sealed files, the state of the link with its key, the logs.
2. Before each update: stop the hub, make a copy of the whole root, and make sure that the copy holds a dump that `anima-database` can check (ROL-054 gives the command). A copy of a hub that runs is not a copy, unless the provider makes an atomic snapshot.
3. The programs of the hub run from the image, not from `programs/current` below the root (ROL-053). If not, a new image does not change the programs.
4. The way back is the copy on a new volume, with the older image. It is not the older image on the volume that the new hub has used: the migrations go one way.
5. The update goes in steps: one hub, a check that it answers and that its machines are connected, then the others (as ROL-023, ROL-026 and ROL-027 for machines).
6. A new image with a different PostgreSQL major version cannot start on the old volume. That case needs a dump and a restore, with each method.
7. A hub that has no volume of its own today needs one move first. For the hub of the operator the move is the deletion of point "Until option B exists".
8. A hub must keep its address and its link key through an update, so that its devices stay joined. A copy of the whole root gives that.

Two facts of the pool of 2026-11-18, as the orchestrating session relayed them (not seen by a session of this repository): the image of a hosted hub is `anima-hub-2ba73005`; and the operator deleted his hub `h6opknk7w29jw56jqd0kc` that day (the pool answered 200). A new hub is made at his next sign-in, which he has not reported.

## Requirements

### One release, one version

- **ROL-001.** A release must give one version and one commit to each program of Anima: the
  desktop application, `anima`, `anima-hub`, `anima-daemon`, `anima-gateway` and `anima-link`
  (INS-002 says this for two of them).
- **ROL-002.** A release must publish `anima-daemon` for each target that a machine of the
  operator has, and the hub for each target that runs a hub. Decision OD-ROL-8 gives the list. (The part for the hub is not approved: deferred with stage C, R-139.)
- **ROL-003.** Each target of a release must have one release record. The record gives the
  version, the commit, the target, and the name, the program, the size and the SHA-256 of each
  file. It also gives the lowest version that this release can replace (ROL-041).
- **ROL-004.** The hub must sign the release record after its checks of the files (BLD-064), with
  the version in the signature (as BLD-046). One signature then covers each file of the target.
- **ROL-005.** The release record and its files must be under `<target>/<version>/` of the
  release host (as BLD-060). There is no package manager channel (INS-008).

### How a release is fetched and verified

- **ROL-006.** The hub must read the release host at start and then each 4 hours (as UPD-001).
  A hub that cannot reach the release host must say so in the Updates section, with no dialog.
- **ROL-007.** The hub must verify the signature of the record and the size and the SHA-256 of
  each file before it offers the release. The public key must be in the program of the hub, not
  in its configuration.
- **ROL-008.** A daemon must get the record and its file from its hub, with its daemon token. It
  must not connect to the release host. MET-020 stays as it is. The hub must keep the files of
  the current release and of the release before it.
- **ROL-009.** The hub must tell a daemon only the version to get. It must never send an
  address, a path, a command or a file name.
- **ROL-010.** The machine must verify by itself. The part that replaces the program (ROL-015)
  must verify the signature of the record with the public key in the program, then the version,
  the target and the program name in the record, then the size and the SHA-256 of the file. It
  must not trust the part that did the download.
- **ROL-011.** If one check of ROL-010 fails, the machine must install nothing. It must report
  "Refused" with the cause, and continue with its version (as UPD-002).
- **ROL-012.** A machine must refuse a version that is lower than its own. The one exception is
  its own previous version in a rollback (ROL-031).
- **ROL-013.** A file must name its program in the signed record. Thus a signed file of the
  desktop application cannot be installed as the daemon or as the hub.
- **ROL-014.** A hub that holds no signing key gets the releases of the Anima project in the same
  way. It cannot publish one (PRO-037, PRO-038, OD-W20).

### Who replaces a program

- **ROL-015.** The part of the daemon that speaks to the hub must not hold the right to replace a
  program file. A second unit of the same program must hold that right. It must have no network
  access. This is the same division as WDG-040 of the watchdog.
- **ROL-016.** The machine must replace a program file with one rename, so that a loss of power
  leaves the old file or the new file, never a part of one.
- **ROL-017.** The machine must keep the previous version of each program that it replaces. It
  keeps one previous version only.
- **ROL-018.** A release must not change a file that the daemon keeps (configuration, token,
  buffer) in a way that the previous version cannot read.
- **ROL-019.** On a server, the hub is a container. The second unit of the daemon on that machine
  must replace it: load the image, compare its digest with the record, start the hub, and keep
  the previous image. The container of the hub must never get the Docker socket. (Not approved: deferred with stage C, R-139.)
- **ROL-020.** A hub that runs as a background program on the operator's computer (S4) updates
  with the desktop application on that computer (UPD-003). A rollout must show it and must not
  replace it.
- **ROL-021.** A user installation of the daemon (MET-019) updates with the same checks. Its
  files belong to the user, so it needs no administrator password (as INS-009).

### The rollout

- **ROL-022.** The operator must be able to start a rollout with one press. The hub must write
  the start, each pause, each stop and each rollback to the audit log (LOG-005).
- **ROL-023.** A rollout must update in this order: the hub, then one machine, then the other
  machines in steps. Each step has at most a quarter of the machines, and at least one. (The part for the hub is not approved: deferred with stage C, R-139.)
- **ROL-024.** While a step replaces a program of Anima on a machine, the watchdog of that
  machine must not restart that program (WDG-030).
- **ROL-025.** A policy must select the first machine: a machine that is connected, that is not
  the machine of the hub and that runs no job. The policy must be a pure function. The operator
  can select a different machine before the press.
- **ROL-026.** A step is good when each of its machines passes its check (ROL-029). The next step
  must start 10 minutes after a good step, not earlier.
- **ROL-027.** A rollout must stop when one machine rolls back or refuses the release. The
  machines that have the new version keep it. The operator can then continue, or roll back each
  updated machine with one press.
- **ROL-028.** A machine must not start its update while it runs a job, a build or a job of a
  runner. The operator can hold a machine: a rollout leaves a held machine out and says so. A
  machine that is set off for updates at the machine must stay off; the hub cannot set it on.

### The check after an update, and the rollback

- **ROL-029.** A new daemon passes its check when, in 3 minutes: it starts, it passes the checks
  of `anima-daemon doctor` (MET-023), the hub accepts one push from it, and the hub sees the new
  version.
- **ROL-030.** The decision to roll back must not depend on the new version. A new version that
  cannot start, or that stops, must be rolled back by the machine, also after a restart of the
  machine. The limit is 3 failed starts, or 3 minutes with no good check (R-115).
- **ROL-031.** A rollback must put the previous version back, start it, and report "Rolled back"
  with the cause. The hub must write an event with the severity Error and send a notification.
- **ROL-032.** A machine must try one version at most two times without the operator. After the
  second rollback, it waits for "Try again".
- **ROL-033.** Before the hub is replaced, the machine of the hub must make a database backup
  (DB-007) and must wait for its success. (Not approved: deferred with stage C, R-139.)
- **ROL-034.** The hub of a release must be able to use the database of the release after it. A
  migration in one release can add; it must not remove or change what the previous hub reads. A
  gate must run the previous hub on the new database.
- **ROL-035.** A new hub passes its check when, in 5 minutes: `anima-hub health` is good, the API
  gives the new version, and the hub accepts one push of a daemon. If not, the machine must start
  the previous image. The database stays as it is (ROL-034). (Not approved: deferred with stage C, R-139.)
- **ROL-036.** A release that cannot obey ROL-034 must say so in its record. A rollout of it
  needs a second confirmation that names the restore procedure. An automatic rollout must never
  take it.
- **ROL-037.** The hub and the daemon of a release must work with the daemon and the hub of the
  release before it. A gate must test each of the two pairs.
- **ROL-038.** While the hub restarts, the client must show "The hub is updating", with the time
  that it started. It must not show the error state of a lost hub (UI-030) for 5 minutes. (Not approved: deferred with stage C, R-139.)

### A machine that is not connected

- **ROL-039.** A rollout must not wait for a machine that is not connected. That machine leaves
  its step with the state "Waiting: not connected". The rollout can finish without it.
- **ROL-040.** When the machine connects again, the hub must tell it the version of the newest
  rollout that includes it. That can be newer than the version of the rollout that it missed. The
  machine then updates alone, with the same check and the same rollback. Its failure sends a
  notification and does not change a finished rollout.
- **ROL-041.** A machine must be able to go to the newest release directly. If a release cannot
  replace a version, its record says so (ROL-003), and the hub gives the releases between them in
  order.
- **ROL-042.** The hub must keep the push route and the update route of a contract version while
  one enrolled machine last reported that version. Before a hub update that removes a contract
  version, the client must name the machines that the new hub cannot update.
- **ROL-043.** A machine whose daemon token ended while it was not connected (MET-013) cannot get
  an update. Its state is "Enrol again".
- **ROL-044.** A daemon that was installed before this function has no update function. Its
  state is "Needs a manual installation". One installation by hand, or by the desktop application
  on its own computer, gives it the function.
- **ROL-045.** If the hub stops while a machine is in its check, the machine rolls back
  (ROL-030). That is safe. The hub must then give that machine the update one more time.

### Automatic update

- **ROL-046.** The setting "Update automatically" must have three values: "Off", "Machines" and
  "Machines and hub". The default is "Off". (The part for the hub is not approved: deferred with stage C, R-139.)
- **ROL-047.** With the setting on, the hub must start a rollout by itself when it has verified a
  new release (ROL-007). The rules of the rollout stay the same, but the wait after the first
  machine is 1 hour.
- **ROL-048.** The operator can set a window for automatic rollouts, as a rule with fields
  (MW-004). An automatic rollout must start a step only in the window.
- **ROL-049.** An automatic update of the hub must start only when no session is live and no
  build runs. If that does not occur in the window, the hub waits for the next window. (Not approved: deferred with stage C, R-139.)
- **ROL-050.** An automatic rollout that stops (ROL-027) must send a notification. It must not
  start again for the same release without the operator.

### What the hub records and shows

- **ROL-051.** The daemon must report its version with each push. The hub must keep the version of
  each machine and its own version, and give them through the API.
- **ROL-052.** The hub must write one event for each machine that updates, rolls back or refuses.
  The actor kind of a rollout from the press is User. The actor kind of an automatic rollout is
  Hub (LOG-013). An API key must not start a rollout (OD-ROL-9).

### The copy of a hub before its replacement (R-189)

- **ROL-053.** A hub must keep each file of its state below one root, and its programs must be
  able to run from a place that is not below that root. A test must prove the two parts. (True
  in the code of 2026-11-18; not stated and not tested before.)
- **ROL-054.** One command must make a copy of the whole root of a stopped hub, with a dump of the
  database in it, and one command must check a copy: each part is present and the dump can be
  read. The command must refuse a hub that runs. It must write no key and no secret to its output.

## In the client

The build order is CMD-030: the hub API, the `anima` command, the MCP tool, then the client.

| Place | What the operator sees |
| --- | --- |
| Navigation | One banner with `role="status"`, as UPD-004: "Anima 0.3.0 is available for the hub and 7 machines", with the button "Review update". A rollout in progress shows in the "running now" area (PRO-026). |
| Settings, Updates (SET-006) | The row of the desktop application stays. A new group "Hub and machines" shows the available version, the versions in use ("The hub and 5 machines have 0.2.4. 2 machines have 0.2.3.") and the primary button "Update all". |
| The confirmation | One sheet before the rollout starts. It gives the order (the hub, then the first machine by name, then the others), and three facts: "2 machines are not connected. They update when they connect." "1 live session will stop when the hub restarts." "The hub is not reachable for about 1 minute." The operator can change the first machine here. |
| A rollout in progress | The steps in order, each with its machines and their states. The time to the next step. The buttons "Pause" and "Stop". After the end, one summary line, for example "Done. 2 machines wait." |
| The list of machines | One row for the hub and one for each machine: the name, the version and the state in words, never as a colour only: "Up to date", "Update available", "Waiting for its step", "Updating", "Checking", "Waiting: not connected", "Rolled back" with the cause, "Refused" with the cause, "Held", "Set off at the machine", "Needs a manual installation", "Enrol again". Row actions: "Update now", "Roll back", "Hold". |
| The hub row, local hub | "The hub updates with the application on this computer" (ROL-020), with no button. |
| Automatic update | A segmented control "Off", "Machines", "Machines and hub", and the window. Each saves at once (SET-005). |
| Devices page and device detail | The Overview shows "Daemon 0.2.4", with "Update available" as a link to the Updates section. "Rolled back" puts the device in "Needs attention" (DEV-026), and the reason sentence of DEV-020 gives the cause. The Logs tab has the events of ROL-052. |
| While the hub restarts | "The hub is updating" (ROL-038), then the result of the hub's check. |
| `anima` and MCP | `anima update status`, `start`, `pause`, `stop`, `rollback`, `hold`. The read tool is `allow`. The change tools are `off` by default (MCP-002). |

## What depends on unfinished work

| Dependency | State today | Effect |
| --- | --- | --- |
| Signed releases: the first release (tasks R5 to R7 of [projects-and-builds-plan.md](projects-and-builds-plan.md)) | In progress: see the release record in [releases.md](../procedures/releases.md). OD-16 to OD-18 are open. | Nothing can be fetched or verified until one release exists. The signature path is proved only in tests. |
| The builder | It builds the desktop application only, from a fixed recipe (OD-W16), for two targets. The general builder (group A) is not started. | The release needs new files: the daemon alone for each target, the image of the hub, and the release record. This is a change to the recipe of the Anima release, or a task of group A. |
| A builder for each target (BLD-003) | `linux-x86_64` and `darwin-universal` only | A machine of a different target, for example Linux on ARM, gets no daemon file (OD-ROL-8). |
| The local hub in the release (task R6; S4 T9) | Not started | ROL-020 needs it. |
| A change to K8, which is frozen (CON-001) | Not asked | ROL-051 needs an optional member of the push. ROL-008 and ROL-009 need a new route, or the work route (task W1, not started). |
| A ruling number for OD-W10, which changes R-115, and a change to MET-002 | R-160 (2026-11-18) is the ruling for OD-W10. R-139 gives the change to MET-002 for the update | The daemon replaces a program on the machine. MET-002 permits it to write only its configuration, its token and its buffer. |
| The hub on the VPS is built there from source | No image is published | ROL-019 needs a published image and a program on the VPS with the Docker right. The first installation of that program is a change to the VPS (VPS-005). |
| The restore procedure (FND-006, work plan tier 1) | Proposal, not written | It is the last way back when the hub cannot roll back (ROL-036). Decision OD-ROL-4 puts the hub after it. |
| The isolation design and the security review (tasks W2 and S) | Not started | ROL-015, ROL-019 and ROL-030 are security subjects. |
| The gates for ROL-034 and ROL-037 | They do not exist | Without them, a rollback of the hub and a rollout that stops in the middle are not safe. |
| The watchdog ([watchdog.md](../proposals/watchdog.md)) | Proposal | ROL-024 applies only when the watchdog exists. This proposal does not need the watchdog. |

The key is a larger risk after this proposal. R-117 accepted that a hub that holds the updater
key can sign an application that an installed application trusts. With this proposal, the same
key signs a program that runs on each machine, and on some machines with the right to replace
programs. The operator must accept this again, or select a second key (OD-ROL-2).

## Conflicts with text that exists

| Text | Says | This proposal |
| --- | --- | --- |
| MET-002 | The daemon writes only its configuration, token and buffer files | Add its own program files, through the second unit (ROL-015). |
| R-115, "Later, not yet spec" | Signed daemon self-update with rollback after 3 failed starts | This proposal is that specification (ROL-030). |
| K8 | Three requests; the push answer is empty | See "What depends on unfinished work". |
| MET-020 | The daemon connects only to the hub | Kept (ROL-008). |
| Design specification Sections 3.9 and 3.10 | A release is the desktop application | Add the daemon, the hub and the release record (ROL-001 to ROL-005). The `UPD` requirements stay. |
| BLD-069, Section 3.9 "Rollback" | The hub writes no lower feed; a rollback is a manual write | Kept. A rollback of a machine uses its own previous version (ROL-017), not the feed. |
| INT-003 | The calls that an API key must not make | Add the start of a rollout (OD-ROL-9). |
| SET-006 | The Updates section shows the version of the application | Add the group "Hub and machines". |
| `deploy/README.md` | The operator builds and starts the hub by hand | That path must stay and must work beside ROL-019. |
| VPS-005 | No change to the VPS without the operator's confirmation in chat | The rule is for agents that build Anima. A press in the client is the operator's own act. An automatic update of the hub needs a ruling that the setting is that confirmation. |

## Open decisions

| Number | Decision | Recommendation | Result |
| --- | --- | --- | --- |
| OD-ROL-1 | Where a daemon gets its files: from its hub, or from the release host | From its hub (ROL-008). MET-020 and the VPN rules stay, and a machine with no internet access can update. | Decided (R-139): from its hub. |
| OD-ROL-2 | The signing key: the updater key of UPD-012 for each program, or a second key for the daemon and the hub | One key in the first version, with the program name in the signed record (ROL-013). It is one key to keep safe and to back up. A second key is correct if the operator wants a limit on the risk of R-117. | Decided (R-139): one key, with the program name in the signed record. OD-ROL-15 asks where the key is. |
| OD-ROL-3 | The form of the published hub: an image file on the release host, an image in a registry, or a build from source on the machine of the hub | An image file on the release host, with its digest in the signed record. GitHub is not in the release path (R-107), and a build from source is slow and needs the tools on the VPS. | Decided (R-139): no form is selected; the question is deferred with stage C. |
| OD-ROL-4 | The order of the build | Three stages. A: the versions show (ROL-051). B: the machines update with one press. C: the hub joins, after the restore procedure (FND-006) is written. Until C, the hub row shows the command for an update by hand. | Decided (R-139): stages A and B. Stage C is deferred. The hub row shows that the pool, the application or the person updates the hub. |
| OD-ROL-5 | The values and the default of the automatic update | "Off" by default. "Machines" in stage B. "Machines and hub" only in stage C. | Decided (R-139): "Off" by default, and "Machines". The value "Machines and hub" is not built. |
| OD-ROL-6 | The numbers: a quarter for each step, 10 minutes between steps, 1 hour after the first machine of an automatic rollout, 3 minutes and 5 minutes for the checks | As written. Fixed in the first version. | Decided (R-139): as written. |
| OD-ROL-7 | Who selects the first machine | The policy, and the operator can change it in the confirmation (ROL-025). | Decided (R-139): the policy, and the operator can change it. |
| OD-ROL-8 | The targets of the daemon and the hub | **Decided (operator, 2026-11-16): no Linux machine on ARM is in use ("don't have any").** The daemon: `linux-x86_64` and `darwin-universal`. The hub: `linux-x86_64`. Add a target only when the operator has such a machine and a builder for it (ETH-009). | Decided (operator, 2026-11-16). |
| OD-ROL-9 | Can an API key or an AI agent start, stop or roll back a rollout? | No. Add those calls to the list of INT-003. A key can read the state. | Decided (R-139): no. A key can read the state. |
| OD-ROL-10 | Does a machine that has "Takes work" off (USE-004) get updates? | Yes. The update has its own switch at the machine (ROL-028). If not, a machine that takes no work keeps an old daemon. | Decided (R-139): yes. |
| OD-ROL-11 | What a rollout does when it stops: only stop, or also roll back each updated machine | Only stop (ROL-027). The updated machines passed their check. The operator has "Roll back" for one and for all. | Decided (R-139): only stop. |
| OD-ROL-12 | A release whose database change the previous hub cannot read | Forbid it in the gates (ROL-034). Permit an exception that the record names, with a second confirmation and no automatic rollout (ROL-036). | Decided (R-139): forbidden in the gates, with the exception that the record names. |
| OD-ROL-13 | On a computer that has the desktop application, who updates the daemon: the rollout, or the application when it updates itself | The rollout only. The application installs the daemon only when the computer has none (MET-019). Two updaters for one program can roll each other back. | Decided (R-139): the rollout only. |
| OD-ROL-14 | Is the desktop application part of a rollout? | No. UPD-001 to UPD-006 stay. The application installs with no prompt and waits for "Restart to update". | Decided (R-139): no. |
| OD-ROL-15 | The operator's hub on the VPS holds the updater key and signs each release (R-117). When the operator uses only a hosted hub (R-130), which place holds the key and signs? The options A to E, with their risks, are in [release-four-platforms.md](../proposals/release-four-platforms.md) Section 5: (A) the VPS hub stays; (B) a small server that only makes releases, apart from the pool; (C) a server of the pool or a hosted hub; (D) the account service; (E) the computer of the operator. | Version 2 of this plan recommended: the VPS hub stays (option A), until a different design is approved. A hosted hub is on a server that it shares with the hubs of other persons, and its key would sign a program for each machine of each person. Task U2 waits for this decision. | Decided (R-158, operator, 2026-11-18: "B: release server"): option B. A small server that only makes releases holds the updater key and signs. It is at Hetzner and is not a host of the pool. The hub on the VPS signs until that server has published one release; then the VPS hub can go away. One updater key, moved one time; no new key. Task U12 is the move. |
| OD-ROL-16 | A hosted hub has no update path: for a new image the pool deletes it and makes it again. The operator (2026-11-18): "this also needs a better method". Which method must the hub program of this repository support? The options A to D, with what the code shows, are in "The update of a hosted hub": (A) the pool puts new programs into the container that runs; (B) the state of a hub is on a volume of its own, the container is replaced, with a copy before each update and a rollout in steps; (C) the hub updates itself from the release feed; (D) delete and make again, with an export and an import. | B, with three conditions: the copy is of the whole root of a stopped hub or an atomic snapshot, with a `pg_dump` that can be checked; the way back is the copy, because the migrations go one way and no gate proves ROL-034 (task U9); the programs run from the image and not from the volume. | Decided (R-189, operator, 2026-11-18: "B: volume, copy, replace"), with the three conditions. Until it exists he deletes his own hub by hand. ROL-053, ROL-054, task U13. |

The tier of this proposal is decision OD-W23 of the work plan.

## Tasks

U1 and U4 are done. The other tasks are not started. The states obey PLN-007.

| Task | Content | Needs | State | Commits |
| --- | --- | --- | --- | --- |
| U1 | Stage A. The daemon reports its version; the hub keeps it and gives it. Hub API, then `anima`, then MCP, then the Device page. | R-122 (operator, 2026-11-16): the optional member of K8 | Done | `e29e59db`, `553a6522`, `68b293fe`, `328f6ff4`, `d162234f`. Reviewed by an independent reviewer agent: accepted, no blocker. Open: the daemon test `r_122_each_batch_carries_the_version_of_the_daemon` runs only on Linux and has not run yet; the release builder does not build `anima-daemon` with `ANIMA_VERSION` yet (task U2). |
| U2 | The release record: its fields, its signature, its place on the release host. The recipe of the Anima release makes the daemon files and the record. | Tasks R5 to R7; a release for each target. OD-ROL-15 is decided (R-158), and R-158 names this task | Not started | — |
| U3 | A prototype on one Linux machine and one Mac: the second unit, the rename, the rollback of a version that stops at once and of one that does not answer, also after a restart of the machine (ROL-030). A record with measured results. | R-139 names it. The operator names one Linux machine and one Mac on which an agent can install a test unit and restart the machine | Not started | — |
| U4 | The contract: how the hub tells a version, how a daemon gets the record and the file, the states of a machine. | R-139: the change to K8 and to MET-002 | Done | `5bd69f99`: K8 Section 8. The full gates passed on 2026-11-17 (21 of 21). The contract is ruled and not built: tasks U5 and U6 build it. |
| U5 | The pure rules: the order of a rollout, the steps, the first machine, the states (ROL-023 to ROL-028, ROL-039 to ROL-041). | U4 | Not started | — |
| U6 | Stage B in the hub and the daemon: fetch, verify, serve, replace, check, roll back. Hub API, then `anima`, then MCP. | U2, U3, U5 | Not started | — |
| U7 | Stage B in the client: the Updates group, the confirmation, the rollout, the list of machines. | U6 | Not started | — |
| U8 | The automatic update of machines (ROL-046 to ROL-050). | U6 | Not started | — |
| U9 | The gates of ROL-034 and ROL-037. | The first release (task R7): before it, no earlier release exists to test against | Not started | — |
| U10 | Stage C: the image of the hub in the release, the replacement of the hub with the backup and the check (ROL-019, ROL-033, ROL-035). | Deferred (R-139): not approved. U6, U9; FND-006; the operator for the first installation on the VPS (VPS-005) | Not started | — |
| U11 | The security review of the update. | U6; then U10 | Not started | — |
| U12 | (2026-11-18: R-173 says that the job of a release workflow signs, with the updater key as a secret of GitHub. What the release server is still for is OD-W34 of the work plan. Do not build this task before that answer.) The place that signs moves as OD-ROL-15 decides. An agent writes the procedure in `deploy/README.md` and in [releases.md](../procedures/releases.md): the set-up of the place, the two secret files of the updater key, the R2 token, the check that publishing is on, and how a builder connects to it. The operator moves the key and the token (UPD-012); an agent never reads them. One release is published from the new place before the old place is stopped. | R-158 names this task. The operator: the new server, each change to a server (VPS-005), and the move of the key and the token (UPD-012). An agent can write the procedure before the server exists | An entry in [need-human-testing.md](../need-human-testing.md): the start log of the new place shows publishing on, one release is published from it, and an installed application updates to it. | Not started | — |
| U13 | The hub side of the update of a hosted hub (R-189; ROL-053, ROL-054): the test that the state is below one root and that the programs run from outside it; the command that makes a copy of the whole root of a stopped hub, and the command that checks a copy, in `anima-local-hub` with `anima-database` for the dump. It is a command of the hub programs on a server, not a function of the hub API, so no `anima` command and no MCP tool (CMD-030). The pool work is in the private repository and is not this task. | R-189 names this task | The gates: a copy of a stopped hub passes the check; a copy with a missing key file or a broken dump fails it; the command refuses a hub that runs; a hub started on the copy with the same programs reads its data. | Not started | — |

The release record in [releases.md](../procedures/releases.md) holds what is released. The macOS releases 0.2.0 and 0.2.1 do not have U1: its commits are not in them. Thus no machine has a released daemon that reports its version, and ROL-044 applies only to the daemons that the operator built by hand. The next release carries U1.
U1 and U3 need no release. U2 and each later task need a release for their target.

## Needs

- For task U12: the release server of OD-ROL-15 (R-158), which only the operator makes.
- For task U3: one Linux machine and one Mac on which an agent can install a test unit and restart the machine.
- A release for each target (task R7), which only the operator can do. No Linux release exists. Tasks U2 and U6 to U9 need one.
- The security reviewer for ROL-010, ROL-015 and ROL-030 (a `security` task, U11).
- R-139 gives the ruling for the change to K8 and to MET-002. The ruling for the work route of OD-W10 is R-160 (2026-11-18); this plan does not need it.

## Council follow-up: mixed-version acknowledgement (2026-11-18)

R-152 names the preparation below. It changes no approved requirement or contract. Read current `main` and the claims before taking a task. A source review is not a Passed human check. The task owner records the tested build and the result; unknown results stay unknown.

| Task | Work and acceptance | Depends on | State |
| --- | --- | --- | --- |
| U-C1 | Give the U6 owner the regression cases for metrics receipt with an optional update instruction: valid instruction, invalid optional instruction and no instruction. Separate a stored metrics batch from update negotiation and execution; specify the observable sender and target outcomes. Do not turn a stored timestamp into proof of an update handoff. This is evidence for U6 and the U9 matrix, not a second update implementation. | U6 owner; approved K8 contract; an update-capable sender and target fixture | Not started. Preparation only; test implementation waits for assignment and the integrated U6 interface |
