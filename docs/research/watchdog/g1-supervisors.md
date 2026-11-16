# Record: G1, what the supervisors show (watchdog prototype)

Version: 1. Date: 2026-11-16.
Status: Partly measured. The macOS part (launchd and Docker) is measured. The Linux part
(systemd, and the right of WDG-040) is not measured: it needs the operator's Linux machine.

This document is a record (work plan, Section 9; PLN-010). It adds no requirement. Its last
section lists inputs for the owner of the requirements, [watchdog.md](../../programme/proposals/watchdog.md).

## 1. Purpose

Task G1 of [watchdog.md](../../programme/proposals/watchdog.md) asks for measured answers to three
questions:

1. Which signs tell a stop by a person from a failure, for each supervisor (WDG-025)?
2. Which states mean that the supervisor starts the program again, so that the watchdog must
   wait (WDG-023)?
3. How does the second unit of a Linux system installation get the right to restart (WDG-040)?

## 2. Method

- The watched program is `g1prog` ([g1/prog/main.go](g1/prog/main.go)). It is not part of Anima.
  It runs until SIGTERM and then exits 0, or it exits with a given code after a given time. Its
  mode `health` gives the result of a health probe.
- Each supervisor has one script. Each script starts the program in each case, reads the state
  of the supervisor each 0.5 or 1 second, and removes what it made at the end.

| Supervisor | Script | Machine | Result |
| --- | --- | --- | --- |
| launchd | [g1/launchd.sh](g1/launchd.sh) | macOS 27.0.1 (26A434), Apple silicon, a job of the user domain `gui/501` | [launchd log](g1/results/launchd-macos-27.0.1.log) |
| Docker | [g1/docker.sh](g1/docker.sh) | Docker Engine 29.9.3 in Docker Desktop, linux/arm64, on the same Mac. The image is built from `scratch`; nothing is downloaded. | [Docker log](g1/results/docker-29.9.3-desktop-arm64.log) |
| systemd | [g1/systemd.sh](g1/systemd.sh) | Not run | — |

The scripts read the state the way that a watchdog can read it: `launchctl print`,
`docker inspect` and `docker events`, `systemctl show` and the journal. None reads the process
table (WDG-006).

## 3. Results: launchd

Cases L1 to L11 of the launchd log. A job has the KeepAlive rule written in the case.

| Case | What happened | What `launchctl print` shows |
| --- | --- | --- |
| L1 | KeepAlive true. The process gets SIGKILL (a crash). | `state = spawn scheduled` for 8 s, then `running`. `runs` goes from 1 to 2. `last terminating signal = Killed: 9`. |
| L2 | KeepAlive true. A person sends `launchctl kill SIGTERM`. The program exits 0. | The same as L1: `spawn scheduled` for 6 s, then `running`, `runs` goes from 2 to 3. `last exit code = 0`. launchd starts the job again. |
| L3 | KeepAlive true. A person sends `launchctl disable`, then `launchctl bootout`. | After `disable`: no change, the job runs. `print-disabled` shows `=> disabled`. After `bootout`: `Could not find service`. The `disabled` mark stays after `bootout`. |
| L4 | KeepAlive `SuccessfulExit = false`. The program exits 1 after 2 s, each time. | `spawn scheduled` for 8 s, `running` for 2 s, again and again: a start each 10 s. No end in 25 s. |
| L5 | KeepAlive `SuccessfulExit = false`. A person sends `launchctl kill SIGTERM`. The program exits 0. | `state = not running`, `last exit code = 0`. launchd does not start it again. |
| L6 | No KeepAlive. The program exits 3. | `not running`, `last exit code = 3`. |
| L7 | No KeepAlive. The process gets SIGKILL. | `not running`, `last terminating signal = Killed: 9`. |
| L8 | No KeepAlive. A person sends `launchctl kill SIGTERM`. The program exits 0. | `not running`, `last exit code = 0`: the same as a program that ends by itself. |
| L9 | No KeepAlive, not running: `launchctl kickstart -k`. | `xpcproxy` for less than 1 s, then `running`. `runs` goes from 1 to 3 and `last terminating signal = Terminated: 15`, although no process ran. |
| L10 | No KeepAlive, not running: `launchctl kickstart` (no `-k`). | `running`. `runs` goes from 1 to 2. `last exit code` does not change. |
| L11 | No KeepAlive, running: `launchctl kickstart -k`. | The process stops and starts again. `runs` goes from 2 to 3. |

Findings:

- **F-L1.** While launchd waits to start a job again, `state` is `spawn scheduled` (L1, L2, L4).
  `xpcproxy` is a short state during a start (L9). These are the states to wait for (WDG-023).
- **F-L2.** launchd waits until 10 s after the last start before it starts the job again (L1,
  L4). It does not stop its attempts: in L4 it started the job each 10 s.
- **F-L3.** `launchctl kill` leaves no sign of a person. A program that exits 0 on SIGTERM
  shows `last exit code = 0` (L2, L5, L8). A program that does not catch SIGTERM shows
  `Terminated: 15`; a crash in a program can give the same signal.
- **F-L4.** With KeepAlive true, a person cannot stop a job with `launchctl kill`: launchd starts
  it again (L2). The only stops by a person that launchd shows are a job that is not loaded
  (`bootout`, L3) and a job that is disabled (`print-disabled`, L3).
- **F-L5.** `kickstart` without `-k` is the correct restart of a job that is not running (L10).
  `kickstart -k` on a job that is not running counts two runs and reports `Terminated: 15`
  (L9). The watchdog's own restart would then look like a stop by SIGTERM.

## 4. Results: Docker

Cases D1 to D10 of the Docker log. The events are from `docker events`.

| Case | Policy | What happened | `docker inspect` | Events |
| --- | --- | --- | --- | --- |
| D1 | `no` | Exit 3 after 2 s. | `exited`, `exit=3` | `die exitCode=3` |
| D2 | `on-failure` | Exit 1 after 1 s, each time. | `restarting` (with `Restarting = true`) between the runs. The waits grow: less than 0.5 s before each of the first three runs again, then about 1, 1.5, 3 and 5.5 s (read each 0.5 s). `RestartCount` goes from 0 to 8 in 20 s. `ExitCode` is 0 while the program runs. | one `die` and one `start` for each run |
| D3 | `on-failure:2` | Exit 1 after 1 s, each time. | After 2 restarts: `exited`, `exit=1`, `RestartCount = 2`, equal to `MaximumRetryCount`. | `die`, `start`, `die`, `start`, `die`. No event says that Docker stopped its attempts. |
| D4 | `always` | A person: `docker stop`. | `exited`, `exit=0`. Docker does not start it again. | `kill signal=15`, `stop`, `die exitCode=0` |
| D5 | `always` | A person: `docker kill` (SIGKILL). | `exited`, `exit=137`. Docker does not start it again. | `kill signal=9`, `die exitCode=137`. No `stop`. |
| D6 | `always` | A person: `docker kill --signal TERM`. The program exits 0. | `exited`, `exit=0`. Docker does not start it again. | `kill signal=15`, `die exitCode=0`. No `stop`. |
| D7 | `always` | The health probe fails after 6 s. | `running`. `health` goes `starting`, `healthy`, then `unhealthy` 1.5 s after the failure. Docker does not restart it. | `health_status: healthy`, `health_status: unhealthy` |
| D8 | `always` | A person: `docker pause`. | `paused` | — |
| D9 | `no` | `docker restart` (the restart of WDG-021). | `running`. `RestartCount` stays 0. | `kill signal=15`, `stop`, `die`, `start`, `restart` |
| D10 | `no` | After `docker stop`: `docker start`. | `running` | — |

Findings:

- **F-D1.** While Docker waits to start a container again, the status is `restarting` (D2). This
  is the state to wait for (WDG-023). The waits grow with each attempt.
- **F-D2.** `ExitCode` is 0 while a container runs, also after failures (D2). Only
  `RestartCount` keeps the history of the restarts by Docker.
- **F-D3.** When Docker stops its attempts under `on-failure:N`, no event and no status says so.
  The sign is `exited` with `RestartCount` equal to `MaximumRetryCount` (D3). This is the case
  of WDG-024.
- **F-D4.** Each stop through the Docker API stops the restart policy, also `always`:
  `docker stop` (D4) and `docker kill` with any signal (D5, D6).
- **F-D5.** `docker inspect` alone cannot tell a stop by a person from a failure. `exited` with
  `exit=0` is the same after `docker stop` (D4) and after `docker kill --signal TERM` (D6), and a
  program that ends by itself with 0 gives it too. `exit=137` after `docker kill` (D5) is the code
  of any SIGKILL. The sign of a person is in the events: each stop through the API wrote a `kill`
  event before the `die` (D4, D5, D6, D9), and each failure wrote a `die` only (D1, D2, D3).
  A `stop` event came only from `docker stop` and `docker restart` (D4, D9). Not measured: a
  SIGKILL from a process outside the Engine, which is expected to write a `die` only.
- **F-D6.** One sign comes from `inspect` alone: a container with the policy `always` or
  `unless-stopped` that is `exited` and not `restarting` was stopped through the API (F-D4).
  For the policies `no` and `on-failure`, this sign does not apply.
- **F-D7.** Docker does not restart an `unhealthy` container (D7). With the probe kind "Docker
  health" of WDG-007, only the watchdog restarts it.
- **F-D8.** `docker restart` does not increase `RestartCount` (D9). A restart by the watchdog
  is thus not in the count that Docker keeps.

## 5. Not measured

| Question | Why | What is needed |
| --- | --- | --- |
| systemd: the states to wait for (WDG-023) and the signs of `systemctl stop` (WDG-025) | No Linux machine here. Docker Desktop runs no systemd, and an image with systemd needs a download. | Run [g1/systemd.sh](g1/systemd.sh) on the operator's Linux machine and add its log to `g1/results/`. |
| The right of WDG-040 (OD-WDG-2) | It needs root on a Linux machine and a polkit rule. | The three manual steps at the end of [g1/systemd.sh](g1/systemd.sh). |
| Docker on Linux, not in Docker Desktop | Docker Desktop runs the Engine in a virtual machine. | Run [g1/docker.sh](g1/docker.sh) on the Linux machine. The results are expected to be the same, because the restart policy and the events are in the Engine. |
| Docker after a restart of the daemon or the machine | To restart Docker Desktop stops the operator's other containers. | A run on the Linux machine. Docker starts `always` containers again after a restart of the daemon, also after `docker stop`; `unless-stopped` containers stay stopped (Docker documentation, not measured). |
| How long the Engine keeps events for `docker events --since` | Not in the scripts. | A run that reads the events after an agent restart. |
| A launchd system job (LaunchDaemon) | It needs root. OD-WDG-10 recommends the jobs of the user only. | Only if OD-WDG-10 is decided the other way. |

## 6. Inputs for the owner (watchdog.md)

These are not requirements. They are inputs for the operator's decisions and for the next version
of [watchdog.md](../../programme/proposals/watchdog.md).

1. **WDG-023:** the states to wait for are `spawn scheduled` and `xpcproxy` for launchd (F-L1),
   and `restarting` for Docker (F-D1). The systemd states stay to be measured.
2. **WDG-025, launchd:** launchd cannot tell a person's `launchctl kill` from a program that exits
   (F-L3). By the rule "when the watchdog cannot tell, it must not restart", the watchdog would
   restart a launchd job only after a nonzero exit code or a crash signal (for example SIGSEGV,
   SIGABRT, SIGBUS, SIGILL, SIGKILL), and treat `not running` with exit 0 or SIGTERM as "cannot
   tell". A person who wants a job to stay stopped uses `bootout` or `disable` (F-L4). The client
   text of "Stopped by a person" can name these two commands.
3. **WDG-025, Docker:** to tell a person, the watchdog must read the events of the Engine while
   it runs, and keep the last `kill` and `stop` event of each watched container (F-D5). After a
   restart of the agent, it can use the sign of F-D6, or `docker events --since` (not measured).
4. **WDG-021, launchd:** the restart of a job that is not running is `launchctl kickstart`
   without `-k` (F-L5). `kickstart -k` is for a job that runs and does not answer.
5. **WDG-024, Docker:** "Docker stopped its own attempts" is `exited` with `RestartCount` equal
   to `MaximumRetryCount` under `on-failure:N` (F-D3). launchd never stops its attempts (F-L2).
6. **WDG-029:** a restart by the watchdog through `docker restart` is not in `RestartCount`
   (F-D8). The Docker check (CHK-014) would not see it. The watchdog's report (WDG-031) must then
   be the source for the count of WDG-029.
7. **WDG-007:** Docker reports `unhealthy` and does nothing more (F-D7), so the probe kind
   "Docker health" does not conflict with the supervisor.

## 7. Cleanup on the Mac

The scripts removed the launchd job, all `g1-*` containers and the image `g1-watch:local`.
`launchctl enable` in case L3 left one line `"ing.anima.g1.watch" => enabled` in the launchd
override list of user 501. It has no effect: no job has that label. To remove the line needs root.
