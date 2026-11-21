---
kind: Proposal
state: Direction decided
evidence: Version 3 (2026-11-20). The operator answered 36 open decisions on question cards of the orchestrating session (R-296, R-290, R-292, R-294; relayed), after the direction of R-286 and the rulings of the same day that the document names. With them no decision of his holds stage 1b (drive Claude Code and Codex on this computer); his standing rule stays, so code that starts an agent merges only after a separate security review of that code. The requirements THR-001 to THR-130 are not approved as a whole: a requirement is in force where a ruling names its decision. Parity with the inventory of T3 Code: 101 functions; 15 built on a branch, 3 partly built, 77 not started, 5 owned by a different document, 1 not copied. Built and pushed, not merged: tasks TH1 to TH5 and TH8 (they read and start no program). Three separate security reads are recorded (28, 19 and 24 findings; Section 9); the list for the review of the code of stage 1b is Section 6.2.1. Open: OD-THR38 to OD-THR44. Risks that he accepted are in Section 8.2 with his words.
---

# Proposal: agent threads (the behaviour of T3 Code, as a native part of Anima)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this proposal (PLN-005).

Version: 3. Date: 2026-11-20.
Status: the header of this document holds the state (PLN-020).
Map: [work-plan.md](../work-plan.md), categories Use and See, row 4.24 of [priorities.md](../plan/priorities.md).
From: R-286, R-287, R-288, R-293, R-289, R-297, R-295, R-291.
Identifiers: THR-001 to THR-130, OD-THR1 to OD-THR44, tasks TH0 to TH96.
Language: Simplified Technical English (design specification, Section 0).
Mockup: [agent-threads/index.html](../mockup/agent-threads/index.html).
Inventory of T3 Code: [t3-code-inventory.md](../research/agent-threads/t3-code-inventory.md) (a record; it adds no requirement).

## 1. What was decided, and what was not

The operator said on 2026-11-20, first-hand to the orchestrating session: "I am using t3 code more and more and want to completley emulate it as a native feature of the app". Asked when to build it, he selected the option "Build now, first stage before launch". The text of that option: a proposal this week; then the first stage, a thread view over Claude Code and Codex on this Mac, is built before the launch; the security review runs at the same time. Earlier he approved that the Agent usage page should "look and work like T3 Code" (R-224 and the row of agent-usage-v2 in [mockup/README.md](../mockup/README.md)). The same day he said of T3 Code: "it is really good and i want it in my app should copy every feature they have". R-286 records these words. More answers followed on 2026-11-20 (orchestration and sub agents: R-297; servers inside the subscription: R-295; the reach of an agent: R-289; functions of other tools and of the ideas: R-291). Two answers on the place of the page: Threads is its own item of the navigation bar (R-287), and on the phone it takes the place of Sessions in the bar (R-288).

What this decides:

- T3 Code is no longer only a tool of another vendor in a pane (R-196, AXT-020, AXT-045). Anima builds the behaviour of T3 Code itself: projects and threads, the chat thread, the tool calls, the approvals, the plans, the diffs, the worktrees, sessions that stay when the window closes.
- The first stage comes before the launch. Section 6 divides it: stage 1a reads and starts nothing (tasks TH1 to TH4, built on branches); stage 1b drives an agent on this computer and waits for the security points.

What this does not decide: each requirement below, each question of Section 10, and the time of the stages after stage 1. A standing rule of the operator applies: nothing that gives an agent a shell, or that lets a hub start a program on a machine, merges before its security review and his rulings. So no code that starts or drives an agent goes to the merge train before he approves this proposal and Section 8. Such code may be built on a branch that is marked held.

### 1.1 Parity with T3 Code, as a count

The table of Section 6.9 has one row for each function of the inventory, with its stage, its task and its state.

| | All functions | Marked must |
| --- | --- | --- |
| Built on a branch (read side; not merged) | 15 | 9 |
| Partly built | 3 | 2 |
| Held (built, waits for the approval) | 0 | 0 |
| Not started | 77 | 25 |
| Owned by a different document of Anima | 5 | 0 |
| Not copied (the telemetry of T3 Code; export, which T3 Code does not have) | 1 | 0 |
| Total | 101 | 36 |

What is built is a viewer of the sessions of this computer. Of the main loop of T3 Code (send a prompt, see the stream, approve, see the diff of the turn, commit) no step is built: each needs the drive side, which waits for the approval of Section 8.

### 1.2 The principle: reach by default (R-289)

The operator, 2026-11-20: "maximum agent reach is a good thing". And, of a question before a server is started: "can change it in the setting to ask for permission but it's more so about staying within your subscription limits".

- **THR-065.** An agent of a thread gets broad power by default: the mode Full access, the environment of the login of the person, sub threads, other machines, other models, and machines inside the allowance. A limit exists only where it protects the operator. The guards are on who may command an agent, not on what the agent may do. After the answers of version 3 (Section 1.3) the parties that must not command an agent are: a different user of the machine, a web page, and a party on the network that is not the hub. The hub is trusted, with the switch at each machine and its list of folders as the guards; a program of the same user counts as the person. The hard stops are few: the allowance of a plan, and an action of Anima that cannot be undone (THR-109).

What this does not change: the standing rule of the operator, that nothing that gives an agent a shell, or that lets a hub start a program on a machine, merges before its security review and his rulings. The review of Section 9 reads the new parts with this principle: it looks for the ways by which a different party could use the reach of the agent, and it does not argue for less reach.

The principle turns the recommendation of four open decisions, which the operator has not answered: OD-THR6, OD-THR7, OD-THR8 and OD-THR21 (Section 10). It also changes what THR-033, THR-040 and THR-046 say; each of them now names its open decision.

### 1.3 The answers of the operator (version 3)

On 2026-11-20 the operator answered 36 of the open decisions on question cards of the orchestrating session. Section 10 has each answer in its row, with his words where he wrote his own. Four rulings record them: R-296 (the shape), R-290 (how Anima drives the programs), R-292 (the hub and the person), R-294 (machines for an agent). Seven answers are not the recommendation of this document, or go against a finding of a security read. Each is written here with its consequence, and none is argued away:

| Decision | His answer | Consequence |
| --- | --- | --- |
| OD-THR3 | "No we can use account methods everyone else is doing it" | `codex app-server` is used with its account methods; the ban of BOT-033 is lifted. Anima can then read which account and plan Codex uses and its limits, and can start the sign-in of Codex from the application (THR-115). bots.md recorded that OpenAI does not permit these methods for a commercial or hosted service; that risk with the vendor is his to take, and he took it. AIS-022 and CAP-025 (the daemon reads no login, no account name and no limit) do not agree with this any more for Codex: Section 11 |
| OD-THR4 | "On the hub", then "Yes, plain on the hub" | The prompts, the code and each secret that an agent prints are kept on the hosted hub in plain form. Whoever controls that server, its database or its copies can read them. THR-012 is changed |
| OD-THR12, OD-THR20, OD-THR35, OD-THR5 | "No, with the guards only"; "Trust the hub, keep the guards"; "Through the hub" | A party that controls the hosted hub can start an agent with full access on a machine where the switch is on, name its prompt, and answer its approvals. The guards that are left are at the machine: the switch "Threads may start here" (off by default) and the list of folders. THR-034 and THR-105 are withdrawn |
| OD-THR18 | "Accept it across any device. If my other desicons are contridicarty to this I want them changed." | A program of the same user, also an agent that was steered by text it read, can do what the person can do in Anima: answer an approval, choose a folder, start a thread, change a setting of the runner. No proof of a person is built. Section 1.4 lists what this changes |
| OD-THR36 | "Yes, like any device" | A machine that an agent started in the pool can open a connection to each other device of the person. THR-106 is changed |
| OD-THR13 | "Yes, cleaned and capped" | The first line of a prompt, as the title of a thread, goes to the hub, to notifications and to the result of an MCP tool |
| OD-THR34 | "Yes, from the start" | The holder process is a part of stage 1b and of the launch |

### 1.4 What his answer to OD-THR18 changes in other texts

He said: "If my other desicons are contridicarty to this I want them changed." These are the texts that asked for a proof of a person, or for a place that a program of the same user cannot write, and what each says now. The orchestrating session changed one answer itself ("not kept" of OD-THR16) and kept the switch at the machine of OD-THR19, because he chose to keep "the guards".

| Text | It said | Now |
| --- | --- | --- |
| THR-031 | Only a person answers an approval, in a window of the operating system; no `anima agents approve` | An approval is answered in each client and with the command `anima agents approve`, by whoever has the login of the person on a device. A program of the same user can do it |
| THR-048 | A script of the web view alone must not choose a folder, choose Full access or answer an approval | Withdrawn |
| THR-034, THR-105, OD-THR5, OD-THR20, OD-THR35 | A signature of a key of a client on a remote approval, a start and a prompt | Withdrawn; the hub is trusted, with the guards at the machine |
| THR-033, the condition of OD-THR6 and OD-THR7 | Full access and the environment of the person for a start through the hub only with a signature | Full access and the environment of the person for each thread |
| The condition of OD-THR8 | Tools of a tree for a different machine only with the grant | The tools work across machines, with the guards at the machine |
| TH33, THR-112 (the findings F9 and G14) | The switch, the folder list and the settings are kept where a program of the user of the agent cannot write them | They are kept in the settings of the daemon of the user. A program of the same user can change them; this is accepted |
| THR-047 | The runner resumes only a thread that it started | It stands: it is a rule against a forged session file, not a proof of a person |
| THR-029 | No MCP tool gives the entries of a thread; the hub refuses them to an API key | It stands for the general MCP server and for an API key (MCP-004). It is not a wall: a program of the same user can read the session files and run `anima agents thread` |
| THR-108 | A larger plan is bought only on the account site, with a sign-in check | It stands, and it is the one place where a proof is still asked for, because it is money. If he wants this changed too, it is OD-THR41 |

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
| Parent, child | A thread that started a different thread is its parent; the thread that was started is its child (stage 7) |
| Side chat | A thread beside a thread: it gets a copy of the conversation at that point, answers a question, and gives nothing back unless the person asks |
| Allowance | The time and the size of machines of the pool that the plan of an account gives for a period. No document defines it yet: HMD-073 gives the hub only the hours and the amount of the plan of the hub itself, and [cloud-machines.md](cloud-machines.md) has a spending cap (CVM-010) and fair-use limits (CVM-021). The pool contract of task TH90 must define it |
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
| 1.6 | Pin, snooze, settle, archive, delete | must (archive, delete); should | Nothing | — | Stage 4, which is built directly after stage 1b (Section 6) |
| 1.7 | Titles: automatic, rename | must; should | Nothing | The title that Claude Code writes (`custom-title`), else the first prompt (TH1) | Rename (stage 4) |
| 1.8 | Fork, side chat, sub-agents | later; should (sub-agents read-only) | The usage reader joins sub-agent files to their session | — | Later |
| 2.1 | One timeline: prompts, replies, grouped tool calls, plans, approvals, diffs, errors | must | Nothing | — | TH1 (entries), TH4 (view) |
| 2.2 | Prompts with chips and history | must; should | Nothing | — | Stage 1b composer |
| 2.3 | Replies, streamed by paragraph | must | Nothing | GraphQL subscriptions (`crates/hub/src/*/graphql.rs`, the "Live" indicator, MOT-011) | Stage 1b: the stream from the runner |
| 2.4 | Reasoning, folded | should | Nothing | — | TH1, TH4 |
| 2.5 | Tool calls by kind: command, edit, read, search, web, MCP, sub-agent | must | Nothing | — | TH1 classifies them (`ToolKind`) |
| 2.6 | Approvals: allow, allow for the session, decline, with feedback | must | MCP approval of a tool call to the person (`crates/anima-cli/src/mcp/approval.rs`, MCP-013) | The pattern "the person approves in the client" | Stage 1b on this computer (TH15, THR-030 to THR-033); stage 3 for a different machine (THR-034) |
| 2.7 | Questions of the agent with choices | must | Nothing | — | Stage 3 |
| 2.8 | Plans and lists of steps; plan mode | must (list); should (plan mode) | Nothing | — | TH1 marks `TodoWrite`, `update_plan`, `ExitPlanMode` as Plan; the plan card is stage 4 |
| 2.9 | The diff of each turn | must | Nothing | The patch of each edit in the file (TH1) | Read side: the patch of each edit. Checkpoint diffs: stage 4 |
| 2.10 | Errors with Retry | must | Nothing | — | Stage 1b |
| 2.11 | Context meter; usage page; limits | should; later | Agent usage (AIS-018 to AIS-024, built) | `fleet/agent_usage` | A link from a thread to its usage (TH4); use by thread needs AIS-018 to change (OD-THR9) |
| 3.1 | One adapter interface and one event model | must | Nothing | `daemon-core` (pure core) | the crate `agent-threads` (TH1) is the start of the event model |
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
| 14 | Search, notifications, usage, issues, secrets, sharing | should; later | Notifications, Agent usage, issues | — | The system notification of the desktop: stage 1a (TH6). Search of the list: built. The rest: Section 6.9 |
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

- **THR-012.** (Version 3: changed by R-292, OD-THR4: the hub keeps the entries of the threads, in plain form, for each device whose switch of OD-THR19 is on. The first form:) For each thread the hub must keep one row: the device, the program, the external ID, the project (by repository name, AIS-002), the title, the state, the first and the last time, and the counts of AIS-018. Under option B it must not keep a prompt, a reply, a tool input or a tool output. This is a promise of the hub program: a person who controls the hub can change it. So a device gives the entries of its threads to the hub only when a switch at the machine is on (OD-THR19).

### 4.4 The adapters: how Anima drives Claude Code and Codex

T3 Code uses the Claude Agent SDK and the account methods of `codex app-server`. Draft requirements of [bots.md](bots.md) forbid both for an adapter of Anima: BOT-021, BOT-032 ("the adapter must run only the `claude` program (`claude -p` or an interactive session). It must not use the Agent SDK with a subscription") and BOT-033 ("only `codex exec` and `codex exec resume`"). The named exception of R-196 is for the T3 Code catalogue entry only. This proposal does not ask to change them silently.

- **THR-020.** The Claude Code adapter must run the installed `claude` program, as the vendor published it, as one process for one thread: `-p`, `--input-format stream-json`, `--output-format stream-json`, `--verbose`, `--include-partial-messages`, a permission mode that the runner always names (`--permission-mode`), and `--session-id <uuid>` for a new thread or `--resume <id>` for one that exists. Approvals and questions come over the same pipe with `--permission-prompt-tool stdio`: the program writes a control request (`can_use_tool`, with an ID of the request, the tool, its input and suggestions for "allow for the session"), and the runner answers with a control response; an interrupt is a control request of the runner. It must not use the Agent SDK, make or pass a long-lived token (`claude setup-token`), or read the login file (BOT-020, BOT-032). Measured on this Mac with `claude` 2.1.311 (task TH10, [th10-driving-claude-code-and-codex.md](../research/agent-threads/th10-driving-claude-code-and-codex.md)): more than one turn in one process, text deltas, approvals for a command and for a file change, a question of the agent, interrupt, resume, fork, the model and the effort, the session ID chosen before the start, and state events (`session_state_changed`: running, requires action, idle) with `CLAUDE_CODE_EMIT_SESSION_STATE_EVENTS=1`. What the adapter must do itself, from the same record: always pass the permission mode (the default of this owner was `auto`); know that a command that the program holds for safe, and a rule of the settings, are decided before a request comes, and that the result says who decided; treat a message that is sent in a turn as queued (to steer is to interrupt and then to send); read the end of a turn from the result event, because the exit code is 0 after an interrupt. The risk: the word `stdio` for the approval tool is not in the help of the program. The fallback, also measured, is an MCP tool of the runner as the approval tool; it gives less (no ID of the request, no question).
- **THR-021.** The Codex adapter runs `codex app-server` (R-290, OD-THR3). Measured with `codex` 0.170.0 (task TH10): `codex exec --json` gives one turn and then ends, with no deltas, no approvals and no questions, so it is not a thread. `codex app-server` gives `thread/start`, `turn/start`, deltas, `item/commandExecution/requestApproval` and `item/fileChange/requestApproval` as requests that the runner answers, `turn/steer`, `turn/interrupt`, `thread/resume`, `thread/fork`, and `turn/completed` with a status as the end of a turn. The adapter calls only the methods of a closed list in the code; a method that is not in the list is not sent, and a test proves it. The list of the first build: the thread, turn and item methods above; `account/read`; `account/rateLimits/read` and its notice; and `account/login/start`, only when the person asks for the sign-in at the client of the same computer. Not in the list until OD-THR40 is answered: `chatgptAuthTokens`, and each method that takes or gives a token. The name of the account, the plan and the limits stay on the machine and in the client of that computer; they do not go to the hub, to a log or to a notification until OD-THR43 is answered. After `turn/interrupt` the command of the turn was still alive in the measurement, so a stop of the runner ends each process below the server of that thread (THR-049). Codex starts the MCP servers that the person configured, in each run.
- **THR-022.** Each adapter must turn the events of its program into the entries of the crate `agent-threads` (TH1): prompt, reply, reasoning, tool call with result and patch, plan, approval, question, end of turn (with the reason), error. The client draws only these entries; it never reads a format of a vendor.

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
| Answer an approval | `anima agents approve <thread> <request> allow|decline` | None (THR-031) | 1b (TH15), 3 |

- **THR-029.** No MCP tool may return the entries of a thread. A prompt or a tool output can hold a secret, and MCP-004 says that no tool returns one. The person reads a thread in the client or with `anima agents thread`. From stage 2 the hub must refuse the entries of a thread to an API key (INT-003) and give them only to a login. On the computer itself this rule cannot hold against a program of the same user: such a program can read the session files and can run `anima agents thread`; the rule keeps the entries out of the result of a tool, which goes to the vendor of the agent.
- **THR-031.** (R-292, OD-THR18.) An approval and a question of a thread are answered in each client, and with the command `anima agents approve <thread> <request> allow|decline`, by whoever has the login of the person on a device; a program of the same user can do it, and no proof of a person is asked for. The general MCP server of `anima mcp` has no tool that answers an approval (MCP-005 stays for it), and an API key cannot answer one. The first form, which is not in force: only a person answers, in a window of the operating system that the desktop shell opens, and no `anima agents approve`.

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
- [agent-layer.md](agent-layer.md) said: "It builds no diff screen, no chat view and no terminal host of its own at first" (from R-263). R-286 changes that for the chat view and the diff. Its hooks (the state "waiting for you") and its alert stay, and the thread list uses them as a source of state when they exist.

### 4.10 What the planning stream of Anima Product Ideas put on `dev`, and how this proposal stands to it

The operator said on 2026-11-20, first-hand to the orchestrating session: "There are also some proposals for it coming in from grok bot onto dev". `origin/dev` was read at commit `116c7b15` (2026-11-20). At that commit no document on `dev` proposes T3 Code as a native function: a search of `docs/proposals`, `docs/ideas` and `docs/plans` for T3 Code, threads, agent tools, the agent layer and agent runs finds only the documents below. A session that pushes or merges this proposal must read `dev` again first; a later document of that stream on the same subject is merged into one owner (PLN-016), not kept beside this one.

| On `dev` | What it gives | Where it is better than this proposal | Where it clashes | What is left for the operator |
| --- | --- | --- | --- | --- |
| [agent-tools-as-extensions.md](agent-tools-as-extensions.md) Section 6.10, AXT-079 to AXT-088 (R-281): the preview of a dev server and the forward of its port | A list "Servers" of the loopback ports of a device, a preview pane, a forward. AXT-082: the daemon ties a port to an agent session, and the row of that agent in AXT-046 shows the port | It owns the function that the inventory calls the local servers panel and the preview browser (7, marked later). It has a path that exists (SSH `direct-tcpip`), refused ports and an audit event. This proposal writes no preview of its own: a thread shows the servers of AXT-082 and opens them through AXT-083 | None with a ruling. AXT-086 has the switch on by default; the standing rule for a thread runner is off by default (TH33). They are different powers (reach a port; start a program) | Nothing new. The thread view takes AXT-082 when AX43 is built |
| Same document, Section 6.9, AXT-054 to AXT-078 (R-280): the shared project folder | A folder that the hub syncs to each device with Syncthing; MCP over HTTP for the hosted hub | It gives a project the same files on each device, which this proposal does not have: a thread on `pop-os` can work on what the Mac wrote | A git worktree for each thread (stage 4) inside a synced folder: Syncthing would sync `.git` and the worktrees between devices while two agents write. AXT-055 makes the hub copy the copy of record, and OD-THR4 option B keeps thread content off the hosted hub: the code of a project is on the hub in one and not in the other | OD-THR15 |
| Same document, Section 6.8, AXT-046 to AXT-053 (R-243, R-244): one list of agents on all devices, states, one alert | The list, the states, the alert through the needs-you inbox | It is the owner of the list and the states, with rulings | Two lists if both are built (4.9). AXT-049: "The view never shows the content of a pane, a prompt or a transcript". The thread view shows a transcript: for this computer in stage 1a, for other devices in stage 2 | OD-THR16 |
| Same document, Section 3 and goal: "Do not build a second Herdr, a second T3 Code or a second assistant bot in Anima.ing" | The direction of R-196 | — | R-286 reverses it for T3 Code. The sentence stays true for Herdr and the bot | The text of that sentence changes when this proposal is approved (Section 11) |
| [agent-layer.md](agent-layer.md) (R-263), on `dev` since 2026-11-20 | Hooks for the state "waiting for you", one alert, kept sessions in `tmux` | The hooks give a state for a session that the person started in a terminal, which the thread runner cannot know | Its kept sessions (AGL-023, `tmux`) and the thread runner (4.2, option B) are two ways to keep an agent alive. Its security read found four High findings on the kept sessions | OD-THR1 covers it: if B is chosen, the kept sessions of AGL-023 are not needed for Claude Code and Codex |
| The pattern of that stream: each new agent function is a section of agent-tools-as-extensions.md under row 4.21 (6.8, 6.9, 6.10) | One owner document and one row | One place to read; fewer cross references | This proposal is its own document and row 4.24, because R-286 gives it a different time (before the launch) and a different kind (Anima's own code, not a tool of another vendor), and row 4.21 is tier 3 after row 4.7 | OD-THR17 |

### 4.11 What Anima adds beyond T3 Code (R-297, R-295, R-291)

#### 4.11.1 The tree of threads and orchestration (stage 7)

The operator: "we have side chats, and then when it's relevant, we spawn a sub-chat that will deal with those problems itself. Then they can spawn sub-agents themselves". T3 Code has a fork, a side chat and sub-agent cards (inventory 1.8); it has no tree that the agents themselves build across machines.

- **THR-066.** A thread can have one parent. The list of threads is a tree: a parent with its children below it, folded or open, with the path of a thread ("orchestrator > child > grandchild") in its head. The order of THR-056 applies to the roots; a child that needs the person raises its root to "Needs you", and the parent shows the count of its children that wait.
- **THR-067.** A side chat: the person opens a chat beside a thread. It gets the conversation of the thread up to that point (the fork of the program where the program has one), it does not disturb the work of the thread, and it gives nothing back to the thread unless the person sends its answer there. A side chat uses the program and the model of its thread; a side chat with a different program or model sends the conversation to a second vendor, so it asks first.
- **THR-068.** A sub thread: a thread starts a child for a problem of its own, with a brief, in its own worktree or folder. The child ends each turn with a report in a fixed form: what is now true, what is not done or not verified, and the branch or the pull request. The report goes to the parent as a message; the parent can send a message to a child, and a child to its parent.
- **THR-069.** The agent of a thread gets tools of Anima to do this, as an MCP server that the thread runner gives to the program of that thread only: start a thread (brief, place, program, model), send a message to a thread of its own tree, read the state and the last report of such a thread, list its children, stop a child. The runner binds the tools to the thread: an agent cannot name a thread outside its own tree. This is the "yes" of OD-THR8.
- **THR-070.** A child has the reach of its parent and never more: its mode, its folders and its machines are those that the parent may use. A tool call that asks for more is refused and shown.
- **THR-071.** (Version 3: changed by R-290, OD-THR25: the limits are guidance, not a stop for the person (THR-114). The first form:) Limits, as settings with defaults (OD-THR25): the depth of a tree, the children of one thread, and the threads of an account that run at one time. A thread that meets a limit goes to "Needs you" and says which limit. Without limits one prompt could start threads without end.
- **THR-072.** A start, a message and a stop that an agent makes is recorded with the thread that made it, so that the person can see who started what (THR-044).
- **THR-073.** Usage by subtree: the tokens and the estimate of the cost of a thread and of each thread below it (needs OD-THR9).

#### 4.11.2 A thread on a chosen machine, with a chosen model and program (stage 7, on stage 3)

The operator: "spawn up sub agents on different machines and different models".

- **THR-074.** The person, or a thread through the tool of THR-069, names the place of a new thread: one device, or "any machine". For "any machine" the hub places it with the placement of the work queue and gives its reasons (PRO-051; task W3, in `dev`; ROU-001 to ROU-016 of [jobs.md](jobs.md)). It names the program (Claude Code, Codex) and the model. A start that an agent places is routed work: the rules of ROU-001 to ROU-016 apply (the caller of the limit of ROU-014 is the root thread and its person), and task TH86 waits for the security review RT7 of jobs.md.
- **THR-075.** (Version 3: decided by R-296, OD-THR29: the shared project folder when the machine has it; else the daemon makes a clone. The first form:) The folder of the project must be on the machine that is chosen (OD-THR29): the daemon clones the repository from the forge into a folder of the runner, or the shared project folder (AXT-054 to AXT-078) gives it. A machine that does not have the program, or where the person did not sign in to it, is not offered (BOT-066: each sign-in is on the machine, by the person).
- **THR-076.** This part is stage 3 with a parent link: it needs W3, W4 and the answer to OD-THR20, as stage 3 does.

#### 4.11.3 A server that fits the agent, inside the subscription (stage 8)

The operator: "given your subscription, automatically spin up another server that matches your needs better or matches that agent's needs. We can inject it into the agent prompt to spin up that server, and then that's how we make money: subscriptions."

Owners that exist, which this part joins and does not copy: [cloud-machines.md](cloud-machines.md) (a proposal, not approved) for a machine that Anima starts; the pool of Anima.ing and HMD-073 of [hosted-mode.md](hosted-mode.md) (the pool gives the hours and the amount of the plan to the hub; open) for the allowance; the idea `agent-execution-on-managed-machine` for the form "the agent here, the commands on a managed machine".

- **THR-077.** The prompt of a thread tells the agent that it may ask Anima for a machine that fits the work (more memory, a GPU, Linux, more disk), with a tool of THR-069. Anima starts the machine, joins it to the hub as a device of the account, runs the thread or the commands there, and removes it when the work ends.
- **THR-078.** (Version 3: decided by R-294, OD-THR27: past the allowance a request is refused, unless the person turned on billing by use or an automatic change to the next plan. For a machine of the own cloud account of the person the limit is his spending cap (CVM-010), not the allowance. The first form:) The pool enforces the allowance, never the agent and not the hub; the hub shows what the pool says, with its time. The agent asks; the pool reserves (THR-107) before it starts a machine, and it refuses a request that passes the allowance. Text that steers an agent can use up allowance; it cannot make a bill beyond the plan.
- **THR-079.** The default is no question to the person while a request is inside the allowance ("it's more so about staying within your subscription limits"). A setting makes each request ask first ("can change it in the setting to ask for permission").
- **THR-080.** At the limit of the allowance the thread goes to "Needs you" with three choices: wait for the reset of the period, run on one of the machines of the person, or change to a larger plan. This is where the larger plan is offered.
- **THR-081.** A machine of this kind is removed when its thread ends or when it is idle for a set time, also when the hub cannot be reached; the pool, not the agent, removes it (THR-112). Its disk is deleted under the rule of THR-109; what must stay is pushed to the forge or copied to a device of the person before.
- **THR-082.** The thread shows the machine that it started, its size, its time, and the allowance that it used, beside its tokens and the estimate of its cost.
- **THR-083.** A login of the person to Claude Code or Codex is not copied to such a machine (BOT-020). A thread there uses an API key that the person set for this use, or a sign-in that the person does on that machine; which of these the vendor terms permit for a rented machine is not checked (OD-THR28).
- **THR-084.** (Version 3: decided by R-294, OD-THR26: the own cloud account of the person first ([cloud-machines.md](cloud-machines.md), with its spending cap), then the pool of Anima.ing. The first form:) Whose machines: the pool of Anima.ing first (the assumption of the side session, told to the operator and not contradicted); his own cloud account is a later option (OD-THR26).

#### 4.11.4 The ideas of `docs/ideas/`, and where each would go (R-291)

(Version 3: the operator took all five, R-296, OD-THR30; they are THR-117 to THR-121 of Section 4.13, and their folders are deleted.) The five ideas were framed when the direction was "agent tools in a pane" (R-196). When this table was written none was picked by a ruling. This table says where each one goes if the operator picks it; the commit that makes an idea a part of this proposal deletes its folder (PLN-018). Until then each stays an idea, and nothing is built from it.

| Idea | What it gives | Where it goes here | Note |
| --- | --- | --- | --- |
| `needs-you-inbox` | Answer a question of an agent from the phone as a form; queue a message for an agent that works; carry on after a usage limit resets | THR-059 to THR-064 (the notification), TH52 (answer from the phone), TH48 (the queue), TH62 (after a limit) | The first two are what R-293 asks for. "After a limit resets" reads a limit of an account, which AIS-022 forbids today |
| `agent-state-hooks` | One small protocol by which each agent or script reports working, blocked or done | The source of a true state for a session that the person started in a terminal (Section 5.1); owner today: [agent-layer.md](agent-layer.md) | The file of a session cannot tell an approval; a hook can |
| `per-agent-resource-attribution` | Which thread uses the processor, the memory and the disk of a machine | A row of facts in the head of a thread, from the metrics of the daemon (stage 5, TH69) | Only Anima has the metrics beside the usage |
| `labelled-login-profiles` | Choose which of his own accounts a thread uses | The picker of the composer (TH69, inventory 10: more than one account) | Needs the terms check and a ruling (BOT-025 forbids a rule that rotates between logins) |
| `agent-execution-on-managed-machine` | The agent here, the commands on a managed machine, with keys that it cannot see | Section 4.11.3 | Joins stage 8 |

What the research on other tools says people choose a tool for (the feature list `agent-multiplexer-feature-list-2026-11-20.md` of the workspace, not in the repository), to hold the design against: the agents keep running when the window closes or the link drops (stage 1b, the runner in the daemon); the state can be trusted, the most praised and the most complained about (Section 5.1 says what stage 1a can and cannot tell); an answer from a different place, with a push (stages 3 and 6); a worktree for each agent with the diff and a pull request inside the tool (stage 4). The strong point of Anima that none of them has: each machine of the person in one place.

#### 4.11.5 What the second security read changes in this section

- **THR-105.** (Version 3: withdrawn by R-292, OD-THR20 and OD-THR35: no keys of clients and no grant. An agent may start a thread on a different machine through the hub; the guards are the switch at that machine and its list of folders. The first form:) A grant for a tree. When the person starts a root thread, the client signs one grant with a key of the client: the machines, the mode, the limits of THR-071, the repositories that are trusted (OD-THR21), whether machines of the pool may be asked for, and an end time. Each start of a child and each message between threads on different machines carries the grant and the chain of the runners that passed it. A runner, and the pool, check the chain against a key of a client that they learned from the person; the hub carries the bytes and cannot make or widen a start. The agent keeps its reach and no question is added. No plan owns the keys of clients today (OD-THR20, OD-THR35). **Until the grant exists: an agent starts threads only on the machine where it runs, and no agent starts a machine of the pool.**
- **THR-106.** (Version 3: changed by R-294, OD-THR36: a machine of the pool may open a connection to the devices of the person, as each device of the account. The rest stands. The first form:) A machine of the pool that a tree asked for is a device of its own kind: it claims only work of its tree; it gets no secret of a project that the grant does not name; the placement does not take it for "any machine" of a different tree; and the devices of the owner take no new connection from it unless the owner opens that with a setting (OD-THR36). It can reach the forge and the internet, and its parent can drive it.
- **THR-107.** The pool reserves; it does not only check. A request names a size and a most time; in one step the pool takes that time from the allowance or refuses. A machine ends at the end of its reservation; more time is a new reservation. The pool keeps a cap on the machines of an account at one time and on their size, for each plan. A GPU is offered only where the plan names it. The fair-use limits of CVM-021 apply; where the two disagree, the pool contract of task TH90 says which wins.
- **THR-108.** A change of a plan happens only on the account site of Anima.ing, with the sign-in check of an account security change (CVM-024). The card of THR-080 only opens that page. No API of the hub, no command, no MCP tool and no tool of a thread changes a plan or accepts an offer (as MCP-005 for a trust decision). The figures on the card come from the pool, not from the hub, and the card says which thread used the allowance and who started it.
- **THR-109.** What cannot be undone, and what stops it. The rule keeps the reach of the agent by making the action possible to undo, in place of a question, where that can be done:

| Action of Anima | Part that enforces | Rule |
| --- | --- | --- |
| The delete of the disk of a machine of the pool | The pool | A snapshot is kept for a set time and counted in the allowance, or the person confirms the delete (CVM-012); OD-THR37 |
| The removal of a worktree with changes | The runner | The work is saved to a private ref first (THR-097) |
| A stop of a thread whose work is not saved | The runner | A checkpoint is taken before the program is stopped |
| A revert of the files to a checkpoint | The runner | A checkpoint of the state before the revert is taken, so that the revert can be undone (THR-099) |
| A change of a plan, a purchase | The account site | THR-108 |

What the agent itself does with a tool of the machine (a force push, a delete of a volume, a drop of a database) is not stopped by Anima in Full access; it is stopped only by the approvals of the program in the other modes. This is said to the person at the mode picker. It is an accepted limit of THR-065, not a hard stop.
- **THR-110.** A message and a report between threads is text that is not trusted, also when it goes up to a parent on a machine that is trusted more. The tools of THR-069 give the state and the last report of a thread, not its entries (THR-029). There is a limit on the messages and on the turns of a tree in a period, so that two threads cannot answer each other without end; at the limit the tree goes to "Needs you". A brief and a report that pass the hub are content: OD-THR19 and the notice of OD-THR4 apply to them.
- **THR-111.** (Version 3: decided by R-294, OD-THR28.) The first form of stage 8: the agent program stays on a machine of the owner, with the login of the owner there, and only its commands run on the machine of the pool (the form of the idea `agent-execution-on-managed-machine`). Then no login and no key of a vendor leaves his machines, BOT-020 and BOT-066 hold, and THR-079 can be true. A thread whose program itself runs on the machine of the pool is a later form (OD-THR28). The bill of the vendor of the model is never a part of the allowance.
- **THR-112.** (Version 3: changed by R-292, OD-THR18: the settings of the runner are kept at the machine, where a program of the same user can change them; this is accepted. The settings of the pool are kept at the account site. The first form:) The pool ends a machine by its own facts: the end of the reservation, and the heartbeat of the daemon of that machine; not by a word of the hub. The removal deletes the disk, the device row, the daemon token, the key and the address of the machine on the network, and each snapshot that THR-109 does not keep; a test proves the list. A machine of the pool for a thread is a virtual machine of one account with a new disk, never a container beside a different account. The settings "ask me first" (THR-079), the limits of THR-071 and the switch of TH33 are kept where the hub and an agent cannot loosen them: at the account site for the pool, and at the machine for the runner (OD-THR18).

### 4.12 What the research on other tools changes (R-291)

Three reports of a side session of the orchestrating session (2026-11-20) read the source of about 14 tools, user comments, and the issues and discussions of T3 Code. They are in the workspace of the operator, not in this repository. The researchers ran no tool, and they did not read Reddit, X or Discord. This section used the short form that the side session wrote of them, and checked one claim in the source of T3 Code; the session that wrote this proposal did not read the three reports in full. Ideas and behaviour are taken, never code; two of the tools have licences that do not permit a copy into this repository in any case (cmux: GPL-3.0; Claude Squad: AGPL-3.0).

The main finding: T3 Code has most of the functions that its users love. What makes people leave a tool of this kind is a wrong state ("working" when the agent waits), a session that is lost or harmed by a reconnect, a heavy application, behaviour that differs from the command line of the agent itself (the strongest complaint about T3 Code), and a question that answers itself after a time. So "better than T3 Code" is: a state that is right, sessions that are never harmed, the behaviour of the real `claude` and `codex`, and better order. The requirements below are gates of the drive side, not additions for later.

**State**

- **THR-085.** The state of a thread has an order of trust for its sources: the end of the process; then the structured stream of the program (THR-020, THR-021); then hooks; then the session file; then the time of the last change. A weaker source never overrides a stronger one. The view names the source of a state in its help text.
- **THR-086.** There is a state "Unknown". A thread whose state no source can tell is Unknown, never Working and never Idle (AXT-048). The state that waits for the person is sticky: only the answer, or a stronger source that says that the agent went on, clears it; a timer never clears it. In stage 1a, "May wait for you" and "Your turn" are cleared by the session file and by an opening of the thread, not by a timer, except the limit of 24 hours of THR-055, which moves a thread to "Earlier" and is said in the help text.
- **THR-087.** A question or an approval is never answered by a timer. A request that nobody answers stays open until the person answers, the person stops the turn, or the process of the program ends; when the process ends, the runner clears the request and says so (THR-030).
- **THR-088.** The runner chooses the session ID of a new Claude Code thread before the start, where the program permits it, so that a resume never takes the session of a different agent in the same folder (task TH10 measures the flag).
- **THR-089.** Events of a sub agent do not change the state of its lead. A compaction can change the session ID of the program in the middle of a turn; the runner follows it and keeps the thread one thread.
- **THR-090.** A session that the person started by hand in a terminal has no runner. Its state comes from the session file (Section 5.1), and from hooks when the person permits them: a file of hooks that Anima makes and gives to the program for that start, with the settings of the person not changed ([agent-layer.md](agent-layer.md) owns the hooks). Hooks alone are not trusted: a stop with Escape, a killed process and a bypass mode fire none.

**The same behaviour as the command line of the agent**

- **THR-091.** A thread of Anima behaves as the program does in a terminal of the person: the same skills, slash commands, MCP servers, permission rules and settings, because the runner runs the real `claude` and `codex` (THR-020, THR-021; OD-THR21). This is a gate with a check: a fixed list of cases (a skill, a slash command, a project MCP server, an allow rule, a hook) gives the same result in a terminal and in a thread. A difference is a defect.

**Attention**

- **THR-092.** Who gets an alert is decided one time, in one place (the runner for this computer; the hub from stage 2), from presence: each client says which thread it shows and when the person was last active there. If a client that is present shows the thread, no alert is sent. Else the alert goes to the client that was active last. A push to the phone is sent only when no client is present. Two devices never ring for one event.
- **THR-093.** An item of "Needs you" closes by itself when the agent leaves the state that waits, wherever the answer was given. There is no "done" alert while a request is open or a message is queued. What is not opened yet stays so after a restart of the application.
- **THR-094.** Keys: go to the newest thread that needs the person; and "done, go to the next". The quiet time of THR-061 is new design: no tool that was read has one.

**Persistence**

- **THR-095.** A reconnect of a client, a restart of the application and an update of the daemon must not harm a turn that runs. This is a measured gate of stage 1b: a test stops and starts the client, and the daemon, while a turn runs, and the turn ends as it would have. To make the update of the daemon possible, a small holder process can own the processes of the agents and the end of their output while the daemon restarts (OD-THR34).
- **THR-096.** After a restart, the runner finds each thread by one scan of its store. It does not mark each running thread as failed. The event log is not cut so far that the view of a thread cannot be built again from it and from the session file of the program. The cursor of a client names the life of the store and the sequence, so that a client knows when its cursor is from an older store.

**Worktrees and landing (stage 4)**

- **THR-097.** A delete of a thread or of its worktree cannot lose work: the runner lists the files that are at risk, saves the work to a private ref first, and puts the thread back if the removal fails. A branch name that exists is never taken over in silence and never deleted in silence.
- **THR-098.** A new worktree can get the files that git ignores and that a file of the repository names (a `.worktreeinclude` file), only when git ignores them on both sides; a set-up script of a repository runs only after the person trusted it, and the trust is for the content of the script (its hash). Each worktree can get a block of ports of its own, so that two threads can run the same dev server.
- **THR-099.** Undo restores only the files that the turn changed and that nothing changed since; the checkpoints are pruned, with a limit of size. The results of CI and of a review of a pull request go back to the agent of the thread by themselves, as one message with the repeats removed and a limit of size, never one prompt for each comment.
- **THR-100.** A comment of the person on a line of a diff goes to the agent with the file, the lines and the text of the diff. T3 Code has this (`apps/web/src/reviewCommentContext.ts`); the first inventory missed it, and the parity table has its row.
- **THR-101.** (Version 3: decided by R-296, OD-THR32: a thread can hold more than one repository from the start.) A project can hold more than one repository (the most asked open request of the users of T3 Code). Its model is decided before task TH41 (OD-THR32).

**Steering and orchestration (stage 7, with Section 4.11.1)**

- **THR-102.** A sub agent is a real child thread, with its own view. A message of an agent to a thread steers it (it is taken at the next step); a message of the person interrupts. Queued prompts go as one turn. A stop also stops the queue and each wake-up that was planned. A retry never sends a prompt two times.
- **THR-103.** (Version 3: decided by R-296, OD-THR31: yes, in a late stage.) The same prompt to more than one agent, each in its own worktree, with a thread that judges the results and recommends one, is a choice of the operator (OD-THR31).

**Order (the gaps of T3 Code itself; stage 5)**

- **THR-104.** Split view, a second window, groups of threads that fold, and a move of a thread to a different project. Each thread can be opened in a terminal, and a session of a terminal is shown as a thread (stage 1a does the second).

Traps that the reports name, which the tasks must not repeat: a hook of Claude Code named `Notification` fires for more than a wait (only its kinds for a permission and for a question mean that the agent waits); the `notify` of Codex holds one program, which a different tool can own; a port taken from a hash alone collides; a folder path that can be renamed breaks the resume of the agent; submodules break a delete in three tools; a secret inside a QR code; a push text that is not encrypted.

What one report judged not worth a copy (navigation in code and an editor in the application, a board of cards, sandboxes of a vendor in a cloud, voice, a browser in the application, the best of N): these are functions of other tools, and the report gives the opinion of users. None is decided here: OD-THR33.

### 4.13 What the answers of version 3 add

- **THR-113.** The holder (R-290, OD-THR34: "Yes, from the start"). A small program, started by the daemon as the same user with an empty environment, owns the processes of the agent programs and the end of their output, so that the daemon can stop, update and start again while agents run (THR-095). What it does: it starts a program that the daemon names, with the arguments, the environment and the folder that the daemon gives; it keeps the pipes; it gives the bytes to the daemon and keeps what the daemon did not confirm yet, up to a limit for each thread (the default: 8 MiB; at the limit it stops reading, so the program waits, and the thread says so); it ends the processes of a thread when told (THR-049). It reads no settings, opens no file of the person and speaks to no network. Its channel is a socket file of the user (THR-128). It cannot tell the daemon from a different program of the same user, and the text does not claim that it can: the channel keeps out other users, web pages and the network. Each command has an ID and is done one time. The holder keeps a record of its processes in a file of the user, so that a new daemon finds them. The protocol has a version; a daemon and a holder of different versions say so and do not guess: the old holder keeps its agents until their turns end, and takes no new start. When the holder itself ends, its agents end with it (they are in its tree); the daemon starts a new holder and shows those threads as stopped. The output that the holder keeps is one more copy of content on the machine: it is in memory only and goes with the process.
- **THR-114.** The limits of a tree are guidance (R-290, OD-THR25). His words: "i like the 3, 5, 10 as guidence but the user shouldn't be hard stopped it can say that you are going over the recomend guidence and are at risk of Cyberpsychosis by running this many session. Do you want to raise the limit? Then Bump it to 5, 10, 15 and ask again for the next step". The first limits are depth 3, 5 children of one thread and 10 threads at one time. At a limit the person sees: "You are over the recommended guidance and at risk of cyberpsychosis from running this many sessions. Do you want to raise the limit?" A yes raises the limits to 5, 10 and 15, and the question comes again at that step. The steps after it are proposed as 8, 15 and 25, then 10, 20 and 40, then 10 more threads at each step (OD-THR39). A no keeps the limit. An agent cannot answer for him: a thread that meets a limit with no person there goes to "Needs you" and waits, so a soft limit is a stop for the agent until he answers. The answer is the answer of a person in the sense of OD-THR18: a program of the same user could give it. The runner counts: the threads of a tree by its record, and the threads of the account on this machine; from stage 3 the hub counts across machines. A tool of an agent that would pass a limit gets the answer "wait for the person", and no tool raises a limit. Two starts at the same time are counted one after the other. Until the count is built, no tool by which an agent starts a thread is given (task TH13 is the command of the person only).
- **THR-115.** Codex with the account methods (R-290, OD-THR3: "No we can use account methods everyone else is doing it"). What the account methods of `codex app-server` do, as T3 Code uses them (inventory 3.3): `account/read` tells which account and which plan Codex uses; `account/rateLimits/read` and its notice tell the limits and when they reset; `account/login/start` starts the sign-in of ChatGPT from the application; and `chatgptAuthTokens` lets the application give the tokens of a ChatGPT login to the server. What this opens: the name of the account and the plan in the thread; a limit display and "resume when the limit resets" for Codex (inventory 2.11, 5); a sign-in to Codex from Anima, with no terminal. What it does not decide: BOT-020 says that Anima does not keep or relay a login and that a login does not go through the hub. With `chatgptAuthTokens` Anima would hold the tokens of the person. The first build uses `account/read`, the limit methods and `account/login/start`, where the tokens stay in the store of Codex on the machine; whether Anima may hold or pass tokens is OD-THR40.
- **THR-116.** More than one repository in one thread (R-296, OD-THR32: "Several repos per thread now"). A thread names one or more repositories. The runner makes one worktree for each, side by side in one folder of the thread, outside each synced folder (OD-THR15). The agent works in that folder. A checkpoint, the diff of a turn, an undo and a pull request are made for each repository of the thread, and the view groups them by repository.

The five ideas, which are parts of this proposal now (R-296, OD-THR30: "All five"). Their folders in `docs/ideas/` are deleted by the commit of version 3; the history of the repository keeps their text.

- **THR-117.** (From `needs-you-inbox`.) A question of an agent is answered from each client as a form with its choices; a message can be queued for an agent that works; and a thread can go on when a limit of the vendor resets. The last one needs the limit of an account: for Codex THR-115 gives it; for Claude Code no ruling permits it yet (AIS-022).
- **THR-118.** (From `agent-state-hooks`.) One small protocol by which an agent program or a script reports working, blocked or done to the daemon, with the hook texts for Claude Code and Codex. It is the source of a true state for a session that the person started in a terminal (THR-090). [agent-layer.md](agent-layer.md) wrote the signal (AGL-008 to AGL-014); the two must be one text.
- **THR-119.** (From `per-agent-resource-attribution`.) The head of a thread shows what its processes use of the processor, the memory and the disk of the machine, from the metrics of the daemon.
- **THR-120.** (From `labelled-login-profiles`.) The person can name more than one account of a program on a machine, and choose which one a thread uses. Each sign-in is by the person on that machine. No rule changes from one account to the next by itself (BOT-025); the vendor terms row comes first.
- **THR-121.** (From `agent-execution-on-managed-machine`.) The agent on a machine of the person, its commands on a machine that was started for it, with keys that the commands cannot see: this is THR-111.

Three functions of other tools (R-296, OD-THR33):

- **THR-122.** Navigation in the code of a project and an editor in the application: a tree of files, open a file, go to a definition where a language server of the machine gives it, change and save a file. It joins task TH67.
- **THR-123.** A board of cards: the threads of a project as cards in columns by state (to do, working, needs you, in review, done), with a move of a card by drag. It is a second view of the list of threads, not a second store.
- **THR-124.** A browser in the application, for the dev server of a thread and for a page that the agent names. It is the preview pane of AXT-083; this proposal adds only that a thread opens it.

## 5. Requirements of stage 1a (the read side)

- **THR-003.** the crate `agent-threads` must read the lines of one session file of Claude Code (`~/.claude/projects/<project>/<id>.jsonl`) or Codex (`~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`) into a thread: the prompts the person typed, the replies, the reasoning, the tool calls with their results and the patch of an edit, the ends of the turns and the compaction marks, in the order of the file. Lines of a sub-agent and lines that the program wrote for itself are not entries. It is pure; it reads no file.
- **THR-004.** The title of a thread is the title that the person or the program gave (`custom-title`, `thread_name_updated`), else the first line of the first prompt, at most 80 characters.
- **THR-005.** A tool call has a kind (command, edit, read, search, web, agent, plan, question, other) from the name of the tool, and one line that says what it does (the command, the path, the pattern).
- **THR-006.** The text of one entry is held to 64 KiB and the output of one tool call to 16 KiB, cut at a character boundary with a mark; a thread holds at most 20,000 entries and counts the rest. A line that is not JSON changes nothing.
- **THR-007.** `anima agents threads` must list the threads of this computer, the last changed first, from the session files of this user only: only the paths of THR-003, never a settings or login file of the programs. What is checked: each folder from the home folder to `.claude/projects` and `.codex/sessions` is a real folder; a folder or a file below them that is a link is not followed (`O_NOFOLLOW` at the open); a session file belongs to this user and has one name. Not checked: that a folder does not change between the list and the open. A row reads only the start (256 KiB) and the end (64 KiB) of its file. `--program` and `--limit` (1 to 500, default 30) narrow it. It sends nothing to the hub and starts no program.
- **THR-008.** `anima agents thread <id>` must show one thread: as text for a person (control characters made harmless in a terminal), or with `--json` with each entry. An ID has 8 characters at least and no slash; it names a file whose name is the ID or ends with it.
- **THR-009.** The read side respects no exclusion of AIS-008, because nothing leaves the computer. If stage 2 sends rows to the hub, the exclusions of AIS-008 and AIS-023 apply to them.
- **THR-018.** (Version 3: changed by R-296, OD-THR13: the tool gives the title of each thread, cleaned and capped. It still gives the name of the folder and not its path, and no branch. The first form:) The MCP tool `anima_agents_threads` (tool set `agents`, read) must give no entry (THR-029) and a narrow row: the ID, the program, the title only when the person or the program gave it (never the start of a prompt), the name of the folder and not its path, no branch, the first and the last time. The result of a tool goes to the vendor of the agent that called it. Each text is cleaned and capped (MCP-008). The tool is never a tool of the remote MCP server of a hosted hub (AXT-071).
- **THR-013.** (Replaced by THR-056, R-293.) The first form: the thread list must group the threads as: changed in the last 2 minutes, today, earlier. A dot in the accent colour marks a file that changed in the last 2 minutes. The screen must not call it "Working": the file cannot tell a state (AXT-048).
- **THR-014.** The thread view must show, in order: the prompts on the trailing side, the replies, the reasoning folded, the tool calls of a turn in one block with one line each (kind, summary, result), the patch of an edit with removed and added lines, the end of each turn with "Stopped" when it was stopped. An open tool call shows its output. Each text of a session is drawn as text, never as markup; a test proves it. Claude Code writes no line for the end of a turn, so its threads show no end of a turn in stage 1a. A row and a thread are what a file says: each program of the user can write such a file.
- **THR-019.** A reply and a prompt are drawn as Markdown: paragraphs, headings, lists, code blocks with a fixed-width font, code in a line, and bold. The text of a session is never put into the page as markup of its own: the page builds each part from the parsed text, and a tag in a reply shows as text (task TH8). Colours for the words of a programming language are not in stage 1a.
- **THR-015.** In stage 1a the composer is a note: the thread is read from the file of the program, and to continue it the person uses the program. The screen must start nothing.
- **THR-016.** Keys of the thread screen: Up and Down move in the list and Enter opens; `/` goes to the search of the list; Cmd+Shift+[ and Cmd+Shift+] open the previous and the next thread; Cmd+1 to Cmd+9 open the thread at that place of the list; Cmd+F finds in the thread; `E` opens or closes the tool calls of the thread; `?` shows the keys. Cmd+K stays the jump field of Anima (R-224). A key is not taken while the focus is in a text field, except Escape.
- **THR-017.** The screen is the page Threads, its own item of the navigation bar, above Agent usage (R-287: "Own nav item"), with the mockup [agent-threads/index.html](../mockup/agent-threads/index.html). It is shown only in the desktop application; the phone and the web client do not show it in stage 1.

### 5.1 The order of the list, the state of a thread, and the notification (R-293)

The operator, 2026-11-20: "For the t3 it is important the session that are procesing go to the bottom and that session that need you input go to the top, they should also send you a notification like claude code".

**What a session file can tell, and what it cannot.** Stage 1a reads only the files that Claude Code and Codex write. Read on this Mac on 2026-11-20:

| Fact | Claude Code | Codex | In stage 1a |
| --- | --- | --- | --- |
| A turn ended | An `assistant` line with `stop_reason` `end_turn` | An `event_msg` line `task_complete` | Known |
| A turn was stopped | The note "[Request interrupted by user" in the turn of the person | An `event_msg` line `turn_aborted` | Known |
| A tool call has no result yet | A `tool_use` block with no `tool_result` | A `function_call` with no output | Known |
| That tool call waits for an approval | Nothing is written until the person answers | No approval line was found in the files | Not known. A call with no result runs, or it waits; the file does not say which |
| The agent asked a question | A tool call `AskUserQuestion` with no result | A tool call `request_user_input` with no result | Known only as an open tool call |
| The agent works now | The file changed a short time ago | The same | Guessed from the time of the last change |

So stage 1a knows "a turn ended" and guesses "may wait for an approval" from a tool call that has had no result for one minute. The true states (approval asked, question asked) come from the thread runner of stage 1b, which gets the request itself, or from the hooks of [agent-layer.md](agent-layer.md) for a session that the person started in a terminal.

- **THR-055.** The reader must say how a session file ends: a turn that ended, a turn that was stopped, an open tool call, an open turn, or nothing. From it, the clock and what the person opened, the client gives each thread one state: "May wait for you" (a tool call with no result for 1 minute or more, in a thread that changed in the last 24 hours and that the person did not open since); "Your turn" (a turn that ended in the last 24 hours, not opened since); "Working" (an open turn whose file changed in the last 2 minutes, or a tool call younger than 1 minute); "Done" (a turn that ended and that the person opened, or that is older than 24 hours); "Stopped"; "Idle" (an open turn whose file did not change for 2 minutes). The words of a state must not claim more than the file tells: "May wait for you" says in its help text that the file cannot tell a long command from an approval. What the person opened is kept on the computer, by the ID of the thread and the time.
- **THR-056.** (Version 3: changed by R-296, OD-THR24: the group "Earlier" is folded; a click or a key opens it, and a search opens it. "Needs you" stays at the top and "Working" at the bottom. The first form:) The order of the list: the group "Needs you" at the top, with "May wait for you" before "Your turn" and the newest first in each; the group "Earlier" between, with the threads that are done, stopped or idle, by their last change (OD-THR24); the group "Working" at the bottom. Each group has its title and its count; an empty group is not shown. Each row has its state as an icon with a word, never as a colour alone (UI-003). While the pointer is on the list, a change of the states moves no row; the rows slide to their new places when the pointer leaves (the rule of the Devices list, design specification Section 5.7.3, Reorder). A search or a choice of a program applies at once. THR-013 is replaced by this requirement.
- **THR-057.** The item Threads of the navigation bar must show the count of the threads of the group "Needs you", and nothing when the count is 0. The count has an accessible name ("3 threads need you").
- **THR-058.** Freshness in stage 1a: the list is read again when the window is seen again and each 30 seconds while it is seen; a hidden window reads nothing. This is a read at an interval, not a watch. Task TH7 gives the desktop shell a watcher of the two folders of session files (events of the file system, no read each second), which sends one event to the page for each change; it is not built, because the shell has no such part today.
- **THR-059.** When a thread passes to "May wait for you" or "Your turn", and the window of the application is not in front or shows a different thread, the desktop application must send one system notification of the operating system. It gives the title of the thread, or the name of its folder when the thread has no title, and what the thread waits for ("Your turn", "May wait for your approval"; from stage 1b: "Asks to run a command", "Asks a question"). A click opens that thread. One notification for one passage: a thread that stays in the state sends no second one, and a thread that the person has open sends none.
- **THR-060.** Not built in stage 1a: the desktop shell has no way to send a system notification today (no notification part is in `apps/desktop/src-tauri`). Task TH6 adds it; it is a new part of the shell and asks the person for the permission of the operating system. A system notification stays on the computer: its text goes to the operating system only.
- **THR-061.** Settings, in Settings under "Threads": which events notify (a turn ended; may wait for an approval; from stage 1b: an approval asked, a question asked, a turn that failed), a quiet time (from an hour to an hour, each day), and off. The default is: each event, no quiet time.
- **THR-062.** (Version 3: changed by R-296, OD-THR13: the title of a thread may be in a notification that leaves the computer, cleaned and capped, also when it is the start of a prompt. No other text of a prompt, no code, no command and no path. The first form:) A notification that leaves the computer (a channel of the hub, the phone) must hold no text of a prompt, no code, no command, no path and no name of a branch. It gives the name of the device, the program, and the words of the state. It gives the title of the thread only when OD-THR13 permits a title to leave the computer, and then only a title that the person or the program gave. The person can permit more in the settings of THR-061; the default is no.
- **THR-063.** From stage 2 the hub sends the notification of a thread of each device through its notifications domain and its channels (NOT-002: webhook, ntfy, email, Apprise), as one more kind of event beside the checks, with the settings of THR-061 on the hub. It is the alert of AXT-053 ("one alert through the needs-you inbox, not through a second alert path") and of AGL-020 to AGL-021 of [agent-layer.md](agent-layer.md): there must be one alert for a thread that waits, not two. The rules of the check notifications do not fit as they are (the review of the agent layer, SF-125): NOT-003 selects channels for each check, NOT-005 sends again, NOT-007 has a template; a thread alert is sent one time, has fixed words (THR-062), and its channels are chosen for threads as a whole. In stage 1a nothing goes through the hub: the hub does not know a thread.
- **THR-064.** For the mobile stream (no phone code is changed by this proposal): the centre button of the phone bar, which is Threads (R-288), shows the count of THR-057 from the hub; and the phone gets the notification of THR-063 through the channel that the phone has today. The mobile plan says that push notifications of Apple and of Google "need a relay that Anima operates" and that "ntfy gives notifications now"; so the first form is the ntfy channel of the hub, and a push of Apple or Google is a decision of the mobile plan.

## 6. Stages and tasks

Each stage is useful alone. A stage that drives an agent waits for the approval of this proposal and of the points of Section 8 that it names.

**The order of the build is by where the work runs, not by the number of a stage:** 1a (read, this computer), 1b (drive, this computer), 4 (each other function on this computer: worktrees, checkpoints, the diff of a turn, undo, commit, push, pull request, images, archive, delete, rename, steer, plan mode, find, a terminal), then 2 (read other machines), 3 (drive other machines), 7 (the tree of threads, orchestration, a chosen machine and model), 8 (servers inside the subscription), 5 (each other function of the inventory), 6 (the phone; its read part can follow stage 2). The numbers of the stages are names: rulings and the mobile plan cite them, so they do not change. Stage 4 comes before stages 2 and 3 because five functions marked must (the diff of a turn, checkpoints, archive, delete, images) are in it, and stage 3 has the heaviest security work. In the column "State", "Built (held)" means a branch that is pushed and not offered to the merge train.

### 6.1 Stage 1a: the threads of this computer, read only (before the launch)

CMD-030 order: there is no hub API in stage 1a, and the plan gives the reason that the order does not apply (`AGENTS.md`, "Command line first"): the function reads files of the computer where the command or the application runs, and sends nothing to the hub. The hub API comes in stage 2.

| Task | Work | Needs | Check | State |
| --- | --- | --- | --- | --- |
| TH0 | This proposal, the ruling, the rows, the mockup, the inventory record, the security read | — | The documents gate | In progress (branch `docs/agent-threads`) |
| TH1 | the crate `agent-threads`: the reader of THR-003 to THR-006 | TH0 merged (PLN-011) | Tests `thr_001` to `thr_006` | Built on `threads/read-side` |
| TH2 | `anima agents threads` and `anima agents thread` (THR-007, THR-008) | TH1 | Tests `thr_007`, `thr_008`, with a home folder of test files and a link that is refused | Built on `threads/read-side` |
| TH3 | The MCP tool `anima_agents_threads` (THR-018, THR-029) | TH2 | The catalogue tests (MCP-009, MCP-011) | Built on `threads/read-side` |
| TH4 | The desktop shell commands `agent_threads_list` and `agent_thread_read` (main web view only, WEB-007), and the page Threads (THR-013 to THR-017) | TH2; the mockup approved | Client tests; HT-079 | Built on `threads/read-side`. No person saw it in a window; not compared with the mockup; no separate reviewer read the page |
| TH5 | The state that a session file tells (THR-055), the order and the marks of the list with the held order (THR-056), the count on the bar (THR-057), the read at an interval (THR-058) | TH4; R-293 | Tests on made session lines and made rows, not on the files of a computer; HT-079 steps 8 and 9 | Built on `threads/needs-you` |
| TH8 | Replies and prompts as Markdown with code blocks, built from parsed text and never as markup of the session (THR-019) | TH4 | Client tests: a tag in a reply shows as text; a code block keeps its lines | Built on `threads/needs-you`; no colours for the words of a programming language |
| TH6 | The system notification of the desktop application (THR-059 to THR-061): a notification part in the shell, the permission of the operating system, the click that opens the thread, the settings | TH5; a decision to add the part to the shell | A human check on a Mac and on Linux | Not started |
| TH7 | A watcher of the folders of session files in the shell, in place of the read at an interval (THR-058) | TH5 | A test with a made folder; a human check | Not started |

Needs from W3, W4, W7: none.

### 6.2 Stage 1b: drive Claude Code and Codex on this computer (before the launch if the operator approves Section 8 in time)

| Task | Work | Needs | Check | State |
| --- | --- | --- | --- | --- |
| TH10 | A record in `research/agent-threads/`: run `claude -p` with stream JSON input and output, `--permission-prompt-tool` and `--resume`; and `codex app-server` (thread, turn and approval methods only) and `codex exec --json`. Write which events come, how an approval comes, how a stop works, which process tree stays | — | A record with measured results on this Mac | Done for this Mac (2026-11-20, [th10-driving-claude-code-and-codex.md](../research/agent-threads/th10-driving-claude-code-and-codex.md)): 16 runs with small prompts in an empty folder, no bypass mode, no account method. Open: the same on Linux; a trusted folder; a reply to an approval that never comes; `on-request` of Codex; a second look at the interrupt of Codex |
| TH11 | The thread runner in `anima-daemon`: start one program for one thread in its folder, with the environment of THR-040, send a prompt, take the events, keep no event log where the session file gives enough (OD-THR22), stop a turn | TH10; the checklist of Section 6.2.1 | Tests with a fake program; the process tree is gone after a stop | Built, held locally on the branch `threads/runner` until its security review (2026-11-20): `anima-daemon threads` and the runner in `run`; both programs measured on this Mac ([th11-runner-on-this-mac.md](../research/agent-threads/th11-runner-on-this-mac.md)). Open: Linux; the idle stop (no default time yet) |
| TH17 | The holder: a small program that owns the processes of the agents, so that the daemon can stop and start while they run (THR-113) | TH11 | Tests: the daemon is stopped and started while a made program runs, and no byte of its output is lost; the holder takes a command from no other user | Built, held locally on `threads/holder` and `threads/runner` until its security review (2026-11-20); the corrections of the first separate review are in. Open: Linux, where the user unit must leave the holder alive (KillMode) |
| TH12 | The local socket between the client and the runner on this computer, that only this user can open (as task W0) | TH11 | A test: a different user cannot open it | Built, held locally on `threads/runner` (2026-11-20): `runner.sock`, mode 0600 in a folder 0700, the peer must be this user. Open: a test with a second user |
| TH13 | `anima agents send`, `anima agents stop` and `anima agents approve` on this computer, with the login of the person. No MCP tool that starts a thread in this task (THR-114) | TH11 | Tests | Built, held locally on `threads/runner` (2026-11-20): also `start`, `interrupt`, `running`, `watch` and `raise-limit`. No MCP tool: an answer is a trust decision (MCP-005), and a start or a prompt runs a program (MCP-006, THR-114). A read tool of the running threads needs a tool that waits on the socket; it is not built |
| TH14 | The composer: model, effort, mode, send, stop; Enter sends, Shift+Enter makes a new line; the stream by paragraph | TH12 | Client tests; a human check | Not started (held) |
| TH15 | Approvals and questions on this computer (THR-030, THR-031, THR-046, THR-087) | TH11 | Tests: a reply to a stale request is refused; no request is answered by a timer, also not after 30 days of a made clock; the idle stop does not fire with an open request | Built, held locally on `threads/runner` (2026-11-20): one answer for each request while its process lives, a refusal is logged, a request survives a restart of the runner, no clock in the core. Open: the idle stop (not built), a real question of an agent |
| TH16 | The gates of the drive side: the states with their order of trust, Unknown and the sticky wait (THR-085, THR-086, THR-089); a reconnect and a restart that harm no turn (THR-095, THR-096); the same behaviour as the command line (THR-091); who gets an alert (THR-092, THR-093) | TH11, TH14 | A measured record for each gate, and a human check side by side with a terminal | Not started (held) |
| TH18 | A record: Full access, the default mode, measured with `claude` and `codex` in a folder that the person trusted and in one that the person did not; which flags the runner passes; what the programs ask at a first start in a folder (the third read, H17) | TH10 | A record in `research/agent-threads/` | Done for this Mac (2026-11-20, [th18-full-access.md](../research/agent-threads/th18-full-access.md)): 8 runs; neither program asks in Full access, a question of Claude Code still comes, the deny rules and hooks of a Claude Code project hold in each folder, the trust of Claude Code is for the exact folder, a bad Codex project file stops the start. Open: Linux, a question of Codex |

Needs from W3, W4, W7: none on this computer. A human check for each program on a Mac and on `pop-os`.


#### 6.2.1 Before the code of stage 1b merges

The operator answered the decisions that held stage 1b. His standing rule stays: code that starts an agent does not merge before a security review of that code. This is the list that the reviewer of the code uses; it is the list of the third security read (Section 9.2), and each point names its requirement.

1. Nothing of a thread leaves the computer in stage 1b; a test with a made hub proves it (THR-126).
2. The sockets of the runner and the holder are files in a 0700 folder of the user, with no port; the peer and the owner are checked; a test with two users passes; the runner does not start in a system unit (THR-128).
3. The holder starts with an empty environment and three descriptors. An agent starts with the environment of the login without each `ANIMA_` name and each value that is a secret of the daemon, with three descriptors, by an absolute path, with no shell, with the prompt on standard input; the test with a made agent passes (THR-040).
4. Each value that becomes an argument is checked; the session ID comes from the record of the runner (THR-040).
5. The credential of the MCP server of a thread is for one thread, ends with it and is in no argument (THR-127). No tool by which an agent starts a thread ships before the count of THR-114.
6. A stop, the idle stop and the end of the holder end each process below the program; the test with three children passes on a Mac and on Linux, and the case of Codex has its own test (THR-049).
7. An approval takes one answer, bound to the ID of the request and the life of the process; the ID is the same after a restart of the daemon; "allow for the session" sends only a rule that the request held; a refusal is logged (THR-030).
8. No timer answers a request: the test with a clock of 30 days passes, and the idle stop does not fire with an open request (THR-087).
9. The holder has a limit of bytes, a rule at the limit, a confirmation of what the daemon read, commands that are done one time, a record of its processes, a clean start after its own end and a version of its protocol (THR-113).
10. The log at the machine is mode 0600, has its five kinds of line, holds no text of a prompt and has a limit of size (THR-044).
11. The Codex adapter calls only the methods of its closed list; a sign-in starts only from this machine; the account, the plan and the limits go only where the operator said (THR-021, OD-THR40, OD-THR43).
12. Full access was measured in a trusted and in a not trusted folder, with a record; the runner writes no file of the vendor program (task TH18).
13. The page Threads passes the set of hostile texts; a separate reviewer read the page and the drive commands of the shell; no web view other than the main one can call a drive command (THR-125).
14. Each start, prompt, answer and stop that the code can make has an entry in [need-human-testing.md](../need-human-testing.md), for a Mac and for `pop-os`.
15. The gates of THR-085 to THR-096 have their measured records (task TH16).

The order of the build of stage 1b: (1) TH18, the measurement of Full access; (2) the protocols of the two programs as pure code in the crate `agent-threads`, with the samples of TH10 as tests (the first part of TH11; it starts nothing); (3) the holder, TH17, with a made program; (4) the runner in the daemon with the two adapters, TH11; (5) the socket, TH12; (6) the commands, TH13; (7) approvals and questions, TH15; (8) the composer and the stream in the client, TH14; (9) the gates, TH16.

### 6.3 Stage 2: the threads of the other machines, read only

| Task | Work | Needs | Check | State |
| --- | --- | --- | --- | --- |
| TH20 | Contract change: the rows of THR-012 in the push of K8 (an optional member, as the counts of AIS-019), with the exclusions of AIS-008; a ruling (CON-001) | OD-THR4, OD-THR9 | The contract review | Not started |
| TH21 | The hub domain `agents/threads/`: rows, queries `agentThreads`, `agentThread`, subscription `agentThreadEntries` (THR-023), with the entries kept on the hub (OD-THR4, THR-130), for a device whose switch is on (THR-126) | TH20 | Hub tests | Not started |
| TH22 | `anima agents threads --device`, the MCP tool with `device` | TH21 | Tests | Not started |
| TH23 | The page Threads with the choice of a device | TH21 | Client tests, a human check with `pop-os` | Not started |

Needs: W0 (the socket of the link) for a machine that reaches the hub through the link. Not W3, W4 or W7: nothing starts.

### 6.4 Stage 3: drive the agents of the other machines through the hub

| Task | Work | Needs | Check | State |
| --- | --- | --- | --- | --- |
| TH30 | Contract: the work type `agent_thread` of K10 and the long poll of THR-026; a ruling | Section 8 approved; OD-THR12 | The contract review, a separate security read | Not started |
| TH31 | The hub: the queue item, the commands of an open thread, the audit events | TH30; W3 | Hub tests | Not started (held) |
| TH32 | The daemon: claim, check, run, report (W4 for the run, without a container: an agent program needs the files of the person) | TH30; W4. Not W7: stage 3 does not wait for it (R-292, OD-THR12) | Tests of Section 8 | Not started (held) |
| TH33 | The switch "Threads may start here" at the machine, off by default (USE-004 is on by default; this one is not), and the list of folders where a thread of the hub may run. They are the guards that the operator kept (OD-THR20). They are kept in the settings of the daemon of the user; a program of the same user can change them (accepted, OD-THR18) | TH32 | A test: off refuses each item | Not started (held) |

### 6.5 Stage 4: each other function on this computer (built directly after stage 1b)

Each task of this stage runs `git` or changes files of the person through the runner, so each waits for the approval of Section 8 (P12, THR-050).

| Task | Work | Needs | Check | State |
| --- | --- | --- | --- | --- |
| TH40 | A folder of this computer as the project of a new thread, chosen by the person in the client (THR-041) | TH14 | A test; a human check | Not started |
| TH41 | A worktree for each thread: base branch, a name by the runner, a set-up report, the ignored files that the repository names, the trust in a set-up script, a block of ports (inventory 6.1; THR-050, THR-097, THR-098, THR-101) | TH11; OD-THR32 | Tests in a made repository | Not started |
| TH42 | Checkpoints after each turn as hidden commits under a private ref of Anima (inventory 6.2) | TH41 | Tests: a checkpoint takes no file that git ignores; the branch of the person does not move | Not started |
| TH43 | The diff of a turn and of the thread from two checkpoints, with its panel, find in the diff, and a comment on a line that goes to the agent (inventory 2.9; THR-100) | TH42 | Tests; client tests | Not started |
| TH44 | "Edit from here": cut the conversation, and revert the files only in a worktree of the runner (inventory 6.3) | TH42 | Tests: refused outside a worktree, and when a different thread uses the folder | Not started |
| TH45 | Commit, push and a pull request through the forge domain of the hub; the state of the branch, fetch, pull (inventory 6.4, 6.5) | TH41 | Tests with a made remote; a human check on GitHub | Not started |
| TH46 | The list: archive, delete, pin, rename, a title that a model writes, groups by project (inventory 1.4, 1.6, 1.7) | TH11 | Tests | Not started |
| TH47 | Images by paste or drag, files, a large paste as a file; sub-agents shown read-only (inventory 11, 1.8) | TH14 | Tests; the size limits | Not started |
| TH48 | The composer: steer, the queue on the runner, plan mode with the plan card, the card of steps, `/clear`, the history of prompts, the context meter (inventory 2.2, 2.8, 2.11, 12) | TH14 | Tests | Not started |
| TH49 | Find in the thread; a terminal for each thread through the terminal of Anima; a set-up script when a worktree is made (inventory 2.12, 7) | TH41 | Tests; a separate security read for the terminal | Not started |

### 6.6 Stage 5: each other function of the inventory

- **THR-053.** No function of the inventory of T3 Code is dropped (R-286: "should copy every feature they have"). Each row of the inventory has a stage in this proposal. A function that a different document of Anima owns is built there and shown in the thread; the table says which. The marks must, should and later give the order inside a stage, not a permission to leave a function out. Two rows of the inventory are not functions to copy: the telemetry of T3 Code, and "export", which T3 Code does not have.

Stages 1 to 4 hold each function marked must or should in Section 3.3. Stage 5 holds the rest. Each task that starts a program or opens a path to a machine has its own security read first.

| Task | Work | State |
| --- | --- | --- |
| TH60 | Projects: new, clone, publish; threads with no project; icons | Not started |
| TH61 | Snooze, settle, auto-settle, shelves, drag in the list | Not started |
| TH62 | The state "limited", resume at the reset of a limit, goals (needs a ruling on AIS-022) | Not started |
| TH63 | Other agent programs, one at a time, each with its vendor terms row | Not started |
| TH64 | Continue the threads after a restart | Not started |
| TH65 | Clean-up rules of worktrees, the copy-on-write cache, sparse checkout, submodules | Not started |
| TH66 | The pull request page, merge, stacks, watches, other forges | Not started |
| TH67 | The file picker, the search of a project, the file viewer and editor, open in an editor | Not started |
| TH68 | Project scripts with keys, terminal splits; the servers and the preview of AXT-079 to AXT-088 shown in the thread; the device panel | Not started |
| TH69 | The rest: the suggested next prompt, cite a reply, the prompt stash, dictation, prompt links, a thread reference, key rules of the person, fast mode, more than one account, guest links, scheduled tasks, secret providers, managed policy, diagnostics, the use of a machine by thread | Not started |

| Function of the inventory (section) | Stage | Owner, when it is not this proposal |
| --- | --- | --- |
| Pin, snooze, settle and auto-settle, archive, delete, tags, drag in the list, the Working shelf, grouping by project (1.4, 1.6) | 4 (pin, archive, delete, rename); 5 (the rest) | — |
| Threads with no project; add a folder, new project, clone, publish (1.1, 1.2) | 5 | Projects ([projects-and-builds-plan.md](../plans/projects-and-builds-plan.md)) for a project that is a repository |
| Fork, side chat, merge back, sub-agents as cards with their own thread (1.8) | 5; stage 1a already leaves sub-agent lines out, and stage 4 shows them read-only | — |
| Plan mode and the proposed plan; goals; the suggested next prompt; `/clear`; compaction; find in thread; cite a reply (2.8, 2.12) | 4 (plan mode, find, `/clear`); 5 (the rest) | — |
| The context meter, limits of a subscription, resume at the reset of a limit (2.11, 5) | 5. A limit of an account is read only if a ruling changes AIS-022 | Agent usage (AIS-018 to AIS-024) for the usage page |
| The modes Accept edits and Auto (3.2) | 3 (Accept edits); 5 (Auto) | — |
| Other agent programs: Cursor, OpenCode, Pi, Kiro, Grok Build, fx, Antigravity, Muse Code, any ACP agent (3.5) | 5, one program at a time, each with its vendor terms row | The catalogue and the terms rules of [bots.md](bots.md) (BOT-029 to BOT-046) |
| Continue threads after a restart; unload when idle; force stop (3.6, 5) | 3 (force stop, idle); 5 (continue after a restart) | — |
| Worktree clean-up rules, the copy-on-write cache, sparse checkout, submodules (6.1) | 5 | — |
| The pull request page (review, comment, merge, auto-merge, stacks), watches of a pull request, other forges, issues (6.4, 14) | 5 | The forge domain and Projects (PRO requirements); PRO-007 for a second forge |
| A terminal for each thread, splits, kept scroll-back; project scripts and set-up scripts (7) | 5 | The terminal of Anima (design specification Section 6.3) |
| The list of local servers of a thread; the preview browser (7, 14) | 5 | [agent-tools-as-extensions.md](agent-tools-as-extensions.md) Section 6.10, AXT-079 to AXT-088 (R-281) |
| The device panel (iOS Simulator, Android Emulator), screen capture (7, 14) | 5 | No owner yet |
| The whole keyboard table and keys that the person can change (8); the settings of Section 9 of the inventory | 1b to 4 for the keys of the functions of each stage; 5 for the file of key rules | Settings |
| Fast mode, more than one account of a program, the picker of the program (10) | 1b (model, effort); 5 (the rest; more than one account needs a ruling on BOT-025) | — |
| Files as attachments, a large paste as a file, excerpts of a terminal (11) | 4 (images); 5 (the rest) | Attachments of the hub (FIL requirements) |
| The queue on the server, edit and reorder of a queued message, steer, restart of a turn (12) | 1b (stop, a queue in the client); 4 (steer, the queue); 5 (the rest) | — |
| Remote: pairing, SSH, Tailscale, the relay of the vendor, the hosted web app, linked servers (13) | 2 and 3, through the hub, the link and the VPN of Anima; the relay of the vendor is not copied because the hub is that part | The hosted mode and the link |
| The phone application (13) | 6 | The mobile plan |
| Guest links (13) | 5 | No owner yet; SEC-008 and RA-035 forbid a public link today, so it needs a ruling |
| The file picker, the search of a project, the file viewer and editor, open in an editor (14) | 5 | — |
| Notifications when a thread ends or needs the person (14) | 3 | The alert of AXT-053 and of [agent-layer.md](agent-layer.md) |
| The `/` menu of commands and skills; restart of the agent session; the prompt stash; dictation; prompt links; copy a thread reference (14) | 1b (the `/` menu passes to the program); 5 (the rest) | — |
| Scheduled tasks and webhooks that start a thread (14) | 5 | [jobs.md](jobs.md) and the routing of R-243, with their reviews |
| Secret providers and a request of an agent for a secret (14) | 5 | Secrets (extensions plan Section 4.4); K10 Section 8 |
| Agents of the outside over MCP; MCP tools that the server gives to its own agents (14) | 5 | The MCP server of Anima ([cli-mcp-plan.md](../plans/cli-mcp-plan.md)); OD-THR8 |
| Import of the history of Claude Code and Codex (14) | 1a: the read side is this function | — |
| Themes and the theme editor; managed policy; diagnostics; triage (14) | 5 | The tokens and Settings of Anima; no second theme system |
| The command of the server (`t3 serve`, `pair`, `service`, `thread`) (14) | 1a to 3 | `anima agents` (Section 4.7) and `anima-daemon` |

### 6.7 Stage 6: threads on the phone

- **THR-054.** In the phone layout, Threads takes the place of Sessions in the bar at the bottom (R-288: "And on mobile it should replace the center button where sessions are"). The phone shows the threads of the devices through the hub (stage 2); it has no session files of its own. From stage 3 the phone sends a prompt and answers an approval under the rules of Section 7.

**Note for the mobile stream.** The mobile plan owns the phone bar (MOB-010: "four destinations: Devices, Sessions, Logs and Settings"; mockup 6; task M2-2) and all phone and iOS code. This proposal and the tasks TH1 to TH4 change none of it: the page Threads is shown only where the capability `agentThreads` is true, which is the desktop application; the browser and the phone have it false. What the mobile stream gets from R-288:

1. MOB-010 changes when stage 2 is built: the second destination of the phone bar is Threads, not Sessions. Until then the phone bar stays as it is.
2. Open point, OD-THR23 here and OD-M11 in the mobile plan: where the live remote sessions (MOB-020 to MOB-022; NAV-008) are reached from on the phone when Sessions is no longer in the bar.
3. The phone needs the hub API of stage 2 (THR-023) and the decisions OD-THR4, OD-THR16 and OD-THR19, because each thread that a phone shows comes through the hub.

| Task | Work | Needs | Check | State |
| --- | --- | --- | --- | --- |
| TH50 | The mobile stream changes MOB-010 and mockup 6, and decides OD-M11 with the operator | R-288; OD-THR23 | The mockup of the phone bar | Not started (the mobile stream) |
| TH51 | The page Threads in the phone layout, read only, through the hub | TH21, TH50 | A human check on an iPhone and an Android phone | Not started (the mobile stream) |
| TH52 | Prompts and approvals from the phone | Stage 3 | A separate security read | Not started (held) |

### 6.8 Stages 7 and 8: orchestration, a chosen machine and model, servers inside the subscription

Each task starts or drives an agent, or starts a machine: none is built for the merge train before the approval of this proposal, of Section 8 and of the reviews that the rows name.

| Task | Work | Needs | State |
| --- | --- | --- | --- |
| TH80 | The parent link, the tree in the list, the path in the head, the count of children that wait (THR-066) | TH11, TH46 | Not started |
| TH81 | Side chat and fork (THR-067) | TH80; the fork of each program (TH10) | Not started |
| TH82 | The tools of Anima for the agent of a thread: start, message, read, list, stop; bound to its tree (THR-069, THR-070, THR-072); one prompt to more than one model | TH80; OD-THR8; a separate security read | Not started |
| TH83 | The limits of a tree and the state "Needs you" at a limit (THR-071) | TH82; OD-THR25 | Not started |
| TH84 | The report form and the messages between a parent and a child (THR-068) | TH82 | Not started |
| TH85 | Usage by subtree (THR-073) | TH80; OD-THR9 | Not started |
| TH86 | The place of a thread: a device or "any machine" with the placement and its reasons; the program and the model (THR-074) | Stage 3; W3 | Not started |
| TH87 | The folder of the project on the chosen machine (THR-075) | TH86; OD-THR29 | Not started |
| TH90 | Contract: the request for a machine, and the allowance from the pool to the hub (THR-078; HMD-073; [cloud-machines.md](cloud-machines.md)) with a ruling | OD-THR26; the approval of cloud-machines.md | Not started |
| TH91 | The hub and the pool: check the allowance, start, join to the hub, remove (THR-077, THR-081) | TH90; a separate security read | Not started |
| TH92 | The tool of the agent and the text of the prompt (THR-077); the setting "ask me first" (THR-079) | TH82, TH91 | Not started |
| TH93 | At the limit: "Needs you" with the three choices (THR-080) | TH91 | Not started |
| TH95 | The machine, its time and the allowance used, shown in the thread (THR-082) | TH91 | Not started |
| TH96 | A record: the vendor terms for Claude Code and Codex on a rented machine (THR-083) | — | Not started |

Tasks of the three functions of other tools (stage 5): TH70, navigation in code and an editor (THR-122, with TH67); TH71, the board of cards (THR-123); TH72, the browser in the application opened from a thread (THR-124, after AXT-083). Each is not started.

### 6.9 The parity table: each function of the inventory with its stage, its task and its state

"Built on a branch" means the branches `threads/read-side` and `threads/needs-you`, which are pushed and not merged. No function is "held": no code of the drive side exists.

| Inventory | Function | Mark | Stage | Task | State |
| --- | --- | --- | --- | --- | --- |
| 1.1 | Add a folder as a project | must | 4 | TH40 | Not started |
| 1.1 | New project, clone, publish | should | 5 | TH60 | Not started |
| 1.1 | Icons, pin and archive of a project | later | 5 | TH60 | Not started |
| 1.2 | Threads with no project | later | 5 | TH60 | Not started |
| 1.3 | Create a thread and send the first prompt | must | 1b | TH11, TH14 | Not started |
| 1.3 | Choose a worktree or the folder at the start | should | 4 | TH41 | Not started |
| 1.3 | One prompt to more than one model | later | 7 | TH82 | Not started |
| 1.4 | The list of threads | must | 1a | TH4 | Built on a branch |
| 1.4 | Threads grouped by project | should | 4 | TH46 | Not started |
| 1.4 | Pinned threads | should | 4 | TH46 | Not started |
| 1.4 | A shelf of the threads that work | later | 1a | TH5 | Built on a branch |
| 1.4 | Settled and snoozed shelves, drag in the list | later | 5 | TH61 | Not started |
| 1.5 | States from the session file: working, your turn, may wait, done, stopped, idle | must | 1a | TH5 | Built on a branch |
| 1.5 | True states: approval asked, question asked, failed | must | 1b | TH15 | Not started |
| 1.5 | Not opened since the last change (unread) | should | 1a | TH5 | Built on a branch |
| 1.5 | Limited by a usage limit; a goal that runs | later | 5 | TH62 | Not started |
| 1.6 | Archive and delete a thread | must | 4 | TH46 | Not started |
| 1.6 | Snooze, settle, auto-settle | later | 5 | TH61 | Not started |
| 1.7 | A title: of the program, else the first prompt | must | 1a | TH1 | Built on a branch |
| 1.7 | A title that a model writes; rename | should | 4 | TH46 | Not started |
| 1.8 | Sub-agents shown read-only in the thread | should | 4 | TH47 | Not started |
| 1.8 | Fork, side chat, merge back, sub-agents as threads | later | 7 | TH80, TH81 | Not started |
| 2.1 | One timeline of a thread | must | 1a | TH4 | Built on a branch |
| 2.2 | Send a prompt | must | 1b | TH14 | Not started |
| 2.2 | Chips and the history of prompts | should | 4 | TH48 | Not started |
| 2.3 | Replies as text | must | 1a | TH4 | Built on a branch |
| 2.3 | Replies as Markdown with code blocks | must | 1a | TH8 | Built on a branch |
| 2.3 | Replies streamed by paragraph; live output of a command | must | 1b | TH14 | Not started |
| 2.4 | Reasoning, folded | should | 1a | TH4 | Built on a branch |
| 2.5 | Tool calls by kind, with input and output | must | 1a | TH1, TH4 | Built on a branch |
| 2.6 | Approvals | must | 1b | TH15 | Not started |
| 2.7 | Questions of the agent | must | 1b | TH15 | Not started |
| 2.8 | A list of steps (todo) as a card | must | 4 | TH48 | Partly built |
| 2.8 | Plan mode and the proposed plan | should | 4 | TH48 | Not started |
| 2.9 | The patch of each edit | must | 1a | TH1, TH4 | Built on a branch |
| 2.9 | The diff of a turn and of the thread | must | 4 | TH43 | Not started |
| 2.9 | A comment on a line of a diff, sent to the agent (found by the research; `apps/web/src/reviewCommentContext.ts`) | should | 4 | TH43 | Not started |
| 2.10 | Errors with Retry | must | 1b | TH14 | Not started |
| 2.11 | The context meter | should | 4 | TH48 | Not started |
| 2.11 | The usage page with the estimate of the cost | later | — | Agent usage (AIS-018 to AIS-024); the link is TH4 | Owned by a different document |
| 2.12 | Compaction marks | should | 1a | TH1 | Built on a branch |
| 2.12 | `/clear`, find in thread | should | 4 | TH48, TH49 | Not started |
| 2.12 | Goals, the suggested next prompt, cite a reply, a request for a secret | later | 5 | TH62, TH69 | Not started |
| 3.1 | One adapter interface and one event model | must | 1b | TH11 | Partly built |
| 3.2 | Modes: supervised and full access | must | 1b | TH14, TH15 | Not started |
| 3.2 | Modes: accept edits, auto | should | 1b | TH15 | Not started |
| 3.3 | The Codex adapter | must | 1b | TH10 (measured), TH11 | Not started |
| 3.4 | The Claude Code adapter | must | 1b | TH10 (measured), TH11 | Not started |
| 3.5 | Other agent programs | later | 5 | TH63 | Not started |
| 3.6 | The life of a turn: start, stream, interrupt, force stop | must | 1b | TH11 | Not started |
| 4.1 | Commands, events, projections, a command done one time | must | 1b | TH11 | Not started |
| 4.2 | The event log | must | 1b | TH11 | Not started |
| 4.3 | A stream with a snapshot and a cursor; reconnect | must | 1b | TH12 | Not started |
| 5 | A thread stays when the window closes | must | 1b | TH11 | Not started |
| 5 | Stop a program that is idle | should | 1b | TH11 | Not started |
| 5 | Continue threads after a restart | should | 5 | TH64 | Not started |
| 5 | Resume when a usage limit resets | later | 5 | TH62 | Not started |
| 6.1 | A worktree for each thread, base branch, name | should | 4 | TH41 | Not started |
| 6.1 | Clean-up rules, copy-on-write cache, sparse checkout | later | 5 | TH65 | Not started |
| 6.2 | Checkpoints after each turn | must | 4 | TH42 | Not started |
| 6.3 | Undo: edit from here, revert the files | should | 4 | TH44 | Not started |
| 6.4 | Commit, push, pull request | should | 4 | TH45 | Not started |
| 6.4 | Pull request page, merge, stacks, watches, other forges | later | 5 | TH66 | Not started |
| 6.5 | Branch status, fetch, pull, switch | should | 4 | TH45 | Not started |
| 7 | A terminal for each thread | should | 4 | TH49 | Not started |
| 7 | A set-up script when a worktree is made | should | 4 | TH49 | Not started |
| 7 | Project scripts with keys; splits; kept scroll-back | later | 5 | TH68 | Not started |
| 7 | The local servers of a thread; the preview browser | later | 5 | AXT-079 to AXT-088; shown by TH68 | Owned by a different document |
| 7 | The device panel (simulators), screen capture | later | 5 | TH68 | Not started |
| 8 | Keys of the list and the thread | must | 1a | TH4 | Built on a branch |
| 8 | Keys of the composer, the pickers, the diff, the terminal | must | 1b, 4 | TH14, TH43, TH49 | Not started |
| 8 | Key rules that the person can change | should | 5 | TH69 | Not started |
| 9 | Settings: default model, mode, program path | must | 1b | TH14 | Not started |
| 9 | The other settings of the inventory | should | 4, 5 | with the function of each | Not started |
| 10 | Model and effort pickers | must | 1b | TH14 | Not started |
| 10 | Fast mode, more than one account, the picker of the program | should | 5 | TH69 | Not started |
| 11 | Images by paste or drag | must | 4 | TH47 | Not started |
| 11 | Files, a large paste as a file, terminal excerpts | should | 4 | TH47 | Not started |
| 12 | Stop a turn; a queue in the client | must | 1b | TH14 | Not started |
| 12 | The queue on the runner, steer, restart of a turn | should | 4 | TH48 | Not started |
| 13 | Threads of other machines, read | later | 2 | TH20 to TH23 | Not started |
| 13 | Drive the agents of other machines | later | 3 | TH30 to TH33 | Not started |
| 13 | Pairing, SSH, Tailscale, the relay of the vendor | later | — | The hub, the link and the VPN of Anima are this part | Owned by a different document |
| 13 | The web client | later | 2 | TH23 | Not started |
| 13 | The phone application | later | 6 | TH50 to TH52 | Not started |
| 13 | Guest links | later | 5 | TH69 | Not started |
| 14 | Search of threads; find in thread | should | 1a, 4 | TH4 (the list), TH49 | Partly built |
| 14 | File picker, search of a project, file viewer and editor, open in an editor | should | 5 | TH67 | Not started |
| 14 | Notifications when a thread ends or needs the person | should | 1a | TH6 | Not started |
| 14 | The `/` menu of commands and skills; restart of the agent session | should | 1b | TH14 | Not started |
| 14 | Prompt stash, dictation, prompt links, copy a thread reference | later | 5 | TH69 | Not started |
| 14 | Issue tracker | later | — | Projects and the forge domain | Owned by a different document |
| 14 | Scheduled tasks and webhooks that start a thread | later | 5 | TH69, with jobs.md | Not started |
| 14 | Secret providers | later | 5 | TH69, with Secrets | Not started |
| 14 | Agents of the outside over MCP; tools that the server gives to its agents | later | 7 | TH82 | Not started |
| 14 | Import of the history of Claude Code and Codex | later | 1a | TH1 to TH4 | Built on a branch |
| 14 | Themes and the theme editor | later | — | The tokens and Settings of Anima; no second theme system | Owned by a different document |
| 14 | Managed policy, diagnostics, triage | later | 5 | TH69 | Not started |
| 14 | The command of the server | should | 1a | TH2 | Built on a branch |
| 14 | Telemetry of T3 Code; export (T3 Code has none) | n/a | — | — | Not copied |
| 15 | A local channel that only this user opens; approvals bound to a live request | must | 1b | TH12, TH15 | Not started |

## 7. Requirements of the drive side (stages 1b to 4; not approved)

- **THR-030.** An approval is bound to one request of one live process of one thread. The runner gives each request an ID, keeps the reference of the program, and writes both to its record, so that the ID is the same after a restart of the daemon while the holder keeps the process. It accepts one answer for an ID while that process lives; a second answer, an answer to an ID that it did not make, or an answer after the process ended is refused and logged (THR-044). "Allow for the session" sends to the program only a rule that the request itself offered; the runner writes no rule of its own and no file of the program.
- **THR-031.** (Its text is in Section 4.7.)
- **THR-032.** The runner must show the person what the request names (the command, the file and the patch, the path outside the folder) from the request of the program, not from text that the hub sends.
- **THR-033.** (R-290, OD-THR6.) The default mode of each new thread is Full access, also of a thread that the hub or an agent starts. The person can choose a different mode for a thread and as the default. The mode picker says, in one sentence, that in Full access the agent runs commands with no question and that Anima stops nothing that it does with the tools of the machine. The first form, which is not in force: Full access only as a choice of the person for each thread.
- **THR-034.** (Version 3: withdrawn by R-292, OD-THR5 and OD-THR20: an approval of a thread on a different machine is answered through the hub with the login. The first form:) With a hosted hub, an approval of a thread on a different machine needs the signature of a key of a client of the operator, so that a hub alone cannot forge an answer. No plan owns such a key today (W7 is the key of a device, which proves the device to the hub, not the hub or the person to the machine). What is signed: the ID of the request, the thread, the device, a hash of the request as the runner made it (the command, the file and the patch), the answer, a value used once and an end time. The runner checks the signature against the request that it holds, not against text of the hub. Until then an approval of a thread on a different machine is answered only on that machine (OD-THR5).
- **THR-040.** (R-290, OD-THR7.) How the runner starts an agent program. (1) By its absolute path, found at the machine, with a list of arguments and no shell. Each value that becomes an argument (the model, the effort, the mode, a session ID) is checked against a closed alphabet, does not start with `-`, and goes after a named flag; a session ID comes from the record of the runner, never from a client. A prompt goes on standard input, never as an argument. (2) With the environment of the login of the person. The daemon reads it one time at its start, by a run of the login shell of the person with a fixed command that prints the environment; this runs the profile of the person, which is code of the person. (3) Never passed to an agent: each variable whose name starts with `ANIMA_`; each variable whose value is a secret that the daemon holds (the daemon token, a token of the link); the socket of the link and the sockets of the runner and the holder as a path in a variable. The holder itself starts with an empty environment. (4) With three open descriptors (standard input, output and error) and no other: each other descriptor of the daemon and of the holder is closed at the start of the program. (5) In the folder of the thread, which holds no file of Anima. What is not claimed: the login token of the `anima` command is in the key store of the person, and a program of the same user can use it (accepted, OD-THR18); so "no secret of Anima reaches the agent" means the five points above and no more. A test with a made agent program proves each point.
- **THR-041.** The folder of a thread must be a folder that the person chose at the machine, or a worktree that the runner made under a folder of the runner. A folder from the hub must be in the list of folders that the person permitted at the machine.
- **THR-042.** Text from the hub (a prompt) and text from the outside world that the agent reads is not trusted. The runner marks nothing as an instruction; the client shows where a prompt came from (this computer, the hub, a schedule, an agent) on the prompt.
- **THR-043.** The event log keeps no value that the runner knows is a secret (the variables of THR-040 by name) and applies the redaction crate of E0-3 to tool outputs before a copy leaves the device. The redaction finds known forms only; it is not a guard. The event log of the runner is a second copy beside the session file of the program: it has a size limit, a time after which it is deleted (OD-THR22), and it is deleted with its thread. (Version 3: the clause of the variables is empty now, because the agent gets the environment of the person. The redaction is of tool outputs only, by known forms; it is not a guard, and with OD-THR4 the hub keeps what it does not find.)
- **THR-044.** Each start, prompt, approval answer and stop is written to a log of the runner at the machine: a file of the user, mode 0600, with a limit of size, with five kinds of line (start, prompt, answer, stop, refusal), each with the time, the thread, and what the runner could itself see of the source (the local socket, or the hub). It holds no text of a prompt and no output. It is a record for the person, not a guard: a program of the same user, and so an agent that the hub started, can change it. From stage 2 each of these is also an audit event on the hub, with the actor and the device, without the text. The label of the source on a prompt (THR-042) for a prompt from the hub is what the hub says.
- **THR-045.** On a device with more than one user, the runner of a user starts programs only as that user and reads only the session files of that user; the socket and the event log are mode 0600 in a folder of that user.

- **THR-046.** (R-290, OD-THR21.) The settings of a project and of the user of an agent program are honoured, as the program does in a terminal, and the thread says so in a notice. The runner sets the permission mode at each start and does not claim that a thread is Supervised. The drive side fails closed for what it does not know: an event, a kind of request or a version of the program that the runner does not know stops the turn and is shown. An approval or a question with no answer stays open (THR-087); only an answer, a stop by the person, or the end of the process closes it. The first form, which is not in force: the settings of the project off, and a missing answer as a decline.
- **THR-047.** The runner resumes only a thread that it started, by its own record. A session that the person started in a terminal is read only, until the person takes it over with a confirmation that names the file.
- **THR-048.** (Version 3: withdrawn by R-292, OD-THR18. The first form:) A script of the web view alone must not be able to choose the folder of a new thread, to choose Full access or to answer an approval: each of the three needs a window of the operating system from the shell.
- **THR-049.** A stop of a turn, the idle stop and the end of the holder end each process that the program of the thread started, also a command that put itself in a different process group (measured: a command of Claude Code runs in its own group, and a command of Codex stays alive after an interrupt). The runner follows the processes below the program of a thread (on Linux a control group of the thread where the user unit permits one, else the walk of the process tree; on a Mac the walk of the process tree), and a test with three made children in three groups proves it on both systems. A process that left the tree on purpose (a daemon that the agent started) is not found; the thread says that it could not prove that all ended. The idle stop ends a program that has no turn, and it does not fire while an approval or a question is open (THR-087). The time of the idle stop and the most threads that run on a device are settings. After a restart of the daemon the programs of the threads still run, in the holder (THR-113), and the daemon finds them again. After the end of the holder, or a start of the computer, no program of a thread runs: the threads show as stopped, and nothing starts again without the person. One `claude` task for each login at a time is the default of BOT-032; a second thread of the same login in stage 1b, and a tree of threads, change it (Section 11).
- **THR-050.** The `git` of the runner (worktrees, checkpoints, revert) runs with the hooks off and with no configuration of the repository that names a program (`core.hooksPath`, `core.fsmonitor`, filters, `core.sshCommand`). A checkpoint does not take a file that git ignores. The name of a worktree is made by the runner from a fixed alphabet, never from the text of a prompt.
- **THR-051.** The Claude Code adapter drops each member of the stream that is about the account or its limits (AIS-022). The Codex adapter reads them as THR-021 and THR-115 say, keeps them on the machine, and gives them to no log (LOG-007) and not to the hub (OD-THR43).
- **THR-125.** Each text of a thread is drawn as text, never as markup (THR-014, THR-019). After the answers of version 3 this is the one wall between the author of a text that an agent read and the commands of the main web view that start, prompt and approve. So it is a guard with a test: a fixed set of hostile texts (tags, scripts, links with a script, entities, bidirectional marks, very long lines) in each field of a thread gives no element and no request. No web view other than the main one can call a command of the shell that drives a thread (WEB-007). A separate reviewer reads the page and the drive commands of the shell before they merge.
- **THR-126.** What leaves the computer. In stage 1b nothing of a thread leaves the computer: no title, no folder, no entry, no ID of a thread, no field of an account. The runner makes no request to the hub for a thread, and it refuses an item of the kind `agent_thread`; a test with a made hub proves it. From stage 2 there are two switches at a machine, both off by default: "This device gives its threads to the hub" (OD-THR19: the rows, the titles and the entries of its threads) and "Threads may start here" (TH33: a start and a prompt from the hub). With the first switch off nothing of a thread goes to the hub, also not a title and not a notification (OD-THR42).
- **THR-127.** The MCP server that the runner gives to the program of a thread (THR-069) has a credential for that one thread: made at the start of the thread, kept in the memory of the runner, given to the program in a file of the user with mode 0600 or over an inherited pipe, never in an argument and never in a variable that a different thread gets. It ends with the thread. With it an agent can use only the tools of its own tree.
- **THR-128.** The channels of the runner and of the holder are Unix socket files in a folder of the user with mode 0700; there is no port. The runner and the holder check the user of the peer; the client checks the owner of the file before it connects. The runner does not start in a system unit (as AIS-024). The channel keeps out a different user of the machine, a web page and a party on the network; it does not keep out a program of the same user (OD-THR18), and the text of THR-113 does not claim that.
- **THR-129.** The switch "Threads may start here" and the list of folders (TH33) are checked by the daemon of the machine, at the claim of an item and again at the start of the program. A folder of a claim is made real (links and `..` resolved) and must then be below a folder of the list; a claim for a different folder gets the refusal `folder_not_permitted` and a line in the log. When the switch goes off, no new start and no new prompt is taken; a thread that runs is stopped or not, by a setting of the person (the default: it ends its turn and takes no more). A clone that the daemon makes (OD-THR29) goes below a folder of the runner, which is in the list by itself. The settings are in the settings file of the daemon of the user; a program of the same user can change them, so an agent with full access that the hub started on one machine could turn the switch on at a different machine that it can reach with the login of the person. This is the consequence of OD-THR18 and OD-THR20 together.
- **THR-130.** The content of threads on the hub (R-292, OD-THR4) is kept for a time that the person sets (OD-THR44), is deleted with its thread and with its device, and is refused to an API key (THR-029). It is in each copy and each backup of the hub. No entry of a thread goes to a log of the hub or of its database (LOG-007 covers the entries of a thread).

## 8. Security: each way this gives power over a machine

### 8.0 The powers and the guards that are in force after version 3

The table of this section below (P1 to P21) is the first form, of versions 1 and 2: some of its rows name a guard that an answer of the operator withdrew (a signature of a client, a window of the operating system, a fixed environment, the settings of a project off, task W7 before stage 3). This table says what is in force. "Accepted" is a row of Section 8.2.

| Power | The guard in force | State |
| --- | --- | --- |
| P1, P3, P17: the hub starts an agent on a machine and names its prompt | The switch "Threads may start here" and the list of folders at the machine (THR-129); the log at the machine | Accepted |
| P2, P4: an agent runs commands; text that it reads steers it | None in Full access, which is the default; the approvals of the program in the other modes | Accepted |
| P5: the hub answers an approval | One answer for one live request (THR-030) | Accepted |
| P6: the environment of the agent | No value of Anima in it (THR-040) | Accepted for the environment of the person; THR-040 to prove |
| P7: the login of the vendor program | Claude: the program reads its own login; Anima reads none. Codex: the closed list of THR-021; no token in Anima | Accepted for the account methods; OD-THR40, OD-THR43 open |
| P8: the content of threads | Kept on the hub in plain form, for a device whose switch is on (THR-126, THR-130) | Accepted; OD-THR44 open |
| P9: the main web view | Text is drawn as text, with a test (THR-125); the shell answers the main web view only | Open until the review of the page |
| P10, P14: a different user; a program of the same user | The modes of the files and the sockets against a different user (THR-128). None against the same user | Accepted for the same user |
| P11: titles to an MCP tool and to the hub | Cleaned and capped | Accepted |
| P12, P20: `git` of the runner; what cannot be undone | THR-050, THR-097, THR-109 | Open until stage 4 |
| P13: a thread stays | The holder (THR-113), the idle stop and the end of each process (THR-049) | Open until the tests of stage 1b |
| P15: a forged session file | The runner resumes only its own threads (THR-047) | Accepted for the read side |
| P16: an agent starts agents | The limits as a question to the person (THR-114); the credential of one thread (THR-127); the limit on messages (THR-110) | Accepted |
| P18, P21: a machine that costs money; a larger plan | The cap of the own cloud account, and the allowance of the pool, kept by the pool (THR-107); a refusal past it unless he turned more on (OD-THR27); a purchase only on the account site (THR-108) | Open until stage 8; OD-THR38 |
| P19: a machine of the pool | It may reach his devices (accepted); the snapshot and the removal by the pool (THR-109, THR-112) | Accepted for the reach; the rest open until stage 8 |


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
| P16 | An agent starts other agents (the tools of THR-069) | The agent of a thread; text that steers it; a child against its parent, by a report that steers | The tools are given to the program of one thread and bound to its tree by the runner (a rule for the tools; a child with full access can also use the command and the socket of the person, P14); messages are not trusted and are limited (THR-110); a child has no more reach than its parent (THR-070); limits of depth, children and threads (THR-071); each start is recorded with the thread that made it (THR-072) | Open: with Full access by default a child is full access too. The limits stop growth, not harm. OD-THR8, OD-THR25 | 7 |
| P17 | An agent places work on a different machine | The agent of a thread; a hub controller | As P1 and P3: the claim of the daemon, the switch at the machine, the folder list, and the answer to OD-THR20. The agent names a place only among the machines that its parent may use (THR-070) | Open: OD-THR20, OD-THR35. Until the grant of THR-105 exists, an agent starts threads only on the machine where it runs (a requirement, THR-105) | 7 |
| P18 | An agent starts a machine that costs money | The agent of a thread; text that steers it; a hub controller | The pool checks the account and the allowance before each start, and removes the machine (THR-078, THR-081); the agent cannot pass the allowance; at the limit the person decides (THR-080) | Open: text that steers an agent can use up the whole allowance of a period. OD-THR27. A hub controller could ask the pool for machines of the account: the pool takes a request only with the grant of THR-105 and reserves it (THR-107). Until the grant exists, no agent starts a machine of the pool | 8 |
| P19 | A machine of the pool holds the code, and maybe a key, of the person | Anima.ing as the owner of the pool; the next account that gets the hardware | The disk is deleted at the removal (THR-081); no login is copied (THR-083); an API key for this use only | Open: the person trusts Anima.ing with the code that runs there, as with each rented machine. OD-THR26, OD-THR28 | 8 |
| P20 | An action that cannot be undone | The runner and the pool, by a command of an agent, of the hub or of the person | THR-109: each such action of Anima is made possible to undo, or needs the person | Open: what an agent does with a tool of the machine in Full access is not stopped (accepted by THR-065, said at the mode picker); OD-THR37 | 4, 8 |
| P21 | A larger plan is bought | Text that steers an agent; a program of the same user | THR-108: only on the account site, with a sign-in check; no tool accepts an offer | Open until the account site has the page | 8 |

The rows P2, P4 and P6 were written before THR-065. With the recommendations of OD-THR6, OD-THR7 and OD-THR21 as they are now, their guards (the approvals of the program, a fixed environment, the settings of the project off) hold only when the person chose the other option. The second read says which requirement still relies on each (the record of Section 9).

### 8.1 What the separate review found

Section 9 holds the findings of the separate reviewer and the change of the text for each one that needs no decision.

### 8.2 What the operator accepted, with his words (version 3)

Each row is a risk that a security read named, which the operator was shown and accepted on 2026-11-20. "Accepted" means: the weakness stays, by his decision. The last column is the guard that is still built.

| Risk | Entries | What he was shown, and his answer | Ruling | The guard that is left |
| --- | --- | --- | --- | --- |
| The hosted hub holds the prompts, the code and each secret that an agent prints | P8; F3, F17; G9 | Shown: "Your hub is hosted on the pool server, so your code, prompts and any secret an agent prints would sit on that server, readable by whoever controls it. Confirm?" His answer: "Yes, plain on the hub", in place of "Hub, but encrypted" and "Device after all" | R-292, OD-THR4 | The switch at the machine, off by default (OD-THR19); the redaction of known forms (THR-043); no entry in a log (LOG-007) |
| A party that controls the hub can answer an approval of a thread on a machine | P5; F6 | Shown: "whoever controls the hub can approve too". His answer: "Through the hub" | R-292, OD-THR5 | THR-030: an answer fits one open request of one live process; an audit event and the log at the machine (THR-044) |
| A party that controls the hub can start an agent with full access on a machine and name its prompt | P1, P3, P17; F4; G1, G2, G7 | Shown: "A taken-over hub could then start a full-access agent on your machines". His answers: "No, with the guards only" and "Trust the hub, keep the guards" | R-292, OD-THR12, OD-THR20, OD-THR35 | The switch "Threads may start here" at each machine, off by default, and its list of folders (TH33); the log at the machine (THR-044) |
| A program of the same user, also an agent that text steered, can do what the person can do: answer an approval, start a thread, change a setting of the runner, use the login token | P14; F1, F5, F9; G10, G14 | His words: "Accept it across any device. If my other desicons are contridicarty to this I want them changed." | R-292, OD-THR18 | None against the same user. Against a different user of the machine: the modes of the files and the socket (THR-045) |
| Each new thread has full access and the environment of the person; the settings of a project run with no question | P2, P4, P6; F2; G15, G16 | His answers: "Full access", "Your login's environment", "Yes, with a notice"; and earlier: "maximum agent reach is a good thing" | R-290, OD-THR6, OD-THR7, OD-THR21 | No secret of Anima reaches the agent (THR-040); the notice on the thread; the approvals of the program when the person chooses a different mode |
| An agent starts other agents with no question | P16; F10; G8 | His answer: "Yes, within limits"; the limits are guidance (THR-114) | R-290, OD-THR8, OD-THR25 | The limits as a question to the person; the limit on messages (THR-110); each start is recorded (THR-072) |
| A machine of the pool can reach each device of the person | G3 | His answer: "Yes, like any device" | R-294, OD-THR36 | It claims only work of its tree and gets no secret that was not named for it (THR-106) |
| The title of a thread, which can be the first line of a prompt, leaves the computer | P11; F11 | His answer: "Yes, cleaned and capped" | R-296, OD-THR13 | Control characters removed, 80 characters at most; the name of the folder and not its path; no branch |
| Anima uses the account methods of Codex, which the vendor may not permit for a hosted product | P7; F18 | His words: "No we can use account methods everyone else is doing it" | R-290, OD-THR3 | The tokens stay in the store of Codex on the machine in the first build (THR-115); OD-THR40 |

Not accepted, and still to close before the code of its stage merges: the gates of THR-085 to THR-096 (a right state, no question answered by a timer, a reconnect that harms no turn); the allowance of machines (THR-107; G4); the place where a plan is bought (THR-108; G5); what cannot be undone (THR-109; G6); the removal of a machine (THR-112; G13); the `git` of the runner (THR-050; F15); the limits of a thread and the end of its processes (THR-049; F14).

## 9. Reviews

The session that wrote this proposal gave a separate reviewer agent this document, the documents it stands on and the positions of an attacker to take, and not its own view. The report is the record [agent-threads-review-2026-11-20.md](../research/agent-threads/agent-threads-review-2026-11-20.md). Its findings F1 to F28 are the entries of [security-findings.md](../security-findings.md) in the table below.

The reviewer found 28 weaknesses: 8 of severity High, 13 Medium and 7 Low. Seven of the eight High findings are about the drive side (stages 1b to 3), of which no code exists; one (F8) and six of the lower ones are about the read side, and those that are a fault of the code are corrected on the branch `threads/read-side`. The largest: nothing in Anima separates the person from a program of the same user, so "only a person answers an approval" had no mechanism (F1, F5); the relay "without a copy" and the key of W7 do not protect a machine against a person who controls the hub (F3, F4, F6); and the approvals of the programs may be answered by settings of a project before Anima is asked (F2, not verified).

| # | Severity | Finding | Entry | What was done |
| --- | --- | --- | --- | --- |
| F1 | High | "Only a person answers an approval" has no mechanism: an agent of the same user reaches the socket, the command and the login token | SF-147 | OD-THR18. THR-031 now names the mechanism that is asked for |
| F2 | High | Settings of the project or the user of the agent program (allow rules, hooks, MCP servers, the policy of Codex) can answer before the approval tool is asked (not verified) | SF-148 | TH10 measures it; THR-046; OD-THR21 |
| F3 | High | "Relay without a copy" is a promise of the hub: a hub controller can pull each thread of each device that is online | SF-149 | THR-012 corrected; OD-THR19 (a switch at the machine) |
| F4 | High | W7 proves the device to the hub, not the hub to the machine; a prompt of the hub is a command that the hub names, with no isolation | SF-150 | P1 and P3 corrected; OD-THR20 |
| F5 | High | The daemon token and the login token of the command line are in reach of an agent of the same user; in stage 3 one poisoned file reaches each machine | SF-151 | P10 row P14 added; OD-THR20 |
| F6 | High | THR-034 names a key of a client that no plan owns, and does not say what is signed | SF-152 | THR-034 now says what is signed; the key is OD-THR5 |
| F7 | High | THR-029 bans only MCP tools; an API key and the `anima` command can read entries | SF-153 | THR-029 widened |
| F8 | High | The main web view is a new reader with open `https:` egress, and later would send prompts and answer approvals; P9 was too kind | SF-154 | P9 corrected; THR-014 (text only, with a test); THR-048 |
| F9 | Medium | The switch, the folder list and the mode can be written by the user of the agent; TH33 misread USE-004; nothing stops the hub from sending Full access | SF-155 | TH33 and THR-033 corrected; OD-THR18 |
| F10 | Medium | The recommendation of OD-THR8 conflicts with MCP-006 and AGL-003; change tools are `allow` by default | SF-156 | OD-THR8 recommendation changed to A |
| F11 | Medium | The built MCP tool gave the full title, the path of the folder and the branch, with no exclusion | SF-157 | Corrected in the code (the tool gives a title only when given, the folder name, no branch); THR-018; OD-THR13 |
| F12 | Medium | Nobody is named as the holder of the setting of option A of the event log | SF-158 | OD-THR4 corrected: the setting is in the configuration file of a hub on a computer of the operator, never a setting that a hosted hub holds |
| F13 | Medium | The long poll of THR-026 is not written against K10 and K8 | SF-159 | THR-026 marked; TH30 owns it |
| F14 | Medium | The idle stop and the life of a thread have no limits, no containment of child processes, no rule for a restart of the daemon | SF-160 | THR-049 |
| F15 | Medium | The `git` of the runner runs hooks and configuration that the agent can write; checkpoints take untracked secrets; worktree names come from the prompt | SF-161 | THR-050 |
| F16 | Medium | The label of the source of a prompt and the audit rest on the hub; no log at the machine | SF-162 | THR-044 corrected |
| F17 | Medium | The event log is a second copy with no retention, deletion or limit; redaction is best effort | SF-163 | THR-043 corrected; OD-THR22 |
| F18 | Medium | Wider conflicts with BOT-020, BOT-032, BOT-033 and AXT-045: a subscription login driven through a hosted hub | SF-164 | Section 11; OD-THR2 and OD-THR3 now say it |
| F19 | Medium | Gaps of THR-040: no deny list, the source of `PATH`, `SSH_AUTH_SOCK`, a prompt in the process list | SF-165 | THR-040 corrected |
| F20 | Medium | A forged session file shows as a thread, and a resume would load its history; the ID of a Codex row came from the content | SF-166 | Corrected in the code (the ID of a row is the ID that opens it); THR-047 |
| F21 | Medium | No rule to fail closed for an unknown event, kind of request or version of the program | SF-167 | THR-046 |
| F22 | Low | Limits missing in the code: the summary title, the patch, the whole thread, the line of a glance | SF-168 | Corrected in the code (THR-004, THR-006) |
| F23 | Low | The file reader followed a link at the root folders and checked no owner and no hard link | SF-169 | Corrected in the code for the root folders, the owner and the link count (THR-007). Open: the open is by path, so a folder can change between the list and the open |
| F24 | Low | The reader can show what did not happen: the output wrapper of Codex, the rule for typed text, no end of a turn for Claude Code | SF-170 | The wrapper is corrected in the code. THR-014 says what Claude Code does not give. Open: a line that the program wrote without a tag shows as a prompt |
| F25 | Low | TH4 was marked built, but the page was not on the branch at the time of the read | SF-171 | The page is on the branch now; no separate reviewer read it (Section 12) |
| F26 | Low | Rows of Section 8 that were wrong or too kind (P2, P3, P10, P13), and rows that were missing | SF-172 | Section 8 corrected |
| F27 | Low | Open decisions and conflicts that were missing | SF-173 | OD-THR18 to OD-THR22; Section 11 |
| F28 | Low | Members about the account and its limits in the streams of the programs conflict with AIS-022 (not verified) | SF-174 | THR-051 |

What the reviewer checked and found sound in stage 1a: no call to the hub and no start of a process; the cleaning of text for a terminal in the text, table and JSON forms; the handling of a thread ID; `O_NOFOLLOW` at the last part of a path; the MCP tool gives no entry; the shell commands answer the main web view only and give no path and no text in an error.

No separate reviewer read the corrections, the page Threads of the branch, or Section 4.10.


### 9.1 The second read: the parts of version 2

A second separate reviewer read Sections 1.2, 4.11 and 6.8, the powers P16 to P19 and the open decisions that changed, with the frame of THR-065: it did not argue for less reach of the agent of the owner; it looked for the ways by which a different party uses that reach, and for a cost or a loss that cannot be undone. Its report is the record [agent-threads-review-2-2026-11-20.md](../research/agent-threads/agent-threads-review-2-2026-11-20.md). It found 19 weaknesses: 6 High, 11 Medium, 2 Low. No code of these parts exists.

The largest: a child that is started on a different machine, and a machine of the pool, have no person at a client and no person at the machine, so each guard there is set by the hub and the pool (G1, G2); a machine of the pool as "a device of the account" gets a path to the other devices of the owner (G3); "inside the allowance" could not be checked at a start (G4); the offer of a larger plan could be caused and answered by a program (G5); and the hard stops of THR-065 were enforced nowhere (G6). One closure comes back in five findings: a grant that a client signs at the start of a root thread, which each runner and the pool check, so that the hub only carries bytes (THR-105). It needs keys of clients, which no plan owns.

| # | Severity | Finding | Entry | What was done |
| --- | --- | --- | --- | --- |
| G1 | High | A start of a child on a different machine has no signer: no person is at a client, and the rule that a child has no more than its parent is kept only by the hub | SF-180 | THR-105; OD-THR20, OD-THR35 |
| G2 | High | A machine of the pool has no person at the machine, so each guard at the machine (the switch, the folder list, the trust question) is set by the pool and the hub | SF-181 | THR-105, THR-106; OD-THR35 |
| G3 | High | "A device of the account" gives a machine of the pool a path to the devices of the owner, a daemon token that claims work, and a place in "any machine" | SF-182 | THR-106; OD-THR36 |
| G4 | High | "Inside the allowance" cannot be checked at the start: the hours are not known, parallel requests race, no cap on machines or size, no end | SF-183 | THR-107; OD-THR27 |
| G5 | High | The offer of a larger plan can be caused by text that steers an agent and answered by a program; the card rests on figures of the hub | SF-184 | THR-108 |
| G6 | High | The hard stops for what cannot be undone were named in THR-065 and enforced nowhere; THR-081 itself deletes a disk with no question | SF-185 | THR-109; OD-THR37 |
| G7 | Medium | Who may ask the pool for a machine was a note in P18, not a requirement; a call from the hub to the pool is against CVM-015 | SF-186 | THR-105; Section 11; OD-THR35 |
| G8 | Medium | Reports and messages are text that steers, and they go up to the machine that is trusted most; no limit on messages or turns | SF-187 | THR-110 |
| G9 | Medium | The tools of a tree give content of a thread, against THR-029; briefs and reports go through the hosted hub | SF-188 | THR-110; OD-THR19 |
| G10 | Medium | The binding of the tools to a tree binds the tools only; a child with full access can use the command, the socket or the login token | SF-189 | P14, P16 corrected; OD-THR18 |
| G11 | Medium | The placement is not written against the routing rules of jobs.md; THR-049 and THR-071 disagree; OD-THR25 changes BOT-032 without saying so | SF-190 | THR-049 and THR-074 corrected; Section 11 |
| G12 | Medium | Each option of OD-THR28 was against a rule or against THR-079; an API key is a bill beyond the plan | SF-191 | THR-111; OD-THR28 changed |
| G13 | Medium | The removal rested on the word of the hub; more stays than the disk; the kind of isolation of a machine of the pool was not said | SF-192 | THR-112 |
| G14 | Medium | Where the settings "ask first" and the limits are kept was not said; the hub or a steered agent could loosen them | SF-193 | THR-112; OD-THR18 |
| G15 | Medium | The turned OD-THR7 had no condition for a prompt from the hub; the environment of a child on a different machine was not defined | SF-194 | OD-THR7 changed |
| G16 | Medium | The one stop of the turned OD-THR21 (trust a folder one time) does not fit a worktree, a fresh clone or a machine with no person | SF-195 | OD-THR21 changed |
| G17 | Medium | Stage 8 disagreed with cloud-machines.md and HMD-073, and Section 11 did not list it | SF-196 | Section 2 and Section 11 corrected; TH90 |
| G18 | Low | The turned text of OD-THR8 was in the wrong table; some rows of Section 8 were stale | SF-197 | Corrected |
| G19 | Low | A side chat with a different program or model sends the whole conversation to a second vendor | SF-198 | THR-067 corrected |

For each of the four turned recommendations the record says which guard goes away and what would make the new recommendation safe. In short: OD-THR6 and OD-THR7 need the grant of THR-105 for each start and prompt that comes through the hub; OD-THR8 needs the limits of THR-071 and THR-110, and its binding is not a wall against a child with full access; OD-THR21 makes "Supervised" not provable, so the trust must be for a repository and its settings files. The rows of Section 10 now say this.

No reviewer read the corrections of this second read, Section 4.12, or the page Threads of the branches.

### 9.2 The third read: version 3 against the answers of the operator

A third separate reviewer read version 3 with the answers of the operator as given facts: it did not argue against an answer, and it did not raise again a finding that Section 8.2 marks as accepted. It checked that the text says what he accepted and nothing wider, and that each guard that is left can be built and tested. Its report is the record [agent-threads-review-3-2026-11-20.md](../research/agent-threads/agent-threads-review-3-2026-11-20.md), which also has a table of the guards with the test for each. It found 24 weaknesses: 4 High, 18 Medium, 2 Low. It read no code of the branches.

| # | Severity | Stage | Finding | Entry | What was done |
| --- | --- | --- | --- | --- | --- |
| H1 | High | 1b | A stop by one process group leaves commands alive | SF-199 | THR-049 in full; checklist 7 |
| H2 | High | 1b | With THR-048 withdrawn, "text is drawn as text" is the only wall between a remote author and a command with full access through the main web view, and it was not named as a guard | SF-200 | THR-125; checklist 14 |
| H3 | High | 1b, 2 | The text did not say what leaves the computer in stage 1b, mixed two switches, and did not say if a title goes with the switch off | SF-201 | THR-126; OD-THR42 |
| H4 | High | 1b, 3 | The account methods of Codex had no closed list, the sign-in was not held to the machine, and nothing said if the hub gets the account, the plan and the limits | SF-202 | THR-021 in full; OD-THR43 |
| H5 | Medium | 1b | Sentences that bind still said the opposite of an answer (THR-065, THR-021, THR-051, THR-031, THR-033, THR-040, THR-046) | SF-203 | Each is written in full in its form of version 3 |
| H6 | Medium | 1b to 8 | Rows of Section 8 still claimed a guard that is withdrawn | SF-204 | Section 8.0 |
| H7 | Medium | 1b | THR-049 said that no program runs after a restart, against the holder; the ID of a request after a restart of the daemon was not defined | SF-205 | THR-049, THR-030 in full |
| H8 | Medium | 1b | "The holder takes commands only from the daemon" cannot hold against a program of the same user | SF-206 | THR-113 in full |
| H9 | Medium | 1b | The end of the holder, its update, the versions of its protocol and agents with no holder were not written | SF-207 | THR-113 in full |
| H10 | Medium | 1b | "No byte is lost" had no limit of size, and the kept output is one more copy | SF-208 | THR-113 in full |
| H11 | Medium | 1b | THR-040 named variables only: descriptors, the environment of the holder, the working folder and a secret by its value were missing | SF-209 | THR-040 in full |
| H12 | Medium | 1b, 7 | The credential of the MCP server of a thread was not defined | SF-210 | THR-127 |
| H13 | Medium | 1b, 7 | The soft limits did not say who counts or what a tool gets at the limit; the start tool came before the limits | SF-211 | THR-114 note; TH13 |
| H14 | Medium | 1b | "Allow for the session" had no rule; "a missing answer is a decline" was against THR-087 | SF-212 | THR-030, THR-046 in full |
| H15 | Medium | 1b, 3 | The log at the machine "that the hub cannot change" claimed too much | SF-213 | THR-044 in full |
| H16 | Medium | 1b | The socket was not said to be a file; the client did not check its owner; the runner was not forbidden in a system unit | SF-214 | THR-128 |
| H17 | Medium | 1b | Full access, the default, was not measured in TH10 | SF-215 | Task TH18 |
| H18 | Medium | 1b, 3 | The values of the arguments had no rule | SF-216 | THR-040 in full |
| H19 | Medium | 3 | TH33 was one line | SF-217 | THR-129 |
| H20 | Medium | 3, 7 | One machine with the switch on can, through an agent, turn the switch on at each machine that it reaches | SF-218 | Written as a consequence in Section 1.3; accepted with OD-THR18 |
| H21 | Medium | 2 | No rule for how long the hub keeps the content of threads, for its delete, its copies and the log of the database | SF-219 | THR-130; OD-THR44 |
| H22 | Medium | 2 | The redaction covers tool outputs only, and its clause of known secrets is empty now | SF-220 | THR-043 note |
| H23 | Low | 1b to 6 | Task rows that were stale; `approve` had no task in stage 1b | SF-221 | Corrected |
| H24 | Low | 1a to 3 | Section 11 missed MCP-004, AIS-018, BOT-066, BOT-032, LOG-007 | SF-222 | Section 11 |

Its list "before the code of stage 1b merges" is Section 6.2.1. No reviewer read the corrections of this third read.

## 10. Open decisions

Each row is one question for the operator with the options, the strength and the weakness, and a recommendation.

| ID | Question | Options | Recommendation |
| --- | --- | --- | --- |
| OD-THR1 | Who starts the agent program on this computer in stage 1b? | (A) The desktop application, as a child. Fast to build; a thread stops when the window closes. (B) The thread runner in `anima-daemon`. Threads stay; needs the local socket (TH12) and more time. | **Decided** (R-290, the answer that he selected: "The daemon (Recommended)"): the thread runner is a part of `anima-daemon`. The recommendation before the answer: B, with A behind the same interface only if B is not ready before the launch |
| OD-THR2 | Do you permit the Claude Code adapter to drive the `claude` program in stream JSON, with its approvals over the same pipe (`--permission-prompt-tool stdio`), with the login of the machine? | (A) Yes: it is the `claude` program that BOT-032 permits, and no SDK. Measured (TH10): it gives each function that was tested, which is what T3 Code gets from the SDK. Weakness: the word `stdio` is not in the help of the program, so the vendor can change it; the fallback is an MCP tool of the runner. (B) No: only an API key for Claude. | **Decided** (R-290, the answer that he selected: "Yes (Recommended)"): the Claude Code adapter drives the `claude` program in stream JSON, with its approvals over the same pipe, with the login of the machine. The recommendation before the answer: A. Know that from stage 3 this is a subscription login that is driven by a prompt that came through a hosted hub; the model traffic does not go through the hub, only the prompt does. The vendor terms row (BOT-038) is needed before stage 3 |
| OD-THR3 | Codex: `codex exec` only (BOT-033), or `codex app-server` without account methods? | (A) `exec` and `exec resume`: inside BOT-033. Measured (TH10): one turn, no approvals, no questions, no deltas, no steer; not a thread. (B) `app-server` with the thread, turn and item methods only, on a device where the person signed in: measured, it gives the thread of T3 Code; it changes BOT-033. | **Decided** (R-290, his own words: "No we can use account methods everyone else is doing it"): the Codex adapter uses `codex app-server` with the account methods. The ban of BOT-033 on them is lifted by this ruling. Section 4.4 (THR-115) says what the account methods do and what this opens. The recommendation before the answer: B, with a ruling that changes BOT-033 for the thread, turn and item methods, and keeps the ban on the account methods. Without B there is no Codex thread with approvals |
| OD-THR4 | Where is the event log of a thread? | (A) The hub keeps all entries (PostgreSQL). Offline devices can be read; the hosted hub holds code and secrets. (B) The device keeps the entries; the hub keeps the rows and relays an open thread without a copy. (C) As B, with encrypted copies on the hub (K-E5). | **Decided** (R-292, the answer that he selected: "On the hub"): the entries of the threads are kept on the hub, in plain form ("Yes, plain on the hub"). Accepted risk; Section 8.2. The recommendation before the answer: B on the hosted hub; A only for a hub on your own computer, set in the configuration file of that hub at that computer, never a setting that a hosted hub holds |
| OD-THR5 | How is an approval of a thread on a different machine answered before task W7? | (A) Through the hub, with the login token. A hub controller can approve. (B) Only at the machine, until W7 gives a key of the client. (C) No remote threads before W7. | **Decided** (R-292, the answer that he selected: "Through the hub"): an approval of a thread on a different machine is answered through the hub with the login, before task W7. Accepted risk; Section 8.2. The recommendation before the answer: B |
| OD-THR6 | What is the default mode of a new thread? | (A) Full access, as T3 Code. Strength: the agent is not stopped, which is what the tool is for (THR-065). Weakness: Claude Code has no sandbox, so text that the agent reads can run a command with no question; and with (A) a thread that the hub starts on a machine is full access too, so OD-THR20 becomes the only guard of a machine. (B) Supervised for a thread that the hub or an agent starts, and the last choice of the person for a thread that the person starts. | **Decided** (R-290, the answer that he selected: "Full access (Recommended)"): the default mode of a new thread is Full access. With OD-THR20 (no keys of clients) this is also the mode of a thread that is started through the hub. The recommendation before the answer: A, by "maximum agent reach is a good thing" (R-289), with one condition: a start that comes through the hub is full access only when OD-THR20 is answered with a signature of a client. Before version 2 the recommendation was B |
| OD-THR7 | What does the environment of the agent program hold? | (A) The environment of the login of the person, as T3 Code and as a terminal give it. Strength: each tool of the person works (keys of a cloud, `SSH_AUTH_SOCK`, version managers). Weakness: the agent, and text that steers it, can use each of them. (B) A fixed allow list and the variables that the person names. | **Decided** (R-290, the answer that he selected: "Your login's environment (Recommended)"): the agent gets the environment of the login of the person, and never a secret of Anima itself. The recommendation before the answer: A, by THR-065, without the values of Anima itself (the daemon token, the link socket, a key of Anima: these are never passed, in each option), and with the same condition as OD-THR6: a thread that is started or prompted through the hub gets the environment of the person only with the grant of THR-105; without it, B. A child on a different machine gets the environment of the person on that machine, never one that is sent. Before version 2 the recommendation was B |
| OD-THR8 | May an AI agent start a thread or send a prompt through a tool of Anima? | (A) No. (B) Yes, as a change tool in state `ask`. (C) Yes, with no question: the agent of a thread gets the tools of THR-069 for its own tree, inside the limits of THR-071. Strength: orchestration needs it (R-297). Weakness: it changes MCP-006 and AGL-003; one poisoned file can start threads up to the limits; and a child with full access can also use the command and the socket of the person, so the binding to a tree is a rule for the tools, not a wall (OD-THR18). | **Decided** (R-290, the answer that he selected: "Yes, within limits (Recommended)"): the agent of a thread may start threads and send prompts in its own tree with no question, inside the limits (which are soft: OD-THR25). The recommendation before the answer: C for the agent of a thread and its own tree, on the machine where it runs. For a different machine and for a machine of the pool: only with the grant of THR-105. A tool of the general `anima mcp` server, by which any agent on the machine starts a thread, stays B. Before version 2 the recommendation was A |
| OD-THR9 | May the daemon send the session ID with the counts of AIS-018, so that Agent usage shows the use of one thread? | (A) No: no use by thread. (B) Yes: the ID only, no text. | **Decided** (R-296, the answer that he selected: "Yes, ID only (Recommended)"): the daemon sends the session ID with the counts, and no text. The recommendation before the answer: B |
| OD-THR10 | Where is the page Threads? | (A) An item Threads in the navigation bar, above Agent usage. (B) Inside Projects. | Decided (R-287, operator, 2026-11-20, on a question card: "Own nav item"): A |
| OD-THR11 | What happens to the T3 Code entry of the catalogue of agent tools (AXT-020, AXT-045)? | (A) Keep it for a person who wants T3 Code itself; not offered by default after stage 1b. (B) Withdraw it. | **Decided** (R-296, the answer that he selected: "Keep, not by default (Recommended)"): the T3 Code entry of the catalogue stays and is not offered by default. The recommendation before the answer: A |
| OD-THR12 | Does stage 3 (drive the agents of other machines) wait for task W7? | (A) Yes. (B) No, with the guards of P1 and OD-THR5 (B) only. | **Decided** (R-292, the answer that he selected: "No, with the guards only"): stage 3 does not wait for task W7. Accepted risk; Section 8.2. The recommendation before the answer: A, the same condition as R-186 |
| OD-THR13 | May the title of a thread (the first line of a prompt) leave the computer in stage 2 and go to an MCP tool? | (A) Yes, cleaned and capped. (B) Only a title that the person or the program gave; else the folder name and the time. | **Decided** (R-296, the answer that he selected: "Yes, cleaned and capped"): the title of a thread may leave the computer, cleaned and capped, also when it is the start of a prompt. This was not the recommendation. Consequence: the first line of a prompt can name a customer or a secret, and it goes to the hosted hub, to a notification and to the result of an MCP tool, which goes to the vendor of the agent that called it. The recommendation before the answer: B for the hub and the MCP tool; the full title stays in the client of the same computer |
| OD-THR15 | A thread in a folder that the shared project folder syncs (AXT-054 to AXT-078): where do its worktrees go? | (A) Worktrees of a thread are made outside each synced folder, under a folder of the runner, and `.git` of a synced project is not synced. (B) No worktree for a thread in a synced folder: such a thread works in the folder. (C) Decide when stage 4 is planned. | **Decided** (R-296, the answer that he selected: "Outside the synced folder (Recommended)"): the worktrees of a thread are made outside each synced folder. The recommendation before the answer: A, written into AXT-056 when stage 4 is planned; until then C costs nothing |
| OD-THR16 | AXT-049 says that the list of agents never shows a transcript. May the thread view show the entries of a thread of a different device (stage 2)? | (A) Yes, to the signed-in operator only, relayed and not kept (OD-THR4 B). (B) No: other devices show rows only; a thread is read on its own computer. | **Decided** (R-292, the answer that he selected: "Yes, relayed, not kept (Recommended)"): the thread view shows the entries of a thread of a different device. "Not kept" is changed by OD-THR4: the hub keeps them. The recommendation before the answer: A, with AXT-049 changed to say that the list shows none and the thread view does |
| OD-THR17 | Is the owner of this function its own document and row 4.24, or a section of agent-tools-as-extensions.md under row 4.21, as the other new agent functions? | (A) Its own document and row. (B) A section 6.11 of that document. | **Decided** (R-296, the answer that he selected: "Own document and row (Recommended)"): this proposal is its own document, with row 4.24. The recommendation before the answer: A: a different time, a different kind of work, and the goal sentence of that document says the opposite |
| OD-THR18 | A program of the same user can do what the person can do in Anima (read the login token, reach a local socket, write a settings file). How is "a person" proven for an approval, for Full access and for the switches of the runner? | (A) A window of the operating system from the desktop shell for each of them; no `anima agents approve` command. An agent that can drive the screen is not stopped. (B) As A, and the settings of the runner in a file that only an administrator can write. (C) Accept that an agent of the same user can approve; say so on the screen. | **Decided** (R-292, his own words: "Accept it across any device. If my other desicons are contridicarty to this I want them changed. The"): a program of the same user can do what the person can do in Anima, also answer an approval, on each device; no proof of a person is asked for. Section 1.4 lists each text that this changes. His sentence ends at "The": the rest did not reach the session that records it. The recommendation before the answer: A for stage 1b; B before stage 3 |
| OD-THR19 | May a device give the entries of its threads to the hub (stage 2)? | (A) Only when a switch at the machine is on; off by default; the hub cannot change it. (B) Always, for a signed-in operator. (C) Never: other devices show rows only (as OD-THR16 B). | **Decided** (R-292, the answer that he selected: "Yes, off by default (Recommended)"): a device gives the entries of its threads to the hub only when a switch at the machine is on; it is off by default. The recommendation before the answer: A |
| OD-THR20 | Stage 3: a person who controls the hosted hub could start a thread and name its prompt on a machine, also with W7. What do you require? | (A) Each start and each prompt for a different machine carries a signature of a key of one of your clients, which the runner checks; needs a plan for such keys. (B) Accept the hub as trusted for this, with the switch at the machine and the folder list. (C) No stage 3 on a hosted hub; only a hub on your own computer. | **Decided** (R-292, the answer that he selected: "Trust the hub, keep the guards (Recommended)"): the hub is trusted for a start and a prompt on a different machine; the guards are the switch at the machine and the list of folders. No signature of a client. The recommendation before the answer: A; until the keys exist, C |
| OD-THR21 | The settings of a project folder of an agent program (allow rules, hooks, MCP servers) may run things with no approval. Does the runner honour them? | (A) No: the runner starts the program with the settings of the project off, where the program permits it. (B) Yes, as the program does in a terminal, with a notice on the thread. Strength: the thread behaves as the person's own Claude Code and Codex do (THR-091). Weakness: a repository of a different person can bring a hook that runs at the start; and with B the runner cannot prove that a thread is Supervised (THR-046), so "ask first" is a wish, not a guard. | **Decided** (R-290, the answer that he selected: "Yes, with a notice (Recommended)"): the settings of a project are honoured, with a notice on the thread. The recommendation before the answer: B, by THR-065, with the notice. The trust is for a repository and the content of its settings files (their hash), given by the person one time at a client; it covers each worktree and clone of that repository, and it ends when those files change. A repository that the person did not trust runs with the settings of the project off. A machine with no person (the pool) uses only a trust that the grant of THR-105 names. Before version 2 the recommendation was A for a thread of the hub |
| OD-THR22 | How long does the runner keep the event log of a thread at the machine? | (A) Until the thread is deleted. (B) 30 days after the last entry. (C) Not at all: the view is made from the session file of the program each time. | **Decided** (R-296, the answer that he selected: "None where possible (Recommended)"): the runner keeps no event log where the session file of the program gives enough; else 30 days after the last entry. The recommendation before the answer: C where the session file gives enough; else B |
| OD-THR23 | On the phone Threads takes the place of Sessions in the bar (R-288). Where are the live remote sessions reached from then? The mobile stream owns the answer (OD-M11 of [mobile-plan.md](../plans/mobile-plan.md)). | (A) From the device: the page of a device lists and opens its sessions, and a mark in the head of the application shows a session that is live. (B) A section "Live sessions" at the top of the page Threads. (C) An entry in Settings. | **Decided** (R-296, the answer that he selected: "From the device (Recommended)"): on the phone the live remote sessions are reached from the page of a device. The mobile stream makes the change (OD-M11). The recommendation before the answer: A: a session belongs to a device, and it is where a person starts one |
| OD-THR24 | Your words put the threads that need you at the top and the threads that work at the bottom (R-293). Where do the threads go that are done, stopped or idle? | (A) Between the two, by their last change (as built). (B) Below the threads that work, so that the working threads are not at the very bottom. (C) In a folded group "Earlier". | **Decided** (R-296, the answer that he selected: "Folded group 'Earlier'"): the threads that are done, stopped or idle are in a group "Earlier" that is folded. Not as built: the list changes (THR-056). The recommendation before the answer: A: it keeps your two rules exact, and an old thread is the least urgent to see but still nearer than a scroll past everything |
| OD-THR25 | The limits of a tree of threads (THR-071): which defaults? | (A) Depth 3, 5 children for one thread, 10 threads of the account at one time. (B) No limits; only the usage limits of the vendor stop a tree. (C) Your numbers. | **Decided** (R-290, his own words: "i like the 3, 5, 10 as guidence but the user shouldn't be hard stopped it can say that you are going over the recomend guidence and are at risk of Cyberpsychosis by running this many session. Do you want to raise the limit? Then Bump it to 5, 10, 15 and ask again for the next step"): the limits 3, 5 and 10 are guidance. At a limit the person is warned and asked, and a yes raises the limits to 5, 10 and 15, and so on (THR-114). An agent with no person there waits at a limit until the person answers. The recommendation before the answer: A, as settings that you can raise |
| OD-THR26 | Whose machines does "spin up another server" start (R-295)? | (A) The pool of Anima.ing, inside the allowance of your plan. (B) Your own cloud account (cloud-machines.md), where the bill is yours and not a part of the plan. (C) Both, A first. | **Decided** (R-294, his own words: "own clouds first then anima pol"): a machine for an agent comes from the own cloud account of the person first, then from the pool of Anima.ing. This turns the order of the recommendation. The recommendation before the answer: C: A is the subscription that you named; B later for a person who wants it |
| OD-THR27 | What counts against the allowance, and what is a hard stop? | (A) Hours of machines by size; a request that passes the allowance is refused, with the three choices of THR-080. (B) As A, with a small amount over the allowance that is billed. | **Decided** (R-294, his own words: "This is something We'll have to opt in. By default, I would like a smaller build, but it should be refused unless they enable API building or automatic upgrading to the next plan."): a request that passes the allowance is refused by default. More is possible only for a person who turned on billing by use or an automatic change to the next plan. What "a smaller build" means is open (OD-THR38). The recommendation before the answer: A: no bill beyond the plan can come from text that steers an agent |
| OD-THR28 | How does a thread use a machine of the pool, and how does it get to Claude or Codex there (THR-111)? | (A) The agent program stays on a machine of yours, with your login there; only its commands run on the machine of the pool. No login and no key leaves your machines. (B) The agent program itself runs on the machine of the pool, with an API key that you set for this use; the bill of the model is then beside the plan. (C) You sign in on that machine each time. (D) Your login is copied there (BOT-020 and BOT-066 forbid it). | **Decided** (R-294, the answer that he selected: "Agent stays on your machine (Recommended)"): the agent program stays on a machine of the person; only its commands run on the machine that was started. The recommendation before the answer: A as the first form. B or C later, after the record TH96 and a ruling on BOT-066 |
| OD-THR29 | How does the folder of a project get to a different machine (THR-075)? | (A) The daemon clones the repository from the forge into a folder of the runner. (B) The shared project folder (AXT-054 to AXT-078). (C) Both. | **Decided** (R-296, his own words: "I like a shared sync folder but if it's not there then the a clone sets it up"): the folder of a project on a different machine is the shared project folder when it is there; when it is not, the daemon makes a clone. The recommendation before the answer: A first: it needs no sync and gives a clean worktree; B when that proposal is approved (OD-THR15) |
| OD-THR30 | Which of the five ideas of Section 4.11.4 become a part of this proposal now? | (A) `needs-you-inbox` and `agent-state-hooks`, which R-293 needs. (B) All five. (C) None yet. | **Decided** (R-296, the answer that he selected: "All five"): the five ideas of Section 4.11.4 are parts of this proposal now (THR-117 to THR-121); their folders are deleted (PLN-018). The recommendation before the answer: A; the other three when their stage comes |
| OD-THR31 | Do you want "the same prompt to several agents, then a thread that judges" (THR-103)? | (A) Yes, in stage 7. (B) No. The users' report ranks it low; your words on orchestration and reach favour it. | **Decided** (R-296, the answer that he selected: "Yes, in a late stage (Recommended)"): the same prompt to several agents with a thread that judges is built, in a late stage (TH82). The recommendation before the answer: A, after the tree works |
| OD-THR32 | A project with more than one repository (THR-101): which model? | (A) A project is a list of repositories; a thread gets one worktree for each repository that it names, in one folder of the thread. (B) A thread has one repository; a project only groups threads of several repositories. (C) Later. | **Decided** (R-296, the answer that he selected: "Several repos per thread now"): a thread can hold more than one repository from the start (model A), not model B first. Consequence: task TH41 makes one worktree for each repository of a thread, and the checkpoints, the diff and the pull request of a thread are for more than one repository. The recommendation before the answer: B first (it needs nothing new in the runner), A when stage 4 works; decide before TH41 |
| OD-THR33 | Functions of other tools that one research report judged not worth a copy: navigation in code and an editor, a board of cards, sandboxes of a vendor, voice, a browser in the application. Which do you want? | Each one yes or no. | **Decided** (R-296, the answer that he selected: "Code navigation and an editor; A board of cards; A browser in the app"): three functions of other tools are added: navigation in code with an editor (THR-122), a board of cards (THR-123), a browser in the application (THR-124). Not voice, and not sandboxes of a vendor. The recommendation before the answer: None now: Anima has a terminal, the preview of AXT-079 to AXT-088 and Projects; ask again after stage 4 |
| OD-THR34 | A small holder process that owns the agent processes, so that the daemon can update or stop while agents run (THR-095)? | (A) Yes, from stage 1b. (B) No: an update of the daemon waits until no thread runs, or stops them with a notice. | **Decided** (R-290, the answer that he selected: "Yes, from the start"): a holder process owns the processes of the agents from stage 1b (THR-113, task TH17). This was not the recommendation. Consequence: a second program that runs for a long time, to secure and to update, is a part of the launch. The recommendation before the answer: B for the launch, A after it: a second long-lived process is a second thing to secure and to update |
| OD-THR35 | The grant of THR-105 needs a key in each of your clients (the desktop application, the phone) that your machines and the pool know. Who plans it? | (A) A new task of the foundation, beside the key of a device (W7), before stage 3, 7 and 8. (B) No client keys: you accept the hosted hub as trusted for starts and prompts (OD-THR20 B). (C) No threads across machines on a hosted hub. | **Decided** (R-292, the answer that he selected: "Trust the hub, keep the guards (Recommended)"): no keys of clients are planned; THR-105 (the grant) is withdrawn. The recommendation before the answer: A. It is the one mechanism that five findings of the second read and three of the first ask for |
| OD-THR36 | May a machine of the pool open a connection to your own devices (THR-106)? | (A) No, by default; a setting opens it. (B) Yes, as each device of the account. | **Decided** (R-294, the answer that he selected: "Yes, like any device"): a machine of the pool may open a connection to the devices of the person, as each device of the account. This was not the recommendation. Accepted risk; Section 8.2. The recommendation before the answer: A |
| OD-THR37 | The disk of a machine of the pool at its removal (THR-109): | (A) A snapshot is kept for a set time, counted in the allowance, then deleted. (B) Deleted at once, with no question (the first text of THR-081; against CVM-012). (C) You confirm each delete (CVM-012 as it is). | **Decided** (R-294, the answer that he selected: "Snapshot kept for a time (Recommended)"): the disk of a machine is kept as a snapshot for a set time before it is deleted, counted in the allowance. The recommendation before the answer: A: nothing is lost by a mistake, and no question stops an agent |
| OD-THR38 | Past the allowance (OD-THR27) you said: "By default, I would like a smaller build". What is the smaller build? | (A) A smaller machine that is still inside the allowance is offered in place of the one that was asked for. (B) A small amount over the allowance that is billed. (C) Something else. | A: it keeps "refused unless they enable" exact, and the agent still gets a machine |
| OD-THR39 | The steps of the limits after 5, 10 and 15 (THR-114)? | (A) 8, 15, 25; then 10, 20, 40; then 10 more threads at each step. (B) Your steps. (C) No limit after 5, 10, 15. | A |
| OD-THR40 | With the account methods of Codex (OD-THR3): may Anima hold or pass the tokens of a ChatGPT login (`chatgptAuthTokens`), or only start the sign-in on the machine and read the account and its limits? BOT-020 forbids the first today. May Anima read the limits of Claude too (AIS-022)? | (A) Only the sign-in on the machine, the account and the limits of Codex; no tokens in Anima; Claude as today. (B) Also tokens, on the device only, never through the hub. (C) Also through the hub. | A |
| OD-THR41 | Your answer to OD-THR18 ended at "The". Is something missing? And does "accept it" also cover a purchase of a larger plan (THR-108), and a tool by which a parent agent answers the approvals of its children? | Your words. | Until you say more: a purchase keeps its sign-in check, and no tool for an agent to answer approvals is built, though a program of the same user can use the command |
| OD-THR42 | With the switch "This device gives its threads to the hub" off: does anything of a thread still go to the hub (a title for a notification on your phone, a count for the centre button)? | (A) Nothing. (B) The count only. (C) The count and the titles. | A in stage 1b, where nothing leaves at all; B from stage 2, so that the phone can show a number |
| OD-THR43 | With the account methods of Codex: where may the name of the account, the plan and the limits go? | (A) They stay on the machine and in the client of that computer. (B) Also to the hub, so that each client and the phone show the limits of each machine. | A now; B is a change of AIS-022 for the hub |
| OD-THR44 | How long does the hub keep the content of threads (OD-THR4)? | (A) Until you delete the thread. (B) 90 days after the last entry. (C) 30 days. | B, as a setting |
| OD-THR14 | Do you approve the mockup [agent-threads/index.html](../mockup/agent-threads/index.html), frames 1 and 2, for stage 1a? | (A) Yes. (B) Changes. | **Decided** (R-296, the answer that he selected: "Approve (Recommended)"): the mockup frames 1 and 2 are approved. The list of the frames changes with OD-THR24 (a folded group "Earlier"). The recommendation before the answer: A |

## 11. Changes that this proposal asks of other documents

Not made in this version; each needs the ruling named.

| Document | Change | Ruling |
| --- | --- | --- |
| [bots.md](bots.md) BOT-032 | Add: with `-p`, the input and output format stream JSON and `--permission-prompt-tool` | OD-THR2 |
| bots.md BOT-033 | Add: `codex app-server` with the thread, turn and approval methods only; the account methods stay forbidden | OD-THR3 |
| [agent-tools-as-extensions.md](agent-tools-as-extensions.md) AXT-020, AXT-045 | The T3 Code entry: as OD-THR11 | OD-THR11 |
| agent-tools-as-extensions.md AXT-046 | One list: the thread list gives the rows of Claude Code and Codex | Approval of THR-024 |
| [agent-layer.md](agent-layer.md) Section 1 | "no chat view" is changed by R-286 | Made by R-286 |
| [extensions-plan.md](../plans/extensions-plan.md) AIS-018 | The session ID with the counts; in stage 2 a title that was given and a folder name leave the device, which AIS-018 forbids today | OD-THR9, OD-THR13 |
| [cli-mcp-plan.md](../plans/cli-mcp-plan.md) MCP-006, agent-layer.md AGL-003 | A tool that sends a prompt, if OD-THR8 permits one | OD-THR8 |
| bots.md BOT-020, BOT-038 | A prompt that came through a hosted hub to a program with a subscription login; the vendor terms row | OD-THR2, OD-THR3 |
| agent-tools-as-extensions.md Section 3, AXT-049 | "Do not build a second ... T3 Code" names Herdr and the bot only; AXT-049 as OD-THR16 | Approval of this proposal; OD-THR16 |
| [cli-mcp-plan.md](../plans/cli-mcp-plan.md) MCP-006; agent-layer.md AGL-003, AGL-027 | The tools of THR-069 let the agent of a thread start and message threads of its own tree | OD-THR8 |
| [cloud-machines.md](cloud-machines.md); [hosted-mode.md](hosted-mode.md) HMD-073 | A machine that an agent asks for, counted against the allowance of the plan | TH90; OD-THR26, OD-THR27 |
| [bots.md](bots.md) BOT-021, BOT-025, BOT-032 | More than one thread of one login at one time (a tree); the form of the adapters after TH10 | OD-THR2, OD-THR3, OD-THR25 |
| [cloud-machines.md](cloud-machines.md) CVM-006, CVM-010, CVM-012, CVM-014 to CVM-016, CVM-021, CVM-022, CVM-024; hosted-mode.md HMD-073 and its Section 6.11 | A machine that an agent asks for: the own machines first, the cap, the confirmation of a delete, the bank check, who makes a machine (the control plane of Anima.ing, not the hub), the enrolment, the fair-use limits; the hub asks the pool nothing today | TH90; OD-THR26, OD-THR27, OD-THR35, OD-THR37 |
| [jobs.md](jobs.md) ROU-001 to ROU-016 and its review RT7 | A thread that an agent places is routed work | TH86 |
| [bots.md](bots.md) BOT-033 | The ban on the account methods of `codex app-server` is lifted (R-290, OD-THR3) | Made by R-290; the text of bots.md changes when its owner next changes it |
| [extensions-plan.md](../plans/extensions-plan.md) AIS-022; CAP-025 of the compute plan | For Codex, Anima reads the account, the plan and the limits (THR-115) | OD-THR40 |
| agent-tools-as-extensions.md AXT-049 | The thread view shows a transcript, also of a different device, and the hub keeps it (R-292) | Made by R-292 |
| [agent-layer.md](agent-layer.md) Section 3 and Section 8 | The ideas `agent-state-hooks` and `needs-you-inbox` are parts of this proposal (THR-117, THR-118); their folders are deleted | Made by R-296, OD-THR30 |
| [mobile-plan.md](../plans/mobile-plan.md) OD-M11 | Decided: the live remote sessions are reached from the page of a device | Made by R-296, OD-THR23 |
| [cli-mcp-plan.md](../plans/cli-mcp-plan.md) MCP-004 | The tool `anima_agents_threads` gives a title that can be the start of a prompt (OD-THR13) | Made by R-296 |
| [extensions-plan.md](../plans/extensions-plan.md) AIS-018 | "It must not send text, a title or the path of a folder": from stage 2 a title and the entries go to the hub for a device whose switch is on | Made by R-296 and R-292 |
| [bots.md](bots.md) BOT-066, BOT-032 | A sign-in of Codex that Anima starts on the machine (THR-021); more than one thread of one login at one time in stage 1b | R-290, OD-THR3; OD-THR25 for the second |
| [design-spec.md](../spec/design-spec.md) LOG-007 | It covers the entries of a thread and the account fields of Codex | THR-051, THR-130 |
| [contracts/work-route.md](../../contracts/work-route.md) Section 6 | The work type `agent_thread` and the long poll of THR-026 | TH30 |
| [design-spec.md](../spec/design-spec.md) NAV-001 | The item Threads, above Agent usage (R-287) | The approval of this proposal |
| [mobile-plan.md](../plans/mobile-plan.md) MOB-010, mockup 6 | Threads in the place of Sessions in the phone bar (R-288); the mobile stream makes the change, in stage 6 | OD-THR23 and OD-M11 |

## 12. What could not be verified

- Task TH10 measured the two programs on one Mac with one version of each. Not measured: Linux; a folder that the person trusted (the allow rules of a project were dropped in a folder that was not trusted, and the deny rules applied); an approval that nobody answers; a question of the agent through `codex app-server`; whether the Agent SDK uses the same pipe.
- After `turn/interrupt` of Codex the command of the turn was still alive. It needs a second look before the stop of a thread is relied on.
- The format of the session files: read from the files of this Mac on 2026-11-20 (Claude Code and Codex of that day). The vendors change the format without notice; the reader ignores what it does not know (THR-006). Old Codex files without `event_msg` lines give no prompts in stage 1a.
- Nobody put T3 Code and the page Threads side by side on the same session, with pictures. The page needs this human check before a person judges it.
- The three research reports that R-291 names: Section 4.12 used their short form. The reports ran no tool, so each claim about Claude Code and Codex in Section 4.12 was a claim of the source of a different tool. Task TH10 then measured four of them on this Mac and each held: the approvals over the pipe (`--permission-prompt-tool stdio`), the state events, the session ID chosen before the start, and `turn/completed` of Codex as the end of a turn. The other claims are not measured.
- The screens of T3 Code: nobody ran it. The layout ideas come from its source and documents.
- How T3 Code shows a read and a search of Codex (inventory).

## 13. History

- 2026-11-20, version 3. The operator answered 36 open decisions (R-296, R-290, R-292, R-294; relayed). Section 1.3 has the answers that are not the recommendation, with their consequences; Section 1.4 has what OD-THR18 changes; Section 8.2 has the risks that he accepted, with his words. THR-034, THR-048 and THR-105 are withdrawn. New: the holder (THR-113), the soft limits (THR-114), Codex with the account methods (THR-115), more than one repository in a thread (THR-116), the five ideas (THR-117 to THR-121), three functions of other tools (THR-122 to THR-124), tasks TH17 and TH70 to TH72, OD-THR38 to OD-THR41. The mockup has a folded group "Earlier". The third security read (H1 to H24) is in Section 9.2: the requirements that an answer changed are written in full in their form of version 3, Section 8.0 gives the guards in force, THR-125 to THR-130 are new, and Section 6.2.1 is the list for the review of the code of stage 1b, with the order of its build.
- 2026-11-20, version 2, second part. Task TH10 is measured on this Mac (the record th10-driving-claude-code-and-codex.md): THR-020 and THR-021 say what the two programs give, and OD-THR2 and OD-THR3 have the evidence. The four claims of the research that it tested held.
- 2026-11-20, version 2. From the brief of a side session of the orchestrating session with the operator: the principle of reach (THR-065, R-289); the parity count and the parity table (Sections 1.1 and 6.9); the order of the build by where the work runs; task rows for stages 4 and 5; approvals are stage 1b on this computer in each table; orchestration, a chosen machine and model, and servers inside the subscription (Section 4.11, THR-066 to THR-084, stages 7 and 8, R-297, R-295); the ideas and where they would go (R-291); the recommendations of OD-THR6, OD-THR7, OD-THR8 and OD-THR21 are turned; OD-THR25 to OD-THR30 are new; Markdown in the thread (THR-019, TH8). Section 4.12 takes the findings of the research on other tools (THR-085 to THR-104, task TH16, OD-THR31 to OD-THR34), and the parity table has one more row (a comment on a line of a diff). The second security read (G1 to G19) is in Section 9.1, with THR-105 to THR-112, the powers P20 and P21, and OD-THR35 to OD-THR37; the recommendations of OD-THR7, OD-THR8, OD-THR21 and OD-THR28 carry its conditions.

- 2026-11-20, version 1, fifth part. R-293: the threads that need the person at the top and the threads that work at the bottom, and a notification (Section 5.1, THR-055 to THR-064, tasks TH5 to TH7, OD-THR24). TH5 is built on the branch `threads/needs-you`; the system notification and the watcher are not built. The reader is the crate `agent-threads` (the release map of `anima` alone and of the iOS application does not take the core of the daemon).
- 2026-11-20, version 1, fourth part. R-287 decides OD-THR10 (its own item of the bar); R-288 puts Threads in the place of Sessions in the phone bar (THR-054, OD-THR23, the note for the mobile stream); each function of the inventory has a stage (Section 6.6, THR-053), after the words "should copy every feature they have".
- 2026-11-20, version 1, third part. The separate security read (28 findings) is recorded in Section 9; the text and the code of the read side have its corrections; OD-THR18 to OD-THR22 are new, and the recommendation of OD-THR8 changed.
- 2026-11-20, version 1, second part. `origin/dev` at `116c7b15` was read for the documents of the other planning stream (Section 4.10); OD-THR15 to OD-THR17 are new.
- 2026-11-20, version 1. Written from R-286, the inventory of T3 Code at commit `6f5293fb`, and the code of `dev` at `4ffc36a5`. Tasks TH1 to TH4 are built on the branch `threads/read-side`. A separate reviewer read version 1 (Section 9).
