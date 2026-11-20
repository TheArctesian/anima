# Overnight ISO implementation reviews (2026-11-20)

Continuation authorized by R-NEW-aosrecovery and R-NEW-aosisopreview. Reviews below are direct code inspections by the implementing agent, not independent reviewers. Each stage retains its tests and unresolved limits.

## Recovery review

- Fixed: image builder duplicated kernel parameters and could omit an approved recovery setting. Entries now use the respective NixOS configuration's parameters.
- Fixed: the main runner recorded a changed unrelated disk but could exit successfully. It now fails the command; a regression test exercises this path.
- Fixed: zero fault iterations could produce an empty successful run. Zero and negative counts are rejected before output creation.
- Fixed: the runtime/late-shutdown watchdog did not bound a stuck service stop. A 60-second queued reboot-job deadline now uses `reboot-force` after reboot was already requested. This is not a workload-draining deadline. It can terminate remaining processes; database consistency is not proved by filesystem teardown.
- Candidate B now uses the tested NixOS initrd panic-on-failure policy. A retains its emergency policy to avoid a common-mode emergency reboot loop. Pre-initrd kernel deadlock and physical watchdog support remain untested.
- Evidence: revised image recovered all three injected initrd emergencies and all three infinite service-stop hangs without harness reset; the latter took approximately 75–78 seconds. Ordinary operation survived a 65-second idle check. Full core regression results are recorded separately when complete.

## Runtime packaging review

- The actual repository daemon is built with the pinned Rust toolchain and lockfile, then packaged with Nix library paths. Source revision and lock/binary hashes accompany it. This is a local preview build, not a signed release or a hermetic Rust build inside Nix.
- The installed service mirrors the existing system unit's unprivileged account, private state, filesystem, process and network restrictions. It does not start until enrollment exists. Enrollment codes use the existing hidden stdin prompt; none are baked into the ISO.
- The existing daemon is monitoring/enrollment software, not a job executor. Including Podman provides local container capability; it does not implement Anima's planned app/job/agent orchestration or revise its grants.
- No account was enrolled or contacted during packaging. Real account/private-network enrollment needs an operator check. The live ISO disables SSH and requires local access for installation.

## Installer review

- Before writes: require UEFI, root, an existing system closure, a fresh mount directory, a writable unused whole disk of at least 32 GiB, an admin password and an exact disk-specific erase phrase.
- Re-read disk identity and mount state after all prompts. Refuse mounted/swap children, active mapper children, device replacement, non-block paths and Anima labels on another disk. Installer operations name only the selected disk/partitions.
- Password hashing uses stdin; only the hash is written to root-only persistent storage. It is never an argument, Nix-store value or image credential. Installed root login is locked; the admin account is authenticated.
- Fixed during review: home and network-connection bind mounts need early-boot persistence. Both are designated persistent mounts rather than writable directories on the read-only root.
- Preview limitations: fixed filesystem labels permit only one connected matching installation; installation is destructive on the selected disk, not an OS repair path. Encryption, BIOS/ARM, graphical/hub-driven setup and preserving existing same-disk contents are absent. Post-install credentials/network setup are explicit local operations.
- Eight installer-policy tests pass, including password-file permissions and no overwrite. Actual ISO cancellation, offline installation, login, persistence and unrelated-disk checks remain mandatory before delivery.

- Fixed by ISO testing: `lsblk` needs explicit tree output when NAME is not selected; otherwise mounted child partitions are not nested and new partitions cannot be discovered. The reader now requests a tree and rejects any top-level partition as incomplete inventory. A real host read confirmed nested mounted disks are excluded; a regression test rejects flat output.
- Fixed by ISO testing: `Path.open` does not support the opener callback needed for private creation. Built-in `open` now creates the password hash file at mode 0600 exclusively; the new test verifies both the mode and refusal to overwrite.

## Final image review

- An actual ISO booted offline, cancelled without changing the target prefix, installed, booted with the ISO removed, authenticated the chosen admin password, verified read-only root and writable home, and preserved user data across reboot. The second disk's complete hash was unchanged. This first successful run preceded the additional container smoke and final packaging check.
- Tightened ESP permissions during installation to `umask=0077`, matching the installed mount policy; this removes bootctl's warning about a readable random seed in the live environment.
- The VM runner now drains serial output while awaiting shutdown, avoiding a test-harness deadlock if the PTY fills. It also fails promptly on installer-reported errors/tracebacks rather than waiting the whole installation timeout.
- Installer packaging includes only its two runtime source files; host bytecode caches and unit tests are excluded from the ISO closure.
- Final artifact qualification adds a real mounted-child exclusion check and local rootless app/job execution with no networking, read-only container root, dropped capabilities and resource limits. This does not claim a managed agent run or hub orchestration.

- Installation completion is now emitted only after successful unmount/cleanup. A cleanup failure cannot be mistaken for a completed installation by the UI or harness.
- QEMU disk paths with commas/newlines are rejected before launch, preventing its comma-separated drive-option parser from interpreting path text as options. A regression test covers the refusal.
- Filesystem types are explicit during installation. The expanded test also names ext4 explicitly for its mounted-partition probe instead of depending on module auto-detection.

Final-image evidence and its exact hash are recorded in the [delivery report](iso-preview-2026-11-20.md).
