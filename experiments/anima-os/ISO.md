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
5. After completion, shut down, remove the USB and start the machine. The installed server opens the Anima graphical console. Unlock it with the local admin password chosen during installation. Network, storage, display, diagnostics, lock and power controls are available there. Root login remains locked; Ctrl+Alt+F2 provides authenticated text recovery.

All installer steps and installed-console pages share resolution-aware Qt scaling, including dialogs and footer controls. At launch the primary active display determines a quarter-step scale that retains at least a 1280×800 logical canvas where the panel permits it; small displays remain at 1×. Form fields grow with the page up to 1100 logical pixels; welcome/completion artwork follows the available area. This is a resolution-based default, not a claim of measured physical DPI.

The footer settings icon opens display orientation, reduced motion and recovery controls. Ctrl+Alt+F2 opens a text recovery console. For CLI installation, stop Calamares before running `anima-install`; both installers share an exclusive installation lock. The CLI uses a typed disk-specific erase phrase.

System NetworkManager profiles are copied privately to persistent storage. Secrets held only in a user's keyring are not copied; such connections can require setup again after installation. The console keyboard map is retained. The installed supplicant uses a read-only configuration while NetworkManager retains profiles in its designated writable directory. Firmware and `lspci`, `lsusb`, `iw` and `rfkill` diagnostics are available. A graphical menu cannot make an unsupported Wi-Fi adapter appear; unknown hardware still needs validation.

The preview uses fixed filesystem labels and refuses installation while another disk carries those labels. Do not connect two Anima installations to the same machine. Disk encryption is not implemented in this installer. The live ISO intentionally has a local root console and no SSH; keep the USB under your control.

## Connect monitoring

Sign in with WorkOS from the Anima screen, complete the secure browser flow, then choose **Add this computer**. The console obtains the account’s hub access, joins the private link, identifies or creates this machine’s device record, and exchanges a fresh enrollment code for the daemon token internally. Neither the code nor account tokens appear in the UI, process arguments or operation logs. Existing daemon enrollment is not silently replaced. A machine-ID-derived name distinguishes spare computers. The link runs as a system service and survives local screen locking and reboots.

This flow depends on the account service and compatible hub routes being available. It reports failures rather than displaying a mock success. Sign-in tokens are memory-only; restart sign-in after a failed enrollment. No production account was enrolled during automated image testing. Packaging monitoring does not add the planned managed job/agent execution engine.

Locking conceals the local interface and closes its authenticated command session; it does not suspend the server or stop its monitoring/link services. Already-authorized operations may finish. Enrollment has a cross-process lock so a new local session cannot race the previous enrollment. Display orientation has a backend-enforced 15-second rollback and is session-local. Restart and shutdown require a visible confirmation.

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

The combined revision replaces the live desktop with fullscreen Calamares. The Network step shows actual NetworkManager status and offers Wi-Fi scans/connections plus advanced profile editing. The footer settings icon opens a styled dialog for display orientation (15-second confirmation/revert), reduced motion and the recovery terminal. Ctrl+Alt+F2 remains a text recovery path. Boot-time rotation uses a reported internal-panel mounting orientation; accelerometer-driven rotation is not enabled. A confirmed live-session rotation is copied to persistent storage during installation and applied before the installed console starts. Confirmed rotations in the installed console persist too; unconfirmed previews still revert. Multi-monitor touch mapping needs hardware qualification.

Storage supports one explicitly selected internal OS disk plus explicitly selected additional internal data disks, each a separate Btrfs volume. USB/removable and busy drives are excluded. Review identifies each disk and erase action; changed selections invalidate authorization. All selected identities are rechecked before erasure, and data disks are checked again immediately before use. A hardware failure can still leave selected disks partially written. Unselected disks are never formatted. Additional volumes mount by UUID under `/persist/volumes/<UUID>`; a missing volume is reported and left unmounted without blocking server boot. No pooling/RAID or automatic workload placement is implied.

Account and keyboard setup remain real. Encryption, configurable hostname/timezone are not implemented by this preview; mockup examples are not shipped as functional settings.
