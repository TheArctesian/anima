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
| Decision 6: setup surfaces | Decided in part | Add setup from within Anima after the machine boots and connects to the internet. An opt-in prepared unattended path was initially requested; Decision 26 later selects A only for the first version, leaving that path for later discussion. A standalone local-browser interface is not implied. Decision 7 provisionally selects graphical and text setup; a full desktop is not selected. Enrollment, network bootstrap, prepared-media identity and authorization to erase a disk remain open. |
| Decision 6: distribution | Decided | Standard downloadable ISO written to USB with dd or a graphical flasher. No custom flasher is required by this choice. It does not settle prepared-media settings, enrollment or automatic disk erasure. Selected ARM boards may need board-specific image formats; do not assume one ISO boots every target. |
| Decision 7: local interface | Provisional | A + B: graphical setup and installed status/recovery screen, plus text setup and recovery menus as a fallback. Setup through Anima remains in scope. The operator is discussing this in a side chat and asked to log the choice for now and revisit later. No interface framework or full desktop is selected. |
| Decision 8: hub disconnection | Decided | A + B + D: approved applications keep running and may restart under their existing policy; already-running jobs and agents may continue within existing permissions and limits; authorized local administration remains available without a reachable hub. Do not start new queued or scheduled jobs while disconnected in the first version. Grant expiry, reconnect reconciliation and local authentication still need design. External-service dependencies can still prevent a workload from working. |
| Decision 9: recovery operations | Decided | A + B + C + D: OS rollback without deliberately reverting workload data; OS repair/reinstallation preserving designated persistent data; selected app/dataset restore from available snapshots or backups; explicit factory reset removing ownership, credentials and workload data. These are separate operations. Application compatibility, preserved-state inventory, backup policy, automated recovery triggers and secure drive sanitization remain to be designed and tested. |
| Decision 10: disk encryption | Decided | C: optional, explicit opt-in at installation, off by default. The operator prioritizes remote access and unattended restart after a crash; a default setup must not wait for someone to enter a disk password locally. Explain the at-rest protection tradeoff and reboot behavior when encryption is selected. No hardware or remote unlock mechanism has been selected. |
| Decision 11: encrypted-storage unlock | Decided | A + B: local passphrase and hardware-backed automatic unlock on validated hardware. Include a recovery key stored outside the machine, as recommended in the accepted choice. Encryption stays opt-in/off by default. Owner-initiated remote unlock, network-assisted automatic unlock and removable-token unlock are deferred. TPM boot policy, key custody and update/rollback validation remain open. |
| Decision 12: default filesystem | Decided | A: Btrfs for installed OS and persistent data, with separate subvolumes. The operator accepted it as sufficient for the current design. Use NixOS generations for system software/configuration rollback; data restoration remains separate. This selects a filesystem, not an application database. Exact partitions, subvolumes, snapshot policy, compression and additional-drive layouts remain open. |
| Decision 13: disk capacity layout | Decided | A: one shared Btrfs filesystem with separate subvolumes, plus hardware-appropriate boot partitions. System and workload data share free capacity. Space limits, cleanup and snapshot retention must be designed; subvolumes do not by themselves reserve system capacity. Data-preserving OS repair must not format the shared filesystem. Separate system/data partitions, LVM and a multi-disk installation are not selected. |
| Product direction: appliance experience | Decided direction (relayed) | Purpose-built appliance with sensible defaults and limited supported host configuration. Prioritize install-and-work simplicity; accept reduced configuration freedom. General-purpose development environments can use OmniXY, Omarchy or other distributions. Caddy, SteamOS and Ubuntu are experience analogies, not selected architectures or a request for a desktop. Existing decisions and optional administrator access remain in force. |
| Decision 14: root behavior | Decided | C: read-only system root during normal operation, with designated writable state for settings, device identity, logs, application data and job workspaces. Updates and repairs use a controlled maintenance path. Keep optional administrator access for diagnostics and recovery. This does not make workload storage read-only or select a separate physical disk. Exact NixOS implementation, writable mounts and compatibility need validation. Reset-on-every-boot root is not selected. |
| Decision 15: automatic OS-update recovery | Decided | A: attempt the previous known-good system when the new OS fails boot or required local health checks. Bound retries and enter recovery if both versions fail. Hub unreachability alone does not trigger rollback. Check the daemon and local network configuration; report reconnection failure through available channels and retain manual rollback. Do not silently revert workload data. Bootloader support, watchdog behavior, health criteria and retry limits require implementation and tests. |
| Decision 16: workload and host dependencies | Decided | A + B + C: separate workload environments by default; curated host integrations; supported custom host packages and services through a controlled configuration mechanism. This expands the supported customization scope beyond curated profiles. Keep the normal system root read-only; do not infer unrestricted imperative host writes or blanket agent authority. Custom configuration must participate in update, validation and recovery design. Exact accepted configuration surface and authorization remain open. |
| Decision 17: agent authority for host changes | Decided | A by default, with B available: owner approval for each host change unless an explicit bounded standing grant covers it. Grants must have precise scope and be revocable. Broad administrator-agent mode C is not selected. Use the existing Anima permission/execution model; do not create a separate OS permission system or infer bypass of maintenance windows. Reconcile the proposal with existing owner requirements before implementation. |
| Decision 18: resource priorities | Decided | A + D: service-first by default, with owner-selectable service-first, worker-first and balanced profiles. Reserve capacity for OS, remote management and recovery under every profile. Jobs use remaining capacity under service-first, and admission may wait. Explain effects of profile changes; do not silently kill running work. Existing scheduler/workload owners define enforcement, resource limits and admission behavior. |
| Decision 19: automatic storage cleanup | Decided | A + B + C: eligible regenerable caches/unused images; history expired under explicit retention; completed job workspaces/artifacts explicitly marked disposable with a configured lifetime. Protect active/pinned workloads, designated recovery points and the previous known-good system. Preserve persistent app data, user files, credentials and unmarked job outputs. If permitted cleanup is insufficient, alert and restrict new allocations rather than broaden deletion authority. Exact eligibility, retention and pressure thresholds remain open. |
| Decision 20: backup destinations | Decided | A + B + C: another explicitly authorized enrolled Anima machine, user-provided network storage, and a deliberately selected separate attached or secondary drive. Do not automatically use or format unselected disks. User-provided cloud/object storage and Anima-managed backup storage are not selected for the first version. Expose destination configuration and last successful backup; define capacity, authentication, retention and restore validation. Same-disk snapshots are not disk-failure backups. |
| Decision 21: backup coverage | Decided | A + D: after destination/policy approval, cover managed persistent data and configuration by default, with optional owner-selected extra paths. Include persistent agent/job workspaces and retained outputs; exclude regenerable OS packages/caches and explicitly disposable work. Show exclusions and workloads without a reliable backup method. Define app-consistent capture and restore handling, including custom services. Credential/identity recovery and backup encryption remain open. Whole-disk imaging is not selected as the default. |
| Decision 22: backup encryption and recovery keys | Decided | A + B: encrypt backups before transfer, provide exportable owner-held recovery material, and optionally keep a copy on another explicitly authorized trusted device. A backup destination does not automatically receive decryption authority. Scheduled backups run without repeated password entry after setup. Recovery must not rely solely on the source machine; account/service-assisted recovery and unencrypted backup mode are not selected. Key storage, rotation, revocation and recovery testing remain to be designed. Disk encryption remains opt-in/off by default. |
| Decision 23: server power behavior | Decided | A + C + D: stay available on external power, including supported closed-lid laptop use; protect laptops on battery loss; recover availability after power returns where hardware supports it. No automatic idle sleep in the first version. Keep thermal safeguards and deliberate shutdown/reboot/recovery available. Critical-battery protective action is distinct from ordinary idle sleep; exact shutdown/suspend choice remains engineering work. Resume approved services, not blindly interrupted jobs. The operator names Proxmox as the server-operation reference; this does not select its virtualization stack or change NixOS/Btrfs choices. |
| Decision 24: app access and internet use | Decided | A default, with B and C available per app: private Anima-network access by default; explicit LAN publication; explicit public publication through supported hosting paths. Keep host administration and recovery separate. These are inbound exposure choices, not a ban on outbound internet use. The operator explicitly requires agentic software to be able to use the internet. Support outbound access for workloads within their permissions; exact egress policy remains open. Installing a workload must not implicitly publish it. |
| Decision 25: outbound networking | Decided | A default, optional C + D: ordinary outbound internet within existing workload permissions, with optional destination-restricted and offline profiles. No extra per-workload internet approval step is selected beyond existing grants. Outbound access does not grant credentials, host control, other workload data or arbitrary private-network access. Inbound publication remains separate. Enforcement and supported allowlist semantics must be reconciled with existing Anima permission/network owners. |
| Decision 26: installation enrollment | Decided | A only for the first version: standard live installer presents short-lived pairing code/QR; owner approves in a signed-in Anima session, verifies the machine and explicitly confirms its installation disk before installation proceeds. Prepared unattended enrollment B and install-locally/enroll-later C are not selected. This narrows the earlier unattended setup direction. Local setup/recovery interfaces remain in scope; they do not imply an offline unenrolled installation path. Reuse existing identity rules and explicitly handle ownership transfer. |
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
2. Two product discussion groups remain after Decision 26: update channels/timing and workload interruption; first-release hardware examples and acceptance scope. Then consolidate the proposal and recommend engineering defaults together instead of continuing an unbounded component questionnaire. Existing provisional local-interface choice can be reviewed during consolidation. Exact protocol, package, partition, retention and test choices can remain explicit proposed defaults or validation tasks.
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

## Appliance design recommendations (not individually approved)

The side chat proposed one supported storage/base-package/update setup; outcome-based settings for networking, workload permissions, resource limits and maintenance windows; workload dependencies kept separate from the host; Anima owning supported host configuration; and recovery returning to a known system while preserving designated data. These are recommendations, not new approved requirements.

Use the approved product direction to propose a coherent engineering default. Ask the operator about consequential product tradeoffs instead of each filesystem, partition or service setting. Keep existing decisions unless explicitly changed. Optional administrator access does not itself settle support for arbitrary host modifications.

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

- 2026-11-19: Decision 12 accepted Btrfs for OS and persistent data. Next: shared versus separate system/data capacity on the installation disk, before exact partition sizes and subvolume paths.

- 2026-11-19: Decision 13 accepted A: one shared Btrfs filesystem with subvolumes and hardware-specific boot partitions. Next: system-state persistence before the detailed subvolume layout.

- 2026-11-19: Received an explicitly authorized side-chat relay of the appliance direction. Marked its provenance and separated the operator's words from the side chat's recommendations. Reframed unanswered Decision 14 around the supported host-customization boundary; did not select a root architecture or revoke administrator access.

- 2026-11-19: With the appliance context, the operator selected C for the original Decision 14: read-only system root and separate writable state. The earlier persistent-root recommendation is superseded. This does not answer the alternative host-customization support question. Next: automatic OS-update recovery policy.

- 2026-11-19: Decision 15 accepted A: bounded automatic rollback on local boot/core-service failures, not hub unreachability alone. Next: workload dependency boundary on the appliance host.

- 2026-11-19: Decision 16 accepted A + B + C, explicitly including supported custom host dependencies. This supersedes the narrower recommendation to support only curated host changes. Preserve read-only normal operation through controlled configuration; host-change authorization is still undecided. Next: owner approval and agent grants.

- 2026-11-19: Decision 17 accepted the recommendation: per-change owner approval by default, optional bounded standing grants. Broad administrator-agent mode is not selected. Next: workload priorities under resource contention.

- 2026-11-19: Decision 18 accepted A + D: service-first default with owner-selectable profiles. Protect OS/management/recovery capacity and explain profile changes. Next: automatic cleanup boundaries for shared storage.

- 2026-11-19: Decision 19 accepted A + B + C. Completed work is removable only when explicitly disposable under its declared lifetime; persistent/unmarked data stays protected. Next: backup destinations beyond same-disk snapshots.

- 2026-11-19: Decision 20 accepted A + B + C: another enrolled machine, network storage and selected separate drives. Cloud destinations and Anima-operated storage are not in the selected first-version scope. Next: default backup coverage.

- 2026-11-19: Decision 21 accepted A + D: managed persistent data/configuration and extra owner-selected paths. Next: backup encryption and recovery-key ownership, separately from installation-disk encryption.

- 2026-11-19: Decision 22 accepted A + B: encrypted backups with owner recovery material and optional explicitly trusted second-device key storage. Next: power behavior and laptop use.

- 2026-11-19: Decision 23 accepted A + C + D, with explicit server-first direction and no ordinary idle sleep. Proxmox is an operational reference. Its official feature overview (https://www.proxmox.com/en/products/proxmox-virtual-environment/features, read 2026-11-19) describes centralized server/workload administration; this check did not establish exact host suspend defaults. No Proxmox implementation or architecture was selected. Next: app network exposure.

- 2026-11-19: Decision 24 accepted A + B + C with private access as default. The operator noted agents need internet use; explicitly separated inbound publication from outbound workload connectivity. Next: default outbound policy.

- 2026-11-19: Decision 25 accepted A default with optional C + D. The operator asked how many questions remain. Proposed three remaining product-decision groups followed by consolidation and engineering defaults, not further low-level questionnaires.

- 2026-11-19: The operator asked for Proxmox or VMware precedent in the remaining decisions. Include sourced precedent, distinguish it from recommendations and retain the three remaining discussion groups.

- 2026-11-19: Decision 26 selected A only: pair in Anima and confirm the disk before installation. Prepared unattended and install-first/enroll-later paths are not first-version selections. Next: Decision 27, update policy (second of the final three groups).

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

In response to Decision 12:

> Yeah Abtfs is the best db in my option so just good enough for now

In response to Decision 13:

> yeah A is good

Operator words relayed by the side chat, with explicit authorization to relay (not independently read from that chat):

> I think following a caddy or steam os thing is not a bad idea here. Like we can point people to omnixy or omarchy for a development OS or they can do whatever they want but this is the ubuntu equivlent like just install it just works don't have to think about it but you sacrfice some configuration

After the revised Decision 14 recommendation (C, read-only system root):

> yeah lets do C

In response to Decision 15:

> go with A

In response to Decision 16:

> Yeah I think A B & C

In response to Decision 17 (recommendation A default, B available):

> Yeah I agree

In response to Decision 18 (recommendation A + D):

> agreeded

In response to Decision 19:

> A B and C

In response to Decision 20:

> Yeah A B C

In response to Decision 21 (recommendation A + D):

> Agreed

In response to Decision 22 (recommendation A + B):

> agreed

In response to Decision 23:

> agreed these are meant to be servers so sleep and idle are not really avalable right it's just whatever lets it be the best server copy proxmox

In response to Decision 24:

> A and B Adn C is fine reminder that some agentic software also need to use the internet

In response to Decision 25:

> agreed how many more questions

Process request:

> Okay lets keep going but in the decion include what the Proxmox or VMWare would do

## Decision 26 research: enrollment and unattended installation

Historical options, before the operator selected A only: standard ISO with a short-lived pairing flow approved in Anima and explicit disk confirmation before installation. Optional prepared installation profile: scoped enrollment authorization and explicit disk selection/erase permission; stop if selection is ambiguous. An alternative is local installation with administrator credentials followed by enrollment. Local recovery remains available; proof of ownership, token lifetime and transfer/re-enrollment need specification.

Precedent checked 2026-11-19:

- [Proxmox installation documentation](https://github.com/proxmox/pve-docs/blob/master/pve-installation.adoc): local graphical/text installation, administrator password, and unattended installation via a prepared ISO and answer file. This is not automatic assignment to an Anima-style account.
- [Broadcom ESXi scripted-installation article](https://knowledge.broadcom.com/external/article/440457/esxi-scripted-installation-fails-to-dyna.html): ESXi 7/8 scripted setup uses kickstart configuration for host-specific installation settings. We should adopt explicit prepared configuration as a pattern, not copy example first-disk overwrite directives.

In response to Decision 26:

> A

## Decision 27 preparation: updates and interruption

Not selected yet. Recommend stable releases by default, optional preview enrollment, automatic download/validation, and disruptive activation only inside an owner-authorized maintenance window. Stop admitting new work and let active work drain; if it cannot safely finish/checkpoint or a service cannot stop gracefully, defer and report. Do not infer live migration or silently kill non-restartable work. Offer explicit per-update approval as an alternative. Existing maintenance authorization and automatic rollback decisions stay in force.

Precedent read 2026-11-19:

- [Proxmox repository documentation](https://github.com/proxmox/pve-docs/blob/master/pve-package-repos.adoc): checks for updates daily and exposes installation through GUI/APT; distinguishes production-oriented and test repositories. This is not evidence of unconditional unattended reboot.
- [VMware ESXi maintenance guidance](https://knowledge.broadcom.com/external/article/434515): host remediation requires maintenance mode; standalone hosts need workload downtime because they lack automated DRS evacuation. Apply the planned-interruption principle, not an assumption that Anima already supports VM migration.
