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
| Decision 5: disk installation modes | Decided | A + B + E: install to one explicitly selected disk after erase confirmation, keep installation and boot files on that disk and preserve unselected disks; offer a live trial before installation. B is a safeguard of A, not a separate mode. Same-disk dual boot and external-drive installation are not first-version commitments. Trial persistence, partition layout and filesystem remain open. |
| Decision 6: setup surfaces | Decided in part | Add setup from within Anima after the machine boots and connects to the internet. Include an opt-in prepared, fully unattended setup path aligned with one-click installation. A standalone local-browser interface is not implied. Decision 7 provisionally selects graphical and text setup; a full desktop is not selected. Enrollment, network bootstrap, prepared-media identity and authorization to erase a disk remain open. |
| Decision 6: distribution | Decided | Standard downloadable ISO written to USB with dd or a graphical flasher. No custom flasher is required by this choice. It does not settle prepared-media settings, enrollment or automatic disk erasure. Selected ARM boards may need board-specific image formats; do not assume one ISO boots every target. |
| Decision 7: local interface | Provisional | A + B: graphical setup and installed status/recovery screen, plus text setup and recovery menus as a fallback. Setup through Anima remains in scope. The operator is discussing this in a side chat and asked to log the choice for now and revisit later. No interface framework or full desktop is selected. |
| Decision 8: hub disconnection | Decided | A + B + D: approved applications keep running and may restart under their existing policy; already-running jobs and agents may continue within existing permissions and limits; authorized local administration remains available without a reachable hub. Do not start new queued or scheduled jobs while disconnected in the first version. Grant expiry, reconnect reconciliation and local authentication still need design. External-service dependencies can still prevent a workload from working. |
| Decision 9: recovery operations | Decided | A + B + C + D: OS rollback without deliberately reverting workload data; OS repair/reinstallation preserving designated persistent data; selected app/dataset restore from available snapshots or backups; explicit factory reset removing ownership, credentials and workload data. These are separate operations. Application compatibility, preserved-state inventory, backup policy, automated recovery triggers and secure drive sanitization remain to be designed and tested. |
| Decision 10: disk encryption | Decided | C: optional, explicit opt-in at installation, off by default. The operator prioritizes remote access and unattended restart after a crash; a default setup must not wait for someone to enter a disk password locally. Explain the at-rest protection tradeoff and reboot behavior when encryption is selected. No hardware or remote unlock mechanism has been selected. |
| Decision 11: encrypted-storage unlock | Decided | A + B: local passphrase and hardware-backed automatic unlock on validated hardware. Include a recovery key stored outside the machine, as recommended in the accepted choice. Encryption stays opt-in/off by default. Owner-initiated remote unlock, network-assisted automatic unlock and removable-token unlock are deferred. TPM boot policy, key custody and update/rollback validation remain open. |
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

1. Revisit Decision 7 after the operator's side discussion. Current provisional choice: graphical setup/status plus text setup/recovery (A + B), alongside setup through Anima. The walkthrough continues; only Decision 7 needs a later review. Prepared-media authorization also remains unresolved; the ISO distribution answer did not select its policy.
2. Decision 12: which filesystem should be the default for installed OS and persistent data? Practical candidates: Btrfs, ext4, XFS, ZFS or a split OS/data design. Recommendation: Btrfs for both, with separate subvolumes and deliberate snapshot boundaries; boot partitions use firmware-appropriate formats. No selection yet. NixOS generations handle system configuration rollback independently of filesystem snapshots. Workload consistency, space exhaustion and storage-runtime compatibility need tests.
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

- 2026-11-19: Decision 5 accepted as A + B + E. The operator noted A and B overlap; recorded preservation of unselected disks as part of dedicated-disk installation. Live trial is included. Next: installation and local interaction surfaces.

- 2026-11-19: The operator requested setup within Anima once the machine boots and has internet, plus an optional prepared autonomous plug-and-play path. Recorded both intentions without assuming approval of disk-erasure rules, pairing design, local interfaces or a separate browser UI. Next: unattended-install authorization and behavior.

- 2026-11-19: The operator specified a standard downloadable ISO written with dd or a flasher application. Recorded the delivery choice without inferring acceptance of all unattended-install options. Next: close the local-interface choice; avoid repeating the delivery question.

- 2026-11-19: The operator provisionally selected A + B for Decision 7 and asked to log it while discussing the subject in a side chat. Marked it for later review and paused the walkthrough. No side-chat conclusions have been imported.

- 2026-11-19: The operator clarified that the walkthrough should continue. Decision 7 stays provisional for later review; the discussion is not paused. Next: behavior when disconnected from the Anima hub.

- 2026-11-19: Decision 8 accepted as A + B + D. Recorded offline continuity for approved apps and running work, plus authorized local administration; new queued/scheduled jobs do not start offline. Next: recovery outcomes before filesystem and partition design.

- 2026-11-19: The operator accepted all recovery operations in Decision 9. Recorded A + B + C + D. Next: encryption policy, then unlock/restart behavior and storage layout.

- 2026-11-19: Decision 10 selected C, encryption opt-in and off by default. Recorded the operator's reason: avoid physical attendance for disk unlock after a crash/reboot of a remotely managed server. Hardware-assisted or remote unlock can avoid that in some designs but is not assumed. Next: unlock methods for owners who opt in.

- 2026-11-19: Decision 11 accepted A + B: local passphrase and hardware-backed automatic unlock, with an external recovery key. Remote and network-assisted unlock are deferred. Next: filesystem choice, then partition and subvolume layout.

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

In response to Decision 5:

> Yeah B and A are kinda the same I agree with A B and E

In response to Decision 6:

> I could see setting up from within anima as a thing that can be done as well just boot it connect to the interent and it will be fine. I also wouldn't mind a D like set option which is fully automous plug and play in line with the one click option

In response to the plug-and-play follow-up:

> yeah standard download an iso and flash it on with dd or a flasher app

In response to Decision 7:

> We can do A + B also thinking about this on the side chat so log that for now and we can circle back too it later

After the pause interpretation:

> No We can keep making desicons lets keep going

In response to Decision 8:

> lets do ABD

In response to Decision 9 (recommendation A + B + C + D):

> agreed

In response to Decision 10:

> I think make it optional so opt in, cuz it can also be an issue for remote accsess if a server crashes you have to wait for someone to physiclly be there to unencrypt it

In response to Decision 11:

> A + Bagreed
