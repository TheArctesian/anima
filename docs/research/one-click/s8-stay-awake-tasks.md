# S8 task breakdown: Stay awake

Date: 2026-11-15. Branch: `claude/s8-stay-awake-7b13a5`, cut from `origin/main` at `4c27cceb`.
Source: `docs/one-click-plan.md` Section 4.8 and ruling OR-8 (on branch `claude/one-click-plan`;
not on `main`). Requirements: AWK-001 to AWK-007 from the plan, and AWK-008 (hold the highest state that is possible; added by this work), in spec Section 3.13.

S8 adds no hub function. CMD-030 does not apply: there is no `anima` command and no MCP tool.

## 1. Platform facts (checked 2026-11-15)

The plan's platform notes were not checked. These are the results. "Source" means that the
fact was read in the vendor's source code and was not run. Nothing was run with more rights.

| # | Fact | How checked |
| --- | --- | --- |
| F-1 | macOS: `caffeinate -i` holds the power assertion `PreventUserIdleSystemSleep`. The display can sleep. It needs no administrator rights. `powerd` releases it when the process ends. `caffeinate -w <pid>` ends when that process ends. | Source (`PowerManagement`, `caffeinate.c`, `PMAssertions.c`); run on this Mac in the tests |
| F-2 | macOS: no power assertion that an ordinary application can take stops sleep when the lid closes. `PreventSystemSleep` (`caffeinate -s`) is not valid on battery. The lid property needs a private entitlement. | Source (`IOPMLib.h`, `PMAssertions.c`) |
| F-3 | macOS: `pmset -a disablesleep 1` sets `SleepDisabled`. It stops idle sleep and lid sleep, on mains and on battery. It needs root. It stays after a restart. `pmset -g` shows it. | Source (`pmset.m`, `IOPMrootDomain.cpp`); the read was run here |
| F-4 | macOS: a private call (`RootDomainUserClient`, selector 12) can stop lid sleep with no rights. It is not released when the process ends, and `powerd` writes the same bit and can clear it. | Source (XNU) |
| F-5 | macOS: Apple's `sudoers` file includes `/private/etc/sudoers.d`. | Source (Apple `sudo`); the folder exists here |
| F-6 | A Mac can already have `SleepDisabled 1` from a different tool. This Mac has. | Run here (`pmset -g`) |
| F-7 | Linux: a `systemd-logind` inhibitor is released when its holder ends. `systemd-inhibit` holds one for as long as its command runs. | Source (systemd) |
| F-8 | Linux: the default policy lets an active local session take `sleep` and `handle-lid-switch` inhibitors in block mode. | Source (`org.freedesktop.login1.policy`) |
| F-9 | Linux: a `sleep` inhibitor alone does not stop lid sleep (`LidSwitchIgnoreInhibited=yes`). A `handle-lid-switch` inhibitor is always obeyed by `logind`. | Source (`logind.conf` manual) |
| F-10 | Linux: an `idle` inhibitor keeps the screen on in KDE. `sleep` alone, in block mode, lets the screen sleep. | Source (PowerDevil) |
| F-11 | Linux, GNOME: `logind` handles the lid, so the `handle-lid-switch` inhibitor works. With systemd before 257, GNOME's own idle sleep ignores a `sleep` inhibitor. | Source (gnome-settings-daemon, systemd `NEWS`) |
| F-12 | Linux, KDE Plasma: PowerDevil handles the lid itself and ignores a `handle-lid-switch` inhibitor from a different program. | Source (PowerDevil) |

Answer to OI-4: yes. On macOS the lid-closed level needs administrator rights once (F-2, F-3).

## 2. Design decisions

| ID | Decision | Reason |
| --- | --- | --- |
| D-1 | "Keep awake" runs a child program: `/usr/bin/caffeinate -i -w <app pid>` on macOS, `systemd-inhibit --what=sleep --mode=block` with a command that waits for the app's process on Linux. | The OS releases the hold when the app ends, also after a crash (AWK-007). No `unsafe` code and no new dependency. |
| D-2 | macOS lid-closed level: `pmset -a disablesleep`. The one set-up step writes `/private/etc/sudoers.d/anima-stay-awake`, which lets this user (by user ID) run exactly `/usr/bin/pmset -a disablesleep 0` and `… 1` with no password. macOS asks for the administrator password (`osascript`, "with administrator privileges"); the shell never sees it (as INS-009). "Remove" deletes the file the same way. | F-2 and F-3. The private call (F-4) is not taken: it is not documented and `powerd` can clear it. A privileged helper service is larger and belongs to the service install of S4. |
| D-3 | The shell restores `SleepDisabled` in three ways: when the level goes down, when the app exits, and from a small watcher process that waits for the app's process to end. A marker file records that Anima set it; the next start restores it if the marker is still there (restart of the computer). | AWK-007 also after a crash. `SleepDisabled` stays after a restart (F-3). |
| D-4 | When `SleepDisabled` is already 1 at the start of the lid-closed level, Anima does not set it and does not restore it. | F-6. The setting belongs to the tool that made it. |
| D-5 | Linux lid-closed level: `systemd-inhibit --what=sleep:handle-lid-switch`. No set-up step. On KDE Plasma (`XDG_CURRENT_DESKTOP` has `KDE`) the level is not offered; the row says that the desktop's own power settings decide the lid. | F-8, F-9, F-12. |
| D-6 | Battery floor: on battery below the floor the control is held off ("paused") and the choice is kept. It starts again on mains power, not when the charge passes the floor. The floor is off by default; the choices are 10 to 50 percent. | A hub must start its checks again when power returns, with no action by the operator. Waiting for mains power stops on-off changes near the floor. |
| D-7 | When the lid-closed level is chosen and cannot be held (no set-up, or the OS refused), the computer is held at "Keep awake" and the row says why. | The lower level is still useful, and the state is never hidden. |
| D-8 | The choice and the floor are saved in the app's configuration folder (`stay-awake.json`) and applied at the start of the app. | A hub computer that restarts must not need the operator. |
| D-9 | The tray or menu bar item exists on every desktop install. It has the three levels and "Open Anima". | AWK-004. It is the item of the later in-app hub; the control is useful before that for a computer with the metrics agent. |
| D-10 | AWK-005 seam: the client's `hubIsThisComputer(origin)` is true when the hub address is a loopback address. The in-app hub answers on loopback, and a development hub on this computer also counts. | A real input today, and one function to change for S6. |
| D-11 | The power state is read with `pmset -g batt` (macOS) and `/sys/class/power_supply` (Linux), once a minute, only while a level and a floor are set. A state that cannot be read counts as mains power. | No new dependency. A read failure must not turn the control off. |
| D-12 | Settings gets a section "Stay awake" after "Hub", in the desktop application only. | SET-004; the control is about this computer. |

## 3. Tasks

| # | Task | Done when | Review |
| --- | --- | --- | --- |
| T1 | Spec: Section 3.13 with AWK-001 to AWK-007, SET-004 names the section. | The text is in `docs/design-spec.md`. | reviewer |
| T2 | Pure logic in `apps/desktop/src-tauri/src/stay_awake/`: the level from the choice, the power source and the lid state (`level.rs`); the power source readers (`power.rs`). | Unit tests for each rule of D-6, D-7 and D-11. | reviewer |
| T3 | OS holds (`hold.rs`), the lid set-up (`lid_set_up.rs`), the saved choice and the marker (`saved.rs`). | Tests with stand-in programs; one test takes and releases a real `caffeinate` assertion on macOS. | reviewer, reviewer-security |
| T4 | The control (`control.rs`), shell commands, the change event, the battery watch, restore at exit, the tray item (`stay_awake.rs`, `tray.rs`), capability and permissions. | The app builds; the control tests pass. The tray item itself is a live check. | reviewer, reviewer-security |
| T5 | Client: `$lib/desktop/stay-awake.ts`, `$lib/stay-awake/`, the Settings section, the capability, AWK-005 notice. | Component and unit tests pass. | reviewer |
| T6 | Merge `origin/main`, full gates, deferred live checks listed. | `bun run gates` passes. | — |

## 4. Not in S8, recorded for later work

- **S4 (hub service):** when the hub runs as a background service without the application
  window, the holds must belong to the service: start `caffeinate -w` and the watcher with the
  service's process ID. Today the holds end with the application (AWK-007).
- **S6 (first run):** show the AWK-005 text in the first-run flow when the operator chooses the
  in-app hub, with a button that sets "Keep awake". Use `hubIsThisComputer` and `sleepNotice`
  from `$lib/stay-awake/stay-awake.ts`.
- **Linux builders:** the tray item needs the AppIndicator library at run time
  (`libayatana-appindicator3`). Add it to the AppImage build notes with OD-18.
- GNOME with systemd before 257: add a `gnome-session` inhibitor if the live check shows idle
  sleep with "Keep awake" on (F-11).

## 5. Deferred live checks

Section 6 is filled at the end of the build.
