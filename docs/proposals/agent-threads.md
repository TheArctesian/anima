---
kind: Proposal
state: Direction decided
evidence: Version 1 (2026-11-20). The direction and the build of stage 1 before the launch are R-NEW-t3native (operator, 2026-11-20, first-hand to the orchestrating session, relayed to the session that records it). The requirements THR-001 to THR-060 are not approved. Only tasks TH1 to TH4, the read side of stage 1, may be built and merged; they start no program and drive no agent. Each task that starts or drives an agent waits for the approval of this proposal and of its security points (Section 8, OD-THR1 to OD-THR22). A separate reviewer read version 1 and found 28 weaknesses, 8 High (Section 9); the text has the corrections that need no decision, and no reviewer read the corrected text.
---

# Proposal: agent threads (the behaviour of T3 Code, as a native part of Anima)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this proposal (PLN-005).

Version: 1. Date: 2026-11-20.
Status: the header of this document holds the state (PLN-020).
Map: [work-plan.md](../work-plan.md), categories Use and See, row 4.24 of [priorities.md](../plan/priorities.md).
From: R-NEW-t3native.
Identifiers: THR-001 to THR-060, OD-THR1 to OD-THR22, tasks TH0 to TH40.
Language: Simplified Technical English (design specification, Section 0).
Mockup: [agent-threads/index.html](../mockup/agent-threads/index.html).
Inventory of T3 Code: [t3-code-inventory.md](../research/agent-threads/t3-code-inventory.md) (a record; it adds no requirement).

## 1. What was decided, and what was not

The operator said on 2026-11-20, first-hand to the orchestrating session: "I am using t3 code more and more and want to completley emulate it as a native feature of the app". Asked when to build it, he selected the option "Build now, first stage before launch". The text of that option: a proposal this week; then the first stage, a thread view over Claude Code and Codex on this Mac, is built before the launch; the security review runs at the same time. Earlier he approved that the Agent usage page should "look and work like T3 Code" (R-224 and the row of agent-usage-v2 in [mockup/README.md](../mockup/README.md)). R-NEW-t3native records these words.

What this decides:

- T3 Code is no longer only a tool of another vendor in a pane (R-196, AXT-020, AXT-045). Anima builds the behaviour of T3 Code itself: projects and threads, the chat thread, the tool calls, the approvals, the plans, the diffs, the worktrees, sessions that stay when the window closes.
- The first stage comes before the launch. Section 6 divides it: stage 1a reads and starts nothing (tasks TH1 to TH4, built on branches); stage 1b drives an agent on this computer and waits for the security points.

What this does not decide: each requirement below, each question of Section 10, and the time of the stages after stage 1. A standing rule of the operator applies: nothing that gives an agent a shell, or that lets a hub start a program on a machine, merges before its security review and his rulings. So no code that starts or drives an agent goes to the merge train before he approves this proposal and Section 8. Such code may be built on a branch that is marked held.

## 2. Terms

| Term | Meaning |
| --- | --- |
| Agent program | Claude Code (`claude`) or Codex (`codex`), installed and signed in by the person on a machine. Later: other programs (Section 3.3) |
| Thread | One conversation with one agent program in one working folder on one device. It is the session of the program (the Claude Code session ID, the Codex thread ID), seen as a timeline. It is not a "session" of Anima: in Anima a "Live session" is a connection to a device (SSH, RDP, VNC) |
| Turn | One prompt of the person and the work of the agent until the agent stops, waits or fails |
| Entry | One step of a turn: a prompt, a reply, a piece of reasoning, a tool call with its result, a plan, an approval, an end of a turn |
| Approval | A question of the agent program before it does a thing (run a command, change a file, read outside the folder). The person allows or declines |
| Read side | The functions that read what an agent program wrote, and start nothing |
| Drive side | The functions that start an agent program, send a prompt, answer an approval or stop a turn |
| Thread runner | The part of `anima-daemon` that starts and drives an agent program on its machine (drive side, Section 4.2) |
| Event log of a thread | The list of the entries of a thread in order, each with a sequence number, from which the client builds the view (Section 4.3) |

## 3. T3 Code: what it is, and what Anima may take from it

### 3.1 The source and the licence

The inventory was read from the public repository `pingdotgg/t3code` at commit `6f5293fbdca476cddbc37466450983ba1dd2ada2` (2026-11-20), by a sub-agent that read the source and the documents and ran nothing. The full record is [t3-code-inventory.md](../research/agent-threads/t3-code-inventory.md).

The licence is MIT: "Copyright (c) 2026 T3 Tools Inc." MIT permits a copy of the code, also in a closed or commercial product, when the copyright notice and the licence text go with each substantial part.

- **THR-001.** Anima must take the behaviour of T3 Code, not its code. A session that builds this function must not copy code of T3 Code into the repository. If a later task wants to copy a part, the task must name the files, keep the MIT notice of T3 Tools Inc. with them, and get a ruling first.
- **THR-002.** Anima must not use the name "T3 Code", its logos and icons (`assets/`), its sounds (CC0, but not needed), or its fonts (Symbols Nerd Font, MesloLGS, KaTeX fonts: each under its own licence). The screen uses the tokens, the icons and the words of Anima. MIT covers copyright, not a trademark.
- The Claude Agent SDK that T3 Code uses is under the licence of Anthropic, not MIT. Section 4.4 does not use it.

### 3.2 How T3 Code works underneath (short)

- A server on the machine (Node, TypeScript, the Effect library) owns the agent processes, the terminals, git and an SQLite database. The web, desktop (Electron) and phone clients connect to it over a WebSocket with JSON remote calls. Execution stays on the machine that holds the folder.
- Codex is driven through `codex app-server` (JSON-RPC on standard input and output): `thread/start`, `thread/resume`, `turn/start`, `turn/steer`, `turn/interrupt`, approvals as requests from Codex (`item/commandExecution/requestApproval`, `item/fileChange/requestApproval`), and the account methods of the app-server for a ChatGPT login.
- Claude Code is driven through the Claude Agent SDK (`query()` with streaming input), pointed at the installed `claude` program. Approvals come through the `canUseTool` callback.
- Each adapter turns the events of its program into one event model (`node.updated`, `message.updated`, `turn_item.updated`, `runtime_request.updated`, `turn.terminal` and others). An orchestrator takes one command at a time and writes the events, the projections, the receipt of the command and the side effects in one SQLite transaction. Clients take a snapshot and then the events after its sequence number, and continue from that number after a reconnect.
- Each thread can have its own git worktree. After each turn a checkpoint is a hidden commit under a private ref; the diff of a turn compares two checkpoints; "Edit from here" restores a checkpoint and cuts the conversation of the program.
- The default permission mode of a new thread is Full access. T3 Code adds no sandbox of its own. The agent process gets the environment of the server.

### 3.3 The inventory, marked for complete emulation

"Must" is the core thread experience, "should" is what a daily user expects, "later" is niche or another program. The column "Anima" says what exists (with the code), what the function can build on, and what is new. Section numbers in the first column are those of the inventory.

| # | Function of T3 Code | Mark | Anima now | Builds on | New |
| --- | --- | --- | --- | --- | --- |
| 1.1 | Projects: add a folder; new project, clone | must; should | Projects come from GitHub (hub `work/projects`, `apps/client/src/lib/projects`); the usage reader names the repository of a folder (`crates/daemon/src/readers/agent_sessions/git.rs`) | The repository name of the folder joins a thread to a project | A local folder that is not on GitHub as a project of a thread |
| 1.3 | Create a thread and send the first prompt | must | Nothing | — | Drive side (stage 1b) |
| 1.4 | A list of threads by project; pinned and settled | must; should | Agent usage lists repositories (`apps/client/src/lib/agent-usage/view/RepositoryList.svelte`) | The list of rows (`apps/client/src/lib/rows`) | The thread list (TH4) |
| 1.5 | States: idle, working, waiting for you, failed | must | AXT-048 names the states (not approved); no code | AXT-046 to AXT-053; the hooks of [agent-layer.md](agent-layer.md) | Read side: "changed in the last 2 minutes" only (THR-013). Real states need the drive side or hooks |
| 1.6 | Pin, snooze, settle, archive, delete | must (archive, delete); should | Nothing | — | Stage 4 |
| 1.7 | Titles: automatic, rename | must; should | Nothing | The title that Claude Code writes (`custom-title`), else the first prompt (TH1) | Rename (stage 4) |
| 1.8 | Fork, side chat, sub-agents | later; should (sub-agents read-only) | The usage reader joins sub-agent files to their session | — | Later |
| 2.1 | One timeline: prompts, replies, grouped tool calls, plans, approvals, diffs, errors | must | Nothing | — | TH1 (entries), TH4 (view) |
| 2.2 | Prompts with chips and history | must; should | Nothing | — | Stage 1b composer |
| 2.3 | Replies, streamed by paragraph | must | Nothing | GraphQL subscriptions (`crates/hub/src/*/graphql.rs`, the "Live" indicator, MOT-011) | Stage 1b: the stream from the runner |
| 2.4 | Reasoning, folded | should | Nothing | — | TH1, TH4 |
| 2.5 | Tool calls by kind: command, edit, read, search, web, MCP, sub-agent | must | Nothing | — | TH1 classifies them (`ToolKind`) |
| 2.6 | Approvals: allow, allow for the session, decline, with feedback | must | MCP approval of a tool call to the person (`crates/anima-cli/src/mcp/approval.rs`, MCP-013) | The pattern "the person approves in the client" | Stage 3 (THR-030 to THR-034) |
| 2.7 | Questions of the agent with choices | must | Nothing | — | Stage 3 |
| 2.8 | Plans and lists of steps; plan mode | must (list); should (plan mode) | Nothing | — | TH1 marks `TodoWrite`, `update_plan`, `ExitPlanMode` as Plan; the plan card is stage 4 |
| 2.9 | The diff of each turn | must | Nothing | The patch of each edit in the file (TH1) | Read side: the patch of each edit. Checkpoint diffs: stage 4 |
| 2.10 | Errors with Retry | must | Nothing | — | Stage 1b |
| 2.11 | Context meter; usage page; limits | should; later | Agent usage (AIS-018 to AIS-024, built) | `fleet/agent_usage` | A link from a thread to its usage (TH4); use by thread needs AIS-018 to change (OD-THR9) |
| 3.1 | One adapter interface and one event model | must | Nothing | `daemon-core` (pure core) | `daemon_core::agent_threads` (TH1) is the start of the event model |
| 3.2 | Permission modes: supervised, accept edits, auto, full access | must (supervised and full) | Nothing | — | Stage 3; the default is OD-THR6 |
| 3.3 | Codex through `codex app-server` | must | BOT-033 permits only `codex exec` and `codex exec resume` (Draft) | — | OD-THR3 |
| 3.4 | Claude Code through the Agent SDK | must | BOT-032 permits only the `claude` program (Draft) | — | Through `claude -p` with stream JSON (Section 4.4); OD-THR2 |
| 3.5 | Other programs (Cursor, OpenCode, Pi, ACP) | later | The catalogue of bots.md | — | Later |
| 3.6 | Turn life: dispatch, queue, steer, interrupt, force stop | must | Nothing | — | Stage 1b, stage 4 |
| 4.1 | Commands, events, projections, idempotent commands | must | Hub domains with `store.rs`; the K10 queue with attempts (W3, partly in `dev`) | The domain pattern of the hub | Section 4.3 |
| 4.2 | SQLite event store with pruning | must | PostgreSQL in the hub | — | Section 4.3, OD-THR4 |
| 4.3 | A stream with a snapshot and a cursor; reconnect | must | GraphQL subscriptions over a WebSocket | Same | THR-022 |
| 5 | Sessions that stay when the window closes; resume after a restart | must | `anima-daemon` runs as a launchd agent or a systemd user unit | The daemon | Stage 2 for the read side, stage 3 for the drive side |
| 6.1 | A git worktree for each thread | should | `git worktree` is not used by Anima | — | Stage 4 |
| 6.2 | Checkpoints as hidden commits under a private ref | must (diff of a turn); should (restore) | Nothing | — | Stage 4 |
| 6.3 | Undo: "Edit from here", revert files | should | Nothing | — | Stage 4 |
| 6.4 | Commit, push, pull request; review page; merge | should; later | Pull requests and issues through `anima` (`crates/anima-cli/src/pull_requests`), Projects | The forge domain of the hub | Stage 4 |
| 7 | A terminal for each thread; project scripts | should | The SSH terminal of the hub (`anima ssh`, the client terminal) | The terminal of Anima | Stage 5 |
| 8 | Keyboard model (send, stop, new thread, previous and next thread, jump to thread N, find, diff) | must; should | The jump field Cmd+K of Anima (R-224) | Same | THR-016 |
| 9 | Settings: default model, mode, program path and environment | must | Settings page | Same | Stage 1b |
| 10 | Model, effort and program pickers; fast mode | must; should | Nothing | — | Stage 1b |
| 11 | Images by paste or drag | must; should (files) | Attachments of the hub (`crates/anima-cli/src/attachments`) | — | Stage 4 |
| 12 | Queued messages, steer, interrupt, stop | must (stop, queue in the client); should | Nothing | — | Stage 1b, stage 4 |
| 13 | Remote: SSH, Tailscale, relay, phone, pairing | later | The hub, the link, the VPN, the phone client (another stream) | All of it | Stage 2 (read) and stage 3 (drive) on other machines |
| 14 | Search, notifications, usage, issues, secrets, sharing | should; later | Notifications, Agent usage, issues | — | Later |
| 15 | Scoped tokens, approvals bound to a live session, the environment of the agent | must | The device token, the login token, API keys (INT-003) | Same | Section 8 |

Not verified (inventory, "Could not verify"): how the screens of T3 Code look and move; how the Agent SDK talks to `claude`; how Codex shows reads and searches; the real speed of steer, interrupt and restart; the phone application. Nobody ran T3 Code for this proposal.

## 4. Architecture

### 4.1 Where the agent program runs

- **THR-010.** An agent program of a thread must run on the device that holds its working folder, as the user of that device, from the install of that user. The hub never runs an agent program, and the client runs one only on its own computer through the thread runner of stage 1b (EXT-009, C3 of [agent-tools-as-extensions.md](agent-tools-as-extensions.md)). This is also the rule of T3 Code: execution stays on the machine that holds the folder.

Reason: the folder, the git credentials and the login of the program are on that device. A copy on the hub breaks BOT-020 (the login of a vendor must not go through the hub).

### 4.2 Who starts and drives the program

Options for the drive side:

- (A) The desktop application starts the program as a child process. Simple, and enough for "this Mac". It stops when the application closes, so a thread does not stay (inventory 5, a must).
- (B) `anima-daemon` starts it, with a new part, the thread runner. The daemon already runs at sign-in as the user (launchd agent, systemd user unit), so a thread stays when the window closes. For this computer the client talks to the daemon over a local socket that only this user can open (as task W0 does for the link). For a different machine the hub passes the command through the work route.
- (C) The daemon starts it inside `tmux`, as the kept sessions of [agent-layer.md](agent-layer.md) (AGL-023). The person can also attach in a terminal. The reviewer of that proposal found that `tmux` gives each program of the user a way to type into the session (SF-117).

Recommendation: B. It gives the "sessions that outlive the window" of T3 Code, and it is the same part for this computer and for the other machines. Stage 1b may start with A behind the same interface if the local socket of B is not ready before the launch (OD-THR1).

- **THR-011.** The client must read the threads of its own computer through the desktop shell (stage 1a) and, from stage 2, through the hub. It must not read the session files of a different computer in any other way.

### 4.3 Who owns the event log

The entries of a thread hold code, file contents, command output and often a secret that the person or a command printed. The hub of the operator is hosted (a container in a pool of Anima.ing). So the place of the event log is a security choice (Section 8, P8).

Options:

- (A) The hub owns the full event log: a domain `agents/threads/` with `threads.rs`, `model.rs`, `log.rs` (the pure rules of sequence and snapshot), `store.rs` (the only SQL of the tables `agent_threads` and `agent_thread_entries`), `graphql.rs` and `tests.rs`. The client gets a snapshot and the entries after a sequence number from the hub. This is the form of T3 Code, with PostgreSQL in place of SQLite. The hosted hub then keeps code and secrets of the operator.
- (B) The device owns the full event log, in a file of the daemon (SQLite or append-only JSON lines in the state folder of the daemon, mode 0600). The hub owns only the row of each thread (device, program, project, title, state, times, counts) and passes the entries of an open thread from the daemon to the client without a copy (as OD-AXT7: "plain text through the hub with a notice, no record and no log"). A thread on an offline device cannot be read.
- (C) As B, and the hub keeps the entries of the last N days encrypted with a key that only the clients of the operator hold (the client-sealed records of the engine, K-E5). Strong, and large.

Recommendation: B for the hosted hub, and A as a setting for a hub on a computer of the operator. B keeps the content where it already is (the session files are on the device today). The hub domain `agents/threads/` exists in both: in B its `store.rs` holds only the rows (OD-THR4).

- **THR-012.** For each thread the hub must keep one row: the device, the program, the external ID, the project (by repository name, AIS-002), the title, the state, the first and the last time, and the counts of AIS-018. Under option B it must not keep a prompt, a reply, a tool input or a tool output. This is a promise of the hub program: a person who controls the hub can change it. So a device gives the entries of its threads to the hub only when a switch at the machine is on (OD-THR19).

### 4.4 The adapters: how Anima drives Claude Code and Codex

T3 Code uses the Claude Agent SDK and the account methods of `codex app-server`. Draft requirements of [bots.md](bots.md) forbid both for an adapter of Anima: BOT-021, BOT-032 ("the adapter must run only the `claude` program (`claude -p` or an interactive session). It must not use the Agent SDK with a subscription") and BOT-033 ("only `codex exec` and `codex exec resume`"). The named exception of R-196 is for the T3 Code catalogue entry only. This proposal does not ask to change them silently.

- **THR-020.** The Claude Code adapter must run the installed `claude` program, as the vendor published it, with `-p`, `--input-format stream-json`, `--output-format stream-json` and `--verbose`, and with `--resume <id>` for an existing thread. It must take approvals through `--permission-prompt-tool` with a tool of the thread runner, so that each approval comes to the runner and then to the person. It must not use the Agent SDK, make or pass a long-lived token (`claude setup-token`), or read the login file (BOT-020, BOT-032). [NEEDS CLARIFICATION: that `--permission-prompt-tool` with stream JSON input gives every approval that the interactive program asks for, and that a turn can be stopped; not verified, task TH10 measures it.]
- **THR-021.** The Codex adapter: OD-THR3. With `codex exec --json` and `codex exec resume` (BOT-033) a turn has no approvals: the sandbox mode is fixed for the turn, and the program asks nothing. With `codex app-server` a turn has approvals, steer and interrupt, as in T3 Code. Recommendation: `codex app-server` with only the thread, turn and approval methods, never an account method (`account/login/start`, `chatgptAuthTokens`, `account/read`), and only on a device where the person signed in to Codex on that device. This needs a ruling that changes BOT-033.
- **THR-022.** Each adapter must turn the events of its program into the entries of `daemon_core::agent_threads` (TH1): prompt, reply, reasoning, tool call with result and patch, plan, approval, question, end of turn (with the reason), error. The client draws only these entries; it never reads a format of a vendor.

### 4.5 How the client gets the stream

- **THR-023.** The hub API must give, in the order of CMD-030: the query `agentThreads(device, program, project, state, first, after)` for the rows; the query `agentThread(id)` for one thread with a snapshot of its entries and the sequence of the last one; the subscription `agentThreadEntries(id, afterSequence)` that gives the entries after that sequence, then each new entry; and, from stage 1b, the mutations of Section 4.7. A client that reconnects asks again with its last sequence. Under OD-THR4 option B the hub asks the daemon for the snapshot and the entries and does not keep them.
- The subscription uses the GraphQL subscription transport that the client already uses for live data, with the "Live" indicator of MOT-011.

### 4.6 How a thread joins a project, a device, a work item, Agent usage and Live sessions

- **THR-024.** A thread must name its device (`DeviceId`). It must name its project when the repository name of its working folder (the rule of AIS-002 and `git.rs` of the usage reader) is the name of a project; else it has no project, and the list shows the folder name.
- **THR-025.** A thread that the hub starts on a different machine (stage 3) is an item of the work route K10, of a new work type `agent_thread`, so that the claim, the lease, the switch at the machine (USE-004), the stop and the report of K10 apply. The turns after the start are not items: they go over the channel of THR-026. [NEEDS CLARIFICATION: whether a new work type or a separate contract; K10 Section 6 has a fixed list of types and Section 3 says that the hub never opens a connection to a machine.]
- **THR-026.** The prompts, the answers to approvals and the stops of a thread on a different machine must reach the daemon only as the answer to a request of the daemon (K10 Section 2: "The daemon asks. The hub never opens a connection to a machine."), by a long poll of the daemon for the commands of its open threads. The daemon checks each command against its own rules (Section 8) before it acts. [NEEDS CLARIFICATION: this text is not yet written against K10 and K8: turns outside items, the audit and the signature of W7 for them, one request slot of a daemon, the end of a poll, task W0, and what a thread does when the hub cannot be reached. Task TH30 owns it.]
- **THR-027.** Agent usage must link to a thread and back: the button "Usage of this thread" opens Agent usage with the repository and the program of the thread chosen. Use by single thread needs AIS-018 to send the session ID of a thread with its counts (OD-THR9).
- **THR-028.** Live sessions stay the connections to a device (NAV-008). A thread that works now is counted on the item Threads of the navigation bar, not in Live sessions.

### 4.7 The `anima` command and the MCP tools

| Function | Command | MCP tool | Stage |
| --- | --- | --- | --- |
| The threads of this computer | `anima agents threads [--program] [--limit]` | `anima_agents_threads` (read) | 1a, built (TH2, TH3) |
| One thread of this computer | `anima agents thread <id>` | None (THR-029) | 1a, built (TH2) |
| The threads of all devices | `anima agents threads --device <name>` | `anima_agents_threads` with `device` | 2 |
| Start a thread, send a prompt | `anima agents send <thread or --folder> <prompt>` | OD-THR8 | 1b, 3 |
| Stop a turn | `anima agents stop <thread>` | `anima_agents_stop` (change) | 1b, 3 |
| Answer an approval | `anima agents approve <thread> <request> allow|decline` | None (THR-031) | 3 |

- **THR-029.** No MCP tool may return the entries of a thread. A prompt or a tool output can hold a secret, and MCP-004 says that no tool returns one. The person reads a thread in the client or with `anima agents thread`. From stage 2 the hub must refuse the entries of a thread to an API key (INT-003) and give them only to a login. On the computer itself this rule cannot hold against a program of the same user: such a program can read the session files and can run `anima agents thread`; the rule keeps the entries out of the result of a tool, which goes to the vendor of the agent.
- **THR-031.** No MCP tool and no API key may answer an approval of a thread. An approval is the decision of a person (the rule of MCP-005 for a trust decision); an agent that approves the command of a different agent defeats the approval. The rule needs a mechanism that an agent of the same user cannot use: the answer is given in a window of the operating system that the desktop shell opens (a native dialog, which a script of the web view and a program without the screen cannot answer), and the runner accepts an answer only from the shell. The command `anima agents approve` exists only if OD-THR18 permits it.

### 4.8 What Anima adds that T3 Code does not have

- One list of the threads of each machine of the operator, in one place, with the device of each (stage 2). T3 Code shows the threads of one server, or of several with a pairing of each.
- The usage beside the thread: Agent usage already counts tokens and the estimate of the cost by program, model, repository and device.
- The other functions of Anima beside the thread: the device, its metrics, its terminal, its containers, its projects and builds.
- The phone later, through the same hub (the phone stream owns the phone client).
- A command and MCP tools for each function (CMD-030).

### 4.9 What stays of R-196 and of the agent tools proposal

- Hermes Agent and Herdr stay tools of other vendors in a pane (AXT-001 to AXT-045 for them, R-196). Nothing of this proposal changes them.
- T3 Code: OD-THR11. Recommendation: keep the T3 Code entry of the catalogue (AXT-020, AXT-045, C2) for a person who wants T3 Code itself, and do not offer it by default once stage 1b ships. Reason: the goal of R-196 was to support the open-source projects and to gain their users; the operator did not take that back.
- The list of agents on all devices (AXT-046 to AXT-053, R-243, R-244): the thread list of stage 2 gives the rows of AXT-046 for Claude Code and Codex. The two lists must be one list (PLN-016). The state rules of AXT-048 (a source that cannot tell a state gives Unknown, never Working) apply to the threads.
- [agent-layer.md](agent-layer.md) said: "It builds no diff screen, no chat view and no terminal host of its own at first" (from R-263). R-NEW-t3native changes that for the chat view and the diff. Its hooks (the state "waiting for you") and its alert stay, and the thread list uses them as a source of state when they exist.

### 4.10 What the planning stream of Anima Product Ideas put on `dev`, and how this proposal stands to it

The operator said on 2026-11-20, first-hand to the orchestrating session: "There are also some proposals for it coming in from grok bot onto dev". `origin/dev` was read at commit `116c7b15` (2026-11-20). At that commit no document on `dev` proposes T3 Code as a native function: a search of `docs/proposals`, `docs/ideas` and `docs/plans` for T3 Code, threads, agent tools, the agent layer and agent runs finds only the documents below. A session that pushes or merges this proposal must read `dev` again first; a later document of that stream on the same subject is merged into one owner (PLN-016), not kept beside this one.

| On `dev` | What it gives | Where it is better than this proposal | Where it clashes | What is left for the operator |
| --- | --- | --- | --- | --- |
| [agent-tools-as-extensions.md](agent-tools-as-extensions.md) Section 6.10, AXT-079 to AXT-088 (R-281): the preview of a dev server and the forward of its port | A list "Servers" of the loopback ports of a device, a preview pane, a forward. AXT-082: the daemon ties a port to an agent session, and the row of that agent in AXT-046 shows the port | It owns the function that the inventory calls the local servers panel and the preview browser (7, marked later). It has a path that exists (SSH `direct-tcpip`), refused ports and an audit event. This proposal writes no preview of its own: a thread shows the servers of AXT-082 and opens them through AXT-083 | None with a ruling. AXT-086 has the switch on by default; the standing rule for a thread runner is off by default (TH33). They are different powers (reach a port; start a program) | Nothing new. The thread view takes AXT-082 when AX43 is built |
| Same document, Section 6.9, AXT-054 to AXT-078 (R-280): the shared project folder | A folder that the hub syncs to each device with Syncthing; MCP over HTTP for the hosted hub | It gives a project the same files on each device, which this proposal does not have: a thread on `pop-os` can work on what the Mac wrote | A git worktree for each thread (stage 4) inside a synced folder: Syncthing would sync `.git` and the worktrees between devices while two agents write. AXT-055 makes the hub copy the copy of record, and OD-THR4 option B keeps thread content off the hosted hub: the code of a project is on the hub in one and not in the other | OD-THR15 |
| Same document, Section 6.8, AXT-046 to AXT-053 (R-243, R-244): one list of agents on all devices, states, one alert | The list, the states, the alert through the needs-you inbox | It is the owner of the list and the states, with rulings | Two lists if both are built (4.9). AXT-049: "The view never shows the content of a pane, a prompt or a transcript". The thread view shows a transcript: for this computer in stage 1a, for other devices in stage 2 | OD-THR16 |
| Same document, Section 3 and goal: "Do not build a second Herdr, a second T3 Code or a second assistant bot in Anima.ing" | The direction of R-196 | — | R-NEW-t3native reverses it for T3 Code. The sentence stays true for Herdr and the bot | The text of that sentence changes when this proposal is approved (Section 11) |
| [agent-layer.md](agent-layer.md) (R-263), on `dev` since 2026-11-20 | Hooks for the state "waiting for you", one alert, kept sessions in `tmux` | The hooks give a state for a session that the person started in a terminal, which the thread runner cannot know | Its kept sessions (AGL-023, `tmux`) and the thread runner (4.2, option B) are two ways to keep an agent alive. Its security read found four High findings on the kept sessions | OD-THR1 covers it: if B is chosen, the kept sessions of AGL-023 are not needed for Claude Code and Codex |
| The pattern of that stream: each new agent function is a section of agent-tools-as-extensions.md under row 4.21 (6.8, 6.9, 6.10) | One owner document and one row | One place to read; fewer cross references | This proposal is its own document and row 4.24, because R-NEW-t3native gives it a different time (before the launch) and a different kind (Anima's own code, not a tool of another vendor), and row 4.21 is tier 3 after row 4.7 | OD-THR17 |

## 5. Requirements of stage 1a (the read side)

- **THR-003.** `daemon_core::agent_threads` must read the lines of one session file of Claude Code (`~/.claude/projects/<project>/<id>.jsonl`) or Codex (`~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`) into a thread: the prompts the person typed, the replies, the reasoning, the tool calls with their results and the patch of an edit, the ends of the turns and the compaction marks, in the order of the file. Lines of a sub-agent and lines that the program wrote for itself are not entries. It is pure; it reads no file.
- **THR-004.** The title of a thread is the title that the person or the program gave (`custom-title`, `thread_name_updated`), else the first line of the first prompt, at most 80 characters.
- **THR-005.** A tool call has a kind (command, edit, read, search, web, agent, plan, question, other) from the name of the tool, and one line that says what it does (the command, the path, the pattern).
- **THR-006.** The text of one entry is held to 64 KiB and the output of one tool call to 16 KiB, cut at a character boundary with a mark; a thread holds at most 20,000 entries and counts the rest. A line that is not JSON changes nothing.
- **THR-007.** `anima agents threads` must list the threads of this computer, the last changed first, from the session files of this user only: only the paths of THR-003, never a settings or login file of the programs. What is checked: each folder from the home folder to `.claude/projects` and `.codex/sessions` is a real folder; a folder or a file below them that is a link is not followed (`O_NOFOLLOW` at the open); a session file belongs to this user and has one name. Not checked: that a folder does not change between the list and the open. A row reads only the start (256 KiB) and the end (64 KiB) of its file. `--program` and `--limit` (1 to 500, default 30) narrow it. It sends nothing to the hub and starts no program.
- **THR-008.** `anima agents thread <id>` must show one thread: as text for a person (control characters made harmless in a terminal), or with `--json` with each entry. An ID has 8 characters at least and no slash; it names a file whose name is the ID or ends with it.
- **THR-009.** The read side respects no exclusion of AIS-008, because nothing leaves the computer. If stage 2 sends rows to the hub, the exclusions of AIS-008 and AIS-023 apply to them.
- **THR-018.** The MCP tool `anima_agents_threads` (tool set `agents`, read) must give no entry (THR-029) and a narrow row: the ID, the program, the title only when the person or the program gave it (never the start of a prompt), the name of the folder and not its path, no branch, the first and the last time. The result of a tool goes to the vendor of the agent that called it. Each text is cleaned and capped (MCP-008). The tool is never a tool of the remote MCP server of a hosted hub (AXT-071).
- **THR-013.** The thread list must group the threads as: changed in the last 2 minutes, today, earlier. A dot in the accent colour marks a file that changed in the last 2 minutes. The screen must not call it "Working": the file cannot tell a state (AXT-048).
- **THR-014.** The thread view must show, in order: the prompts on the trailing side, the replies, the reasoning folded, the tool calls of a turn in one block with one line each (kind, summary, result), the patch of an edit with removed and added lines, the end of each turn with "Stopped" when it was stopped. An open tool call shows its output. Each text of a session is drawn as text, never as markup; a test proves it. Claude Code writes no line for the end of a turn, so its threads show no end of a turn in stage 1a. A row and a thread are what a file says: each program of the user can write such a file.
- **THR-015.** In stage 1a the composer is a note: the thread is read from the file of the program, and to continue it the person uses the program. The screen must start nothing.
- **THR-016.** Keys of the thread screen: Up and Down move in the list and Enter opens; `/` goes to the search of the list; Cmd+Shift+[ and Cmd+Shift+] open the previous and the next thread; Cmd+1 to Cmd+9 open the thread at that place of the list; Cmd+F finds in the thread; `E` opens or closes the tool calls of the thread; `?` shows the keys. Cmd+K stays the jump field of Anima (R-224). A key is not taken while the focus is in a text field, except Escape.
- **THR-017.** The screen is the page Threads in the navigation bar, above Agent usage (OD-THR10), with the mockup [agent-threads/index.html](../mockup/agent-threads/index.html). It is shown only in the desktop application; the phone and the web client do not show it in stage 1.

## 6. Stages and tasks

Each stage is useful alone. A stage that drives an agent waits for the approval of this proposal and of the points of Section 8 that it names. In the column "State", "Built (held)" means a branch that is pushed and not offered to the merge train.

### 6.1 Stage 1a: the threads of this computer, read only (before the launch)

CMD-030 order: there is no hub API in stage 1a, and the plan gives the reason that the order does not apply (`AGENTS.md`, "Command line first"): the function reads files of the computer where the command or the application runs, and sends nothing to the hub. The hub API comes in stage 2.

| Task | Work | Needs | Check | State |
| --- | --- | --- | --- | --- |
| TH0 | This proposal, the ruling, the rows, the mockup, the inventory record, the security read | — | The documents gate | In progress (branch `docs/agent-threads`) |
| TH1 | `daemon_core::agent_threads`: the reader of THR-003 to THR-006 | TH0 merged (PLN-011) | Tests `thr_001` to `thr_006` | Built on `threads/read-side` |
| TH2 | `anima agents threads` and `anima agents thread` (THR-007, THR-008) | TH1 | Tests `thr_007`, `thr_008`, with a home folder of test files and a link that is refused | Built on `threads/read-side` |
| TH3 | The MCP tool `anima_agents_threads` (THR-018, THR-029) | TH2 | The catalogue tests (MCP-009, MCP-011) | Built on `threads/read-side` |
| TH4 | The desktop shell commands `agent_threads_list` and `agent_thread_read` (main web view only, WEB-007), and the page Threads (THR-013 to THR-017) | TH2; the mockup approved | Client tests; HT-NEW-threadview | Built on `threads/read-side`. No person saw it in a window; not compared with the mockup; no separate reviewer read the page |

Needs from W3, W4, W7: none.

### 6.2 Stage 1b: drive Claude Code and Codex on this computer (before the launch if the operator approves Section 8 in time)

| Task | Work | Needs | Check | State |
| --- | --- | --- | --- | --- |
| TH10 | A record in `research/agent-threads/`: run `claude -p` with stream JSON input and output, `--permission-prompt-tool` and `--resume`; and `codex app-server` (thread, turn and approval methods only) and `codex exec --json`. Write which events come, how an approval comes, how a stop works, which process tree stays | — | A record with measured results on this Mac | Not started |
| TH11 | The thread runner in `anima-daemon` (or the shell, OD-THR1): start one program for one thread in its folder, with the environment of THR-040, send a prompt, take the events, write the event log (THR-012, OD-THR4), stop a turn | TH10; OD-THR1 to OD-THR7 decided; Section 8 approved | Tests with a fake program; the process tree is gone after a stop | Not started (held) |
| TH12 | The local socket between the client and the runner on this computer, that only this user can open (as task W0) | TH11 | A test: a different user cannot open it | Not started (held) |
| TH13 | `anima agents send` and `anima agents stop` on this computer; the MCP tool of OD-THR8 | TH11 | Tests | Not started (held) |
| TH14 | The composer: model, effort, mode, send, stop; Enter sends, Shift+Enter makes a new line; the stream by paragraph | TH12 | Client tests; a human check | Not started (held) |
| TH15 | Approvals and questions on this computer (THR-030 to THR-034) | TH11, OD-THR5 | Tests: a reply to a stale request is refused | Not started (held) |

Needs from W3, W4, W7: none on this computer. A human check for each program on a Mac and on `pop-os`.

### 6.3 Stage 2: the threads of the other machines, read only

| Task | Work | Needs | Check | State |
| --- | --- | --- | --- | --- |
| TH20 | Contract change: the rows of THR-012 in the push of K8 (an optional member, as the counts of AIS-019), with the exclusions of AIS-008; a ruling (CON-001) | OD-THR4, OD-THR9 | The contract review | Not started |
| TH21 | The hub domain `agents/threads/`: rows, queries `agentThreads`, `agentThread`, subscription `agentThreadEntries` (THR-023), with the relay of entries under option B | TH20 | Hub tests | Not started |
| TH22 | `anima agents threads --device`, the MCP tool with `device` | TH21 | Tests | Not started |
| TH23 | The page Threads with the choice of a device | TH21 | Client tests, a human check with `pop-os` | Not started |

Needs: W0 (the socket of the link) for a machine that reaches the hub through the link. Not W3, W4 or W7: nothing starts.

### 6.4 Stage 3: drive the agents of the other machines through the hub

| Task | Work | Needs | Check | State |
| --- | --- | --- | --- | --- |
| TH30 | Contract: the work type `agent_thread` of K10 and the long poll of THR-026; a ruling | Section 8 approved; OD-THR12 | The contract review, a separate security read | Not started |
| TH31 | The hub: the queue item, the commands of an open thread, the audit events | TH30; W3 | Hub tests | Not started (held) |
| TH32 | The daemon: claim, check, run, report (W4 for the run, without a container: an agent program needs the files of the person) | TH30; W4; W7 (the key of the device signs each claim, R-186) | Tests of Section 8 | Not started (held) |
| TH33 | The switch "Threads may start here" at the machine, off by default (USE-004 is on by default; this one is not), kept where a program of the user of the agent cannot write it, if such a place exists (OD-THR18) | TH32 | A test: off refuses each item | Not started (held) |

### 6.5 Stage 4: worktrees, checkpoints, diffs, pull requests

TH40 and the next tasks: a worktree for each thread (base branch, name from the first prompt, clean-up off by default), checkpoints as hidden commits under a private ref after each turn, the diff of a turn and of the thread, "Edit from here" with revert of the files only in a worktree, commit, push and a pull request through the forge domain of the hub. Then the rest of the inventory marked should: pin, archive, rename, plan mode, queue and steer, images, a terminal for each thread, find in thread, the context meter. The tasks are written when stage 3 is approved.

## 7. Requirements of the drive side (stages 1b to 4; not approved)

- **THR-030.** An approval must be bound to one request of one live process of one thread: the runner gives each request an ID, keeps the native reference, and accepts one answer only, for that ID, while that process lives. A second answer, an answer to an ID that the runner did not make, or an answer after the process ended is refused and recorded.
- **THR-031.** (Section 4.7.) Only a person answers an approval: in the client, or with `anima agents approve` with a login token. Not an MCP tool, not an API key.
- **THR-032.** The runner must show the person what the request names (the command, the file and the patch, the path outside the folder) from the request of the program, not from text that the hub sends.
- **THR-033.** The mode "Full access" must be a choice of the person for each thread, never the default of a thread that the hub or an agent starts (OD-THR6). The runner must refuse a start from the hub that asks for Full access.
- **THR-034.** With a hosted hub, an approval of a thread on a different machine needs the signature of a key of a client of the operator, so that a hub alone cannot forge an answer. No plan owns such a key today (W7 is the key of a device, which proves the device to the hub, not the hub or the person to the machine). What is signed: the ID of the request, the thread, the device, a hash of the request as the runner made it (the command, the file and the patch), the answer, a value used once and an end time. The runner checks the signature against the request that it holds, not against text of the hub. Until then an approval of a thread on a different machine is answered only on that machine (OD-THR5).
- **THR-040.** The runner must start the program with a fixed list of arguments and no shell, by its absolute path found at the machine, with an environment of a fixed allow list (`HOME`, `USER`, `PATH`, `LANG`, `TERM`, `SHELL`, `TMPDIR`, and the variables that the person names in the settings of the machine), never the daemon token, the link socket, a key of Anima or a variable of the daemon (OD-THR7). `PATH` is the one of the login of the person, read at the machine, never one from the hub. `SSH_AUTH_SOCK` and the variables of a cloud account are passed only when the person names them. A prompt goes to the program on standard input, never as an argument, so that it is not in the process list.
- **THR-041.** The folder of a thread must be a folder that the person chose at the machine, or a worktree that the runner made under a folder of the runner. A folder from the hub must be in the list of folders that the person permitted at the machine.
- **THR-042.** Text from the hub (a prompt) and text from the outside world that the agent reads is not trusted. The runner marks nothing as an instruction; the client shows where a prompt came from (this computer, the hub, a schedule, an agent) on the prompt.
- **THR-043.** The event log keeps no value that the runner knows is a secret (the variables of THR-040 by name) and applies the redaction crate of E0-3 to tool outputs before a copy leaves the device. The redaction finds known forms only; it is not a guard. The event log of the runner is a second copy beside the session file of the program: it has a size limit, a time after which it is deleted (OD-THR22), and it is deleted with its thread.
- **THR-044.** Each start, prompt, approval answer and stop is an audit event on the hub, with the actor (person, API key, agent) and the device, without the text. The runner also writes each of them to a log at the machine that the hub cannot change, with what it could itself see of the source (the local socket, or the hub). The label of the source on a prompt (THR-042) for a prompt from the hub is what the hub says.
- **THR-045.** On a device with more than one user, the runner of a user starts programs only as that user and reads only the session files of that user; the socket and the event log are mode 0600 in a folder of that user.

- **THR-046.** The drive side fails closed. An event, a kind of request or a version of the program that the runner does not know stops the turn and is shown; an unknown or missing answer to an approval is a decline. The runner sets the permission mode and the policy of the program itself at each start, and does not call a thread Supervised when it cannot prove that the settings of the project or of the user do not answer before it (task TH10 measures this; OD-THR21).
- **THR-047.** The runner resumes only a thread that it started, by its own record. A session that the person started in a terminal is read only, until the person takes it over with a confirmation that names the file.
- **THR-048.** A script of the web view alone must not be able to choose the folder of a new thread, to choose Full access or to answer an approval: each of the three needs a window of the operating system from the shell.
- **THR-049.** A thread has limits: the most threads that run on a device, an idle time after which a program with no turn and no open approval is stopped, and a stop that ends the whole process group of the program. After a restart of the daemon no program of a thread runs: the threads are shown as stopped, and nothing starts again without the person. One `claude` task for each login at a time stays the default (BOT-032).
- **THR-050.** The `git` of the runner (worktrees, checkpoints, revert) runs with the hooks off and with no configuration of the repository that names a program (`core.hooksPath`, `core.fsmonitor`, filters, `core.sshCommand`). A checkpoint does not take a file that git ignores. The name of a worktree is made by the runner from a fixed alphabet, never from the text of a prompt.
- **THR-051.** The adapters drop each member of a stream of a program that is about the account or its limits (AIS-022); the event log and the hub get none.

## 8. Security: each way this gives power over a machine

The form is that of the earlier reviews (the work route, the container work types): the power, who gets it, the guard, and what is accepted. "Stage" says when the power first exists. Nothing of stage 1a starts a program, so P1 to P7 and P10 do not exist in stage 1a.

| # | Power | Who could use it | Guard | Accepted, or open | Stage |
| --- | --- | --- | --- | --- | --- |
| P1 | The hub starts a process on a machine (an agent program) | A person or a program that controls the hosted hub; a thief of the login token or of an API key | Only through an item of K10 that the daemon claims (THR-025); the switch at the machine, off by default (TH33); the daemon checks the program, the folder (THR-041) and the arguments itself; W7 signs each claim; audit (THR-044) | Open. W7 proves the device to the hub. It does not prove the hub or the person to the machine: with W7 a person who controls the hub can still start a thread in a permitted folder and name its prompt. An agent program is not a job in a container: it has the files and the logins of the person. Only a signature of a client of the operator on each start and each prompt, or a rule that such threads are started at the machine only, closes it (OD-THR20) | 3 |
| P2 | An agent runs a shell command | The agent program, steered by a prompt or by text it reads | The approvals of the program in the modes Supervised and Accept edits; the sandbox of Codex; the person chooses Full access for each thread (THR-033) | Open, not accepted: the approvals hold only if the settings of the project and of the user do not answer first (THR-046, OD-THR21; not verified), and Codex has a sandbox, Claude Code has none. Full access by default (as T3 Code) is OD-THR6 | 1b |
| P3 | A prompt from the hub | A hub controller, an API key, an MCP tool of a different agent | Only the operator's login or an API key with the scope for threads sends a prompt; the prompt shows its source (THR-042); audit; no prompt of the hub to a machine before OD-THR20 is decided | Open: the source label is what the hub says (THR-044). OD-THR8 (may an agent send a prompt through MCP), OD-THR20 | 1b local, 3 remote |
| P4 | Text that an agent reads steers it (a file, an issue, a web page, a tool output) | Each author of such text | The approvals of the program; no MCP tool returns the entries (THR-029), so one agent does not read the thread of a different agent through Anima; Full access is a choice | Accepted as a property of the agent programs; the person is told on the mode picker | 1b |
| P5 | The hub forges an approval | A hub controller | THR-030 binds an answer to a live request; THR-034: a remote answer needs a key of a client; until W7 a remote thread is answered only at the machine | Open: OD-THR5 | 3 |
| P6 | Secrets in the environment of the agent | The agent program and each command it runs; a prompt that asks it to print them | THR-040: a fixed allow list; never the daemon token, the link socket or a key of Anima | Accepted: the agent gets the variables that the person names; the program itself reads its own login | 1b |
| P7 | The login of the vendor program | The hub, the relay | The adapters never read, copy or pass a login (BOT-020); `codex app-server` without account methods (THR-021); Claude through `claude -p`, not the SDK (THR-020) | Open: OD-THR2, OD-THR3 (changes of BOT-032 and BOT-033) | 1b |
| P8 | The event log holds code and secrets | The hosted hub, its backups, its logs; a reader of the database | Option B of 4.3: the content stays on the device, the hub relays an open thread without a copy; the redaction of THR-043 before a copy leaves; no entry in a log (LOG-007) | Open: OD-THR4 | 2 |
| P9 | The read side shows the session files | Each person who can open the desktop application of the user; a different program of the same user | The shell commands answer the main web view only (WEB-007); they read only the paths of THR-007, never a link; nothing leaves the computer | Open: the window adds one reader, the main web view. It draws text that is not trusted, and its content policy permits a connection to `https:` and `wss:` addresses, so a fault that lets such text run as script could send each thread out. Guards now: each text is drawn as text (a test), a thread has a size limit, errors carry no path. From stage 1b: THR-048 | 1a |
| P10 | A second user of a machine | A different account on the same computer | THR-045: the runner of a user, the files, the socket and the log in a folder of that user, mode 0600 | Open: the runner as a system service is not permitted (as AIS-024). A different user is kept out by the modes; a program of the same user is not (P14) | 1b |
| P11 | An MCP tool lists the threads to an agent | Each agent that has the `anima` MCP server | The tool gives rows, not entries (THR-029); titles are cleaned and capped (MCP-008); the tool set can be set off (MCP-002) | Corrected in the code: the tool gives a title only when the person or the program gave one, the name of the folder, no path and no branch (THR-018). Open: a title that the person gave, and a folder name, can still name a customer; the exclusions of AIS-008 are not applied. OD-THR13 | 1a |
| P12 | A worktree, a checkpoint or a revert changes the files of the person | The runner, by a command of the hub or of the person | Revert of files only in a worktree of the runner (as T3 Code); a check that no other thread uses the folder; never `git clean` outside a worktree | Open until stage 4 | 4 |
| P13 | A thread stays when the window closes | Whoever reaches the runner later | The runner stops a program that has no turn and no open approval after an idle time; the socket of THR-045 | Open: THR-049 gives the limits; they are not measured | 1b |

| P14 | A program of the same user (an agent that the person runs, steered by text it read) uses the powers of the person in Anima | Each agent on the machine | Today such a program can already read the session files, the login token of the command line in the key store and the daemon token. The drive side adds: the socket of the runner, the settings of the runner, `anima agents send`. Guards asked for: THR-031 and THR-048 (a window of the operating system for an approval, a folder and Full access); OD-THR18 | Open. No mechanism of Anima separates a person from a program of the same user today | 1b |
| P15 | A forged session file | Each program of the user | A row is a claim of a file (THR-014); the runner resumes only its own threads (THR-047) | Accepted for the read side | 1a |

### 8.1 What the separate review found

Section 9 holds the findings of the separate reviewer and the change of the text for each one that needs no decision.

## 9. Reviews

The session that wrote this proposal gave a separate reviewer agent this document, the documents it stands on and the positions of an attacker to take, and not its own view. The report is the record [agent-threads-review-2026-11-20.md](../research/agent-threads/agent-threads-review-2026-11-20.md). Its findings F1 to F28 are the entries of [security-findings.md](../security-findings.md) in the table below.

The reviewer found 28 weaknesses: 8 of severity High, 13 Medium and 7 Low. Seven of the eight High findings are about the drive side (stages 1b to 3), of which no code exists; one (F8) and six of the lower ones are about the read side, and those that are a fault of the code are corrected on the branch `threads/read-side`. The largest: nothing in Anima separates the person from a program of the same user, so "only a person answers an approval" had no mechanism (F1, F5); the relay "without a copy" and the key of W7 do not protect a machine against a person who controls the hub (F3, F4, F6); and the approvals of the programs may be answered by settings of a project before Anima is asked (F2, not verified).

| # | Severity | Finding | Entry | What was done |
| --- | --- | --- | --- | --- |
| F1 | High | "Only a person answers an approval" has no mechanism: an agent of the same user reaches the socket, the command and the login token | SF-NEW-thr01 | OD-THR18. THR-031 now names the mechanism that is asked for |
| F2 | High | Settings of the project or the user of the agent program (allow rules, hooks, MCP servers, the policy of Codex) can answer before the approval tool is asked (not verified) | SF-NEW-thr02 | TH10 measures it; THR-046; OD-THR21 |
| F3 | High | "Relay without a copy" is a promise of the hub: a hub controller can pull each thread of each device that is online | SF-NEW-thr03 | THR-012 corrected; OD-THR19 (a switch at the machine) |
| F4 | High | W7 proves the device to the hub, not the hub to the machine; a prompt of the hub is a command that the hub names, with no isolation | SF-NEW-thr04 | P1 and P3 corrected; OD-THR20 |
| F5 | High | The daemon token and the login token of the command line are in reach of an agent of the same user; in stage 3 one poisoned file reaches each machine | SF-NEW-thr05 | P10 row P14 added; OD-THR20 |
| F6 | High | THR-034 names a key of a client that no plan owns, and does not say what is signed | SF-NEW-thr06 | THR-034 now says what is signed; the key is OD-THR5 |
| F7 | High | THR-029 bans only MCP tools; an API key and the `anima` command can read entries | SF-NEW-thr07 | THR-029 widened |
| F8 | High | The main web view is a new reader with open `https:` egress, and later would send prompts and answer approvals; P9 was too kind | SF-NEW-thr08 | P9 corrected; THR-014 (text only, with a test); THR-048 |
| F9 | Medium | The switch, the folder list and the mode can be written by the user of the agent; TH33 misread USE-004; nothing stops the hub from sending Full access | SF-NEW-thr09 | TH33 and THR-033 corrected; OD-THR18 |
| F10 | Medium | The recommendation of OD-THR8 conflicts with MCP-006 and AGL-003; change tools are `allow` by default | SF-NEW-thr10 | OD-THR8 recommendation changed to A |
| F11 | Medium | The built MCP tool gave the full title, the path of the folder and the branch, with no exclusion | SF-NEW-thr11 | Corrected in the code (the tool gives a title only when given, the folder name, no branch); THR-018; OD-THR13 |
| F12 | Medium | Nobody is named as the holder of the setting of option A of the event log | SF-NEW-thr12 | OD-THR4 corrected: the setting is in the configuration file of a hub on a computer of the operator, never a setting that a hosted hub holds |
| F13 | Medium | The long poll of THR-026 is not written against K10 and K8 | SF-NEW-thr13 | THR-026 marked; TH30 owns it |
| F14 | Medium | The idle stop and the life of a thread have no limits, no containment of child processes, no rule for a restart of the daemon | SF-NEW-thr14 | THR-049 |
| F15 | Medium | The `git` of the runner runs hooks and configuration that the agent can write; checkpoints take untracked secrets; worktree names come from the prompt | SF-NEW-thr15 | THR-050 |
| F16 | Medium | The label of the source of a prompt and the audit rest on the hub; no log at the machine | SF-NEW-thr16 | THR-044 corrected |
| F17 | Medium | The event log is a second copy with no retention, deletion or limit; redaction is best effort | SF-NEW-thr17 | THR-043 corrected; OD-THR22 |
| F18 | Medium | Wider conflicts with BOT-020, BOT-032, BOT-033 and AXT-045: a subscription login driven through a hosted hub | SF-NEW-thr18 | Section 11; OD-THR2 and OD-THR3 now say it |
| F19 | Medium | Gaps of THR-040: no deny list, the source of `PATH`, `SSH_AUTH_SOCK`, a prompt in the process list | SF-NEW-thr19 | THR-040 corrected |
| F20 | Medium | A forged session file shows as a thread, and a resume would load its history; the ID of a Codex row came from the content | SF-NEW-thr20 | Corrected in the code (the ID of a row is the ID that opens it); THR-047 |
| F21 | Medium | No rule to fail closed for an unknown event, kind of request or version of the program | SF-NEW-thr21 | THR-046 |
| F22 | Low | Limits missing in the code: the summary title, the patch, the whole thread, the line of a glance | SF-NEW-thr22 | Corrected in the code (THR-004, THR-006) |
| F23 | Low | The file reader followed a link at the root folders and checked no owner and no hard link | SF-NEW-thr23 | Corrected in the code for the root folders, the owner and the link count (THR-007). Open: the open is by path, so a folder can change between the list and the open |
| F24 | Low | The reader can show what did not happen: the output wrapper of Codex, the rule for typed text, no end of a turn for Claude Code | SF-NEW-thr24 | The wrapper is corrected in the code. THR-014 says what Claude Code does not give. Open: a line that the program wrote without a tag shows as a prompt |
| F25 | Low | TH4 was marked built, but the page was not on the branch at the time of the read | SF-NEW-thr25 | The page is on the branch now; no separate reviewer read it (Section 12) |
| F26 | Low | Rows of Section 8 that were wrong or too kind (P2, P3, P10, P13), and rows that were missing | SF-NEW-thr26 | Section 8 corrected |
| F27 | Low | Open decisions and conflicts that were missing | SF-NEW-thr27 | OD-THR18 to OD-THR22; Section 11 |
| F28 | Low | Members about the account and its limits in the streams of the programs conflict with AIS-022 (not verified) | SF-NEW-thr28 | THR-051 |

What the reviewer checked and found sound in stage 1a: no call to the hub and no start of a process; the cleaning of text for a terminal in the text, table and JSON forms; the handling of a thread ID; `O_NOFOLLOW` at the last part of a path; the MCP tool gives no entry; the shell commands answer the main web view only and give no path and no text in an error.

No separate reviewer read the corrections, the page Threads of the branch, or Section 4.10.


## 10. Open decisions

Each row is one question for the operator with the options, the strength and the weakness, and a recommendation.

| ID | Question | Options | Recommendation |
| --- | --- | --- | --- |
| OD-THR1 | Who starts the agent program on this computer in stage 1b? | (A) The desktop application, as a child. Fast to build; a thread stops when the window closes. (B) The thread runner in `anima-daemon`. Threads stay; needs the local socket (TH12) and more time. | B, with A behind the same interface only if B is not ready before the launch |
| OD-THR2 | Do you permit the Claude Code adapter to drive `claude -p` with stream JSON in and out and `--permission-prompt-tool`, with the login of the machine? | (A) Yes: it is the `claude` program, which BOT-032 permits; no SDK. (B) No: only an API key for Claude. | A. BOT-032 already names `claude -p`; this adds the input format and the approval tool. Know that from stage 3 this is a subscription login driven by a prompt that came through a hosted hub; BOT-020 forbids that the model traffic goes through the hub, and it does not: only the prompt does. The vendor terms row (BOT-038) is needed before stage 3 |
| OD-THR3 | Codex: `codex exec` only (BOT-033), or `codex app-server` without account methods? | (A) `exec` and `exec resume`: inside BOT-033; no approvals, no steer, a stop kills the process. (B) `app-server` with the thread, turn and approval methods only, on a device where the person signed in: the behaviour of T3 Code; changes BOT-033. | B, with a ruling that changes BOT-033 for the thread, turn and approval methods, and keeps the ban on the account methods |
| OD-THR4 | Where is the event log of a thread? | (A) The hub keeps all entries (PostgreSQL). Offline devices can be read; the hosted hub holds code and secrets. (B) The device keeps the entries; the hub keeps the rows and relays an open thread without a copy. (C) As B, with encrypted copies on the hub (K-E5). | B on the hosted hub; A only for a hub on your own computer, set in the configuration file of that hub at that computer, never a setting that a hosted hub holds |
| OD-THR5 | How is an approval of a thread on a different machine answered before task W7? | (A) Through the hub, with the login token. A hub controller can approve. (B) Only at the machine, until W7 gives a key of the client. (C) No remote threads before W7. | B |
| OD-THR6 | What is the default mode of a new thread? | (A) Full access, as T3 Code. (B) Supervised for a thread that the hub or an agent starts, and the last choice of the person for a thread that the person starts on this computer. | B |
| OD-THR7 | What does the environment of the agent program hold? | (A) The environment of the daemon, as T3 Code. (B) A fixed allow list and the variables that the person names at the machine (THR-040). | B |
| OD-THR8 | May an AI agent start a thread or send a prompt through MCP? | (A) No. (B) Yes, as a change tool in state `ask` (MCP-013). (C) Yes, routed and automatic, as R-243 permits for agent runs. | A. MCP-006 and AGL-003 say that no tool and no agent starts or types into a session, and a change tool is `allow` by default (MCP-002), so B would be automatic for most people. Ask again after stage 1b is in use |
| OD-THR9 | May the daemon send the session ID with the counts of AIS-018, so that Agent usage shows the use of one thread? | (A) No: no use by thread. (B) Yes: the ID only, no text. | B |
| OD-THR10 | Where is the page Threads? | (A) An item Threads in the navigation bar, above Agent usage. (B) A tab of the page Agent usage. (C) In Projects, on the page of a project. | A |
| OD-THR11 | What happens to the T3 Code entry of the catalogue of agent tools (AXT-020, AXT-045)? | (A) Keep it for a person who wants T3 Code itself; not offered by default after stage 1b. (B) Withdraw it. | A |
| OD-THR12 | Does stage 3 (drive the agents of other machines) wait for task W7? | (A) Yes. (B) No, with the guards of P1 and OD-THR5 (B) only. | A, the same condition as R-186 |
| OD-THR13 | May the title of a thread (the first line of a prompt) leave the computer in stage 2 and go to an MCP tool? | (A) Yes, cleaned and capped. (B) Only a title that the person or the program gave; else the folder name and the time. | B for the hub and the MCP tool; the full title stays in the client of the same computer |
| OD-THR15 | A thread in a folder that the shared project folder syncs (AXT-054 to AXT-078): where do its worktrees go? | (A) Worktrees of a thread are made outside each synced folder, under a folder of the runner, and `.git` of a synced project is not synced. (B) No worktree for a thread in a synced folder: such a thread works in the folder. (C) Decide when stage 4 is planned. | A, written into AXT-056 when stage 4 is planned; until then C costs nothing |
| OD-THR16 | AXT-049 says that the list of agents never shows a transcript. May the thread view show the entries of a thread of a different device (stage 2)? | (A) Yes, to the signed-in operator only, relayed and not kept (OD-THR4 B). (B) No: other devices show rows only; a thread is read on its own computer. | A, with AXT-049 changed to say that the list shows none and the thread view does |
| OD-THR17 | Is the owner of this function its own document and row 4.24, or a section of agent-tools-as-extensions.md under row 4.21, as the other new agent functions? | (A) Its own document and row. (B) A section 6.11 of that document. | A: a different time, a different kind of work, and the goal sentence of that document says the opposite |
| OD-THR18 | A program of the same user can do what the person can do in Anima (read the login token, reach a local socket, write a settings file). How is "a person" proven for an approval, for Full access and for the switches of the runner? | (A) A window of the operating system from the desktop shell for each of them; no `anima agents approve` command. An agent that can drive the screen is not stopped. (B) As A, and the settings of the runner in a file that only an administrator can write. (C) Accept that an agent of the same user can approve; say so on the screen. | A for stage 1b; B before stage 3 |
| OD-THR19 | May a device give the entries of its threads to the hub (stage 2)? | (A) Only when a switch at the machine is on; off by default; the hub cannot change it. (B) Always, for a signed-in operator. (C) Never: other devices show rows only (as OD-THR16 B). | A |
| OD-THR20 | Stage 3: a person who controls the hosted hub could start a thread and name its prompt on a machine, also with W7. What do you require? | (A) Each start and each prompt for a different machine carries a signature of a key of one of your clients, which the runner checks; needs a plan for such keys. (B) Accept the hub as trusted for this, with the switch at the machine and the folder list. (C) No stage 3 on a hosted hub; only a hub on your own computer. | A; until the keys exist, C |
| OD-THR21 | The settings of a project folder of an agent program (allow rules, hooks, MCP servers) may run things with no approval. Does the runner honour them? | (A) No: the runner starts the program with the settings of the project off, where the program permits it (task TH10). (B) Yes, as the program does in a terminal, with a notice on the thread. | A for a thread that the hub or an agent starts; B with the notice for a thread that you start in your own folder |
| OD-THR22 | How long does the runner keep the event log of a thread at the machine? | (A) Until the thread is deleted. (B) 30 days after the last entry. (C) Not at all: the view is made from the session file of the program each time. | C where the session file gives enough; else B |
| OD-THR14 | Do you approve the mockup [agent-threads/index.html](../mockup/agent-threads/index.html), frames 1 and 2, for stage 1a? | (A) Yes. (B) Changes. | A |

## 11. Changes that this proposal asks of other documents

Not made in this version; each needs the ruling named.

| Document | Change | Ruling |
| --- | --- | --- |
| [bots.md](bots.md) BOT-032 | Add: with `-p`, the input and output format stream JSON and `--permission-prompt-tool` | OD-THR2 |
| bots.md BOT-033 | Add: `codex app-server` with the thread, turn and approval methods only; the account methods stay forbidden | OD-THR3 |
| [agent-tools-as-extensions.md](agent-tools-as-extensions.md) AXT-020, AXT-045 | The T3 Code entry: as OD-THR11 | OD-THR11 |
| agent-tools-as-extensions.md AXT-046 | One list: the thread list gives the rows of Claude Code and Codex | Approval of THR-024 |
| [agent-layer.md](agent-layer.md) Section 1 | "no chat view" is changed by R-NEW-t3native | Made by R-NEW-t3native |
| [extensions-plan.md](../plans/extensions-plan.md) AIS-018 | The session ID with the counts; in stage 2 a title that was given and a folder name leave the device, which AIS-018 forbids today | OD-THR9, OD-THR13 |
| [cli-mcp-plan.md](../plans/cli-mcp-plan.md) MCP-006, agent-layer.md AGL-003 | A tool that sends a prompt, if OD-THR8 permits one | OD-THR8 |
| bots.md BOT-020, BOT-038 | A prompt that came through a hosted hub to a program with a subscription login; the vendor terms row | OD-THR2, OD-THR3 |
| agent-tools-as-extensions.md Section 3, AXT-049 | "Do not build a second ... T3 Code" names Herdr and the bot only; AXT-049 as OD-THR16 | Approval of this proposal; OD-THR16 |
| [contracts/work-route.md](../../contracts/work-route.md) Section 6 | The work type `agent_thread` and the long poll of THR-026 | TH30 |
| [design-spec.md](../spec/design-spec.md) NAV-001 | The item Threads | OD-THR10 |

## 12. What could not be verified

- That `claude -p` with stream JSON input gives each approval through `--permission-prompt-tool`, and that a turn can be stopped without the loss of the session (task TH10).
- The events of `codex app-server` on this Mac; the version of Codex that has them (task TH10).
- The format of the session files: read from the files of this Mac on 2026-11-20 (Claude Code and Codex of that day). The vendors change the format without notice; the reader ignores what it does not know (THR-006). Old Codex files without `event_msg` lines give no prompts in stage 1a.
- The screens of T3 Code: nobody ran it. The layout ideas come from its source and documents.
- How T3 Code shows a read and a search of Codex (inventory).

## 13. History

- 2026-11-20, version 1, third part. The separate security read (28 findings) is recorded in Section 9; the text and the code of the read side have its corrections; OD-THR18 to OD-THR22 are new, and the recommendation of OD-THR8 changed.
- 2026-11-20, version 1, second part. `origin/dev` at `116c7b15` was read for the documents of the other planning stream (Section 4.10); OD-THR15 to OD-THR17 are new.
- 2026-11-20, version 1. Written from R-NEW-t3native, the inventory of T3 Code at commit `6f5293fb`, and the code of `dev` at `4ffc36a5`. Tasks TH1 to TH4 are built on the branch `threads/read-side`. A separate reviewer read version 1 (Section 9).
