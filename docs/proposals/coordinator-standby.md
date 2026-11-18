---
kind: Proposal
state: Direction decided
evidence: Direction decided (OD-W9). No requirement is approved. R-140 (2026-11-17): the operator starts fresh on the hosted hub, so FND-006 moves no data to the pool. R-141 names task F1 (FND-006 for a hub on a server), which is In progress. R-157 (2026-11-18): the standbys are deferred, and FND-007 is approved for the application on a computer (task F2, built, waits for HT-030).
---

# Proposal: copies of the coordinator database, and how to move the coordinator

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the planning session of 2026-11-12.
Status: the header of this document holds the state (PLN-020). History: a proposal for a ruling. The operator decided its direction on 2026-11-12 (OD-W9, see
"Decision"). The requirements below are not yet approved one by one.
Map: [work-plan.md](../work-plan.md), principles ETH-001 to ETH-005.
Related: [bulk-data-placement.md](bulk-data-placement.md) for large files and backups.

## Problem

The hub, the gateway and PostgreSQL run on one VPS (OD-9). The only protection is the daily
backup (DB-007, VPS-006). If the VPS stops, the operator has no device list, no sessions through
the hub and no checks until a restore is done by hand. No procedure for that restore is written.
The same procedure is what moves the coordinator to a different host (ETH-002).

## Decision (OD-W9, operator, 2026-11-12)

The operator selected one home with copies, not a copy on each node that syncs between nodes
([GH-2](../work-items/device-manager/GH-2.md)). The operator's words:

- "lets go with the one in the side chat as the every machine has alocal copy doesn't really
  make sense as different computer have different capacities"
- "i don't want to choose it it should be as hands off as possible and policy not agent makes a
  decison, maybe at least 2 copies"
- On a switch that needs one confirmation, a per-machine key switch, and encrypted pieces of
  bulk data on other machines: "ya i don't mind that", then "okay yeah lets do that".

What this means:

| Subject | Decided |
| --- | --- |
| Where the live database is | One home, and standby copies that a policy places. The operator does not select the machines. |
| Number of copies | The home and at least two standbys, when enough permitted machines exist |
| Who can write | The home only. A standby is read-only until it becomes the home. |
| A new home | The hub finds the failure and selects the best standby. The operator confirms one time. No machine becomes the home without that confirmation. |
| Large data | Not in the database copies. See [bulk-data-placement.md](bulk-data-placement.md). |
| Each client | A small read-only copy of recent data, with its age |

The operator then made [GH-2](../work-items/device-manager/GH-2.md) smaller (2026-11-12): only its phase L0 stays, a hash chain for
the audit log, which needs no copies. Its phases L1 to L5 are withdrawn.

The operator also set the order: FND-006 (restore and move) comes directly after the device
manager is accepted. The standbys come later, with the extensions.

## Proposal

State of the requirements (R-141, R-157): FND-006 is approved for a hub on a server. FND-007 is approved for
the application on a computer. FND-001 to FND-005 and FND-008 to FND-012 are not approved: they are deferred,
because a hosted hub keeps its database in the pool (R-130).

### Which machines can hold a copy

- **FND-001.** A policy must select the standby machines. The operator must not have to select
  them.
- **FND-003.** A standby holds the encryption key (SEC-004), so it can read all data. Thus a
  machine can be a standby only after the operator permits it one time, at that machine. This
  is the only manual step. The default is off.
- **FND-008.** The policy must be a pure function. Its inputs are, for each permitted machine:
  its site, its free disk space, the share of time that it is on, and the delay of its copy.
- **FND-009.** The policy must prefer a machine at a site that is not the site of the home. It
  must prefer two standbys at two different sites. It must not select a machine that is on for
  less than a set share of the time.
- **FND-010.** When a standby stops for longer than a set time, the policy must select a
  replacement and the hub must start the copy there. The hub must write this to the event log.

### The copies

- **FND-002.** Each standby must receive a continuous copy of the database. Use PostgreSQL
  streaming replication. The target is the home and two standbys.
- **FND-011.** The client must show the home, each standby, the delay of each copy, and a
  Degraded state when there are fewer than two standbys. The hub must send a notification when
  this state starts (NOT-001).

### A new home

- **FND-004.** When the home does not answer for a set time, a standby must select the best
  standby (the smallest delay) and ask the operator to confirm the switch with a notification.
  After the confirmation, one command does the switch. The command must also move the name of
  the coordinator on the VPN (NET-005).
- **FND-005.** No standby must become the home without the confirmation of the operator
  (ETH-004). Two sites that lose the VPN link can each think that the other is down.
- **FND-012.** The old home must not accept writes when it returns. It must become a standby or
  stop, and the hub must tell the operator.

### Restore and move

- **FND-006.** The operator must be able to restore a daily backup on a new machine and continue
  (VPS-006). This is the procedure to move the coordinator. It does not depend on the standbys
  and can be built first.

### Clients

- **FND-007.** Each client must keep a local read-only copy of the data that it showed recently:
  devices, checks, recent metrics and the event log. The size of the copy must fit the device.
  The client must show the age of this data (NET-004). The copy must not contain a credential.
  IOS-003 already requires this copy for iOS.

## The hosted hub and FND-006 (R-130, R-140)

The operator will use only the hosted version (R-130). This changes FND-006:

- The pool makes and restores the backup of a hosted hub. That work belongs to the pool service in the
  private repository (HMD-014). FND-006 does not include it.
- The operator starts fresh on the hosted hub (R-140). The data of the hub on the VPS is not moved.
  Each machine joins the hosted hub again.
- What exists for a hub on a server: a daily checked dump, a copy at a second site, `restore.sh` and a
  monthly restore check ([deploy/backup/README.md](../../deploy/backup/README.md)). `anima-database` can
  write, check and restore a backup of a hub on the operator's computer. Not written: the procedure for a
  new machine, with the encryption key, the database passwords and the Headscale database, which the dump
  does not hold.
- A move of a hub between a server, a computer and the pool with one file is HMD-016 of
  [hosted-mode.md](hosted-mode.md). It is not part of FND-006.

## The copy in the application on a computer (FND-007, R-157, task F2)

- The copy is kept at one place: where the application sends a request to the hub (`apps/client/src/lib/hub/recent/`).
  When the hub answers a query of the list, the application keeps the answer. When the hub does not answer, the
  application gives the kept answer of the same query, with the same variables, for the same hub.
- The list of queries: the devices and one device, the tags and the sites of the device list, the metrics of a
  device, and the event log. A query is added to the list only when its answer holds no credential. The answer of a
  mutation is never kept.
- The checks of a device are not kept. The settings of a check can hold a credential: a push path, a header, a body
  or a URL. The device list and the device still show the status that the checks gave. To keep the checks, the hub
  must first give them without these values; that is not planned.
- The event log asks from a moment that moves with the clock. The copy does not use that moment, so the copy of the
  last hour, of the last day and of the last week of one filter are one copy: the newest answer.
- The copy is read-only. A change that the operator starts while the hub does not answer fails as before.
- The age: the banner "Not on the network" shows the age of the oldest copy on the screen, also after a start of the
  application with no hub (NET-004). When the hub answers again after a copy was shown, the page loads again, so
  that no old data stays on the screen with no age. An open session stays open. Text in a form of that page is lost.
- A chart of metrics from a copy that is older than its range ends at the newest point of the copy.
- The size: at most 100 answers, each of at most 1 MB of text. The oldest answers go first. A copy older than 30 days
  is not shown.
- The place: the IndexedDB store of the web view of the application, on this computer. It is not encrypted by the
  application. Sign-out deletes it, and so does "Leave this hub". If the delete fails, the application says so. The
  application also deletes it when the hub refuses the access token. An answer that arrives after the delete is not
  kept.
- Not done: a hub at an `http` address on this computer gets no copy, because the application does not read its
  token when no program listens there. "Load more" with no hub can add an older page below a newer one. The
  IndexedDB store has no automatic test (PLN-014, HT-030).
- A review by agents on 2026-11-18 found these defects in the first build, and they are corrected: the copy held the
  settings of checks, the event log was never given from the copy, and an old copy stayed on the screen with no age
  when the hub answered again.
- Only the application on a computer keeps the copy. A browser keeps none: the hub serves its page, and the computer
  can be a shared one. The phone applications keep none here: IOS-003 and the mobile plan own that copy.
- The hub has no new function, so there is no new `anima` command and no MCP tool (CMD-030).

## Open decisions

| Number | Decision | Recommendation | Result |
| --- | --- | --- | --- |
| OD-FND-1 | When the operator goes to the hosted hub: move the data of the hub on the VPS, or start fresh? | Start fresh. Each machine joins again in the two cases, and the pool has no import. | Decided (R-140, operator, 2026-11-17): start fresh. |
| OD-FND-3 | The standby copies (FND-001 to FND-005, FND-008 to FND-012): build them, or defer them now that the hub is hosted? | Defer. A hosted hub keeps its database in the pool; a standby at a home needs the database and its key outside the pool. | Decided (R-157, operator, 2026-11-18): deferred, not approved. |
| OD-FND-4 | The read-only copy in the application (FND-007): build it now for the application on a computer? | Yes. It is the part that helps when a hosted hub does not answer. | Decided (R-157, operator, 2026-11-18): yes, task F2. |
| OD-FND-2 | Is FND-006 now only the procedure for a person who runs a hub on a server: one document from the scripts that exist, tested by a restore on a new, empty setup in Docker, with no change to the VPS (task F1)? | Yes. | Decided (R-141, operator, 2026-11-17): yes. |

## Tasks

| Task | Content | Needs | State | Commits |
| --- | --- | --- | --- | --- |
| F1 | FND-006 for a hub on a server: the procedure `docs/procedures/restore-and-move.md`, proved by a restore on a new, empty setup in Docker. | R-141 names it | In progress | Built, not checked on a real server: [restore-and-move.md](../procedures/restore-and-move.md), with its proof in Docker, commit `1b135102`. The full gates passed on 2026-11-17 (21 of 21). It waits for HT-024 (PLN-014). |
| F2 | FND-007 for the application on a computer: a read-only copy of the data that the application showed recently, with its age and with no credential. The phone applications are not in this task (IOS-003, mobile plan). | R-157 names it | In progress | Built in 53af82a1 and corrected after a review in 3b46dfef, not checked in the installed application: `apps/client/src/lib/hub/recent/`. The tests give the copy when the hub does not answer, the age in the banner, the limits, the delete at sign-out and when the hub refuses the token, and the load of the pages when the hub answers again. The store was run in Chromium on 2026-11-18, not in the web view of the application. It waits for HT-030 (PLN-014). |

## Open points

- The standby must send the notification of FND-004 when the home is down. Thus a standby needs
  its own path to the notification channels.
- The "set time" and "set share" values of FND-004, FND-009 and FND-010.
- A machine that is permitted as a standby holds the device credentials. A standby at home puts
  them at a second place (OD-9).
- The size of the database on the VPS is not measured. The retention periods limit it (DB-009).

## Needs

- A ruling number in `docs/rulings.md` for the decision above.
- A security review: the standby holds the key, and the switch moves the VPN name.
- VPS and Headscale changes need the operator's confirmation in chat (VPS-005).
- After approval: one work item for FND-006 (restore and move), and one for the standbys, in
  `docs/work-items/` (PLN-012), each with a tier.
