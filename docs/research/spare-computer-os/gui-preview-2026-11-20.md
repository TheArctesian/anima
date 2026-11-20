# Graphical Anima.ing preview — 2026-11-20

This continues OS3 under R-273. It is an experimental x86-64 UEFI image,
not the supported hardware release. The operator requested Calamares, graphical
network setup, Anima colours, white text and the anima-site hero background.

## Implementation and direct review

These reviews were performed by the implementing agent, not independent reviewers.

1. **Installer boundary (Composable, Predictable).** Calamares supplies its standard
   welcome, keyboard, owner, disk, summary and confirmation screens. A small adapter
   calls the existing offline installer. Calamares partition jobs are not executed.
   The selected disk is checked against the startup inventory immediately before
   formatting, including size, identity, mounts, active mappings and conflicting
   Anima labels. Both entry points hold the same exclusive lock. Only dedicated-disk
   installation is exposed; the preview does not imply dual boot or encryption.
2. **Credentials and state (Predictable).** Passwords are hashed through standard
   input and written with mode 0600. Neither passwords nor hashes are put in command
   arguments or logs. Root-owned regular NetworkManager keyfiles are copied with
   mode 0600; symlinks are skipped. Per-user keyring secrets are not copied. The
   selected keyboard is converted to a console map and retained under `/persist`.
3. **Presentation (Idiomatic).** Stock Xfce and NetworkManager provide the live
   desktop and network controls. Calamares uses Anima's Nord palette and white text,
   including a patch for upstream's hard-coded black partition labels. Boot branding
   and the installed console use Anima.ing. The wallpaper is a deterministic render
   of the existing anima-site hero. No desktop is added to the installed server.
4. **Integration corrections (Predictable).** VM trials found and corrected live
   home-directory ownership ordering, required Calamares slideshow configuration,
   KPMcore D-Bus registration, and a restrictive inherited umask that prevented
   `nixos-install` from accepting the target root. The installer now restores its
   caller's umask after creating normal system directories, while secrets retain
   explicit private modes. A subsequent test assertion was corrected to check the
   result of the one-shot keyboard service, rather than require it to remain active. The GUI test also waits for the shell prompt after sudo authentication, preventing the terminal password reader from flushing a queued test command.

## Evidence

The [result record](gui-preview-results.json) records the exact ISO hash: `6b49842f605147a979927664c15cc9f98473f2d89c07339bd06645ff0e34c824` (3,097,722,880 bytes). All three graphical checks, all eight CLI checks and twelve Python regression checks passed. Nix flake evaluation also passed. The repository gate run passed all 21 categories, including 6,468 Rust tests (one skipped).
Raw VM disks, serial logs and screenshots remain outside git under
`~/.local/share/anima-os-prototype/2026-11-20/`. Failed development trials are retained.
The graphical test operates the actual installer in a VM with no NIC and checks
cancellation, installation, owner login, read-only root, copied network configuration,
keyboard setup and persistence. The separate CLI test covers the fallback and local
rootless app/job fixtures. Both compare the complete unselected 32 GiB disk hash.

The installer regression tests cover disk refusal and private network-profile copying.
Hardware and real hub enrollment remain in HT-069. Firmware and diagnostic
tools are included, but no unknown Wi-Fi chipset is declared supported by a GUI test.
The installed runtime is unchanged: existing enrollment/monitoring and local Podman,
not completed Anima app/job/agent orchestration.

Visual checks: [live desktop](gui-screenshots/desktop.png), [installer summary](gui-screenshots/installer.png), [boot splash](gui-screenshots/boot.png).

## Wi-Fi backend correction after hardware trial

The operator's RTL8821CE is detected with `rtw88_8821ce`; NetworkManager reports no rfkill block or missing firmware, but cannot activate `fi.w1.wpa_supplicant1`. The live ISO forced wireless off, removing the backend required by NetworkManager. Remove that override.

Direct review (Predictable): keep NetworkManager's standard D-Bus backend configuration. Add a live-ISO regression check that activates the backend and reads its Interfaces property. The check fails on the delivered GUI ISO and passes after booting the rebuilt ISO, SHA256 `e75864eeaceef1b9612708135735dca21ac198d1e8b7c5c4190dd95b962139a4`. Documents and Python syntax checks pass. The new image has not repeated the full installation suite; installation logic is unchanged. Physical Wi-Fi scanning and connection remain open under HT-069.

## Fullscreen revision under R-272

This revision supersedes the live-desktop presentation above. Its reference is the operator-approved `anima-installer-wordmark-preview.html` in the local `anima-os-designs/fullscreen-calamares` directory. Fullscreen Calamares uses the supplied vector wordmark, IBM Plex Sans, Graphite surfaces, white text and frost controls. The Anima-site door is behind the welcome/completion content. Openbox supplies only the underlying window session; there is no desktop panel. Account and keyboard forms retain their real validation. Mockup hardware/progress values are not shipped.

Direct review (Idiomatic, Predictable): remove the About control and stacked sidebar utilities, put display/recovery and reduced motion in a frameless styled dialog behind a Carbon settings icon, and use the same Carbon arrows as the client for footer navigation. Initialize navigation correctly before the first step emits updates. Erase approval is the explicit review checkbox plus the labelled installation action; there is no second stock confirmation popup. Wi-Fi passwords use a styled dialog and a private process pipe, not command-line arguments. NetworkManager scan results retain both adapter identity and BSSID through activation.

Direct review (Predictable): selected additional data drives receive separate Btrfs filesystems, not an implicit pool. Every identity is validated before the first destructive operation, and additional drives are revalidated immediately before use. Authorization must exactly match the reviewed selection. USB/removable, busy and unselected disks are not installation targets. The installed system mounts recorded data-volume UUIDs; absent volumes are reported without blocking boot. Installer hardware inventory is a startup snapshot, not an automatic hotplug selection mechanism.

Display evidence: the styled orientation dialog reverted a virtual display to its previous orientation after its countdown. Automatic orientation uses the internal panel's reported mounting orientation, not an accelerometer. Physical touchscreen transforms and the operator's panel remain unqualified.

Networking evidence: installer and appliance use the same redistributable firmware set; no evidence supports adding unrelated firmware to fix this Realtek failure. The standard NetworkManager supplicant wiring is restored. A virtual-radio development check completed WPA authentication and a connection with a fixture static address. Its same-host DHCP fixture did not establish DHCP, so this does not qualify DHCP, internet access or the physical RTL8821CE. Physical scan/authentication/address acquisition and profile persistence remain in HT-069.

Repository validation: 6,468 Rust tests passed (one skipped). The initial broad client test run hit 16 timeouts while other build work ran; the full client suite then passed all 4,873 tests with four workers. Client production build and limits, names, imports, documents and baseline checks passed. Installer regression tests (13), display parser test (1) and VM harness tests (3) passed. Final combined-image installation evidence is recorded separately when complete; these checks do not establish visual approval or physical hardware support.

Final combined-image evidence: [fullscreen results](fullscreen-preview-results.json), SHA256 `637c0d4f2fd27a5a8fa51cd91e1cc98eadbc6d8e54257baf0c35af36e636fe2f`. All three graphical cases and all nine CLI cases passed on this exact image, including two selected disks, authorization reset after selection changes, boot without the ISO, private network-profile persistence, a separate data volume across reboot and the full unselected-disk hash. Early GUI-driver trials failed on OCR/button targeting; they were retained and corrected before the passing run. The actual final welcome and storage screens were inspected; Carbon footer icons render and the native erase confirmation is absent.
