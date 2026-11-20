# Graphical Anima.ing preview — 2026-11-20

This continues OS3 under R-NEW-aosgui. It is an experimental x86-64 UEFI image,
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
Hardware and real hub enrollment remain in HT-NEW-aospreview. Firmware and diagnostic
tools are included, but no unknown Wi-Fi chipset is declared supported by a GUI test.
The installed runtime is unchanged: existing enrollment/monitoring and local Podman,
not completed Anima app/job/agent orchestration.

Visual checks: [live desktop](gui-screenshots/desktop.png), [installer summary](gui-screenshots/installer.png), [boot splash](gui-screenshots/boot.png).
