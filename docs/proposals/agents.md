---
kind: Proposal, with a task table
state: Draft
evidence: Version 1 (2026-11-20). The direction is R-NEW-hermesagents (operator, first-hand, relayed by the orchestrating session). The requirements HAG-001 to HAG-042 are not approved and no code that installs or starts Hermes exists. A separate security read of this version is done (Section 9.3).
---

# Proposal: Agents (named assistants on Hermes Agent, each with routines, each on a computer that the operator chooses)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this proposal (PLN-005).
Version: 1. Date: 2026-11-20.
Status: the header of this document holds the state (PLN-020).
Map: [work-plan.md](../work-plan.md), categories Use and Reach. Row 4.24 (new, this document). It stands on row 4.21 (agent tools as extensions, R-196) and on row 4.7 (the work route).
From: R-NEW-hermesagents.
Identifiers: HAG-001 to HAG-042, OD-HAG1 to OD-HAG17, tasks HG0 to HG24.
Language: Simplified Technical English (design specification, Section 0).

## 1. What was decided, and what was not

The operator, first-hand, 2026-11-20, to the orchestrating session (relayed to the session that writes this document). R-NEW-hermesagents records these words:

> "am thinking a lot about harness can you look into this and what we might be able to take from them. As well I really do want to integreate a grokbot style interface for hermes agent that can pretty much automaticly be hosted"

Asked whether "Grok Bot style" means the chat that he uses today (he talks to it from his phone, it plans and pushes work) and where it must run:

> "yes but also that there are multiple agents like this grokbot that run routines but also can be housed on different computers"

Asked whether to add it to the T3 Code proposal:

> "I think I might want it as a seperate session, as t3 is more for dedicated agentic coding where hermes is for everything"

Decided by these words (the direction only):

- Anima has **agents**: several named assistants. Each one is a Hermes Agent instance. Each one has routines. Each one is housed on a computer that the operator chooses. Each one has a chat in Anima, in the style of the chat that he uses today.
- The install and the start must be close to automatic ("pretty much automaticly be hosted").
- This is not the Threads work (`agent-threads.md` on the branch `docs/agent-threads`). Threads is for coding. Agents is "for everything".
- He asked what Anima can take from the idea of a harness (Section 8).

Not decided: each requirement below, each open decision of Section 11, the place of the row in the priority order, and the time of the build. His standing rule holds: nothing that gives an agent a shell, or lets a hub start a program on a machine, merges before its security review and his rulings.

### 1.1 Why this is its own document, and not a change of agent-tools-as-extensions.md

R-196 made Hermes the assistant of Anima and put it in a pane (AXT-020, AXT-021). That document owns a host for tools of other vendors: a pane, a grant, a state, a health and an install. Its rule is "Anima.ing gives the pane, the grant, the state, the health and the install. It does not give the tool."

The new direction adds things that a pane host does not own: a record of an agent on the hub, many instances on one machine, routines that run with no person present, a budget and a stop for each agent, a move of an agent to another machine, and a chat that is a thread of Anima. Each of these is a new concern with its own security surface. One document for each concern (PLN-016) gives a new document. It does not copy AXT requirements: it uses them by number (Section 2). The recommendation is a new row 4.24, tier 3, beside 4.21 (OD-HAG1).

The other choice, a new Section of agent-tools-as-extensions.md, is shorter to find but makes that document own a runtime, which R-196 said Anima does not build ("do not build a second assistant bot"). This proposal also does not build a second assistant: Hermes stays the assistant. Anima only installs, supervises, schedules the budget of, and shows each instance.

### 1.2 The present "Grok Bot", as far as the repository records it

What the repository says:

- The rulings R-196, R-197, R-207 and R-243 give the source "Grok Bot chat": the operator decides open decisions in that chat, and the session that records the ruling gets his words from it.
- [ideas/README.md](../ideas/README.md) names "the Grok bot" as a source of ideas: "The operator gives the text of the bot to a session."
- The background of this task says that he talks to it from his phone, and that it plans and pushes work, and that a planning stream that he calls "Grok Bot" pushes proposals to `dev`.

Not in the repository, and asked of the operator (Section 12): which program runs it (Grok on the web, the xAI API, a bot of a messaging platform, or a tool of his own), on which machine, which tools and which keys it has (for example a GitHub token that can push to `dev`), how he starts a conversation, and what he likes most and least in it. The design below does not depend on the answers; the answers change only the first routines and the order of the messaging platforms.

## 2. What exists, and who owns it

| Subject | Owner | State | What this proposal does |
| --- | --- | --- | --- |
| Hermes is the assistant; Hermes first in "Add an agent"; no install without a click; no Claude login file | [agent-tools-as-extensions.md](agent-tools-as-extensions.md) A3, AXT-020 to AXT-024 (R-196) | Direction decided; requirements not approved | Uses them as they are. The install of an agent is the Hermes recipe of AXT-023 with a different home folder (HAG-010) |
| The Hermes dashboard in a pane; the terminal kind (`hermes --tui`) | AXT-020, AXT-014 | Proposed | Stage 1 uses it as the chat. The native chat comes in stage 3 |
| The list of agent programs on all devices, their states, one alert when one waits | AXT-046 to AXT-053, [agent-layer.md](agent-layer.md) AGL-001 to AGL-034 (R-263) | Proposed | A Hermes agent is one more row of that list (HAG-031). This proposal does not make a second list of live programs |
| Threads: the runner, the event log, approvals, "needs you at the top, working at the bottom", notifications, the phone | `agent-threads.md` (branch `docs/agent-threads`, session `t3-native`) | Proposed, version 2 | Uses the runner, the event log, the approvals and the order. Adds one adapter (Section 6.5 and Section 7) |
| The catalogue, recipes, vendor terms, budgets of bots | [bots.md](bots.md) BOT-019 to BOT-024, BOT-029 to BOT-046, BOT-058 to BOT-078 | Draft; BOT-001 to BOT-018 withdrawn | Adds one recipe kind: "agent instance" (HAG-012) |
| Containers on the hub's pool, pinned images | [hosted-applications-plan.md](../plans/hosted-applications-plan.md) (R-209 to R-251) | Plan | Records what the pool would need (Section 5.2). It asks for nothing now |
| The work route: queue, placement, a job on a machine, the device key | [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) W1 to W7, contract K10 0.5 | W1 waits for review | Install, update, start, stop and remove of an instance are work types of the route (HAG-015) |
| The Chat page, tools in chat, the model service | [compute-plan.md](../plans/compute-plan.md) CHAT-008 to CHAT-045, Level 1 | Proposed | A Hermes agent can use the model address of Level 1 as its model provider (HAG-038). The Chat page stays the chat with a model; an agent's chat is a thread |
| Scheduled checks and reminders of the engine | [extensions-plan.md](../plans/extensions-plan.md) (the engine) | Built in part | The hub's clock is not the routine clock. Hermes runs its own cron (Section 6.4); the hub only records |
| Agent usage: sessions, tokens and cost per device | extensions-plan.md Section 4.12.0, AIS-018 to AIS-024 | Built | Adds "by agent" and the budget line (HAG-040) |
| A program that an AI agent or a key starts on a machine | [routed-command-lines.md](routed-command-lines.md) RCL-001 to RCL-024 (R-258) | Draft | Starts nothing for an MCP tool or a key (Section 6.9) |

What the code has (read on 2026-11-20): `rg -i hermes crates apps link` finds nothing. There is no record of an agent, no command `anima agents` other than `anima agents usage`, and the daemon starts no program other than its own readers.

## 3. Hermes Agent: what it is, from its source

Read on 2026-11-20 from a shallow clone of `NousResearch/hermes-agent` (the default branch) and the documents in its folder `website/docs/`. Nothing was installed or run. The full notes are in the shared notes folder of this group of sessions (`hermes-inventory.md`). Paths below are paths of that repository.

| Subject | What the source says | Verified |
| --- | --- | --- |
| Licence | MIT, copyright 2025 Nous Research (`LICENSE`). Anima may reuse and change the code if it keeps the notice. The licence gives nothing for the name or the logos (`assets/nous-girl-*.svg`); no trademark policy was found. Anima uses the name only to say what it installs, with the credit of AXT-028, and does not use the logos | Licence: yes. Name and logos: not verified |
| Install | `curl -fsSL https://hermes-agent.nousresearch.com/install.sh \| bash` (with a pinned `uv`), or Nix, or Docker (`getting-started/installation.md`). Python 3.11 to 3.14 (`pyproject.toml`); the web, terminal and desktop parts need Node and pnpm; the installer needs git, curl, tar and a SHA-256 tool | Yes |
| Home folder | All state is in `~/.hermes`; `HERMES_HOME` changes it (`hermes_constants.py`). In it: `config.yaml`, `.env` (secrets), `auth.json`, `memories/MEMORY.md` and `USER.md`, `skills/`, `cron/jobs.json`, `state.db`, `SOUL.md` | Yes |
| Several instances on one machine | `hermes profile create <name>` makes `~/.hermes/profiles/<name>/` with its own configuration, `.env`, SOUL, memories, sessions, skills, cron and `state.db`. The logins of `auth.json` are shared between profiles unless the profile is made with `--isolated-auth` (`user-guide/profiles.md`). One gateway for each profile is documented (`user-guide/multi-profile-gateways.md`) | Yes |
| API for a chat from another program | `API_SERVER_ENABLED=true`, a Bearer key `API_SERVER_KEY`, default `http://127.0.0.1:8642/v1`. OpenAI-compatible: `/v1/chat/completions`, `/v1/responses` (state through `previous_response_id`), `/v1/runs`, `/v1/capabilities`, with streaming by server-sent events (`features/api-server.md`). There is also an ACP adapter (`acp_adapter/`) | Yes. A WebSocket chat: not found |
| Scheduler | Jobs in `cron/jobs.json` of the home; made with `hermes cron create/edit`, the dashboard, or the agent's own tool `cronjob_manage`. The gateway runs them on each tick. The result goes to the chat that made the job, to a file or to a platform. Jobs with a script and no model are possible. Dangerous commands in cron are refused by default (`features/cron.md`) | Yes |
| Skills and self-improvement | `skills/` of the home is the one source. The agent makes, changes, rewrites and deletes skills with its tool `skill_manage`, also in a background review after a task and in a "curator" pass. Writes need no approval by default; `skills.write_approval` adds a review. Skills from the skills hub are scanned (`tools/skills_guard.py`) (`features/skills.md`) | Yes |
| Tools and sandbox | `terminal.backend`: local, docker, ssh, modal, daytona, vercel_sandbox, or a plugin (`features/tools.md`). Approval of a dangerous command: `approvals.mode` smart, manual or off, with a time-out of 300 seconds. Cron, one-shot and unattended (API, webhook) sessions refuse dangerous commands by default (`user-guide/security.md`) | Yes |
| Messaging platforms | Telegram, Discord, Slack, WhatsApp, Signal, SMS, email, Matrix, Mattermost, Teams, Google Chat, IRC and about twelve more, webhooks and A2A. Allow-lists `<PLATFORM>_ALLOWED_USERS`, group allow-lists, or pairing (`user-guide/messaging/`) | Yes |
| Credentials | Secrets in `.env` and `auth.json` of each profile, credential pools and a credential vault. **Hermes reads the Claude Code login:** `agent/anthropic_credentials.py` names `~/.claude/.credentials.json` and the macOS Keychain item "Claude Code-credentials" as a "borrowed fallback" and runs `security find-generic-password -s "Claude Code-credentials"`; also `agent/credential_sources.py` and `hermes_cli/models.py`. Only the Anthropic card of the dashboard skips that file (`hermes_cli/web_server_oauth.py`). This confirms constraint C1 of agent-tools-as-extensions.md | Yes. A setting that turns the fallback off: not found; see HAG-013 |
| What it needs of a machine | No minimum of memory or processor is documented. Docker, Modal and Daytona are optional | Not verified. Measure in task HG6 |
| Update | `hermes update` updates the checkout that it manages; `--set-channel main` follows each commit. A package-manager install updates through that manager (`getting-started/updating.md`) | Yes |
| Dashboard | `hermes dashboard` on `http://127.0.0.1:9119`, `--host` and `--port`. Its pages send `Content-Security-Policy: frame-ancestors 'none'` (`hermes_cli/webapp_page_policy.py`), so it cannot be in an iframe. A child web view of Tauri that loads it as its own top page is not stopped by that rule. No `X-Frame-Options` was found | Yes for the header. Which routes send it, and the child web view: not verified |

What this changes in the design:

- Each agent is a separate `HERMES_HOME` (HAG-005), not a profile of one shared home: a profile shares `auth.json` unless `--isolated-auth` is given, and a separate home cannot share by mistake.
- The chat adapter of stage 3 uses `/v1/responses` with `previous_response_id`, on 127.0.0.1, with a key that the daemon makes (HAG-017, HAG-026).
- The Claude login fallback is real (threat T6). Since no switch to turn it off was found, the recipe must make it unreachable: the routine sandbox does not mount `~/.claude`, and the gateway runs with a `HOME` that is the agent's folder, so that `~/.claude` is not the person's folder (HAG-013). On a Mac the Keychain read through `security` still reaches the person's login Keychain; that is open (OD-HAG17).
- `skills.write_approval` is the switch for threat T9 (HAG-024 and Section 9).
- `hermes update` is not used; Anima pins a release (HAG-012, HAG-018).

## 4. Terms

| Term | Meaning |
| --- | --- |
| Agent | One named assistant of the operator: a record on the hub and one Hermes instance on one device. In the client the word "Agents" is the page of these records. A coding program (Claude Code, Codex) is an "agent program" (AGL terms), not an agent of this document |
| Instance | One Hermes home folder on one device with its configuration, memory, skills, keys and cron jobs, and the gateway process that runs it |
| House | The device that holds the instance of an agent. One agent has one house at one time |
| Routine | A job of the Hermes cron of one agent: a schedule or an event, a prompt, a delivery. Anima records each run |
| Run | One execution of a routine or of one chat turn, with its start, end, result, tokens and cost |
| Waits for you | A run or a chat that needs a decision of the person: an approval of a dangerous command, a question, a stop at the budget |

## 5. Where an agent runs

### 5.1 Machine first

- **HAG-001.** An agent must be housed on a device of the operator that runs `anima-daemon` and that he chose for it (machine first). Version 1 knows a Mac and a Linux computer (`pop-os`).
- **HAG-002.** The hub must not run an instance in its own container. The hosted hub has one processor and 1.5 GiB; it holds PostgreSQL and the hub. A Hermes gateway with its Python runtime and its browser tools takes the memory that the hub needs (Section 3, "What it needs of a machine").
- **HAG-003.** The page "Add an agent" must show, for each device, whether it can house an agent: the daemon is connected, the free memory, the platform, and whether the work route is ready on it (W4). A device that cannot gets the reason, not a hidden choice.

### 5.2 What the pool would need (not proposed now)

A pool-housed agent is a container of [hosted-applications-plan.md](../plans/hosted-applications-plan.md), not the hub's container. It would need: a pinned image of Hermes (HOST rules for pinned images), at least 1 GiB of memory and one shared processor for each agent without browser tools, a volume for the home folder with a backup, no network to the link except the hub's MCP address, a budget enforced by the pool, and the terminal backend "local" in the container (the container is the sandbox). It would also need an answer to who pays for the processor time. This is OD-HAG9; the recommendation is "later, after stage 4".

### 5.3 Several agents on one device

- **HAG-004.** One device may house several agents. Each agent must have its own home folder (Section 3: `HERMES_HOME` or profiles), its own gateway port on 127.0.0.1, its own keys file, its own memory, its own skills and its own cron store. No two agents share a folder.
- **HAG-005.** The home folder of an agent must be `<data folder of the daemon>/agents/<agent id>/home`, mode 0700, owned by the user that runs the daemon. The name that the operator gives is a label only; it is never a path.
- **HAG-006.** All agents of one device run as the same user account in version 1 (Section 9, threat T7). A separate user account for each agent is OD-HAG6.

## 6. The design

### 6.1 The record of an agent on the hub

- **HAG-007.** The hub must hold one record for each agent: id, name, purpose (one line), house (a device id), state, the version of Hermes, the model route (provider and model, never a key), the budget (Section 6.8), the routines (a copy that the daemon reports), the last run and its result, and the date of the last move.
- **HAG-008.** The state of an agent must be one of: Not installed, Installing, Running, Waits for you, Stopped, Over budget, Moving, Failed, Unknown. A source that cannot tell gives Unknown, never Running (the rule of AXT-048).
- **HAG-009.** The record must never hold a key, a token, the text of memory, the text of a skill, or the text of a conversation. The hub shows these only when the person opens them, through the daemon, the way the reveal of sealed records works (E0-6), and stage 1 does not show them at all.

### 6.2 The catalogue entry and the install

- **HAG-010.** The catalogue entry "Hermes agent" is the Hermes recipe of AXT-023 with three additions: a name, a purpose and a house. Its review screen says what will be installed, where, as which user, which port, which model provider, and the text of AXT-022 about the Claude login.
- **HAG-011.** The install must start only from a click of the operator on that screen (AXT-021). No install by a schedule, by an event, by another agent, by an MCP tool or by an API key (BOT-065).
- **HAG-012.** The install must use a release of Hermes that the recipe pins (a tag and its digest), not the default branch, and must not run a script piped from the network (`curl | bash`). The recipe installs into a folder of the daemon with `uv` or `pip` from the pinned release.
- **HAG-013.** The install must not import keys from other programs. Hermes reads the Claude Code login file and the Keychain item "Claude Code-credentials" as a fallback (Section 3). A different `HOME` is not a guard (SR-3): a shell command can still read the file. The chat and the routines must run in a container or as a separate user that cannot read the person's `~/.claude`, and on a Mac the Keychain read stays possible (OD-HAG17). The keys of the agent come only from the operator, in the client, through the secret path of the work route (W7).
- **HAG-014.** "Pretty much automatic": after the click the install, the configuration, the first start and the health check run with no further step. The operator types only a model key once (or picks the model address of Level 1, which needs no key on the device).
- **HAG-015.** Install, update, start, stop and remove are work types of the work route (K10), claimed by the daemon of the house with the device key (W7). They are native jobs on the machine, not container jobs.

### 6.3 Supervision and update

- **HAG-016.** The daemon must run the gateway of each agent as a child that it supervises (launchd or systemd user unit of the daemon's own, not a unit that Hermes writes), restart it with a back-off, and report the state.
- **HAG-017.** The gateway must listen only on 127.0.0.1 on its port, with an API key that the daemon makes and keeps; the hub reaches it only through the daemon.
- **HAG-018.** The update must be a click of the operator on one agent or on all, to a version that the catalogue pins. Before the update the daemon copies the home folder (a local snapshot). A failed health check after the update restores the snapshot. Hermes' own update command is not used.
- **HAG-019.** Remove must stop the gateway, remove its unit, and move the home folder to the trash of the daemon for 30 days; the operator can delete it at once with a second confirmation.

### 6.4 Routines

- **HAG-020.** A routine is a cron job of Hermes in the home of its agent. Anima writes and reads it through the daemon with the commands that Hermes gives (Section 3, "Scheduler"). Anima does not run a second scheduler.
- **HAG-021.** A routine may start from a schedule, or from an event of Anima that the operator chose from a fixed list (a check goes down, a build fails, a reminder is due). An event reaches the agent as a message through the daemon, not as a shell command.
- **HAG-022.** Each run of a routine must be recorded on the hub: the routine, the start, the end, the result (Done, Failed, Waits for you, Stopped at budget), the tokens, the cost, and a summary of at most 500 characters that the agent writes. The full output stays in the home of the agent.
- **HAG-023.** With no person awake a routine may: read, search the web, write in the folder `work/` of its agent, propose notes to its memory (shown as a diff, SR-7), send a message to the operator through Anima. It must not, unless the operator allowed it for that routine by name: run a terminal command outside its sandbox, push to a forge, send a message to a person other than the operator, spend more than the run budget, change its own skills or another routine, or install anything. A run that needs one of these stops and is "Waits for you".
- **HAG-024.** The configuration that Anima writes must set Hermes' `approvals.mode` to `manual` (never `smart`, which a model decides, and never `off`) and `skills.write_approval` on (SR-1, SR-4); an approval that a routine needs goes to the chat of the agent as an approval entry (Section 6.5).

### 6.5 The chat

- **HAG-025.** Stage 1: the chat is the Hermes dashboard (or `hermes --tui`) in a pane, as R-196 allows (AXT-020).
- **HAG-026.** Stage 3: the chat of an agent is a thread of the Threads design. One adapter, "hermes", in the thread runner of `agent-threads.md` turns the gateway's conversation API into the entries of the crate `agent-threads` (prompt, reply, tool call with result, approval, end of turn).
- **HAG-027.** An agent's thread must use the same order as other threads: threads that need the person at the top, working threads at the bottom, and the same notifications.
- **HAG-028.** One agent has one long thread ("the conversation") and one thread for each routine run that needs the person. A run that ends Done is a line in the history, not a thread.
- **HAG-029.** The phone must be able to read and answer an agent's chat and its approvals. This is a requirement for the mobile stream; this document does not change phone code.
- **HAG-030.** Messaging platforms of Hermes (Telegram, Discord, Slack and others) are off in the configuration that Anima writes. The operator can turn one on for one agent, with an allow-list of his own account, after a warning (threat T4; OD-HAG4).

### 6.6 The Agents page

- **HAG-031.** The bar entry "Agent usage" becomes "Agents" with three tabs: Agents (this document), Threads (agent-threads.md) and Usage (the present Agent usage page). Live sessions stay in the bar where they are: they are connections to a device, not agents.
- **HAG-032.** The Agents tab shows one row for each agent: name and purpose, house, state, next routine and when, last result and when, cost this month against the budget. Rows that wait for the person come first; then running; then the others by name.
- **HAG-033.** One agent's page shows: the header (name, house, state, version, model route), the buttons Open chat, Pause, Stop now; the routines with their schedule, next run and last result; the recent runs; the budget; the keys by name only (never the value); the folders that the agent may read; and "Move to another computer".
- **HAG-034.** Stage 1 shows the page read-only: the one agent, its state, its house, and Open chat. No routines, no edit.

### 6.7 Moving an agent

- **HAG-035.** A move is a click of the operator: stop the agent, take a snapshot of its home folder without the keys file, carry it over the link from daemon to daemon (not through the hub's disk), install the same pinned version on the new house, restore, ask the operator for the keys again (or carry them only through the secret path of W7), start, check health, then remove the old copy after the operator confirms.
- **HAG-036.** The hub must not hold the snapshot. If the hub must relay, the snapshot must be sealed to the key of the new device (W7) so that the hub cannot read it.
- **HAG-037.** Paths in memory and skills that name the old machine are not rewritten. The page says so after a move.

### 6.8 Keys, folders and budget for each agent

- **HAG-038.** Each agent has its own model key or uses the model address of Level 1. One key is never written to the home of two agents.
- **HAG-039.** Each agent has a list of folders that it may read or write, outside its home. Empty by default. The terminal backend of a routine is a container (Docker on Linux, OD-HAG5 on a Mac) with only those folders mounted.
- **HAG-040.** Each agent has a budget for a run, a day and a month, in tokens or in money. Agent usage shows the use of each agent beside its budget. At the budget the daemon stops the gateway's runs (it does not only warn) and the state becomes Over budget.
- **HAG-041.** Each agent has a Stop now button on its page, on the phone and as a command. Stop now ends the gateway process and pauses its cron at once, and works when the hub is not reachable from the device (the daemon keeps the last stop).

### 6.9 The `anima` command and the MCP tools (CMD-030 order)

| Function | Hub API | `anima` command | MCP tool | Stage |
| --- | --- | --- | --- | --- |
| List agents | `agents` | `anima agents list` | `agents_list` | 1 |
| Show one agent | `agent(id)` | `anima agents show <name>` | `agents_show` | 1 |
| Runs of an agent | `agentRuns` | `anima agents runs <name>` | `agents_runs` | 2 |
| Install an agent | `installAgent` | `anima agents add` (asks to confirm at the terminal) | None: a trust decision (MCP-005, BOT-065) | 1 (held) |
| Start, pause | `setAgentState` | `anima agents start|pause <name>` | None in version 1 (OD-HAG10) | 2 |
| Stop now | `stopAgent` | `anima agents stop <name>` | `agents_stop` (a stop only reduces power) | 2 |
| Add or change a routine | `setRoutine` | `anima agents routine ...` | None: it gives a program a schedule | 2 |
| Send a message to an agent | through the thread runner | `anima agents say <name> <text>` | None in version 1 (threat T3) | 3 |
| Move | `moveAgent` | `anima agents move <name> <device>` | None | 5 |

- **HAG-042.** `anima agents usage` stays as it is. The names above must not change its meaning.

## 7. What is shared with agent-threads.md, and what is this document's own

| Part | Owner | Note |
| --- | --- | --- |
| The thread runner in the daemon | agent-threads.md (THR-010 to THR-026) | This document adds one adapter, `hermes`, and asks the runner to accept a long-lived process (the gateway) beside one process for each thread |
| Entries, the event log, the stream to the client | agent-threads.md (crate `agent-threads`, TH1) | Used as it is |
| Approvals: allow, allow for the session, decline; no MCP tool or key answers one (THR-031) | agent-threads.md | Used as it is. "Allow always" is never offered for a routine (HAG-024) |
| The order "needs you at the top, working at the bottom" and notifications | agent-threads.md and AXT-053 | Used as it is |
| Grants for a tree, scoped tokens (THR-105) | agent-threads.md | Not used in version 1. An agent does not start threads |
| The record of an agent, routines, runs, budgets, the house, the move, the instance supervision, the Agents tab | This document | |
| The list of live agent programs and hooks | agent-layer.md | A Hermes agent shows in that list as one row, from this document's record |

Note for the session `t3-native` (given in the report, not written to its branch): this proposal asks agent-threads.md for (1) an adapter slot "hermes" whose process is long-lived and shared by many threads, (2) a thread kind "agent conversation" and "routine run" so that the list can filter them, (3) that the bar entry becomes "Agents" with the tabs Agents, Threads, Usage (HAG-031). It asks for nothing else.

## 8. Ideas from the harness

The video's claim: the harness (the loop, the tools, the context, the routing and the sandbox around a model) matters as much as the model; Microsoft's best result came from a router; an optimiser rewrote system prompts and tool descriptions against evals; own your harness, evals and context. His caution: do not tune to a harness that the user does not control.

| Idea | What it means in Anima | Recommendation | Stage |
| --- | --- | --- | --- |
| A router of model and effort for each run | The route of a routine names a model class (small, standard, large) and an effort; a daily check uses a small model; a run that fails or asks for help may go one class up once. Agent usage shows the cost of each route | Yes, as a field of the routine with a default "small" for scheduled checks. No automatic router before there are evals to judge it | 6 |
| A small eval set | 20 to 40 tasks for Anima's MCP tools and for the routines (for example "list devices that are down", "say which check failed last night"). Each task: done or not, the tokens, the time. It runs on a click and in the gates of the CLI crate with a recorded model answer | Yes. It is the one thing that makes changes of tool descriptions measurable. Store the tasks in the repository | 6 (the MCP part can start any time as a document) |
| Tune tool descriptions and prompts against the evals | Change the text of Anima's own MCP tool descriptions, measure before and after | Yes for Anima's own tools and prompts only. Do not tune Hermes' or Claude Code's prompts: the user does not control those harnesses, and a vendor update can undo the tune | 6 |
| Context on demand through MCP tool sets | The MCP server gives a small default set and loads a set (devices, checks, builds, records) when the agent asks, so that the context holds only what the task needs | Yes. It agrees with R-243 (fewer, broader tools). Measure it with the eval set | 6 |
| Hermes' self-improving skills | Hermes writes and rewrites its skills after a task | Keep it, with guards (threat T9): a rewritten skill takes effect only after the operator sees the diff, for routines; in the chat it may take effect at once | 2 |
| A managed harness as a product | Anima runs the loop and the sandbox (Hermes on a machine of the user, or in the pool); the user brings knowledge (memory, files), procedures (skills, routines) and tools (MCP servers) | A direction for after stage 5. From `anima-cloud` it would need: a container type "agent" with a pinned image, a volume with a backup, a budget enforced by the pool, a network policy (MCP to the hub only), and billing for processor time. Record it as an idea, not a requirement | Later (OD-HAG9) |

## 9. Security

### 9.1 Threats and guards

Each row is a way that this gives a program or a person power over a machine or an account.

| No. | Threat | Guard | The operator's choice |
| --- | --- | --- | --- |
| T1 | An agent acts with no person present (a routine at night) and runs a command, pushes, deletes, or spends | Hermes' refusal of dangerous commands in cron is a list of patterns, not a guard. HAG-023 list of what a routine may do; every other act waits for him; Hermes approvals stay on (HAG-024); the terminal of a routine is a container with only the chosen folders (HAG-039); budget stops (HAG-040); Stop now (HAG-041) | Which acts a routine may do without him, by name (OD-HAG3) |
| T2 | Text from outside (a web page, an email, a file, a message) steers the agent (prompt injection) | A routine that read outside text cannot, in the same run, do an act of the "waits for you" list without approval; outside text is never put in a system prompt or a skill by Anima; the summary on the hub is shown as text, never run | Whether a routine that reads the web may also write to a forge in the same run (OD-HAG3) |
| T3 | The hub, or a person with a hub login, names a prompt that the agent then runs on the machine | The hub can send only a message to a conversation, and only from a signed request of a client of the operator (the device key path, W7); no MCP tool and no API key sends a message in version 1; an agent's tools are its own, not the hub's | Whether an MCP tool may send a message to an agent later (OD-HAG10) |
| T4 | A messaging platform (Telegram, Discord, Slack) is a way in for anyone who finds the bot | All platforms off by default (HAG-030); when on: an allow-list of his own account id, pairing on first use, no group chats, and a routine never takes a command from a platform message | Which platform, if any (OD-HAG4) |
| T5 | Keys: model keys, tool keys, tokens of a forge, leak from the home folder, the logs, or the hub | Keys only through W7; never on the hub record (HAG-009); one key for one agent (HAG-038); home folder 0700; logs of the daemon never print the keys file (LOG-007) | Whether an agent may hold a token of a forge at all (OD-HAG7) |
| T6 | The Claude login file (`~/.claude/.credentials.json`) or a vendor subscription is used by Hermes (vendor terms and the account) | Hermes' import of other tools' logins off (HAG-013); the review screen text of AXT-022; the recipe does not mount `~/.claude` into the sandbox | None: R-196, AXT-022 |
| T7 | Several agents on one user account read each other's home, memory and keys, or the person's whole home folder | The terminal of a routine is a container with only its folders; the chat's local terminal stays approval-gated. Same-user isolation is not real isolation: say so on the page | A user account for each agent (OD-HAG6) |
| T8 | Moving an agent copies memory and keys through the hub, or to the wrong machine | Daemon to daemon over the link, sealed to the new device's key (HAG-035, HAG-036); keys asked again; old copy removed only after confirmation | None |
| T9 | Self-rewritten skills: an injected instruction becomes a stored skill and runs in every later routine | `skills.write_approval` on in the configuration that Anima writes; skills that a routine uses are pinned by a hash; a changed skill is shown as a diff and takes effect in routines only after the operator accepts it; the page lists skills that changed | Whether the chat may change skills at once (OD-HAG8) |
| T10 | The install itself runs untrusted code (a script from the network, the default branch) | A pinned release and digest; no `curl | bash`; the install runs as the user, no root (HAG-012, BOT-063) | None |
| T11 | The gateway port is reachable from the network or from another program of the machine | 127.0.0.1 only, with a key that the daemon makes (HAG-017). Whether the dashboard has a login is not verified; until it is, the dashboard is reached only through a daemon proxy (SR-11) | None |
| T12 | The update brings a new tool or a new default that opens a hole | Pinned versions; the operator reads a changelog link before an update; the configuration that Anima writes is applied again after each update and checked | None |
| T13 | The budget fails open: a loop spends without limit at night | A count from the gateway can fail open (SR-8): model calls go through the daemon or the Level 1 address and are counted there; a hard limit at the provider; a failed count stops the agent | The default month budget (OD-HAG2) |
| T14 | An agent sends messages to people as the operator | Sending to anyone but the operator waits for him (HAG-023) | Never, or by name (OD-HAG3) |

### 9.2 What must be the operator's choice

The open decisions OD-HAG2 to OD-HAG8 and OD-HAG10 in Section 11.

### 9.3 Findings of the separate security read

A sub agent that did not write this document read version 1 on 2026-11-20. It got the text, not the conclusions of the writer. Its findings are below with the severity that it gave. Each one is open. This version corrects the five statements that it found wrong (marked "corrected"); the other fixes are requirements of version 2 and need the operator's rulings where the column says so. No finding goes to [security-findings.md](../security-findings.md) before a ruling gives it a number (SF-NEW- placeholders then).

| ID | Severity | Concerns | Attack | Fix recommended | State |
| --- | --- | --- | --- | --- | --- |
| SR-1 | Critical | HAG-023, T9, HAG-024 | The home folder holds `config.yaml`, `.env`, `cron/jobs.json` and `skills/`. A routine that may write there can turn approvals off, turn a platform on, add a cron job, or turn `skills.write_approval` off | Split the home: the daemon owns configuration, skills and cron and makes them read-only to the gateway; the agent writes only `work/` and `memories/`; the daemon checks the configuration before each start and run and refuses on a drift | HAG-023 corrected; the split is a requirement of version 2 |
| SR-2 | Critical | HAG-006, T7, T5 | Agents run as the daemon's user, so an agent's shell can read the daemon's data folder (the device key of W7, each gateway key), other agents' homes, `~/.ssh` and browser cookies, and pose as the daemon | Device key and gateway keys where the agent's user cannot read them (Keychain, TPM, or a daemon user); a user for each agent (OD-HAG6 (b)) before stage 4 | Operator: OD-HAG6 |
| SR-3 | Critical | HAG-013, T6 | A different `HOME` does not stop `cat /Users/<name>/.claude/.credentials.json`, and Python may find the home through `getpwuid` | `HOME` is not a guard. The chat terminal must also be in a container or a separate user. Stage 1 on a Mac waits for OD-HAG17 (a) or (b) | HAG-013 corrected; operator: OD-HAG17 |
| SR-4 | High | OD-HAG5 (b), HAG-039 | The chat uses the local backend with the whole home; the "smart" approval mode is decided by a model, which an injection can pass | Chat in a container too; `approvals.mode: manual`; never offer "smart" | HAG-024 corrected; operator: OD-HAG5 |
| SR-5 | High | HAG-039 | On Linux the `docker` group equals root; an agent that reaches `docker.sock` owns the computer | Rootless Docker or Podman; never the `docker` group; never mount the socket | Version 2 |
| SR-6 | High | HAG-020, HAG-011 | The agent's own tool `cronjob_manage` makes routines with no click; script-only jobs pass the approvals and the budget | Turn off `cronjob_manage` and script-only jobs; the daemon accepts only cron entries that it signed | Version 2 |
| SR-7 | High | T2, HAG-023 | Outside text written to memory in an unattended run steers each later run (a stored injection) | Memory of an unattended run is untrusted: shown as a diff, loaded after he accepts it | Version 2 |
| SR-8 | High | T13, HAG-040 | The budget counts what the limited process reports; paid tools are not counted; when the daemon is down nothing stops | A hard limit at the provider, or all model calls through the daemon or the Level 1 address, counted there; a failed count stops the agent | T13 corrected; version 2 |
| SR-9 | High | HAG-041, HAG-016 | Stop now ends the gateway, but its containers and shells run on, and the restart of HAG-016 starts it again | Stop the process group or cgroup and its containers; a "stopped" flag that the restart obeys; a test | Version 2 |
| SR-10 | High | HAG-012, T10, T12 | A pinned tag does not pin the Python and Node dependencies; the skills hub, `npx` MCP servers and browser downloads fetch code at run time; a compromised hub can change the digest | A hash-locked lock file; skills hub and run-time installs off; the daemon checks a release signature with a key it holds, not a digest from the hub | Version 2 |
| SR-11 | High | T11, HAG-017 | Programs of the same user read the API key from the home; the dashboard on 9119 may have no login; DNS rebinding or CSRF from a web page | The key out of the home; check `Host` and `Origin`; prove the dashboard login or put it behind a daemon proxy; one port for each agent | T11 corrected; version 2 |
| SR-12 | Medium | HAG-030, T4 | Webhooks, A2A, email and SMS are listeners that the proposal does not name | List each inbound listener and turn each off; a platform message is outside text (T2) | Version 2 |
| SR-13 | Medium | HAG-021, T3 | Event text (a check name, a build log) is written by others, and a compromised hub can send any text | Events as fixed codes and ids, never free text; checked against the operator's signed list | Version 2 |
| SR-14 | Medium | Section 8 | The proposal does not say which hub token an agent holds for the MCP server of Anima | One read-only token for each agent with a list of tools, that he can revoke | Version 2 |
| SR-15 | Medium | HAG-035, T8 | The move leaves out only the keys file; `auth.json`, the vault, tokens in `config.yaml` and `state.db` can hold secrets | Leave out each secret store; apply Anima's configuration again; ask him to rotate the old keys | Version 2 |
| SR-16 | Medium | OD-HAG7 (b) | A forge token that pushes to `dev` turns an injection into code that runs in CI | Push only to a branch for each agent; branch protection on `dev`; the daemon gates the push | Operator: OD-HAG7 |
| SR-17 | Medium | HAG-023 | An injected agent asks him to approve, again and again (approval fatigue) | Each approval shows the exact command and its source; no "approve all"; a limit on how often an agent asks | Version 2 |
| SR-18 | Low | HAG-018, HAG-019 | The trash and the update snapshots keep keys for 30 days | Delete secrets at once at a removal; snapshots 0700 and short-lived | Version 2 |
| SR-19 | Low | HAG-038 | Several agents on the Level 1 address cannot be told apart | One credential for each agent at the Level 1 address | Version 2 |

Statements that the reader found wrong, and what this version did: HAG-013 (`HOME` is not a guard) corrected; T11 (the dashboard login is not shown) corrected; T13 (the count can fail open) corrected; HAG-024 (Hermes has the modes smart, manual and off, no "always allow") corrected; HAG-023 (writes to the home break the guards) corrected; "dangerous commands are refused by default" is a list of patterns, not a guard (T1 notes it); Section 2 named HAG-052 and HAG-021 wrongly, corrected.

## 10. Stages and tasks

Each stage is useful alone. Stage 1 is small.

| Stage | What the operator gets | Needs from W3, W4, W7 |
| --- | --- | --- |
| 1 | One Hermes agent installed on this Mac from the catalogue by his click; the Agents page read-only with its state; its chat in a pane (R-196) | W4 (a native job on a Mac) for the install; W7 for the model key; none of W3's placement (the house is chosen) |
| 2 | Routines: add, list, pause; runs recorded; budgets and Stop now; "waits for you" | W7 for the signed stop and the signed routine change |
| 3 | The native chat: the agent's conversation as a thread; the phone entry (mobile stream) | The thread runner of agent-threads.md |
| 4 | More agents, and agents on `pop-os` | W4 on Linux; W3 for the queue across machines |
| 5 | Move an agent | W7 to seal the snapshot |
| 6 | The router field, the eval set, tool description tuning, context sets | None |

| Task | Stage | What | Depends on | Check | State |
| --- | --- | --- | --- | --- | --- |
| HG0 | 0 | This proposal, its security read, the register and priority rows, the mockups | — | `bun run quality:documents` | Done on the branch `docs/agents` |
| HG1 | 0 | The operator decides OD-HAG1 to OD-HAG17 and approves HAG-001 to HAG-042 | HG0 | A ruling | Not started |
| HG2 | 1 | The catalogue entry text and the review screen text (no install) | HG1 | Gates | Not started |
| HG3 | 1 | Hub: the record of an agent, `agents`, `agent(id)`, read-only | HG1 | Gates | Not started |
| HG4 | 1 | `anima agents list`, `show`; MCP `agents_list`, `agents_show` | HG3 | Gates | Not started |
| HG5 | 1 | Client: the Agents tab and one agent's page, read-only | HG4 | Gates; mockup comparison | Not started |
| HG6 | 1 | Daemon: the install job of a pinned Hermes into an agent home, its configuration (platforms off, imports off, approvals on, 127.0.0.1), the supervised unit, the health report | HG1, W4, W7, its security review | A human check on the Mac (HT-NEW-hermesinstall) | Not started, held |
| HG7 | 1 | The chat in a pane (AXT task for the Hermes pane) | HG6 | A human check | Not started |
| HG8 | 2 | Routines through the Hermes cron; runs recorded | HG6 | Gates; a human check of one night | Not started |
| HG9 | 2 | Budgets and Stop now, counted by the daemon | HG8 | Gates; a human check | Not started |
| HG10 | 2 | Skills pinned for routines; a diff to accept | HG8 | Gates | Not started |
| HG11 | 3 | The `hermes` adapter of the thread runner | agent-threads TH tasks | Gates | Not started |
| HG12 | 3 | The phone entry (mobile stream) | HG11 | The mobile stream's checks | Not started |
| HG13 | 4 | More agents on one device; agents on Linux | HG6 | A human check on `pop-os` | Not started |
| HG14 | 5 | Move | HG13, W7 | A human check | Not started |
| HG15 | 6 | The eval set for the MCP tools | — | Runs in the CLI crate | Not started |
| HG16 | 6 | The route field of a routine; cost by route in Agent usage | HG8 | Gates | Not started |

Human checks (to add to [need-human-testing.md](../need-human-testing.md) when the task is built, not before): the install on a Mac by a click; a routine at night that stops at an approval; the budget stop; Stop now with the hub unreachable; a move from the Mac to `pop-os`.

## 11. Open decisions, shortest first

| No. | Question | Options | Recommendation |
| --- | --- | --- | --- |
| OD-HAG1 | Own document and row? | (a) This document, new row 4.24, tier 3 beside 4.21. Pro: one owner for the runtime. Con: one more document. (b) A Section of agent-tools-as-extensions.md. Pro: one place for Hermes. Con: that document then owns a runtime, against its rule | (a) |
| OD-HAG2 | Default month budget of a new agent? | (a) $10. (b) $25. (c) No default; he must type one. Pro of (c): no surprise. Con: one more step | (a), shown on the review screen |
| OD-HAG4 | Messaging platforms? | (a) None in version 1; the Anima chat and phone only. (b) Telegram with an allow-list. Pro of (b): works now from his phone. Con: a way in that Anima does not see | (a) |
| OD-HAG5 | The sandbox of a routine on a Mac? | (a) Docker Desktop or Colima container. Pro: real isolation. Con: one more install, memory. (b) Hermes local backend with approvals. Pro: no install. Con: the user's whole home | (a), (b) only for chat |
| OD-HAG6 | A user account for each agent? | (a) No, one user, containers for routines. (b) Yes. Pro: real isolation. Con: root to make users, harder install | (a) in version 1 |
| OD-HAG7 | May an agent hold a forge token? | (a) No. (b) Yes, a fine-grained token per agent, pushes wait for him. Pro of (b): it can do what Grok Bot does today | (b) from stage 2 |
| OD-HAG8 | May the chat change a skill at once? | (a) Yes in chat, diff for routines. (b) Always a diff | (a) |
| OD-HAG9 | Agents in the pool? | (a) Later, after stage 4. (b) Now. Con: the hub container cannot; it needs a new container type and billing | (a) |
| OD-HAG3 | What may a routine do with no person present? | (a) The list of HAG-023 and nothing more. (b) The list plus acts that he names for each routine. Pro of (b): useful routines like a nightly push of a report. Con: each name is a door | (b), with no name allowed by default |
| OD-HAG10 | May an MCP tool or API key send a message to, or start, an agent? | (a) No, version 1. (b) Send only. Con: T3 | (a) |
| OD-HAG11 | The bar: "Agents" with tabs Agents, Threads, Usage? | (a) Yes. (b) Agents as a separate bar entry beside Agent usage | (a); it needs the agreement of the Threads session |
| OD-HAG12 | Which model provider by default? | (a) The model address of Level 1 where it runs, else the key he types. (b) Always a key | (a) |
| OD-HAG13 | The first routines? | Ask: a morning summary of the hub (checks, builds, agent usage), a nightly read of the security findings list, a weekly cost report | Those three |
| OD-HAG14 | The harness work: when? | (a) Stage 6. (b) The eval set now as a document | (b) for the eval tasks only |
| OD-HAG15 | Does stage 1's held install branch wait for W4? | (a) Yes. (b) A one-off install by the daemon outside the work route. Con: a second path that the review of W1 did not see | (a) |
| OD-HAG16 | Name of the requirement prefix and the page | "Agents" and HAG | Keep |
| OD-HAG17 | Hermes on a Mac can read the Keychain item of the Claude Code login, with the person's user. What to do? | (a) Run agents on a Mac only after a change of Hermes upstream that adds a switch to turn the fallback off (a pull request to Nous Research). (b) Run each Mac agent as a separate macOS user with its own Keychain (OD-HAG6 (b)). (c) Accept it, with the warning of AXT-022. Pro of (c): stage 1 now. Con: against C1 | (a), and stage 1 on `pop-os` first if the upstream change is slow |

## 12. Questions for the operator that this document cannot answer

- What is the present Grok Bot: which program, on which machine, with which keys and tools, and how does it push to `dev`?
- What do you like most and least in it? Which of its jobs should the first agent take over?
- Which agents do you want first (names and purposes)?

## 13. Texts of other documents that a ruling must change

| Document | Text | Change |
| --- | --- | --- |
| agent-tools-as-extensions.md | AXT-020 | Add: an instance of the Hermes recipe may be an agent of [agents.md](agents.md) with its own home |
| bots.md | BOT-060 | Add the recipe kind "agent instance" |
| extensions-plan.md | AIS-018 to AIS-024 (the Agent usage page) | Add "by agent" and the budget line; the bar entry becomes a tab (OD-HAG11) |
| agent-threads.md (branch) | The runner | An adapter slot for a long-lived process (Section 7) |

## 14. Not verified

- The memory and processor that one Hermes gateway needs (no minimum is documented). Measure it in task HG6.
- A trademark policy for the name and the logos of Hermes Agent.
- A setting of Hermes that turns off the fallback read of the Claude Code login (none was found).
- Whether the dashboard has a login, and which of its routes send `frame-ancestors 'none'`.
- Whether the dashboard works in a Tauri child web view (also open in agent-tools-as-extensions.md).
- A WebSocket chat API (not found; the HTTP API with server-sent events is enough for HAG-026).
- The live documents site of Hermes: the documents were read in the repository, not on the site.
- What the present Grok Bot is (Section 1.2 and Section 12).
