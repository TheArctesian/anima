---
kind: Proposal, with a task table
state: Draft
evidence: Version 1 (2026-11-16). Not approved (OD-W22). No tier yet (Section 4, row 4.19). Task G1, a prototype, is In progress: launchd, Docker and systemd are measured; the right of WDG-040 is open ([research/watchdog/g1-supervisors.md](../research/watchdog/g1-supervisors.md)). The other tasks of its "Tasks" are Not started.
---

# Proposal: the watchdog (keep the important programs of a machine running)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version: 1. Date: 2026-11-16.
From: the operator's request of 2026-11-16 for two proposals: a watchdog, and the update of the
agent and the hub ([agent-and-hub-update.md](agent-and-hub-update.md)).
Status: Draft. A proposal for a ruling. No requirement is approved. Nothing is built.
Map: [work-plan.md](../work-plan.md), categories See and Use, decision OD-W22.
Identifiers: WDG-001 to WDG-041.

## Problem

A check tells the operator that a program stopped (the systemd check and the Docker check of
design specification Section 6.4, and CHK-014 for restart loops). Nothing starts the program
again. The operator opens an SSH session and does it by hand.

systemd, launchd and Docker can start a program again after it exits. They do not start again a
program that runs and does not answer. They do not tell the hub what they did. Thus the operator
does not see a program that restarts each night.

## Terms

| Term | Meaning |
| --- | --- |
| Watch | One program of one machine that the operator marked as important, with its rules. |
| Watchdog | The function of `anima-agent` that does the watches of its machine. |
| Supervisor | The program of the machine that starts and stops a watched program: systemd, launchd or Docker. |
| Probe | One local test that finds if a watched program answers. |

CHK-010 uses the word "watchdog" for a different thing: the hub's guard of its check scheduler.
To keep names truthful, the code of this proposal uses `Watch` and `watches/`, and the interface
uses "Watchdog" only as the name of the function.

## What exists

- The hub has a systemd check and a Docker check. They run from the hub through SSH. They read
  state and change nothing (CHK-012, CHK-014).
- `anima-agent` runs on each machine. Its rule is to read only (MET-002). Its Linux system unit
  runs as the user `anima-agent` with no capability, `ProtectSystem=strict` and
  `NoNewPrivileges=yes`. Thus the agent of today cannot restart a unit or a container.
- The agent makes three requests only: enrol, push and renew (K8). The hub cannot send it a list.
- OD-W10 (operator, 2026-11-12) permits work on a machine through one program, `anima-agent`,
  and no second binary. It has no ruling number yet. The work route is not built (task W1).
- R-115 lists "read-only service discovery (systemd, Docker, listeners, cron and timers)" as
  later work. HOST-001 of [hosted-applications.md](hosted-applications.md) asks for the same list.

## Proposal

### What a watch can be

- **WDG-001.** A watch must name one program of one of these kinds. There is no other kind.

  | Kind | Machine | Named by | Supervisor |
  | --- | --- | --- | --- |
  | systemd unit | Linux | The unit name (the name rule of the systemd check) | systemd |
  | launchd job | macOS | The label of the job | launchd |
  | Docker container | Linux, macOS | The container name (the name rule of the Docker check) | Docker |

- **WDG-002.** A watch must not name a process that has no supervisor. To start such a process
  needs a command, and a command from the hub is remote execution (USE-006, CHK-012). The
  operator must put the program in a unit, a job or a container first.
- **WDG-003.** A watch holds names and numbers only. No field of a watch is a command, a path of
  a program, a script or an environment value.
- **WDG-004.** A machine can have at most 32 watches.
- **WDG-005.** The agent must report the units, the jobs and the containers that it finds on its
  machine: the name, the kind, the state and the restart rule of the supervisor. It must report
  no other field. The client uses this list to offer a watch (HOST-001).

### How the watchdog finds a failure

- **WDG-006.** The watchdog must read the state of each watched program from its supervisor each
  30 seconds. It must not read the process table to decide.
- **WDG-007.** A watch can have one probe. The probe kinds are: the health status that Docker
  gives for the container, an HTTP request, and a TCP connection.
- **WDG-008.** An HTTP probe or a TCP probe must go only to a loopback address of the machine or
  to the address of the watched container. The agent must refuse each other address. Thus the hub
  cannot use the agent to send requests into a site.
- **WDG-009.** An HTTP probe must use the method GET, must follow no redirect and must send no
  credential. It must keep only the status code and the response time. It must not keep the body.
- **WDG-010.** A program "does not answer" after 3 probes in sequence fail (as CHK-002).
- **WDG-011.** A probe failure must not count during the start time of the program. The default
  start time is 60 seconds after each start. The operator can set it up to 15 minutes.
- **WDG-012.** The probe rules and the restart rules must be pure functions that take the time as
  a parameter (CQ-021).

### Restart rules

- **WDG-013.** The watchdog must restart a watched program in two cases only: the program stopped
  and its supervisor did not start it again, or the program does not answer (WDG-010).
- **WDG-014.** The delay before each restart must be: none, 30 seconds, 2 minutes, 5 minutes and
  15 minutes. The delay starts when the watchdog finds the failure.
- **WDG-015.** After 5 restarts in 1 hour, the watch must go to the state "Gave up". The watchdog
  must not restart the program again until the operator selects "Try again", or the program runs
  again for another reason.
- **WDG-016.** The count of WDG-015 must go to zero after the program runs and answers for
  30 minutes (the same period as CHK-014).
- **WDG-017.** The watchdog must do one restart at a time on a machine. A second restart waits
  until the first program is in its start time.
- **WDG-018.** The operator can pause a watch (as CHK-005). The watchdog must not restart a
  paused program. It must continue to report its state.
- **WDG-019.** The watchdog must not restart a program while a maintenance window applies to its
  machine or to its watch (CHK-006, MW-003).
- **WDG-020.** The watchdog must apply its rules when the hub cannot be reached. It must keep its
  list of watches on the machine.

### No conflict with systemd, launchd and Docker

- **WDG-021.** The watchdog must never start a process itself. It must ask the supervisor to
  restart the program: a restart job of systemd, a kickstart of launchd, a restart of the
  container through the Docker Engine. Thus one program always owns the process.
- **WDG-022.** The watchdog must read the restart rule of the supervisor: `Restart=` of a unit,
  `KeepAlive` of a job, the restart policy of a container. The client must show it in words.
- **WDG-023.** When the supervisor has a restart rule, a program that exits belongs to the
  supervisor. The watchdog must wait while the supervisor says that it starts the program again
  (systemd `activating` and `auto-restart`, Docker `restarting`). It must not count that time as
  a failure.
- **WDG-024.** When the supervisor stopped its own attempts (for example systemd reached the
  start limit of the unit), the watchdog can restart the program with the delays of WDG-014, to
  the limit of WDG-015. Decision OD-WDG-5 can set this off.
- **WDG-025.** The watchdog must tell a stop by a person from a failure. A program that a person
  stopped with `systemctl stop`, `launchctl` or `docker stop` must stay stopped, with the state
  "Stopped by a person". When the watchdog cannot tell, it must not restart.
- **WDG-026.** The watchdog must not change a unit file, a job file, a restart policy, a Compose
  file or an image. It must not make a container again. Its only change to a machine is a restart
  of a watched program.
- **WDG-027.** When the supervisor does not answer (for example the Docker daemon is stopped),
  the watchdog must not restart its programs. The state of each is "Supervisor not reachable". If
  the supervisor is itself a watch, the rules apply to it first.
- **WDG-028.** The watchdog must not restart a program while the machine stops or starts. After
  a start of the machine, it must wait until the supervisor says that the start is complete.
- **WDG-029.** A restart by the watchdog must count as a restart for CHK-014. Three restarts in
  15 minutes, by the supervisor or by the watchdog, give "Restart loop".
- **WDG-030.** The watchdog must not watch `anima-agent`. systemd or launchd keeps the agent
  running (`Restart=on-failure`, `KeepAlive`). The update of Anima pauses each watch of a program
  of Anima for the time of its step (ROL-024).

### What the hub gets

- **WDG-031.** The watchdog must report each restart to the hub: the watch, the time, the cause
  (stopped, or does not answer), the number of the attempt and the result. It must report each
  change of the state of a watch.
- **WDG-032.** A report must contain no output of the watched program and no environment value
  (LOG-007). Decision OD-WDG-8 can add the last lines of output later.
- **WDG-033.** While the hub cannot be reached, the agent must keep the reports in its buffer
  file and send them later (as MET-022). It must keep at most 24 hours of reports.
- **WDG-034.** The hub must write one event log entry for each restart, with the severity
  Warning, and one for each "Gave up", with the severity Error. The actor kind is Hub (LOG-019).
- **WDG-035.** Each watch must give a check of the new type `watch` (CHK-009). Its status is Up
  while the program runs and answers, Degraded for "Restart loop", and Down for a program that is
  stopped, that does not answer, or that is in "Gave up". Thus the device status (CHK-004), the
  uptime, the notifications (NOT-001, NOT-003) and the maintenance windows apply with no new rule.
- **WDG-036.** A watch in the state "Stopped by a person" or paused must not be Down.
- **WDG-037.** If the hub gets no report from a machine for 90 seconds, each watch of that
  machine must show "Not reported". It must not show "Running" (as MET-018).
- **WDG-038.** The operator must be able to select, for each watch, a notification for each
  restart. The default is off: a restart that succeeds is an event, and only "Restart loop" and
  "Gave up" send a notification (OD-WDG-7).

### Permission, installation and update

- **WDG-039.** The watchdog is a function of `anima-agent`. It is not a second program and not a
  container (OD-W10, OD-WDG-1). It gets its update with the agent
  ([agent-and-hub-update.md](agent-and-hub-update.md)), and it reports the version of the agent.
- **WDG-040.** The part of the agent that speaks to the hub must not hold the right to restart a
  program. On a Linux system installation, a second unit of the same program must hold that
  right. It must have no network access other than the probes. The two parts must share only two
  files: the list of watches and the reports. The second unit must check each line of the list
  against the grammar of WDG-001 and WDG-003 before it uses it. Decision OD-WDG-2 selects how the
  second unit gets the right.
- **WDG-041.** "Restart programs on this machine" is a permission of the machine, separate from
  "Takes work" (USE-004). It is off until the operator sets it on for that machine. The operator
  can set it off at the machine. A machine that is set off at the machine must stay off; the hub
  must not be able to set it on.

A user installation (a systemd user unit or a LaunchAgent, MET-019) can watch only what its user
can restart: user units, the user's own jobs, and Docker when that user can use Docker. The
installation must not ask for an administrator password (as INS-009).

## Why not a container

The request named a container as one option. This proposal does not take it, for four reasons:

1. To restart a unit of the host, a container needs the D-Bus socket of the host or its process
   namespace. To restart a container, it needs the Docker socket. Each one is equal to root on
   the machine. The container then adds no isolation.
2. Docker is one of the supervisors. If Docker stops, a watchdog in a container stops with it.
3. A Mac has no container runtime unless the operator installed one.
4. OD-W10 decided one program on each machine.

## Stages

The watchdog can come in two stages. The first needs no route from the hub to the agent.

| Stage | Where the operator makes a watch | What it needs |
| --- | --- | --- |
| 1 | At the machine: `anima-agent watch add`. The client shows the watches and the restarts, and cannot change them. | Reports from the agent to the hub: an optional member of the push of K8, or a new route (a ruling, CON-001) |
| 2 | In the client, on the Device page, and with `anima watch`. | The work route (task W1): the agent asks the hub for its list |

## In the client

The operator used the word "Seeing". No page has that name. This proposal reads it as the
category See of the work plan: the Devices page, the device detail view and the event log.

The build order is CMD-030: the hub API, the `anima` command, the MCP tool, then the client.

| Place | What the operator sees |
| --- | --- |
| Devices page, device card | The device status includes each watch (CHK-004). "Gave up" and "Restart loop" put the device in "Needs attention" (DEV-026). |
| Device detail, the detail bar | The reason sentence of DEV-020 names the program, for example "nginx was restarted 3 times in 15 minutes", with the Review link. |
| Device detail, new tab "Programs" | One row for each unit, job and container that the agent found (WDG-005): the name in the monospace font, the kind as text, the state in words, the restart rule of the supervisor in words, and the switch "Keep running". The watched rows come first. |
| A watched row | The state in words, never as a colour only: "Running", "Restarting: attempt 2 of 5, next in 30 seconds", "Stopped by a person", "Gave up" with the button "Try again", "Paused", "Supervisor not reachable", "Not reported". The count "3 restarts in 24 hours". |
| A watched row, opened | The probe (none, Docker health, HTTP, TCP), the start time, the limits with their defaults, "Pause", and the notification setting of WDG-038. One sentence says who restarts what, for example: "systemd restarts it after a failure. Anima restarts it when it does not answer." |
| Device detail, Checks tab | Each watch is a check of the type "Watch", with its heartbeat history and its uptime. It links to its row in the Programs tab. |
| Device detail, Logs tab | One event for each restart and each "Gave up" (WDG-034), with the cause and the attempt. |
| Device detail, Settings tab, Monitoring | The switch "Restart programs on this machine" (WDG-041). When the machine is set off at the machine, the switch is off and says "Set off at the machine". |
| Settings page, Notifications and Checks | The channels of a watch, as for a check (NOT-003). The defaults of WDG-014 and WDG-015 are shown and not changed here in the first version. |
| `anima` and MCP | `anima watch list`, `add`, `remove`, `pause`, `retry`. The read tools are `allow` and the change tools are `off` by default (MCP-002). |

In stage 1 the tab "Programs" has no switch. It shows the watches that the operator made at the
machine, and the command to add one.

## What depends on unfinished work

| Dependency | State today | Effect |
| --- | --- | --- |
| A ruling number for OD-W10, which changes R-115 | Decided, with no ruling number ([jobs.md](jobs.md), "Needs") | The watchdog changes a device. Without the ruling, MET-002 and R-115 forbid it. |
| The work route (tasks W1, W4 and W5 of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md)) | Not started | Stage 2 needs it. Stage 1 does not. |
| A change to K8, which is frozen (CON-001) | Not asked | The reports of WDG-031 need an optional member of the push, or `/agent/v2/`. |
| The update of the agent ([agent-and-hub-update.md](agent-and-hub-update.md)) | Proposal | Without it, each machine gets the watchdog only when a person builds and installs the agent by hand. |
| Signed releases: the first release (task R7), and an agent artefact for each target | No release is published. The builder makes only the desktop application. | A machine with no desktop application has no published agent to install. |
| The isolation design and the security review of the work route (tasks W2 and S) | Not started | The right of WDG-040 is a security subject. It needs the security reviewer. |
| Read-only discovery (R-115 "later", HOST-001) | Not built, no requirement | WDG-005 is that function. Build it one time, for this proposal and for hosted applications. |
| The client's audit list and the actor kinds in the client (LOG-020) | Built in the hub, the command line and the MCP server; not in the client | The Logs tab can show the restarts without it. |

## Conflicts with text that exists

| Text | Says | This proposal |
| --- | --- | --- |
| MET-002 | The agent only reads, and never changes the device | Add: "but a restart of a watched program (WDG-026)". It needs the ruling for OD-W10. |
| R-115, "Not taken" | "anything that changes a device" | OD-W10 changed the line on the command channel. This line needs the same ruling. |
| USE-006, JOB-011 | The agent accepts only the work types of a fixed list | A watch is not a work type: it is a rule that stays, not one item of work. The list of watch kinds is fixed in the same way (WDG-001). |
| K8, "The agent makes no other request" | Enrol, push and renew only | See "What depends on unfinished work". |
| HOST-002, HOST-008 | The operator can start, stop and update a container; each hosted application gets a check | A watch restarts only. HOST-002 stays in its proposal. HOST-008 can use the check type `watch`. |
| Design specification Section 6.4 | The check types and their identifiers | Add the type `watch` (CHK-009). |

## Open decisions

| Number | Decision | Recommendation |
| --- | --- | --- |
| OD-WDG-1 | The form of the watchdog: a function of `anima-agent`, a container, or a second program | A function of `anima-agent` (WDG-039). "Why not a container" has the reasons. |
| OD-WDG-2 | How the second unit of WDG-040 gets the right to restart on a Linux system installation: it runs as root with a strict sandbox, or it runs as its own user with a polkit rule for the named units and membership of the group `docker` | Its own user with a polkit rule, and the group `docker` only when a container is watched. State in the client that the Docker socket is equal to root on that machine. The security reviewer must accept the design first. |
| OD-WDG-3 | A process with no supervisor | Not in the first version (WDG-002). Later, Anima can offer to write a unit for it at the machine. |
| OD-WDG-4 | Stage 1 first (watches made at the machine), or wait for the work route and build stage 2 only | Stage 1 first. It gives the restarts and the reports early and needs no route from the hub to the agent. |
| OD-WDG-5 | When systemd or Docker stopped its own attempts, does the watchdog try again (WDG-024)? | Yes, with the long delays. The supervisor has stopped, so two programs do not act at the same time. |
| OD-WDG-6 | Is a watch a check of a new type (WDG-035), or does it have its own status rules? | A check. The notifications, the uptime and the maintenance windows then need no second set of rules. |
| OD-WDG-7 | Which restarts send a notification by default | Only "Restart loop" and "Gave up". Each restart is an event. The operator can set a notification for each restart of one watch (WDG-038). |
| OD-WDG-8 | Does a report contain the last lines of the output of the program? | No, in the first version (WDG-032). The output of a program can contain a secret (LOG-007). |
| OD-WDG-9 | Can an API key or an AI agent add, remove or pause a watch? | Pause and "Try again": yes, with a write key. Add and remove: no; add `addWatch` and `removeWatch` to the list of INT-003. A watch gives the right to restart a program. |
| OD-WDG-10 | macOS: watch only the jobs of the user, or also system jobs (LaunchDaemons), which need root | The jobs of the user and Docker only. The agent on a Mac is a LaunchAgent (MET-019). |
| OD-WDG-11 | The numbers of WDG-010, WDG-011 and WDG-014 to WDG-016 | As written. The operator can set the start time for each watch; the other numbers are fixed in the first version. |
| OD-WDG-12 | Docker only, or Docker and Podman | The same answer as the open decision of [hosted-applications.md](hosted-applications.md). Docker only until that is decided. |
| OD-WDG-13 | A probe to an address that is not on the machine | No (WDG-008). The hub already has checks for other addresses. |

The tier of this proposal is decision OD-W22 of the work plan.

## Tasks

G1 is in progress. The other tasks are not started. The states obey PLN-007.

| Task | Content | Needs | State | Commits |
| --- | --- | --- | --- | --- |
| G1 | A prototype on one Linux machine and one Mac: the signs of "stopped by a person" for each supervisor (WDG-025), the states to wait for (WDG-023), and the right of WDG-040. A record with measured results. | — | In progress. launchd, Docker and systemd (user manager) are measured: [g1-supervisors.md](../research/watchdog/g1-supervisors.md). Open: the right of WDG-040, which needs root on the operator's Linux machine. | — |
| G2 | The contract: the report of WDG-031, the list of WDG-005, the check type `watch`. | A ruling for OD-W10 and for K8 | Not started | — |
| G3 | The pure rules in `agent-core`: probe count, delays, limit, the states (WDG-010 to WDG-017). | G2 | Not started | — |
| G4 | The agent: read the supervisors, restart through them, the second unit, `anima-agent watch`. | G1, G3 | Not started | — |
| G5 | The hub: store the watches and the reports, the check type, the events. Hub API, then `anima`, then MCP. | G2 | Not started | — |
| G6 | The client: the tab "Programs" of stage 1, the Checks and Logs rows. | G5 | Not started | — |
| G7 | Stage 2: the list goes from the client to the machine. | The work route (W1, W4, W5) | Not started | — |
| G8 | The security review of the watchdog. | G4, G5 | Not started | — |

## Needs

- A decision on OD-W22 (the tier) and on OD-WDG-1 to OD-WDG-13.
- A ruling number in `programme.md` for OD-W10, with the change to MET-002 and to R-115.
- A ruling for the change to K8, or for a new contract (CON-001).
- The security reviewer for WDG-040 and WDG-041 (a `security` task).
- After approval: one issue with the labels of Section 4 of the work plan.
