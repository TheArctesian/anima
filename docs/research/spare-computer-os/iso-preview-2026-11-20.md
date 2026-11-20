# Anima OS experimental ISO — 2026-11-20

Authorized by R-274. This is an installable experimental x86-64 UEFI image, not the supported first release or completion of the 28 product decisions. [Build/use instructions](../../../experiments/anima-os/ISO.md) and the [stage-by-stage code reviews](overnight-reviews-2026-11-20.md) accompany it. Reviews were performed directly by the implementing agent; no independent council re-review is claimed.

## What is built

- Bootable live ISO with local console, NetworkManager setup and an explicit guided installer. Installation works without a network connection.
- Whole-disk GPT installation: 1 GiB EFI plus shared Btrfs with distinct root, store, variable-state and persistent-data subvolumes. Fixed labels are a preview constraint; another matching installation is refused.
- Mounted/swap disks and active mapper children excluded; disk identity checked again after password/erase prompts; exact disk-specific erase confirmation required. No unattended installation path.
- Read-only root after controlled boot activation, persistent admin home/network state, locked installed root login and a password chosen during installation. No credentials are baked into the image.
- The real Anima daemon, packaged from the pinned repository toolchain/lockfile, retaining the existing unprivileged service and private-network restrictions. Local enrollment helper and status command are included. Actual account enrollment was not attempted.
- Rootless Podman and an offline smoke image, demonstrating an isolated local app and finite job. This is not Anima-scheduled execution, an agent run or public app hosting.
- Server-style idle/lid policy and bounded reboot transaction after reboot is requested. No automatic maintenance scheduler is included.

## Artifact

Final ISO: `/home/okita/.local/share/anima-os-prototype/2026-11-20/delivery/anima-os-preview-x86_64.iso`. Size: 1,686,224,896 bytes (about 1.6 GiB).

SHA-256: `3cb19d776edab956579b4f44c25429a9a3c448c70ef5fff76dba62aac5fa9aff`.

All eight checks in the [final artifact test record](iso-preview-results.json) passed in `iso-test-008`. The delivery folder includes `SHA256SUMS`, `INSTALL.md`, `manifest.json` and `test-results.json`. The build input remains NixOS 26.05, nixpkgs `3457b69ac4aba6d9ea9a743370af160fb502db29`. Runtime build metadata in the package records its source revision and binary/lockfile hashes. This is a local preview build, not a signed release or a claim of bit-for-bit reproducibility.

## Verification

The actual ISO test uses QEMU/KVM, OVMF, two vCPUs, 4 GiB RAM, a 64 GiB target and an unrelated 32 GiB sentinel disk, with **no NIC** and no host filesystem share. It verifies mounted-child exclusion, cancellation before erase, offline installation, boot with the ISO removed, admin login, normal root read-only, writable home, daemon executable loading, no automatic enrollment, isolated container app/job behavior and persistence after reboot. It hashes the entire unrelated disk before and after.

The first successful install/reboot run was `iso-test-004`. Expanded runtime/safety checks passed in `iso-test-006`; final artifact checks all passed in `iso-test-008`, including explicit rootless operation, a refused container-root write and machine identity preservation. Early failures are retained rather than reported as passes: flat disk inventory, password-file creation API misuse and a test filesystem auto-detection problem. The fixes and review findings are in the review record.

Recovery is separately evidenced by [the new core run](recovery-results/core.json), [initrd failures](recovery-results/initrd-hang.json), [shutdown hangs](recovery-results/shutdown-hang.json) and [idle behavior](recovery-results/idle.json). All three initrd and all three shutdown-hang trials recovered without harness reset; shutdown recovery took 75–78 seconds. Five persistence reboots and three healthy/failed activation cycles passed. The recovery image SHA-256 is `40825ede35a7f372aac17920de28823fb5600dc8332265631dc12252315f85bb`. This separate fixture does **not** establish a production update coordinator in the installed ISO. Pre-initrd kernel deadlock and physical watchdog support remain unqualified.

Repository checks: all 21 gate categories have passing local evidence across runs. A full run encountered a transient PostgreSQL failure: 5,690 tests passed before seven failures and a stop. Targeted completion runs passed 2,514 tests and the remaining 11 hub-binary tests. Comparing test identities with the prior full suite confirms passing coverage for all 6,468 enabled Rust tests. The other 18 gate categories passed together; formatting and Clippy passed in the initial run. This is not one uninterrupted green full run. Eleven Python harness/installer regression tests, shell checking and Nix evaluation/build checks also pass. GitHub checks remain separate landing requirements.

## Remaining work

The ISO is an experimental delivery, not completion of OS2–OS8. Still unimplemented or unqualified:

- Secure pairing/install orchestration from Anima and graphical local setup.
- Anima-managed apps/jobs/agents, permission reconciliation, offline execution semantics and resource-profile scheduling. The monitoring daemon is not a substitute execution engine.
- Signed/update delivery, production health policy, automatic maintenance, preserving-data repair, reset, backup/restore and optional encryption.
- BIOS installation, Intel Mac and ARM qualification, physical networking, TPM and laptop battery protection. Apple Silicon remains later scope.
- Real hub/account/private-network enrollment, tracked with hardware checks in HT-069 of [the human-testing list](../../need-human-testing.md). Hosted enrollment also retains the HM-D16 dependency.

No physical disk, production account or remote host was modified. No ISO was publicly published. Source is kept on the authorized implementation branch.
