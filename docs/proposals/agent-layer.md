---
kind: Proposal
state: Direction decided
evidence: Version 2 (2026-11-20). The direction is R-NEW-agentlayer (relayed twice). The requirements AGL-001 to AGL-034 are not approved, and no code exists. Nothing is built before the operator approves the requirements, and the build comes after the launch path. A separate security read of version 1 is done (SF-NEW-agl01 to SF-NEW-agl18, 4 High). Version 2 corrects the text for the eight findings that need no decision; the ten that need the operator are OD-AGL3 and OD-AGL13 to OD-AGL20, and he has decided none. No reviewer read version 2.
---

# Proposal: the agent layer of Anima (one list of agents on each machine, a waiting state from hooks, one alert, sessions that stay)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this proposal (PLN-005).

Version: 2. Date: 2026-11-20. Version 2 has the corrections of the security read of version 1 (Section 10); each changed requirement names its finding.
Status: the header of this document holds the state (PLN-020).
Map: [work-plan.md](../work-plan.md), categories Reach and Use, row 4.21 (agent tools as extensions) for the list, and row 4.7 (jobs, builds and AI agent runs) for a start of a program on a machine.
From: R-NEW-agentlayer.
Identifiers: AGL-001 to AGL-034, OD-AGL1 to OD-AGL20, tasks AL0 to AL12.
Language: Simplified Technical English (design specification, Section 0).

## 1. What was decided, and what was not

R-NEW-agentlayer (operator, 2026-11-20, relayed twice) decides a direction and no requirement. In the terms that reached this document:

- Anima builds a thin layer of its own across the machines of the operator, beside the hosted tools: one list of each agent on each machine; a state "waiting for you" that comes from the hooks of the agents, not from a read of the screen; one alert when an agent needs him; sessions that the daemon keeps alive.
- It builds no diff screen, no chat view and no terminal host of its own at first.
- Hosting stays. R-196 is not reversed. Herdr, or a different tool that runs on a machine, is in the list of Anima as each other agent is.
- The question "embed or replace" waits until he has used the layer.

Not decided: each requirement below, each question of Section 9, and the time of the build. The fork told him that this is not launch work and starts after the launch path (the release, the metrics, Agent usage, Projects, the web hub). He did not answer that. This document takes it as the order until he says a different one.

## 2. Terms

| Term | Meaning |
| --- | --- |
| Agent program | A coding agent that a person starts in a terminal. Version 1 knows two: Claude Code and Codex, the two that the usage counts read (AIS-018) |
| Live agent | One agent program that runs now, or ended in the last 24 hours, in one folder on one machine. It is the row of the list. It is not a "session" of the client: "Live sessions" in the bar are the connections to a device (SSH, RDP, VNC), and "sessions" on the Agent usage page is a count |
| State | One word of a fixed list (AXT-048 owns the list) |
| Hook | A command that an agent program runs at an event, by its own configuration file. The person, or Anima with a click of the person, writes that file |
| Signal | What a hook gives to the daemon: a fixed set of fields (AGL-008) |
| Kept session | An agent program that runs in a `tmux` session that Anima named, so that it stays when a window or a connection closes |

## 3. What exists, and who owns it

This layer is not new ground. One document already owns most of the list. This proposal adds only what that document does not have, and names each text of it that a ruling must change. It copies no requirement (PLN-016).

| Subject | Owner today | State | What this proposal does |
| --- | --- | --- | --- |
| The list of agents on all devices, read-only, with a state for each | [agent-tools-as-extensions.md](agent-tools-as-extensions.md) Section 6.8, AXT-046 to AXT-052 (R-243); tasks AX17 to AX19 | Proposed, not approved | Uses it as it is. It asks for three changes (Section 8) |
| The list of states | AXT-048: "Working, Waiting for input, Done, Idle, Stopped, Unknown. A source that cannot tell a state gives Unknown, never Working." | Proposed | Uses it. The group of the client is named "Waiting for you"; the state is "Waiting for input" |
| One alert when an agent waits | AXT-053 (R-244, OD-AXT20: "yes, alert when an agent waits for input.") | Proposed, with an open point: the session files cannot tell Waiting | Gives the source that can tell it (Section 5.2) and the rules of the alert (Section 5.5) |
| The order of the sources of a state | AXT-047: Herdr first; then the protocol of the idea `agent-state-hooks`; then the session files of the daemon. R-196 item 10: "Version 1 leaves the state of each agent to the screen of Herdr; the idea `agent-state-hooks` and the Needs-you inbox come next." | Proposed | Makes the hooks the first source, and defines the protocol that AXT-047 only names (Section 5.2) |
| The usage counts and the Agent usage page | [extensions-plan.md](../plans/extensions-plan.md) Section 4.12.0, AIS-018 to AIS-024 (approved, R-135); the page v2 is on the branch `hosted/agent-usage-v2` | Built; v2 not merged | Changes nothing in the counts. The list is a second source beside them (Section 5.4). The place is one page (Section 6) |
| A `tmux` session that Anima names and owns | AXT-007 (through the SSH of the hub) and BOT-069 of [bots.md](bots.md) | Proposed | Adds a second way to keep a session, by a unit of the person (Section 5.6), and asks which of the two (OD-AGL7) |
| A program that an AI agent or an API key starts on a machine | [routed-command-lines.md](routed-command-lines.md), RCL-001 to RCL-024 (R-258); its security review | Draft; 24 findings, none decided | Starts nothing for an agent or a key (AGL-027). A kept session is the same subject, so it goes through the same review (Section 10) |
| Which machine gets work | [jobs.md](jobs.md) ROU-001 to ROU-016 (R-243, R-244); task W3 part 3 | Proposed | Changes nothing. A kept session that the operator starts names its machine |
| The daemon of a person at sign-in | [hosted-mode.md](hosted-mode.md) Section 6.8, HMD-052 to HMD-058 (R-206, R-240) | Built on branches, not checked; its security review (HM-T23) is not started | Stands on it. On a Mac that daemon runs as the person |
| The ideas | `docs/ideas/agent-state-hooks`, `needs-you-inbox`, `per-agent-resource-attribution` | Framed; "The operator has not picked this idea." | R-NEW-agentlayer takes the first as its source. Of the second it takes only the alert; an answer from a phone is not in this proposal. The third is not in this proposal |

What the code has today (read on 2026-11-20): the daemon reads the session files of Claude Code and Codex each 15 minutes and sends counts (`crates/daemon/src/readers/agent_sessions.rs`); the hub keeps sums (`crates/hub/src/fleet/agent_usage/`). There is no state of an agent, no list of agents, no command `anima agents list`, no code for a hook, no `tmux`, and the daemon starts no program other than fixed read-only commands (`crates/daemon/src/readers/command.rs`). The research list says that Anima has "Active or Idle only, from session files (a write in the last 2 minutes)": that is the text of AXT-047, not code.

## 4. What the research gives, and how far to trust it

The list of features is `agent-multiplexer-feature-list-2026-11-20.md` (outside the repository; written from research notes and from pages that an agent fetched on 2026-11-19). It installed no tool. Each line below is a claim of that list, not a result.

- Each tool of the list has: sessions that stay when a window closes, a state for each agent, a notification, a worktree for each agent, a second machine through SSH, a command line and an MCP server, and by now a phone client.
- What sets a tool apart: a waiting state that a person can trust, an answer from a different place, and a prompt that is queued or sent without an attach.
- The states are the weak point of each tool. Hooks change with each release of an agent program; some agent programs send no signal for "waiting"; Herdr 0.9.5 went back from hooks to a read of the screen for Codex, and has open reports of wrong states.
- Herdr is now two products that do not share agents: the terminal program (0.9.5), and a desktop program on a new daemon (`herdr-core`) that gives a state only through the escape sequence OSC 7501. The phone programs work only with the second. Herdr Cloud is a waiting list.

What that means for R-196 (item 2: "If Herdr Cloud ships or the Herdr project stalls, the operator reviews this decision."): the condition is not met, so R-196 stands. Two things follow that R-196 does not say. (1) "Herdr" in AXT-020 and AXT-024 is the terminal program, the only one of the two that an SSH terminal can show (AXT-014); AXT-025 already excludes the desktop program. That is the reading of this document; the owner of AXT decides it (OD-AGL12). (2) AXT-047 puts Herdr first as the source of a state, and OD-AXT19 says "then Herdr's own interface once AX17 finds one". With two products that do not share agents, a list that reads Herdr sees only the agents of one of them. A hook of the agent program itself fires in each of them and with no Herdr. That is the reason for the order of Section 5.2.

The hooks themselves. The research list names no event. From the documentation of the two programs as the writer of this document knows it, not fetched on this day and not run: Claude Code runs a command at the events `SessionStart`, `UserPromptSubmit`, `Notification` (the program needs a permission or waits for an input), `Stop` (the turn ended) and `SessionEnd`, and gives it a JSON object with the session identifier, the path of the transcript, the working folder and, for `Notification`, a text. Codex runs one program (`notify`) with a JSON object when a turn ends. Task AL0 proves or corrects each of these words with recorded events before a requirement is approved (the finding AGENT-WEDGE-02 of the research asks for the same).

## 5. Requirements (proposed)

### 5.1 The limits of the layer

- **AGL-001.** Version 1 of the layer must add no screen that shows a difference of files, no view of a conversation and no terminal host. A person looks at an agent in the terminal that Anima has (the SSH terminal) or in a hosted tool (R-196).
- **AGL-002.** The layer must not depend on a hosted tool. An agent program that runs in Herdr, in T3 Code, in a `tmux` session or in a plain terminal must be in the list by the same rules.
- **AGL-003.** The layer must take no power from, and give no power to, an AI agent or an API key (AGL-027). Each requirement of this document is for the operator as a person.

### 5.2 The state of a live agent comes from its hooks

- **AGL-004.** The daemon must take the state of a live agent from these sources, the first that has an answer: (1) a signal of a hook of the agent program; (2) the session files that the usage reader reads, which can tell only that a file was written in the last 2 minutes or not. A hosted tool is not a source in version 1. This changes the order of AXT-047 (Section 8).
- **AGL-005.** The source 2 must give only `Idle`, `Stopped` or `Unknown`. It must never give `Working`, `Waiting for input` or `Done` (AXT-048: a source that cannot tell a state gives Unknown). It adds no state to the list of AXT-048: the time of the last change of the row shows that a file was written a short time before (SF-NEW-agl15). Thus a machine with no hook shows no agent as waiting, and the client says that the hooks are not installed there.
- **AGL-006.** The map from an event to a state must be a table in the daemon, one for each agent program and each version range that task AL0 recorded. An event that the table does not know must give `Unknown`. The proposed table, to prove in AL0:

| Program | Event | State |
| --- | --- | --- |
| Claude Code | `SessionStart` | Idle |
| Claude Code | `UserPromptSubmit` | Working |
| Claude Code | `Notification` | Waiting for input |
| Claude Code | `Stop` | Done |
| Claude Code | `SessionEnd` | Stopped |
| Codex | `notify`, the turn ended | Done |

- **AGL-007.** A state must have an age. A state `Working` or `Waiting for input` must become `Unknown` when no signal came for that agent and its session file was not written for 10 minutes. The daemon looks for no process. The client shows the time of the last change of each row (AXT-046). `Notification` can have more than one cause, and a permission that the person gives can end a wait with no event (not verified): task AL0 must show which form of the event means a wait and which event ends it. For a program whose record does not show the two, the state `Waiting for input` and the alert of AGL-020 are not built (SF-NEW-agl15).
- **AGL-008.** A signal must hold only these fields: the program (`claude_code`, `codex`); the identifier that the program gives its session, with the alphabet and the length that contract K8 gives a session identifier, and a signal with a different one is dropped; the name of the event (one of a fixed list); and the repository name of the folder, in the form and with the exclusions of AIS-018. The hook program of Anima reads the object that the agent program gives it and must drop each other member before it writes: the text of a notification, the path of the transcript, the prompt, the names and the arguments of tools, and the working folder as a path. The hook program finds the repository name and the exclusion itself, from the working folder, before it writes: for an excluded repository, and for a machine whose counts are off, it writes nothing. So no row and no alert exists for an excluded repository at any moment (SF-NEW-agl08). The daemon, not the hook, gives a signal its time (SF-NEW-agl07).
- **AGL-009.** The hook must give its signal to the daemon with no listener and no network: the command `anima-daemon agent-signal` appends one line of fixed form to a file in the folder of the daemon, which only that user can read and write, and the daemon reads the file. The command must end in less than 100 ms and must always exit with 0, so that a fault of Anima never stops or slows an agent program. It must print nothing, on success and on each error, write no log, make no core file and send no error report: the output of a hook can go into the context of the agent program (SF-NEW-agl11). It must read at most 64 KiB of its input. It must open the file as the daemon opens its token file: a regular file of that user, no link followed, not a pipe; a different thing in its place makes the command do nothing. The file has a limit of 1 MiB; the daemon takes it away by a rename and reads the renamed file (SF-NEW-agl14). Codex gives its object as an argument of the program, which each user of the computer can read in the process list while the hook runs (not verified): Anima cannot change that, task AL0 records it, and the installer says it before a person installs the hook for Codex.
- **AGL-010.** The daemon must take a signal as a claim of a program of that user, not as a fact: each program of the user can write the file. A signal can make a row, change the state of a row, cause a batch and, through AGL-020, cause an alert; it can do nothing more: it must not start a program, change a setting or widen a right. A program of the user can thus show a false wait or hide a real one; this document does not prevent that (OD-AGL16). The limits: the daemon keeps at most 64 rows and, when there are more, keeps first the rows whose identifier a session file of the usage reader has; the hub sends at most 6 alerts in an hour for one machine (SF-NEW-agl07).

### 5.3 The hooks are installed by a click at the machine

- **AGL-011.** The daemon must not write the configuration file of an agent program by itself. The hub must not be able to make it do so. The person installs the hooks at the machine: a setting "Agent states" of that machine, off by default, that the hub cannot set, or the command `anima-daemon hooks install`.
- **AGL-012.** Before it writes, the installer must show the file and the exact lines that it adds. It must write only the configuration file of the program in the home folder of the user, never a file in a repository. It must follow no link: a link in place of the file stops it. Its entry must name `anima-daemon` by the full path of the program that runs the installer, never by its name alone. It must add only its own entries, keep each entry that is there, and write a copy of the file beside it first, which only that user can read (the file can hold a token). Codex has one `notify` program (not verified): when one is set, the installer changes nothing for Codex and says so. `anima-daemon hooks remove` must remove exactly its own entries, and the removal of the daemon runs it (SF-NEW-agl10).
- **AGL-013.** The installer must not run when the daemon is a system unit (AIS-024), and must write only files of the user that runs the daemon.
- **AGL-014.** The device page must show, for each agent program, if its hooks are installed, and the version of the program that the table of AGL-006 knows. The daemon takes the version only from a member of the hook object or of a session file, if task AL0 finds one there; it must not run the agent program to ask (CAP-022). When a program has a newer version than the table knows, or no version can be read, the states of its agents must be `Unknown` until a record for that version exists. The two facts ("hooks installed", the version) leave the device for each program, beside the rows of AGL-015 (SF-NEW-agl16).

### 5.4 What leaves a device, and what never does

The rule that exists: AIS-018, "It must not send text, a title or the path of a folder"; LOG-007; RA-008, "The hub must not record the session content"; R-133, counts only. A state is a word of a fixed list, so it is not text of a session. A name that a person wrote is text.

- **AGL-015.** For each live agent the daemon must send only: the program; the identifier of the session, which AIS-018 already sends; the state, from the list of AXT-048; the time of the last change of the state; the source of the state (`hook`, `files`); if Anima keeps the session (AGL-022); and the repository name in the form `host/owner/name` of the remote `origin`, by the rule and with the exclusions of AIS-018. A folder with no remote `origin` has no name: the row says "a folder with no repository".
- **AGL-016.** The daemon must never send, and the hub must never hold: the text of a prompt, of an answer or of a notification; the question that the agent waits on; the title of a session; the name or the arguments of a tool; a path; a line of a terminal; the name of a file.
- **AGL-017.** The name of a branch is text that a person wrote, and it can hold the name of a customer or the title of a ticket. The daemon must not send it unless OD-AGL3 permits it. If it does: only for a repository that is not excluded, at most 100 bytes of the characters `A-Z a-z 0-9 . _ / -`, each other character replaced by `?`, and never on a hub that Anima.ing runs unless the person set it on at the machine.
- **AGL-018.** The live path must be a new optional member of the push batch of the daemon (contract K8, Section 4), beside `agentUsage` and not inside it: at most 64 rows, sent in the next batch after a state changed and at least each 5 minutes, and removed 24 hours after the end of an agent. The hub must keep the last state of each row and no history of states. The hub must delete a row 24 hours after its end, a row that no batch named for 24 hours, and each row of a machine that is removed. A person cannot check from a device what a hub keeps: on a hub that a different party runs, this sentence is a promise of that party (SF-NEW-agl09; OD-AGL9, OD-AGL18). This is a change of K8 and of K1 by a ruling (CON-001). The usage counts stay as they are.
- **AGL-019.** The switch that turns the usage counts off for a machine, and the list of excluded repositories of that machine, must apply to the live rows in the same way: a machine that sends no counts sends no rows.

### 5.5 One alert when an agent needs the operator

- **AGL-020.** When a live agent changes to `Waiting for input` from a source `hook`, the hub must send one notification through the channels that exist (NOT-001 to NOT-008). It must send no second one for the same agent while it stays in that state, and none for a row whose source is `files`.
- **AGL-021.** The text of the alert must be fixed words with the name of the agent program, for example "Claude Code waits for you". It has no template of the user (NOT-007 does not apply to it), it is sent one time with no second sending (NOT-005 does not apply), and the person selects its channels in a setting of its own, because it belongs to no check (NOT-003). It must not hold the question, a path or a branch. The name of the machine, the repository name and a link to the list are in the alert only by OD-AGL5 and OD-AGL19: a channel such as ntfy is a server of a different party, which also learns the time of each wait. The alert is off until the person sets it on in Settings, Notifications (SF-NEW-agl12).

### 5.6 Sessions that stay

This part is the second step and has its own gate (Section 10). It is the first time that the daemon side starts a program that is not a fixed read-only command.

- **AGL-022.** A kept session must be a `tmux` session with a name that Anima gives (`anima-<number>`), in a `tmux` server of the user, with one agent program in one folder. Anima must not install `tmux`. A machine with no `tmux`, and a Windows machine, has no kept session, and the client says why.
- **AGL-023.** The `tmux` server must not be a child of the daemon. It must be a unit of its own of the person (a `LaunchAgent` on a Mac, a systemd user unit on Linux) with no sandbox, because an agent must write the repositories of the person. The `tmux` server has a socket of its own in a folder that only that user can open, and starts with no configuration file of the user. The daemon must ask the unit through a written interface with two requests and no more: start (the number of a program of the fixed list, the number of a repository of the list that the unit reads itself) and stop (a name). The unit checks each request itself; no request carries a text that reaches a command line. A stop, a restart or an update of the daemon must not end a kept session. On Linux the unit of the daemon keeps its protections, and it must be changed so that it reaches that one socket: a change that the review of the unit sees again. On a Mac the daemon has no sandbox today, so there the two programs have the same rights. On Linux a unit of a user ends when the person logs out, unless the system setting "linger" is on for that user; Anima does not set it, and the client says when a kept session will end at logout (SF-NEW-agl03, SF-NEW-agl18). (As the units are written today, launchd ends the process group of the daemon, and the systemd user unit has a home folder that it cannot write, a `/tmp` of its own and a stop that ends each child. Read from the unit files; not tried.)
- **AGL-024.** What can run in a kept session must be a fixed list in the program: the agent programs of Section 2, with no argument that the caller gives. The unit starts each by a full path that the person confirmed at the machine when the setting was set on, not by a search of the path at each start. The folder must be a folder of a repository that the daemon already found for the usage counts. There is no command line, no argument, no environment variable and no path that comes from the hub. The agent program reads its own configuration and that of the repository at its start, which can run commands with no question (not verified); the environment that it gets is OD-AGL17 (SF-NEW-agl05).
- **AGL-025.** Only the operator, as a signed-in person, can start or stop a kept session. The request must name the machine. The machine must have the setting "Keep agent sessions" on, which is off by default and which the hub cannot set. The setting is a file of the user: each program of that user can change it (OD-AGL16). How a request reaches the machine, and what at the machine proves that it comes from the operator, is not decided: a request that the hub carries is proved by nothing at the machine (SF-NEW-agl02). It is OD-AGL14, and no start through the hub is built before it.
- **AGL-026.** The person must reach a kept session with a terminal that exists: at the machine, `tmux attach` with the socket of AGL-023; from a different place, an SSH terminal that runs the same command (AXT-014 has the same form). Anima adds no second way to see a terminal. A terminal through the hub is under the rules that exist for an SSH session (RA-008), and it needs a login that the hub holds: on a hub of Anima.ing that is a shell of the person in the hands of a different party, so it is not built there unless OD-AGL13 says so (SF-NEW-agl01).
- **AGL-027.** No interface of Anima (the API of the hub for a key, an MCP tool, a bot, a schedule, a routed start) must start, stop, attach to or type into a kept session by this document. A start for an agent or a key is the subject of [routed-command-lines.md](routed-command-lines.md) and of its review, and stays closed until that review is decided (R-258: "nothing that gives an agent a shell is merged"). This is a rule for the interfaces of Anima and no more: a program that runs as the person, also a coding agent with a shell, can type into each `tmux` session of that person, can answer its own question there, and can use the login of the person for `anima`. This document has no mechanism against that (SF-NEW-agl04; OD-AGL16). Before a row says that Anima keeps a session, the unit must find that the program of that pane is the one that it started.
- **AGL-028.** The hub must write one audit entry for each start and each stop of a kept session: who, the machine, the program, the repository name. No command text and no output (LOG-007).
- **AGL-029.** After a restart of the machine, Anima must not start a session again by itself. The row says `Stopped`.

### 5.7 Interfaces (CMD-030)

- **AGL-030.** The order of the build is the hub API, the command `anima`, the MCP tool, the client. `anima agents list` and the MCP tool `anima_agents_list` are those of AXT-050 and read only. There is no MCP tool for a start, a stop or an alert setting (AGL-027; MCP-006).
- **AGL-031.** A row that the MCP tool returns is data, not an instruction (MCP-008). It holds only the fields of AGL-015.

### 5.8 The place in the client

- **AGL-032.** The list must be on the Agent usage page, not on a page of its own and not as a new item of the bar (Section 6). A group "Waiting for you" is at the top of that page, and it is absent when no agent waits. The rows of other machines use the device dimension that the page has.
- **AGL-033.** The page must say in words what the hub holds for a live agent (the fields of AGL-015) beside its sentence "counts only, no text of a session leaves a device", so that the sentence stays true.
- **AGL-034.** When a machine is not reachable, each of its rows must be `Unknown` (AXT-052).

## 6. One screen, agreed with the two sessions that own it

Asked by message on 2026-11-20; both answered.

- The session of the interface review (it owns the drawing `docs/mockup/agent-usage-v2/index.html`, approved by the operator on 2026-11-19): "one screen, yes". No second screen and no new item in the bar. The requirements of the page are AIS-018 to AIS-024 of the extensions plan; this proposal names requirements, that session places them in the mockup, the session that owns the code builds. A row of v2 today is never a session: it is a sum for a repository, for a program with a model, or for a device. So a row with a state is a new source, with a change of K8 and K1. The mockup has a dashed box with a question to the operator: single sessions and "what runs now" are things that the hub does not hold. This proposal is the answer to that box, so he decides one time (OD-AGL2).
- The session of the hosted first use (it built the daemon at sign-in and the page v2): it agrees, as an opinion. The batch of the daemon already carries the session identifier with a start and an end; the usage reader runs each 15 minutes, which is too slow for a waiting state, so a hook must cause a batch. A state word is in the spirit of AIS-018 and AIS-022; a branch name is new text and needs its own requirement and the ruling of the operator. The facts of AGL-023 come from its read of the unit files.

A conflict to settle: AXT-046 puts the list in "a tab 'All devices' beside PNL-002" (OD-AXT21, R-244), and the mockup `docs/mockup/agent-routing` draws it as a page of its own. This proposal and the two sessions put it on the Agent usage page. One of the two must change (OD-AGL1).

## 7. Tasks, in the order that shows something first

No task starts before the operator approves the requirements that it names (PLN-007), and none before the launch path, apart from AL0, which is a record. AX17 to AX19 of the agent tools proposal are the same work as AL3 to AL6 for the list: one of the two sets is built, by the answer to OD-AGL1.

| Task | What | Needs | Shows |
| --- | --- | --- | --- |
| AL0 | A record: the hook events of Claude Code and Codex, recorded from real runs on a Mac and on Linux, for each state of AGL-006; which program versions; what an event holds. No product code | Nothing | If "waiting" can be trusted, before a requirement is approved |
| AL1 | The operator approves the requirements and decides Section 9 | AL0 | — |
| AL2 | The ruling for the contracts: the new member of K8, the fields of K1 (AGL-018), and the texts of Section 8 | AL1 | — |
| AL3 | The daemon: the command `agent-signal`, the file, the table of states, the source `files`, the rows in the batch | AL2, and the gate of OD-AGL20 | A state in the output of the daemon |
| AL4 | The hub: the last state of each row, the query; then `anima agents list`; then `anima_agents_list` | AL3 | The list in a terminal, for each machine |
| AL5 | The client: the group "Waiting for you" and the rows on the Agent usage page, from the mockup of the interface session | AL4, the mockup | The list in the application |
| AL6 | The installer of the hooks with its setting at the machine (AGL-011 to AGL-014) | AL3 | Hooks with one click |
| AL7 | The alert (AGL-020, AGL-021) | AL4 | One notification when an agent waits |
| AL8 | The human check: a day of real work with the list and the alert; the count of wrong states | AL5 to AL7 | If the state is right |
| AL9 | The gate of the kept sessions: the second read of the security review, of this text with the routed command line text, has no finding that the operator has not decided (R-258) | AL1 | — |
| AL10 | The unit of the kept sessions on a Mac and on Linux; a session that stays through a restart of the daemon | AL9 | A session that stays |
| AL11 | Start and stop, by the answer to OD-AGL14; the audit entries | AL10 | A start from the application |
| AL12 | After he has used it: the question "embed or replace" | AL8, AL11 | — |

## 8. Texts of other documents that a ruling must change

This document changes none of them. The ruling that approves it names them.

- AXT-047 of the agent tools proposal: the order of the sources (AGL-004); a hosted tool is not a source in version 1.
- AXT-046 and the mockup `docs/mockup/agent-routing`, frame 1: the place of the list, by OD-AGL1.
- AXT-053: the alert goes through the notification channels (AGL-020); the open point about the session files is closed by AGL-005.
- R-196 item 10 and OD-AXT19 ("then Herdr's own interface once AX17 finds one"): the state of version 1 comes from the hooks, with or without Herdr.
- AIS-018 of the extensions plan: a sentence that the daemon also sends the state of a live agent, and the branch name only by OD-AGL3.
- Contract K8 Section 4 and contract K1: AGL-018.
- The unit files of the daemon: the Linux user unit must reach the socket of the new unit of the kept sessions (AGL-023); the review of that unit sees the change.
- NOT-003, NOT-005 and NOT-007 of the design specification: the alert of AGL-021 is not of a check, is sent one time and has no template.
- Contract K10 Section 3, MET-002 and MET-013, if a start reaches a machine through the hub (OD-AGL14): the daemon then takes a request that starts a program.
- The ideas `agent-state-hooks` and `needs-you-inbox`: a line that names this document as the proposal that took them, by the rule of [ideas/README.md](../ideas/README.md). The stream that owns the ideas writes it.

## 9. Questions for the operator

| ID | Question | Options | Recommendation |
| --- | --- | --- | --- |
| OD-AGL1 | Where is the list? | A: on the Agent usage page, with "Waiting for you" at the top (the two screen sessions agree). B: a tab "All devices" beside the Agents page, as OD-AXT21 and the routing mockup have it. | A. One place for agents; the mockup of v2 already asks this question. It changes AXT-046 |
| OD-AGL2 | Does the hub hold a row for each live agent (a state and times), where today it holds only sums? | A: yes, the fields of AGL-015 and no history. B: no, counts stay the only data; then there is no list for a second machine and no alert. | A. It is the layer itself. It is the dashed box of the v2 mockup |
| OD-AGL3 | Does a branch name leave a device? | A: no. B: only on a hub that the person runs; on a hub of Anima.ing only when the person sets it on at the machine. C: always. | A. The limit of characters of AGL-017 stops an injection, not a disclosure: a name such as `fix/acme-breach-ticket-812` passes whole, and each agent that reads the list gets it (SF-NEW-agl13). The list works with the repository name |
| OD-AGL4 | Who writes the hooks into the configuration of an agent program? | A: the person, by a setting at the machine or a command, after the exact lines are shown (AGL-011). B: the daemon by itself at install. C: Anima writes nothing and shows the lines to copy. | A. B changes a file of a different program with no word of the person, on each computer that signs in |
| OD-AGL5 | Is the repository name in the alert? | A: no, the machine and the program only. B: yes. | A for a channel of a different party (ntfy, a webhook); B can be a setting later |
| OD-AGL6 | Which programs first? | A: Claude Code, then Codex when AL0 shows a waiting signal for it. B: both at once. | A. The research says that Codex has no reliable waiting signal |
| OD-AGL7 | How is a session kept? | A: a unit of the person with `tmux`, with no sandbox, beside the daemon (AGL-023). B: through the SSH of the hub, as AXT-007 has it. C: not in this layer; a hosted tool keeps sessions. | A on a hub of Anima.ing, where the hub has no SSH login of the person; B stays for a hub with SSH. C is the smallest and loses the fourth part of the direction |
| OD-AGL8 | Can a start of a kept session take an argument or a first prompt? | A: no, the program by its name only (AGL-024). B: a prompt, which the hub then carries. | A. A prompt through the hub is text of a session at the hub, and a free argument is a command line |
| OD-AGL9 | Is the layer on for a hub that Anima.ing runs? | A: yes, with the same switches as the usage counts. B: only on a hub that the person runs, until the review of the daemon in the application (HM-T23) is done. | B until HM-T23 |
| OD-AGL10 | Does the alert reach the phone application? | A: through the notification channels only, in version 1. B: also a push of the phone application. | A. The phone application is on a branch |
| OD-AGL11 | When does the build start? | A: after the launch path, as the fork told him. B: AL0 now, the rest after the launch path. C: now. | B. AL0 is a record of one day and tells if the core of the layer can work |
| OD-AGL12 | Which Herdr does Anima host (R-196), now that there are two? | A: the terminal program. B: wait for the desktop program. | A, and it is a question for the owner of AXT-020; it does not block this layer |
| OD-AGL13 | Can a kept session be reached through a hub of Anima.ing? (SF-NEW-agl01) | A: no; at the machine, or by an SSH of the person's own. B: yes, the hub gets a login, and the client says in words that it is a shell that Anima.ing can use. | A |
| OD-AGL14 | How does a start of a kept session reach the machine? (SF-NEW-agl02) | A: only at the machine (a command of the daemon there); the hub starts nothing. B: through the hub, signed by a key of the client of the operator that the machine checks, for one use, with a log at the machine. C: through the hub with no proof at the machine. | A first; B when the device key exists (task W7). Not C: a person who controls the hub then starts an agent in each repository of each machine whose setting is on |
| OD-AGL15 | Is a unit with no sandbox, beside the daemon, accepted for the kept sessions? (SF-NEW-agl03) | A: yes, with the two requests of AGL-023 and the second read of the review. B: no kept sessions in this layer. | A |
| OD-AGL16 | Is it accepted that each program of the same user, also a coding agent with a shell, can write a false state, set the two settings of the machine, type into a kept session and use the person's login for `anima`? (SF-NEW-agl04, SF-NEW-agl06) | A: yes, and the client says it where the settings are. B: no; wait for a second user or the isolation of task W2. | A for his own machines. B is the measure that would close it, and it is the work of W2 |
| OD-AGL17 | Which environment does an agent in a kept session get? (SF-NEW-agl05) | A: that of the person, with the configuration of the person and of the repository, as when he starts it in a terminal. B: a fixed small environment, and the agent asks for each permission. | A if OD-AGL14 is A or B; with C only B |
| OD-AGL18 | Which identifier does a row have at the hub? (SF-NEW-agl09) | A: a new random one of the daemon, which does not join a row to the usage counts. B: the session identifier that the counts already send. | A |
| OD-AGL19 | What does an alert give a channel of a different party? (SF-NEW-agl12) | A: the fixed words only: no name of the machine, no link. B: also the name of the machine. C: also a link to the list, which holds the address of the hub. | A for ntfy and a webhook; B and C for a channel that the person runs |
| OD-AGL20 | Which gate is before the first code? (SF-NEW-agl17) | A: one gate before task AL3: each finding of the security read is decided. B: a gate only before the kept sessions (AL9). | A. The rows and the alert go to a hub of a different party too |

## 10. What goes to the security review

The review of [routed-command-lines.md](routed-command-lines.md) and this text are one subject: a program on a machine that a hub can cause to start, and data about the work of a person that leaves a device. The second read of that review takes the two texts together. A first separate read of version 1 of this text alone is done: [agent-layer-review-2026-11-20.md](../research/work-route/agent-layer-review-2026-11-20.md), 18 findings (4 High, 9 Medium, 5 Low), SF-NEW-agl01 to SF-NEW-agl18 in [security-findings.md](../security-findings.md). It says that 14 findings of the routed command line review apply here too. Version 2 changed the text for SF-NEW-agl07, 08, 10, 11, 14, 15, 16 and 18, and for the three sentences that the reviewer found false (AGL-010, "with its protections" in AGL-023, "unit files: no change" in Section 8); the correction is the writer's and no reviewer read it. The ten that need the operator: SF-NEW-agl01 (OD-AGL13), 02 (OD-AGL14), 03 (OD-AGL15), 04 and 06 (OD-AGL16), 05 (OD-AGL17), 09 (OD-AGL9, OD-AGL18), 12 (OD-AGL5, OD-AGL19), 13 (OD-AGL3), 17 (OD-AGL20).

To review, at least:

1. The file of signals (AGL-009, AGL-010): each program of the user can write it; what a false state can cause (a false alert, a hidden wait); what a very large file does to the daemon.
2. The hook program reads an object that holds the text of a notification and paths (AGL-008): a proof that nothing of it is written, logged or sent, also on an error.
3. The installer writes the configuration of a different program (AGL-011 to AGL-013): a damaged file, an entry that runs a different program by the search path, a link in place of the file, what a removal leaves.
4. What the rows tell a hub that Anima.ing runs (AGL-015 to AGL-019): the times and the states of the work of a person, the repository names, the session identifier as a join key to the usage counts; if the exclusions hold; the branch name.
5. The alert through a server of a different party (AGL-021).
6. The unit with no sandbox (AGL-023), beside a daemon whose protections were the result of the review AU-5: what the daemon can ask of that unit and nothing more; a second program of the user that asks the same.
7. The fixed list of programs (AGL-024): the search path, a program of the same name in the folder of a repository, the environment that the session gets, the configuration of the agent program that says what it can do with no question.
8. The start by the operator through the hub (AGL-025): a person who controls the hub can start an agent program in a repository of each machine whose setting is on. It runs as the person with the logins of that person.
9. The line to an AI agent (AGL-027): an agent with a shell on the machine can write the signal file and can type `tmux` commands; what that does to "no agent starts a kept session".
10. The terminal of a kept session through the hub (AGL-026): what the rules for an SSH session give and do not give.

## 11. Not verified

- No hook event was recorded. The table of AGL-006 is from the documentation as the writer knows it.
- No `tmux` was started under a unit of launchd or systemd.
- The claims of Section 4 are those of the research list, which ran no tool.
- The page v2 and the daemon at sign-in are on branches that are not merged; this document names them as they were on 2026-11-20.
- The mockup of the group "Waiting for you" does not exist yet.
