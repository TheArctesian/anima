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
2. Fullscreen Calamares opens directly. Use its Network step to scan for Wi-Fi and connect. Installation also works offline.
3. Choose the keyboard layout and local `admin` password. Explicitly select an internal installation disk of at least 32 GiB. Optional additional data disks become separate Btrfs volumes; USB/removable drives are excluded.
4. Review each disk's identity and erase action. Check the erase authorization, then select **Erase selected disks & install**. Changing selections clears authorization. Unselected disks remain untouched; this preview does not support dual boot or repair that preserves the selected disk's data.
5. After completion, shut down, remove the USB and start the machine. The installed server uses a branded console. Log in as `admin`, then use `anima-status` and `nmtui`. Root login is locked; administrative commands require `sudo` and the chosen password.

The footer settings icon opens display orientation, reduced motion and recovery controls. Ctrl+Alt+F2 opens a text recovery console. For CLI installation, stop Calamares before running `anima-install`; both installers share an exclusive installation lock. The CLI uses a typed disk-specific erase phrase.

System NetworkManager profiles are copied privately to persistent storage. Secrets held only in a user's keyring are not copied; such connections can require setup again after installation. The console keyboard map is retained. Firmware and `lspci`, `lsusb`, `iw` and `rfkill` diagnostics are available. A graphical menu cannot make an unsupported Wi-Fi adapter appear; unknown hardware still needs validation.

The preview uses fixed filesystem labels and refuses installation while another disk carries those labels. Do not connect two Anima installations to the same machine. Disk encryption is not implemented in this installer. The live ISO intentionally has a local root console and no SSH; keep the USB under your control.

## Connect monitoring

The installed daemon retains its existing private-network restrictions. First establish connectivity to your compatible private hub (for a Tailscale network, run `sudo tailscale up` and complete its sign-in). Create a device enrollment code in Anima. Run `sudo anima-enroll`, enter the hub origin, then paste the code at the hidden prompt. Never put the code in a shell command or test log.

The service starts only after enrollment exists. It provides the existing device monitoring and credential behavior; packaging it does not add a job/agent execution engine. Hosted enrollment is still subject to the existing HM-D16 integration gap. No production account was enrolled during the image tests.

Podman is available for local containers. Managed app publication, Anima-scheduled jobs/agents, owner grant UI, automatic updates, backup/restore, encryption and hub-driven installation are not implemented by this preview. The separate recovery prototype proves specific failure scenarios; that evidence is not a claim that the installed preview has a production update coordinator.

## Automated VM installation tests

Requires `qemu-img` as well as `qemu-system-x86_64` on PATH and the Arch OVMF paths used in `vm.py`:

```sh
python3 experiments/anima-os/gui_test.py \
  /tmp/anima-os-preview-iso/iso/anima-os-preview-x86_64.iso \
  "$HOME/.local/share/anima-os-prototype/gui-test-new"

python3 experiments/anima-os/iso_test.py \
  /tmp/anima-os-preview-iso/iso/anima-os-preview-x86_64.iso \
  "$HOME/.local/share/anima-os-prototype/iso-test-new"
```

Use a fresh disk-backed directory with at least 64 GiB free. The GUI test creates a 64 GiB OS disk, a 40 GiB data disk and a separate 32 GiB sentinel disk. It attaches no NIC, checks erase authorization and its reset after selection changes, installs, removes the ISO, logs in and checks persistence after reboot. The CLI test separately covers its typed confirmation flow. All disks are disposable regular files. No host disk is attached. Raw logs and results remain in that directory.

Run the Python regression checks separately:

```sh
python3 -m unittest discover -s experiments/anima-os -p 'test_*.py'
python3 -m unittest discover -s experiments/anima-os/installer
```

Physical hardware and real enrollment checks are tracked in HT-NEW-aospreview in `docs/need-human-testing.md`.

Brand sources and the hero render command are in [branding/README.md](branding/README.md).

## Fullscreen preview revision

The combined revision replaces the live desktop with fullscreen Calamares. The Network step shows actual NetworkManager status and offers Wi-Fi scans/connections plus advanced profile editing. The footer settings icon opens a styled dialog for display orientation (15-second confirmation/revert), reduced motion and the recovery terminal. Ctrl+Alt+F2 remains a text recovery path. Boot-time rotation uses a reported internal-panel mounting orientation; accelerometer-driven rotation is not enabled. Manual live-session rotation is not a persistent installed-server setting. Multi-monitor touch mapping needs hardware qualification.

Storage supports one explicitly selected internal OS disk plus explicitly selected additional internal data disks, each a separate Btrfs volume. USB/removable and busy drives are excluded. Review identifies each disk and erase action; changed selections invalidate authorization. All selected identities are rechecked before erasure, and data disks are checked again immediately before use. A hardware failure can still leave selected disks partially written. Unselected disks are never formatted. Additional volumes mount by UUID under `/persist/volumes/<UUID>`; a missing volume is reported and left unmounted without blocking server boot. No pooling/RAID or automatic workload placement is implied.

Account and keyboard setup remain real. Encryption, configurable hostname/timezone and hub pairing are not implemented by this preview; mockup examples are not shipped as functional settings.
