# Anima OS F1 / UEFI F2 prototype

Authorized scope: OS1, Section 13 of [the proposal](../../docs/proposals/spare-computer-os.md).
This is a disposable x86-64 VM experiment, not an installer or release. It has a
root autologin on the serial console, synthetic identity and no SSH or Anima
credentials. Never install it on real hardware or expose its console to others.

## Build and run

On the recorded Arch host, install Nix, QEMU and OVMF. Start `nix-daemon.socket`
for the build session. KVM must be accessible to the builder and current user.
The harness uses `/usr/share/edk2/x64/OVMF_{CODE,VARS}.4m.fd`.

```sh
NIX_REMOTE=daemon nix --extra-experimental-features 'nix-command flakes' build \
  path:./experiments/anima-os --out-link /tmp/anima-os-image -L
python3 experiments/anima-os/vm.py \
  /tmp/anima-os-image/anima.qcow2 "$HOME/.local/share/anima-os-prototype/run-001"
```

Create the output parent first and use a new output directory every time. The harness rejects tmpfs/ramfs and requires at least 32 GiB free on disk; do not put VM disks in `/tmp` on a RAM-backed host. The harness refuses directory reuse and
copies the immutable build artifact before starting QEMU. It does not mount host
folders, forward network ports or attach physical disks. The second disk is a
new sparse 1 GiB file with a sentinel; its hash is compared after the run.
`serial.log` and `results.json` are in that run directory. The VM has no network
interface, so local health cannot accidentally depend on hub availability.

The flake pins the NixOS 26.05 input. `image.nix` creates GPT/EFI plus Btrfs inside
a disposable builder VM, copies both configuration closures and installs boot
entries. No host disk formatting occurs. The system image contains A and B to
exercise activation without a package server; this does not prove signed update
downloads, release provenance validation or a production activation API.

For an interactive serial console, add `--interactive` to the `vm.py` command
and use a different fresh output directory. Run it in a terminal; Ctrl-] exits
and stops that VM. The console is deliberately root-autologin for this fixture.

## State inventory

| Path | Normal mode / writer | Persistence and recovery |
| --- | --- | --- |
| `/` (`@root`, including `/etc`) | Writable during NixOS boot activation, remounted read-only before local health; privileged maintenance only | Persists across boots; not an ephemeral root. Configuration activation is not a whole-data snapshot restore. |
| `/nix` (`@nix`) | Root/Nix maintenance; NixOS mounts store read-only for ordinary operation | Both A/B closures retained in the image; no garbage collector experiment yet. |
| `/var` (`@var`) | Service-specific owners | Journals, NixOS service state and test evidence; backup/restore policy not implemented. |
| `/persist` (`@persist`) | Root-owned identity; designated workload dataset | Synthetic identity and workload sentinels survive fallback; never automatically restored backwards. |
| `/boot` (EFI) | Root only, umask 0077 | Boot entries and candidate attempt counts; publication is explicit. |
| `/run`, `/tmp` | Runtime/tmpfs permissions | Ephemeral; no backup. |

This proves a *normal-operation* read-only root only if the VM tests pass. It does
not prevent an authorized root administrator from remounting writable. The
fixture intentionally exposes no daemon-to-root configuration endpoint.

## Failure experiments

Use `faults.py IMAGE OUTPUT --scenario NAME` for `staging-cut`,
`publication-cut`, `rejected-entry`, `pre-health-cut`, `both-health-fail`,
`initrd-hang`, `shutdown-hang` or `capacity`. The default is
three iterations; it stops an identical scenario after its first failure so it
can be diagnosed. Capacity may use `--iterations 1` for the initial smoke test.
Each case gets a fresh disk copy, unique sentinel and its own result/log.

Staging/publication cuts deliberately kill QEMU, then restart it; these are
power-return tests, not evidence of autonomous reset. Missing-entry and initrd
hang cases never reset the guest to manufacture recovery. The capacity test
requires the guest to report ENOSPC before checking read/cleanup responsiveness;
it does not implement production admission control. Any host storage exhaustion
invalidates the affected run.

## Recovery experiment

Known-good A is the initial default. Tests rename B's entry to `anima-b+2.conf`
and select `anima-*.conf`, keeping A eligible after B's attempts expire. The candidate has two attempts. Healthy local
verification invokes systemd's boot blessing; a deliberately failed B health
check triggers the failure unit to reboot without blessing. A failing fallback
prints `ANIMA_RECOVERY_REQUIRED` and leaves the console available, without
requesting another reboot. systemd-boot should exhaust B and select
A. Serial evidence must establish that behavior. The test harness never resets
a guest to manufacture a successful automatic fallback.

The emulated i6300esb watchdog and systemd runtime watchdog are included, but
that alone does not prove early-kernel-hang or shutdown-hang recovery. Report
those separately. Likewise, preserving a sentinel does not establish a database
backup, a safe schema migration or full Anima workload support.

## Cleanup

Stop only the QEMU process associated with the run. Delete only the named run
directory after retaining its evidence. Removing the `/tmp/anima-os-image`
symlink releases that result reference; Nix store garbage collection is a
separate owner operation. Do not run global cleanup or delete unrelated images.

The `initrd-emergency-policy` follow-up adds `boot.panic_on_fail` only to the
candidate entry on its disposable copy, then deliberately selects an initrd
emergency target. NixOS has a built-in panic-on-fail unit; `panic=10` restarts the
guest and the counter can select A. This tests a proposed candidate policy, not
a change to the base artifact or proof of recovery before the initrd starts.

## Primary references

- [NixOS stable release](https://nixos.org/download/)
- [Pinned disk-image helper](https://github.com/NixOS/nixpkgs/blob/3457b69ac4aba6d9ea9a743370af160fb502db29/nixos/lib/make-disk-image.nix)
- [systemd boot blessing](https://www.freedesktop.org/software/systemd/man/latest/systemd-bless-boot.service.html)
