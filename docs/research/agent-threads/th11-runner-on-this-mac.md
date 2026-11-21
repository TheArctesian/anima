# TH11: the thread runner with the real programs on this Mac

Version 1, 2026-11-20. State: a measured record (tasks TH11, TH12, TH13, TH15, TH17 of
[agent-threads.md](../../proposals/agent-threads.md)).

## 1. What was run

- The build of the branch `threads/runner` (debug), not the installed application. The runner ran
  as `anima-daemon threads` with `ANIMA_DAEMON_HOME` set to a folder of the test
  (`/private/tmp/claude-501/at/daemon`). The installed daemon and its LaunchAgent were not changed.
- The commands were the `anima agents` commands of the same build: `start`, `watch`, `approve`,
  `send`, `interrupt`, `stop`, `running`.
- `claude` 2.1.311 (Claude Code) and `codex-cli` 0.170.0, found on the `PATH` of the login shell,
  with the login of the person on this Mac. Small prompts in two empty folders.

## 2. Results

| Run | What was done | Result |
|---|---|---|
| 1 | Claude Code, mode Ask: "create hello.txt with the Write tool" | The session ID is the thread ID (THR-088). The Write call came as a request "change a file" with its path. After `approve` the file was there with "hi", the reply "done" came, the state went to "your turn". A second `approve` of the same request was refused: "That request is answered, closed, or its program ended." |
| 2 | Codex, mode Ask: "mkdir made-by-codex" | `initialize`, `thread/start` and `turn/start` in order. A request "run a command" for `pwd -P` came. |
| 3 | The runner was ended with SIGTERM while the request of run 2 was open, and started again | The holder kept both programs. `running` showed Claude Code "idle" and Codex "needs_you". The request kept its ID; `approve` from the new runner was taken, and Codex went on to the next request (`mkdir made-by-codex`); after its approval the folder was there and the turn ended. |
| 4 | Codex, Full access: "run sleep 418" | The command ran in a process group of its own below `codex app-server`. `interrupt` ended the turn ("interrupted"); no `sleep 418` was left; the program stayed, idle. |
| 5 | Claude Code, Full access: "run sleep 419" | Claude Code did not run it: a rule of the user's own settings blocks a bare `sleep`. This shows that the settings of the user apply (THR-046). |
| 6 | Claude Code, Full access, the same thread: a long `perl` command | The command ran in a process group of its own. `interrupt` ended it; no process was left; the program stayed. |
| 7 | Codex, Full access, the thread of run 4: a long `perl` command, then `stop` | "The program and its processes ended." The command and `codex app-server` were gone; the thread showed "stopped". |

The log of the runner (`runner.log`, mode 0600) had a line for each start, prompt, answer, stop and
refusal, with no text of a prompt.

## 3. What was found and changed

- The state folder of a test in the scratch folder gave a socket path over the limit of 104 bytes
  of a Mac; the holder could not bind and the runner said only "the holder did not answer in time".
  The runner now refuses a folder whose socket path is over 100 bytes, with that reason.
- After a restart, a thread showed "starting" for a moment. A restored thread now keeps the last
  state that its program said until the program says more.

## 4. What this record does not show

- Linux.
- A question of an agent (AskUserQuestion of Claude Code, `requestUserInput` of Codex) through the
  runner: the tests use the lines of TH10; no real question was asked.
- A close of the desktop window: the client is not built yet. A restart of the runner stands for it
  here.
- A process that an agent started to leave the tree on purpose (a daemon); the stop does not find it.
