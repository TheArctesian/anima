---
kind: Proposal, with a task table
state: Draft
evidence: Version 2 (2026-11-18). The goal follows the first-hand words of the operator of 2026-11-18 22:32 PT (not a ruling). The four answers of 17:03 PT are relayed by Anima Product Ideas and not first-hand (OD-AXT1). No ruling. No requirement is approved and no task is built. Checked against origin/main at `cdd720ef`. Serves row 4.21 of the priorities, a placeholder that waits for OD-AXT1 and OD-AXT11.
---

# Proposal: agent tools as extensions (an extension host for tools on the operator's machines)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this proposal (PLN-005).

| Item | Value |
| --- | --- |
| Document | Proposal: agent tools as extensions. A host that opens a tool of another vendor, which runs on a machine of the operator, in a pane of the client |
| Version | 2 |
| Date | 2026-11-18 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | The header of this document holds the state (PLN-020). History: Draft. Made from the idea `agent-tools-as-extensions` (PLN-018); the commit that adds this document deletes the idea folder. Version 1 had relayed direction only. Version 2 changes the goal to the first-hand direction of the operator (2026-11-18, 22:32 PT): support the open-source projects and gain their users. It adds Section 6.7 (AXT-028 to AXT-043), the input of Anima Marketing Bot, OD-AXT12 to OD-AXT15 and tasks AX13 to AX16. No ruling. |
| Serves | Row 4.21 of [priorities.md](../plan/priorities.md) (a new row, by the relayed answer 4 of the operator; placeholder until OD-AXT1 and OD-AXT11) |
| Related | [extensions-plan.md](../plans/extensions-plan.md) Sections 2.8 (EXT-009), 2.8.1, 3.4 and 4.12 (AIS-009 to AIS-024); [bots.md](bots.md) (BOT-001 to BOT-078); the design specification Section 6.3.4 (WEB-001 to WEB-010); [hosted-mode.md](hosted-mode.md) (HMD-048, HMD-049); [hosted-applications-plan.md](../plans/hosted-applications-plan.md) (task H1, fixed commands through SSH) |
| Identifiers | AXT-001 to AXT-043, OD-AXT1 to OD-AXT15, tasks AX0 to AX16 |

## 1. Goal

Make Anima.ing the best place to run open-source agent tools remotely. Anima.ing supports the projects of these tools, and gains their users.

- **Support for the projects.** Anima.ing gives credit to each project and links its repository, its documents, its licence and its sponsor page. It installs only the official release of the project. It does not fork or rebrand a tool. A fix that Anima.ing needs goes to the project as a pull request. The tool keeps working without Anima.ing, so there is no lock-in (Section 6.7).
- **Gain of their users.** These tools run on the machine of the user, and the difficult part is to reach them from a different place. Anima.ing already does that, so remote access is the hook: "open your Hermes dashboard from your phone or your laptop". One click in "Add an agent" installs the tool and opens it. A project can choose to list Anima.ing in its own documents (OD-AXT12).

Do not build a second Herdr, a second T3 Code or a second assistant bot in Anima.ing. Anima.ing gives the pane, the grant, the state, the health and the install. It does not give the tool. The first tools are Hermes Agent (a web page on a local port, and a terminal screen) and Herdr (a terminal program). T3 Code is not in the catalogue (Section 5, constraint C2; OD-AXT15).

The goal comes from the operator's first-hand words (2026-11-18, 22:32 PT, Grok Bot chat, unchanged), and his choice "Yes, reframe the goal around supporting the projects and gaining their users." This is the direction of the goal. It is not a ruling, and OD-AXT1 stays Open:

> I mean its more about how do I support all of these great other open source apps and also gian their users

The first words of the operator (2026-11-18, 16:57 PT, relayed by an agent, unchanged):

> also i’m thinking instead of rebuilding herdr or t3 code or grok bot (we should just use hermes instead) we integrate them all as extensions such that they can be opened in a pane in the app or something like that

## 2. The direction of the operator (relayed, awaiting first-hand confirmation)

Anima Product Ideas relayed these four answers of the operator (2026-11-18, 17:03 PT). They are the working direction of this draft. They are **relayed, awaiting first-hand confirmation** (OD-AXT1, OD-AXT2, OD-AXT3). No ruling records them.

| Number | Relayed answer | Where this draft uses it |
| --- | --- | --- |
| A1 | Version 1 is a real extension host, not tabs only. Effort L (an estimate). | Sections 6 and 9 |
| A2 | Replace the native Herdr-style panes and the native bot runtime. Keep the usage counts, the status and the install. | Section 7; OD-AXT2, OD-AXT3 |
| A3 | Suggest Hermes first in "Add an agent". Never install it without the operator's click. | AXT-021; OD-AXT5 |
| A4 | Give the work a new row of its own in the priorities. | Row 4.21; OD-AXT11 |

## 3. Overlap check (PLN-004)

Files read: [plan/ownership.md](../plan/ownership.md), [plan/register.md](../plan/register.md), [deferred.md](../deferred.md), [bots.md](bots.md) (version 5), [cli-mcp-plan.md](../plans/cli-mcp-plan.md) Section 11, [priorities.md](../plan/priorities.md) rows 4.4, 4.7, 4.8, 4.9 and 4.20, and the EXT requirements of [extensions-plan.md](../plans/extensions-plan.md). Commit `cdd720ef`, 2026-11-18. This is a keyword search and a read of the named sections, not a full read of each document.

| Subject | Owner today | Result |
| --- | --- | --- |
| A tool of another vendor that runs on the operator's machine and opens in a pane | None. No row of ownership.md names it. | This proposal owns it (AXT). |
| Community extensions and their isolation and permissions design | [extensions-plan.md](../plans/extensions-plan.md) EXT-009, Sections 2.8.1 and 3.4 (draft text) | Not changed. EXT-009 holds community extensions until the 15 are built. Section 3.4.4 says that "a hub runtime for third-party code needs a separate design under EXT-009". This proposal is that design for one narrow case: tools that run on the operator's own machine, with manifests that Anima.ing writes (OD-AXT4). |
| Agent panes, Herdr-style (AIS-009 to AIS-017) | extensions-plan.md Section 4.12.1, direction of 2026-11-16, no work package | Relayed answer A2 reverses it. A ruling must withdraw it (OD-AXT2). The change is written in Section 7 of this document and is not made in the plan. |
| Usage counts (AIS-018 to AIS-024) | extensions-plan.md Section 4.12.0, approved (R-135), built | Kept. Not changed. |
| The native bot runtime (BOT-001 to BOT-018, BOT-025 to BOT-028) | [bots.md](bots.md), Draft | Relayed answer A2 replaces it with Hermes. A ruling must withdraw it (OD-AXT3). |
| Agent states, install, catalogue, vendor terms (BOT-019 to BOT-024, BOT-029 to BOT-046, BOT-058 to BOT-078) | bots.md, Draft | Kept. This proposal adds two catalogue entries (Hermes, Herdr) through bots.md (AXT-020 to AXT-024). PLN-004: the install stays in bots.md and is not copied. |
| The web view in a session tab | design specification Section 6.3.4 (WEB-001 to WEB-010) | Used. AXT-009 extends WEB-006 (one store for each tool) and reaches a loopback port, which WEB-004 cannot. |
| The terminal in the client (SSH from the hub) | design specification, R-062, K4 Section 8 | Used for the terminal kind (AXT-014). |
| Fixed commands that the hub runs on a device through SSH | [hosted-applications-plan.md](../plans/hosted-applications-plan.md), task H1 | Used as the pattern for start and stop (AXT-007). |
| The link of a hosted hub and its list of open ports | [hosted-mode.md](hosted-mode.md) HMD-048, HMD-049 | Not changed if OD-AXT6 is choice (a). |
| The terminal screen `anima tui` | cli-mcp-plan.md Section 11 (TUI) | No overlap. `anima tui` gives the terminal to `anima ssh`; it does not open tools. |
| Ideas folder | [ideas/](../ideas/README.md): `agent-state-hooks` (the open status protocol of Herdr), `needs-you-inbox`, `agent-execution-on-managed-machine` | Neighbours, not overlaps. OD-AXT10 points to `agent-state-hooks`. |
| Deferred list | deferred.md | No item (searched Herdr, T3 Code, Hermes, extension host, third-party). |

Decision on the owner. PLN-004 says to change the owning plan. No plan owns "a tool of another vendor in a pane". The extensions plan owns community extensions, but its Section 3.4.4 asks for a separate design, and its text there is a draft for review. bots.md owns the install, and this proposal changes it only through two catalogue entries. So this is a new proposal with one new concern row in ownership.md. The changes to extensions-plan.md (4.12.1) and to bots.md (the runtime) are written in Section 7 and wait for the rulings of OD-AXT2 and OD-AXT3. Until then each of the two documents has only a pointer note.

## 4. Current state in the code (origin/main at `cdd720ef`, 2026-11-18)

| Piece | State | Evidence |
| --- | --- | --- |
| Native agent panes (AIS-009 to AIS-017): a session server in the daemon, pane states, the page of all panes | Not built. No AIS-009 to AIS-017 identifier is in the code. The daemon has no terminal server and no `tmux` code. | `rg 'AIS-009\|AIS-01[0-7]' crates apps contracts link` finds nothing; `crates/daemon/src` has `push`, `readers`, `sampling` and no session server |
| Native bot runtime (BOT-001 to BOT-018) | Not built. No `BOT-` identifier and no bot domain in the hub, the daemon or the command line. | `rg 'BOT-0' crates apps contracts` finds nothing |
| Usage counts (AIS-018 to AIS-024) | Built. Waits for HT-026. | `crates/daemon-core/src/agent_usage/`, `crates/hub/src/fleet/agent_usage/`, `apps/client/src/routes/agents/usage/` |
| Status of agents (BOT-022 to BOT-024) | Not built. The daemon finds `claude` and `codex` and their versions only, with an empty home folder. | `crates/daemon-core/src/inventory/tools.rs` (`ToolName`, 13 values; Claude and Codex are the only agents) |
| Install of agents (BOT-058 to BOT-078) | Not built. Tasks BI0 to BI11 are Not started. The work route (task W1) is Not started. | bots.md, "Tasks"; projects-and-builds-plan.md, task W1 |
| Extension loading | No package, manifest or third-party code is loaded. The hub has a fixed registry of the 15 extensions and their switches. | `crates/hub/src/engine/extensions/registry.rs` (15 keys), `extensions.rs` ("The registry lists the fifteen"); K1 type `Extension` in `contracts/schema.graphql` |
| Web view in a session tab (WEB-004 to WEB-008) | Built in the desktop application: child web views, one store for each device, no IPC, only `http` and `https`. Pinning works on macOS only; on Linux an `https` address opens in the system browser and only `http` is embedded. | `apps/desktop/src-tauri/src/web_view.rs` (`pinning_available`), `apps/client/src/lib/web-interfaces/` |
| A path from the client to a loopback port of a remote device | None. WEB-004 connects directly to the device. A loopback port (`127.0.0.1:9119`) of another machine cannot be reached that way. | design specification WEB-004 |
| The link of a hosted hub | It reaches only ports 22, 5900 and (Windows) 3389 of a computer, on its loopback address, when the person turned the port on. | `link/ports/ports.go` (`Allowed`), HMD-048, HMD-049 |
| The hub's SSH: forwarded channels | The terminal refuses each channel that the device opens, `direct-tcpip` included (K4 8.4). The hub opens no `direct-tcpip` channel to a device. | `crates/hub/src/fleet/sessions/terminal/tests/wire/channels.rs` (`k4_8_4_every_channel_the_device_opens_is_refused`) |
| Fixed commands through SSH | Built for containers (task H1). | `crates/hub/src/work/containers/` |
| Hermes, Herdr and T3 Code in the code | None. | `rg -i 'hermes\|herdr\|t3 code' crates apps` finds nothing |

Not verified: whether the Hermes dashboard and Herdr work in a Tauri child web view and in a terminal pane of the client; whether the dashboard refuses a request that has a different `Host` header or `Origin`; whether `sshd` on a stock macOS or Linux machine permits a `direct-tcpip` channel to the loopback address for the operator's user (OpenSSH permits it by default; not read on these machines); the install commands of Hermes and Herdr; the attach command of Herdr. Task AX0 measures these.

## 5. Constraints (they bind each entry of the catalogue)

- **C1. Hermes and the Claude login.** The Hermes documents say that Hermes can read the Claude Code login file (`~/.claude/.credentials.json`) in its credential pools (hermes-agent docs, `user-guide/features/credential-pools.md`, read 2026-11-18 by Anima Product Ideas; not read again for this draft). BOT-003 says that a bot "must never take or store the login of a subscription". BOT-020 and BOT-023 forbid Anima.ing to read, copy, move, keep or relay a login. BOT-021 and BOT-032 allow Claude only as the unmodified `claude` program, not through an SDK and not through a token. BOT-025 forbids a rule that rotates between logins of one vendor or pools logins. So the Hermes entry must not offer, preselect or script the Claude subscription login or the Claude Code login file as a source of Hermes, and must say so (AXT-022). Anima.ing cannot stop a program of the operator from reading a file of the operator. The rule is a warning, a confirmation and an audit event, not a technical block (OD-AXT9).
- **C2. T3 Code is not in the catalogue.** T3 Code drives Claude through the Agent SDK and keeps a ChatGPT login in its server (bots.md, "The claim of T3 Code, checked"). BOT-021 and BOT-032 ban the SDK path for a subscription, and BOT-033 bans the server-side ChatGPT login for a hosted service. The section "Not in scope" of the install in bots.md keeps T3 Code out. So T3 Code must not have a manifest, a recipe or a suggestion (AXT-020). A later version can read this again only with a new terms row (BOT-038) and a ruling.
- **C3. No code of other people in the hub.** EXT-009: "The hub must not run the code of other people until a design for isolation and permissions is approved." The host runs each tool on the operator's machine, as the operator's user, from the operator's install (AXT-003).
- **C4. Login rules of the install.** BOT-066: each sign-in is on the machine, in the flow of the vendor, by the operator. BOT-078: a recipe must not take, show, store or relay a key.
- **C5. Order and contracts.** CMD-030: hub API, `anima` command, MCP tool, client. K1 to K8 are frozen; a ruling changes them (CON-001). A plan, a contract and a ruling merge to `main` before the code (R-155).

## 6. Requirements (proposed, not approved)

All requirements here are proposed. None is approved, and none is built until the operator approves it with a ruling (PLN-007). Terms: a **tool** is a program of another vendor. A **tool extension** is the manifest of one tool and the grants of the operator for it. A **pane** is a session tab of the client that shows one tool of one device. The **web kind** serves a web page on a loopback port of the device. The **terminal kind** runs in a terminal.

### 6.1 The host and the manifest

- **AXT-001.** A tool extension must be a manifest that Anima.ing writes and ships with a release of the client and the daemon. It must hold: the key, the name, the address of the vendor, the licence, the supported versions, the kind (web or terminal), the start command, the stop command, the loopback port (web kind), the attach command (terminal kind), the key of the install recipe of bots.md (BOT-060) or "installed by the operator", the key of the vendor terms row (BOT-038), and the credential notes. A command must be a program and a list of arguments (BOT-061). A manifest holds no code.
- **AXT-002.** In the first version the catalogue must hold only manifests that Anima.ing writes. The host must not load a manifest from a file, from the hub or from the network at run time (as BOT-060). A manifest that a person outside Anima.ing writes is a community extension and waits for EXT-009 (OD-AXT4).
- **AXT-003.** The host must not run the code of a tool in the hub, in the container of the hub, or in the main web view of the client. The tool runs on a machine of the operator, as the operator's user. The hosted hub and the images of rented machines must carry no tool (as BOT-031).
- **AXT-004.** The client must show the switch "Agent tools" in Settings, Extensions. The default is off (EXT-001). Its data class is C: it keeps grants and states, and no record text. It is not one of the 15 extensions of EXT-009 (OD-AXT8).
- **AXT-005.** The operator must turn on each tool extension for each device. The grant names the device, the extension key and the version of the manifest. A new manifest version that changes a command or the port must ask for the grant again. The operator can revoke a grant at any time. A revoke must close each open pane of that grant.

### 6.2 State, start and stop

- **AXT-006.** The device page must show, for each granted tool, one state: not installed, installed and stopped, running, or not checked. "Installed" comes from the tool inventory of the daemon (CAP-023, a new `ToolName` for each tool). "Running" comes from a check of the loopback port through the path of AXT-009 (web kind), or from the stop command's status form (terminal kind). The check must read no login (BOT-023).
- **AXT-007.** Start and stop must run the fixed start and stop commands of the manifest through the SSH connection of the hub, as task H1 does for containers. The start must run the tool in a session that stays alive (a `tmux` session that Anima.ing names and owns, as BOT-069), not as a system service and not at login. The operator starts it in the app or in the command line. A schedule, an event, a bot or an AI agent must not start a tool in the first version.
- **AXT-008.** A web tool must listen on the loopback address of its device only. The start command must give the tool's own option for this when the tool has one. The host must not open a port of the device on another address. This narrows BOT-064 for tools: the loopback address, never the address on the Anima.ing network and never all addresses.

### 6.3 The pane and its isolation

- **AXT-009.** A web pane must reach the loopback port of the device through the hub: the hub opens one SSH `direct-tcpip` channel to `127.0.0.1:<port>` on the device for the pane, and relays the bytes to the client over a WebSocket with a session ticket (as K4 Section 8). This works on a hub on the VPN and on a hosted hub (port 22 through the link, HMD-048). The device opens no new port (OD-AXT6).
- **AXT-010.** On the client, the pane must be a Tauri child web view (WEB-004, WEB-005) with: its own data store for each pair of device and tool (this extends WEB-006); no Tauri commands (WEB-007); navigation only to the origin of the tool; each other address opens in the system browser; no popup in the client; no download in the first version. The Anima.ing header of the pane names the tool, the device and the grant, and has the actions Reload, Stop and Revoke.
- **AXT-011.** The client must give the web view a loopback address of the client computer that only that pane can use. The first design is a listener on `127.0.0.1` on a random port that accepts a request only with a one-time pane token, then a cookie of that pane, and that removes the token and the cookie before it relays the request. [NEEDS CLARIFICATION: task AX0 measures this design and a custom scheme of Tauri with WebSocket; the design that passes is the one for AX8.]
- **AXT-012.** Only the operator, with a login or a browser token, can open a pane. An API key, an MCP tool and an AI agent must not open a pane, read its content or start a tool (MCP-004, MCP-006). [NEEDS CLARIFICATION: what a member of a team hub can open, when members exist (MBR).]
- **AXT-013.** A tool that trusts requests from its own loopback address (for example a dashboard with no sign-in on `127.0.0.1`) is open to each person who can open its pane. The grant screen must say so, and must name the sign-in of the tool when the tool has one.
- **AXT-014.** The terminal kind must open the SSH terminal of the device (R-062) and run the attach command of the manifest in place of the login shell (Herdr, or `hermes --tui`). It needs no port. It changes the rule of K4 Section 8 that the command is the login shell (GW-013), for a manifest command only.
- **AXT-015.** The hub must relay the bytes of a pane and must not record or log them (RA-008, LOG-007). The pane must show this notice: "This page goes through your hub as it is. Your hub does not keep it." (OD-AXT7). A pane that is open must close when the grant is revoked, the switch is set off, or the session of the operator ends.
- **AXT-016.** The audit log must hold one event for each of: grant, revoke, tool started, tool stopped, pane opened, pane closed, and the confirmation of AXT-022. Each event names the actor, the device, the tool key, the manifest version and the time. No event holds an address path, page content, a token or an account name (LOG-007).
- **AXT-017.** When the tool does not answer, the pane must show an error page with the reason and the actions Start and Reload (as WEB-010). A failure of a tool must leave the client usable.
- **AXT-018.** The first version is for the desktop client on macOS and Linux. The phone clients and the web client do not open a pane.

### 6.4 Interfaces (CMD-030 order)

- **AXT-019.** The function must have, in this order: the hub API (the catalogue, the grants, the states, start, stop, and the pane ticket); the commands `anima tools list`, `status`, `grant`, `revoke`, `start`, `stop` and, for the terminal kind only, `open`; one MCP read tool for the states (`anima_tools_list`), with no tool that starts, stops or opens; then the client. Each new root field needs its command (CMD-031). [NEEDS CLARIFICATION: the names are a suggestion for the interface review.]

### 6.5 The catalogue of the first version

- **AXT-020.** The first catalogue must hold two tools: Hermes Agent (web kind: `hermes dashboard` on port 9119; terminal kind: `hermes --tui`) and Herdr (terminal kind). T3 Code must not be in the catalogue (constraint C2; BOT-021, BOT-032, BOT-033).
- **AXT-021.** "Add an agent" (BOT-058) must show Hermes first, with the label "Suggested". The host must never install Hermes without the operator's click: no install by default, at the first run, by a schedule, by an event or by a bot (relayed answer A3). [NEEDS CLARIFICATION: OD-AXT5, an install through MCP in the state `allow`.]
- **AXT-022.** The Hermes entry must not offer, preselect or script a Claude subscription login or the Claude Code login file as a credential source (constraint C1; BOT-003, BOT-020, BOT-021, BOT-023, BOT-025, BOT-032, BOT-078). Its review screen must say: "Hermes can read the Claude Code login file of this machine. Do not give Hermes that source. Set an API key, a local model or another provider in Hermes on this machine." The operator must confirm once for each device. The confirmation goes in the audit log.
- **AXT-023.** The Hermes install must be a recipe in the one catalogue of bots.md (BOT-060), with its vendor terms row (BOT-038) or "terms not checked" (BOT-041). It must run as the operator's user, with no root (BOT-063). [NEEDS CLARIFICATION: the install and update commands of Hermes, not verified.]
- **AXT-024.** The Herdr install must be a recipe in the same catalogue. Herdr is a terminal program, not an agent. Its recipe gets no sign-in step. [NEEDS CLARIFICATION: the install and attach commands of Herdr, not verified.]
- **AXT-025.** The desktop application of Herdr, the phone applications of Herdr and the desktop application of Hermes are not in the first version. The vendor says that the Herdr terminal program and the Herdr desktop application "don't share agents".

### 6.6 What stays

- **AXT-026.** The usage counts (AIS-018 to AIS-024), the agent states (BOT-022 to BOT-024) and the install (BOT-058 to BOT-078) stay as they are. When Herdr is installed on a device, BOT-069 must start an agent in a Herdr pane; otherwise in the `tmux` session of BOT-069.
- **AXT-027.** Agent tools must be free on every plan, including self-host (as AIS-016 and BOT-009).

### 6.7 Support for the projects and growth

Anima Marketing Bot gave the input for this section (2026-11-18). It holds no price and no funding fact. The pages of the public site are in the private site repository `anima-site`; this document records the requirement and its owner only.

**Support for the projects.**

- **AXT-028.** Each catalogue entry must link the repository, the documents and the licence of the project, and its sponsor page when the project has one (OD-AXT13). The manifest holds these addresses (AXT-001).
- **AXT-029.** A recipe must install only the official release of the project, from the release channel that the project names (BOT-062).
- **AXT-030.** Anima.ing must not fork or rebrand a tool. It must keep the name and the version of the tool as the project writes them. Before Anima.ing uses the name or the logo of a tool, a person must read the licence and the name and logo rules of the project. Where the rules are not clear, Anima.ing shows no logo.
- **AXT-031.** A fix that Anima.ing needs in a tool (for example a header so that the page opens in a pane) must go to the project as a pull request. Anima.ing must not keep a private patch. If the project does not take the fix, the entry waits; Anima.ing does not fork.
- **AXT-032.** The tool must keep working without Anima.ing. A revoke, a removal of Anima.ing or a stop of the hub must leave the tool, its settings and its data on the machine as they are (BOT-072).
- **AXT-033.** The pane header must show the attribution: the name of the project, its licence, and the links of AXT-028. The links open in the system browser.
- **AXT-034.** Each text about a tool must say "works with". It must not say "official partner", must not use exclusive words, and must not rank one tool against a different tool. It must not say that a project endorses Anima.ing unless the project says so.
- **AXT-035.** Each recipe must give the versions that it supports, or run a compatibility check before the install. When a new release of a project breaks the recipe, the page must say so, the recipe must stop, and a person must report it to the project. The page then says "Reported to the project" with the link of the report.

**Growth.**

- **AXT-036.** The public site must have one catalogue page for each tool (for example `/agents/hermes`): what the tool is, the credit and the links of AXT-028, and a guide "Run it remotely". `llms.txt` must list these pages. Owner: the site repository `anima-site` and the design specification Section 3.12 (SITE). This document does not build the pages.
- **AXT-037.** Each catalogue page must have an "Edit this page" link or a contact for the maintainer. When a maintainer asks, Anima.ing must remove the listing within one day.
- **AXT-038.** Anima.ing can give maintainers an optional badge or snippet, "Run it remotely with Anima.ing". Its link must carry a source tag `?ref=<tool>`. A project is never asked to add it as a condition (OD-AXT12).
- **AXT-039.** Each tool must have a short demo clip: open the tool from a phone or a laptop in a different place. A project can use the clip.
- **AXT-040.** The uptime and metrics view must show the health of each granted tool, with the states of AXT-006 and BOT-024, for example "Hermes Agent is down on office-mac". A tool that goes down can send an alert through the notification system (the NOT requirements of the design specification), and the operator can mute it.
- **AXT-041.** Each text about security must say how access works: the private network or the relay of the hub, and the sign-in of the operator. It must say who can reach the dashboard of a tool (AXT-012, AXT-013). It must never say that the traffic is encrypted from end to end, because the hub carries the traffic of a pane as plain text (OD-AXT7).
- **AXT-042.** Anima.ing can measure growth only with counts. A signal must hold no text, no address path, no account name and no content of a tool (LOG-007). The signals are in the table below.
- **AXT-043.** The offer of a docs line "Run it remotely with Anima.ing" to a project must come only after the public release and after a contribution that the project merged (OD-AXT12). It is an offer. It is never a condition of a listing.

**Signals (AXT-042).** Counts only. "New telemetry" means a count that the product does not send today.

| Signal | Source | New telemetry? |
| --- | --- | --- |
| Completed installs for each tool for each week (not clicks) | The install events of BOT-073 on each hub | Yes. A hub keeps the events, but no count goes from a hub to Anima.ing today. [NEEDS CLARIFICATION: how a count leaves a hosted hub and a self-hosted hub; opt-in for a self-hosted hub.] |
| Sign-ups and downloads with a `?ref=<tool>` tag | The site and the sign-up of the hosted mode | Yes. The site must keep the tag to the sign-up or the download. [NEEDS CLARIFICATION: where the tag is kept, with no other data of the person.] |
| The share of new users who add a catalogue tool in their first week | The sign-up day and the first completed install | Yes. It joins two counts. [NEEDS CLARIFICATION: on the hosted hub only, or also opt-in self-host.] |
| The maintainer funnel: contacted, listed with consent, docs line added, upstream pull requests merged | A list that a person keeps | No. It is not product telemetry. |

**Outreach to maintainers.** The outreach is not a code task. These rules apply to each person and agent that does it.

| Always | Ask the operator first | Never |
| --- | --- | --- |
| Give before you ask: a merged upstream fix or docs pull request comes first. Lead with the AGPL licence of Anima.ing. In the first message, ask for permission and feedback ("Can we list the tool? Does this install path look right?"). Make it easy to correct or opt out (AXT-037). | Each first message to a project. The offer of a docs line or a badge (OD-AXT12). Sharing install counts with a project (OD-AXT14). | Contact a maintainer, or announce the catalogue, before the public release, which the operator calls. Ask for a docs listing in the first message. Ask for co-marketing in the first message. Use "official partner" or exclusive words. |

**A difference with an older note.** An older spec in the business vault, "One-Click Agent Setup", has a different first-version list of agents. This proposal keeps T3 Code out (constraint C2), and bots.md version 5 has six agents (BOT-077). [NEEDS CLARIFICATION: OD-AXT15.]

ASCII wireframe of the pane and the grant screen:

```text
Anima.ing                                            [ mac-mini | Hermes Agent ]  x
+----------------------------------------------------------------------------------+
| Hermes Agent on mac-mini   127.0.0.1:9119   This page goes through your hub as   |
| [Reload] [Stop] [Revoke]                    it is. Your hub does not keep it.    |
| Hermes Agent, MIT licence. Works with Anima.ing.  [Repo] [Docs] [Sponsor]        |
|----------------------------------------------------------------------------------|
|                                                                                  |
|              (the page of the tool, in its own web view, no IPC)                 |
|                                                                                  |
+----------------------------------------------------------------------------------+

Grant: Hermes Agent on mac-mini
+----------------------------------------------------------------------------------+
| What runs        hermes dashboard, as your user, in a session that Anima.ing owns|
| Port             127.0.0.1:9119 only. No port opens on another address.          |
| Who can open it  You. The dashboard trusts its own machine, so the pane is open  |
|                  to each person who can open it.                                 |
| Logins           Do not give Hermes the Claude Code login file. Use an API key,  |
|                  a local model or another provider that you set on this machine. |
| Vendor terms     terms not checked                                               |
|                  [ ] I use my own keys on my own machine                         |
|                                                       [ Cancel ]  [ Turn on ]    |
+----------------------------------------------------------------------------------+
```

A mockup of these screens is in [mockup/agent-tools-as-extensions/index.html](../mockup/agent-tools-as-extensions/index.html) (state Proposed, PLN-019).

## 7. Proposed changes to other documents (they wait for a ruling)

These changes are not made. Each one waits for the ruling of its decision. The commit of the ruling makes them.

| Document | Change | Decision |
| --- | --- | --- |
| [extensions-plan.md](../plans/extensions-plan.md) Section 4.12.1 | AIS-009 to AIS-017 change to "Withdrawn (R-nnn): replaced by the Herdr tool extension of agent-tools-as-extensions.md". Open decisions 1 to 3 of 4.12.1 change to "Superseded". AIS-018 to AIS-024 do not change. | OD-AXT2 |
| [ai-direction.md](ai-direction.md) OD-AI5 ("own session server, or embed Herdr?") | "Superseded: host Herdr as it is (R-nnn)". | OD-AXT2 |
| [bots.md](bots.md) | BOT-001 to BOT-018 and BOT-025 to BOT-028 (the chat, the bot definition, schedules, budgets, bot computers, agent switches and handoffs) change to "Withdrawn (R-nnn): Hermes is the assistant". BOT-047 to BOT-057 (the fallback to a local model) change to "Parked". BOT-019 to BOT-024, BOT-029 to BOT-046 and BOT-058 to BOT-078 stay. BOT-004, BOT-008 and BOT-069 lose their link to AIS-013 and AIS-017. | OD-AXT3 |
| bots.md BOT-064 | Add: "A tool extension may listen on the loopback address of the device only (AXT-008)." | OD-AXT4 |
| bots.md BOT-077 | Add the Hermes and Herdr recipes to the list, Hermes first in "Add an agent" (AXT-021, AXT-023, AXT-024). | OD-AXT1, OD-AXT5 |
| design specification WEB-006 | Add: "and a separate data store for each tool of each device (AXT-010)". | Approval of AXT |
| `contracts/tunnel.md` (K4) | A new section: the tool tunnel (a `direct-tcpip` channel that the hub opens to the loopback port of the device). Section 8: the manifest command in place of the login shell for the terminal kind (AXT-014). | OD-AXT6, approval of AXT |
| `contracts/schema.graphql` (K1), `contracts/database.md` (K2), `contracts/hub-domains.md` (K3), `quality/hub-modules.json` | The types, tables and domain of the grants and states. | Approval of AXT |
| The site repository `anima-site` | The catalogue pages, `llms.txt`, the badge and the `?ref=` tag (AXT-036, AXT-038). The site owns them; this document does not change them. | Approval of AXT; the public release |
| [priorities.md](../plan/priorities.md) row 4.21 | From the placeholder to its tier and state. | OD-AXT11 |

## 8. Reviews

### 8.1 Scope review

| Mode | What it means here | Verdict |
| --- | --- | --- |
| Expand | Community manifests, T3 Code, the desktop apps of Herdr and Hermes, the phone clients, tools that an AI agent starts | No. Each one breaks EXT-009, a terms rule (C1, C2) or MCP-006. |
| Selective | A host with manifests that Anima.ing writes, grants for each device, two kinds (web and terminal), two tools, one path through the hub | Yes. This is the chosen mode. It is a real host (relayed answer A1): the manifest, the grants, the states, start and stop, and the isolation are general. Only the catalogue is small. |
| Hold | Write the design and build nothing | Yes until the rulings of OD-AXT1 to OD-AXT4 and the approval. AX0 is a record and may run before (PLN-007). |
| Reduce | Tabs only: open the Hermes dashboard as a web interface of WEB-001 | The bot recommended this in the idea stage. The operator chose the host (relayed answer A1). It also does not work for a loopback port of a remote device (Section 4). |

### 8.2 Design scores

The scores are the judgement of the Planner on 2026-11-18, from 0 (bad) to 10 (good). They are not a measurement.

| Dimension | Score | Why, and what raises it |
| --- | --- | --- |
| Isolation of the client from the tool | 7 | WEB-006 and WEB-007 are built and tested; AXT-010 adds the origin lock. The client listener (AXT-011) is new. Raise to 8 when AX0 proves the pane token and AX8 has tests for each refusal. |
| Isolation of the hub from the tool | 8 | No tool code runs in the hub (AXT-003). The hub relays bytes only. It sees them in plain text (OD-AXT7), as the terminal does (R-062). |
| Login safety | 5 | Anima.ing reads no login. Hermes can read the Claude login file and Anima.ing cannot stop it (C1). A warning and a confirmation give 5; a terms row and a Hermes option that turns the source off would give 7. |
| Loopback trust | 4 | A tool with no sign-in on its own machine is open to each person who can open the pane (AXT-013). Only the operator can open it now. Members and shares lower it unless AXT-012 is decided. |
| Fit with rulings | 6 | Fits R-062, RA-008, LOG-007, MCP-004, MCP-006 and EXT-009 for a manifest of Anima.ing. Needs rulings for the reversal (OD-AXT2, OD-AXT3), for EXT-009 (OD-AXT4) and for K4 (OD-AXT6). |
| Dependency risk | 4 | The install needs BI0 to BI4 and task W1, which are not started. Three vendors change fast (Hermes 0.23, T3 Code 0.1, Herdr 0.9). The web view inside a tool can break on `Host` or `Origin` checks (AX0). |
| Testability | 6 | Manifest validation, grants, audit, the relay and the web view refusals have automatic tests. The real tools need a hand check (PLN-014). |
| Reversibility | 8 | A grant, a switch or a release can turn a tool off. No data of the operator moves. |
| Effort realism | 5 | Thirteen tasks of half a day to three days: about four to six weeks for one person, without BI0 to BI11 and W1 (an estimate). This matches effort L. |
| Clarity | 6 | Each requirement tests as pass or fail. Four [NEEDS CLARIFICATION] markers stay until AX0 and the interface review. |

### 8.3 Engineering review

**Types (a sketch for the contract review, not a contract).**

```text
ToolManifest { key, name, vendorUrl, licence, versions, kind: WEB | TERMINAL,
               start: Command, stop: Command, port: u16?  (WEB, loopback only),
               attach: Command?  (TERMINAL), recipe: RecipeKey | OPERATOR,
               terms: TermsKey, credentialNotes: [Text] }
Command      { program: Text, args: [Text] }        -- no shell, no free text (BOT-061)
ToolGrant    { device, tool, manifestVersion, grantedBy, grantedAt, revokedAt? }
ToolState    NOT_INSTALLED | STOPPED | RUNNING | NOT_CHECKED
PaneTicket   { grant, session, expiresAt }          -- one use, as the session tickets
```

**Trust boundaries.**

```text
 operator --login--> client (main web view: Anima.ing only)
                       | child web view (own store, no IPC, origin lock)   [B1]
                       v
                     client listener 127.0.0.1:<random> + pane token       [B2]
                       | WebSocket + pane ticket
                       v
                     hub (relay only; no record, no log; no tool code)     [B3]
                       | SSH direct-tcpip to 127.0.0.1:<port>  (or the terminal, R-062)
                       v
                     device: the tool, as the operator's user, loopback only [B4]
```

- B1: the tool's page must not reach the Anima.ing interface, its storage, its cookies or Tauri IPC (WEB-006, WEB-007, AXT-010).
- B2: another program on the client computer must not use the listener (AXT-011).
- B3: the hub must not store the bytes, and an API key must not get a pane (AXT-012, AXT-015).
- B4: the tool must not listen on another address (AXT-008). Anima.ing does not sandbox the tool on the device: it is a program of the operator, with the operator's rights. The grant screen says so.

**Failure modes.**

| Failure | What happens |
| --- | --- |
| The tool is not running | The pane shows the error page with Start (AXT-017). |
| The tool refuses the `Host` or the `Origin` of the relay, or refuses a frame | AX0 finds it. The listener sends `Host: 127.0.0.1:<port>`. If the tool still refuses, the entry is not in the catalogue (kill criterion of the idea). |
| `sshd` of the device refuses `direct-tcpip` | The pane shows the reason and the `sshd` option to change. The host does not change `sshd`. |
| The tool listens on all addresses | The start command forces the loopback option. [NEEDS CLARIFICATION: whether the daemon can see the address of a listening port without root.] |
| The tool reads the Claude login file | Anima.ing does not see it and cannot stop it (C1). The warning and the confirmation of AXT-022 are the control. |
| The grant is revoked while a pane is open | The hub closes the channel; the pane shows "Access revoked". |
| The tool updates itself to a version that the manifest does not support | The state shows "version not supported"; the pane still opens, with a warning. |
| The hosted hub is down | The pane cannot open. The tool keeps running on the device. |
| An AI agent asks to open a pane | Refused: no MCP tool opens a pane (AXT-012). |

**Tests.**

- Pure tests of the manifest validation: a command with free text, a shell, `sudo`, a port that is not loopback, or a Claude login source fails (AXT-001, AXT-008, AXT-022).
- Hub tests: a grant and a revoke are in the audit chain; an API key cannot get a pane ticket; a ticket works once; a revoke closes the channel; the relay keeps no bytes (AXT-012, AXT-015, AXT-016).
- SSH tests with the fake device of `crates/hub/src/fleet/sessions/terminal/tests/wire/fake_ssh/`: the hub opens `direct-tcpip` only to `127.0.0.1` and only to the port of the grant; the existing test that refuses channels from the device still passes (K4 8.4).
- Desktop tests, as `apps/desktop/src-tauri/tests/pinning/`: the tool web view has no IPC, its own store, and cannot navigate to another origin; the listener refuses a request with no pane token (AXT-010, AXT-011).
- The coverage test of the command line for each new root field (CMD-031).
- A hand check on a Mac and a Linux machine, on the hosted hub, with Hermes and Herdr (an entry in [need-human-testing.md](../need-human-testing.md) in task AX12).

### 8.4 Blast radius on shared contracts

| Shared item | Change | Who else uses it | Action |
| --- | --- | --- | --- |
| K1 `contracts/schema.graphql` | Types and fields for the catalogue, grants, states, start, stop and the pane ticket | Command line (CMD-031), MCP, client, mock hub fixtures | A ruling (CON-001). The type `Extension` does not change; "Agent tools" is one more registry entry (OD-AXT8). |
| K2 `contracts/database.md` | A table of grants | Hub domains, backups, retention | A ruling. |
| K3 `contracts/hub-domains.md`, `quality/hub-modules.json` | A new domain, for example `fleet/tool_panes`, that may import `ssh`, `sessions`, `devices` and `events` | Import and folder-size gates | A ruling. |
| K4 `contracts/tunnel.md` | A new tool tunnel; Section 8 gets the manifest command for the terminal kind | Terminal client, session list, the relay tests | A ruling (OD-AXT6). Section 8.4 (refuse each channel from the device) does not change. |
| K8 `contracts/daemon-push.md` | None. Two new `ToolName` values in the inventory, which the hub ignores when it does not know them (K8 Section 4) | Tool inventory | None. |
| K5 design tokens, K6, K7 | None | — | None. |
| WEB-006 of the design specification | One store for each tool of each device | Web interfaces | A ruling with the approval. |
| BOT-064, BOT-077 of bots.md | Loopback port for a tool; two recipes | The install (BI tasks) | A ruling (OD-AXT4, OD-AXT5). |
| EXT-009 | Not changed. A ruling says that this host is the isolation design for manifests of Anima.ing only. | Community extensions (2.8.1, 3.4) | OD-AXT4. |
| HMD-048, HMD-049 (the link) | None with choice (a) of OD-AXT6 | Hosted mode | None. |
| MCP rules (MCP-004, MCP-006) | None. One read tool only. | Each MCP client | None. |

## 9. Open decisions

Each row asks one question with yes, no or one choice, and gives the recommendation of the Planner (PLN-018, PLN-021). A row is open until the operator answers first-hand and a ruling records it.

| Number | Question | Recommendation, with strengths and weaknesses | State |
| --- | --- | --- | --- |
| OD-AXT1 | Do you confirm, first-hand, the relayed answers A1, A3 and A4: version 1 is a real extension host (effort L); Hermes is suggested first in "Add an agent" and never installed without your click; the work gets its own priorities row? (yes/no) | Yes. Strength: it turns the relayed direction into a ruling and unblocks the approval. Weakness: effort L before the 15 extensions are built. | Open |
| OD-AXT2 | Do you withdraw the native agent panes of Section 4.12.1 (AIS-009 to AIS-017) and host Herdr as a terminal tool extension in their place, keeping the usage counts (AIS-018 to AIS-024)? (yes/no; it needs a ruling) | Yes. Strength: nothing of 4.12.1 is built, so nothing is lost, and Herdr already keeps panes alive. Weakness: the one page of all agents on all devices, the alerts and the overload warning (AIS-010 to AIS-012, AIS-017) go away until OD-AXT10. | Open |
| OD-AXT3 | Do you withdraw the native bot runtime of bots.md (BOT-001 to BOT-018, BOT-025 to BOT-028), park the fallback to a local model (BOT-047 to BOT-057), and keep the states, the terms rules, the catalogue and the install (BOT-019 to BOT-024, BOT-029 to BOT-046, BOT-058 to BOT-078)? (yes/no; it needs a ruling) | Yes. Strength: none of it is built; Hermes gives chat, schedules and tools. Weakness: Anima.ing then has no own read-only bot with hub-side limits (BOT-012, BOT-013); Hermes has its own rules, which Anima.ing does not enforce. | Open |
| OD-AXT4 | May this host start before the 15 extensions are built (EXT-009), if it loads only manifests that Anima.ing writes, runs no code in the hub, and starts only after you approve the isolation and permissions design of Section 8.3? (yes/no) | Yes, with those three limits. Community manifests stay behind EXT-009. Strength: EXT-009's text holds (no code of other people in the hub). Weakness: it is a second extension kind before the first 15 are done, and it takes time from row 4.4. | Open |
| OD-AXT5 | Does "never installed without your click" also stop an AI agent from installing Hermes through MCP in the state `allow` (R-153, OD-BOT17)? (yes/no) | Yes: an install of Hermes always asks a person, in each MCP state. Strength: Hermes is an assistant with wide access; an agent must not install a second agent alone. Weakness: one recipe then has a rule that differs from R-153. | Open |
| OD-AXT6 | Which path reaches the loopback port of the tool: (a) an SSH `direct-tcpip` channel that the hub opens (port 22, which the link already allows), or (b) the port of the tool in the device's list of open ports of the link (a change to HMD-049)? (one choice) | (a). Strength: no new port on the device and no change to the link; it works on the VPN hub and the hosted hub. Weakness: the device needs SSH set up in Anima.ing and `sshd` must permit the channel; a K4 change. | Open |
| OD-AXT7 | May the web traffic of a tool pass through the hub as plain text with a notice, as the terminal does (R-062, R-153), in the first version? (yes/no) | Yes, with no record and no log (AXT-015). Strength: it uses the path that exists. Weakness: a person with access to the hub could read the page, as for the terminal. | Open |
| OD-AXT8 | Is "Agent tools" one new switch in the registry of extensions (default off, Class C), and not one of the 15 extensions that EXT-009 counts? (yes/no) | Yes. Strength: EXT-001 to EXT-003 apply as they are, and the count of EXT-009 does not move. Weakness: a 16th registry entry; the text "the fifteen" in the code and the plan changes. | Open |
| OD-AXT9 | Is a warning and your confirmation for each device enough for C1, given that Anima.ing cannot stop Hermes from reading the Claude Code login file on your machine? (yes/no) | Yes, with the terms row before the release (BOT-038). Strength: honest, and the same kind of control as BOT-038 and BOT-041. Weakness: it is not a block; a wrong Hermes setting can still use the login. | Open |
| OD-AXT10 | Does version 1 leave the state of each agent to Herdr's own screen, with no Anima.ing page of all agents and no alerts (the place of AIS-010 to AIS-012)? (yes/no) | Yes. Revisit with the idea `agent-state-hooks` (the open status protocol of Herdr). Strength: smaller version 1. Weakness: no alert on the phone when an agent waits. | Open |
| OD-AXT11 | Is row 4.21 tier 3, after row 4.7 (the work route, which the install needs), with AX0 free to run before? (yes/no) | Yes. Strength: the panes do not wait, but the install does, so the order is honest. Weakness: Hermes cannot be installed from the app until W1 and BI0 to BI4 are done. | Open |
| OD-AXT12 | May Anima.ing offer each project an upstream docs line "Run it remotely with Anima.ing", only after the public release and only after the project merged a contribution of Anima.ing (AXT-043)? (yes/no) | Yes: offer it, never require it. Strength: the users of the project find Anima.ing from the side of the tool, and the give-first rule protects goodwill. Weakness: it is slow, and a project can say no after work was given. | Open |
| OD-AXT13 | Do catalogue entries and the pane show the sponsor link of the project when it has one (AXT-028, AXT-033)? (yes/no) | Yes. Strength: it sends support back to the project and shows that Anima.ing is not a reseller. Weakness: a sponsor link that moves or ends needs a person to check it before each release. | Open |
| OD-AXT14 | May Anima.ing share anonymised install counts with each project, opt-in only, later than version 1? (yes/no) | Yes, later than version 1. Strength: useful to the project and builds trust. Weakness: it needs the new telemetry of AXT-042 and a consent flow; a small count can point to one person. | Open |
| OD-AXT15 | Do you confirm that T3 Code stays out of the catalogue (constraint C2), although an older vault spec, "One-Click Agent Setup", lists a different first version? (yes/no) | Yes, per C2 (BOT-021, BOT-032, BOT-033). Strength: it keeps the terms rules. Weakness: T3 Code users are a large group that this version does not reach. | Open |

## 10. Tasks

One pull request for each task. The size of each task is a few hours to three days (an estimate). All tasks are Not started. Nothing is built from this draft except AX0, which is a record in `docs/research/` and which PLN-007 permits for a Draft. A task does not start while its column "Needs" names an open decision.

| Task | Wave | Content | Needs | State | Commits |
| --- | --- | --- | --- | --- | --- |
| AX0 | 0 | A measurement, a record only: the Hermes dashboard and Herdr in a Tauri child web view and a terminal pane, through a test relay to a loopback port; `Host`, `Origin`, WebSocket and frame refusals; the two designs of AXT-011; `direct-tcpip` on stock macOS and Linux `sshd`; the install and attach commands | Nothing (record only) | Not started | — |
| AX1 | 1 | The operator answers OD-AXT1 to OD-AXT15 and approves AXT-001 to AXT-043 with a ruling | The operator | Not started | — |
| AX2 | 1 | Docs only, merged first (R-155): the rulings; the changes of Section 7 to extensions-plan.md, ai-direction.md, bots.md and WEB-006; the contract text of K1, K2, K3 and K4; `hub-modules.json` | AX1 | Not started | — |
| AX3 | 2 | `daemon-core`: the manifest types and their pure validation, with the attribution fields of AXT-028 and the supported versions of AXT-035; the manifests of Hermes and Herdr; two `ToolName` values and their probes | AX2 | Not started | — |
| AX4 | 2 | The hub: the domain of the catalogue, the switch, the grants, the states and the audit events; the GraphQL | AX2; OD-AXT8 | Not started | — |
| AX5 | 3 | The hub: start and stop through SSH with fixed commands; the tool tunnel (`direct-tcpip` to the loopback port), the pane ticket, the relay with no record | AX4; AX0; OD-AXT6, OD-AXT7 | Not started | — |
| AX6 | 3 | `anima tools list`, `status`, `grant`, `revoke`, `start`, `stop`, `open` (terminal kind) | AX4 | Not started | — |
| AX7 | 4 | The MCP read tool `anima_tools_list` | AX6 | Not started | — |
| AX8 | 4 | The desktop: the tool web view (own store, no IPC, origin lock, no popup, no download) and the client listener with the pane token | AX5; AX0 | Not started | — |
| AX9 | 4 | The client: the switch, the grant screen, the tools on the device page, the pane header and its error page; the mockup is compared (PLN-019) | AX4, AX8 | Not started | — |
| AX10 | 4 | The terminal kind: the manifest command in the SSH terminal (Herdr, `hermes --tui`) | AX4; OD-AXT2 | Not started | — |
| AX11 | 5 | The recipes of Hermes (suggested first, never without a click) and Herdr in the catalogue of bots.md, with their terms rows, the confirmation of AXT-022, the attribution fields (AXT-028), the official release only (AXT-029), the version check and the "Reported to the project" state (AXT-035) | BI0 to BI4 of bots.md; OD-AXT1, OD-AXT5, OD-AXT9 | Not started | — |
| AX12 | 6 | The security review of B1 to B4 and the entry in `need-human-testing.md`; the hand check on a Mac and a Linux machine on the hosted hub | AX8, AX9, AX10, AX11 | Not started | — |
| AX13 | 4 | The client: the attribution in the pane header and the catalogue rows (AXT-033, AXT-034), the links in the system browser; the mockup frames 1, 3 and 6 are compared | AX9; OD-AXT13 | Not started | — |
| AX14 | 4 | Tool health in the uptime and metrics view, with the alert and its mute (AXT-040) | AX4, AX5 | Not started | — |
| AX15 | 5 | The new counts of AXT-042 (installs for each tool for each week, the first-week share), counts only | Approval of AXT-042; the [NEEDS CLARIFICATION] markers of the signals table | Not started | — |
| AX16 | 7 | Not a task of this repository: the catalogue pages, `llms.txt`, the badge, the `?ref=` tag and the clips in `anima-site` (AXT-036 to AXT-039). Recorded here so that the order is clear | The public release, which the operator calls; AX11 | Not started | — |

### 10.1 Task briefs

**Rules for every task.** Start with `AGENTS.md`, "Start of a session". Claim the task (PLN-022). Build on a branch in your own worktree. Use the order of CMD-030. At the end run `bun run quality:documents` and `bun run gates`. Done means a commit on `main` that changes the state of the task in this table (PLN-008). Always for each task: cite AXT identifiers; log no URL path, page content or token (LOG-007). Ask: before a new dependency, and before a change to an existing field of a contract. Never: run code of a tool in the hub; read a login file (BOT-023); push to `main` without the claim.

| Task | Acceptance (pass or fail) | Check commands | Out of scope |
| --- | --- | --- | --- |
| AX0 | Pass when `docs/research/agent-tools/ax0-panes.md` records, for each tool and each design, a result (works, refused, not tested) with the version, the date and the machine; fail if a row has no result | `bun run quality:documents` | Product code; a commit to `apps/` or `crates/` |
| AX2 | Pass when each changed document names the ruling and each new root field names its command; fail otherwise | `bun run quality:register`, `bun run quality:documents` | Any code |
| AX3 | Pass when a manifest with a shell, free text, `sudo`, a port that is not loopback or a Claude login source fails validation in a test, and the Hermes and Herdr manifests pass; fail otherwise | `cargo nextest run -p daemon-core`, `cargo clippy -p daemon-core --all-targets -- -D warnings` | Running a tool; the hub |
| AX4 | Pass when a grant and a revoke write audit events with no secret, a revoke of an unknown grant is refused, and the switch follows EXT-001 to EXT-003; fail otherwise | `cargo nextest run -p anima-hub`, `bun run quality:imports` | The tunnel |
| AX5 | Pass when the hub opens `direct-tcpip` only to `127.0.0.1` and the granted port, an API key gets no ticket, a ticket works once, a revoke closes the channel, the relay keeps no bytes, and the test of K4 8.4 still passes; fail otherwise | `cargo nextest run -p anima-hub` | The client |
| AX6 | Pass when each new root field has its command and the coverage test passes; fail otherwise | `cargo nextest run -p anima-cli` | MCP |
| AX7 | Pass when the tool returns only states in named fields with a length limit (MCP-008) and no tool starts, stops or opens; fail otherwise | `cargo nextest run -p anima-cli` | Change tools |
| AX8 | Pass when tests show that the tool web view has no IPC, its own store, no navigation to another origin, no popup, no download, and that the listener refuses a request with no pane token; fail otherwise | `cargo nextest run -p anima-app`, `bun run --cwd apps/client test` | The hub |
| AX9 | Pass when the screens match the mockup at 1040 px in light and dark and its row in `mockup/README.md` names the commit; fail otherwise | `bun run --cwd apps/client test`, `bun run quality:documents` | New tools |
| AX10 | Pass when the terminal opens with the manifest command and the login shell is used for each other session; fail otherwise | `cargo nextest run -p anima-hub` | The web kind |
| AX11 | Pass when the Hermes recipe is first in "Add an agent", is never started by a schedule, an event, a bot or (if OD-AXT5 is yes) MCP, and shows the warning of AXT-022; fail otherwise | `cargo nextest run -p daemon-core`, `cargo nextest run -p anima-hub` | T3 Code (C2) |
| AX12 | Pass when the review lists B1 to B4 with a result and the human check has an entry with steps; fail otherwise | `bun run quality:documents` | Fixes, which are their own tasks |
| AX13 | Pass when the pane header and each catalogue row show the name, the licence and the links of AXT-028, the links open in the system browser, no text says "partner" or ranks tools, and the mockup row names the commit; fail otherwise | `bun run --cwd apps/client test`, `bun run quality:documents` | Logos (AXT-030 needs a person first) |
| AX14 | Pass when a tool that stops shows "down" in the uptime and metrics view within one check interval, sends one alert, and a mute stops the alert; fail otherwise | `cargo nextest run -p anima-hub`, `bun run --cwd apps/client test` | New metrics of the tool itself |
| AX15 | Pass when each count holds no text, no path and no account name (a test of LOG-007), and a self-hosted hub sends nothing without opt-in; fail otherwise | `cargo nextest run -p anima-hub` | Site analytics |
| AX16 | Not checked in this repository | — | Everything in this repository |

Prompt for each task: "Follow AGENTS.md 'Start of a session'. Claim and build task AXn of docs/proposals/agent-tools-as-extensions.md. Read Sections 5, 6 and 8 first. Build only what the row of AXn says, in the order of CMD-030. Run the check commands of the row, then bun run quality:documents and bun run gates."

## 11. Not verified

- The Hermes and Herdr behaviour in a pane, the install and attach commands, and `direct-tcpip` on stock `sshd` (task AX0).
- The facts about the three tools in Section 1 come from the idea stage (GitHub API and vendor pages, read 2026-11-18 by Anima Product Ideas). The Planner did not read them again.
- Whether the daemon can see the address of a listening port without root.
- The licence and the name and logo rules of Hermes Agent and Herdr (AXT-030). A person reads them before AX13.
- Whether each project has a sponsor page (AXT-028).
- The size of the work: an estimate.

## 12. Review notes

- 2026-11-18, version 1. The Planner made this proposal from the idea `agent-tools-as-extensions` and deleted the idea folder in the same commit (PLN-018). The four answers of the operator are relayed and wait for first-hand confirmation (OD-AXT1 to OD-AXT3). No ruling is recorded. The Planner added row 4.21 as a placeholder, a concern row in ownership.md, pointer notes in extensions-plan.md Section 4.12.1 and in bots.md, and the mockup.
- The Planner found that WEB-004 cannot reach a loopback port of a remote device, and that the link reaches only ports 22, 5900 and 3389. So the pane needs a path through the hub (OD-AXT6).
- 2026-11-18, version 2. The operator said first-hand (22:32 PT): "I mean its more about how do I support all of these great other open source apps and also gian their users", and chose to reframe the goal. The Planner rewrote Section 1 and added Section 6.7 (AXT-028 to AXT-043) with the input of Anima Marketing Bot: credit, the official release, no fork, upstream fixes, no lock-in, attribution, "works with" wording, version checks, catalogue pages on the site, the badge with `?ref=`, clips, tool health, security text, the signals and the outreach rules. It added OD-AXT12 to OD-AXT15 and tasks AX13 to AX16, and changed AX3 and AX11. The goal is direction, not a ruling; OD-AXT1 stays Open. The Planner recorded that an older vault spec, "One-Click Agent Setup", differs on the first-version list (OD-AXT15), and copied nothing from it.
