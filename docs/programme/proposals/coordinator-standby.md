# Proposal: copies of the coordinator database, and how to move the coordinator

From: the planning session of 2026-11-12.
Status: a proposal for a ruling. The operator decided its direction on 2026-11-12 (OD-W9, see
"Decision"). The requirements below are not yet approved one by one.
Map: [work-plan.md](../../work-plan.md), principles ETH-001 to ETH-005.
Related: [bulk-data-placement.md](bulk-data-placement.md) for large files and backups.

## Problem

The hub, the gateway and PostgreSQL run on one VPS (OD-9). The only protection is the daily
backup (DB-007, VPS-006). If the VPS stops, the operator has no device list, no sessions through
the hub and no checks until a restore is done by hand. No procedure for that restore is written.
The same procedure is what moves the coordinator to a different host (ETH-002).

## Decision (OD-W9, operator, 2026-11-12)

The operator selected one home with copies, not a copy on each node that syncs between nodes
(issue #2). The operator's words:

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

Issue #2 is not changed by this file. Its parts that sync between nodes (phases L1 to L4) do not
agree with this decision. Its phase L0, a hash chain for the audit log, needs no copies and can
stay. The operator decides what to do with the issue.

## Proposal

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

## Open points

- The standby must send the notification of FND-004 when the home is down. Thus a standby needs
  its own path to the notification channels.
- The "set time" and "set share" values of FND-004, FND-009 and FND-010.
- A machine that is permitted as a standby holds the device credentials. A standby at home puts
  them at a second place (OD-9).
- The size of the database on the VPS is not measured. The retention periods limit it (DB-009).

## Needs

- A ruling number in `programme.md` for the decision above.
- A security review: the standby holds the key, and the switch moves the VPN name.
- VPS and Headscale changes need the operator's confirmation in chat (VPS-005).
- After approval: one issue for FND-006 (restore and move), and one for the standbys, with the
  label `infrastructure` and a priority label.
