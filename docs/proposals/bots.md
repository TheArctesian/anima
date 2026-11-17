---
kind: Proposal
state: Draft
evidence: Version 1 (2026-11-17). Direction of the operator of 2026-11-16; not approved. BOT-001 to BOT-038. It was Section 4.16 of [extensions-plan.md](../plans/extensions-plan.md) until 2026-11-17 (PLN-018). Its work package XP-16 is proposed and is not in the tables of Section 5 of that plan. No tier in Section 4 of the work plan yet.
---

# Proposal: Bots

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this proposal (PLN-005).

| Item | Value |
| --- | --- |
| Document | Proposal: the Bots extension (XP-16, proposed) |
| Version | 1 |
| Date | 2026-11-17 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | The header of this document holds the state (PLN-020). History: Draft. Direction of the operator of 2026-11-16. No requirement is approved. |
| Related | [extensions-plan.md](../plans/extensions-plan.md) Sections 4.12.1 (agent panes) and 4.15 (model gateway) |

Version 1 only moved this text from Section 4.16 of the extensions plan, because a proposal is not a part of a plan (PLN-016, PLN-018). The text did not change. A section number in this document (for example "Section 4.15") is a section of the extensions plan. When the operator approves this proposal, its requirements go back into the extensions plan as Section 4.16, with a ruling.

## Bots

**Purpose.** Always-on bots that work like employees, run on the operator's own computers, and use
the operator's own models. The operator defines a bot by chatting with it: its job, its schedule,
what it may touch. Each reply goes to a model that the router picks: the operator's subscription
(Claude Code, Codex) or any upstream of the model gateway (Section 4.15). A bot that needs to code
does so in an agent pane (Section 4.12.1). **Class:** B for the conversation text. C for the counts.
Status: direction only. The proposed work package is XP-16. It is not in the tables of Section 5.

**Data.** `bot` (name, job, instructions, device, model rule, schedule, permissions, state),
`bot_message` (bot, role, sealed text, model used, time), `bot_task` (bot, title, state, pane, result).

- **BOT-001.** The operator must be able to make a bot in a chat. The chat writes the bot definition:
  name, job, instructions, schedule, allowed devices and allowed tools. The operator must see the
  definition and confirm it before the bot starts.
- **BOT-002.** A bot must run on a device that the operator chooses, under the Anima daemon on that
  device. The hub keeps the definition and the history. The bot keeps running when no client is open.
- **BOT-003.** For each reply, the bot must choose a model by a rule: a fixed model, the cheapest
  model that can do the task, or local first. It must send the request through the model gateway
  (MGW-001, MGW-007). With no gateway upstream, it must use the operator's subscription through the
  vendor's own program on that device, for example `claude -p` or `codex exec`. It must never take
  or store the login of a subscription.
- **BOT-004.** A bot can start a coding task in an agent pane (AIS-013) in an allowed repository. Its
  state shows on the agent panes page with the other panes, and a task that waits for the operator
  sends the same alert (AIS-012).
- **BOT-005.** A bot can act on a schedule or an event: a time, a check that changes state, a new
  issue or pull request (Integrations, Section 4.5). It uses the rule engine (ENG-001).
- **BOT-006.** A bot must ask the operator before it sends a message to another person, spends money,
  deletes data or pushes to a shared branch. The operator can give a standing permission for one kind
  of action and can take it back.
- **BOT-007.** Each bot must have a budget (MGW-011) and a log of its actions. The operator must be
  able to pause or stop a bot in one action, from the client, the CLI (`anima bots`) and the phone.
- **BOT-008.** Bots count toward the overload warning (AIS-017), and their finished work goes into the
  daily digest (AIS-015).
- **BOT-009.** The Bots extension must be free on every plan, as agent panes are (AIS-016). Its
  conversation text is sealed like session text (AIS-004, AIS-005).
- **BOT-010.** Each bot can have its own computer: a virtual machine on one of the operator's devices,
  or a rented machine (CVM-001). The bot's shell, files and agent panes run there, not on the host
  device. The operator picks the size and the image when the bot is made (BOT-001).
- **BOT-011.** A bot's own computer joins the mesh as a device with a tag for that bot only. Access
  rules must not let one bot's computer reach another bot's computer, or the operator's devices,
  unless the operator allows it.
- **BOT-012.** The operator can make a bot read-only. A read-only bot can read checks, metrics, logs,
  files and repositories that it is allowed to see, and it can write to the operator in its chat. It
  cannot run change tools (MCP-002), write files, push, start agent panes, or act on a device. The
  hub must enforce this on its side, not only in the bot's instructions.
- **BOT-013.** A read-only bot must not be able to ask for more access itself. Only the operator can
  change a bot from read-only, and that change goes in the audit log.

The bot configurator (operator direction, 2026-11-16: the customer picks both the coding agent or
subscription, for example Claude Code or Codex, and the machine or machines that the bot runs on,
that is cloud machines, local machines such as a MacBook, or both):

- **BOT-014.** When the operator makes a bot (BOT-001), the client must show a configurator with
  two choices that the operator must make before the bot starts: the coding agent or subscription,
  and the machine or machines that the bot runs on. The configurator must write both choices into
  the bot definition (BOT-001).
- **BOT-015.** The list of coding agents must come from what is installed on the machines that the
  operator chose, for example `claude` and `codex`, and it must show a subscription only as the
  vendor's own program on that machine (BOT-003). The configurator must never read, ask for or
  store the login of a subscription.
- **BOT-016.** The list of machines must show the operator's own devices, for example a laptop,
  and the rented machines of the account (CVM-001), in one list. The operator can pick local
  machines, cloud machines or both. The hub must refuse a machine that the operator cannot use and
  a machine that another bot owns (BOT-011).
- **BOT-017.** Before the operator confirms, the configurator must show for each machine whether
  the chosen agent is installed on it, whether the bot is read-only (BOT-012), and, for a rented
  machine, its cost for each day if it stays on (CVM-004).
- **BOT-018.** A bot with more than one machine must have a rule that says which machine runs each
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
  login that this program holds. This extends the rule of BOT-003 (a fixed model, the cheapest model,
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
- **BOT-022.** The agent on each device must find the installed vendor programs without reading a login
  file. It must do these steps in order, for each program. First, look for the program in the PATH and in the
  folders of its installer by the name of the file. Second, run `<program> --version`. Third, ask the
  program for its own sign-in state with its status command: `claude auth status`,
  `codex login status` or `agent status`. Read only the exit code and the name of the sign-in method (for
  example `claude.ai` or `api_key` for Claude Code, the name that the program prints for the others). Discard all other output, including the name and the email of the
  account. For a program without a verified status command (Grok Build and Antigravity), report "installed,
  sign-in not checked". Do not guess. T3 Code does the same kind of check in `ProviderInstallationGate.ts`,
  `ClaudeProvider.ts` and `CodexProvider.ts`.
- **BOT-023.** The agent must not open, read, list the size of, or copy these files and stores, and must
  not read the keychain of the operating system for them: `~/.claude*`, `~/.codex/auth.json`,
  `~/.grok/auth.json`, the login store of the Cursor CLI (place not verified), and the matching items of
  the Keychain, the Secret Service and the Windows Credential Manager. A test must start a probe against a sandbox and fail when the
  probe opens one of these paths. Each probe must stop after 10 seconds and must run with no input.
- **BOT-024.** The device page must show, for each device, a list of agents with one state each: not
  installed, installed and signed out, installed and signed in (with the method: subscription, API key or
  other), or installed and not checked. The probe of BOT-022 must run when the agent starts, when
  the operator opens the page and when the operator makes a bot. The configurator (BOT-015, BOT-017)
  must use the same result. The state must not show the name or the email of the account.
- **BOT-025.** A conversation with a bot can change the agent or the model between two replies. The
  operator can do it at any time. A rule can do it, for example "my Claude subscription, then Codex when
  Claude has no quota". A change by a rule must show in the conversation and in the audit log (BOT-007).
  A rule must only move between agents of the same operator. A rule must not rotate between logins of the same
  vendor, and it must not pool logins.
- **BOT-026.** A change of agent must send a bounded handoff to the new agent. The handoff is a
  selection of the saved messages, not a summary that an agent writes. This costs no quota of a
  subscription. The bound is 16 000 tokens by default. The operator can set it from 1 024 to 64 000
  tokens. The hub must count one token for each UTF-8 byte, with the role and the agent name of each message
  included, as T3 Code does (`docs/user/portable-handoffs.md`). The handoff must also leave free at least
  16 000 tokens or one quarter of the context window of the target model, whichever is larger. When the
  window is not known, the hub must use 128 000 tokens. These numbers are the T3 Code defaults. Not verified:
  they fit each adapter.
- **BOT-027.** The handoff must contain the first request, then the newest messages that fit, in
  order, each with its role. It must say how many messages it left out and must name the place where the
  agent can read them. It must not contain the reasoning, the tool state or the attachments of the
  agent that the bot left. The new request of the operator must never be shortened. When the handoff
  does not fit even in a new session, the hub must say so and must not cut the new request.
- **BOT-028.** When a bot goes back to an agent that it used before, the adapter must resume the
  session of that agent when the vendor's program supports it (`claude -r`, `codex exec resume`). It must
  send only a handoff of the messages since that agent last ran. Otherwise it must start a new session with
  a full bounded handoff. This follows `docs/orchestration-v2/provider-switching-and-context.md` in T3 Code.
- **BOT-029.** A login of a vendor program must be used only for the operator who owns it. The hub must
  refuse to run an agent for a bot on a machine of another account. It must not let a bot, a customer
  or a member of the staff of Anima.ing use the subscription of another person. It must not share a login
  between two operators or between two machines of different owners.
- **BOT-030.** Anima.ing must not pay for, resell or include the use of a subscription in any plan, price
  or credit. BOT-009 stays true: the Bots extension is free on every plan. The charge for a rented machine
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
- **BOT-034.** Cursor: the adapter must run only `agent -p` of the Cursor CLI. It must treat the error
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

**Depends on.** Model gateway (XP-15), agent panes (Section 4.12.1), the rule engine (ENG-001), and
for long tasks the jobs and agent-runs proposals. Rented bot computers (BOT-010) depend on the
cloud machines proposal and are billed like any rented machine (CVM-009, CVM-010).

**Risks.**

- Subscription terms: driving a Claude or ChatGPT plan from our own code may break the vendor's
  terms. Calling the vendor's own program (BOT-003) is the safe path. The vendor terms table above
  (checked 2026-11-16) shows that this is allowed for Cursor and Grok Build, restricted for Claude Code
  and for Google, and unclear for Codex. BOT-020 to BOT-038 make the restrictions into requirements.
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
