# Operator queue

| Item | Value |
| --- | --- |
| Document | What waits for the operator, in one place (PLN-023) |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Generated. Do not change the lists by hand |

This document is generated: `bun run quality:register` writes it from the documents that own each fact, and `bun run quality:documents` fails when it does not agree with them. It holds no fact of its own (PLN-016). To take an item off a list, change its owner: decide the decision in its plan, set the state in the header of the document, or write the result of the check in [need-human-testing.md](need-human-testing.md).

- List 1 is in the order of the priorities ([plan/priorities.md](plan/priorities.md)). Start at the top.
- List 2 has each row of a decision table that does not say "Decided". The work items in [work-items/README.md](work-items/README.md), section "Decisions", are in it.
- List 3 has each plan and proposal whose state is Draft, Direction decided or Not checked.
- List 4 has the open entries of [need-human-testing.md](need-human-testing.md). The older checks are in its section "Checks that other documents hold", grouped by machine.
- The business vault (a different, private repository) holds the deadlines of the company. They are not in this list.

<!-- generated from the planning documents: bun run quality:register -->

## 1. In the order of the priorities (11)

| Row | Work | Waits for |
| --- | --- | --- |
| 4.1 | Finish the device manager | Operator: OD-W25 (what the acceptance run WP-53 is on a hosted hub; R-169 defers eight of its checks); the Linux release and the check that an installed application updates (R7); the check of the rename to `anima-daemon` (HT-018, with new steps for a hosted hub; HT-019 is deferred, R-169); the Linux checks of the defects GH-52 and GH-54 (HT-020, HT-021). Agent: nothing; GH-53 is corrected |
| 4.16 | One-click install | Operator: the live checks (need-human-testing.md, "One-click") |
| 4.17 | The hosted mode and the shared pool | Operator: HT-027, HT-028, HT-029 and HT-031 in need-human-testing.md, which need a hub of the pool from a new hub image and an application with the new link program; the proposed requirement HMD-051 of hosted-mode.md; HM-D2, HM-D8, HM-D11 and HM-D13. Agent: the tasks HM-T10 and HM-T13 to HM-T15 in Section 8 of hosted-mode.md |
| 4.18 | Update the daemon and the hub | Agent: task U5 in daemon-and-hub-update-plan.md. Operator: one Linux machine and one Mac for task U3, the release server of OD-ROL-15 (R-158, task U12), and a release for each target for the tasks that need one |
| 4.3 | Decisions and approvals for the foundation | Operator: each decision and approval in the row. Agent: PE-C1, then PE-C2 of planning-evidence-plan.md, read-only preparation (R-152) |
| 4.5 | Level 1 model service | Operator: no Level 1 task starts until the macOS application 0.2.2 is published and the hosted live checks HT-027, HT-028, HT-029 and HT-031 in need-human-testing.md have a result (the condition of R-165); then the missing values of OD-C1 after task L1-1, and OD-AI1 of ai-direction.md. Agent: nothing before the condition is met |
| 4.7 | Jobs, builds and AI agent runs | Agent: task W1, the contract of the work route, in projects-and-builds-plan.md (R-160 is the ruling that changes R-115). Operator: the acceptance of the contract (W1) and of the design (W2); the security review of the work route comes before code |
| 4.9 | Hosted applications | Agent: None. H1 to H4 are claimed (docs/claims/h1.md, branch `claude/awesome-kepler-3fcd9e`, started 2026-11-18). Operator: nothing now |
| 4.19 | The watchdog | Operator: approval of the requirements WDG-001 to WDG-041. Agent: nothing; no ruling names a task |
| 4.20 | The terminal screen (`anima tui`) | Operator: OD-TUI6 and OD-TUI7 after TU1, then the approval of TUI-001 to TUI-083. Agent: task TU1 in cli-mcp-plan.md Section 11.8 (a record only); TU2 to TU10 wait |
| 4.13 | A hash chain for the audit log | Operator: HT-023 on the real hub. Agent: nothing |

## 2. Open decisions (106)

| Decision | Question | Recommendation or state | Document |
| --- | --- | --- | --- |
| OD-W25 | What is the acceptance run of Phase 3 (WP-53, row 4.1) on a hosted hub? R-169 defers eight of its seventeen checks (P-1 to P-6, P-14, P-16), which can run only… | Open. No recommendation of the operator's side yet. The choices: (a) the run is the nine checks that stay, done on the hosted hub; (b) the run waits for the sel… | [plan/decisions.md](plan/decisions.md) |
| OD-TUI6 | Are three subscriptions plus a re-read every 60 seconds the way to keep the data live (TUI-021)? The other ways are: only a re-read every few seconds (simpler,… | Open. | [plans/cli-mcp-plan.md](plans/cli-mcp-plan.md) |
| OD-TUI7 | Is a growth of the standalone `anima` file up to a limit that you name after TU1 measures it acceptable (TUI-070)? | Open until TU1. | [plans/cli-mcp-plan.md](plans/cli-mcp-plan.md) |
| OD-C1 | What is the hardware of each compute node? | Open for the values that are missing. The operator said (2026-11-18, through the orchestrating session): "I have a couple other machiens but that is good for no… | [plans/compute-plan.md](plans/compute-plan.md) |
| OD-C3 | Level 2, parked: which tolerances does the conformance test use? | Set them from the C4 measurements (RNG-032). | [plans/compute-plan.md](plans/compute-plan.md) |
| OD-C4 | Level 2, parked: are the block adapters offered to the Shard project first? | Yes: offer them to the Shard project first. Keep them in `blocks/` only if Shard does not accept them. | [plans/compute-plan.md](plans/compute-plan.md) |
| OD-E3 | Aggregator for the US accounts (BofA, Chase, Robinhood, Acorns) | SimpleFIN Bridge. It is for private persons and costs about 15 USD each year (its pages give different prices). Check each institution on bridge.simplefin.org b… | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E4 | Hosted vision model for Class B and C extraction | Permit a hosted model for Wardrobe and Library photos, with the operator switch (VIS-003). Use only on-device extraction for Documents. | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E5 | Hub folder grouping (REPO-E-001) | Group by parent nouns as the first task of E0 (`fleet/`, `engine/`, `vault/`, `life/`, `work/`), in a separate change. | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E7 | Garmin access | Use the Garmin bridge with `python-garminconnect` (Section 4.9.1). Keep FIT and export uploads as the fallback. Accept the risk that Garmin blocks unofficial ac… | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E8 | Obsidian two-way sync | Read-only in this version (WRT-005). Decide after XP-03. | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E10 | Price source for holdings that the source does not price | IBKR, Coinbase and Gemini give their own prices. For MetaMask tokens and the SimpleFIN holdings, select one free price API. | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E11 | Path for a request to a local upstream (MGW-010). A program such as Claude Code has one base address, so it cannot select the path itself. | The `anima` program on each computer listens on the loopback address. It sends a request for a local upstream directly to the device on the VPN, sends all other… | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E16 | Clock synchronization between Anima machines (TML-011, issue #7): how skew is found and reported, the permitted skew, and what an unreliable clock does | Not decided. E0-4 builds no clock check. Decide before a timeline source that uses the clocks of more than one machine. | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E17 | The portable form of a record reference across machines and replicas, and what a link does when its target is deleted (RLK-005, RLK-006, issue #7) | Not decided. E0-4 uses `kind:id` with an opaque `id` and keeps a link and its label when the target is missing. | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E19 | More ways to deliver a reminder or a notification: the desktop and iOS applications subscribe to the channel, and message channels as plug-ins, such as those of… | Not decided. E0-7 builds the delivery to an open application and to ntfy (REM-008). Recommendation: record the channels and the licences of those projects first… | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-HOST-3 | Are HOST-015 to HOST-024 approved, and in which stage is each one? | Approved in three groups: with stage 1, HOST-015, HOST-017, HOST-019, HOST-020 and HOST-021; with stage 2, HOST-016, HOST-018, HOST-022 and HOST-023; after a se… | [plans/hosted-applications-plan.md](plans/hosted-applications-plan.md) |
| OD-HOST-4 | Does the catalogue read the form of the template registry of Arcane (HOST-025)? | Yes, for reading. A list of a different person is off until the operator adds its address (R-150). | [plans/hosted-applications-plan.md](plans/hosted-applications-plan.md) |
| OD-HOST-1 | How is the data copy of HOST-005 made: a backup of the volume, or replication that the application supplies? | A backup of the volume, kept by bulk-data-placement.md. It is asked when HOST-004 to HOST-008 come to approval; it blocks no task of this plan. | [plans/hosted-applications-plan.md](plans/hosted-applications-plan.md) |
| OD-HOST-2 | Do you approve HOST-004 to HOST-009 (the home machine, the data copy, the move, a check and a web interface for each application, and no build platform)? | Yes, after stage 2 is in use. It blocks no task of this plan. | [plans/hosted-applications-plan.md](plans/hosted-applications-plan.md) |
| D-1 | Go gates. The gates are Rust and TypeScript. Which Go gates, and where do they join? | Five gates in `bun run gates`, after `pinning` and before `install`: `gofmt -l` (must print nothing), `go vet ./...`, lint (`staticcheck`, and `revive` for func… | [plans/one-click/s1-link-tasks.md](plans/one-click/s1-link-tasks.md) |
| D-2 | An unreleased Tailcat commit. The functions that S1 needs are only on Tailcat's `main` (spike Section 8, item 1). | Pin `main` at `e00cdde6ff78` (2026-11-05) by pseudo-version; `go.sum` fixes the content. Do not vendor (the dependencies are tens of MB). Keep Tailcat in the `t… | [plans/one-click/s1-link-tasks.md](plans/one-click/s1-link-tasks.md) |
| D-3 | The relay for the first version. The Headscale relay is set to refuse link keys. A relay of our own is a VPS change. | First version: Tailscale's shared relays, the library's default. No VPS change in S1. S1 builds and tests the own-relay setting against a relay that the tests s… | [plans/one-click/s1-link-tasks.md](plans/one-click/s1-link-tasks.md) |
| D-4 | What waits for the second-Mac measurements? | Nothing in T1 to T12 waits; they are proved by tests on one machine. Three things wait: the acceptance numbers of the keepalive (T6 is built with parameters; th… | [plans/one-click/s1-link-tasks.md](plans/one-click/s1-link-tasks.md) |
| D-5 | Invites by a temporary key (1.4), or an enrolment port that accepts every key? | The temporary key. The hub then never admits a key that it does not know, also when the address is public (D-6). The security review of T8 must confirm the deri… | [plans/one-click/s1-link-tasks.md](plans/one-click/s1-link-tasks.md) |
| D-6 | A domain name for the address (LNK-007). The address holds the pre-shared key, and DNS is public. Then the list of permitted keys is the only protection. | Build it, off by default, with this warning in the setting and in the documents. An invite is the normal way. | [plans/one-click/s1-link-tasks.md](plans/one-click/s1-link-tasks.md) |
| D-7 | The device key for the hub (the seam for S5). A loopback stream does not say which device it is from. S5 needs that. Build the sending side in S1? | Yes, as T12, off by default: the hub link puts a PROXY protocol v2 header with the device key at the start of each stream. It is tested against a fake hub. The… | [plans/one-click/s1-link-tasks.md](plans/one-click/s1-link-tasks.md) |
| D-8 | Where the private key is kept. A file with owner-only access, or the system key store? | A file in S1, for both roles (the hub role also runs on Linux with no key store). For the device role in the application, S6 can move the key to the key store t… | [plans/one-click/s1-link-tasks.md](plans/one-click/s1-link-tasks.md) |
| D-9 | Folder name: `link/` at the root, program `anima-link`, proposed requirement LNK-009 for the keepalive. | Yes. Spec Section 13 needs one line (Section 4). | [plans/one-click/s1-link-tasks.md](plans/one-click/s1-link-tasks.md) |
| D-1 | The seam: a gateway program with the `guacd` protocol (1.1), or VNC code inside the hub with a worker that the hub starts? | The gateway program. No change to the hub's session code, to K3, K4 or K7. The other way changes the `tunnel` router signature (K3), adds a setting (K7) and tou… | [plans/one-click/s2-vnc-tasks.md](plans/one-click/s2-vnc-tasks.md) |
| D-2 | The gateway listens on loopback TCP with no authentication, as `guacd` does (GW-003). On a Mac any local program can connect to it. Such a program gets no crede… | Yes. A Unix socket with owner-only access is better, but `ANIMA_GUACD` is `host:port` in K7. Ask for that K7 change with the security review of T6. | [plans/one-click/s2-vnc-tasks.md](plans/one-click/s2-vnc-tasks.md) |
| D-3 | A Mac with several displays sends all of them as one screen of about 14 megapixels (spike 4.2). First version: scale the screen down in the gateway to a largest… | Scale down to at most the client's window size (the `size` instruction gives it). It needs no Apple-specific message and also cuts bytes. One display at a time… | [plans/one-click/s2-vnc-tasks.md](plans/one-click/s2-vnc-tasks.md) |
| D-4 | Name: crate `crates/anima-gateway`, program `anima-gateway`. "Gateway" is the spec's word for this role (Section 4.3). | Yes. | [plans/one-click/s2-vnc-tasks.md](plans/one-click/s2-vnc-tasks.md) |
| D-1 | The seam: a program `anima-database` beside the hub (1.1), or a `database` domain inside the hub? | The program. No change to the hub, to K7, to `main.rs` or to `hub-modules.json`, and the hub stays unaware of where its database is. | [plans/one-click/s3-database-tasks.md](plans/one-click/s3-database-tasks.md) |
| D-2 | Name: crate `crates/anima-database`, program `anima-database`, folder word `cluster`. | Yes. | [plans/one-click/s3-database-tasks.md](plans/one-click/s3-database-tasks.md) |
| D-3 | The password and where it is kept. (a) Generated 192-bit passwords, SCRAM, kept in the two URL files with mode 0600 below the root. This is DB-002's "file" and… | (a). The files are in a folder with mode 0700 and never leave it; `backup` and `status` never print them. The same question exists for the hub's key file (`ANIM… | [plans/one-click/s3-database-tasks.md](plans/one-click/s3-database-tasks.md) |
| D-4 | Durability on macOS: `wal_sync_method=fsync_writethrough`? It makes a commit 3.7 ms in place of 45 microseconds (spike 10.2). Without it a power cut can damage… | Yes, on macOS. A Mac mini has no battery. The hub's write rate can carry it. Linux keeps its default, which is already safe. | [plans/one-click/s3-database-tasks.md](plans/one-click/s3-database-tasks.md) |
| D-5 | The socket path limit is 103 bytes. With the socket in `~/Library/Application Support/ing.anima.desktop/pg/` a home folder path of 40 bytes or less fits (`/User… | (a) for the first version. (b) is a second connection mode to build, test and review for a case that may never occur. On Linux the socket is in `$XDG_RUNTIME_DI… | [plans/one-click/s3-database-tasks.md](plans/one-click/s3-database-tasks.md) |
| D-6 | The major-version upgrade (1.5). Build the full `pg_upgrade` driver now, or only the parts that protect data? | Now: the version check, the refusal of a newer cluster, the versioned tree folders and `backup`. Later, in the release that brings PostgreSQL 18: the `pg_upgrad… | [plans/one-click/s3-database-tasks.md](plans/one-click/s3-database-tasks.md) |
| D-7 | Backup. S3 gives the `backup`, `restore` and `verify` commands. The schedule, the place, how many are kept and the command in the client and CLI are S9. Agree? | Yes. S3 also takes one backup itself before a major upgrade. | [plans/one-click/s3-database-tasks.md](plans/one-click/s3-database-tasks.md) |
| D-8 | Time Machine copies the cluster while the server runs. Such a copy restores, at best, as a crash recovery. Exclude `<root>/cluster` from Time Machine (`tmutil a… | Yes. | [plans/one-click/s3-database-tasks.md](plans/one-click/s3-database-tasks.md) |
| D-9 | Raw sample retention in the application mode: 30 days needs about 4.9 GB for 10 devices with a daemon (spike 10.2). The charts do not read raw samples. | 3 days in the application mode. This is a hub setting (`retention.periods`) and a DB-009 delta, not S3 code. It waits from the last report. | [plans/one-click/s3-database-tasks.md](plans/one-click/s3-database-tasks.md) |
| D-10 | The oldest Linux that the application supports. The tree is built on the release builder, so it follows the application. | State the builder's distribution in the release notes; no extra build container for the first version. | [plans/one-click/s3-database-tasks.md](plans/one-click/s3-database-tasks.md) |
| OD-AI1 | Does one document own the model address, the list of models and the record of use? | Yes. The model gateway owns the address on each computer, the list and the record. The compute plan owns the machines: runtimes, pulls, placement. The router of… | [proposals/ai-direction.md](proposals/ai-direction.md) |
| OD-AI4 | Is a pane that runs the vendor program as the user of the operator the first way to run an AI agent? | Yes. It is the way that works with a subscription. A run in a container stays in agent-runs.md and comes later. The daemon gets one new function, the session se… | [proposals/ai-direction.md](proposals/ai-direction.md) |
| OD-AI5 | Does Anima write its own session server, or embed Herdr? | Open decision 1 of Section 4.12.1 of the extensions plan holds this. Recommendation there: not ruled. | [proposals/ai-direction.md](proposals/ai-direction.md) |
| OD-AI6 | Does the automatic choice of a model wait? | Yes. The operator or the caller names the model. The rule "cheapest model" of a bot waits for its own decision. | [proposals/ai-direction.md](proposals/ai-direction.md) |
| OD-AI7 | Does job 1 get its own row in Section 4 of the work plan, before row 4.5? | Yes: one row "AI agents: usage and panes", tier 2b, before the model service. Row 4.5 keeps its number. | [proposals/ai-direction.md](proposals/ai-direction.md) |
| OD-BOT1 | Is breadth, as the agents that Orca lists, a goal of the first release of Bots, and is the generic runner (BOT-039) the first work of Bots after the agent panes… | Proposed, not ruled: yes. A custom entry gives all 34 agents at launch-and-watch depth for the cost of the panes work. The panes work (AIS-009 to AIS-013, OD-AI… | [proposals/bots.md](proposals/bots.md) |
| OD-BOT2 | Which agents get a Tier 2 catalog entry, and in what order (BOT-043)? | Proposed, not ruled: the order of BOT-043. It comes from GitHub stars on 2026-11-17, a weak signal, and no public number exists for the closed agents (Cursor, C… | [proposals/bots.md](proposals/bots.md) |
| OD-BOT3 | Inside Tier 1, does the generic ACP adapter come before the named adapters of Claude Code and Codex (BOT-042)? | Proposed, not ruled: no. Claude Code and Codex first, as the two agents that Anima.ing detects today and that most operators have. The note estimates each of th… | [proposals/bots.md](proposals/bots.md) |
| OD-BOT4 | Does Anima.ing list Crush at all (BOT-044)? | Proposed, not ruled: no, until counsel has read the "Competing Use" clause of its licence. | [proposals/bots.md](proposals/bots.md) |
| OD-BOT5 | May an operator run `agy` or Gemini CLI on a consumer Google login in a pane that he drives by hand (BOT-045)? A reply on the Google tracker of 2026-11-12 says… | Proposed, not ruled: a bot never uses that login (BOT-036). A person's own terminal pane is his own use, and Anima.ing offers no entry for it. | [proposals/bots.md](proposals/bots.md) |
| OD-BOT6 | Do the agents that speak ACP only through a program that their vendor did not publish (Pi with `pi-acp`, Muse with an unofficial adapter) run through the ACP ad… | Proposed, not ruled: no. They stay terminal entries (BOT-039) until the operator rules. | [proposals/bots.md](proposals/bots.md) |
| OD-BOT7 | Do the 14 agents with no checked terms row show as "terms not checked" in the first release (BOT-041, BOT-043), or does a person read their terms first? | Proposed, not ruled: show them only as custom entries with "terms not checked" (Tier 3); read the rows of Tier 2 before the release (BOT-043). | [proposals/bots.md](proposals/bots.md) |
| OD-BOT8 | May Anima.ing start the unmodified `claude` program against a local server that is not Anthropic's (BOT-052)? Anthropic does not support it and has no text that… | Proposed, not ruled: yes, as an option, with the safe values of BOT-052 and a note in the pane that Anthropic does not support it. The traffic never goes to Ant… | [proposals/bots.md](proposals/bots.md) |
| OD-BOT9 | What is the default mode of a fallback (BOT-051): off, ask each time, or start by itself? The operator ruled full auto for the security choices (R-120). Return… | Proposed, not ruled: ask each time. A model with less skill has the same rights, so a start by itself is a choice for each project. | [proposals/bots.md](proposals/bots.md) |
| OD-BOT10 | Does AIS-022 ("must not read ... a usage limit"), which R-135 approved, forbid that the daemon notices the limit message that the program prints in a pane (BOT-… | Proposed, not ruled: no, because it reads no quota or account. The text of AIS-022 then needs one sentence. If the operator says yes, only the manual start stay… | [proposals/bots.md](proposals/bots.md) |
| OD-BOT11 | Which local model does the fallback use? Stephen named Qwen 3.8. The open Qwen3.8 weights are the 27B and the 2.4T. | Proposed, not ruled: Anima.ing sets no default model. It offers the variants that the fit verdict of the device allows (BOT-048), with Qwen3.8-27B first for a d… | [proposals/bots.md](proposals/bots.md) |
| OD-BOT12 | Does the fallback keep the standing permissions of the stopped session (BOT-006, BOT-051)? | Proposed, not ruled: no. It starts with fewer rights (no push to a shared branch, no delete), and the operator adds them. | [proposals/bots.md](proposals/bots.md) |
| OD-BOT13 | Is the server on the same device in the first version (BOT-049), or on any device of the operator? | Proposed, not ruled: the same device. A server on another device waits for the spike of the path from one device to another. | [proposals/bots.md](proposals/bots.md) |
| OD-BOT14 | Does the fallback also cover Codex when it stops on its limit? | Proposed, not ruled: later. Claude Code first. The terms of Codex are unclear (BOT-033). | [proposals/bots.md](proposals/bots.md) |
| OD-BOT21 | Does MCP-006 (no tool runs a command on a device) get a narrow exception, so that an MCP tool can ask the daemon to run a recipe of the catalog (BOT-075)? (yes/… | Open | [proposals/bots.md](proposals/bots.md) |
| OD-BOT22 | May an AI agent start a recipe before the operator confirmed its terms row on that machine (BOT-041, BOT-065)? (yes/no) | Open | [proposals/bots.md](proposals/bots.md) |
| OD-BOT23 | May the operator type a key that lasts (Gemini CLI, Aider, OpenCode) in the secret-entry pane, when R-153 covers a one-time code only (BOT-078)? (yes/no) | Open | [proposals/bots.md](proposals/bots.md) |
| OD-PNL1 | Are Agents and Projects a part of the foundation, or extensions that are on by default? OD-E13 (operator, 2026-11-13) ruled that Projects is not a part of the f… | Proposed, not ruled: extensions that are on by default (PNL-006). It keeps EXT-005 and OD-E13, and the operator can still turn them off. Only EXT-001 and EXT-00… | [proposals/default-panels.md](proposals/default-panels.md) |
| OD-PNL2 | What does "runner" mean in "add runners on easily": an agent runner (BOT-039), a build runner of GitHub Actions (PRO-027), or both? | Proposed, not ruled: both, in one choice on "Add runner" (PNL-005). The two flows stay separate. | [proposals/default-panels.md](proposals/default-panels.md) |
| OD-PNL3 | Which features move out of the foundation as extensions? | Proposed, not ruled: none. Bookshelf (Library, Section 4.14), Maps (Map, Section 4.1) and the others are extensions already. The map canvas component stays shar… | [proposals/default-panels.md](proposals/default-panels.md) |
| OD-PNL4 | Does the first-start question of EXT-008 stay, with Devices, Agents and Projects already selected, or is it removed? | Proposed, not ruled: keep it for a local and a self-hosted first run, and leave it out of the hosted first run (HMD-037). | [proposals/default-panels.md](proposals/default-panels.md) |
| OD-PNL5 | Is the Agents page a new extension, or the extension "AI coding sessions" with its panes and its usage? The text of sessions (AIS-001 to AIS-008) needs the vaul… | Proposed, not ruled: one default-on extension "Agents" with the panes (AIS-009 to AIS-017), the usage (AIS-018 to AIS-024) and the bots (BOT). The session text… | [proposals/default-panels.md](proposals/default-panels.md) |
| OD-PNL6 | Does the phone layout show the same three panels? It has four destinations: Devices, Sessions, Logs and Settings (MOB-010). | Proposed, not ruled: not in the first version. Show Agents and Projects on the phone later. | [proposals/default-panels.md](proposals/default-panels.md) |
| HM-D8 | What does a hub that sleeps cost, and how fast does it start? | Measure on one pool host in step 5. Change the 15 minutes of HMD-024 and the 30 seconds of HMD-026 from the result. | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D2 | Where does the usage line sit? | Set it after the load test of shared-pool-hosting.md. | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D9 | Which sign-in methods does AuthKit show? | GitHub, Google and email with a code; passkeys offered after the first sign-in. | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D10 | Do the sign-in pages use a domain of Anima.ing, for example `auth.anima.ing`? | Not at the launch. Add it when the cost is small against revenue. | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D5 | In which region is the first pool host? | Price a US host before the launch. | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D13 | Does a public client need `client_secret` to refresh a WorkOS token, and does the device request take JSON or a form? | The forms of the shell stay. The refresh of a real token is step 4 of HT-022. Not ruled. | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D11 | How does the device list offer "Add this computer" for a computer that reaches its hosted hub only through the link (step 8 of Section 4.1)? | Add a path from the hub to the device through the link, then offer the computer by its link key. Not ruled. The operator saw the empty device list after the fir… | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D12 | What must the phone app have before a phone can use "Sign in"? | Build M5-1 with M3-1. Not ruled. | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D16 | Can the daemon of a computer of a person (`anima-daemon`, not the daemon in the container of the hub, HMD-046) enrol at a hosted hub and push to it, and by whic… | Open. The operator did not answer this on 2026-11-18. First step, for an agent, with no decision: try the enrolment of the daemon of 0.2.2 at a hosted hub throu… | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| Q1 | Is the root helper (Section 4) accepted as a design? | Yes, after a security review. Ship the basic level first. | [proposals/power-health.md](proposals/power-health.md) |
| Q2 | Does the SSH collector also get these readers, or the daemon only? | Daemon first. Add the basic readers to the SSH collector later (MET-016 makes this cheap). | [proposals/power-health.md](proposals/power-health.md) |
| Q3 | Power meters: which plug types first? | Decide after the research task in P4. | [proposals/power-health.md](proposals/power-health.md) |
| Q4 | Is this a part of Phase 3 metrics work or a new phase? | A new package set after the one-click spikes report, so the two do not compete for the same builders. | [proposals/power-health.md](proposals/power-health.md) |
| Q5 | Does the hub keep the power use of each program over time, or only the latest list? | The latest list only at the start. History is large and its value is not proved. | [proposals/power-health.md](proposals/power-health.md) |
| OD-1 | User interface framework | SvelteKit in static mode. It makes small bundles and works well with Tauri. React is also acceptable. | [spec/design-spec.md](spec/design-spec.md) |
| OD-2 | Hub authentication method | One operator account with a password and a TOTP second factor. Add OIDC in a later version. | [spec/design-spec.md](spec/design-spec.md) |
| OD-3 | Desktop client without a hub | Not supported. A later version can add direct SSH from the Rust layer for offline use. | [spec/design-spec.md](spec/design-spec.md) |
| OD-4 | Hub on Proxmox | The operator can run the hub in an LXC container on the Proxmox node. | [spec/design-spec.md](spec/design-spec.md) |
| OD-5 | Time-series storage | Start with plain PostgreSQL and daily partitions. Add the TimescaleDB extension only if metric queries become slow. | [spec/design-spec.md](spec/design-spec.md) |
| OD-7 | Self-signed certificates in the Tauri web view | Test certificate pinning on macOS (WKWebView) and Linux (WebKitGTK) before Phase 2. If the web view cannot pin a certificate, open the interface in the system b… | [spec/design-spec.md](spec/design-spec.md) |
| OD-8 | Spring library | `motion` (vanilla API). It re-targets springs from the current velocity and works with Svelte and React. | [spec/design-spec.md](spec/design-spec.md) |
| OD-9 | Hub location | Hostinger VPS. The hub continues to operate when a site loses power or internet, and it can report that the site is down. Risk: a VPS breach exposes the device… | [spec/design-spec.md](spec/design-spec.md) |
| OD-10 | Headscale or the Tailscale hosted control server | Headscale, because it keeps the control server under the control of the operator. Headscale does not have "Tailnet Lock" (devices must approve each new node). T… | [spec/design-spec.md](spec/design-spec.md) |
| OD-11 | Gateway location | The gateway runs with the hub on the VPS. A session to a device at a site then goes VPS to site. If the latency is too high, a later version can run a gateway a… | [spec/design-spec.md](spec/design-spec.md) |
| OD-12 | Build trigger | Phase 2: the operator starts each release (Section 3.10). A later version can let the hub poll `main` and queue a release for each new commit. | [spec/design-spec.md](spec/design-spec.md) |
| OD-14 | Compute workers (R-110) | The hub keeps a job queue for compute work. A job is a WebGPU (WGSL) compute shader with WebAssembly, so every client runs it: desktop, web and iOS (WebKit on i… | [spec/design-spec.md](spec/design-spec.md) |
| OD-6 | Central log search | Not necessary now. A later version can send the process log to Loki or a similar system. | [spec/design-spec.md](spec/design-spec.md) |
| OD-15 | Contract change for the actor kinds (Section 3.8.1) | A programme ruling must approve the additions to K1, K2 and K7 before the build (CON-001). The ruling must also give the value of `AuditEntry.operatorName` for… | [spec/design-spec.md](spec/design-spec.md) |
| OD-19 | How the website reads the feed (SITE-003) | Two options: a build step of the website at each release, or a script in the page. A script in the page needs cross-origin read access to the release host, whic… | [spec/design-spec.md](spec/design-spec.md) |
| OD-20 | Domain name and host of the website (SITE-011) | Host decided: Vercel (operator, 2026-11-13). Domain open: the operator decides. The site is built and not deployed. | [spec/design-spec.md](spec/design-spec.md) |
| OD-22 | What the website can say about the paid hosted option (SITE-012; work-plan.md OD-W2, other-operators.md) | Nothing now. The proposal has no requirements. Add text only after the operator approves the words. | [spec/design-spec.md](spec/design-spec.md) |
| OD-23 | Does `anima-daemon` also ship by Homebrew or Cargo (INS-008, INS-010, INS-011; R-151) | Proposed, not ruled. The operator ruled for the command line because it "can operate alone" (R-151). The daemon is a separate program with its own install path… | [spec/design-spec.md](spec/design-spec.md) |
| OD-24 | Which file does the Homebrew formula install, and does the release publish it with its SHA-256 (INS-010, INS-012) | Proposed, not ruled. The release would publish a standalone `anima` file for each target, with its SHA-256 in the release record (ROL-003). The formula would in… | [spec/design-spec.md](spec/design-spec.md) |
| OD-25 | How the command line alone signs in to a hosted hub (INS-013; HMD-032, HMD-038, R-129) | Proposed, not ruled. `anima auth login` asks for a user name, a password and an authenticator code (CMD-010), and the hosted hub has no such account. The hosted… | [spec/design-spec.md](spec/design-spec.md) |
| OD-26 | Does the formula go in the operator's own tap before Homebrew's own repository (INS-010) | Proposed, not ruled. The tap is the Marketing Bot's word, not the operator's. A tap needs no review and the operator controls it. Homebrew's own repository need… | [spec/design-spec.md](spec/design-spec.md) |

## 3. Documents that wait for an approval (25)

| Document | Kind | State |
| --- | --- | --- |
| [plans/one-click/s12-spec-delta.md](plans/one-click/s12-spec-delta.md) | Proposal: spec delta for packaging | Draft |
| [plans/one-click/s3-spec-delta.md](plans/one-click/s3-spec-delta.md) | Proposal: spec delta for the bundled PostgreSQL | Draft |
| [plans/one-click/s4-spec-delta.md](plans/one-click/s4-spec-delta.md) | Proposal: spec delta for the hub as a background program | Draft |
| [plans/one-click/s6-spec-delta.md](plans/one-click/s6-spec-delta.md) | Proposal: spec delta for the first run | Draft |
| [plans/ui-redesign-direction.md](plans/ui-redesign-direction.md) | Plan: direction of the redesign | Not checked |
| [plans/ui-redesign-plan.md](plans/ui-redesign-plan.md) | Plan | Not checked |
| [proposals/add-this-machine.md](proposals/add-this-machine.md) | Proposal | Draft. Deferred; partly built |
| [proposals/agent-runs.md](proposals/agent-runs.md) | Proposal | Draft |
| [proposals/ai-direction.md](proposals/ai-direction.md) | Proposal | Draft |
| [proposals/bots.md](proposals/bots.md) | Proposal, with a task table | Draft |
| [proposals/cloud-machines.md](proposals/cloud-machines.md) | Proposal | Draft |
| [proposals/coordinator-standby.md](proposals/coordinator-standby.md) | Proposal | Direction decided |
| [proposals/cve-watch-and-network-awareness.md](proposals/cve-watch-and-network-awareness.md) | Proposal | Draft |
| [proposals/default-panels.md](proposals/default-panels.md) | Proposal | Draft |
| [proposals/first-run-setup.md](proposals/first-run-setup.md) | Proposal | Draft. Deferred; not built |
| [proposals/in-app-enrolment.md](proposals/in-app-enrolment.md) | Proposal | Draft. Deferred; partly built |
| [proposals/jobs.md](proposals/jobs.md) | Proposal | Direction decided |
| [proposals/machine-profiles.md](proposals/machine-profiles.md) | Proposal | Draft |
| [proposals/new-sources.md](proposals/new-sources.md) | Proposal | Draft |
| [proposals/other-operators.md](proposals/other-operators.md) | Proposal | Direction decided |
| [proposals/power-health.md](proposals/power-health.md) | Proposal | Draft |
| [proposals/release-four-platforms.md](proposals/release-four-platforms.md) | Proposal, with a task table | Direction decided |
| [proposals/shared-pool-hosting.md](proposals/shared-pool-hosting.md) | Proposal | Draft |
| [proposals/shell-prompt.md](proposals/shell-prompt.md) | Proposal | Draft |
| [proposals/watchdog.md](proposals/watchdog.md) | Proposal, with a task table | Direction decided |

## 4. Checks that only the operator can do (76)

| Check | What |
| --- | --- |
| HT-001 | `anima snapshot` against GitHub |
| HT-002 | the tool inventory from a daemon to the command line |
| HT-003 | the tool inventory in an MCP client |
| HT-004 | no window of Apple on a Mac without the developer tools |
| HT-005 | the tool inventory of a daemon that runs as a system unit |
| HT-006 | the probes of `claude` and `codex` leave the account alone |
| HT-007 | the pinned Rust version on the Linux computer |
| HT-008 | a macOS release build with the pinned Rust version |
| HT-009 | the hub image builds with Rust 1.100.0 |
| HT-010 | merge a pull request, and a set of pull requests, on GitHub |
| HT-011 | run a workflow again, cancel a run and start a workflow on GitHub |
| HT-012 | set a commit status on GitHub |
| HT-013 | an API key cannot merge, control a run or read a file |
| HT-014 | the issue tools of the MCP server, and a workflow that a comment starts |
| HT-015 | the Work, Code and History tabs of a project in the client |
| HT-016 | a release build on the Linux builder puts the bundle in its own clone |
| HT-017 | "This machine" with the Anima VPN and NetBird together |
| HT-018 | each machine with the old program gets `anima-daemon` and enrols again |
| HT-019 | the VPN role `daemon` on the VPS |
| HT-020 | the desktop application builds on Linux with no lint error (GH-52) |
| HT-021 | the delay of the session picture in the Linux application (GH-54) |
| HT-022 | the first real sign-in with the Anima.ing account (HM-T1 to HM-T7) |
| HT-027 | the computer that signs in is a device of the hosted hub (HM-T8) |
| HT-028 | the hosted hub is a device of its own device list (HM-T9) |
| HT-029 | the computer is Up while it is connected to the link (HM-T11) |
| HT-031 | a hosted hub reaches SSH of a computer on its link (HM-T12) |
| HT-023 | the hash chain of the audit log on the real hub (GH-2 L0, LOG-021 to LOG-028) |
| HT-024 | an SSH session on a phone, with the key row |
| HT-025 | a phone joins a hub with an invite, through the link in the app |
| O-1 | RDP: the certificate, the picture, resize, copy and paste, Ctrl+Alt+Delete |
| O-2 | RDP: a changed certificate stops the session; the Warning badge |
| O-4 | SFTP: list, upload, download, the refusals, the audit entries |
| O-5 | Wake-on-LAN through the site relay |
| O-6 | The web view on macOS: fingerprint, a changed certificate, the Down page |
| O-7 | OD-7 on Linux: the trust hook spike. Also the frame time of the Devices page |
| O-8 | SSH metrics: a new point each 30 s, the bars on the card |
| O-9 | Docker, SNMP, Tailscale ping and systemd checks |
| P-1 | The deployed image and the compose file are the accepted ones |
| P-2 | The T4 probe, with no key |
| P-3 | Browsers and sign-in on macOS; the cookie attributes |
| P-4 | A browser on Linux, and the desktop application |
| P-5 | The web client: the device list and the detail view |
| P-6 | SSH and VNC sessions in the web client |
| P-7 | The notification channels are entered |
| P-8 | Channels are selected on a check |
| P-9 | "Send test" on each channel |
| P-10 | A Down and Up cycle sends the messages |
| P-11 | Maintenance sends no message |
| P-12 | A live RDAP check |
| P-13 | GPU metrics on the Apple silicon Mac |
| P-14 | Optional: a real Prometheus scrape |
| P-15 | Optional: the API key checks in Settings |
| P-17 | The desktop application: the update banner |
| P-16 | Last: the T3 log scan of the live hub log |
| S1-T14 | The link between two machines: the direct rate, the first-connection time, relayed Screen Sharing, the firewall prompt. It sets the keepalive numbers (LNK-009) |
| S2-T9 | The native VNC gateway against macOS Screen Sharing and TigerVNC, with the figures |
| S2-RDP | The native RDP gateway against a GNOME Remote Desktop host |
| S3-T10 | The database tree recipe on the Linux builder |
| S3-T12 | Signature and notary check of the database tree |
| S3-T13 | No data loss after a power cut |
| S3-T14 | The database on a real Linux desktop |
| S4-T10 | The systemd user unit on Linux. The Local Network prompt on a Mac |
| S6-1 | The first run in the desktop window: create a hub, TOTP, the key copy, an invite |
| S6-2 | Join from a second Mac, then revoke |
| S6-3 | The first run on Linux with the systemd user unit; finding F-10 |
| S6-4 | No port mapping on a real router; no TCP port answers from outside |
| S6-5 | A relayed path shows `RELAYED` |
| S8-1 to S8-8 | Stay awake on macOS: the tray, keep awake, the set-up, the lid, quit and failure, the battery floor, remove, eight hours (8 checks) |
| S8-9 to S8-13 | Stay awake on Linux: the tray, keep awake, the lid on GNOME, quit, the battery (5 checks) |
| CLI-1 | `anima mcp` with a real MCP client: the tool list, one read, one change, and the approval request for a tool in state "ask" (MCP-013) |
| CLI-2 | The MCP section in Settings in the real desktop application |
| CLI-3 | Linux: the crate compiles; the Secret Service token store and its file fallback |
| CLI-4 | macOS: the Keychain item and its access control |
| CLI-5 | TLS against a hub with a bad certificate |
| CLI-6 | The list of GH-60 against the production hub: sign-in, `anima ssh`, `anima files`, a confirmation prompt |
| REL-1 | A real release confirms the signature, the notarization and the AppImage launcher (OD-16 to OD-18) |

<!-- end of the generated queue -->
