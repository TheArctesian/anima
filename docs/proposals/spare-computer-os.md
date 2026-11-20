---
kind: Proposal
state: Draft
evidence: Version 0.1 (2026-11-19). The operator requested an implementation proposal after 28 product decisions recorded in this conversation. Those decisions establish direction; the proposed requirements, contracts and build tasks below are not approved. No implementation or hardware validation has been performed. Serves priority row 4.16, with dependencies on rows 4.7, 4.9 and 4.18. Replaces the spare-computer-os idea; its decisions and source evidence are retained here.
---

# Proposal: Anima OS for spare computers

| Item | Value |
| --- | --- |
| Version | 0.1 |
| Date | 2026-11-19 |
| Status | Draft. The header holds the state. This proposal is not permission to build or release. |
| Serves | [Priority 4.16](../plan/priorities.md): a dedicated-machine installation path. This mapping adds no new tier and does not delay existing install work. Workload dependencies remain in rows 4.7 and 4.9; updates in 4.18. |
| Owner | This proposal owns the proposed OS image, installer, host state layout and recovery integration. Existing owners retain application, job, agent, identity and permission semantics. |
| Identifiers | AOS-001 to AOS-024; open decisions OD-AOS1 to OD-AOS6; proposed tasks OS0 to OS8 |
| Source | The operator's 28-decision conversation, retained in the appendices. Earlier idea history ends at commit 2371f957. |

## 1. Product and scope

Give a spare computer a new purpose as an Anima-managed server. The first stable release supports persistent applications, queued jobs and agents that execute on the machine. It is a fresh project, not a fork of OmniXY or nascentOS. NixOS is the proposed base; its exact release is not selected.

The normal path is: download an image, write it to removable media, boot the live environment, configure networking, pair through Anima, inspect the target disk, confirm erasure, install and reboot. Unselected disks stay unchanged. The owner then assigns workloads through Anima.

The system behaves as an appliance. The normal root is read-only. Declared custom host packages and services remain supported through a controlled configuration path. Administrator access is optional; it is not a promise that arbitrary unmanaged host changes survive an update. The exact customization surface is an open design decision.

Target families are 64-bit Intel/AMD PCs, Intel Macs and selected ARM64 machines. Apple Silicon, 32-bit PCs, same-disk dual boot, a full desktop, prepared unattended enrollment and install-first/enroll-later are outside the selected first-version scope. Decision 26 narrows the earlier unattended-install idea. The graphical/text interface remains provisional under Decision 7. Do not infer VM hosting or local model inference from the word "agent".

Some hardware or capabilities may prove impossible or impractical. Record failed evidence and request an explicit scope change rather than quietly remove a selected family. Preview builds are experimental. A stable label requires the acceptance evidence in Section 7.

## 2. Repository boundaries and delivery artifacts

| Repository / owner | Proposed responsibility | Boundary |
| --- | --- | --- |
| New OS repository, working name `anima-os` | Flake and lockfile, Nix modules, machine profiles, boot images, installer backend and local interfaces, host recovery, VM tests and release manifests | Name/location and creation are not approved. It consumes pinned Anima artifacts; it does not fork the hub, scheduler or permission system. |
| This product repository | Planning authority; daemon/link/CLI changes; hub contracts; account pairing integration; maintenance API; client views; workload and backup integration in the proper owner domains | Build shared capabilities in hub/API, CLI, MCP where permitted, then UI order. Installer erasure and ownership approval must not become unrestricted agent tools. |
| Existing account/site/cloud repositories, if required | Account-side pairing or release-delivery integration owned by their existing plans | No provider, account or deployment change is authorized by this proposal. Determine the minimum necessary changes in OS0. |

Expected artifacts: x86-64 installer ISO, selected ARM board images where needed, release manifest with hashes and authenticated provenance, versioned host configuration, compatible daemon/link versions, recovery media, operator documentation, compatibility results and a known-limitations list. Do not promise one ISO can boot every ARM board.

Proposed OS repository layout: `flake.nix`, `flake.lock`, `modules/`, `profiles/`, `images/`, `installer/`, `recovery/`, `tests/`, `release/` and `docs/`. These names describe responsibilities, not a mandate to create empty folders. Keep one installation backend behind local and Anima-led interfaces. The exact language and UI toolkit are deferred to OS0; use existing project tooling where it fits.

## 3. Dependencies and contract boundaries

These are design dependencies, not assertions that the dependent capability is complete. OS0 must inspect each current task and implementation before selecting integration work.

| Concern | Existing owner | Required integration / evidence |
| --- | --- | --- |
| Device enrollment and private link | [One-click plan](../plans/one-click/one-click-plan.md), [hosted mode](hosted-mode.md), K8 in [daemon push](../../contracts/daemon-push.md) | Pair live environment to an owner, authorize one installation, establish installed identity without duplicating the live identity. Reuse trust and revocation rules. |
| Applications | [Hosted applications](../plans/hosted-applications-plan.md), [stage 2](hosted-applications-stage-2.md) | Use approved runtime, application grants and storage capture hooks. No direct installer/UI bypass of the applications program. |
| Jobs, routing and agents | [Projects and builds](../plans/projects-and-builds-plan.md), [jobs](jobs.md), [agent runs](agent-runs.md), [agent tools](agent-tools-as-extensions.md), [routed command lines](routed-command-lines.md) | Respect work-route contracts and pending reviews. Reconcile offline execution, resource profiles and bounded host-change grants; do not silently override existing rules. |
| Maintenance and releases | [Daemon/hub updates](../plans/daemon-and-hub-update-plan.md), [release procedures](../procedures/releases.md) | Define which component updates daemon/link on this image. Prefer one OS-owned version set rather than independent updaters racing. Define compatibility and signing before release. |
| Backups and persistent state | [Coordinator recovery](coordinator-standby.md), [bulk data](bulk-data-placement.md), hosted-app owners above | Backing up an appliance is not a second coordinator-standby system or a distributed live filesystem. Define workload-consistent backup hooks and replacement-device restore. |
| Network, permissions and local access | [Design specification](../spec/design-spec.md), [CLI/MCP](../plans/cli-mcp-plan.md), work owners above | Separate inbound app publication from outbound internet. Keep permission enforcement in existing domains; local recovery cannot depend solely on a reachable hub. |
| Power and hardware | [Power/health](power-health.md), design specification stay-awake rules | Add appliance-specific power policy through the owner; retain thermal/battery protection and record firmware limits. |
| User documentation | [User documentation proposal](user-documentation.md) | Provide install, pairing, recovery, storage, update and hardware-limit pages through the selected docs owner. |

Proposed contract work: an expiring installation session with explicit owner/disk confirmation; a versioned host configuration with validation and activation states; maintenance admission/drain/rollback status; and backup capture/restore hooks. Contracts need exact schemas, idempotency, reconnect behavior, secrets handling and owner review before code. This document changes no frozen contract.

## 4. Proposed requirements

The product choices are recorded in Appendix A. The following testable requirements are proposed, not approved. They define the OS integration; referenced owners still own their underlying capabilities.

| ID | Proposed requirement | Decision source / acceptance |
| --- | --- | --- |
| AOS-001 | The image must support the approved app, job and agent execution paths without a second scheduler or permission store. | 1, 16, 17; M3 |
| AOS-002 | The release must identify tested hardware, supported boot paths and unverified configurations separately. | 3, 4, 28; M6 |
| AOS-003 | The installer must provide a non-destructive live trial, report target-disk identity and require explicit erase confirmation. All installation writes must be confined to approved storage and required machine firmware settings. | 5, 6; M2, verify disk bytes on unrelated drives |
| AOS-004 | The first-version installation must use an expiring owner-approved pairing session and must reject stale/replayed authorization or silent ownership replacement. | 26; M2 |
| AOS-005 | The installed root must be read-only in normal operation, with an inventoried writable state boundary and a controlled update/repair path. | 14; M1/M4 |
| AOS-006 | System and workload storage must share a Btrfs filesystem with explicit subvolume boundaries; data-preserving repair must not format it. | 12, 13; M1/M4 |
| AOS-007 | Disk encryption must be off by default. Opted-in installations must support local passphrase recovery and validated hardware-backed unlock where available. | 10, 11; M4 |
| AOS-008 | Approved apps and already-running jobs/agents must follow existing offline grants when the hub is unreachable. New queued/scheduled jobs must not start offline. | 8; M3, reconcile owner contracts first |
| AOS-009 | Authorized local diagnostics and recovery must remain possible without a reachable hub. | 2, 7, 8; M2/M4 |
| AOS-010 | Host customization must use validated declared configuration. Agent host changes require per-change approval unless an explicit bounded grant covers them. | 16, 17; M3/M4 |
| AOS-011 | Resource profiles must default to service-first, allow worker-first/balanced choices, and protect management/recovery capacity without silently killing work on profile changes. | 18; M3 |
| AOS-012 | Cleanup must only remove eligible regenerable material, retention-expired history and explicitly disposable outputs. It must preserve protected recovery state and unmarked persistent data. | 19; M5 |
| AOS-013 | Storage pressure that permitted cleanup cannot resolve must cause an alert and bounded admission, not broader deletion authority. | 13, 19; M5 |
| AOS-014 | OS rollback, data-preserving repair, selected data restore and confirmed factory reset must be distinct operations. | 9; M4/M5 |
| AOS-015 | Failed boot or required local health checks must cause bounded fallback to a known-good OS where validated; hub unreachability alone must not trigger fallback. | 15; M4 |
| AOS-016 | Stable updates must validate before activation, drain work and respect an approved maintenance window or per-update approval. Blocked drain must defer and report. Preview must be opt-in. | 27; M4 |
| AOS-017 | OS rollback must not silently restore old workload data. Incompatible state/schema changes must block unsafe rollback or use a separately authorized restore procedure. | 9, 15, 27; M4 |
| AOS-018 | Backup destinations must support authorized peers, supported network storage and selected separate drives without implicitly claiming or formatting a destination. | 20; M5 |
| AOS-019 | Enabled backups must cover managed persistent data/configuration and allow explicit extra paths; exclusions and inconsistent/unsupported capture must be visible. | 21; M5 |
| AOS-020 | Backup contents must be encrypted before transfer, with exportable owner recovery material and optional explicit trusted-device recovery. Destination access alone must not grant decryption. | 22; M5 |
| AOS-021 | The server must avoid automatic idle sleep on external power, preserve thermal protection and implement supported battery/power-return recovery without blindly replaying jobs. | 23; M6 |
| AOS-022 | Apps must default to private access, with deliberate per-app LAN/public publication through the existing hosting paths. Outbound internet must follow existing grants, with restricted/offline profiles available. | 24, 25; M3 |
| AOS-023 | Stable-release evidence must include installation, workload, failure/recovery and replacement-storage restore tests on named reference hardware. No VM-only result may establish physical hardware support. | 28; M6 |
| AOS-024 | Every artifact must identify its OS configuration, packaged Anima versions, source revisions, integrity/authenticity metadata and compatibility limits. | Proposed delivery detail; M1/M4/M6 |

## 5. Engineering defaults to validate

These are starting designs, not accepted implementations or support guarantees.

- Use a pinned NixOS stable configuration with separately built x86-64 and ARM artifacts. The OS build owns a compatible daemon/link version set; custom host configuration is an explicit input to the build.
- On a conventional PC, start with GPT, a proposed 1 GiB EFI system partition, a BIOS boot area where required, and the remaining space as Btrfs. Optional LUKS2 wraps that Btrfs area. ARM firmware may require a different layout. Test boot-file capacity before fixing sizes.
- Evaluate systemd-boot for UEFI and GRUB for BIOS. Select only after boot attempt counting, fallback and power-interruption experiments. Secure Boot/TPM policy is part of that evaluation, not assumed from a bootloader name.
- Separate root, controlled Nix-store/update writes, machine identity/configuration, logs, app datasets, persistent workspaces and disposable caches. The root remains read-only; `/nix` must not be generally writable merely because maintenance needs to write it. Mount policy and privileged update authority need proof.
- Use NixOS generations for system configuration/software rollback. Use data snapshots and encrypted backups for data restore, with application-consistency hooks. Never treat all filesystem state as one rollback unit.
- Evaluate bounded zram rather than reserve a hibernation partition. Set RAM, storage, retention and pressure thresholds from measurements. These are not yet product minimums.
- Reuse supported Anima container/work runtimes. Do not select Docker versus Podman, a VM platform or arbitrary command execution independently of their owner contracts.
- Evaluate an existing encrypted backup engine rather than invent a format. Compare supported destinations, append/delete authority, interrupted backup behavior, credentials, restore isolation and key recovery before selection.

## 6. First feasibility experiments

All experiments are proposed, not started. Use disposable VM disks and isolated test identities. No real disk may be erased by an experiment without explicit target authorization.

| Experiment | Question | Pass evidence | If it fails |
| --- | --- | --- | --- |
| F1: root and state | Can the selected base run normally with read-only root while preserving identity, networks, workloads and declared custom services? | Reboot and controlled update preserve designated state; normal users/workloads cannot write the base; interrupted update leaves a bootable known state. | Change writable boundaries or configuration design; seek approval if product behavior must change. |
| F2: boot recovery | Can each boot path detect failed activation and fall back without a person? | Inject bad kernel/initrd or failed core health; bound retries; return to known-good; hub outage alone does not roll back; data sentinel stays unchanged. | Qualify a narrower tested path or revise boot mechanism. Do not label untested BIOS/ARM fallback supported. |
| F3: Anima integration | Can a live installer pair, install and become the correct managed device under current contracts? | Expiry/replay/wrong-device tests fail safely; installed identity is unique; old session is unusable; daemon and hub agree on versions and ownership. | Submit exact contract deltas to their owners before further integration. |
| F4: runtime and data | Do the approved app/job/agent paths work on this base and shared storage? | Run all three; keep management usable under pressure; test offline grants; capture and restore a real stateful app consistently. | Report missing upstream work or incompatible runtime assumptions; do not bypass owners. |

F1 and F2 form the architecture gate before substantial UI work. F3 gates disk installation integration. F4 gates claims that the image is a useful server. Booting one VM is not sufficient evidence for any later gate.

## 7. Milestones and acceptance

| Milestone | Deliverable and dependencies | Acceptance before next claim |
| --- | --- | --- |
| M0: design and feasibility | OS0/OS1; owner/repository decisions and reviewed contract deltas | F1-F3 reports, risk decisions and a dependency map tied to actual code/plan states. A failure is a result, not completion of the feature. |
| M1: bootable foundation | OS2; selected base, package builds, state mounts, images; depends on F1/F2 | Repeated boot, correct persistent identity, read-only normal root, reproducible version manifest and daemon visibility in a test hub. |
| M2: install and ownership | OS3; backend, network setup, pairing, disk confirmation, local interfaces; depends on F3/M1 | Trial leaves disks unchanged; wrong/replayed requests reject; approved installation boots enrolled; unrelated disk hashes match before/after; local recovery works offline. |
| M3: useful server | OS4; approved app/job/agent runtime integration, resource/network/grant policies; depends on M2/F4 and workload owners | Stateful app, build and agent execute together; grants and network modes enforced; management stays responsive; hub outage follows Decision 8. |
| M4: maintain and recover | OS5; versioned activation, draining, fallback, repair/reset and optional encryption; depends on M3 and update contracts | Failed update and power-interrupted activation recover; no silent data rollback; blocked work defers reboot; encrypted unlock/recovery tested on named hardware; custom host configuration survives supported updates. |
| M5: protect data and capacity | OS6; destination adapters, backup keys, capture/restore and cleanup; depends on M3/M4 | Restore onto replacement storage without source machine; all selected destination types tested; wrong/missing keys report clearly; full-disk tests preserve protected data and admit no unsafe new allocation. |
| M6: qualify and publish | OS7/OS8; reference hardware, docs, artifacts and explicit release authorization; depends on M1-M5 | Publish exact image revisions, device/firmware identifiers, measured minimums, test outcomes and limits. Cover selected PC/Intel Mac/ARM families and boot paths before stable support claims. |

An experimental preview may be prepared after M2 and an explicit preview-scope review. It must say which later milestones are absent. All three workload classes and the accepted recovery/backup functions remain in the stable-release target. Moving a capability or family later requires a visible operator decision.

## 8. Proposed task sequence

No task is buildable from this Draft. Each later claim must name its task and approved requirements. Scope-changing findings return to this proposal and the relevant owner before implementation.

| Task | Work | Depends on | State | Completion evidence |
| --- | --- | --- | --- | --- |
| OS0 | Reconcile owner contracts/current code; select repository/base; define installer, maintenance and restore trust boundaries; obtain decisions | OD-AOS1 to OD-AOS5 | Not started | Reviewed architecture and exact proposed contract deltas; no deployment |
| OS1 | Run F1-F4 feasibility experiments, report pass/fail and hardware limits | OS0 and explicit experiment approval | Not started | Reproducible commands, versions, logs without secrets, failure cases |
| OS2 | Build versioned base image and state layout | OS1 architecture gate | Not started | M1 record |
| OS3 | Build installer/enrollment and local interfaces | OS2, approved identity contracts, OD-AOS3 | Not started | M2 record |
| OS4 | Integrate workload execution, host changes and resource/network profiles | OS3, F4, approved workload dependencies | Not started | M3 record |
| OS5 | Build maintenance, rollback, repair/reset and optional encryption | OS4, update/boot contracts | Not started | M4 record |
| OS6 | Build backup/restore and bounded cleanup | OS4, OS5 recovery interfaces | Not started | M5 record |
| OS7 | Qualify hardware, produce docs and preview/stable evidence | OS2-OS6; models from OD-AOS4 | Not started | M6 reports and human-check entries with exact steps |
| OS8 | Prepare release manifest and publish approved artifacts | OS7 and separate release authorization | Not started | Published artifact identity and matching evidence |

Independent packaging and hardware research can proceed in parallel only after authorization and without delaying higher-priority work. Integration follows the dependencies above. Do not estimate calendar dates before OS1 and an audit of missing workload capabilities.

## 9. Open decisions and proposal adoption

| ID | Question | Recommendation | State |
| --- | --- | --- | --- |
| OD-AOS1 | Confirm NixOS as the base and a separate OS repository? | Yes; working name `anima-os`, actual location/name confirmed before creation. Keep shared Anima code in existing repositories. | Open; no repository created |
| OD-AOS2 | Retain this proposal under priority 4.16 or assign a new priority row? | Use 4.16 for the installation deliverable now; do not change its tier or authorize work ahead of its existing tasks. | Open; mapping only, no reprioritization |
| OD-AOS3 | Confirm the provisional graphical plus text local interface from Decision 7? | Yes, sharing one installer backend; no full desktop. | Open; side-chat revisit requested |
| OD-AOS4 | Which named PCs, Intel Macs and ARM boards can be used for validation? | Select available devices covering UEFI, BIOS, ARM boot, networking and optional TPM. Measure minimum resources. | Open; no hardware access or result assumed |
| OD-AOS5 | Approve the proposed requirements and a bounded start of OS0/OS1 after architecture/contract review? | Review AOS-001 to AOS-024 and the dependency conflicts first. Authorize a bounded feasibility phase before the rest of implementation. | Open; writing this proposal does not approve its build |
| OD-AOS6 | What custom host configuration is a supported public input? | Start with declared packages/services and reviewed modules; define allowed options, privilege boundaries and update tests before advertising arbitrary Nix modules. | Open; reconcile with Decision 16, do not narrow it silently |

The implementation proposal is complete for review with these open decisions. No new product questionnaire is required to read it. Engineering specifics can be resolved through the approved feasibility work; material changes to the 28 choices require operator decisions. When approved, move this file to the plans directory and update its register/ownership links in the same change.

## 10. Validation, risks and planning integration

This is a documentation change only. Run `bun run quality:register`, `bun run quality:documents` and `bun run gates:fast`; the repository's branch/merge-train gates govern landing. No OS build, recovery trial, network integration or backup restore has been run.

Primary risks: read-only base plus host customization; fallback differences across firmware; TPM/update policy compatibility; missing ARM/Intel Mac drivers; shared storage exhaustion; workload-consistent backups; secret-bearing restore; and existing contracts that prohibit a proposed action. OS0 must reconcile the recent routed-command-line and agent-layer work instead of treating this proposal as an override.

This proposal is registered under the existing installation concern. It adds an OS ownership row and a reference under priority 4.16, not a new active implementation priority. The idea folder is removed on promotion; the following appendices retain decision provenance. Separate backup, permission, runtime or release requirements belong to their existing owners where applicable.

## 11. Prior art and its limits

- [Proxmox installation](https://github.com/proxmox/pve-docs/blob/master/pve-installation.adoc): shared graphical/text installation backend, explicit disk selection and prepared answer files. Anima selects interactive pairing for v1; unattended profiles are later.
- [Proxmox update repositories](https://github.com/proxmox/pve-docs/blob/master/pve-package-repos.adoc): visible updates and production/test distinctions. This is not a claim that Proxmox performs unconditional unattended reboot.
- [VMware maintenance guidance](https://knowledge.broadcom.com/external/article/434515): planned host maintenance and workload interruption, especially on standalone hosts. No Anima live migration is assumed.
- [Proxmox requirements](https://proxmox.com/en/products/proxmox-virtual-environment/requirements) and [VMware compatibility guidance](https://knowledge.broadcom.com/external/article/313743/confirming-esxesxi-host-hardware-system.html): publish support boundaries and evidence. Do not borrow their minimums or claim their certification.
- [OmniXY](https://github.com/TheArctesian/omnixy) and [nascentOS](https://github.com/TheArctesian/nascentOS): operator experience and background only. [NixOS Apple Silicon support](https://github.com/nix-community/nixos-apple-silicon) is a possible later path, not in v1.

Sources above were read during the planning conversation on 2026-11-19. Proxmox, VMware, Caddy, SteamOS and Ubuntu are references for specific behavior, not architecture selections.

## Appendix A. Recorded direction

These are conversation decisions, not evidence that the requirements are built. Later numbered choices resolve earlier open details. The new requirement text above remains proposed.

| Subject | State | Choice and reason |
| --- | --- | --- |
| Intended use | Decided | Give a spare computer a new purpose. Managing its existing installation is not the purpose of this image. |
| Project starting point | Decided | Start from scratch. Do not assume a fork of OmniXY or nascentOS. Repository name and location remain open. |
| NixOS foundation | Provisional | The operator proposed NixOS as a good fit. Confirm its selection during the design review; no release or package choices are made. |
| Decision 1: first-version workloads | Decided | Hosted applications, queued jobs and agent execution are all in scope. An agent runner executes agents on the machine; it is not only a machine that an external agent can control. |
| Decision 2: maintenance | Decided | A + B + C: automatic approved routine upkeep and problem reporting; owner-defined maintenance windows and control over disruptive actions; optional SSH or local administrative access. Fully manual maintenance mode is out of the first version. Exact update, reboot, access and configuration policies remain to be designed. |
| Decision 3: hardware families | Decided in part | A + B + C: 64-bit Intel/AMD PCs, Intel Macs and selected ARM64 boards or mini PCs. Exact supported models need selection and testing. Apple Silicon is deferred to a later release (follow-up choice C). Keep its possible integration in the design; it is not an experimental first-release target. Older 32-bit PCs were not selected. Firmware and resource minimums remain open. |
| Decision 4: boot paths | Decided | A + B + C: 64-bit UEFI, legacy BIOS for 64-bit PCs and board-specific boot for selected ARM64 hardware. This does not add 32-bit CPU support. Bootloader choice, Secure Boot policy, exact hardware targets and installation/recovery validation remain open. |
| Decision 5: disk installation modes | Decided | A + B + E: install to one explicitly selected disk after erase confirmation, keep installation and boot files on that disk and preserve unselected disks; offer a live trial before installation. B is a safeguard of A, not a separate mode. Same-disk dual boot and external-drive installation are not first-version commitments. Trial persistence remains open; Decisions 12 and 13 select the filesystem and shared-capacity layout. |
| Decision 6: setup surfaces | Decided in part | Add setup from within Anima after the machine boots and connects to the internet. An opt-in prepared unattended path was initially requested; Decision 26 later selects A only for the first version, leaving that path for later discussion. A standalone local-browser interface is not implied. Decision 7 provisionally selects graphical and text setup; a full desktop is not selected. Decision 26 defines first-version enrollment and disk confirmation; network bootstrap details remain open. |
| Decision 6: distribution | Decided | Standard downloadable ISO written to USB with dd or a graphical flasher. No custom flasher is required by this choice. It does not settle prepared-media settings, enrollment or automatic disk erasure. Selected ARM boards may need board-specific image formats; do not assume one ISO boots every target. |
| Decision 7: local interface | Provisional | A + B: graphical setup and installed status/recovery screen, plus text setup and recovery menus as a fallback. Setup through Anima remains in scope. The operator is discussing this in a side chat and asked to log the choice for now and revisit later. No interface framework or full desktop is selected. |
| Decision 8: hub disconnection | Decided | A + B + D: approved applications keep running and may restart under their existing policy; already-running jobs and agents may continue within existing permissions and limits; authorized local administration remains available without a reachable hub. Do not start new queued or scheduled jobs while disconnected in the first version. Grant expiry, reconnect reconciliation and local authentication still need design. External-service dependencies can still prevent a workload from working. |
| Decision 9: recovery operations | Decided | A + B + C + D: OS rollback without deliberately reverting workload data; OS repair/reinstallation preserving designated persistent data; selected app/dataset restore from available snapshots or backups; explicit factory reset removing ownership, credentials and workload data. These are separate operations. Application compatibility, preserved-state inventory, backup policy, automated recovery triggers and secure drive sanitization remain to be designed and tested. |
| Decision 10: disk encryption | Decided | C: optional, explicit opt-in at installation, off by default. The operator prioritizes remote access and unattended restart after a crash; a default setup must not wait for someone to enter a disk password locally. Explain the at-rest protection tradeoff and reboot behavior when encryption is selected. Decision 11 selects the supported unlock methods; their implementation remains open. |
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
| Decision 21: backup coverage | Decided | A + D: after destination/policy approval, cover managed persistent data and configuration by default, with optional owner-selected extra paths. Include persistent agent/job workspaces and retained outputs; exclude regenerable OS packages/caches and explicitly disposable work. Show exclusions and workloads without a reliable backup method. Define app-consistent capture and restore handling, including custom services. Credential/identity recovery remains open; Decision 22 selects backup encryption policy. Whole-disk imaging is not selected as the default. |
| Decision 22: backup encryption and recovery keys | Decided | A + B: encrypt backups before transfer, provide exportable owner-held recovery material, and optionally keep a copy on another explicitly authorized trusted device. A backup destination does not automatically receive decryption authority. Scheduled backups run without repeated password entry after setup. Recovery must not rely solely on the source machine; account/service-assisted recovery and unencrypted backup mode are not selected. Key storage, rotation, revocation and recovery testing remain to be designed. Disk encryption remains opt-in/off by default. |
| Decision 23: server power behavior | Decided | A + C + D: stay available on external power, including supported closed-lid laptop use; protect laptops on battery loss; recover availability after power returns where hardware supports it. No automatic idle sleep in the first version. Keep thermal safeguards and deliberate shutdown/reboot/recovery available. Critical-battery protective action is distinct from ordinary idle sleep; exact shutdown/suspend choice remains engineering work. Resume approved services, not blindly interrupted jobs. The operator names Proxmox as the server-operation reference; this does not select its virtualization stack or change NixOS/Btrfs choices. |
| Decision 24: app access and internet use | Decided | A default, with B and C available per app: private Anima-network access by default; explicit LAN publication; explicit public publication through supported hosting paths. Keep host administration and recovery separate. These are inbound exposure choices, not a ban on outbound internet use. The operator explicitly requires agentic software to be able to use the internet. Support outbound access for workloads within their permissions; Decision 25 sets the default egress policy. Installing a workload must not implicitly publish it. |
| Decision 25: outbound networking | Decided | A default, optional C + D: ordinary outbound internet within existing workload permissions, with optional destination-restricted and offline profiles. No extra per-workload internet approval step is selected beyond existing grants. Outbound access does not grant credentials, host control, other workload data or arbitrary private-network access. Inbound publication remains separate. Enforcement and supported allowlist semantics must be reconciled with existing Anima permission/network owners. |
| Decision 26: installation enrollment | Decided | A only for the first version: standard live installer presents short-lived pairing code/QR; owner approves in a signed-in Anima session, verifies the machine and explicitly confirms its installation disk before installation proceeds. Prepared unattended enrollment B and install-locally/enroll-later C are not selected. This narrows the earlier unattended setup direction. Local setup/recovery interfaces remain in scope; they do not imply an offline unenrolled installation path. Reuse existing identity rules and explicitly handle ownership transfer. |
| Decision 27: update timing and channels | Decided | A default, B and D available; C deferred. Automatically download/validate stable releases; disruptive activation requires an owner-authorized window or per-update approval. Stop admitting work, allow safe drain/checkpoint where supported, and gracefully stop approved restartable services. Defer/report if work cannot safely drain; no deadline-forced termination. Offer explicit preview-channel enrollment. Preserve agreed local-failure rollback and do not revert workload data automatically. No live migration is implied. |
| Decision 28: release readiness | Decided direction, feasibility conditional | A + C: experimental preview followed by stable support on named tested reference machines; allow compatible unverified machines without claiming certification. Accept the proposed installation, mixed-workload, offline/power, update-failure, backup-restore and optional-encryption recovery checks. Some target hardware or capabilities may prove impossible or impractical to support. Record limitations and bring material scope changes back to the operator; do not claim untested support or silently remove selected families. Exact models and measured resource minimums remain open. |
| Earlier apps-only answer | Superseded | The operator first selected useful apps, then corrected this to all capabilities and explicitly included agents and jobs. Do not use the earlier answer to exclude these from the first version. |
| Decision process | Decided | Discuss practical options and tradeoffs, give a recommendation, take the operator's answer and update this record. Allow combined answers. Record uncertainty and tests before treating an unverified choice as settled. |


## Appendix B. Source excerpts


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


Final enrollment choice (Decision 26): "A". Update policy (Decision 27): "agreeded". Release readiness (Decision 28): "Yeah A and and C just mention is might not be possible".

The operator requested this implementation proposal with: "okay lets do that".
