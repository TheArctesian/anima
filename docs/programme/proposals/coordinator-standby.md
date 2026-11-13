# Proposal: a standby copy of the coordinator, and how to move it

From: the planning session of 2026-11-12. The operator agreed the idea in a side chat: one
coordinator, one warm standby, a switch that the operator starts.
Status: a proposal for a ruling. Not approved. It conflicts in part with issue #2 (see below).
Map: [work-plan.md](../../work-plan.md), principles ETH-001 to ETH-005, decision OD-W9.

## Problem

The hub, the gateway and PostgreSQL run on one VPS (OD-9). The only protection is the daily
backup (DB-007, VPS-006). If the VPS stops, the operator has no device list, no sessions through
the hub and no checks until a restore is done by hand. No procedure for that restore is written.
The same procedure is what moves the coordinator to a different host (ETH-002).

## Proposal

- **FND-001.** The operator must be able to name one machine as the standby.
- **FND-002.** The standby must receive a continuous copy of the database. Use PostgreSQL
  streaming replication.
- **FND-003.** The standby must hold the encryption key (SEC-004). Thus the standby needs the
  same protection as the coordinator.
- **FND-004.** The operator must be able to make the standby the coordinator with one command.
  The command must also move the name of the coordinator on the VPN (NET-005).
- **FND-005.** The procedure must not start automatically (ETH-004).
- **FND-006.** The operator must be able to restore a daily backup on a new machine and continue
  (VPS-006). This is the procedure to move the coordinator.
- **FND-007.** Each client can keep a local copy of data that it showed recently. The client must
  show the age of this data (NET-004). IOS-003 already requires this copy for iOS and permits it
  for the desktop clients.

FND-006 does not depend on the conflict below. It can be approved alone.

## Conflict with issue #2

Issue #2 (`priority: later`, operator direction of 2026-11-05) plans a different future:

| Subject | This proposal | Issue #2 |
| --- | --- | --- |
| Where the data is | One home and one standby (ETH-003) | A local copy on each client and each site agent. The hub is the anchor, the archive and the backup. |
| When the hub is down | Clients show old data with its age (FND-007). The operator can switch to the standby. | Sites continue their checks. The operator reads and edits locally. Nodes sync directly on the VPN until the hub returns. |
| Who can write | The coordinator only | Each node writes its own signed chain. High-risk changes need more than one signature. |
| Agreement between machines | None | Quorum checkpoints in the background (phase L4) |
| Automatic election | No (ETH-004) | No. One anchor stays. |
| Cost | Small: replication and one command | Large: a sync engine, node identities, a ledger, phases L0 to L5 |

The two agree on one anchor and on no election. They do not agree on ETH-003 ("one copy") and
ETH-005 ("data stays at its home"), and version 0.1 of the work plan listed "a database that is
spread across machines" as not planned.

The operator must select one of these (OD-W9):

1. **Standby now, issue #2 later.** Build FND-001 to FND-007 as the first stage. Write ETH-003
   and ETH-005 as rules for now, not for always. Issue #2 stays `priority: later` and later
   replaces them.
2. **Standby only.** Close issue #2, or decrease it to its phase L0 (a hash chain for the audit
   log), which needs no copies.
3. **Issue #2 only.** Do not build the standby. Keep only FND-006 (restore and move) until the
   sync engine of issue #2 gives each node its copy.

## Needs

- A ruling on OD-W9.
- A decision on which machine is the standby, and where it is. A standby at home puts the
  credentials and the key at a second place (OD-9, and OD-E1 in #27).
- VPS and Headscale changes need the operator's confirmation in chat (VPS-005).
- After approval: one issue with the label `infrastructure` and a priority label.
