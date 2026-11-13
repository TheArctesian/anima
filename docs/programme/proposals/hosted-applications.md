# Proposal: hosted applications (containers that run all the time)

From: the planning session of 2026-11-12. The operator wants easy self-hosting of published
applications on the owned machines: the container management part of Portainer, not the build
pipeline of Coolify.
Status: a proposal for a ruling. Not approved. It needs the work route of [jobs.md](jobs.md),
whose direction is decided (OD-W10).
Map: [work-plan.md](../../work-plan.md), category Use.

## Problem

The operator runs published applications in containers on different machines and manages each
one with SSH. Anima does not show them, cannot start or update them, and does not know which
machine has the data. Spec Section 1.1 says that a later version may run containers (R-109).

## Proposal

A hosted application is a container that runs all the time on one machine. Anima installs
published applications. Anima is not a build platform.

- **HOST-001.** The operator must be able to see the containers on each machine, with their
  state and their logs.
- **HOST-002.** The operator must be able to start, stop and update a container.
- **HOST-003.** The operator must be able to install an application from a recipe. A recipe is a
  Docker Compose file.
- **HOST-004.** Each hosted application has one home machine (ETH-003).
- **HOST-005.** An application that keeps data must have a copy of its data on a second machine.
- **HOST-006.** The operator must be able to move an application to the machine that has the
  copy. Anima must not move it automatically (ETH-004).
- **HOST-007.** An application that keeps no data can start on a different machine when its home
  stops.
- **HOST-008.** Each hosted application must get a check (spec Section 6.4) and a web interface
  entry (WEB-001).
- **HOST-009.** Anima does not build source code, supply a reverse proxy, or make preview
  deployments. Use Vercel or Coolify for these.

HOST-001 is read-only. It can come first and alone: R-115 lists "read-only service discovery
(systemd, Docker, listeners, cron and timers)" as later work, and it needs no new permission.
HOST-002 to HOST-007 change a device, so they need the work route of [jobs.md](jobs.md).

## Work that already exists

- Issue #1, "Ideas from Coolify (hub)": the rules that make one status from many containers,
  and the image and package scan.
- Issue #1, "Ideas from Coolify Sentinel": how to calculate container metrics.
- Spec Section 6.4: the Docker container check, and CHK-014 for restart loops.

## Needs

- The work route of [jobs.md](jobs.md) for all of this except HOST-001.
- A decision on how the data copy of HOST-005 is made: a volume backup, or replication that the
  application supplies. [bulk-data-placement.md](bulk-data-placement.md) can hold the backup.
- A decision on Docker only, or Docker and Podman.
- After approval: one issue for HOST-001 and HOST-008, and one for the rest, with the label
  `priority: later`.
