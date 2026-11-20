# Give a spare computer a new purpose

- From: the operator, 2026-11-19
- Source: direct planning chat about a minimal Anima operating system
- Status: Framed
- Serves: candidate rows 4.16 (installation), 4.7 (jobs and agent runs) and 4.9 (hosted applications); a primary row or a new row is not confirmed.
- Owner if it becomes work: a new proposal for the operating system and installation image, subject to the overlap check. Existing plans keep ownership of workloads.
- Effort: L, an estimate. Recommendation: could; priority is not decided.

## The idea

Make a minimal operating system that gives a spare computer a new purpose as an Anima-managed machine. The first version must support hosted applications, jobs and agents that execute on the machine. Start the project from scratch. NixOS is the proposed foundation. OmniXY and nascentOS are background experience, not a selected code base. This is an idea under discussion, not approval to build or release.

## Decisions recorded from the operator

| Subject | State | Choice and reason |
| --- | --- | --- |
| Intended use | Decided | Give a spare computer a new purpose. Managing its existing installation is not the purpose of this image. |
| Project starting point | Decided | Start from scratch. Do not assume a fork of OmniXY or nascentOS. Repository name and location remain open. |
| NixOS foundation | Provisional | The operator proposed NixOS as a good fit. Confirm its selection during the design review; no release or package choices are made. |
| Decision 1: first-version workloads | Decided | Hosted applications, queued jobs and agent execution are all in scope. An agent runner executes agents on the machine; it is not only a machine that an external agent can control. |
| Earlier apps-only answer | Superseded | The operator first selected useful apps, then corrected this to all capabilities and explicitly included agents and jobs. Do not use the earlier answer to exclude these from the first version. |
| Decision process | Decided | Discuss practical options and tradeoffs, give a recommendation, take the operator's answer and update this record. Allow combined answers. Record uncertainty and tests before treating an unverified choice as settled. |

Building in stages is possible. No stage order, workload runtime, agent provider, model-inference requirement or storage role has been selected. Supporting all three workload classes does not imply supporting every application or agent in the first release.

## Evidence

- Direct operator conversation, 2026-11-19; excerpts below. The operator asked for ongoing records while decisions are discussed.
- Public background repositories supplied by the operator: <https://github.com/TheArctesian/omnixy> and <https://github.com/TheArctesian/nascentOS>. Their published descriptions were read in this chat. No implementation audit, build or hardware test was performed.

## Overlap check

Checked 2026-11-19 against origin/dev at c32d866f: ownership.md, and keyword searches of register.md, deferred.md and the idea folders. No dedicated operating-system image owner was found. This is a scoped check, not an audit of every plan.

- [One-click installation](../../plans/one-click/one-click-plan.md) and design specification Section 3.11 own existing application installation (INS-001 to INS-013); a disk installer needs a distinct scope.
- [Daemon and hub updates](../../plans/daemon-and-hub-update-plan.md) own ROL-001 to ROL-052. Whole-system updates and recovery need a defined boundary with them.
- [Jobs](../../proposals/jobs.md) owns USE, JOB and ROU; [agent runs](../../proposals/agent-runs.md) owns AGN. Do not create a second scheduler or execution policy in the OS proposal.
- [Hosted applications](../../plans/hosted-applications-plan.md) and [stage 2](../../proposals/hosted-applications-stage-2.md) own HOST. The OS should provide the required environment.
- [Agent tools as extensions](../../proposals/agent-tools-as-extensions.md) owns AXT. Agent packaging, grants and isolation must agree with this owner.
- [Machine profiles](../../proposals/machine-profiles.md) owns REP-001 to REP-025. It mentions Omarchy / Omnixy as background; it does not establish a disk-installable OS owner.
- [Power and hardware health](../../proposals/power-health.md) owns PWR, HWH and HWI. Hardware checks and unattended power behavior must agree with it.

## Narrowest version, and when to stop

- Candidate proof, not yet approved: install on one spare computer, pair it with Anima, run a persistent app, a queued job and an agent, then test update failure and recovery. It must cover all three agreed workload classes.
- Reconsider the design if it cannot preserve user data during recovery or cannot meet the agreed maintenance experience. Hardware limits, acceptance criteria and stop conditions need operator decisions.

## Decision method

For each decision: state the user outcome, previous constraints, practical options and tradeoffs, the recommended choice and its downside, then record the operator's answer. Choices can be combined where compatible. Use Decided, Provisional, Deferred and Superseded states. Give provisional technical choices a test where needed. Approval of an idea is separate from permission to build.

Review linked choices together: storage layout, filesystem, encryption, persistent state, updates and recovery. Before a proposal is ready, walk through installation, normal use, lost connectivity, full storage, failed updates, disk failure and transfer to another owner.

## Open questions

1. Decision 2: which maintenance controls should the owner have? Options can be combined: automatic routine maintenance; approval of disruptive changes; advanced access through SSH or local console; manual maintenance. Recommendation pending the operator's answer: automatic routine maintenance, approval of disruptive changes and optional advanced access.
2. Which hardware and installation targets should the first version support? Discuss CPU architecture, firmware and the installation experience separately after the maintenance goals.
3. Which priority row and owner should the proposal use? Resolve after the scope and dependencies are clear.

## Topics still to discuss

These are discussion topics, not selected requirements or a task breakdown:

- Hardware targets, minimum resources and UEFI or legacy boot.
- Whole-disk installation, dual boot, live trial, offline installation and installer interface.
- Local status screen, headless use and optional desktop.
- Enrollment, local administration, offline behavior and ownership transfer.
- Partitions, root and data filesystems, swap, extra disks, snapshots, quotas and compression.
- Encryption, unattended unlock and recovery keys.
- Persistent state, reset, failed-boot recovery and rescue media.
- Base software, Anima components, container and VM runtimes, drivers and diagnostics.
- Networking, private connectivity, discovery, firewall and application exposure.
- Workload isolation, resource limits, GPU access and agent permissions.
- Nix versus Anima configuration ownership and manual customization.
- Release builds, package sources, signatures, update timing and rollback.
- Backups, application data recovery and moving to a replacement machine.
- Sleep, laptop lids, battery and thermal health, power loss and wake-on-LAN.
- Hardware validation, image distribution, support and security maintenance.

## Reviews

- 2026-11-19: First record. Captured the fresh-project direction and corrected Decision 1 to include applications, jobs and agents. No filesystem, partition layout or software stack is selected. The next discussion is the maintenance model, with combined answers allowed.

## Source text

Selected direct operator messages, unchanged:

> More so this computer is spare give it a new purpose

> Was playing a lot around with nix os and think it would be perfect for this https://github.com/TheArctesian/omnixy this is what i built as a omarchy clone and was starting to think about this with https://github.com/TheArctesian/nascentOS

> i think i would want to start from scratch on this

> Wait no I want all of it

> Like should also be an agent and job runner

> Okay write these up in the planning docs while we go through it once thats done give me the next descions (allow for multi slect next time)
