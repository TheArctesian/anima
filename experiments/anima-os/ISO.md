# Experimental Anima OS ISO

This preview installs a real NixOS-based system on a selected disk. It is not the supported first release. It targets x86-64 UEFI only; Secure Boot, BIOS installation, ARM and physical hardware are unqualified.

## Build

From the repository root, with the pinned Rust toolchain, Cargo dependencies, Nix, KVM, QEMU and OVMF available:

```sh
experiments/anima-os/prepare-runtime.sh
NIX_REMOTE=daemon nix --extra-experimental-features 'nix-command flakes' build \
  path:./experiments/anima-os#iso --out-link /tmp/anima-os-preview-iso -L
```

The image is `/tmp/anima-os-preview-iso/iso/anima-os-preview-x86_64.iso`. Runtime preparation builds the repository daemon with `--locked --release`, strips a copy and records its source revision and hashes. `payload/` is generated and excluded from git. This is a local preview build, not a signed release or a fully hermetic Rust build inside Nix.

## Try and install

1. Write the ISO to an explicitly selected USB with a suitable flasher. Boot the spare machine in UEFI mode with Secure Boot disabled.
2. The live console starts without changing internal disks. Use `nmtui` for networking if desired. Installation itself works offline.
3. Run `anima-install`. Select a writable, unused disk of at least 32 GiB. The selected disk is erased; same-disk dual boot and repair preserving its data are not supported.
4. Set the local admin password, then type the exact disk-specific erase phrase. Empty or incorrect confirmation cancels. Disk identity and mount state are checked again after the prompts.
5. After completion, remove the USB and reboot. Log in as `admin`. Use `anima-status` and `nmtui`. Installed root login is locked; local administrative commands require `sudo` and the chosen password.

The preview uses fixed filesystem labels and refuses installation while another disk carries those labels. Do not connect two Anima installations to the same machine. Disk encryption is not implemented in this installer. The live ISO intentionally has a local root console and no SSH; keep the USB under your control.

## Connect monitoring

The installed daemon retains its existing private-network restrictions. First establish connectivity to your compatible private hub (for a Tailscale network, run `sudo tailscale up` and complete its sign-in). Create a device enrollment code in Anima. Run `sudo anima-enroll`, enter the hub origin, then paste the code at the hidden prompt. Never put the code in a shell command or test log.

The service starts only after enrollment exists. It provides the existing device monitoring and credential behavior; packaging it does not add a job/agent execution engine. Hosted enrollment is still subject to the existing HM-D16 integration gap. No production account was enrolled during the image tests.

Podman is available for local containers. Managed app publication, Anima-scheduled jobs/agents, owner grant UI, automatic updates, backup/restore, encryption and hub-driven installation are not implemented by this preview. The separate recovery prototype proves specific failure scenarios; that evidence is not a claim that the installed preview has a production update coordinator.

## Automated VM installation test

Requires `qemu-img` as well as `qemu-system-x86_64` on PATH and the Arch OVMF paths used in `vm.py`:

```sh
python3 experiments/anima-os/iso_test.py \
  /tmp/anima-os-preview-iso/iso/anima-os-preview-x86_64.iso \
  "$HOME/.local/share/anima-os-prototype/iso-test-new"
```

Use a fresh disk-backed directory with at least 64 GiB free. The test creates a 64 GiB target and a separate 32 GiB sentinel disk, attaches no NIC, tests cancellation, installs, removes the ISO, logs in and checks persistence after reboot. All disks are disposable regular files. No host disk is attached. Raw logs and results remain in that directory.

Run the Python regression checks separately:

```sh
python3 -m unittest discover -s experiments/anima-os -p 'test_*.py'
python3 -m unittest discover -s experiments/anima-os/installer
```

Physical hardware and real enrollment checks are tracked in HT-NEW-aospreview in `docs/need-human-testing.md`.
