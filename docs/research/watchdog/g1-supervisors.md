# Record: G1, what the supervisors show (watchdog prototype)

Version: 3. Date: 2026-11-16.
Status: Partly measured. launchd, Docker and systemd (the user manager) are measured. The right
of WDG-040 is not measured: it needs root on the Linux machine.
Version 2 adds the cases that an independent review found missing, and runs both scripts again.
The reviewer accepted version 2. Version 3 adds the systemd results from the operator's Linux
machine (operator, 2026-11-16: "you can run it on my linux machine with the anima cli").

This document is a record (work plan, Section 9; PLN-010). It adds no requirement. Its Section 7
lists inputs for the owner of the requirements,
[watchdog.md](../../proposals/watchdog.md).

## 1. Purpose

Task G1 of [watchdog.md](../../proposals/watchdog.md) asks for measured answers to three
questions:

1. Which signs tell a stop by a person from a failure, for each supervisor (WDG-025)?
2. Which states mean that the supervisor starts the program again, so that the watchdog must
   wait (WDG-023)?
3. How does the second unit of a Linux system installation get the right to restart (WDG-040)?

## 2. Method

- The watched program is `g1prog` ([g1/prog/main.go](g1/prog/main.go)). It is not part of Anima.
  The mode `serve` runs until SIGTERM and then exits 0. The mode `plain` catches no signal. The
  mode `crash` exits with a given code after a given time. The mode `health` is a health probe.
- Each supervisor has one script. Each script starts the program in each case, reads the state of
  the supervisor at intervals, and removes what it made at the end, also when the script stops.
- Each reading starts with the time since the step of the case, from the clock. launchd is read at
  intervals of about 1 s, and Docker and systemd at intervals of about 0.5 s. A time in this
  record is thus correct to about one interval.

| Supervisor | Script | Machine | Result |
| --- | --- | --- | --- |
| launchd | [g1/launchd.sh](g1/launchd.sh) | macOS 27.0.1 (26A434), Apple silicon, a job of the user domain `gui/501` | [launchd log](g1/results/launchd-macos-27.0.1.log) |
| Docker | [g1/docker.sh](g1/docker.sh) | Docker Engine 29.9.3 in Docker Desktop, linux/arm64, on the same Mac. The image is built from `scratch`; nothing is downloaded. | [Docker log](g1/results/docker-29.9.3-desktop-arm64.log) |
| systemd | [g1/systemd.sh](g1/systemd.sh) | Pop!_OS 24.04, x86_64, systemd 255 (255.4-1ubuntu8.15pop0), the user manager of user 1000. The files went up with `anima files put`, the script ran through `anima ssh`, and the log came back with `anima files get`. | [systemd log](g1/results/systemd-255-pop-os-24.04-user.log) |

The scripts read the state as a watchdog can read it: `launchctl print`, `docker inspect` and
`docker events`, `systemctl show` and the journal. None reads the process table (WDG-006).

## 3. Results: launchd

Cases L1 to L15 of the launchd log. "A person" is a command that a person types. "From a shell"
is a signal that the script sends with `kill`; a person and a crash can both cause it.

| Case | KeepAlive | What happened | What `launchctl print` shows |
| --- | --- | --- | --- |
| L1 | true | `serve` gets SIGKILL from a shell. | `state = spawn scheduled` until about 8 s, then `running`. `runs` goes from 1 to 2. `last terminating signal = Killed: 9`. |
| L2 | true | A person: `launchctl kill SIGTERM`. `serve` exits 0. | `spawn scheduled` until about 6 s, then `running`. `runs` goes from 2 to 3. `last exit code = 0`. |
| L12 | true | A person: `launchctl stop`. | The same as L2: `spawn scheduled` until about 8 s, then `running`, `last exit code = 0`. |
| L13 | true | A person: `kill -TERM` from a shell. | The same as L2: `spawn scheduled` until about 6 s, then `running`, `last exit code = 0`. |
| L3 | true | A person: `launchctl disable`, then `launchctl bootout`. | After `disable`: no change, the job runs; `print-disabled` shows `=> disabled`. After `bootout`: `Could not find service`. The `disabled` mark stays after `bootout`. |
| L4 | `SuccessfulExit = false` | `crash` exits 1 after 2 s, each time. | `spawn scheduled`, then `running` for about 2 s. The starts are at intervals of about 10 s. The attempts did not stop in the 28 s of the case. |
| L5 | `SuccessfulExit = false` | A person: `launchctl kill SIGTERM`. `serve` exits 0. | `not running`, `last exit code = 0`. launchd does not start it again. |
| L6 | none | `crash` exits 3. | `not running`, `last exit code = 3`. |
| L7 | none | `serve` gets SIGKILL from a shell. | `not running`, `last terminating signal = Killed: 9`. |
| L8 | none | A person: `launchctl kill SIGTERM`. `serve` exits 0. | `not running`, `last exit code = 0`. |
| L14 | none | A person: `launchctl kill SIGTERM`. `plain` does not catch it. | `not running`, `last terminating signal = Terminated: 15`. |
| L9 | none | Not running: `launchctl kickstart -k`. | The command returns after about 15 s. Then `running`. `runs` goes from 1 to 3 and `last terminating signal = Terminated: 15`. |
| L10 | none | Not running: `launchctl kickstart` (no `-k`). | The command returns after about 7 s. Then `running`. `runs` goes from 1 to 2. `last exit code` does not change. |
| L11 | none | `serve` runs: `launchctl kickstart -k`. | The command returns after about 7 s. A new process runs; `runs` goes from 2 to 3. `last exit code = 0`, because `serve` exits 0 on SIGTERM. |
| L15 | none | `plain` runs: `launchctl kickstart -k`. | The command returns after about 8 s. A new process runs; `runs` goes from 1 to 2. `last terminating signal = Terminated: 15`. |

Findings:

- **F-L1.** While launchd waits to start a job again, `state` is `spawn scheduled` (L1, L2, L4,
  L12, L13). This is the state to wait for (WDG-023). An earlier run, which is not in the log,
  also showed `xpcproxy` for one reading during a start; this is not logged evidence.
- **F-L2.** launchd starts a job again about 10 s after its last start (L1, L2, L4). In L4 the
  attempts did not stop in 28 s. This record does not show if launchd stops its attempts later.
- **F-L3.** launchd does not show who stopped a job. After `launchctl kill SIGTERM`,
  `launchctl stop` or `kill -TERM`, a program that exits 0 on SIGTERM shows `last exit code = 0`
  (L2, L5, L8, L12, L13). A program that does not catch SIGTERM shows `Terminated: 15` (L14).
  A program that ends by itself with 0 shows the same as the first.
- **F-L4.** With KeepAlive true, a person cannot stop a job with `launchctl kill`,
  `launchctl stop` or `kill`: launchd starts it again (L2, L12, L13). In the cases measured, the
  stops that stay are a job that is not loaded (`bootout`) and a job that is disabled (L3).
- **F-L5.** `launchctl kickstart` returned only after 7 to 15 s (L9 to L11, L15). The waits are
  about the time left until 10 s after the last start (F-L2), but the log does not show the
  cause. The 15 s of L9, which added two runs, is not explained.
- **F-L6.** `kickstart` without `-k` starts a job that is not running and adds one run (L10).
  `kickstart -k` on a job that is not running adds two runs and reports `Terminated: 15` (L9).
  `kickstart -k` on a running job reports the exit of the old process: `last exit code = 0` for
  `serve` (L11), `Terminated: 15` for `plain` (L15). A restart by the watchdog with `-k` can thus
  look the same as a person's `launchctl kill SIGTERM` (L15 and L14).

## 4. Results: Docker

Cases D1 to D13 of the Docker log. The events are from `docker events`.

| Case | Policy | What happened | `docker inspect` | Events |
| --- | --- | --- | --- | --- |
| D1 | `no` | `crash` exits 3 after 2 s. | `exited`, `exit=3` | `die exitCode=3` |
| D2 | `on-failure` | `crash` exits 1 after 1 s, each time. | `restarting` (`Restarting = true`) between the runs. The waits grow; the last two were about 3 s and 6.5 s. `ExitCode` is 0 while the program runs. `RestartCount` is 8 at about 22 s. | `die exitCode=1` and `start` for each run; no `kill` |
| D11 | `always` | `crash` exits 1 after 1 s, each time. | The same as D2: `restarting` between the runs, `RestartCount` goes from 0 to 4 in 5 s. | `die exitCode=1` and `start` for each run; no `kill` |
| D3 | `on-failure:2` | `crash` exits 1 after 1 s, each time. | After 2 restarts: `exited`, `exit=1`, `RestartCount = 2`, equal to `MaximumRetryCount`. | `die`, `start`, `die`, `start`, `die`. No event says that Docker stopped its attempts. |
| D4 | `always` | A person: `docker stop`. | `exited`, `exit=0`. Not started again. | `kill signal=15`, `stop`, `die exitCode=0` |
| D12 | `unless-stopped` | A person: `docker stop`. | `exited`, `exit=0`. Not started again. | `kill signal=15`, `stop`, `die exitCode=0` |
| D5 | `always` | A person: `docker kill` (SIGKILL). | `exited`, `exit=137`. Not started again. | `kill signal=9`, `die exitCode=137` |
| D6 | `always` | A person: `docker kill --signal TERM`. `serve` exits 0. | `exited`, `exit=0`. Not started again. | `kill signal=15`, `die exitCode=0` |
| D13 | `always` | A person: `docker kill --signal HUP`. `plain` does not catch it. | `exited`, `exit=129`. Not started again. | `kill signal=1`, `die exitCode=129` |
| D7 | `always` | The probe (interval 2 s, 2 retries) fails from 6 s. | `running`. `health` goes `starting`, `healthy`, then `unhealthy` at about 8 s. Not restarted. | `health_status: healthy`, `health_status: unhealthy` |
| D8 | `always` | A person: `docker pause`. | `paused` | `pause` |
| D9 | `no` | `docker restart` (the restart of WDG-021). | `running`. `RestartCount` stays 0. | `kill signal=15`, `stop`, `die`, `start`, `restart` |
| D10 | `no` | After `docker stop`: `docker start`. | `running` | — |

Findings:

- **F-D1.** While Docker waits to start a container again, the status is `restarting` (D2, D11).
  This is the state to wait for (WDG-023). The waits grow with each attempt. Docker documents
  that the delay doubles from 100 ms; the readings agree with this.
- **F-D2.** `ExitCode` is 0 while a container runs, also after failures (D2, D11). Only
  `RestartCount` keeps the count of the restarts by Docker.
- **F-D3.** When Docker stops its attempts under `on-failure:N`, no event and no status says so.
  The sign is `exited` with `RestartCount` equal to `MaximumRetryCount` (D3). This is the case of
  WDG-024.
- **F-D4.** In each case measured, a stop through the Docker API stopped the restart policy, also
  `always` and `unless-stopped`: `docker stop` (D4, D12) and `docker kill` with SIGKILL, SIGTERM
  and SIGHUP (D5, D6, D13).
- **F-D5.** `docker inspect` alone cannot tell a stop by a person from a failure. `exited` with
  `exit=0` is the same after `docker stop` (D4) and after `docker kill --signal TERM` (D6), and a
  program that ends by itself with 0 gives it too. `exit=137` after `docker kill` (D5) is the code
  of any SIGKILL.
- **F-D6.** The events tell a stop through the API. Each stop through the API wrote a `kill`
  event before the `die` (D4, D5, D6, D9, D12, D13). Each failure wrote a `die` with no `kill`
  (D1, D2, D3, D11). A `stop` event came only from `docker stop` and `docker restart` (D4, D9,
  D12). Not measured: a SIGKILL from a process outside the Engine, which is expected to write a
  `die` only.
- **F-D7.** One sign comes from `inspect` alone. With the policy `always`, a crash gives
  `restarting` (D11), and a stop through the API gives `exited` (D4, D5, D6, D13). Thus `exited`
  under `always` means a stop through the API. `unless-stopped` gave `exited` after a stop (D12);
  its result after a crash is expected to be the same as `always`, but it is not measured. With
  the policies `no` and `on-failure`, this sign does not apply.
- **F-D8.** Docker does not restart an `unhealthy` container (D7).
- **F-D9.** A paused container is `paused` (D8). This is a stop by a person, and Docker shows it
  in the status.
- **F-D10.** `docker restart` does not increase `RestartCount` (D9).

## 5. Results: systemd

Cases S1 to S7 of the systemd log. Each unit has `RestartSec=2`, `StartLimitIntervalSec=30` and
`StartLimitBurst=3`. The journal lines are from `journalctl --user-unit`. `ExecMainCode` 1 is an
exit and 2 is a signal.

| Case | Restart | What happened | `systemctl show` | Journal |
| --- | --- | --- | --- | --- |
| S1 | `no` | `crash` exits 3 after 2 s. | `failed`/`failed`, `Result=exit-code`, `ExecMainCode=1`, `ExecMainStatus=3` | not read |
| S2 | `on-failure` | `crash` exits 1 after 1 s, each time. | Between the runs: `activating`/`auto-restart` for about 2 s (`RestartSec`). `NRestarts` goes 0, 1, 2, 3. Then `failed`/`failed`, `Result=exit-code`, and no more starts. | not read |
| S3 | `always` | A person: `systemctl stop`. | `inactive`/`dead`, `Result=success`, `NRestarts=0`. Not started again. | A job of type `stop`: "Stopping …" (`MESSAGE_ID=de5b426a…`), then "Stopped …" with `JOB_RESULT=done` (`MESSAGE_ID=9d1aaa27…`). |
| S4 | `always` | A person: `systemctl kill --signal=TERM`. `serve` exits 0. | `activating`/`auto-restart` for about 2 s, then `active`/`running`, `NRestarts=1`. | "Sent signal SIGTERM to main process … on client request." Then "Scheduled restart job, restart counter is at 1." (`MESSAGE_ID=5eb03494…`). |
| S5 | `always` | `serve` gets SIGKILL from a shell. | `Result=signal`, `ExecMainCode=2`, `ExecMainStatus=9`, `activating`/`auto-restart` for about 2 s, then `running`, `NRestarts=1`. | not read |
| S6 | `always` | A person: `systemctl kill --signal=TERM`. `plain` does not catch it. | `Result=success`, `ExecMainCode=2`, `ExecMainStatus=15`, `auto-restart`, then `running`, `NRestarts=1`. | "Sent signal SIGTERM … on client request.", then "Scheduled restart job". |
| S7 | `no` | `systemctl restart` (the restart of WDG-021). | `active`/`running`. `NRestarts` stays 0. | "Stopping …" (`JOB_TYPE=stop`), "Stopped …" with `JOB_TYPE=restart`, "Started …". |

Findings:

- **F-S1.** While systemd waits to start a unit again, the state is `ActiveState=activating` with
  `SubState=auto-restart` (S2, S4, S5, S6). This is the state to wait for (WDG-023), as
  WDG-023 says.
- **F-S2.** When the start limit stops the attempts, the unit is `failed`/`failed` with
  `Result=exit-code` and `NRestarts` at its last value (S2). `Result` does not say
  `start-limit-hit` in this case. A `failed` unit with a restart rule other than `no` is thus the
  sign that systemd stopped its attempts (WDG-024). The journal of S2 was not read.
- **F-S3.** systemd shows a stop by a person in two ways. `systemctl stop` gives
  `inactive`/`dead` with `Result=success`, and the journal has a job of type `stop` (S3).
  `systemctl kill` writes "on client request" to the journal (S4, S6). A SIGKILL from a shell
  gives `Result=signal` (S5); its journal was not read.
- **F-S4.** `systemctl kill` does not stop the restart rule: with `Restart=always`, systemd starts
  the unit again (S4, S6). Only `systemctl stop` keeps it stopped (S3).
- **F-S5.** systemd counts a SIGTERM exit as a success: `Result=success` with
  `ExecMainStatus=15` (S6).
- **F-S6.** `systemctl restart` does not increase `NRestarts` (S7), as `docker restart` does not
  increase `RestartCount` (F-D10).
- **F-S7.** The first run of the script failed: it wrote the unit files with a double suffix
  (`.service.service`), and systemd found no unit. The script is corrected; the log is of the
  second run. Both runs removed their units.

## 6. Not measured

| Question | Why | What is needed |
| --- | --- | --- |
| systemd system units (not the user manager) | The `anima ssh` session is user 1000. | A run as root, if the watchdog is to watch system units. The rules of F-S1 to F-S6 are expected to be the same. |
| The right of WDG-040 (OD-WDG-2) | It needs root on a Linux machine and a polkit rule. The session has no root, and this record does not use a password. | The operator does the manual steps at the end of [g1/systemd.sh](g1/systemd.sh). The group `docker` is not in them. |
| Docker on Linux, not in Docker Desktop | Docker Desktop runs the Engine in a virtual machine. | Run [g1/docker.sh](g1/docker.sh) on the Linux machine. The results are expected to be the same, because the restart policy and the events are in the Engine. |
| Docker after a restart of the daemon or the machine | To restart Docker Desktop stops the operator's other containers. | A run on the Linux machine. The Docker documentation says that `always` containers start again after a restart of the daemon, also after `docker stop`, and that `unless-stopped` containers stay stopped. |
| How long the Engine keeps events for `docker events --since` | Not in the scripts. | A run that reads the events after a restart of the agent. |
| A SIGKILL to a container's process from outside the Engine | Not in the scripts. | A run on the Linux machine, where the process is visible to the host. |
| If launchd stops its attempts after more than 28 s | Not in the scripts. | A longer case L4. |
| A launchd system job (LaunchDaemon) | It needs root. OD-WDG-10 recommends the jobs of the user only. | Only if OD-WDG-10 is decided the other way. |

## 7. Inputs for the owner (watchdog.md)

These are not requirements. They are inputs for the operator's decisions and for the next version
of [watchdog.md](../../proposals/watchdog.md).

1. **WDG-023.** The states to wait for are `spawn scheduled` for launchd (F-L1), `restarting`
   for Docker (F-D1), and `activating`/`auto-restart` for systemd (F-S1).
2. **WDG-025, launchd.** launchd does not show who stopped a job (F-L3). A SIGKILL from a shell
   can be a person or a failure (L1, L7). The findings mean that, for launchd, only a nonzero exit
   code (L4, L6) is a failure that a watchdog can tell. A signal such as SIGSEGV, SIGBUS, SIGILL
   or SIGABRT is expected to be one too; no case produced these signals. An exit 0, SIGTERM and
   SIGKILL fall under "cannot tell". With KeepAlive true, launchd restarts the job in each of
   these cases (F-L4). A person keeps a job stopped with `bootout` or `disable` (F-L4).
3. **WDG-025, Docker.** The events tell a stop through the API (F-D6). A watchdog needs to read the
   events while it runs to use this sign. After a restart of the agent, the sign of F-D7 is
   available for `always`, and is expected for `unless-stopped`; `docker events --since` is not
   measured.
4. **WDG-025, systemd.** systemd tells a person: the journal has the job of type `stop`, or "on
   client request" for `systemctl kill` (F-S3). Unlike Docker, the journal stays, so a watchdog
   can read it after a restart of the agent. Only `systemctl stop` keeps a unit with a restart
   rule stopped (F-S4).
5. **WDG-021, launchd.** `kickstart` without `-k` starts a job that is not running (F-L6).
   `kickstart -k` restarts a running job, but its result can look like a person's stop (F-L6).
   `kickstart` returned only after 7 to 15 s in the cases measured (F-L5). This wait is not a
   failure of the command.
6. **WDG-024.** For Docker, "the supervisor stopped its own attempts" is `exited` with
   `RestartCount` equal to `MaximumRetryCount` under `on-failure:N` (F-D3). For systemd, it is a
   `failed` unit with a restart rule other than `no` (F-S2). For launchd, the attempts did not
   stop in 28 s (F-L2).
7. **WDG-029.** A restart by the watchdog through `docker restart` is not in `RestartCount`
   (F-D10), and one through `systemctl restart` is not in `NRestarts` (F-S6). A check that counts
   restarts from these counters does not count the watchdog's restarts. This record does not
   check how CHK-014 counts.
8. **WDG-007.** Docker reports `unhealthy` and does nothing more (F-D8). The probe kind "Docker
   health" does not conflict with the supervisor.
9. **WDG-018 and WDG-025, Docker.** `paused` is a stop by a person that the status shows (F-D9).

## 8. Cleanup

The scripts removed the launchd job, all `g1-*` containers, the image `g1-watch:local` and the
`g1-watch-*` units. The files in `/tmp` of the Linux machine were deleted through `anima ssh`.
`launchctl enable` in case L3 left one line `"ing.anima.g1.watch" => enabled` in the launchd
override list of user 501. It has no effect: no job has that label. No command to remove the line
was tried.
