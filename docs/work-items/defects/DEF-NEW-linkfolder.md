# DEF-NEW-linkfolder: on Linux the link is not made a unit, so the daemon is not installed and the computer has no metrics

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | DEF-NEW-linkfolder |
| Kind | Defect |
| Owner | [hosted-mode.md](../../proposals/hosted-mode.md) HMD-058, HMD-053, HMD-054; tasks HM-T24 and HM-T19 |
| Serves | Row 4.17 of the work plan (hosted mode) |
| Version | 1.0 |
| Date | 2026-11-20 |
| Origin | The operator, on 2026-11-20, on Pop!_OS 24.04 with the AppImage 0.0.3 signed in to his hosted hub: "why is there no metrics on this one" (the row `pop-os` said "No metrics"). Not a GitHub issue (PLN-012). |

## Cause

Read in the log of the application and in the code of `main` at `3a277cfd`.

- The shell copied the link program to `~/.local/bin/anima-link` and ran `anima-link unit install`. The link refuses a program whose file or folder its group or others can write (`link/unit/owner_unix.go`, `CheckProgram`). His `~/.local/bin` is mode 775 (`okita:okita`, a private group): Ubuntu and its derivatives give a user a private group and the umask 002, so a folder that a person makes is often 775.
- The shell logged only "`anima-link unit install` failed", with no cause.
- The application then ran the link itself with no socket for the daemon. The daemon install of HM-T19 waited for the socket five times ("the link has no socket for the daemon yet") and stopped. No daemon ran, so the hub had no metrics of the computer.

The daemon copy is not affected: its check (`daemon/trust.rs`) accepts a folder whose group is the private group of the user.

## Correction

On the branch `fix/link-private-folder`:

- [x] The copy of the link program is `<application>/link/program/anima-link`, in the folder of the link that the application makes with mode 0700. The mode of `~/.local/bin` no longer matters. The operator asked for this and not for a `chmod` of his folder: "the isntall should embed it do it properly".
- [x] A unit of an earlier application that runs `~/.local/bin/anima-link` is found, moved to the new copy at the next start, and the earlier copy is deleted. A computer whose unit was never made gets the new copy at the hand-over, and the earlier copy is deleted then.
- [x] The log has the first line of the link's refusal when `unit install` fails.
- [x] Seen on 2026-11-20 on his Pop!_OS 24.04 with the desktop program of this branch in the 0.0.3 AppImage: "a unit of the user keeps the device link"; `anima-link.service` active; the link ran from `~/.local/share/anima/link/program/anima-link` with `-daemon-socket`; `~/.config/anima-daemon/link.sock` was made; `~/.local/bin/anima-link` was deleted.
- [ ] The daemon installs and the row of the computer shows metrics: [HT-NEW-linkfolder](../../need-human-testing.md).
