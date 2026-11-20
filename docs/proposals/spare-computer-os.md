---
kind: Proposal
state: Draft
evidence: Version 0.6 (2026-11-20). Section 14 records the approved installed-console direction and motion engineering programme; the experimental console is implemented and VM-tested, while measured motion qualification remains pending. An experimental UEFI installer ISO is built and VM-tested; see the ISO continuation record. The operator authorized the bounded OS1 prototype; implementation and VM evidence are in the OS1 record. Section 13 completes the bounded overnight implementation brief and dependency audit. Three independent council reviewers examined architecture, security/recovery and delivery; Section 12 records the reconciled elaboration. The operator requested an implementation proposal after 28 product decisions recorded in this conversation. Those decisions establish direction; the proposed requirements, contracts and build tasks below are not approved. A disposable x86-64 UEFI prototype is implemented under R-276; no physical hardware support or production integration is validated. Serves priority row 4.16, with dependencies on rows 4.7, 4.9 and 4.18. Replaces the spare-computer-os idea; its decisions and source evidence are retained here.
---

# Proposal: Anima OS for spare computers

| Item | Value |
| --- | --- |
| Version | 0.6 |
| Date | 2026-11-20 |
| Status | Draft outside the explicitly authorized OS1 recovery experiment and OS2/OS3 installer/console preview slices. The header holds the state. No release is authorized. |
| Serves | [Priority 4.16](../plan/priorities.md): a dedicated-machine installation path. This mapping adds no new tier and does not delay existing install work. Workload dependencies remain in rows 4.7 and 4.9; updates in 4.18. |
| Owner | This proposal owns the proposed OS image, installer, host state layout and recovery integration. Existing owners retain application, job, agent, identity and permission semantics. |
| Identifiers | AOS-001 to AOS-024; open decisions OD-AOS1 to OD-AOS7; proposed tasks OS0 to OS8 |
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

F1 and the UEFI slice of F2 are authorized and implemented as a bounded experiment; see the [OS1 record](../research/spare-computer-os/os1-prototype-2026-11-20.md) for evidence and limits. F3/F4 remain unapproved and not started. Use disposable VM disks and isolated test identities. No real disk may be erased by an experiment without explicit target authorization.

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

Only the bounded OS1 F1/UEFI-F2 experiment in Section 13 is authorized by R-276. Other implementation tasks remain unapproved. Each claim must name its task and approved scope. Scope-changing findings return to this proposal and the relevant owner before implementation.

| Task | Work | Depends on | State | Completion evidence |
| --- | --- | --- | --- | --- |
| OS0 | Complete dependency audit and bounded experiment brief; propose repository/base and trust boundaries | Planning review; no implementation authority | Planning complete | Section 13 is the OS0 acceptance record: dependency snapshot, proposed contract deltas, bounded experiment protocol and handoff; no implementation approval or runtime evidence |
| OS1 | Run independently gated F1-F4 experiments; report each separately | OS0 brief, OD-AOS5 experiment authorization; base/repository decisions only where needed; F3/F4 owner contracts | Recovery follow-up built; tested UEFI cases passed, full F2 unqualified | [Initial OS1 evidence](../research/spare-computer-os/os1-prototype-2026-11-20.md) and [recovery/ISO continuation](../research/spare-computer-os/iso-preview-2026-11-20.md); pre-initrd deadlock and hardware recovery remain unqualified |
| OS2 | Build versioned base image and state layout | Approved F1/F2 architecture evidence | Experimental preview built under R-274 | [ISO evidence](../research/spare-computer-os/iso-preview-2026-11-20.md); runtime packaged and installed state tested; full M1 qualification remains open |
| OS3 | Build installer/enrollment and local interfaces | OS2, approved identity contracts, OD-AOS3 | Experimental local UEFI installer built under R-274 | [Offline ISO install test](../research/spare-computer-os/iso-preview-results.json); [Calamares preview](../research/spare-computer-os/gui-preview-2026-11-20.md) built under R-273; fullscreen, network/display controls and separate data drives built under R-272; [combined-image graphical/CLI install and reboot checks passed](../research/spare-computer-os/fullscreen-preview-results.json); the [installed console and automatic enrollment](../research/spare-computer-os/console-preview-2026-11-20.md) are built under R-271; [hardware follow-up and persona evidence](../research/spare-computer-os/hardware-followup-2026-11-20.md) record scaling, installed Wi-Fi backend and rotation corrections; physical qualification, live-account verification and hub-driven pre-install setup remain open; motion work is sequenced in Section 14 |
| OS4 | Integrate workload execution, host changes and resource/network profiles | OS3, F4, approved workload dependencies, OD-AOS6/7 | Not started | M3 record |
| OS5 | Build maintenance, rollback, repair/reset and optional encryption | OS4, update/boot contracts | Hosted preview updates built under R-282; other maintenance/recovery capabilities remain unimplemented | [Installed-operation preview and VM update/rollback evidence](../research/spare-computer-os/installed-operation-2026-11-20.md); no M4 acceptance claimed |
| OS6 | Build backup/restore and bounded cleanup | OS4, OS5 recovery interfaces | Not started | M5 record |
| OS7 | Maintain hardware evidence and user documentation; qualify preview and stable separately | Start matrix with OS1; preview after M2; stable after M1-M5; named hardware from OD-AOS4 | Not started | Separate preview/stable evidence packets and human-check entries; installation docs with M2, recovery docs with M4 |
| OS8 | Prepare release manifest and publish approved preview or stable artifacts | Corresponding OS7 evidence packet and separate release authorization for that scope | Not started | Published artifact identity and matching evidence |

Independent packaging and hardware research can proceed in parallel only after authorization and without delaying higher-priority work. Integration follows the dependencies above. Do not estimate calendar dates before OS1 and an audit of missing workload capabilities.

## 9. Open decisions and proposal adoption

| ID | Question | Recommendation | State |
| --- | --- | --- | --- |
| OD-AOS1 | Confirm NixOS as the base and a separate OS repository? | Yes; working name `anima-os`, actual location/name confirmed before creation. Keep shared Anima code in existing repositories. | Open; no repository created |
| OD-AOS2 | Retain this proposal under priority 4.16 or assign a new priority row? | Use 4.16 for the installation deliverable now; do not change its tier or authorize work ahead of its existing tasks. | Open; mapping only, no reprioritization |
| OD-AOS3 | Confirm the provisional graphical plus text local interface from Decision 7? | Yes, sharing one installer backend; no full desktop. | Open; side-chat revisit requested |
| OD-AOS4 | Which named PCs, Intel Macs and ARM boards can be used for validation? | Select available devices covering UEFI, BIOS, ARM boot, networking and optional TPM. Measure minimum resources. | Open; no hardware access or result assumed |
| OD-AOS5 | Authorize bounded feasibility implementation after the OS0 planning brief? | Review AOS-001 to AOS-024, Section 12 and the exact experiment brief. Authorize F1/F2 first without implying approval of F3/F4 contract changes or later implementation. | Decided for the Section 13 F1/UEFI-F2 slice only: R-276; F3/F4 and later work remain unapproved |
| OD-AOS6 | What custom host configuration is a supported public input? | Start with declared packages/services and reviewed modules; define allowed options, privilege boundaries and update tests before advertising arbitrary Nix modules. | Open; reconcile with Decision 16, do not narrow it silently |
| OD-AOS7 | How should the work-route owner reconcile offline continuation and retained work with current lease and cleanup rules? | Preserve Decisions 8/19/21 as the desired outcome; review a per-work-type contract delta, duplicate-execution prevention and separate retained datasets. Do not bypass current K10 rules. | Open; blocks corresponding F4 and OS4 acceptance |

The implementation proposal is complete for review with these open decisions. No new product questionnaire is required to read it. Engineering specifics can be resolved through the approved feasibility work; material changes to the 28 choices require operator decisions. When approved, move this file to the plans directory and update its register/ownership links in the same change.

## 10. Validation, risks and planning integration

The proposal began as documentation only. OS1 now includes a disposable VM image and test harness; the [OS1 record](../research/spare-computer-os/os1-prototype-2026-11-20.md) owns its results. Run `bun run quality:register`, `bun run quality:documents`, `bun run gates:fast` and the prototype-specific checks. The branch/merge-train gates govern landing. Production network integration and backup restore remain unimplemented.

Primary risks: read-only base plus host customization; fallback differences across firmware; TPM/update policy compatibility; missing ARM/Intel Mac drivers; shared storage exhaustion; workload-consistent backups; secret-bearing restore; and existing contracts that prohibit a proposed action. OS0 must reconcile the recent routed-command-line and agent-layer work instead of treating this proposal as an override.

This proposal is registered under the existing installation concern. It adds an OS ownership row and a reference under priority 4.16, not a new active implementation priority. The idea folder is removed on promotion; the following appendices retain decision provenance. Separate backup, permission, runtime or release requirements belong to their existing owners where applicable.

## 11. Prior art and its limits

- [Proxmox installation](https://github.com/proxmox/pve-docs/blob/master/pve-installation.adoc): shared graphical/text installation backend, explicit disk selection and prepared answer files. Anima selects interactive pairing for v1; unattended profiles are later.
- [Proxmox update repositories](https://github.com/proxmox/pve-docs/blob/master/pve-package-repos.adoc): visible updates and production/test distinctions. This is not a claim that Proxmox performs unconditional unattended reboot.
- [VMware maintenance guidance](https://knowledge.broadcom.com/external/article/434515): planned host maintenance and workload interruption, especially on standalone hosts. No Anima live migration is assumed.
- [Proxmox requirements](https://proxmox.com/en/products/proxmox-virtual-environment/requirements) and [VMware compatibility guidance](https://knowledge.broadcom.com/external/article/313743/confirming-esxesxi-host-hardware-system.html): publish support boundaries and evidence. Do not borrow their minimums or claim their certification.
- [OmniXY](https://github.com/TheArctesian/omnixy) and [nascentOS](https://github.com/TheArctesian/nascentOS): operator experience and background only. [NixOS Apple Silicon support](https://github.com/nix-community/nixos-apple-silicon) is a possible later path, not in v1.

Sources above were read during the planning conversation on 2026-11-19. Proxmox, VMware, Caddy, SteamOS and Ubuntu are references for specific behavior, not architecture selections.

## 12. Council review and implementation elaboration

Three independent reviewers examined architecture, security/recovery and delivery. The [council record](../research/spare-computer-os/council-review-2026-11-19.md) summarizes findings and their disposition. This is design review, not runtime validation, formal security approval or authorization to build. All 28 product choices remain intact. The following proposed details elaborate the requirements above; existing owners must approve changes to their contracts.

### 12.1 Resolve contract gaps before integration

- **Offline execution:** [K10 Section 7](../../contracts/work-route.md) ends a build after 90 seconds without an accepted report, permits reassignment and gives runners a separate orphan timeout. This conflicts with unrestricted continuation in Decision 8. OS0 must map each workload type to disconnect, grant expiry, cancellation, reassignment and reconnect behavior. F4 must partition a machine beyond its lease while another worker is available and demonstrate that the approved design prevents duplicate execution and stale-result acceptance. Until OD-AOS7 is resolved, the existing stop policy is a known incompatibility, not proof of OS offline support.
- **Retained work:** K10 Section 9 deletes execution isolation, checkout and work folders at completion and daemon restart. Propose an owner-defined export/commit into retained datasets, separate from secret-bearing scratch state. Do not back up a running job's whole isolation. Test completion, crash and daemon restart: declared retained outputs survive; scratch credentials do not; cleanup cannot escape its authorized dataset. Persistent workspace support requires its own owner-approved lifecycle, not renaming the current disposable work folder.
- **Dependency evidence:** OS0 records each owner requirement/task, approval state, implementation revision, missing delta and verification evidence. A proposal or accepted contract does not establish implemented runtime support. In particular, audit work-route daemon work and hosted-application integration before F4. OS1 records blocked upstream work instead of inventing a substitute scheduler.

### 12.2 State, customization and update transaction

F1 produces a path/dataset inventory: owner, normal mount mode, authorized writer, persistence, snapshot/backup inclusion, restore handling and migration rule. Inspect root, `/etc`, `/var`, `/run`, `/tmp`, `/nix/store`, the Nix database/profiles, boot storage, network state, device identity and workload datasets. Inventory nested subvolumes explicitly; a parent snapshot is not assumed to capture them. Test adding, changing and removing a declared custom package and persistent service through reboot and fallback. Record every writable exception.

Proposed activation states are: staged, validated, waiting for maintenance, draining, candidate boot, local verification, committed; each has a recorded failure exit. Pin the candidate and previous known-good configuration and closures. Verify designated mounts, daemon/local control and local network configuration before blessing the candidate; hub reachability alone is not a health gate. Define whether each custom service is host-critical instead of making every app failure roll back the OS. Check persistent-state compatibility before activation and prove that the fallback can read state left by the candidate. Incompatible migrations block activation unless a separately authorized recovery strategy exists.

Separate workload, daemon/link and privileged host-activation principals. A daemon token alone is not host-change approval. Bind approval to a canonical change, machine, base configuration revision, scope and expiry; revalidate at activation. Test payload substitution and attempts to alter boot, trust, update or recovery settings from workload and daemon identities. Declarative Nix text is not inherently a bounded permission. OD-AOS6 determines the supported customization surface without removing custom packages/services from scope.

OS0 also specifies build/evaluation placement, accepted sources, artifact authentication, resource limits and missing-binary/architecture behavior. A missing artifact must not silently trigger an unrestricted local build. Failed customization leaves the previous configuration active; an update cannot silently drop the owner's declared configuration to pass validation.

### 12.3 Installation and recovery trust

F3 defines one durable installation attempt with explicit states for pairing, approved disk plan, destructive installation, installed-identity handoff and completion/recovery. Bind authorization to the owner, live-session identity, disk fingerprint and plan revision. Revalidate immediately before destructive writes. Define the surviving identity and retirement of live credentials, accounting for [K8 enrollment](../../contracts/daemon-push.md) code consumption and token replacement.

Test dropped responses, concurrent approvals, expiry, disk substitution, power loss before and after the first write, installed-enrollment failure and booting the same USB again. A status retry must not repeat erasure. Neither normal enrollment retry nor restoring a backup may silently replace ownership.

OS0 defines offline local credential provisioning/reset, available recovery operations and audit reconciliation. Physical presence does not silently authorize ownership transfer or erasure. OS5/OS6 classify device/link keys, daemon tokens, administrator credentials, workload secrets and backup keys as restore directly, restore after authorization, regenerate or exclude. Restore into quarantine before services or publication resume. Test source loss, source still online, revoked credentials and custom-service restoration; do not create two active copies of a device identity.

Backup design separates production, restore and deletion authority. Define key export, trusted-device addition/removal, rotation and recovery across rotation. Removing a trusted device cannot retract keys or plaintext it already received. Test a destination that stores but cannot decrypt backups, missing keys and source-independent recovery.

### 12.4 Independent experiment gates

| Evidence gate | Prerequisites | Added pass evidence | Current state |
| --- | --- | --- | --- |
| F1 state/customization | Approved bounded brief; selected experimental base | Mount inventory and custom-service lifecycle above; normal write restrictions and interrupted activation | Fixture proof recorded in OS1; real network/identity integration untested |
| F2 recovery | F1 candidate; boot-path-specific harness | Rejected boot entry, early kernel hang, userspace health failure and shutdown hang; identify what forces the next boot, timeout/retry bounds and outcome when both versions fail | Partial: local-health fallback and bounded recovery proved; base initrd and shutdown hang cases fail; see OS1 |
| F3 installation | Approved identity/installation contract deltas; F1/F2 base | Transaction and cutover tests in Section 12.3; unselected-disk hashes unchanged | Not started |
| F4 workload/storage | Approved owner runtimes and OD-AOS7 resolution for affected behavior | App/job/agent coexistence; partition/reassignment and retained-state tests; workload writes, snapshots and update staging competing for shared capacity | Not started |

F2 must distinguish selecting a fallback on the next boot from forcing a hung machine to restart. Record watchdog availability and cases requiring physical intervention per boot path. UEFI success does not establish BIOS or ARM recovery.

F4 includes data and metadata pressure, retained snapshots and cleanup that frees less space than expected. Demonstrate continued management, diagnostics and recovery capacity; a subvolume boundary or free-space alert is not a reserved allocation. M5 implements and qualifies the proven capacity mechanism.

Before each experiment, fix iteration counts, injection points, resource limits, management response/error thresholds and pass/fail rules in its brief. Report measurements and exact revisions; do not invent public minimum resources from an unrun test.

The recommended first authorized implementation is F1 plus the UEFI slice of F2 on disposable x86-64 VM disks: shared Btrfs, read-only normal root, identity/configuration and workload sentinels, one declared custom service, interrupted activation and failed-health fallback. It proves only that narrow architecture claim. UI selection and named physical hardware do not block this experiment; F3/F4 and physical qualification retain their separate gates. OS0 remains planning work; Section 13 completes its bounded dependency audit and brief. Neither the council nor the brief authorizes OS1.

### 12.5 Qualification and preview delivery

Start the hardware matrix during feasibility: exact model/firmware, architecture, boot path, storage/network devices, image digest, scenario, result/log, limitation and human-check reference. Distinguish unavailable hardware from pass/fail. All selected families remain stable targets; infeasible support returns to the operator.

A preview after M2 needs its own evidence packet: exact digest/source revision, passed install/pair/unselected-disk checks for named preview targets, recovery/removal instructions, absent milestones clearly listed and explicit publication authorization. OS7/OS8 accept this preview packet independently of the later stable packet. Stable still requires M1-M5 and the selected hardware evidence.

Write installation instructions with M2 and recovery instructions with M4, then validate them on a clean target. The [user documentation owner](user-documentation.md) proposes `docs/user/` for user-facing sources; the OS repository's documentation should hold developer/build material and link to those guides rather than create a second user manual.

## 13. Overnight prototype implementation brief — OS0 acceptance record

This section is the deliverable requested by “finish it off” and “enough so we can do an overnight build.” It prepares one bounded implementation run. It does not claim that a complete supported OS can be built overnight. The first run targets F1 and the UEFI portion of F2; all other product commitments remain in the stable scope.

### 13.1 Dependency snapshot and changes required

Audit basis: proposal revision `39db123c`, based on `origin/dev` revision `f30663a6`, inspected on 2026-11-20. These are source/plan observations, not fresh runtime verification. Refresh the table before integration if owner plans have changed. Branch-only implementation is not assumed present in the base.

| Dependency / owner | Observed state at audit | Missing work / gate | Effect on overnight run |
| --- | --- | --- | --- |
| [K8 enrollment](../../contracts/daemon-push.md), daemon/link | Enrollment contract and daemon sources exist; K8 only permits its declared interface plus K10. Work-route W0 lists branch implementation awaiting human/hosted checks. | Installation-session authorization and live-to-installed identity transaction require reviewed contract changes. Existing enrollment alone does not authorize disk erasure. | No real hub enrollment, account or credentials. Use explicitly synthetic identity sentinels. F3 waits. |
| [Work execution W2-W4](../plans/projects-and-builds-plan.md) and [K10](../../contracts/work-route.md) | W2 isolation evidence incomplete for Linux; W3 queue partly implemented on its branch; W4 daemon execution Not started. Wire types exist in `crates/daemon-core/src/wire/work.rs`; this is not proof of an execution engine. | Approved Linux isolation, integrated queue/daemon, offline and retention decisions below. | No real jobs or replacement scheduler. F4 waits. |
| [Hosted applications H28/H31/H32](../plans/hosted-applications-plan.md) | Compose validation, applications program and installer tasks Not started. | Owner runtime and storage/capture hooks; their W3/W4 dependencies. | A synthetic persistent test service exercises mounts only; it is not Anima app support. |
| [Agent runs](agent-runs.md), [routed command lines](routed-command-lines.md) | Draft proposals; routed execution has reviewed findings and pending operator decisions. | Approved agent execution/permission contract and implementation. | No agent shell endpoint, model credentials or agent execution in the VM. |
| [Updates U3-U6](../plans/daemon-and-hub-update-plan.md) | U4 contract Done; U3 prototype, U5 rollout rules and U6 integration Not started. | OS-owned daemon/link version-set ownership and rollout integration. | Exercise local image activation with a test health endpoint; no competing product updater or rollout API. |
| Backup/restore and local recovery | OS requirements proposed; no selected appliance backup engine or validated key lifecycle in this audit. | Capture/restore inventory, destination adapters, authorization and recovery evidence. | Keep data sentinels through OS fallback; do not label this backup or replacement-machine restore. |
| User guides and hardware | Existing documentation owner; no named OS reference machines or measured minimums. | Hardware selection and physical human checks. | Write developer reproduction and VM recovery instructions; mark every physical family untested. |

Proposed contract deltas for owner review, outside the overnight code scope:

1. **Offline work:** add an explicit per-attempt execution policy distinguishing lease-bound work from work authorized to finish disconnected. For a disconnected-continuation attempt, the hub must not automatically reassign merely because its heartbeat expires. Specify a maximum authorization lifetime, locally enforced limits, cancellation delivery and reconciliation of a terminal result. A replacement attempt must wait for proof the old execution ended, effective fencing, or an explicit owner-approved duplicate-risk policy. Do not claim exactly-once side effects from queue bookkeeping. Existing lease-bound behavior remains until the owner approves a change. Apps, runners and agents need their own explicit mapping; one build-policy field is not enough.
2. **Retained state:** define a retained dataset identifier, owning workload, authorized writers, quota/retention class and an explicit export/commit operation. Execution scratch and temporary secrets retain mandatory cleanup. Persistent workspaces need a separate approved mount/lifecycle. Publish retained-output metadata only after commit; incomplete exports stay visibly incomplete. Backup includes committed retained datasets, not the execution sandbox. Completion, daemon crash and restart must preserve committed outputs while removing temporary secrets.
3. **Installation:** define attempt identifier, live identity, owner, disk fingerprint, approved plan digest, expiry and state transitions. Separate pairing approval from erase authorization; consumed authorization and status retries must be idempotent. Specify token replacement and the installed-device identity handoff under K8. Disk substitution, conflicting approval and uncertain state stop destructive progress.
4. **Host activation:** define a canonical configuration revision and diff, approval/grant binding, privileged activation principal, maintenance authorization and audit record. The request cannot accept an arbitrary command hidden inside a bounded package grant. The OS update owner and existing updater must agree which controls the packaged daemon/link versions.

These are reviewable design proposals, not amendments to K8/K10. OD-AOS7 remains open because the current rules cannot be changed by this brief.

### 13.2 Fixed first-run scope and preflight

Build a local x86-64 UEFI/QEMU prototype from a pinned NixOS stable input. The runner resolves the then-supported stable input from official NixOS sources, records its exact revision in a lockfile and does not change it mid-run. This is an experimental base choice, not final approval of the distribution/repository decision.

Use a dedicated prototype directory in the authorized implementation worktree. Do not create a remote repository, alter the operator's desktop/system configuration, publish artifacts, write physical block devices or enroll a production account. Use a new local repository only if its location is explicitly selected before the run; otherwise keep the experiment under the existing worktree. The later `anima-os` repository decision must not block a self-contained experiment.

Preflight records available disk/RAM, host architecture, Nix, QEMU, UEFI firmware and acceleration. On the current planning shell, `nix` and `qemu-system-x86_64` were not found on PATH. Therefore this machine is **not yet verified build-ready**. The implementation run must provision an explicitly authorized build environment or use an already prepared one before claiming it can execute overnight. Do not silently install a system daemon or change host services. KVM absence permits a documented slower emulation run; it does not justify skipping tests. If tooling cannot be obtained within scope, deliver the source and exact setup blocker without claiming a built image.

Proposed guest test budget: 2 vCPUs, 4 GiB RAM, a 24 GiB disposable primary virtual disk and a second 1 GiB sentinel disk. These are test-fixture settings, not product minimums. Confine disk paths to a freshly created experiment directory; reject block-device paths, symlinks outside that directory and existing unrelated images. Use no guest access to host home, SSH agents, production secrets or writable host-directory shares.

Deliver a bootable VM disk image and reproducible build/test commands first. A downloadable installer ISO, graphical setup, BIOS/ARM boot, TPM/encryption, pairing, real workloads, full backup and stable release are outside this first run. Do not spend the run making a decorative interface or empty repository scaffold.

### 13.3 Implementation order and acceptance protocol

The runner creates task rows/claims through the repository workflow once OS1 is authorized. Implement in this order and retain useful partial results when a later step fails:

| Step | Concrete work | Test and pass criterion |
| --- | --- | --- |
| 1. Build inputs and disk | Pin flake inputs; define UEFI boot, one shared Btrfs filesystem and explicit root/state/store/boot write boundaries. Add a serial console. | Build command exits successfully; manifest records source/lock/image hashes and tooling versions. Boot reaches the local test health endpoint within 180 seconds under accelerated test conditions. Slower emulation requires a predeclared recorded timeout, not a retrospective pass. |
| 2. State and custom service | Add synthetic identity/configuration, workload data and a declared stateful test service; document the mount inventory. | Five clean reboot cycles preserve hashes of identity/configuration and workload sentinels; normal root is mounted read-only. Both an unprivileged workload and the simulated daemon identity fail base/store/boot writes. The maintenance path can make an authorized change. |
| 3. Configuration lifecycle | Add, update and remove one declared package/service configuration through the controlled local path. | Each operation completes once; reboot uses the requested revision; service data remains in its designated dataset. A rejected configuration leaves the known-good revision active. No unrestricted daemon-user activation permission. |
| 4. Candidate activation | Stage version B from known-good A, with explicit pending/verified/committed state and protected fallback. | Three A-to-B activation cycles succeed; candidate and fallback identities are observable. A deliberate required-health failure recovers A without reverting the workload sentinel. Local health does not depend on internet or hub availability. |
| 5. Failure injection | Cut VM power during staging, boot-entry publication and before health commit; separately test a rejected boot entry, early boot hang and failed userspace health. | Run each case three times from a clean fixture. Every case ends in a known revision or a clearly reported recovery failure; no silent pass or destroyed sentinel. Automatic recovery passes only if the guest restarts without test-harness reset. Proposed limit: two candidate attempts, then one fallback attempt; if both fail, bounded recovery state with no reboot loop. Record any watchdog gap as a failed F2 case. |
| 6. Capacity smoke test | On a clone, fill disposable workload storage while retaining a known-good system and attempt a rejected or safe update staging operation. | Within 30 seconds the local health/status path responds or reports a measured failure; no protected-state deletion, partial active configuration or sentinel change. Record Btrfs data/metadata usage. This preliminary result does not complete F4/M5 capacity qualification. |
| 7. Reproduction and report | Rebuild/evaluate from the recorded lockfile, run smoke tests from a clean VM copy and inspect the unrelated disk. | Manifest and configuration identity are reproducible; do not claim bit-for-bit image reproducibility without measuring it. Sentinel disk hash is unchanged. Report every case as Passed, Failed, Blocked or Not run, with logs and reasons. |

Host-side test probes may observe serial output and terminate a timed-out VM to collect evidence. Such intervention must be recorded and cannot count as autonomous recovery. Guest internet may be used for build preparation only as documented; disconnect it during the local-health recovery case.

### 13.4 Stop conditions and morning handoff

Treat “overnight” as one bounded run, proposed maximum eight hours after tooling is ready. Check elapsed time between build/test stages. Stop launching new long work with 30 minutes left and use that time for the report and cleanup. The deadline is not permission to mark unfinished cases passed. No self-scheduled continuation or remote deployment is part of this brief.

Stop dependent work on a missing prerequisite, an unresolvable failed F1/F2 condition, a request for credentials unavailable to the runner, or a required change to a frozen contract. Continue independent documentation or test harness work that remains within scope. Do not repeatedly rebuild identical inputs after an unchanged failure; record the cause and change hypothesis before another attempt.

Morning deliverables:

- Exact source commit, lockfile, build command and environment inventory.
- VM image path, size and SHA-256 if built; otherwise the first build failure and reproduction command.
- Mount/state inventory, local activation states and boot-failure result matrix.
- Test logs with no credentials, sentinel hashes, timings and automatic-versus-harness recovery distinction.
- One-command VM launch and test instructions, plus how to remove only the disposable artifacts.
- A short verdict: F1 pass/fail/blocked, UEFI F2 pass/fail/blocked, what can be demonstrated, and the next specific fix. BIOS, ARM, physical machines, pairing and real Anima workloads remain explicitly untested.

Commit source/docs and leave large VM artifacts outside git. Push only the authorized implementation branch through normal gates; publishing an image or merging to `main` requires separate release authorization. Update OS1 with evidence for the slice actually attempted, never mark all F1-F4 complete from this run.

OS0 is complete as planning work through this record. The operator authorized this OS1 slice on 2026-11-20 (R-276). The build environment is the local `framework` host with disposable QEMU disks; preparation and claim are recorded on the implementation branch. The broader product decisions need not be reopened for the VM experiment.

## 14. Installed console and motion engineering

The operator approved the installed-console mockup, browser-based WorkOS sign-in direction, local lock and power controls, and consistent icons. They then requested the motion-audit skill and asked that the premium feel be engineered into this plan (R-275). This section is part of OS3, not a separate product plan. Implementation and experimental ISO rebuild are authorized by R-271. The installed console and automatic enrollment integration are now implemented; [build and validation evidence](../research/spare-computer-os/console-preview-2026-11-20.md) distinguishes VM checks from pending live-account and hardware qualification.

### 14.1 Design baseline and evidence

The approved local design sources are `/home/okita/.local/share/anima-os-designs/local-console/anima-local-console.html` and its standalone `anima-local-console-preview.html`. Preserve their spacing, proportions, Graphite surfaces, light IBM Plex headings, white text, frost accent, coloured wordmark and Carbon icons. Lock, restart and shutdown use icon controls with accessible names and tooltips; primary actions retain useful text. Bring the approved sources into the repository's mockup inventory with the implementation so that the baseline is reviewable on other machines.

The console covers Overview, Network, Anima, Storage, Display and Recovery. WorkOS sign-in returns from the browser, then the owner explicitly adds the computer. Reuse existing identity contracts; animation must not imply enrollment before the backend confirms it. Local recovery and authenticated unlock work without the hub. Lock protects the local session while the node, authorized remote access and workloads continue; restart and shutdown remain separate confirmed actions.

Static source inspection on 2026-11-20 found that `draw()` replaces the main content with `innerHTML`, while dialogs appear and disappear by replacing markup and setting `hidden`. These are motion opportunities, not measured runtime defects. All mockup hardware, authentication and operation results are synthetic. No frame-rate, physical-device or production sign-in success is established by this design.

Use the operator-selected skill at `/home/okita/Scripts/Personal/skills/motion-audit/SKILL.md`, including its measurement and build references. The baseline vocabulary is [Anima's motion tokens](../../design/tokens/motion.tokens.json) and design-spec Sections 5.5/5.7: press 100 ms, fast/exit 150 ms, crossfade 200 ms; critically damped default/snappy/move springs with response values 0.35/0.25/0.4 seconds. Spring response is not a fixed completion deadline. Reuse the web motion helpers where applicable; for Qt, map the same roles explicitly and verify perceptual parity. Do not copy approximate constants into each screen or introduce a second token owner. These are starting values to validate, not evidence that the finished interface feels right.

### 14.2 Motion storyboard

Motion communicates state, continuity and response. It must preserve the approved layout rather than compensate for cramped spacing. Proposals below are implementation candidates; the measured audit can amend or drop them.

| Surface and meaning | Proposed behaviour | Interruption and reduced motion |
| --- | --- | --- |
| Navigation and first arrival | Keep the shell fixed; crossfade outgoing/incoming content using the base role. Use a snappy selection indicator only where it clarifies the destination. Avoid repeated whole-page slides or staggered forms. | Latest navigation wins; cancel stale arrivals. Reduced motion uses an immediate content swap with stable layout and focus. |
| Buttons and icon controls | Press feedback uses the press role; hover colour uses fast. Reserve space for pending indicators and preserve the action label. Focus indication is immediate. | Release/cancel restores the current state. Disable duplicate submissions while preserving cancellation where supported. Reduced motion removes transforms and animated feedback. |
| Dialogs, confirmation and dismissal | Coordinated backdrop and restrained panel arrival; retain the outgoing panel through its fast exit. Keep typography and controls aligned with the app. | Escape and repeated dismissal leave no orphan overlay; restore focus and make background content inert. Reduced motion opens/closes immediately. |
| WorkOS browser handoff and enrollment | Show truthful waiting, return, add-computer and confirmed-connected states within a stable content region. Crossfade only after each real state change. | Cancellation, stale callbacks and network loss cannot produce a false success. No artificial success delay or animation-dependent authentication. Reduced motion retains every status. |
| Wi-Fi scan, connect and diagnostics | Preserve adapter/network identity and stable rows. Show scan, authentication, address and connectivity stages separately; update only the affected status. | Rapid selection and late results cannot overwrite the chosen adapter. Reduced motion updates text/icons immediately; never rely on a pulsing colour alone. |
| Disclosures, storage and async results | Preserve scroll position and stable row identity. Reserve realistic loading space; use the move role for meaningful rearrangement and base crossfade for replacement. | Retarget from the current visual position; no snap-back on repeated input. Reduced motion keeps the same geometry guarantees without travel. |
| Display orientation and recovery | Keep the confirmation readable after rotation. Show a real deadline for automatic reversion; use stable numerals and immediate error states. | Reversion and recovery continue even if rendering is interrupted. Reduced motion changes neither deadline nor outcome. |
| Lock, restart and shutdown | Lock conceals sensitive content immediately; an optional surface fade must occur only after concealment. Power confirmation communicates affected workloads and then actual backend state. | No animation may delay locking, unlock authorization or shutdown dispatch. Reduced motion is immediate; locking never sleeps the node. |
| Door artwork | Optional restrained motion on welcome/completion only, subordinate to content and contingent on measured cost. Forms remain still. | Stop when hidden or locked; use a static image with reduced motion. Drop the effect if it competes with workloads or readability. |

Do not animate health readings, security decisions or progress into invented intermediate values. No fake percentages, celebratory effects, bouncing controls, continuous decorative activity on status screens, or motion-dependent waits. Loading perception thresholds are distinct from animation durations; reduced motion must not collapse them into flashing loaders or delay completed operations.

### 14.3 OS3 execution sequence

| Step | Deliverable and acceptance | State |
| --- | --- | --- |
| OS3-M1: baseline and storyboard | Inventory console and installer routes, shared tokens/helpers, exits, async states and reduced-motion gaps. Record static versus measured evidence. Check concurrent motion branches and assign one shared-foundation owner. Rank opportunities by frequency × visibility × meaning divided by cost, with explicit scores and unknowns. | Static starting observations recorded above; measured audit not started |
| OS3-M2: motion reel | Build a synthetic looping reel of up to 8–12 useful proposals using the approved product layout. Specify each beat, inspect a frame from every beat, and reconcile it with the report before full rendering. Demonstrate normal and reduced-motion behaviour. Store exported media outside git and record its artifact location. | Not started |
| OS3-M3: shared foundation | Define arrival, exit retention, async-action feedback and cancellation APIs using existing tokens. Map any native equivalents. Preserve keyboard/focus semantics and dynamic reduced-motion changes. Review the foundation before screen integration. | Implemented under R-271; local VM verification recorded |
| OS3-M4: screen integration | Implement the storyboard against real console operations, preserving the approved spacing and icon treatment. Re-measure each proposal first; document amendments or dropped effects. Review code and behaviour after each implementation step. | Experimental console implemented and reviewed; live-account verification pending |
| OS3-M5: measured qualification | Publish a ranked audit report with route inventory, durations, sampled properties, interruption results, reduced-motion results, device/build details and unresolved limits. Compare running installer and installed console against the approved design. | VM visual comparison and local behaviour checks recorded; physical motion qualification and full audit remain open |

Audit outputs belong under `docs/research/spare-computer-os/`; the final report must distinguish static inspection, synthetic local runtime, and physical hardware evidence. Each retained proposal specifies property/from/to, motion role, mechanism, choreography, cancellation, reduced variant, performance risk and verification. The reel illustrates proposals; it is not proof that production code implements them.

### 14.4 Completion gates

- Exercise first paint, navigation in both directions, slow/fast/failed operations, empty states, dialogs, disclosures, power confirmation and lock/unlock at compact, medium and wide sizes, including portrait orientation.
- Interrupt transitions with rapid navigation, Escape, back and repeated activation. Delay responses and count requests: one confirmed action must not submit twice. Verify retained exits do not block input or leave hidden originals, stale overlays or misplaced focus.
- Sample both position axes after input and under delayed results; a layout-shift score alone can miss interaction-induced movement. Inspect intermediate frames for overlapping text and doubled elements. Verify final screenshots against the approved proportions.
- Exercise reduced motion at startup and while a transition is active, covering CSS, JavaScript, pseudo-elements and native effects. All functions remain available; no operation or deadline depends on animation completion. Confirm keyboard focus, accessible icon names and non-colour status feedback.
- Measure frame pacing, long tasks, paint activity and idle cost on the selected physical spare hardware with representative workloads running. Target the device refresh budget (about 16.7 ms per frame at 60 Hz); publish measured results and limitations rather than treating headless rAF rate as hardware performance. Remove or simplify effects that impair management responsiveness. Hidden/locked decorative surfaces must stop painting.
- Validate real backend states separately from appearance: browser return/cancel, enrollment failure, offline recovery, Wi-Fi failure, authentication rejection, display rollback and power actions. Synthetic success in a reel or mockup is not acceptance evidence.
- Run relevant existing gates and behavioural regression checks after implementation, then conduct a visual review of the running build. Passing code tests alone does not establish premium motion. Human hardware checks stay pending until the operator records their results.

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

## Overnight ISO continuation (2026-11-20)

R-274 authorizes the next experimental slice: package existing runtime components, build a guided offline UEFI installer ISO and validate installation on disposable VM disks. Review recovery, runtime packaging, installer safeguards and final image in that order. The existing daemon provides enrollment/monitoring; app/job/agent execution integration is not implied by packaging it. Named hardware, BIOS/ARM, encryption, backups, hub-driven installation and production workload integration remain explicit qualification gaps until implemented and tested. Keep earlier failure records and publish a separate continuation report.

Continuation result: [experimental ISO delivery](../research/spare-computer-os/iso-preview-2026-11-20.md) and [reviews after each stage](../research/spare-computer-os/overnight-reviews-2026-11-20.md). The ISO installs offline and runs local isolated container workloads. It does not complete managed job/agent execution, production update/backup policy or physical qualification.

### Installer display-space correction (2026-11-20)

Physical feedback showed the fullscreen installer using tiny controls and a fixed welcome composition on a high-resolution panel. The operator requested better screen use on every page. Apply one resolution-aware scale to all eight installer steps and all installed-console pages, dialogs and navigation; expand form controls within a readable width and make welcome/completion artwork follow the page. Keep small-screen usability and disk authorization unchanged. Validate both a 1280×800 viewport and a high-resolution viewport; physical readability remains an operator check.

Installed hardware feedback also reported Wi-Fi working in the live installer but unavailable after installation, and confirmed rotation being lost. Reproduce the installed read-only-root Wi-Fi service startup separately from live-media tests. Persist only confirmed rotations, transfer them during installation, apply them before the installed GUI starts and retain the timed rollback. Do not treat VM backend activation as physical radio qualification.

### Installer orientation-panel follow-up

The operator reported that changing orientation can leave the settings panel off-center with Done unreachable. Keep the panel within the current screen's available geometry when rotation or screen changes occur, allow its body to scroll on short displays, and keep Done visible outside the scroll area. Preserve the existing confirmation countdown and automatic orientation rollback. Validate rotation while the panel is open in a disposable VM; physical display and touch behavior remain human checks. The [rotation-panel record](../research/spare-computer-os/rotation-panel-2026-11-20.md) owns the regression evidence and delivery details.

### Current preview follow-up: installed operation

The operator requested one combined build on 2026-11-20: compact sign-in browser,
provider-neutral copy, visible self-hosted enrollment, completion power actions,
battery icons in both headers, local telemetry, corrected hub storage reporting,
and hosted OS updates so subsequent changes do not require reinstalling. This is
an explicitly authorized continuation of OS3 and an experimental OS5 update slice,
not completion of the stable release gates. Hosted preview artifacts use a separate
OS namespace and signing identity; desktop release feeds and `main` stay unchanged.
The update slice must verify signed metadata and artifact bytes, preserve the last
bootable system and persistent state, require deliberate activation, and be tested
on disposable installed VMs before delivery. Existing running installations need a
bootstrap path; the requested next fresh install carries the updater by default.

The operator also selected `anima-node-xxxx` for installed machine names. Generate
the four-character suffix once per installation, persist it through reboot and OS
updates, and show the actual name in the console and new hosted device records.
Keep the longer enrollment identity separate from this human-readable hostname.
