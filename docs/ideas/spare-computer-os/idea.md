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
| Decision 2: maintenance | Decided | A + B + C: automatic approved routine upkeep and problem reporting; owner-defined maintenance windows and control over disruptive actions; optional SSH or local administrative access. Fully manual maintenance mode is out of the first version. Exact update, reboot, access and configuration policies remain to be designed. |
| Decision 3: hardware families | Decided in part | A + B + C: 64-bit Intel/AMD PCs, Intel Macs and selected ARM64 boards or mini PCs. Exact supported models need selection and testing. Apple Silicon is deferred to a later release (follow-up choice C). Keep its possible integration in the design; it is not an experimental first-release target. Older 32-bit PCs were not selected. Firmware and resource minimums remain open. |
| Decision 4: boot paths | Decided | A + B + C: 64-bit UEFI, legacy BIOS for 64-bit PCs and board-specific boot for selected ARM64 hardware. This does not add 32-bit CPU support. Bootloader choice, Secure Boot policy, exact hardware targets and installation/recovery validation remain open. |
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

1. Decision 5: which disk installation modes should the first version offer? Allow combined choices: dedicate a selected disk; preserve an existing OS on another disk; share a disk with an existing OS; install to an external drive; offer a live trial with no installation. Recommendation: dedicate a selected disk, preserve other disks and offer a live trial. External installation is a possible additional target; same-disk dual boot is not recommended for the first version. No answer yet.
2. Which bootloader, Secure Boot policy, resource minimums and exact hardware models should be selected? Discuss after installation and recovery goals; no selection yet.
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

## Apple Silicon research

Read 2026-11-19 at the operator's request:

- [nixos-apple-silicon](https://github.com/nix-community/nixos-apple-silicon) provides modules and packages for bare-metal NixOS using Asahi Linux support. This is an existing implementation route, not merely Nix packages running on macOS.
- Its [installation guide](https://github.com/nix-community/nixos-apple-silicon/blob/main/docs/uefi-standalone.md) documents a customized NixOS ISO and preparation from macOS through the Asahi installer. Apple boot and recovery partitions need special handling. A generic PC whole-disk erase flow cannot be assumed suitable.
- [Asahi feature support](https://asahilinux.org/docs/platform/feature-support/overview/) is listed by chip generation and device features. Support must be checked for each selected model; the existence of a page does not prove that model works.
- Proposed integration: share the ARM64 application configuration where possible, with Apple-specific boot, kernel, firmware and installation modules. This is a design recommendation, not a tested result. No claim is made about GPU inference, all agent packages or automatic recovery on these Macs.

## Reviews

- 2026-11-19: First record. Captured the fresh-project direction and corrected Decision 1 to include applications, jobs and agents. No filesystem, partition layout or software stack is selected. The next discussion is the maintenance model, with combined answers allowed.
- 2026-11-19: The operator accepted the recommendation for Decision 2 (A + B + C). Recorded routine automation, owner control over disruption and optional advanced access. Fully manual mode is not in the first version. Next: hardware families, with combined answers allowed.
- 2026-11-19: The operator selected A + B + C for hardware and requested Apple Silicon research. Recorded the existing NixOS/Asahi route and left the Apple Silicon release commitment open.

- 2026-11-19: The operator chose C for the Apple Silicon follow-up: later release. The first release targets PCs, Intel Macs and selected ARM64 machines. Next: boot paths.

- 2026-11-19: The operator accepted all three boot paths (Decision 4, A + B + C). Their comment that these are easy to support is not a hardware validation result. Next: disk installation modes.

## Source text

Selected direct operator messages, unchanged:

> More so this computer is spare give it a new purpose

> Was playing a lot around with nix os and think it would be perfect for this https://github.com/TheArctesian/omnixy this is what i built as a omarchy clone and was starting to think about this with https://github.com/TheArctesian/nascentOS

> i think i would want to start from scratch on this

> Wait no I want all of it

> Like should also be an agent and job runner

> Okay write these up in the planning docs while we go through it once thats done give me the next descions (allow for multi slect next time)

In response to the Decision 2 recommendation A + B + C:

> ueah agreed

In response to Decision 3:

> yeah ABC silicon mac's might be able to be supported by nix though might be worth search it up

In response to the Apple Silicon follow-up:

> We can leave apple silicon for a later release so C

In response to Decision 4 (recommendation A + B + C):

> yeah they are easy to support
