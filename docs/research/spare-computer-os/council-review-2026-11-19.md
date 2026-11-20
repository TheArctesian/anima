# Record: Anima OS council review (2026-11-19)

Three independent agents reviewed version 0.1 of the [OS proposal](../../proposals/spare-computer-os.md). Their briefs covered architecture, security/recovery and delivery respectively. They read the proposal and relevant repository owners/contracts; none edited files or ran OS experiments. The coordinating agent reconciled their findings into version 0.2, Section 12. This record summarizes their reports; it is not a verbatim transcript, formal security approval or a source of new requirements.

The council found the product direction coherent, with unresolved engineering and contract gates. No reviewer recommended silently changing the 28 product decisions. All implementations and hardware behavior remain unverified.

## Findings and disposition

| Finding | Reviewers; severity; CUPID property | Disposition in proposal v0.2 |
| --- | --- | --- |
| K10 stops disconnected builds after a 90-second lease and permits reassignment; desired offline continuation needs reconciliation | Security, delivery; high; Predictable | Section 12.1; OD-AOS7; F4 partition/reassignment test. Existing contract remains in force. |
| Current job cleanup removes work folders; retained work needs a separate lifecycle | Security; high; Domain-based, Predictable | Section 12.1; owner-defined retained datasets/export and completion/crash tests; no wholesale sandbox backup. |
| Boot fallback selection does not itself restart a hung machine | Architecture; high; Predictable | Section 12.4; separate hang cases, watchdog/restart evidence, retry bounds and physical-intervention limits by boot path. |
| Candidate activation may change state before rollback; health and compatibility boundaries are unclear | Architecture; high; Composable, Predictable | Section 12.2; proposed activation states, pinned fallback and persistent-state compatibility tests. |
| Read-only root needs an exact mount and activation-write inventory | Architecture; high; Domain-based, Predictable | Section 12.2; path owners/writers, persistence and restore classification; custom package/service lifecycle tests. |
| Shared capacity could exhaust recovery space before late cleanup work | Architecture; high; Predictable | Section 12.4; early data/metadata/snapshot pressure experiment; no claim that subvolumes reserve space. |
| Enrollment cutover can fail after code consumption or disk writes | Security; high; Predictable | Section 12.3; durable attempt, owner/disk/plan binding, identity retirement and interrupted/replayed transaction tests. |
| Daemon identity is not sufficient authority to activate arbitrary host configuration | Security; high; Composable, Predictable | Section 12.2; separate privileged activation boundary, canonical change approval, revision/expiry checks; OD-AOS6 stays open. |
| OS0 approval was circular; OS1 combined independent gates | Architecture, delivery; high; Composable, Predictable | Task dependencies revised; OS0 planning brief precedes bounded experiment approval; independent F1-F4 states. UI/hardware do not block disposable VM proof. |
| Custom configuration lacks build placement and missing-artifact behavior | Architecture; medium; Composable, Predictable | Section 12.2; build/source/resource/authentication design outputs, no silent unrestricted build or dropped customization. |
| Restore could duplicate device identity or lose essential credentials | Security; medium; Domain-based, Predictable | Section 12.3; credential inventory, quarantine and source-still-online/revocation tests. |
| Offline recovery authentication and backup-key lifecycle are unspecified | Security; medium; Predictable | Section 12.3; credential provisioning, operation scope, key roles/rotation and trusted-device removal limitations. |
| Preview after M2 contradicted OS7/OS8's all-milestones prerequisite | Delivery; medium; Predictable | Separate preview and stable evidence/publication gates; both require explicit release authorization. |
| Hardware evidence and user guides arrived too late | Delivery; medium; Domain-based, Predictable | Section 12.5; evidence matrix begins early; install/recovery guides accompany their features and retain the existing user-documentation owner. |
| Terms such as responsive and repeated boot lack a pass protocol | Delivery; medium; Predictable | Section 12.4; predeclared iteration counts, injection points, resource/response bounds and measurements before a pass claim. |

## Evidence and limits

The decisive local sources were [K10 work route, Sections 7 and 9](../../../contracts/work-route.md), [K8 enrollment and identity assumptions](../../../contracts/daemon-push.md), [projects/builds tasks](../../plans/projects-and-builds-plan.md), [hosted applications](../../plans/hosted-applications-plan.md), and the [user documentation proposal](../../proposals/user-documentation.md). The review did not amend their rules or establish implementation readiness.

The architecture reviewer consulted primary documentation: [systemd automatic boot assessment](https://github.com/systemd/systemd/blob/main/docs/AUTOMATIC_BOOT_ASSESSMENT.md), [systemd watchdog settings](https://www.freedesktop.org/software/systemd/man/259/systemd-system.conf.html), [Btrfs subvolumes](https://btrfs.readthedocs.io/en/latest/Subvolumes.html) and [Btrfs quota groups](https://btrfs.readthedocs.io/en/latest/Qgroups.html). These support the distinction between boot selection and restart, and the need to measure shared-capacity/snapshot behavior. They do not prove the proposed image works.

## Recommendation

Complete the OS0 dependency map and exact experiment brief, then obtain bounded implementation approval for F1 and the UEFI slice of F2. Produce the mount inventory, activation state model, boot-failure matrix and reproducible evidence. Gate enrollment and real workloads on their own contracts. Do not call a booted VM a supported server, reopen a long product questionnaire, or reduce the stable hardware/workload commitments without a visible operator decision.
