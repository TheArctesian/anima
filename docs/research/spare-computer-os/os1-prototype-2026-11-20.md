# Record: OS1 disposable UEFI prototype (2026-11-20)

This record covers the bounded F1/UEFI-F2 experiment authorized by R-276 in the [proposal](../../proposals/spare-computer-os.md), Section 13. It adds no requirement and does not approve later implementation. Implementation source is commit `29a875a2`; later evidence/documentation changes do not alter the image. Source and reproduction instructions are in [the prototype](../../../experiments/anima-os/README.md).

## Artifact and environment

- Host: `framework`, Linux x86-64, KVM; each guest has two vCPUs and 4 GiB RAM.
- Pinned NixOS 26.05 input: `3457b69ac4aba6d9ea9a743370af160fb502db29` (2026-11-18).
- Nix 2.35.3, QEMU 11.1.2, Arch OVMF 202608; no guest network interface or host filesystem share.
- Primary disk: 24 GiB virtual capacity, GPT, 1 GiB EFI plus shared Btrfs; separate root, Nix, variable-state and persistent-data subvolumes. A second disposable 1 GiB disk has an unrelated sentinel.
- Final image SHA-256: `f7b703f63f8026c914993f7f08ca1661420113d6779bfdb9789d3243f9d01201`.
- Local artifact directory: `/home/okita/.local/share/anima-os-prototype/2026-11-20/final/`. Image: `anima.qcow2`. Raw logs and disposable test disks remain local; compact result records are linked below. No image was published.
- Configuration A: `/nix/store/cl4sp3k323a68vmkd9vlb99m08mkgfh4-nixos-system-anima-prototype-26.05.20261118.3457b69`.
- Configuration B: `/nix/store/bh23iv8vqf9m7bf3mkbp5wllxznhjq21-nixos-system-anima-prototype-26.05.20261118.3457b69`.

The image contains a serial root autologin and synthetic identity. It is a test fixture, not an installation candidate. Build inputs and configuration identity are pinned; bit-for-bit image reproducibility is not claimed because filesystem creation includes fresh identifiers.

## Findings from implementation

1. The pinned standard disk-image helper restricts partitioned images to ext4. The experiment instead creates GPT/EFI and Btrfs inside a disposable builder VM. It never formats a host block device.
2. Root lockdown must be per mount (`remount,bind,ro`), preserving writable state mounts on the shared Btrfs filesystem. A whole-filesystem read-only remount failed. NixOS boot activation still needs its controlled writable phase; ordinary operation is read-only afterward.
3. Boot success must be acknowledged only for a counted entry. Blessing uncounted A fails. The minimal service shell also lacks `compgen`; a plain file glob now checks the EFI counter before invoking systemd's blessing operation. Candidate failure triggers a separate failure unit; a failed fallback reports recovery required without requesting another reboot.
4. `/tmp` on this host is tmpfs. Early concurrent VM copies and a fill test exhausted its quota, invalidating affected runs. Large artifacts moved to disk-backed storage. The harness now rejects tmpfs/ramfs and requires at least 32 GiB free. Early logs remain under the local `evidence/initial-tmpfs-runs/`; those incomplete runs are not acceptance evidence.
5. Persistence tests write fresh unpredictable sentinels after first boot. This prevents boot-time initialization from hiding lost data by recreating the same fixed value.

6. A candidate-only boot default kept selecting B after its counter was exhausted. The activation fixture now uses a default pattern that includes both entries; the fallback test must establish the actual transition to A. Entry counting alone was insufficient.
7. The first shutdown-hang probe exceeded 60 seconds. A confirmation uses the full 180-second readiness bound, longer than the configured 60-second late-shutdown watchdog, to avoid mistaking watchdog latency for lack of recovery.

## Scope limits

The test activates preinstalled A/B closures; it does not implement authenticated download, a remote configuration API, the Anima scheduler or production grants. The health fixture checks root/state/local prerequisites, not a production Anima daemon. The capacity smoke checks retained-data reads and cleanup after guest ENOSPC; no production admission policy is implemented. No ISO, physical installation, BIOS/ARM boot, TPM, encryption, account pairing, real app/job/agent runtime or backup restore is claimed.

Power-loss cases intentionally kill and restart QEMU. They test recovery after power returns, not autonomous power restoration. Tests of rejected entries or hangs must recover without harness reset to pass. A fallback health failure should remain locally recoverable instead of looping. Physical hardware support remains wholly untested.

## Results

| Check | Evidence / result |
| --- | --- |
| UEFI boot and read-only Btrfs root | Passed; first ready marker in 11.81 seconds in the recorded accelerated fixture. |
| Workload and simulated daemon write denial | Passed for root, Nix store and boot storage. |
| Persistent state | Five clean reboot cycles passed with freshly changed sentinels; this does not validate real network credentials or Anima device keys. |
| Declared package/service changes | Three A/B cycles passed; B's package/service appears, removal through A leaves the retained service output. |
| Failed candidate health | Three cycles passed: two failed B attempts, then A, with unchanged retained data. |
| Power cut after partial staging | Three cases passed; only the first 1 MiB of the staged initrd was copied before the cut. |
| Power cut after boot selection publication | Three cases passed; B boots when power returns. |
| Power cut before health confirmation | Three cases passed; an unblessed B entry resumes and completes verification after power returns. |
| Missing candidate kernel | Three cases passed without harness reset. |
| Guest storage exhaustion | Passed once: guest ENOSPC, intact retained data and read/cleanup response in 0.44 seconds. No admission control claim. |
| Both candidate/fallback health fail | Three cases passed: two B attempts, recovery-required on A, console available and boot identity unchanged during a further 25-second observation. |
| Initrd emergency target | Base policy failed to recover within 180 seconds; unrelated disk unchanged. This is a userspace initrd stall, not a claim to have injected a pre-initrd kernel deadlock. |
| Candidate initrd emergency-policy follow-up | Three cases passed without harness reset using `boot.panic_on_fail` on the candidate entry only; separate result, not a change to the base artifact. |
| Infinite service-stop hang | Failed to recover within 180 seconds; unrelated disk unchanged. |

The [core result record](os1-results/core-corrected.json) and [shutdown failure record](os1-results/shutdown-hang-confirmed.json) include hashes of the local raw serial logs. Other completed records are in the adjacent `os1-results/` directory. Early interrupted runs and the candidate-only-selector run are debugging evidence, not acceptance evidence.

F1 has a successful disposable-fixture proof, with real networking, real identities and production customization still outside that proof. F2 is **not passed**: the shutdown-hang case fails, and the base initrd emergency policy also fails. A candidate-only policy follow-up is separate from base-artifact qualification. F3/F4 remain unapproved and unimplemented. Do not advance to a stable or supported release from this result.

## Follow-up and repository checks

Before broader OS integration, finish the boot/shutdown deadline and watchdog design, define which custom service timeout settings are supported, and repeat the failed scenarios. A runtime watchdog alone must not be treated as coverage for every earlier boot stage or an indefinitely stopping service. These are next engineering actions, not new approved requirements.

All 21 repository gate categories have passing local evidence across the main run and targeted completions, not one uninterrupted green run. The lower-concurrency run passed Rust (6,468 tests, one skipped), doctests, pinning, all Go checks, dependency installation, lint and type checks. Bun tests had three failures solely from missing ShellCheck; after correcting the tool path, all 36 tests in the three affected files passed. Client build and all quality checks passed; baseline used `origin/phase-2` because no local `phase-2` ref exists. Nix configuration/derivation evaluation, image build and Python syntax passed. An earlier login timing failure under concurrent load passed in isolation and in the lower-concurrency suite; authentication code was not changed. GitHub gates remain separate landing requirements.

Local gate logs are retained under the artifact directory’s sibling `evidence/` directory. The bounded-run claim is released; OS1 remains incomplete beyond this experiment. No physical-hardware feature or production integration was built by this slice.

