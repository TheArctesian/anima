---
kind: Proposal, with a task table
state: Draft
evidence: Version 6 (2026-11-18): R-196 withdraws BOT-001 to BOT-018 and BOT-025 to BOT-028 (Hermes Agent is the assistant), parks BOT-047 to BOT-057, replaces Aider with Pi in BOT-077 (OD-AXT16), adds the recipes of Hermes Agent, Herdr and T3 Code to the catalogue with a named exception for T3 Code (OD-AXT15), and adds an exception to BOT-065 for Hermes (OD-AXT5); no requirement is approved. Version 5 (2026-11-18): the operator decided OD-BOT15 to OD-BOT20 (R-153); the section "Install an agent on a machine" now names six agents for the first version, BOT-065 follows the MCP setting, and OD-BOT21 to OD-BOT23 are open; no requirement is approved. Version 4 (2026-11-18) added the section "Install an agent on a machine" (BOT-058 to BOT-078, OD-BOT15 to OD-BOT20, tasks BI0 to BI11, all Not started), checked against origin/main at 7342500c. Version 3 (2026-11-18). Direction of the operator of 2026-11-16; not approved. BOT-001 to BOT-078 (BOT-058 to BOT-078, install an agent, are proposed; OD-BOT15 to OD-BOT20 are decided by R-153, OD-BOT21 to OD-BOT23 are open; BOT-039 to BOT-046, agent coverage against Orca, and BOT-047 to BOT-057, the fallback to a local model, are proposed with OD-BOT1 to OD-BOT14). It was Section 4.16 of [extensions-plan.md](../plans/extensions-plan.md) until 2026-11-17 (PLN-018). Its work package XP-16 is proposed and is not in the tables of Section 5 of that plan. No tier in Section 4 of the work plan yet.
---

# Proposal: Bots

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this proposal (PLN-005).

| Item | Value |
| --- | --- |
| Document | Proposal: the Bots extension (XP-16, proposed) |
| Version | 6 |
| Date | 2026-11-18 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | The header of this document holds the state (PLN-020). History: Draft. Direction of the operator of 2026-11-16. Version 2 adds BOT-039 to BOT-046, version 3 adds BOT-047 to BOT-057, version 4 adds BOT-058 to BOT-078 and version 5 adds BOT-078 and decides OD-BOT15 to OD-BOT20 (R-153). Requirements are proposed; no requirement is approved. Version 6 records R-196 (agent tools as extensions); no requirement is approved. |
| Related | [extensions-plan.md](../plans/extensions-plan.md) Sections 4.12.1 (agent panes) and 4.15 (model gateway); [agent-tools-as-extensions.md](agent-tools-as-extensions.md): R-196 withdraws the native bot runtime (OD-AXT3) and adds Hermes Agent, Herdr and T3 Code to the catalogue |

Version 2 (2026-11-17) adds the section "Agent coverage" (BOT-039 to BOT-046, with open decisions OD-BOT1 to OD-BOT7). It changes no earlier requirement. Version 3 (2026-11-18) adds the section "Fallback to a local model" (BOT-047 to BOT-057, OD-BOT8 to OD-BOT14). Version 4 (2026-11-18) adds the section "Install an agent on a machine" (BOT-058 to BOT-078, OD-BOT15 to OD-BOT20, tasks BI0 to BI11). It changes no earlier requirement. Version 5 (2026-11-18) records the decisions of the operator on OD-BOT15 to OD-BOT20 (R-153), amends BOT-065, BOT-067, BOT-068, BOT-071, BOT-073, BOT-075 and BOT-077, and adds BOT-078 and OD-BOT21 to OD-BOT23. A later edit of 2026-11-17 adds to BOT-034 and BOT-040 that detection accepts both program names of Cursor. Version 1 only moved this text from Section 4.16 of the extensions plan, because a proposal is not a part of a plan (PLN-016, PLN-018). The text did not change. A section number in this document (for example "Section 4.15") is a section of the extensions plan. When the operator approves this proposal, its requirements go back into the extensions plan as Section 4.16, with a ruling.

## Bots

**R-196 (operator, 2026-11-18, first-hand; OD-AXT3 of [agent-tools-as-extensions.md](agent-tools-as-extensions.md)): "yes, withdraw the native bot runtime, park BOT-047 to 057, keep status, terms, catalogue and install."** The native bot runtime is withdrawn: BOT-001 to BOT-018 and BOT-025 to BOT-028. Hermes Agent is the assistant, as a tool extension of that proposal. The fallback to a local model, BOT-047 to BOT-057, is parked. BOT-019 to BOT-024, BOT-029 to BOT-046 and BOT-058 to BOT-078 stay: the states, the terms rules, the catalogue and the install. The text of the withdrawn and parked requirements below is kept as a record.

**Purpose.** Always-on bots that work like employees, run on the operator's own computers, and use
the operator's own models. The operator defines a bot by chatting with it: its job, its schedule,
what it may touch. Each reply goes to a model that the router picks: the operator's subscription
(Claude Code, Codex) or any upstream of the model gateway (Section 4.15). A bot that needs to code
does so in an agent pane (Section 4.12.1). **Class:** B for the conversation text. C for the counts.
Status: direction only. The proposed work package is XP-16. It is not in the tables of Section 5.

**Data.** `bot` (name, job, instructions, device, model rule, schedule, permissions, state),
`bot_message` (bot, role, sealed text, model used, time), `bot_task` (bot, title, state, pane, result).

- **BOT-001.** Withdrawn (R-196): Hermes Agent is the assistant. The operator must be able to make a bot in a chat. The chat writes the bot definition:
  name, job, instructions, schedule, allowed devices and allowed tools. The operator must see the
  definition and confirm it before the bot starts.
- **BOT-002.** Withdrawn (R-196): Hermes Agent is the assistant. A bot must run on a device that the operator chooses, under the Anima daemon on that
  device. The hub keeps the definition and the history. The bot keeps running when no client is open.
- **BOT-003.** Withdrawn (R-196): Hermes Agent is the assistant. For each reply, the bot must choose a model by a rule: a fixed model, the cheapest
  model that can do the task, or local first. It must send the request through the model gateway
  (MGW-001, MGW-007). With no gateway upstream, it must use the operator's subscription through the
  vendor's own program on that device, for example `claude -p` or `codex exec`. It must never take
  or store the login of a subscription.
- **BOT-004.** Withdrawn (R-196): Hermes Agent is the assistant. A bot can start a coding task in an agent pane (AIS-013) in an allowed repository. Its
  state shows on the agent panes page with the other panes, and a task that waits for the operator
  sends the same alert (AIS-012).
- **BOT-005.** Withdrawn (R-196): Hermes Agent is the assistant. A bot can act on a schedule or an event: a time, a check that changes state, a new
  issue or pull request (Integrations, Section 4.5). It uses the rule engine (ENG-001).
- **BOT-006.** Withdrawn (R-196): Hermes Agent is the assistant. A bot must ask the operator before it sends a message to another person, spends money,
  deletes data or pushes to a shared branch. The operator can give a standing permission for one kind
  of action and can take it back.
- **BOT-007.** Withdrawn (R-196): Hermes Agent is the assistant. Each bot must have a budget (MGW-011) and a log of its actions. The operator must be
  able to pause or stop a bot in one action, from the client, the CLI (`anima bots`) and the phone.
- **BOT-008.** Withdrawn (R-196): Hermes Agent is the assistant. Bots count toward the overload warning (AIS-017), and their finished work goes into the
  daily digest (AIS-015).
- **BOT-009.** Withdrawn (R-196): Hermes Agent is the assistant. The Bots extension must be free on every plan, as agent panes are (AIS-016). Its
  conversation text is sealed like session text (AIS-004, AIS-005).
- **BOT-010.** Withdrawn (R-196): Hermes Agent is the assistant. Each bot can have its own computer: a virtual machine on one of the operator's devices,
  or a rented machine (CVM-001). The bot's shell, files and agent panes run there, not on the host
  device. The operator picks the size and the image when the bot is made (BOT-001).
- **BOT-011.** Withdrawn (R-196): Hermes Agent is the assistant. A bot's own computer joins the mesh as a device with a tag for that bot only. Access
  rules must not let one bot's computer reach another bot's computer, or the operator's devices,
  unless the operator allows it.
- **BOT-012.** Withdrawn (R-196): Hermes Agent is the assistant. The operator can make a bot read-only. A read-only bot can read checks, metrics, logs,
  files and repositories that it is allowed to see, and it can write to the operator in its chat. It
  cannot run change tools (MCP-002), write files, push, start agent panes, or act on a device. The
  hub must enforce this on its side, not only in the bot's instructions.
- **BOT-013.** Withdrawn (R-196): Hermes Agent is the assistant. A read-only bot must not be able to ask for more access itself. Only the operator can
  change a bot from read-only, and that change goes in the audit log.

The bot configurator (operator direction, 2026-11-16: the customer picks both the coding agent or
subscription, for example Claude Code or Codex, and the machine or machines that the bot runs on,
that is cloud machines, local machines such as a MacBook, or both):

- **BOT-014.** Withdrawn (R-196): Hermes Agent is the assistant. When the operator makes a bot (BOT-001), the client must show a configurator with
  two choices that the operator must make before the bot starts: the coding agent or subscription,
  and the machine or machines that the bot runs on. The configurator must write both choices into
  the bot definition (BOT-001).
- **BOT-015.** Withdrawn (R-196): Hermes Agent is the assistant. The list of coding agents must come from what is installed on the machines that the
  operator chose, for example `claude` and `codex`, and it must show a subscription only as the
  vendor's own program on that machine (BOT-003). The configurator must never read, ask for or
  store the login of a subscription.
- **BOT-016.** Withdrawn (R-196): Hermes Agent is the assistant. The list of machines must show the operator's own devices, for example a laptop,
  and the rented machines of the account (CVM-001), in one list. The operator can pick local
  machines, cloud machines or both. The hub must refuse a machine that the operator cannot use and
  a machine that another bot owns (BOT-011).
- **BOT-017.** Withdrawn (R-196): Hermes Agent is the assistant. Before the operator confirms, the configurator must show for each machine whether
  the chosen agent is installed on it, whether the bot is read-only (BOT-012), and, for a rented
  machine, its cost for each day if it stays on (CVM-004).
- **BOT-018.** Withdrawn (R-196): Hermes Agent is the assistant. A bot with more than one machine must have a rule that says which machine runs each
  task. The first rule is the operator's order of machines, with the next machine as the fallback.
  The audit log records which machine ran each task (BOT-007).

**Bring your own subscription (operator direction, 2026-11-16).** This part extends the
configurator (BOT-014 to BOT-018). It does not replace it. The customer picks an agent that he already
pays for, for example Claude Code, Codex, Cursor or Grok Build, or any ACP agent, and the machines
that run it. Anima.ing sells no model use and holds no login. The design follows idea 1 of
the note `Market/Competitors/Analysis/Ideas from T3 Code.md` in the Anima.ing vault. It follows
[T3 Code](https://github.com/pingdotgg/t3code) (MIT, commit `2b2d65d`). File paths below are from that commit.
T3 Code runs the vendor's own program on the
machine that does the work and uses the login of that program (`docs/user/faq.md`). Anima.ing does the same.
The configurator already knows the machines, and T3 Code does not. T3 Code also offers "Connect with
ChatGPT", which keeps the login in the server (`apps/server/src/provider/CodexChatGptAuth.ts`,
`CodexManagedTokenBroker.ts` and `ProviderCredentialStore.ts`). BOT-003 forbids that design, and
BOT-020 keeps the ban. Anima.ing takes the T3 Code design rules and the wording of its documents. It
takes no T3 Code code (the T3 Code server is TypeScript and Effect, and the hub is Rust).

- **BOT-019.** The configurator must offer one adapter for each of these agents: Claude Code
  (`claude`), Codex (`codex`), Cursor (`agent`), Grok Build (`grok`) and any ACP agent. Each adapter must
  start the vendor's own program on the chosen machine, as the operator's own user, and must use the
  login that this program holds. This extends the rule of BOT-003 (withdrawn, R-196: a fixed model, the cheapest model,
  or local first) with the choice of the agent and the subscription.
- **BOT-020.** Anima.ing must not read, copy, move, keep or relay the login of a vendor program. It
  must not keep a token or a password of a subscription in the hub, the database, a log, a backup, an
  MCP tool or the client. Each sign-in must happen on the machine, in the sign-in flow of the vendor.
  The client can show the sign-in command for the operator to run. The client must not run it with a
  token from Anima.ing. The model traffic of a subscription must go from the machine to the vendor. The hub
  and the relay must not carry it. This is the contrast with T3 Code's server-side "Connect with ChatGPT".
- **BOT-021.** The adapter must run the vendor's program as the vendor published it. It must not patch
  the program, copy its sign-in, remove or disable a sign-in method, change the identity text of the
  program, or pass a token that it did not get from the operator's own shell. It must not call the
  vendor's private endpoints. It must not use an SDK of a vendor for a subscription login (the Claude
  Agent SDK, `@cursor/sdk`, `codex app-server` account methods).
- **BOT-022.** The daemon (`anima-daemon`) on each device must find the installed vendor programs without reading a login
  file. It must do these steps in order, for each program. First, look for the program in the PATH and in the
  folders of its installer by the name of the file. Second, run `<program> --version`. Third, ask the
  program for its own sign-in state with its status command: `claude auth status`,
  `codex login status` or `agent status`. Read only the exit code and the name of the sign-in method (for
  example `claude.ai` or `api_key` for Claude Code, the name that the program prints for the others). Discard all other output, including the name and the email of the
  account. For a program without a verified status command (Grok Build and Antigravity), report "installed,
  sign-in not checked". Do not guess. T3 Code does the same kind of check in `ProviderInstallationGate.ts`,
  `ClaudeProvider.ts` and `CodexProvider.ts`.
- **BOT-023.** The daemon must not open, read, list the size of, or copy these files and stores, and must
  not read the keychain of the operating system for them: `~/.claude*`, `~/.codex/auth.json`,
  `~/.grok/auth.json`, the login store of the Cursor CLI (place not verified), and the matching items of
  the Keychain, the Secret Service and the Windows Credential Manager. A test must start a probe against a sandbox and fail when the
  probe opens one of these paths. Each probe must stop after 10 seconds and must run with no input.
- **BOT-024.** The device page must show, for each device, a list of agents with one state each: not
  installed, installed and signed out, installed and signed in (with the method: subscription, API key or
  other), or installed and not checked. The probe of BOT-022 must run when the agent starts, when
  the operator opens the page and when the operator makes a bot. The configurator (BOT-015 (withdrawn, R-196), BOT-017 (withdrawn, R-196))
  must use the same result. The state must not show the name or the email of the account.
- **BOT-025.** Withdrawn (R-196): Hermes Agent is the assistant. A conversation with a bot can change the agent or the model between two replies. The
  operator can do it at any time. A rule can do it, for example "my Claude subscription, then Codex when
  Claude has no quota". A change by a rule must show in the conversation and in the audit log (BOT-007).
  A rule must only move between agents of the same operator. A rule must not rotate between logins of the same
  vendor, and it must not pool logins.
- **BOT-026.** Withdrawn (R-196): Hermes Agent is the assistant. A change of agent must send a bounded handoff to the new agent. The handoff is a
  selection of the saved messages, not a summary that an agent writes. This costs no quota of a
  subscription. The bound is 16 000 tokens by default. The operator can set it from 1 024 to 64 000
  tokens. The hub must count one token for each UTF-8 byte, with the role and the agent name of each message
  included, as T3 Code does (`docs/user/portable-handoffs.md`). The handoff must also leave free at least
  16 000 tokens or one quarter of the context window of the target model, whichever is larger. When the
  window is not known, the hub must use 128 000 tokens. These numbers are the T3 Code defaults. Not verified:
  they fit each adapter.
- **BOT-027.** Withdrawn (R-196): Hermes Agent is the assistant. The handoff must contain the first request, then the newest messages that fit, in
  order, each with its role. It must say how many messages it left out and must name the place where the
  agent can read them. It must not contain the reasoning, the tool state or the attachments of the
  agent that the bot left. The new request of the operator must never be shortened. When the handoff
  does not fit even in a new session, the hub must say so and must not cut the new request.
- **BOT-028.** Withdrawn (R-196): Hermes Agent is the assistant. When a bot goes back to an agent that it used before, the adapter must resume the
  session of that agent when the vendor's program supports it (`claude -r`, `codex exec resume`). It must
  send only a handoff of the messages since that agent last ran. Otherwise it must start a new session with
  a full bounded handoff. This follows `docs/orchestration-v2/provider-switching-and-context.md` in T3 Code.
- **BOT-029.** A login of a vendor program must be used only for the operator who owns it. The hub must
  refuse to run an agent for a bot on a machine of another account. It must not let a bot, a customer
  or a member of the staff of Anima.ing use the subscription of another person. It must not share a login
  between two operators or between two machines of different owners.
- **BOT-030.** Anima.ing must not pay for, resell or include the use of a subscription in any plan, price
  or credit. BOT-009 is withdrawn (R-196); the agent tools that replace the bot runtime are free on every plan (AXT-027 of [agent-tools-as-extensions.md](agent-tools-as-extensions.md)). The charge for a rented machine
  (CVM-009) must not include the use of a model of a vendor.
- **BOT-031.** The images of rented machines and the hosted hub must not contain a preinstalled vendor
  program. The operator installs it on the machine and signs in on it. The reason is the
  condition of Anthropic for a product that runs Claude Code (the table below). Open decision 6 holds
  the question of an install command that the operator starts.
- **BOT-032.** Claude Code: the adapter must run only the `claude` program (`claude -p` or an interactive
  session). It must not use the Agent SDK with a subscription. It must not make, ask for or pass a
  long-lived token (`claude setup-token`, `CLAUDE_CODE_OAUTH_TOKEN`). A bot must run one `claude` task at a
  time for each login by default, because the plans assume ordinary, individual use. The client must warn the operator
  when the use of a bot looks like more than ordinary, individual use. Open decision 7 holds the limit.
- **BOT-033.** Codex: the adapter must run only `codex exec` and `codex exec resume`. It must not use the
  account methods of `codex app-server` (`account/login/start`, `chatgptAuthTokens`), because OpenAI does
  not permit them for a commercial or hosted service. When the login is a ChatGPT login, a bot must not start a
  Codex run from an event that a stranger can create, for example an issue or a pull request in a public
  repository. The configurator must offer an API key as the other choice, set by the operator in the Codex
  configuration of the machine and not seen by Anima.ing.
- **BOT-034.** Cursor: the adapter must run only `agent -p` of the Cursor CLI. `agent` is the main name of the program. The installer of Cursor also makes the older name `cursor-agent` as a second link to the same program (checked 2026-11-17: `https://cursor.com/install`). The detection (BOT-022, BOT-040) must accept both names and must run the one that it found. It must treat the error
  of a team that disabled headless mode as a stop, tell the operator, and not try another way. It must not use the
  private endpoints of the Cursor client or a proxy for the Cursor models.
- **BOT-035.** Grok Build: the adapter must run only `grok -p` or `grok agent stdio` (ACP). Anima.ing
  must not read `~/.grok/auth.json`. For a business use of Grok Build, the operator must check the enterprise terms
  of the vendor (not checked in the table below).
- **BOT-036.** Google Antigravity and Gemini CLI: the configurator must not offer a Google account
  (consumer OAuth) login for a bot, because Google does not support the wrap of `agy` or Gemini CLI in a
  third-party agent on consumer plans. The configurator can offer them only with a Gemini API key,
  Vertex AI or Gemini Enterprise that the operator set in the program on the machine. The operator must
  confirm this before the bot starts, and the confirmation goes in the audit log.
- **BOT-037.** ACP agent that is not in the table: the configurator must show "terms not checked" with
  the link to the terms of the vendor of the agent. The operator must confirm that the agent uses his own
  login or key, and the confirmation goes in the audit log. Anima.ing must call the ACP `authenticate`
  method only with a method that needs no secret from Anima.ing (for example a cached login that the
  agent holds). It must not pass a token.
- **BOT-038.** Next to each agent, the configurator must show the verdict of the table below (allowed,
  restricted or unclear), its date of check and a link. The operator must confirm once for each agent
  that the login is his own and that the use follows the terms of the vendor. The confirmation goes in the
  audit log. A person must check each row again before each release, and the table must show the new date.
  A row that changes to "restricted" for a use that an adapter makes must turn that use off in the same release.

**Vendor terms, checked on 2026-11-16.** The question for each row: can a third-party app or an
orchestrator start the vendor's own subscription program on the user's own machine, with the login that
the user made there? "Allowed" means that the vendor's text names this use or documents it. "Restricted" means
that the vendor's text bans the use, or allows it only in a narrower form. "Unclear" means that no text
allows or bans it. The quotes are from the pages on the date of the check. This is not legal advice.

| Vendor | Verdict | Clause (quoted) | URL (checked 2026-11-16) |
| --- | --- | --- | --- |
| Anthropic: Claude Code and Claude subscription | **Restricted.** The unmodified `claude` program with the user's own login is allowed, including on a hosted platform. The Agent SDK on a subscription, and token handling, are not. Unattended use is unclear. | "Nor does it prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription, including where a platform hosts Claude Code". Also: "Anthropic does not permit third-party developers to offer Claude.ai login into their own applications, or to route requests through Free, Pro, Max, or Ultra plan credentials on behalf of their users. Moreover, developers may not collect, store, or intermediate Claude.ai credentials or session tokens". Also: "Customers may not pay for, resell, or intermediate Claude usage on their end users' behalf". Also: "Advertised usage limits for Pro, Max, and Ultra plans assume ordinary, individual usage". Also: "The Claude Code binary must not be modified". Consumer Terms (effective 2025-10-08): "Except when you are accessing our Services via an Anthropic API Key or where we otherwise explicitly permit it, to access the Services through automated or non-human means, whether through a bot, script, or otherwise", and "You may not share your Account login information, Anthropic API key, or Account credentials with anyone else." | https://code.claude.com/docs/en/legal-and-compliance and https://www.anthropic.com/legal/consumer-terms |
| OpenAI: Codex and ChatGPT plan | **Unclear.** No text bans or allows a third-party app that starts `codex exec` on the user's ChatGPT login. The `codex app-server` login is banned for a hosted service. | "App-server authentication has never been permitted for commercial or hosted services." Also: "Use API key authentication for programmatic Codex CLI workflows, such as CI/CD jobs. Don't expose Codex execution in untrusted or public environments." Terms of Use (effective 2026-01-01): "You may not share your account credentials or make your account available to anyone else", and the ban to "Automatically or programmatically extract data or Output". | https://developers.openai.com/codex/app-server, https://developers.openai.com/codex/auth and https://openai.com/policies/row-terms-of-use/ |
| Cursor: Cursor CLI | **Allowed** for `agent -p` on the user's own account. One conflict in the Acceptable Use Policy. No resale. | Docs: "Use print mode (`-p, --print`) for non-interactive scripting and automation." Terms of Service (updated 2026-09-03), 1.5(iii): "you may not ... rent, lease, lend, or sell the Service". Acceptable Use Policy (updated 2026-08-11) bans "Accessing the Service through automated or non-human means, whether through a bot, script, or otherwise". A Cursor staff reply on the forum says the CLI and the SDK are "the supported and safe path" and bans a proxy for the Cursor models. The reply is not a term. | https://cursor.com/docs/cli/headless, https://cursor.com/terms-of-service, https://cursor.com/acceptable-use-policy and https://forum.cursor.com/t/using-cursor-frontier-models-like-composer-2-5-in-external-harnesses-e-g-codex/164676 |
| xAI: Grok Build CLI (SuperGrok, X Premium Plus) | **Allowed** with the official `grok` program. No shared logins. | Docs: "Use headless mode for scripts, bots, or other machine-friendly tasks." Announcement: "Grok Build also provides full ACP support to build your own bots and agent orchestration apps." Consumer Terms (updated 2026-10-27): "You may not share your account credentials or make your account available to anyone else". Acceptable Use Policy bans "using bots to access, reverse engineer, decompile, disassemble or otherwise seek to obtain the source code of our Service" (the context is source code, so the clause is not a clear ban). | https://docs.x.ai/build/cli/headless-scripting, https://x.ai/news/grok-build-cli, https://x.ai/legal/terms-of-service and https://x.ai/legal/acceptable-use-policy |
| Google: Antigravity CLI and Gemini CLI | **Restricted.** On a consumer Google login, no third-party agent or orchestrator can drive the program, even when it never reads the token. An API key, Vertex AI or Gemini Enterprise is the allowed way. | Antigravity Additional Terms, Section 6: "Using third party software, tools, or services to access the Service (e.g. using OpenClaw with Antigravity OAuth) is a breach of this Agreement." FAQ: "Wrapping `agy` inside third-party coding agents or applications, or automating account rotation, is not supported under consumer plans." A Google reply on the Antigravity CLI tracker (2026-11-12) says what is not permitted on consumer sign-in is "using agy acp as a backend worker inside a third-party coding agent, autonomous agent harness, or background orchestrator", and that a person who drives an interactive ACP session is allowed. The reply is not a term. Gemini CLI: "Directly accessing the services powering Gemini CLI ... using third-party software, tools, or services ... is a violation of applicable terms and policies." | https://antigravity.google/terms/, https://antigravity.google/docs/faq.md, https://github.com/google-antigravity/antigravity-cli/issues/1493 and https://geminicli.com/docs/resources/tos-privacy/ |
| ACP (Agent Client Protocol) | **Unclear.** The protocol is open and has no terms on subscriptions. The terms of the agent's vendor apply. | The repository is under the Apache License 2.0. The protocol "standardizes communication between code editors ... and coding agents". Each agent does its own sign-in through the `authenticate` method. | https://github.com/agentclientprotocol/agent-client-protocol and https://agentclientprotocol.com |

**A named exception for T3 Code (R-196, operator, 2026-11-18, first-hand; OD-AXT15 of [agent-tools-as-extensions.md](agent-tools-as-extensions.md): "no, add T3 Code.").** BOT-021, BOT-032 and BOT-033 bind each adapter of Anima.ing, and they do not change. T3 Code is a program that the operator runs on his own machine, and T3 Code itself uses the Claude Agent SDK and the account methods of `codex app-server`. Anima.ing does not drive T3 Code through an adapter. R-196 permits a catalogue entry for T3 Code (AXT-020, AXT-045 and constraint C2 of that proposal) with these limits: the entry never offers, preselects or scripts the Claude subscription login, so Claude goes only through an API key, Amazon Bedrock or Google Vertex AI that the operator sets in T3 Code on the machine; the ChatGPT login of T3 Code stays on the device and never goes through the hosted hub; and the entry needs its vendor terms row (BOT-038). This exception is for T3 Code only.

**The claim of T3 Code, checked.** `docs/user/faq.md` says "T3 Code is within the TOS of Claude Code,
Codex, and Google Antigravity". The public text does not support this claim for Anima.ing.

- Claude: T3 Code drives Claude through the Agent SDK (`docs/user/faq.md`), and Anthropic's page says
  that products built on the Agent SDK should use API keys. T3 Code may have an agreement with the
  vendor. Anima.ing has none, so BOT-021 and BOT-032 forbid the SDK path.
- Codex: T3 Code uses the app-server account methods (`apps/server/src/provider/CodexProvider.ts`, `account/read`) and
  keeps a ChatGPT login. OpenAI allows a local or open-source app to keep doing this, and does not
  permit it for a commercial or hosted service. Anima.ing has a hosted mode, so BOT-033 forbids it.
- Antigravity: T3 Code handles the Google sign-in itself (`AntigravityAuth.ts`, `antigravityCallback.ts`).
  Google's FAQ does not support a third-party wrap on consumer plans. BOT-036 forbids it.
- Cursor and Grok Build: the vendor text and docs support the CLI path, so the claim holds for the
  documented CLI use.

**Agent coverage (proposed 2026-11-17, version 2; not ruled).** The note
`Market/Competitors/Analysis/Agent Coverage vs Orca.md` in the Anima.ing vault (checked 2026-11-17)
compares the agents that [Orca](https://github.com/stablyai/orca) lists (34 command-line agents) with
this proposal. It holds the install line, the sign-in and the effort for each agent. This section does
not copy them (PLN-016). The state today, by that note: the daemon detects 2 of the 34, Claude Code and
Codex, and does only that (`crates/daemon-core/src/inventory/tools.rs` prints their version; AU-1 will
read their session files for counts, AIS-018). This proposal names an adapter for 3 more (BOT-019,
BOT-034, BOT-035, BOT-036: Cursor, Grok Build, and Antigravity on an API key only). It covers 14 only
as "any ACP agent" (BOT-037). 15 have no entry. A generic runner (BOT-039) gives breadth for the cost of
a pane (the agent panes AIS-009 to AIS-013 are withdrawn by R-196; Herdr is hosted in their place). A named adapter gives depth and is worth building only for
the agents of Tier 1. BOT-039 to BOT-046 are proposed. A requirement here does not change BOT-019 to
BOT-038.

- **BOT-039.** Tier 0, the generic runner. The operator must be able to add an agent that has no adapter
  as a custom entry: a name, the program, the launch line, how the first prompt goes in (an argument, a
  flag, or text typed after the start), and the sign-in command. Re-planning (R-196 withdrew AIS-009 to AIS-013): the runner must start the program
  on a machine that the operator chose, as the operator's own user, in a Herdr pane when Herdr is installed
  (the Herdr tool extension of [agent-tools-as-extensions.md](agent-tools-as-extensions.md), AXT-026), or
  else in a `tmux` session that the daemon names and owns (as BOT-069). The runner needs no status adapter
  and no structured chat. [NEEDS CLARIFICATION: where the state of the runner comes from: version 1
  leaves the state of each agent to Herdr (R-196, OD-AXT10); the text was "from terminal activity
  (AIS-010)".] The terms gate (BOT-041) applies to each entry.
- **BOT-040.** The daemon must read one catalog of known agents, with one entry for each: the program
  name (or names: Cursor has `agent` and `cursor-agent`, BOT-034), the version command, the sign-in status command (or "not verified", BOT-022), how the first
  prompt goes in, and the row of the vendor terms table (BOT-038) or "terms not checked". A new entry
  for an agent must need no code when the agent runs in a pane (BOT-039). The daemon must find these
  programs with the steps of BOT-022 and must not read a login file (BOT-023). The state of each agent
  shows on the device page (BOT-024).
- **BOT-041.** The terms gate of BOT-037 and BOT-038 applies to each catalog entry and each custom entry,
  not only to an ACP agent. The configurator must show "terms not checked" and the link to the terms of
  the vendor, or the verdict and its date. The operator must confirm once for each agent that the login
  is his own, and the confirmation goes in the audit log. An entry whose verdict is "restricted" for the
  use that the entry makes must be off.
- **BOT-042.** Tier 1. The named adapters are Claude Code (BOT-032), Codex (BOT-033), Cursor (BOT-034)
  and Grok Build (BOT-035), and one generic ACP adapter that serves any ACP agent (BOT-019, BOT-037).
  Only the named adapters of Tier 1 can give a status that is more than terminal activity, or a
  structured chat. Claude Code and Codex must not use an ACP adapter, because the adapters that the
  public ACP agent list names for them use the SDKs that BOT-021 bans. Claude Code must run only as
  the unmodified `claude` program with the login of the operator (BOT-032).
- **BOT-043.** Tier 2. These agents get a catalog entry (BOT-040) and a version check, in this order,
  which is the order of their public use: OpenCode, Hermes Agent, Pi, Cline, Goose, oh-my-pi, Continue,
  Qwen Code, Kilocode, Kimi, Mistral Vibe, GitHub Copilot, Amp, Droid and Kiro. Each one runs in a pane
  (BOT-039), or through the ACP adapter when the agent speaks ACP by itself. A Tier 2 entry shows its
  verdict and date, or "terms not checked" (BOT-041). A person must write its row in the vendor terms
  table before the release that lists it by name (BOT-038), except for the entries that the operator
  chooses to run with "terms not checked".
- **BOT-044.** Tier 3. The configurator must not list these agents by name until a person has read the
  terms of the vendor and has written a dated row in the vendor terms table (BOT-038): DeepSeek Harness,
  ZCode, MiMo Code, OpenClaude, Devin, Auggie, Autohand Code, Charm (Crush), CodeBuddy, Codebuff,
  Freebuff, Command Code, Rovo Dev and Muse. The operator can still run any of them as a custom entry
  (BOT-039) with "terms not checked". Crush is under the FSL-1.1-MIT licence, which bans a "Competing
  Use". It must not be listed until counsel has read that clause.
- **BOT-045.** Parked: Antigravity (`agy`) and Gemini CLI on a consumer Google login. BOT-036 stays
  as it is: only a Gemini API key, Vertex AI or Gemini Enterprise that the operator set in the program on
  the machine. A catalog entry or a custom entry for these programs must not give a bot a consumer
  Google login. The generic runner (BOT-039) must not be a way around BOT-036.
- **BOT-046.** A catalog entry must not set up a second login of the same vendor, and it must not
  rotate or pool logins (BOT-025 (withdrawn, R-196), BOT-029). The configurator must keep to one login of each vendor on
  each machine until open decision 8 is ruled. A multi-account switcher is not a feature of any entry.

Terms limits that the note found, and that the requirements above already carry: Claude only through the
unmodified `claude` program with the operator's own login (BOT-021, BOT-032); the `codex app-server`
login is banned for the hosted mode (BOT-033), and `codex exec` on a ChatGPT login is unclear (open
decision 7); Google on a consumer login is restricted (BOT-036). The verdicts of the note for Copilot
(ACP documented), Amp (headless with an access token) and Droid (`droid exec` with an API key) are not
rows of the table below yet. They stay in the note until a person adds a dated row (BOT-038).

**Open decisions: agent coverage.** Each row is proposed, not ruled. Only the operator can rule it.

| Decision | Question | Recommendation |
| --- | --- | --- |
| OD-BOT1 | Is breadth, as the agents that Orca lists, a goal of the first release of Bots, and is the generic runner (BOT-039) the first work of Bots after the agent panes? | Proposed, not ruled: yes. A custom entry gives all 34 agents at launch-and-watch depth for the cost of the panes work. The panes work (AIS-009 to AIS-013, OD-AI4, OD-AI5) needs its own approval first. |
| OD-BOT2 | Which agents get a Tier 2 catalog entry, and in what order (BOT-043)? | Proposed, not ruled: the order of BOT-043. It comes from GitHub stars on 2026-11-17, a weak signal, and no public number exists for the closed agents (Cursor, Copilot, Amp, Droid, Kiro, Devin). |
| OD-BOT3 | Inside Tier 1, does the generic ACP adapter come before the named adapters of Claude Code and Codex (BOT-042)? | Proposed, not ruled: no. Claude Code and Codex first, as the two agents that Anima.ing detects today and that most operators have. The note estimates each of them at 1 to 2 weeks and the ACP adapter at the same size, once. |
| OD-BOT4 | Does Anima.ing list Crush at all (BOT-044)? | Proposed, not ruled: no, until counsel has read the "Competing Use" clause of its licence. |
| OD-BOT5 | May an operator run `agy` or Gemini CLI on a consumer Google login in a pane that he drives by hand (BOT-045)? A reply on the Google tracker of 2026-11-12 says an interactive ACP session by a person is allowed, and a background worker is not. The reply is not a term. | Proposed, not ruled: a bot never uses that login (BOT-036). A person's own terminal pane is his own use, and Anima.ing offers no entry for it. |
| OD-BOT6 | Do the agents that speak ACP only through a program that their vendor did not publish (Pi with `pi-acp`, Muse with an unofficial adapter) run through the ACP adapter (BOT-021)? | Proposed, not ruled: no. They stay terminal entries (BOT-039) until the operator rules. |
| OD-BOT7 | Do the 14 agents with no checked terms row show as "terms not checked" in the first release (BOT-041, BOT-043), or does a person read their terms first? | Proposed, not ruled: show them only as custom entries with "terms not checked" (Tier 3); read the rows of Tier 2 before the release (BOT-043). |

**Fallback to a local model when a usage limit stops a session (proposed 2026-11-18, version 3; not ruled).**
Founder direction of 2026-11-18, in his words: "Include something for spawning another claude code session
using Qwen 3.8 when usage limits expire can be down with finish work in claude code and can slop to the
local unsloth model". I read it as: when a Claude Code session stops on the usage limit of the operator's
subscription, Anima.ing can start a second session in the same project and the same pane, which runs a
local model on a machine of the operator, so that the work goes on. This is not a ruling (PLN-021). It
needs the panes (AIS-009 to AIS-013, direction only) and the model service of the compute plan (not
approved), so nothing here can be built yet (PLN-007).

What the documents already say. [agent-runs.md](agent-runs.md) has the same wish: "work continues when
a hosted model account refuses, with a model on a different machine", and AGN-014 says that the hub must
not read the usage limit of an account (CAP-025). The compute plan already has the parts of the local
model: the model library with pulls from the `unsloth` organisation of Hugging Face (LIBM-003, LIBM-006
gives `qwen3.8-27b:q4_k_m` as its example), the runtimes (RTC-001 to RTC-005, llama-server by default),
and a router that can forward the Anthropic Messages API (RTR-015). The usage counts (AIS-018 to
AIS-024) read the session files of Claude Code. They hold tokens, not limits, and AIS-022 forbids a read of
a usage limit. Thus the counts cannot tell that a limit was reached. This section adds the fallback and
does not copy those parts (PLN-016).

- **BOT-047.** Parked (R-196). The operator must be able to start a fallback session from a pane or a bot task of Claude
  Code (BOT-032), in the same repository folder (and the same worktree) and the same pane. The fallback
  session must run a model that a model server on a device of the operator serves. It must not use the
  subscription. It must show as a fallback in the pane, with the name of the model and the device.
- **BOT-048.** Parked (R-196). The local model is a capability of a device. The device page must show, for each device,
  one state for the fallback: no model server, model not pulled, memory below the need of the model, or
  ready. The model comes from the model library (LIBM-001 to LIBM-006), and the fit verdict comes from the
  compute plan. Unsloth gives the need of Qwen3.8-27B as 16 GB to 19 GB of memory for a 4-bit file
  (checked 2026-11-18; the source is in the note below). The hub must not pick the model for the operator
  (OD-BOT11). The probe of a device must run with an empty home folder (CAP-025).
- **BOT-049.** Parked (R-196). The fallback session must talk to the model server directly, from the device to the
  server. The hub must not carry the request (R-131). In the first version the server must be on the
  same device, on the loopback address. A server on a different device needs the path from one device
  to a different device, which is a spike (ai-direction.md, step 2; OD-BOT13).
- **BOT-050.** Parked (R-196). The operator must always be able to start a fallback by hand ("Continue on a local
  model"). Anima.ing can also offer it when the program stops on its limit. The detection must not use the
  token counts of AIS-018. It must not read a quota, a reset time, a login or an account (AIS-022,
  CAP-025). It can use only what the program itself prints in a pane that Anima.ing started (AIS-010) or
  the exit result and the message of a `claude -p` task of a bot (BOT-003). The patterns of the messages
  of each version of the program go in the catalog (BOT-040). A message that does not match a pattern
  gives no offer, and the manual start stays. Not verified: the exact text and the exit code of
  `claude` at its limit. Whether the daemon may notice that message at all is OD-BOT10.
- **BOT-051.** Parked (R-196). Anima.ing must not start a fallback unless the operator set a mode for the pane, the bot or
  the project: off, ask each time, or start by itself. The default is open (OD-BOT9). In each mode, the
  start must go in the audit log (BOT-007) and send the alert of AIS-012, and the pane must show the
  fallback (BOT-047). The session must not have more rights than the session that stopped. The standing
  permissions of BOT-006 carry over only when the operator says so (OD-BOT12).
- **BOT-052.** Parked (R-196). The fallback of Claude Code must run the unmodified `claude` program as the operator's own
  user (BOT-021), with these values set for that process only, and not written into a settings file of the
  operator: `ANTHROPIC_BASE_URL` with the address of the local server, and a credential variable
  (`ANTHROPIC_API_KEY` or `ANTHROPIC_AUTH_TOKEN`) with a value that is not a secret. The reason is in the
  Anthropic text below: with the base address only, the saved claude.ai login stays the active credential,
  and its token goes to the address. The adapter must never read, pass, copy or keep a login or a token
  (BOT-020, BOT-023, BOT-032), and must not set `CLAUDE_CODE_OAUTH_TOKEN`. A test must start the program
  against a sandbox server with a saved login and fail when the server gets an `Authorization` header
  from that login. The adapter can set `CLAUDE_CODE_ATTRIBUTION_HEADER=0`, because Unsloth reports that
  the header makes a local model about 90 % slower by a cache miss on each request. This requirement waits
  for OD-BOT8.
- **BOT-053.** Parked (R-196). The safe alternative must exist before BOT-052 is on: a fallback with an open harness that
  speaks to the local server in the same pane, for example OpenCode, Hermes Agent or Pi, which Unsloth also
  starts against a local model (`unsloth start opencode`). It is an entry of the catalog (BOT-040) and runs as
  a runner (BOT-039). The operator picks the harness for the fallback in the same setting as the mode
  (BOT-051). With "Claude Code" not allowed (OD-BOT8), this is the only fallback.
- **BOT-054.** Parked (R-196). The handoff must stay on the device. The daemon must write one file in its own state
  folder, with the rights of the user only and outside the repository, and the first request must name its
  path. The file holds: the request that the operator wants finished, as one line that the operator
  confirms (for a bot task, the title of the task, and the bounded handoff of BOT-026 and BOT-027 with
  the context window of the local model); the branch, the head commit, the list of the changed files
  with the counts of `git diff --stat`, and the last five commit subjects. It must not hold diff text, the
  environment, a file that matches a secret pattern (`.env*`, key files), or a login. The hub gets only
  the event that a fallback started and ended (BOT-056). Resuming the stopped session with `claude -r` on
  the local model (BOT-028) is not verified: the stored thinking of a different model can fail. It is
  an option only after a test.
- **BOT-055.** Parked (R-196). Switch back. The daemon does not know when a limit resets and must not read it
  (AIS-022). The pane must always offer "Return to Claude Code". The operator can set a reminder time
  (the alert of AIS-012). Two sessions must not write in one worktree: the fallback must end or pause
  before the Claude session starts. The return gives the Claude session a handoff of what changed since
  the fallback began (BOT-054, BOT-028). Return is never automatic, because a failed return would lose
  the work (OD-BOT9).
- **BOT-056.** Parked (R-196). The audit log (BOT-007) must record each start, stop and return of a fallback, who gave it
  (the operator or a rule), the device and the model name. The usage page (AIS-020) must show the tokens
  of a fallback session as local use and give no cost estimate for them: the public prices (MGW-014) have
  no row for a local model, and AIS-021 must not apply a Claude price to them. A session file of a fallback
  also shows as `claude` in the usage counts; the model name tells them apart.
- **BOT-057.** Parked (R-196). The fallback is a kind of runner. It is a harness (BOT-040) and a model server of a device,
  and the operator adds it from the Agents page and from "Add runner" (PNL-002, PNL-003, PNL-005). The
  menu of each pane has "Continue on a local model" (BOT-050). The local model appears in the model
  address of the compute plan when that is built (RTR-015; ai-direction.md step 4), but the first version
  needs only the server on the device (BOT-049).

Sources, read on 2026-11-18. Not legal advice.

- Qwen: "Qwen 3.8" exists. Qwen3.8-Max was announced on 2026-08-02 (`https://qwen.ai/blog?id=qwen3.8`).
  Open weights: Qwen3.8-2.4T-A95B, 2.4 trillion parameters and about 95 billion active, on 2026-08-12, and
  Qwen3.8-27B, a dense model with vision under the Apache 2.0 licence, on 2026-08-14
  (`https://github.com/QwenLM/Qwen3.8`, `https://www.alibabacloud.com/blog/alibaba-unveils-qwen3-8-27b-and-releases-weights-of-qwen3-8-flagship-model_603463`).
  Alibaba Cloud lists later hosted versions (Qwen3.8-Max-0902 of 2026-09-02, Qwen3.8-Flash of 2026-08-26):
  `https://docs.modelstudio.console.alibabacloud.com/en/model-studio/newly-released-models`. Only the 27B
  runs on one ordinary machine.
- Unsloth: Qwen3.8-27B GGUF files (Dynamic v3.1 of 2026-11-11): `https://huggingface.co/unsloth/Qwen3.8-27B-GGUF`.
  Its guide gives 16 GB to 19 GB for 4 bits, 23 GB to 26 GB for 6 bits, and a 256 000-token window:
  `https://unsloth.ai/docs/models/qwen3.8`. Claude Code with a local model through `ANTHROPIC_BASE_URL`
  and llama-server, and the attribution header fix: `https://unsloth.ai/docs/basics/claude-code`.
  `unsloth start claude` and the other agents: `https://unsloth.ai/docs/integrations/unsloth-start`. The
  guides name Qwen3.5 and Qwen3.6 in their Claude Code examples; the Qwen3.8 page gives the run command.
- Anthropic: a gateway on `ANTHROPIC_BASE_URL` is a documented setup. "Anthropic doesn't endorse, maintain,
  or audit third-party gateway products, and doesn't support routing Claude Code to non-Claude models
  through any gateway." While a gateway credential variable is active, the subscription login is not sent;
  with the base address only, "a saved claude.ai login remains the active credential"
  (`https://code.claude.com/docs/en/llm-gateway`). The legal page asks for an unmodified program with its
  sign-in methods kept, and bans collecting, storing or intermediating Claude.ai credentials
  (`https://code.claude.com/docs/en/legal-and-compliance`). Finding: the texts do not ban an unmodified
  `claude` that has a different base address and no subscription login, and they say that this use is not
  supported. This is **unclear**, not allowed. It is OD-BOT8.

**Open decisions: fallback to a local model.** Each row is proposed, not ruled. Only the operator can rule it.

| Decision | Question | Recommendation |
| --- | --- | --- |
| OD-BOT8 | May Anima.ing start the unmodified `claude` program against a local server that is not Anthropic's (BOT-052)? Anthropic does not support it and has no text that bans it. | Proposed, not ruled: yes, as an option, with the safe values of BOT-052 and a note in the pane that Anthropic does not support it. The traffic never goes to Anthropic and no Anthropic login is used. Ask Anthropic in writing before the release (as in open decision 7). The open harness (BOT-053) is the alternative, and it is built first. |
| OD-BOT9 | What is the default mode of a fallback (BOT-051): off, ask each time, or start by itself? The operator ruled full auto for the security choices (R-120). Return (BOT-055) is never automatic in this text. | Proposed, not ruled: ask each time. A model with less skill has the same rights, so a start by itself is a choice for each project. |
| OD-BOT10 | Does AIS-022 ("must not read ... a usage limit"), which R-135 approved, forbid that the daemon notices the limit message that the program prints in a pane (BOT-050)? | Proposed, not ruled: no, because it reads no quota or account. The text of AIS-022 then needs one sentence. If the operator says yes, only the manual start stays. |
| OD-BOT11 | Which local model does the fallback use? Stephen named Qwen 3.8. The open Qwen3.8 weights are the 27B and the 2.4T. | Proposed, not ruled: Anima.ing sets no default model. It offers the variants that the fit verdict of the device allows (BOT-048), with Qwen3.8-27B first for a device with 16 GB or more. |
| OD-BOT12 | Does the fallback keep the standing permissions of the stopped session (BOT-006, BOT-051)? | Proposed, not ruled: no. It starts with fewer rights (no push to a shared branch, no delete), and the operator adds them. |
| OD-BOT13 | Is the server on the same device in the first version (BOT-049), or on any device of the operator? | Proposed, not ruled: the same device. A server on another device waits for the spike of the path from one device to another. |
| OD-BOT14 | Does the fallback also cover Codex when it stops on its limit? | Proposed, not ruled: later. Claude Code first. The terms of Codex are unclear (BOT-033). |

**Depends on.** Model gateway (XP-15), the Herdr tool extension of agent-tools-as-extensions.md (R-196 withdrew the agent panes of Section 4.12.1, AIS-009 to AIS-017; BOT-039 uses a Herdr pane or `tmux`), the rule engine (ENG-001), and
for long tasks the jobs and agent-runs proposals. Rented bot computers (BOT-010) depend on the
cloud machines proposal and are billed like any rented machine (CVM-009, CVM-010). The install of an agent (BOT-058 to BOT-078) depends on a work type for the daemon (OD-BOT16; the work route of task W1 in [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md)), on BOT-022 to BOT-024 and BOT-040, and on a session that stays alive (OD-BOT19).

**Risks.**

- Subscription terms: driving a Claude or ChatGPT plan from our own code may break the vendor's
  terms. Calling the vendor's own program (BOT-003) is the safe path. The vendor terms table above
  (checked 2026-11-16) shows that this is allowed for Cursor and Grok Build, restricted for Claude Code
  and for Google, and unclear for Codex. BOT-020 to BOT-038 (and BOT-045) make the restrictions into requirements.
  A vendor can change its terms at any time. A person must check each row again before each release (BOT-038).
- "Always on" only holds while the device is on. A cloud computer add-on would cover users with one
  laptop ([cloud-machines.md](../proposals/cloud-machines.md)).
- A bot with shell access is an agent that runs code. It needs the same isolation ruling as
  community extensions (EXT-009) before it runs anything that is not the operator's own instructions.
  A bot's own computer (BOT-010) is the simplest way to meet that ruling, because a bad command
  breaks only that computer.
- A local virtual machine needs a hypervisor on the device: Apple Virtualization on macOS, KVM on
  Linux, Hyper-V on Windows. Memory and disk for each bot come out of the operator's device.

**Open decisions.**

1. Whether a bot's chat lives in the Anima.ing client only, or also in Slack, iMessage or Telegram.
2. Whether bots can talk to each other (a team of bots), or only to the operator in the first version.
3. How the router judges "the cheapest model that can do the task".
4. Whether a bot's computer is kept or wiped between tasks, and whether read-only bots get one at all.
5. Whether the terms of a coding-agent subscription allow its use on a cloud machine, and how a task
   moves to another machine when the first is off (BOT-014 to BOT-018). The table above answers the
   first part for each vendor: Claude Code allows the user's own sign-in on a hosted machine, Google does
   not on a consumer login, and Codex is unclear. Proposed, not ruled.
6. Whether Anima.ing can offer an install command for a vendor program that the operator starts on a
   rented machine (BOT-031), or must first sign the Commercial Terms of Service of Anthropic. Proposed,
   not ruled.
7. The limit for "ordinary, individual usage" of a subscription (BOT-032), and whether an unattended
   run of `claude -p` or `codex exec` on a subscription is inside the terms of the vendor. Ask each
   vendor in writing before release. Proposed, not ruled.
8. Whether one machine can hold a second login of the same vendor, for example a work account and a
   personal account (T3 Code allows it with `CLAUDE_CONFIG_DIR` and `CODEX_HOME`). BOT-025 bans only the
   rotation and the pool of logins. Proposed, not ruled.

## Install an agent on a machine

**Install an agent on a machine (proposed 2026-11-18, version 4; the operator decided the open decisions on 2026-11-18 in R-153, version 5; the requirements are not approved).** This section adds one action to the configurator of BOT-014 to BOT-018: "Add an agent". The operator picks a machine and a coding agent, reads what will run, and clicks once. The daemon installs the vendor's own program as the operator's user, the operator signs in on the machine in the vendor's own flow, and the agent starts in a session that stays alive. It is a change to this proposal and not a new document (PLN-004, PLN-016): this document owns the agent catalog (BOT-040), the device states (BOT-024), the vendor terms gate (BOT-038, BOT-041) and the sign-in rules (BOT-020, BOT-023). The text comes from a spec of the operator's marketing bot of 2026-11-18. Its private part stays in the business vault. Its placeholder identifiers OCA-01 to OCA-12 are replaced by BOT-058 to BOT-078. The operator decided OD-BOT15 to OD-BOT20 on 2026-11-18 (R-153). He chose six agents for the first version, and he chose that installs follow the MCP setting. Nothing is built. Nothing here is approved, built or claimed as a function (PLN-007).

Serves: no row of Section 4 of the work plan names Bots yet. The nearest row is 4.8, "The other extensions" (AI coding sessions; the agent panes AIS-009 to AIS-017 are withdrawn by R-196). [NEEDS CLARIFICATION: the operator names the row that Bots serves (PLN-003) before approval.]

### Goal

A person who has a machine with `anima-daemon` can put Claude Code, Codex, Gemini CLI, Cursor CLI, Pi or OpenCode on it from the device page, in a few minutes, without a terminal. Each step is shown first and is in the audit log. Anima.ing never holds the login of the person.

### Current state (code on `origin/main` at `7342500c`, 2026-11-18)

| Piece | State | Source |
| --- | --- | --- |
| The daemon finds `claude` and `codex` and prints their versions. The probe runs with an empty home folder, so it touches no login. | Built. Detection only. | `crates/daemon-core/src/inventory/tools.rs`, `probes.rs` (`own_home: true`) |
| The hub gives the found tools by device | Built | `contracts/schema.graphql`, `tools: [Tool!]!` (CAP-023, CAP-024) |
| The device page list of agents with a sign-in state (BOT-022, BOT-024) | Not built. No `BOT-` identifier is in `crates/`, `apps/` or `contracts/`. | `rg 'BOT-0' crates apps contracts` finds nothing |
| The catalog of known agents (BOT-040) and the generic runner (BOT-039) | Not built | same |
| Any way for the hub to make the daemon run something | Not built. The daemon only sends reports out (K8). It reads no work and listens on no port. | `crates/daemon/src/push/`, `contracts/daemon-push.md`; USE-005 and USE-006 say how work will come |
| The work route (claim, report, log, stop) and its fixed list of work types | Not built. Task W1 is Not started. R-160 (2026-11-18) is the ruling that changes R-115; the contract and the security review come before code. | `docs/plans/projects-and-builds-plan.md`, task W1; `docs/proposals/jobs.md`, JOB-011 |
| Agent panes (AIS-009 to AIS-017) | Withdrawn (R-196). Herdr is hosted as a tool extension in their place. | `docs/plans/extensions-plan.md` Section 4.12.1; [agent-tools-as-extensions.md](agent-tools-as-extensions.md) |
| SSH terminal in the client (the hub opens SSH and relays bytes) | Built | R-062, `contracts/tunnel.md` Section 8, `crates/hub/src/fleet/sessions/terminal/` |
| Agent usage counts (AU-1 to AU-5) | Built; waits for HT-026 | `crates/hub/src/fleet/agent_usage/` |
| The MCP server: change tools, the rule that no tool runs a command on a device | Built (the default of R-120 is ruled and not built) | `docs/plans/cli-mcp-plan.md`, MCP-002, MCP-006, MCP-024 to MCP-026 |
| The hash-chained audit log | Built; waits for HT-023 | `crates/hub/migrations/0506_events_audit_chain.sql`, `anima audit verify` |

### Sealed terminal: what is ruled, what is open

The question: is terminal traffic sealed from end to end, so that a one-time sign-in code that the operator types in a pane cannot be read on the way?

- **Ruled.** SSH sessions run from the hub. The hub opens the SSH connection with `russh` and relays terminal bytes to the client over a WebSocket (R-062, K4 Section 8). The hub decrypts and encrypts the bytes at its own end of both connections. So the terminal is **not** sealed from end to end, by design. `crates/hub/src/fleet/sessions/terminal/relay.rs` is the relay.
- **Ruled.** The hub must not record the content of a session (RA-008) and no log may hold the content of a session (LOG-007). Recording is deferred (RA-040, R-113).
- **Not ruled.** No requirement says that terminal bytes must be sealed. In the extensions plan, "sealed" means encrypted at rest with a data key, and "client-sealed" means encrypted on the client (Sections 1.3 and 2.4). Neither word is about the terminal.
- **Decided (R-196).** The agent panes (AIS-009 to AIS-017) are withdrawn. A pane is a Herdr pane, opened in the SSH terminal of the device (AXT-014 of agent-tools-as-extensions.md).
- **Decided (R-153, 2026-11-18, OD-BOT15).** A one-time sign-in code may pass through the hub as plain text in the first version, with a notice. The operator's words are in [rulings.md](../rulings.md). This covers a one-time code. It does not cover a key that lasts (OD-BOT23).

Does it block this change? **No.** The operator decided OD-BOT15 (R-153): the code passes through the hub as plain text with a notice. A sealed channel is not part of the first version. The hub already carries a password that the operator types in an SSH session in the same way. BOT-068 makes the pane for the sign-in a "secret entry" pane with a notice. [NEEDS CLARIFICATION: whether a code of the "copy the URL, paste the code back" flow of Claude Code is useless to a reader of the traffic. The code is bound to the machine that made the request, but this was not read from the vendor's documents.]

### Overlap check (PLN-004)

| Subject | Owner today | Result |
| --- | --- | --- |
| Configurator, agent states, terms gate, catalog, sign-in rules | This document (BOT-014 to BOT-046) | Change this document. |
| Agent panes and the session that stays alive | Herdr, as a tool extension of [agent-tools-as-extensions.md](agent-tools-as-extensions.md) (R-196 withdrew AIS-009 to AIS-017 of the extensions plan) | Use it. Do not copy it. BOT-069 names the `tmux` fallback. |
| Work that the daemon takes, and its fixed list of work types | [jobs.md](../proposals/jobs.md) (USE-004 to USE-006, JOB-011); task W1 of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) | Use it. The install is one more work type (OD-BOT16). |
| Containers and recipes of hosted applications | [hosted-applications-plan.md](../plans/hosted-applications-plan.md) (HOST-003, HOST-012) | Different subject: containers, not agent programs. No overlap in code. |
| Machine profiles (copy a setup to a new computer) | [machine-profiles.md](../proposals/machine-profiles.md) (REP-022) | Proposal only. It can use the catalog later. |
| Update of `anima-daemon` and the hub | [daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md) (ROL) | Different programs. BOT-071 does not update the daemon. |
| Installing Anima.ing itself | design specification Section 3.11 (INS-001 to INS-013) | Different subject. |
| Ideas folder, deferred list | `docs/ideas/` has no folder for this. `docs/deferred.md` has no item for it. | No overlap. |

### Not in scope

- Windows machines, rented bot machines (BOT-010), and machines that do not run `anima-daemon`.
- Agent apps that run a server (Orca, Paseo). They need a port and a pairing secret. They wait for a later version of this section. T3 Code is a tool extension of [agent-tools-as-extensions.md](agent-tools-as-extensions.md) (R-196), with its recipe in this catalogue (BOT-077) and the named exception before "The claim of T3 Code, checked".
- Grok Build, Copilot CLI, Antigravity (BOT-045) and the other agents of BOT-043 and BOT-044. They need a dated terms row first (BOT-038). The six agents of the first version are in BOT-077.
- Automatic update, "update when idle", and a bulk "set up all my machines" action.
- Showing a device code or a pairing link in the client.
- Any claim in the marketing copy. That is for the business vault.

### Requirements

All requirements here are proposed and not approved. A requirement is not built until the operator approves it with a ruling (PLN-007).

- **BOT-058.** The device page must show an "Add an agent" action. Each row of the list of BOT-024 must show the action for its state: Install, Sign in, Open, or Update. A machine that no recipe supports must be grayed, and the page must say why.
- **BOT-059.** Before anything runs, the client must show one review screen. It must give: the method (a package manager, or the vendor's script), the exact source address, the version, the checksum that the daemon will check or "the vendor gives none", that the install is in the home folder with no root, that no port is opened, and whether the program starts at login. It must show the vendor terms row of BOT-038 with its check date and the one confirmation of BOT-041.
- **BOT-060.** A recipe must be data in the catalog of BOT-040, with the install fields that this section adds. The catalog must come with a daemon release. The daemon must not fetch a recipe from the hub or from the network at run time. There must be one catalog and not a second one.
- **BOT-061.** A recipe must be a list of fixed steps. Each step must be a program and an argument list. The daemon must not build a command line from text that the hub, the operator or an AI agent gave. Only the machine, the agent and the folder for the session are input.
- **BOT-062.** The install must use, in this order: a signed package that the machine's own package manager gives and the machine already uses, or the script of the vendor. For a script, the daemon must download it to a file, check its checksum when the vendor gives one, write its SHA-256 and its source address to the audit log, and only then run it. The daemon must never send a download straight into a shell.
- **BOT-063.** The daemon must run each step as the operator's own user. It must not use `sudo`. A step that needs root must stop and tell the operator which step, and must not ask for a password.
- **BOT-064.** A recipe of the first version (BOT-077) must not open a port or start a service. A later recipe that opens a port must open it only on the address of the machine on the Anima.ing network, never on all addresses, and the review screen must say so. A tool extension of agent-tools-as-extensions.md may listen on the loopback address of the device only (AXT-008; R-196, OD-AXT4).
- **BOT-065.** The operator must be able to start each install, update and removal in the app or in the command line, and must approve it on the review screen (BOT-059). An install that an AI agent asks for through MCP follows the MCP setting (R-120, MCP-012, MCP-024, MCP-025; operator, R-153, OD-BOT17): in the state `allow` it runs without a question, in `ask` the person is asked, in `off` the tool is hidden. A schedule, an event, a bot or a standing permission (BOT-006 (withdrawn, R-196)) must not start an install. A machine takes an install only when it takes work (USE-004; operator, R-153, OD-BOT20). An install that an AI agent starts must show as started by an AI agent, with the name of the MCP client (BOT-073). Exception (R-196, OD-AXT5): an AI agent must not install Hermes Agent through MCP in any state, `allow` included; the install of Hermes always needs the click of the operator (AXT-021). [NEEDS CLARIFICATION: MCP-006 says that no tool runs a command on a device. An MCP install tool needs a narrow exception, OD-BOT21. Whether an agent may start a recipe before the operator confirmed its terms row on that machine is OD-BOT22.]
- **BOT-066.** Each sign-in must happen on the machine, in the flow of the vendor, by the operator. This extends BOT-020 and BOT-023. Anima.ing must not read, copy, move or keep a token, key, cookie or login file. It must not run `claude setup-token`. It must give no field that takes an API key. It must never offer or automate the copy of a login file to another machine.
- **BOT-067.** An agent has one of two sign-in kinds. (a) An account login of the vendor: Claude Code, Codex and Cursor CLI. The flows, in order of preference, are a device-code flow of the vendor, a browser on the machine itself, and, for a machine with no browser, the prompt of the vendor ("open this address on your own device and paste the code back"). (b) A key that the operator sets in the program on the machine: Gemini CLI, Pi and OpenCode (BOT-036, BOT-078). For kind (b) the review screen must say how the operator sets the key on the machine, and Anima.ing must open no pane for it in the first version (OD-BOT23). The review screen must name each condition that the vendor sets. See the table of recipes below for what is verified.
- **BOT-068.** The pane in which the operator signs in must be marked "secret entry" and must show a notice: the hub relays this text and does not store it (operator, R-153, OD-BOT15). The hub must not record or log its bytes (RA-008, LOG-007). The audit log must say only that the sign-in started and finished, with the name of the method (BOT-022). The result that the page shows is only the exit code and the method name (BOT-024). The operator must open the sign-in pane in the app or in the command line. An MCP tool must not open it (MCP-006), and no MCP tool returns a code (MCP-004). The pane is for a one-time code. It is not for a key that lasts (BOT-078).
- **BOT-069.** After the sign-in, the daemon must start the agent in a folder that the operator chose: in a Herdr pane when Herdr is installed on the machine (AXT-026), or else in a `tmux` session that it names and owns. R-196 withdrew the agent panes (AIS-013). It must check that `tmux` is present and, if it is not, say so and stop. It must not install `tmux` when that needs root (BOT-063). [NEEDS CLARIFICATION: OD-BOT19.]
- **BOT-070.** After the install, the daemon must check the program with the steps of BOT-022: the file in the path, the version, and the status command. The page must show the result with the states of BOT-024. A failed step must stop the install, name the step and the cause, and leave nothing half-installed that it made.
- **BOT-071.** The default policy for an update is "Manual". The hub must compare the version of the daemon's probe with the version of the catalog and show "update available". A version that changed without Anima.ing must be logged as "changed by the vendor". Claude Code (native install) and Cursor CLI update themselves by default, so this log line is the normal case for them; the recipe must set the switch of the vendor that turns this off when the vendor has one. [NEEDS CLARIFICATION: whether each vendor has such a switch.] An update must show what it changes, and a failed update must leave the old version working. In the first version there is no automatic update by Anima.ing.
- **BOT-072.** A removal must use the steps of the vendor, must keep the data of the operator unless the operator chooses otherwise, and must be in the audit log.
- **BOT-073.** The audit log must hold one event for each of: terms confirmed; install requested, approved or denied; each install step (started, succeeded, failed, with the step name, the exit code and the time); install finished (result, version, SHA-256 of the script); sign-in started and finished (method name only); session started (pane, folder); update available, started, finished or rolled back; version changed by the vendor; removal. Each event must name the actor (operator in the app, operator in the command line, or an AI agent that asked, with the name of its MCP client as in MCP-026), the device, the agent and the time. No event may hold a token, an account name, an email address, a pairing link, a password, the bytes of a sign-in pane, or the text of a script.
- **BOT-074.** The catalog must be able to switch one recipe off in a daemon release, for example when a vendor changes its terms. A person must read the terms row of BOT-038 again before each release, and a row that turns "restricted" must turn its recipe off in the same release.
- **BOT-075.** The function must have the command `anima agents install`, `status`, `update` and `remove`, and the MCP tools for the states and the versions (CMD-030). Install, update and remove are MCP change tools. They follow the MCP states and the switch "Ask before destructive actions" (MCP-012, MCP-024, MCP-025, R-153); `remove` has `destructiveHint` (MCP-009). Each call that runs without a question must be in the audit log (MCP-026, BOT-073). The tool starts only a recipe of the catalog (BOT-060, BOT-061). It must not open a session (MCP-006) and it must not take or return a secret (MCP-004). [NEEDS CLARIFICATION: OD-BOT21, the exception to MCP-006.]
- **BOT-076.** The install runs on a machine of the operator, through the daemon. The images of rented machines and the hosted hub must carry no vendor program (BOT-031). The Claude Code recipe must work only on a machine that the operator owns, until open decision 6 above is ruled.
- **BOT-077.** The first version must offer six recipes (operator, R-153, OD-BOT18): Claude Code, Codex, Gemini CLI, Cursor CLI, Pi and OpenCode, on macOS and on Linux, on a machine that runs `anima-daemon`. Pi replaces Aider (R-196, OD-AXT16: "replace Aider with Pi."). Each one is a harness that runs in a pane. Each recipe must carry its row of the vendor terms table, or "terms not checked" (BOT-041). The same catalogue also holds the recipes of three tool extensions of [agent-tools-as-extensions.md](agent-tools-as-extensions.md) (R-196): Hermes Agent, suggested first in "Add an agent" (AXT-021, AXT-023), Herdr (AXT-024) and T3 Code (AXT-045). They are tools, not harnesses. No other recipe is in the first version.
- **BOT-078.** A recipe for an agent that takes a key (Gemini CLI, Pi, OpenCode) must install the program and must not take, show, store or relay the key. The operator sets the key in the program on the machine himself (BOT-036, BOT-066). A recipe must not offer, preselect or script a login of a vendor for a program that is not the program of that vendor: not an Anthropic subscription (BOT-021, BOT-032), not a ChatGPT login (BOT-033), and not a consumer Google login (BOT-036, BOT-045). The Gemini CLI recipe must say that Google gives Gemini CLI to Code Assist Standard and Enterprise licences and to a paid API or Vertex key (README of the program, read 2026-11-18). Pi connects "a subscription or API key" with `/login` inside the program (README of Pi, read 2026-11-18): the Pi recipe must name only a key or a provider that BOT-021, BOT-032, BOT-033 and BOT-036 permit.

**The six recipes of the first version (BOT-077).** The commands below were read from the official pages named in the table on 2026-11-18. A command that is marked "not verified" was not on those pages. It comes from a note of the operator's marketing bot or from memory, and a person must check it in task BI4 and in the hand test of BI11 before it enters the catalog. [NEEDS CLARIFICATION: every "not verified" item.] Each recipe also needs the probe of BOT-022 with an empty home folder (CAP-025); the daemon has probes for Claude Code and Codex only (`crates/daemon-core/src/inventory/tools.rs`), so each other agent needs a new `ToolName`.

| Agent | Install (verified) | Sign in | Update | Limits that the pages give | Not verified |
| --- | --- | --- | --- | --- | --- |
| Claude Code | `curl -fsSL https://claude.ai/install.sh \| bash` on macOS and Linux; then `claude --version`. Source: [setup page](https://docs.claude.com/en/docs/claude-code/setup), 2026-11-18. | Kind (a). The browser opens on the first start; or `claude auth login` first. Source: [authentication page](https://code.claude.com/docs/en/authentication), 2026-11-18. | The native install updates itself in the background (setup page). | The same pages list apt, dnf and apk packages that need root (BOT-063: not used). `claude setup-token` makes a long-lived token and is banned (BOT-066). Terms: BOT-032. | `claude auth status`; `claude update`. |
| Codex | `curl -fsSL https://chatgpt.com/codex/install.sh \| sh` on macOS and Linux. Source: [Codex CLI page](https://developers.openai.com/codex/cli), 2026-11-18. | Kind (a). `codex login` opens the browser and is the default. Source: [auth page](https://developers.openai.com/codex/auth), 2026-11-18. `codex login --with-api-key` takes a key from standard input: not used (BOT-066). | The same install command updates it (Codex CLI page). | The auth page mentions device-code failures in the login log. The flag that starts a device-code flow was not read. Terms: BOT-033, open decision 7. | `codex --version`; `codex login status`; the device-code flag. |
| Gemini CLI | `npm install -g @google/gemini-cli`, or `brew install gemini-cli`. Source: [README](https://raw.githubusercontent.com/google-gemini/gemini-cli/main/README.md), 2026-11-18. | Kind (b) in the first version. The README names a Google sign-in for Code Assist licences of an organisation, the `GEMINI_API_KEY` variable, and Vertex AI. | `npm install -g @google/gemini-cli@latest` (README). | The README says that Gemini CLI is for Gemini Code Assist Standard and Enterprise, and that free and Google AI plans moved to Antigravity CLI on 2026-06-18. BOT-036 and BOT-045 still ban a consumer Google login and Antigravity. | Whether `npm install -g` needs root on the machine (BOT-063); Node is a prerequisite. |
| Cursor CLI | `curl https://cursor.com/install -fsS \| bash` on macOS, Linux and WSL; the program is `agent` (`agent --version`); `~/.local/bin` must be in the path. Source: [installation page](https://cursor.com/docs/cli/installation), 2026-11-18. | Kind (a). `agent login` opens the browser; `NO_OPEN_BROWSER=1` prints the address; `agent status`; `agent logout`. Source: [authentication page](https://cursor.com/docs/cli/reference/authentication), 2026-11-18. | It updates itself by default; `agent update` updates by hand (installation page). | BOT-034 names two program names, `agent` and `cursor-agent`. An API key in the environment is the other method: not used (BOT-066). | Whether the automatic update has a switch. |
| Pi | `curl -fsSL https://pi.dev/install.sh \| sh`; or `npm install -g --ignore-scripts @earendil-works/pi-coding-agent`. Node.js 22.19 or newer. Source: [README](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/README.md), 2026-11-18, read only; not run on a machine. | Kind (b). `/login` inside the program connects "a subscription or API key" (README). Only a key or a permitted provider (BOT-078). | `pi update` (README). | MIT. A terminal agent. As an ACP agent (BOT-019) it needs the adapter `pi-acp` ([svkozak/pi-acp](https://github.com/svkozak/pi-acp), MIT); whether the recipe installs it is not decided. Pi is not named in BOT-043 or BOT-044, so its terms row is "terms not checked" (BOT-041). | Each command on a real machine; the version command; whether `npm install -g` needs root (BOT-063); the commands of `pi-acp`. [NEEDS CLARIFICATION: whether the Pi recipe installs `pi-acp`.] |
| OpenCode | `curl -fsSL https://opencode.ai/v2/install \| bash`; or `npm install -g @opencode/cli`; or `brew install anomalyco/tap/opencode-v2`. Source: [docs](https://opencode.ai/docs/), 2026-11-18. | Kind (b). The docs connect a provider with `/connect` inside the program. | The page does not say. | The page says that an older `mise` installs OpenCode 1. OpenCode is first in the order of BOT-043. Its providers include logins that BOT-021, BOT-032 and BOT-033 ban for a program that is not the vendor's own (BOT-078). | `opencode auth login`; the update and version commands. |

**Aider (removed from the first version).** R-196 (OD-AXT16) removes Aider from the six agents: its last release is of 2025-08-09 (stale, per Anima Competitor Bot, 2026-11-18). Aider can come back as a later recipe only with a new decision. Its install facts, for a later version: `python -m pip install aider-install`, then `aider-install` ([install page](https://aider.chat/docs/install.html), 2026-11-18); kind (b); terms not checked (BOT-041).

### Interfaces (CMD-030 order)

1. **Hub API (K1).** A query for the install requests of a device and their steps. A mutation to make a request, one to approve or deny it, and one to cancel it. The request holds: device, agent, recipe version, state, steps, and who started it (the operator in the app, the operator in the command line, or an AI agent with the name of its MCP client). It holds no secret. Each new root field needs its command (CMD-031).
2. **`anima` command.** `anima agents install <agent> --device <device>`, `anima agents status`, `anima agents update`, `anima agents remove`. The names follow `anima agents usage`, which exists. [NEEDS CLARIFICATION: the names are a suggestion for the interface review.]
3. **MCP tool.** Read tools for the states and the versions (BOT-024), and the change tools install, update and remove. They follow the MCP states and the switch of MCP-025 (R-153, OD-BOT17). A tool starts only a recipe of the catalog. No tool opens a session, and no tool takes or returns a secret (MCP-004, MCP-006). [NEEDS CLARIFICATION: OD-BOT21, the narrow exception to MCP-006.]
4. **Client.** The "Agents" section of the device page and the review screen. A mockup of the screens is in [mockup/agent-install/index.html](../mockup/agent-install/index.html) (state Proposed, PLN-019).
5. **Daemon.** A new work type for the fixed list of USE-006 (OD-BOT16). The daemon asks for it. It listens on no port (USE-005).

ASCII wireframe of the device page, section "Agents", and the review screen:

```text
Device: mac-mini                                   [ Overview | Checks | Sessions | Agents ]
+--------------------------------------------------------------------------------------+
| Agents                                                         [ Add an agent v ]    |
|--------------------------------------------------------------------------------------|
| Agent        State                                   Version   Action                |
| Claude Code  installed, signed in (subscription)     2.x       [ Open ] [ Update ]   |
| Codex        installed, signed out                   0.x       [ Sign in ]           |
| (none)       Add one of the six agents from the list                                 |
+--------------------------------------------------------------------------------------+

Review: Install Codex on mac-mini
+--------------------------------------------------------------------------------------+
| What will run          the script of the vendor, from <address>, version <v>         |
| Checked with           SHA-256 <hash>   (or: the vendor gives none)                  |
| Where                  your home folder; no root; no port; does not start at login   |
| Vendor terms           Unclear for codex exec on a ChatGPT login. Checked 2026-11-16 |
|                        [ ] This login is mine                                        |
|                                                         [ Cancel ]  [ Install ]     |
+--------------------------------------------------------------------------------------+
Steps: [ok] download  [ok] check  [ .. ] run  [  ] probe  [  ] sign in  [  ] start
```

### Design

```text
operator --click--> client --mutation--> hub (request, approval, audit)
                                          | work type "agent_install" (the daemon asks)
                                          v
                              anima-daemon: fixed steps, as the user
                                          | steps and result (no secret)
                                          v
                          hub: state, audit chain -> device page, CLI, MCP read tools
sign-in:  operator --terminal (secret entry)--> vendor's own flow on the machine
```

- The hub makes a request. From the app or the command line, the operator approves it on the review screen. From MCP, the state of the tool decides (BOT-065). The daemon takes it when it asks for work, runs the steps of the recipe, and reports each step. The hub never sends a command line.
- The recipe is data in the catalog (BOT-060). It is the same catalog as BOT-040, with more fields. A recipe can be switched off by a new daemon release (BOT-074).
- The sign-in uses the terminal that exists (R-062) until panes are built. The sign-in pane has a flag so that the relay of the hub keeps no content (BOT-068). The flag is the only new function in the relay.
- Updates are detected from the probe that exists. An update or a removal starts by the rule of BOT-065, and never by Anima.ing on its own (BOT-071).

### Failure modes

| Failure | What happens |
| --- | --- |
| The vendor changes its install line or its checksum | The step fails at "check"; nothing is run; the page names the step. The recipe can be switched off in the next release (BOT-074). |
| No network, or a full disk | A preflight step stops the install before any change. |
| The step needs root | The install stops and says which step (BOT-063). |
| The operator closes the client during an install | The daemon finishes or stops at a step boundary and reports. The page shows the state when it opens again. |
| The machine goes off during an install | The request ends as "machine lost". Nothing is retried on its own. The operator can start again. |
| The sign-in is not finished | The state stays "installed, signed out". No timer retries it (BOT-068). |
| A vendor program changes itself (Claude Code native install and Cursor CLI do so by default) | The version change is logged as "changed by the vendor" (BOT-071). The recipe sets the vendor's switch against it when there is one. |
| `tmux` is missing and the panes are not built | The start step stops and says so (BOT-069). |
| An AI agent starts an install while no operator is present (the MCP default is `allow`, R-120; R-153, OD-BOT17) | The install runs. What limits it: only a recipe of the catalog runs (BOT-060, BOT-061); no root and no port (BOT-063, BOT-064); the machine must take work (USE-004, OD-BOT20); the audit log has the event with the name of the MCP client (BOT-073, MCP-026); the page shows "started by an AI agent" (BOT-065); the catalog can switch the recipe off (BOT-074); the operator can set the tool to `ask` or `off` (MCP-012). Open: OD-BOT21 and OD-BOT22. |
| An agent-started install and the plain-text sign-in (R-153, OD-BOT15) with no operator present | The install can finish with nobody at the machine. The sign-in cannot start by itself: it is a session, no MCP tool opens a session (MCP-006), and no MCP tool returns a code (MCP-004). The state stays "installed, signed out" until the operator opens the pane (BOT-068). The code passes through the hub only in the pane that the operator opens, with the notice, with no recording and no log (RA-008, LOG-007). If a later ruling lets a tool open that pane, the code would pass the hub with nobody to read it; the audit event and the notice are then the only limits. |
| An operator wants to type an API key for Gemini CLI, Pi or OpenCode in a pane | Not offered in the first version. The key lasts, and R-153 covers a one-time code only (BOT-078, OD-BOT23). The review screen says how to set the key on the machine. |
| The hub is hosted and the machine is behind a router | The daemon asks the hub for work, so no inbound port is needed (USE-005). |

### Test plan

- Pure tests in `daemon-core` for the validation of a recipe: no step takes free text, no step uses root, no recipe opens a port in the first version (BOT-061, BOT-063, BOT-064).
- A test that fails when a probe or a step opens a path of BOT-023 (the sandbox test that BOT-023 already asks for).
- A daemon test against a local web server that serves a script and a wrong checksum: the step stops (BOT-062).
- A hub test: a request needs an approval; an AI-agent request does not run (BOT-065, BOT-075); the audit events of BOT-073 are in the chain and none holds a secret (LOG-007).
- A test that the relay keeps no content of a "secret entry" session (BOT-068).
- A hub test of the MCP states: `allow` starts the install and the audit event names the MCP client; `ask` asks; `off` hides the tool; no tool opens the sign-in pane (BOT-065, BOT-068, BOT-075, MCP-026).
- For each of the six recipes: a validation test and a probe test with a fake program and an empty home (BOT-022, CAP-025).
- The coverage test of the command line (CMD-031) for each new root field.
- A check by a person on a new Mac and a new Linux machine for each of the six recipes, with the real steps and the time written down. An entry in `docs/need-human-testing.md` (PLN-014). The target of five minutes from the click to the first prompt, without the sign-in, is an estimate and is not a requirement.

### Scope review

| Mode | What it means here | Verdict |
| --- | --- | --- |
| Expand | Add apps (Orca, Paseo; T3 Code is a tool extension by R-196), OpenCode, auto-update, bulk setup, Windows | No. Each one adds a port, a secret or a terms conflict (BOT-021, BOT-032, BOT-046). |
| Selective | Take only what is new and not already in this document: the recipe runner, the review screen, the audit events, the secret-entry pane, update detection | Yes. This is the chosen mode. |
| Hold | Write only the requirements and build nothing | Yes for approval: nothing is built until the operator approves BOT-058 to BOT-078 (BI0) and rules OD-BOT21 to OD-BOT23. OD-BOT15 to OD-BOT20 are decided (R-153). |
| Reduce | Two harnesses (Claude Code and Codex), two platforms, manual update, no port | The Planner recommended this. The operator chose six agents instead (OD-BOT18, R-153). The mode is now Selective with six recipes, two platforms, manual update and no port (BOT-077). |

### Design scores

The scores are the judgement of the Planner on 2026-11-18, from 0 (bad) to 10 (good). They are not a measurement.

| Dimension | Score | Why, and what raises it |
| --- | --- | --- |
| Login safety (BOT-020, BOT-066) | 8 | No token is read or kept. Raise to 9 with the sandbox test in BI2 and BI8. |
| Sign-in step safety | 5 | A one-time code passes through the hub in plain text, with a notice (decided, R-153). A sealed channel would give 8. A key that lasts has no pane (BOT-078). |
| Fit with rulings | 5 | R-062, RA-008, USE-005, USE-006 and R-120 fit now that installs follow the MCP setting (R-153). MCP-006 (no command on a device) needs a narrow exception (OD-BOT21). BOT-045 still limits Gemini CLI. |
| Dependency risk | 4 | Needs the narrow work type (decided, R-153), BOT-022 to BOT-024 (not built), four new `ToolName` values and probes, and a `tmux` session (decided, R-153). |
| Testability | 5 | Pure parts and the hub are easy to test. Six vendors change their scripts, so each needs a hand check on two platforms (PLN-014). |
| Reversibility | 8 | Every step is the vendor's own and can be removed (BOT-072). The catalog switch turns a recipe off. |
| Effort realism | 5 | Seventeen tasks of half a day to three days. The size is an estimate, and about six to eight weeks for one person, without panes and without W1. Six recipes cost more than two. |
| Clarity of the requirements | 6 | Each requirement tests as pass or fail. Six "not verified" command sets and OD-BOT21 to OD-BOT23 still carry a marker. |

### Blast radius on shared contracts

| Shared item | Change | Who else uses it | Action |
| --- | --- | --- | --- |
| K1 `contracts/schema.graphql` | New query and mutations (Interfaces, step 1) | Command line (CMD-031), MCP, client, mock hub fixtures | A ruling (CON-001). One command for each root field. |
| K2 `contracts/database.md` | New tables for requests and steps | Hub domains, backups, retention | A ruling. A migration number in task BI3. |
| K3 `contracts/hub-domains.md` and `quality/hub-modules.json` | New domain, for example `fleet/agent_installs`, that may import `events` and `devices` | Import and folder-size gates | A ruling. Add the domain to `hub-modules.json`. |
| K8 `contracts/daemon-push.md` | A work type and the report of steps | The watchdog (WDG, G2), the update plan (U4, U6), the work route (W1) | Coordinate in one ruling for K8. Do not add a second route. |
| K4 `contracts/tunnel.md` Section 8 | A flag "secret entry" on a terminal session | Terminal client, session list | A ruling if the flag is a frame. Otherwise none. [NEEDS CLARIFICATION: how the flag reaches the relay.] |
| Audit log chain (LOG-021 to LOG-028) | New event kinds | `anima audit verify`, the redaction of `events/redaction` | Test that the new events pass redaction (LOG-007). |
| MCP rules (MCP-004, MCP-006, MCP-002, MCP-012, MCP-024 to MCP-026) | Install, update and remove are change tools that follow the MCP states (R-153). MCP-006 needs a narrow exception for a catalog recipe (OD-BOT21). | Every MCP client | A ruling for OD-BOT21 before task BI6. No other rule changes. |
| Daemon work rules (USE-004 to USE-006, JOB-011) | One more work type | Jobs, builds, runners, watchdog | OD-BOT16. The list must stay fixed. |
| Catalog (BOT-040) and `ToolName` of `daemon-core` | Install fields; more names | Hub ignores a name that it does not know (K8 Section 4) | No break. Hub and daemon can differ in version. |
| Design tokens (K5) | None. The mockup uses the tokens. | Mockup, client | None. |
| Vendor terms table (BOT-038) | A confirmation for each install | Configurator | Re-read each row before each release. |

### Tasks

One pull request for each task. The size of each task is a few hours to three days (an estimate). All tasks are Not started. The operator decided OD-BOT15 to OD-BOT20 in R-153, but did not approve the requirements, so none starts before BI0 (PLN-007, PLN-013). A task does not start while its "Needs" column names an open decision.

| Task | Wave | Content | Needs | State | Commits |
| --- | --- | --- | --- | --- | --- |
| BI0 | 0 | The operator approves BOT-058 to BOT-078 with a ruling, and rules OD-BOT21 to OD-BOT23. OD-BOT15 to OD-BOT20 are already decided (R-153). | The operator | Not started | — |
| BI1 | 1 | The contract text: K1, K2, K3 and K8 changes, and the entry in `hub-modules.json` (no code) | BI0; a ruling for CON-001 | Not started | — |
| BI2 | 1 | `daemon-core`: the types of a recipe and their pure validation. No entries and no execution. | BI0 | Not started | — |
| BI2a | 2 | The recipe of Claude Code: probe, entry, tests | BI2 | Not started | — |
| BI2b | 2 | The recipe of Codex: probe, entry, tests | BI2 | Not started | — |
| BI2c | 2 | The recipe of Cursor CLI: new `ToolName`, probe, entry, tests | BI2 | Not started | — |
| BI2d | 2 | The recipe of OpenCode: new `ToolName`, probe, entry, tests | BI2 | Not started | — |
| BI2e | 2 | The recipe of Gemini CLI: new `ToolName`, probe, entry, tests | BI2 | Not started | — |
| BI2f | 2 | The recipe of Pi: new `ToolName`, probe, entry, tests (Pi replaces Aider, R-196) | BI2 | Not started | — |
| BI3 | 2 | The hub: the domain for requests, approval, steps and audit events; the GraphQL | BI1 | Not started | — |
| BI4 | 3 | The daemon: take the work type and run the fixed steps of a recipe | BI1, BI2 | Not started | — |
| BI5 | 3 | `anima agents install`, `status`, `update`, `remove` | BI3 | Not started | — |
| BI6 | 4 | The MCP read tools and the change tools install, update and remove | BI5; OD-BOT21 | Not started | — |
| BI7 | 4 | The client: the Agents section, the review screen, the progress; the mockup is compared | BI3; the mockup | Not started | — |
| BI8 | 5 | The sign-in step and the "secret entry" session | BI4, BI7 | Not started | — |
| BI9 | 5 | The start of the session: panes, or the `tmux` fallback | BI4 | Not started | — |
| BI10 | 5 | The detection of "update available" and "changed by the vendor" | BI3, BI4 | Not started | — |
| BI11 | 6 | The security review, the entry in `need-human-testing.md`, and the hand test of the six recipes | BI8, BI9, BI2a to BI2f | Not started | — |

#### Task briefs

**Rules for every task.** Start with `AGENTS.md`, "Start of a session". Claim the task (PLN-022). Build on a branch in your own worktree. Use the order of CMD-030. The check commands are the ones named in the task and, at the end, `bun run quality:documents` and `bun run gates`. Done means a commit on `main` that changes the state of the task in this table (PLN-008). The prompt of each task starts the same way.

**BI1. The contract text.**
- Why: K1, K2, K3 and K8 are frozen. Only a ruling changes them (CON-001).
- Current state: no field, table, domain or work type exists for an install.
- Change: write the K1 fields, the K2 tables, the K3 domain rule and the K8 work type and step report. Add the domain to `quality/hub-modules.json`. Change no code.
- Acceptance: pass when each contract names every new item and the ruling number; fail when a new root field has no named command; `bun run quality:documents` passes.
- Check: `bun run quality:documents`.
- Out of scope: any Rust or TypeScript code.
- Always: cite BOT identifiers. Ask: before any change to an existing field. Never: add a field that holds a secret.
- Done: the commit that merges the contract text and sets BI1 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI1 of docs/proposals/bots.md (section 'Install an agent on a machine'). Write only the contract text for K1, K2, K3 and K8 and the hub-modules entry. Write no code. Run bun run quality:documents."

**BI2. Recipe types in `daemon-core`.**
- Why: a recipe must be data that a test can check before anything runs (BOT-060, BOT-061).
- Current state: `crates/daemon-core/src/inventory/tools.rs` has 13 `ToolName` values and probes with no install fields. Only Claude Code and Codex are agents there.
- Change: add the types for a recipe (steps as program plus arguments, checksum, source address, version, terms row key, the sign-in kind of BOT-067) and a pure function that rejects a recipe that breaks BOT-061, BOT-063, BOT-064 or BOT-078. Add no entry. The entries are tasks BI2a to BI2f.
- Acceptance: pass when a recipe with a step that takes free text, uses `sudo`, opens a port or offers a banned login fails the validation in a test, and a valid test recipe passes; fail otherwise. No step runs.
- Check: `cargo nextest run -p daemon-core`, `cargo clippy -p daemon-core --all-targets -- -D warnings`.
- Out of scope: any entry; running a step; the hub; the wire format.
- Always: pure functions with time and input as parameters. Ask: before a new crate dependency. Never: read a login file or spawn a program.
- Done: the commit that merges the types and tests and sets BI2 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI2 of docs/proposals/bots.md. Add recipe types and a pure validation to crates/daemon-core. Add no recipe entry. Run nothing that starts a program. Run cargo nextest run -p daemon-core and bun run gates."

**BI2a. The recipe of Claude Code.**
- Why: BOT-077 names Claude Code for the first version (operator, R-153, OD-BOT18).
- Current state: Claude and its probe exist in `tools.rs` (`ToolName::Claude`, empty home).
- Change: add the catalog entry (BOT-040, BOT-060) from the row of Claude Code in the table of recipes above. The recipe uses the install script of the table above. It runs only the unmodified `claude` with the operator's own login, and only on a machine that the operator owns (BOT-032, BOT-076). Set the vendor's switch against the self-update if the vendor page names one. Write the sign-in kind (Kind (a); BOT-067) and the terms row key (the Claude row (BOT-032)). Read each vendor page again, fill each "not verified" command, and write the source address and the date in the entry.
- Acceptance: pass when the entry passes the validation of BI2; a test with a fake `claude` script in a temporary folder shows that the probe finds it and prints its version with an empty home folder (BOT-022, CAP-025); no step uses root, a free-text argument or a login file (BOT-023, BOT-061, BOT-063); fail otherwise.
- Check: `cargo nextest run -p daemon-core`, `cargo clippy -p daemon-core --all-targets -- -D warnings`, `bun run quality:documents`.
- Out of scope: running the vendor script; the hub; the client; the hand test (BI11).
- Always: arguments as lists; a source address and a date for each command. Ask: before a command that the vendor page does not give. Never: a step that pipes a download into a shell; a step that reads or copies a login.
- Done: the commit that merges the entry and sets BI2a to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI2a of docs/proposals/bots.md. Add the catalog entry and the probe for Claude Code to crates/daemon-core, from the recipe table of the section 'Install an agent on a machine'. Do not run a vendor script. Run cargo nextest run -p daemon-core and bun run gates."

**BI2b. The recipe of Codex.**
- Why: BOT-077 names Codex for the first version (operator, R-153, OD-BOT18).
- Current state: Codex and its probe exist in `tools.rs` (`ToolName::Codex`, empty home).
- Change: add the catalog entry (BOT-040, BOT-060) from the row of Codex in the table of recipes above. The recipe uses the install script of the table above, and the same command for the update. Show the terms row of BOT-033 (a ChatGPT login is not allowed for a hosted service; `codex exec` is unclear, open decision 7). Write the sign-in kind (Kind (a); BOT-067) and the terms row key (the Codex row (BOT-033)). Read each vendor page again, fill each "not verified" command, and write the source address and the date in the entry.
- Acceptance: pass when the entry passes the validation of BI2; a test with a fake `codex` script in a temporary folder shows that the probe finds it and prints its version with an empty home folder (BOT-022, CAP-025); no step uses root, a free-text argument or a login file (BOT-023, BOT-061, BOT-063); fail otherwise.
- Check: `cargo nextest run -p daemon-core`, `cargo clippy -p daemon-core --all-targets -- -D warnings`, `bun run quality:documents`.
- Out of scope: running the vendor script; the hub; the client; the hand test (BI11).
- Always: arguments as lists; a source address and a date for each command. Ask: before a command that the vendor page does not give. Never: a step that pipes a download into a shell; a step that reads or copies a login.
- Done: the commit that merges the entry and sets BI2b to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI2b of docs/proposals/bots.md. Add the catalog entry and the probe for Codex to crates/daemon-core, from the recipe table of the section 'Install an agent on a machine'. Do not run a vendor script. Run cargo nextest run -p daemon-core and bun run gates."

**BI2c. The recipe of Cursor CLI.**
- Why: BOT-077 names Cursor CLI for the first version (operator, R-153, OD-BOT18).
- Current state: No Cursor `ToolName` or probe exists.
- Change: add the catalog entry (BOT-040, BOT-060) from the row of Cursor CLI in the table of recipes above. Add a `ToolName` and a probe for both program names, `agent` and `cursor-agent` (BOT-034), with an empty home. Use the install line of the table above. Read the page again for a switch against the self-update. Write the sign-in kind (Kind (a); BOT-067) and the terms row key (the Cursor row (BOT-034)). Read each vendor page again, fill each "not verified" command, and write the source address and the date in the entry.
- Acceptance: pass when the entry passes the validation of BI2; a test with a fake `agent` script in a temporary folder shows that the probe finds it and prints its version with an empty home folder (BOT-022, CAP-025); no step uses root, a free-text argument or a login file (BOT-023, BOT-061, BOT-063); fail otherwise.
- Check: `cargo nextest run -p daemon-core`, `cargo clippy -p daemon-core --all-targets -- -D warnings`, `bun run quality:documents`.
- Out of scope: running the vendor script; the hub; the client; the hand test (BI11).
- Always: arguments as lists; a source address and a date for each command. Ask: before a command that the vendor page does not give. Never: a step that pipes a download into a shell; a step that reads or copies a login.
- Done: the commit that merges the entry and sets BI2c to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI2c of docs/proposals/bots.md. Add the catalog entry and the probe for Cursor CLI to crates/daemon-core, from the recipe table of the section 'Install an agent on a machine'. Do not run a vendor script. Run cargo nextest run -p daemon-core and bun run gates."

**BI2d. The recipe of OpenCode.**
- Why: BOT-077 names OpenCode for the first version (operator, R-153, OD-BOT18).
- Current state: No OpenCode `ToolName` or probe exists.
- Change: add the catalog entry (BOT-040, BOT-060) from the row of OpenCode in the table of recipes above. Add a `ToolName` and a probe with an empty home. Use the v2 install line of the table above, and check that the install gives OpenCode 2 and not OpenCode 1. The recipe must not preselect a provider login that BOT-021, BOT-032 or BOT-033 bans (BOT-078). Write the sign-in kind (Kind (b); BOT-067) and the terms row key (the OpenCode row of BOT-043, or "terms not checked"). Read each vendor page again, fill each "not verified" command, and write the source address and the date in the entry.
- Acceptance: pass when the entry passes the validation of BI2; a test with a fake `opencode` script in a temporary folder shows that the probe finds it and prints its version with an empty home folder (BOT-022, CAP-025); no step uses root, a free-text argument or a login file (BOT-023, BOT-061, BOT-063); fail otherwise.
- Check: `cargo nextest run -p daemon-core`, `cargo clippy -p daemon-core --all-targets -- -D warnings`, `bun run quality:documents`.
- Out of scope: running the vendor script; the hub; the client; the hand test (BI11).
- Always: arguments as lists; a source address and a date for each command. Ask: before a command that the vendor page does not give. Never: a step that pipes a download into a shell; a step that reads or copies a login.
- Done: the commit that merges the entry and sets BI2d to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI2d of docs/proposals/bots.md. Add the catalog entry and the probe for OpenCode to crates/daemon-core, from the recipe table of the section 'Install an agent on a machine'. Do not run a vendor script. Run cargo nextest run -p daemon-core and bun run gates."

**BI2e. The recipe of Gemini CLI.**
- Why: BOT-077 names Gemini CLI for the first version (operator, R-153, OD-BOT18).
- Current state: No Gemini `ToolName` or probe exists.
- Change: add the catalog entry (BOT-040, BOT-060) from the row of Gemini CLI in the table of recipes above. Add a `ToolName` and a probe with an empty home. Use `npm install -g` or `brew install`, whichever runs with no root (BOT-063); if neither does, the recipe stops with a message. State the limit of the README: Code Assist Standard and Enterprise, or a paid API or Vertex key. No consumer Google login (BOT-036, BOT-045, BOT-078). Write the sign-in kind (Kind (b); no sign-in pane (OD-BOT23); BOT-067) and the terms row key (the Google row of BOT-036 and BOT-045). Read each vendor page again, fill each "not verified" command, and write the source address and the date in the entry.
- Acceptance: pass when the entry passes the validation of BI2; a test with a fake `gemini` script in a temporary folder shows that the probe finds it and prints its version with an empty home folder (BOT-022, CAP-025); no step uses root, a free-text argument or a login file (BOT-023, BOT-061, BOT-063); fail otherwise.
- Check: `cargo nextest run -p daemon-core`, `cargo clippy -p daemon-core --all-targets -- -D warnings`, `bun run quality:documents`.
- Out of scope: running the vendor script; the hub; the client; the hand test (BI11).
- Always: arguments as lists; a source address and a date for each command. Ask: before a command that the vendor page does not give. Never: a step that pipes a download into a shell; a step that reads or copies a login.
- Done: the commit that merges the entry and sets BI2e to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI2e of docs/proposals/bots.md. Add the catalog entry and the probe for Gemini CLI to crates/daemon-core, from the recipe table of the section 'Install an agent on a machine'. Do not run a vendor script. Run cargo nextest run -p daemon-core and bun run gates."

**BI2f. The recipe of Pi.**
- Why: BOT-077 names Pi for the first version in place of Aider (operator, R-196, OD-AXT16).
- Current state: No Pi `ToolName` or probe exists.
- Change: add the catalog entry (BOT-040, BOT-060) from the row of Pi in the table of recipes above. Add a `ToolName` and a probe with an empty home. Use the command of the table above that runs with no root. Pi is not in BOT-043 or BOT-044, so the terms row is "terms not checked" (BOT-041). Write the sign-in kind (Kind (b); no sign-in pane (OD-BOT23); BOT-067, BOT-078: never a subscription login of a vendor that is not Pi) and the terms row key ("terms not checked"). Read each vendor page again, fill each "not verified" command, and write the source address and the date in the entry. Do not add `pi-acp` until the [NEEDS CLARIFICATION] marker of its row is answered.
- Acceptance: pass when the entry passes the validation of BI2; a test with a fake `pi` script in a temporary folder shows that the probe finds it and prints its version with an empty home folder (BOT-022, CAP-025); no step uses root, a free-text argument or a login file (BOT-023, BOT-061, BOT-063); fail otherwise.
- Check: `cargo nextest run -p daemon-core`, `cargo clippy -p daemon-core --all-targets -- -D warnings`, `bun run quality:documents`.
- Out of scope: running the vendor script; `pi-acp`; the hub; the client; the hand test (BI11).
- Always: arguments as lists; a source address and a date for each command. Ask: before a command that the vendor page does not give. Never: a step that pipes a download into a shell; a step that reads or copies a login.
- Done: the commit that merges the entry and sets BI2f to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI2f of docs/proposals/bots.md. Add the catalog entry and the probe for Pi to crates/daemon-core, from the recipe table of the section 'Install an agent on a machine'. Do not run a vendor script. Run cargo nextest run -p daemon-core and bun run gates."

**BI3. The hub domain.**
- Why: the hub holds the request, the approval and the audit events (BOT-065, BOT-073). The hub API comes first (CMD-030).
- Current state: no domain. `fleet/agent_usage/` is the nearest example.
- Change: the domain `fleet/agent_installs` with `model.rs`, a pure state machine, `store.rs`, `graphql.rs` and tests. A request that the operator starts in the app or the command line needs his approval. A request that an AI agent starts follows the MCP state of the tool (BOT-065); it records the MCP client name and the actor `agent`. Write the events of BOT-073 to the audit chain.
- Acceptance: pass when an operator request without approval never reaches the daemon queue, an agent request with the tool `off` is refused and with `ask` waits, and no event holds a secret (a test with a seeded fake token); fail otherwise.
- Check: `cargo nextest run -p anima-hub agent_installs`, `cargo clippy -p anima-hub --all-targets -- -D warnings`, `bun run quality:imports`.
- Out of scope: the daemon, the command line, the client.
- Always: typed errors; SQL only in `store.rs`. Ask: before a change to an existing table. Never: log request arguments that came from an AI agent without the clean-up of MCP-013.
- Done: the commit that merges the domain and sets BI3 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI3 of docs/proposals/bots.md. Build the hub domain fleet/agent_installs from the contracts that BI1 merged. Build only the hub API and its tests. Run bun run gates."

**BI4. The daemon runs a recipe.**
- Why: the install is a fixed list of steps run as the operator's user (BOT-061 to BOT-064, BOT-070).
- Current state: the daemon sends reports only. It takes no work (USE-005).
- Change: take the work type; run the preflight, the download, the checksum check, the run and the probe; report each step; stop at the first failed step. Re-read the vendor pages and fix the commands in the catalog.
- Acceptance: pass when a wrong checksum stops the install before the run, a step that needs root stops, the daemon opens no port, and a probe opens no path of BOT-023; fail otherwise.
- Check: `cargo nextest run -p anima-daemon -p daemon-core`, `cargo clippy --all-targets -- -D warnings`.
- Out of scope: the sign-in; the session; update.
- Always: arguments as lists; download to a file first. Ask: before a new dependency. Never: pipe a download into a shell; use `sudo`.
- Done: the commit that merges the runner and sets BI4 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI4 of docs/proposals/bots.md. Add the work type and the step runner to anima-daemon, using the recipe types of BI2 and the contract of BI1. Do not run a vendor script in a test: use a local test server. Run bun run gates."

**BI5. The command line.**
- Why: the command comes after the hub API (CMD-030, CMD-031).
- Current state: `anima agents usage` exists in `crates/anima-cli/src/agents/`.
- Change: `install`, `status`, `update`, `remove`, with a GraphQL document for each new root field and an approval by `--yes` or a question at the terminal, as CMD-024 does for a delete.
- Acceptance: pass when the coverage test finds a command for every new root field and `install` shows the review text before it asks; fail otherwise.
- Check: `cargo nextest run -p anima-cli`, `cargo clippy -p anima-cli --all-targets -- -D warnings`.
- Out of scope: MCP; client.
- Always: follow the style of `anima agents usage`. Ask: before a new flag name. Never: print a token.
- Done: the commit that merges the commands and sets BI5 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI5 of docs/proposals/bots.md. Add the anima agents install, status, update and remove commands on the hub API of BI3, with tests and the CMD-031 coverage. Run bun run gates."

**BI6. The MCP tools.**
- Why: an AI agent can read the states and start an install, update or removal, by the MCP setting (BOT-075, R-153 OD-BOT17).
- Current state: `anima_agents_usage` is an MCP tool. Permission states `allow`, `ask`, `off` exist; the default of change tools is `off` until R-120 is built (task T11 of [cli-mcp-plan.md](../plans/cli-mcp-plan.md)).
- Change: read tools for the states and versions; change tools for install, update and remove, with `destructiveHint` on `remove`; each call that runs without a question is written to the audit log with the MCP client name (MCP-026). The tools open no session and return no secret.
- Acceptance: pass when `off` hides each tool, `ask` asks the person, `allow` starts the request and the audit event names the MCP client, no tool opens the sign-in pane, and no tool returns a one-time value; fail otherwise.
- Check: `cargo nextest run -p anima-cli mcp`.
- Out of scope: any change to MCP-002 or MCP-024; the sign-in pane.
- Always: `destructiveHint` set on `remove`. Ask: before a tool name. Never: start a recipe that is not in the catalog; build before OD-BOT21 is ruled.
- Done: the commit that merges the tools and sets BI6 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI6 of docs/proposals/bots.md. Check first that OD-BOT21 is ruled. Add the MCP read tools and the install, update and remove tools on the commands of BI5. Add a test that no tool opens a session or returns a secret. Run bun run gates."

**BI7. The client.**
- Why: the operator reads what will run before it runs (BOT-058, BOT-059).
- Current state: no Agents section on the device page. The mockup is `docs/mockup/agent-install/index.html`.
- Change: the Agents section, the review screen, the progress list and the error states, from the tokens; update the mockup in the same commit when the layout changes (PLN-019).
- Acceptance: pass when the screen shows each field of BOT-059 from a fixture, a failed step names the step, and the mockup row says "Compared" with the commit; fail otherwise.
- Check: `bun run --filter '*' test`, `bun run gates`.
- Out of scope: the sign-in pane (BI8).
- Always: tokens only; shared code in `$lib/<domain>/`. Ask: before a new route. Never: show an account name or an email address.
- Done: the commit that merges the screens and sets BI7 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI7 of docs/proposals/bots.md. Build the Agents section and the review screen of the device page from docs/mockup/agent-install/index.html and the hub API of BI3. Use a fixture for the mock hub. Run bun run gates."

**BI8. The sign-in step.**
- Why: the operator signs in on the machine and the hub keeps no content (BOT-066 to BOT-068).
- Current state: the SSH terminal relays bytes and keeps nothing (RA-008), with no flag.
- Change: for an agent of kind (a) (BOT-067), start the sign-in command of the vendor in a terminal session marked "secret entry" with the notice; the operator opens it. For kind (b), show how to set the key on the machine, and open no pane. Write only the start, the end and the method name to the audit log.
- Acceptance: pass when a test types a fake code and finds it in no log, no event and no database row; fail otherwise.
- Check: `cargo nextest run -p anima-hub sessions`, `bun run --filter '*' test`.
- Out of scope: a sealed channel; showing the code in the client.
- Always: show the notice of BOT-068 (R-153, OD-BOT15). Ask: before a change to K4. Never: read the output of a sign-in pane; open the pane for a key that lasts (BOT-078).
- Done: the commit that merges the step and sets BI8 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI8 of docs/proposals/bots.md. Add the secret-entry sign-in step. Test that a typed fake code is in no log, event or table. Run bun run gates."

**BI9. The session that stays alive.**
- Why: the agent must survive a closed client (BOT-069).
- Current state: panes are direction only. The operator decided to use `tmux` when it is already installed (R-153, OD-BOT19).
- Change: start the agent in a Herdr pane when Herdr is installed (AXT-026), or in a `tmux` session that the daemon names, in the folder of the operator; stop with a clear message when `tmux` is missing.
- Acceptance: pass when the session lives after the client disconnects in a test with a fake program; fail otherwise.
- Check: `cargo nextest run -p anima-daemon`.
- Out of scope: pane states and alerts (AIS-010 to AIS-012 are withdrawn by R-196; version 1 leaves agent state to Herdr).
- Always: the session name carries a prefix of Anima.ing. Ask: before installing `tmux`. Never: use root.
- Done: the commit that merges the start and sets BI9 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI9 of docs/proposals/bots.md. Start a chosen agent in a tmux session that the daemon owns, or in a pane when panes exist. Test with a fake program. Run bun run gates."

**BI10. Update detection.**
- Why: the operator must see "update available" and must not get a silent update (BOT-071).
- Current state: the probe gives a version once.
- Change: compare the probe with the catalog version; mark "update available" and "changed by the vendor"; the update runs only on request.
- Acceptance: pass when a changed version logs "changed by the vendor" once and no update starts without a request; fail otherwise.
- Check: `cargo nextest run -p anima-hub -p daemon-core`.
- Out of scope: update when idle.
- Always: one event for each change. Ask: before a polling rule. Never: start an update.
- Done: the commit that merges the detection and sets BI10 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI10 of docs/proposals/bots.md. Add update detection and the event 'changed by the vendor'. Do not start an update. Run bun run gates."

**BI11. The review and the check by a person.**
- Why: the daemon now runs a program that the vendor wrote, and the gates cannot prove a real install (PLN-014).
- Current state: no entry.
- Change: a security review of BI2a to BI2f and BI3 to BI10 (the findings go in `docs/security-findings.md`), with the questions of R-153: an agent-started install, the plain-text code, and the MCP exception; an entry HT-nnn with the steps for a new Mac and a new Linux machine, for each of the six recipes.
- Acceptance: pass when each finding has a state and the entry gives steps and the expected result of each step; fail otherwise. The task is Done after the operator sets the entry to Passed.
- Check: `bun run quality:documents`.
- Out of scope: code changes (they are new tasks).
- Always: write steps that the operator can do with no help. Ask: before a new identifier series. Never: mark the entry Passed.
- Done: the commit that adds the findings and the entry; the entry is Passed.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task BI11 of docs/proposals/bots.md. Review BI2a to BI10 for the BOT-066 to BOT-068 and BOT-078 rules, write the findings, and add the human-testing entry. Change no code."

### Open decisions

Each row has one question and a recommendation. OD-BOT15 to OD-BOT20 are decided by R-153. The other rows are proposed, not ruled. Only the operator can rule them.

| ID | Question | Recommendation | Result |
| --- | --- | --- | --- |
| OD-BOT15 | May a one-time sign-in code pass through the hub in plain text in the first version, with a disclosure, or must a sealed terminal come first? (allow with disclosure / sealed first) | Proposed, not ruled: Allow with disclosure. | Decided (R-153, operator, 2026-11-18, first-hand, Grok Bot chat): "OD-BOT15: yes, plain text through the hub with a notice." Yes, with a notice (BOT-068). |
| OD-BOT16 | Does the recipe runner wait for the work route (W1), or does it get its own narrow work type `agent_install` in the fixed list of USE-006 now? (wait / narrow work type) | Proposed, not ruled: A narrow work type that W1 can take over. | Decided (R-153, operator, 2026-11-18, first-hand, Grok Bot chat): "OD-BOT16: yes, add the narrow agent_install work type." The narrow work type. |
| OD-BOT17 | Do install, update and removal always need the operator's approval, also when the MCP server is fully automatic by default (R-120)? (always / follow R-120) | Proposed, not ruled: Always (MCP-006). This was the Planner's recommendation. | Decided (R-153, operator, 2026-11-18, first-hand, Grok Bot chat): "OD-BOT17: no, installs follow the MCP automatic setting." Against the recommendation. BOT-065 is amended. See OD-BOT21 and OD-BOT22. |
| OD-BOT18 | Is the first version limited to Claude Code and Codex? (yes/no) | Proposed, not ruled: Yes. This was the Planner's recommendation. | Decided (R-153, operator, 2026-11-18, first-hand, Grok Bot chat): "OD-BOT18: no, v1 should include more agents." He chose: Claude Code, Codex, Gemini CLI, Cursor CLI, Aider, OpenCode (BOT-077). |
| OD-BOT19 | Until panes (AIS-009) are built, may the daemon start the agent in a `tmux` session? (yes/no) | Proposed, not ruled: Yes, if `tmux` is already on the machine. No root install. | Decided (R-153, operator, 2026-11-18, first-hand, Grok Bot chat): "OD-BOT19: yes, use tmux when it is already installed." Yes. |
| OD-BOT20 | Does an install need the machine's "takes work" switch (USE-004) to be on, or a new switch? (takes work / new switch) | Proposed, not ruled: Takes work. | Decided (R-153, operator, 2026-11-18, first-hand, Grok Bot chat): "OD-BOT20: reuse the takes work switch (USE-004)." Takes work. |
| OD-BOT21 | Does MCP-006 (no tool runs a command on a device) get a narrow exception, so that an MCP tool can ask the daemon to run a recipe of the catalog (BOT-075)? (yes/no) | Proposed, not ruled: yes, for a catalog recipe only. No tool opens a session, so the sign-in pane stays with the operator (BOT-068). Needed before task BI6. | Open |
| OD-BOT22 | May an AI agent start a recipe before the operator confirmed its terms row on that machine (BOT-041, BOT-065)? (yes/no) | Proposed, not ruled: no. The operator confirms the row once in the app for each recipe and machine, and then an agent may start it. | Open |
| OD-BOT23 | May the operator type a key that lasts (Gemini CLI, Pi, OpenCode; Pi replaces Aider, R-196) in the secret-entry pane, when R-153 covers a one-time code only (BOT-078)? (yes/no) | Proposed, not ruled: no in the first version. He sets the key on the machine himself. | Open |

### Review notes

- 2026-11-18, version 4. The Planner moved the spec of the marketing bot into this document and replaced OCA-01 to OCA-12 with BOT-058 to BOT-078. The spec's app recipes and its auto-update are not in the first version.
- Two rules of the spec conflicted with rulings. Persistent session: it cannot wait for the panes (BOT-069). Installs by an AI agent: the Planner said never, because of MCP-006 and R-120; the operator ruled that installs follow the MCP setting (R-153), so MCP-006 needs a narrow exception (OD-BOT21).
- 2026-11-18, version 5. The operator decided OD-BOT15 to OD-BOT20 in R-153: plain text with a notice, the narrow work type, installs that follow the MCP setting (against the recommendation), six agents, `tmux`, and the takes-work switch. The Planner changed BOT-065, BOT-067, BOT-068, BOT-071, BOT-073, BOT-075 and BOT-077, added BOT-078, and split the recipe task into BI2 and BI2a to BI2f. The Planner also found two limits that the answers do not settle: MCP-006 and the terms rows (OD-BOT21, OD-BOT22), and the key that lasts (OD-BOT23). The state of this document stays Draft: no requirement is approved.
- The commands of the six recipes were read from the official pages on 2026-11-18. Each "not verified" command is marked in the table of recipes.
- Not verified: the vendor commands marked so, the sign-in flows of Claude Code and Codex on a machine with no browser, the five-minute target, the size of the work, and whether the daemon-managed `tmux` session is acceptable to the panes plan (open decision 1 of Section 4.12.1).
- Nothing in this section is a promise to a user. The claim rules of the product stay: a planned item is not described as shipping.
- 2026-11-18, a pointer only. [agent-tools-as-extensions.md](agent-tools-as-extensions.md) proposes to withdraw BOT-001 to BOT-018 and BOT-025 to BOT-028, to park BOT-047 to BOT-057, and to keep BOT-019 to BOT-024, BOT-029 to BOT-046 and BOT-058 to BOT-078 (OD-AXT3). It also proposes the Hermes and Herdr recipes, Hermes first in "Add an agent" (AXT-021 to AXT-024), and a loopback-only port for a tool (AXT-008, a change to BOT-064). The operator direction is relayed and not first-hand. No requirement of this document changes until a ruling.
- 2026-11-18, version 6. R-196 (operator, first-hand) decides OD-AXT1 to OD-AXT18 of [agent-tools-as-extensions.md](agent-tools-as-extensions.md). This document marks BOT-001 to BOT-018 and BOT-025 to BOT-028 Withdrawn and BOT-047 to BOT-057 Parked, keeps BOT-019 to BOT-024, BOT-029 to BOT-046 and BOT-058 to BOT-078, replaces Aider with Pi in BOT-077 and task BI2f (the commands of Pi are read from its README and are not verified on a machine; it needs the adapter `pi-acp` for ACP), adds the recipes of Hermes Agent, Herdr and T3 Code to BOT-077, adds the named exception for T3 Code before "The claim of T3 Code, checked", the exception for Hermes to BOT-065 and the loopback rule for tools to BOT-064, and changes BOT-069 (a Herdr pane or `tmux`). No requirement is approved.
- 2026-11-18, version 6.1. A follow-up pass after R-196: the requirements that stay (BOT-019, BOT-024, BOT-030, BOT-039, BOT-046, BOT-065), the overlap table, the "Depends on" line and task BI9 now say where they cite a withdrawn requirement or the withdrawn agent panes. BOT-039 is marked for re-planning: a Herdr pane or `tmux`, with a [NEEDS CLARIFICATION] marker for the state of the runner. No requirement is approved.
