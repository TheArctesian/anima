# Proposal: bulk data on the operator's machines, in encrypted pieces

From: the planning session of 2026-11-12. The operator asked that data be spread to other
machines by their capacity, and that the VPS not carry the storage: "it also means a lot of
storage burden on the hub no which is a VPS which is expensive". The operator agreed this form:
"okay yeah lets do that".
Status: a proposal for a ruling. Its direction is decided (OD-W9). The requirements are not yet
approved one by one. It changes FIL-001 of the extensions plan.
Map: [work-plan.md](../../work-plan.md), principles ETH-003 and ETH-005.
Related: [coordinator-standby.md](coordinator-standby.md) for the live database.

## Problem

The extensions plan keeps all files in a Docker volume on the hub (FIL-001). The hub is on a
VPS, where disk space costs much more than on the operator's own machines. Photos, documents,
export archives, database backups and model files will become large. The operator's machines
have free space, but different amounts, and some are not always on.

## Terms

| Term | Meaning |
| --- | --- |
| Bulk data | Files and backups. Not the rows of the live database. |
| Piece | One file, or one part of a large file, after it is sealed. Its name is its content hash. |
| Store machine | A machine of the operator that keeps pieces |

## Proposal

- **STO-001.** The hub must not keep bulk data on its own disk, apart from a small cache. The
  hub keeps the index: which piece is on which machine.
- **STO-002.** The hub or the client must seal each piece before it leaves (FIL-002, VLT-001).
  A store machine holds only ciphertext and has no key. Thus a store machine needs no permission
  like FND-003.
- **STO-003.** A policy must select the store machines for each piece. It must be a pure
  function of the free space, the site and the share of time that each machine is on. The
  operator does not select the machines.
- **STO-004.** Each piece must have at least two copies on machines that are always on, at two
  different sites when that is possible. Copies on a laptop or a phone are additional copies
  and do not count.
- **STO-005.** The operator must be able to set a storage limit for each machine. The default
  must leave free space on the machine (as the disk share of CAP-010 does for model files).
- **STO-006.** When a store machine stops for longer than a set time, or its disk becomes full,
  the policy must make the missing copies on other machines.
- **STO-007.** The hub must check each copy against its hash at intervals, and replace a copy
  that is damaged or missing.
- **STO-008.** When the operator opens a file, the hub must get the piece from a machine that is
  on. If no machine with the piece is on, the client must say which machine it waits for.
- **STO-009.** The client must show, for each machine, the space that Anima uses, and the pieces
  that have fewer than two copies.
- **STO-010.** When the operator deletes a record, the hub must delete its pieces on all
  machines (FIL-004).
- **STO-011.** The daily database backup (VPS-006) is bulk data. It goes to the store machines as
  pieces. IOS-004 is one case of this.

## What this changes in other plans

| Item | Change |
| --- | --- |
| Extensions plan FIL-001 | The file store is on the store machines, not in a volume on the hub. Issue #8 owns the file store. |
| OD-E1, issue #27 (where Class B data lives) | A recommendation: sealed files live on the operator's machines; the VPS holds the database rows and the index. The rows of Class B data stay on the hub, so #27 is not closed by this. |
| Design specification VPS-006 | The backup goes to more than one site. |

## Open points

- A store machine must receive and send pieces. That is new work for `anima-agent`, so OD-W10
  (work that the hub gives to the agent) applies. Until OD-W10 is decided, only the hub's own
  machine and the standbys can be store machines.
- A copy outside all sites of the operator (object storage, for example R2) is optional. It is a
  rented service, so the operator decides.
- Model files are large and are not secret. They can use the same placement without sealing, or
  stay with the compute plan (PULL-001 to PULL-010). Decide with C1 (#34).
- The size of a piece, and the "set time" of STO-006.

## Needs

- A ruling number in `programme.md`.
- The OD-W10 decision.
- A change to FIL-001 before the engine builds the file store (#8). The engine is
  `priority: foundation`, so this proposal must be decided before E0-5 starts.
- After approval: one issue with the labels `infrastructure`, `security` and a priority label.
