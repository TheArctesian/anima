# Anima command line and MCP plan

Version 0.1 (draft, 2026-11-12). Operator instruction, 2026-11-12: "plan out a CLI first and then
a MCP such that coding agents like yourself can use it. The CLI should mirror all the app
functionality and so show the MCP". And: "all new features first get built into CLI then MCP then
the UI".

Identifiers: `CMD-nnn` for the command line, `MCP-nnn` for the MCP server. The prefix `CLI-` is
already used for the graphical client (design specification Section 3.1), so this plan does not
use it. The decisions in this plan need a ruling number in `programme.md`.

## 1. Purpose

The hub has one API (GraphQL, contract K1). The graphical client is one user of it. This plan adds
two more users of the same API, with one change to the hub: the actor kind of each audit log entry
(design specification Section 3.8.1, LOG-012 to LOG-020):

1. `anima`, a command line program for the operator and for scripts.
2. `anima mcp`, an MCP server in the same program, for coding agents.

## 2. What other tools do, and what Anima takes

| Tool | What Anima takes | What Anima does not take |
| --- | --- | --- |
| Tailscale CLI | `--json` on each read command. Secrets from a file or standard input, never as an argument. | An output format that can change. Anima's JSON is the K1 field names, which are frozen. |
| Coolify CLI | Noun then verb (`server list`). Named contexts for more than one instance. | A token in a plain configuration file. |
| GitHub CLI (`gh`) | `auth login`, `auth status`, a token from standard input, a token from an environment variable, exit code 4 for "sign in". The `api` command for a raw request. | |
| clig.dev | Data on standard output, messages on standard error. Ask only when a person is at a terminal. A flag for each question. `NO_COLOR`. | |
| GitHub MCP server | Read-only by default. Tool sets that the operator selects. | |
| Sentry MCP server | The MCP server is a command of the CLI and uses its sign-in. | |
| Kubernetes MCP server | The server calls the API directly. It does not start the CLI. | |
| Coolify MCP server | Sensitive fields are never in a tool result. | |
| Anthropic tool guidance | A small number of tools, one for each task of an agent, not one for each API call. Names, not only IDs, in results. Short results by default. An error says what to do next. | |

Sources: the research notes of 2026-11-12 (tailscale.com/kb/1080/cli, github.com/coollabsio/coolify-cli,
cli.github.com/manual, clig.dev, modelcontextprotocol.io/specification/2026-07-28,
anthropic.com/engineering/writing-tools-for-agents, github.com/github/github-mcp-server).

## 3. Structure

- **CMD-001.** One crate, `crates/anima-cli/` (the folder that the extensions plan reserves for
  `anima run`), one binary, `anima`.
- **CMD-002.** One folder for each operator noun, with the names of the hub domains: `devices/`,
  `checks/`, `sessions/`, and so on. Each folder holds the GraphQL documents of that noun, its
  commands and its table output. The MCP tools live in `src/mcp/` (`src/mcp/tools/<set>.rs`, one
  file or folder for each tool set) and call the documents of the noun folders.
- **CMD-003.** `hub/` is the only code that talks to the network: the GraphQL request, the
  subscription socket, the error codes. The commands and the MCP tools both call the same typed
  operations. The MCP server must not start the `anima` program.
- **CMD-004.** Each GraphQL document is a `.graphql` file that is checked against
  `contracts/schema.graphql` when the crate is built or tested. A document that does not agree
  with K1 must fail the gates.
- **CMD-005.** The crate must not change the hub, K1 or any other contract.

## 4. Sign-in and tokens

The hub has two tokens that a program can hold (design specification INT-003):

| Token | Gets | Use |
| --- | --- | --- |
| Login token (user name, password, authenticator code; 30 days) | All operations | The operator at a terminal |
| API key (`anima_…`, read or write, expiry at most one year) | Not: sessions, files, credentials, trust decisions, device change or delete, retention, builds, VPN keys, API keys | Scripts and coding agents |

- **CMD-010.** `anima auth login` asks for the user name, the password and the authenticator code
  at the terminal. It must not accept the password as an argument.
- **CMD-011.** `anima auth login --with-token` reads an API key from standard input.
- **CMD-012.** The token is kept in the operating system key store (service `ing.anima.cli`,
  account: the hub origin). On Linux with no key store it is a file with mode `0600`, as the agent
  does (MET-013).
- **CMD-013.** `ANIMA_TOKEN` in the environment is used before the stored token. `ANIMA_HUB` and
  `--hub` select the hub. The default is `https://hub.anima.ing` (DNS-001).
- **CMD-014.** No command prints a stored token. No token goes into a URL, an argument, a log or
  an error message (LOG-007, SEC-009).
- **CMD-015.** A context is a name for one hub origin. `anima context add|use|list|remove`. The
  configuration file (`$XDG_CONFIG_HOME/anima/config.json`) holds contexts and no secret.

## 5. Output and behaviour

- **CMD-020.** Each command prints a table for a person. With `--json` it prints JSON with the
  field names of K1: the bare value of the command's root field, with no envelope (a delete or
  revoke prints the bare ID string, an approval the bare boolean). Data goes to standard output;
  messages and errors go to standard error. A secret the hub shows once goes alone on standard
  output as one line (under its field with `--json`), with the "shown one time" note on standard
  error.
- **CMD-021.** Exit codes: 0 success, 1 failure, 2 wrong usage, 3 not found, 4 sign-in needed,
  5 not permitted, 6 conflict, 7 the hub or a dependency is not available.
- **CMD-022.** With `--json`, an error is `{"error":{"code","message","hint"}}` on standard error.
  `code` is a K7 code, or one of the local codes the hub has no code for: `USAGE` (wrong usage,
  exit 2), `FAILED` (a local failure such as an unreadable file, a failed write, a notification
  that did not arrive, or paging that stopped, exit 1) and `DECLINED` (the operator answered no
  to a confirmation, exit 1). `hint` is a command to run (the text shows it as ``run `...` ``) or
  a sentence (shown as it is). A usage error from the command line parser names the missing
  flags and points at the command's `--help`. Every string from the hub is cleaned of control
  characters before it reaches a terminal.
- **CMD-023.** A command asks a question only when standard input is a terminal. `--no-input`
  never asks and fails with the flag that is missing.
- **CMD-024.** A command that deletes must ask for confirmation, or get `--yes` (DEV-014). Every
  confirmation ends the same way: an answer other than `y` or `yes` is `DECLINED`, exit 1 with a
  message on standard error (and the JSON error under `--json`), so a script can tell that the
  delete did not run. Nothing is sent to the hub after a decline.
- **CMD-025.** A secret that a command takes (a device password, a private key) comes from a
  prompt, from standard input (`-`) or from `--from-file`. Never from an argument.
- **CMD-026.** Lists take `--limit` and `--after` (the K1 cursor) and `--all`. `--all` stops
  with a failure when the hub repeats a cursor, and after 100,000 rows. The next-page hint keeps
  the filters of the command that printed it.
- **CMD-027.** Colour is off when the output is not a terminal or `NO_COLOR` is set.
- **CMD-028.** `anima completion <shell>` prints shell completion.

## 6. Commands

"L" = needs the login token. All other commands also work with an API key (read commands with a
read key; change commands with a write key).

| Noun | Commands |
| --- | --- |
| `auth` | `login`, `logout`, `status` |
| `context` | `list`, `add`, `use`, `remove` |
| `device` | `list`, `get`, `create`, `update` L, `delete` L, `wake`, `test`, `metrics` (`--watch`), `uptime`, `watch` |
| `device credential` | `set ssh\|rdp\|vnc` L, `clear ssh\|rdp\|vnc` L |
| `device trust` | `accept-host-key` L, `accept-certificate` L |
| `site` | `list`, `create`, `set-wake-relay` |
| `tag` | `list` |
| `check` | `list`, `get`, `create`, `update`, `delete`, `pause`, `resume`, `heartbeats`, `defaults get\|set`, `notifications set`, `watch` |
| `maintenance` | `list`, `create`, `update`, `delete`, `start`, `stop` |
| `event` | `list`, `watch` |
| `audit` | `list` (`--actor user\|agent\|hub`, LOG-020) |
| `session` | `list`, `end`, `watch` |
| `ssh <device>` L | A terminal on the device through the hub (K4 Section 8). The credential stays on the hub. |
| `files` L | `ls`, `get`, `put` (K4 Section 9) |
| `web` | `list`, `create`, `update`, `delete`, `suggestion accept\|dismiss`, `accept-certificate` L |
| `agent` | `status`, `enrol` L, `revoke` L |
| `notify channel` | `list`, `create`, `update`, `delete`, `test` |
| `notify` | `settings get\|set`, `deliveries` |
| `retention` | `list`, `set` L |
| `release` | `list`, `next-version`, `create` L, `cancel` L, `log` (`--follow`) |
| `builder` | `list`, `enrol` L, `revoke` L |
| `vpn` | `status` (`--watch`), `invite` L, `router-key` L, `register` L, `renew` L, `approve-route` L |
| `key` | `list` L, `create` L (`--for user\|agent`, required, LOG-015), `revoke` L |
| `api` | Send a GraphQL document from a file or standard input |
| `mcp` | Start the MCP server (Section 7) |
| `completion` | Shell completion |

`watch`, `--watch` and `--follow` keep a subscription open: one line for each change, or one JSON
object per line with `--json`. `release log --follow` ends when the job is published, failed or
cancelled; the others end when you stop them.

Not in the command line, with the reason:

- RDP and VNC displays, and web interface tabs: they draw pixels. `anima session list|end` and
  `anima device wake` still apply.
- The appearance settings, the updater and the builder pipeline of the desktop application: they
  are functions of that application, not of the hub.
- `anima run` (SCR-006): it needs the Secrets extension (#15). The command name is reserved.
- The operator password and the authenticator setup: they stay on the hub host
  (`anima-hub operator`).

## 7. MCP server

- **MCP-001.** `anima mcp` is an MCP server on standard input and output. It uses the same
  context and token as the command line. A remote HTTP server is not planned: it needs OAuth and
  a hub change.
- **MCP-002.** Each tool has one state: `allow` (listed; the agent may call it), `ask` (listed;
  each call needs the person's approval, MCP-013) or `off` (not listed, and refused if called,
  without a hub request). With nothing saved, every read tool is `allow` and every change tool is
  `off`: the server is read-only.
- **MCP-003.** The hub token is the real limit. The server shows only the tools that its token
  can use. With an API key, the limits of INT-003 apply and the hub enforces them.
- **MCP-004.** No tool takes a secret and no tool returns one. Thus these are not tools: set a
  device credential, make an API key, enrol an agent or a builder, make a VPN key, make a
  notification channel (its address can hold a secret). `anima_check_update` does not replace
  the config of any HTTP check or SNMP check: the check read does not show an HTTP request body,
  so such a check may hold a secret that a new config would drop.
- **MCP-005.** No tool makes a trust decision: accept a host key or a certificate. A person does
  that, in the client or with the command line.
- **MCP-006.** No tool opens a session or runs a command on a device. (Open point 1.)
- **MCP-007.** A tool that deletes takes `confirm`, which must be the name of the thing.
- **MCP-008.** Text that comes from a device or a check (names, log lines, messages) is data. Each
  result gives it in named fields with a length limit, and the server instructions tell the agent
  not to obey it.
- **MCP-009.** Each tool has a description, an input schema, an output schema, and the hints
  `readOnlyHint`, `destructiveHint` and `idempotentHint`. Each list takes `limit` and `after`.
  The default result is short; `detail: true` gives all fields.
- **MCP-010.** A hub error is a tool result with `isError` and a sentence that says what to do.
- **MCP-011.** Tool names are `anima_<noun>_<verb>`.
- **MCP-012.** The states are saved in the configuration file, by hub origin, under `mcp`: a
  default for each kind (`read`, `change`) and the operator's explicit choices by tool name
  (`tools`). A tool with no choice takes its kind's default, so a tool added later takes the
  default of its kind. The file holds no secret. The form before MCP-012 (`toolsets`, `write`)
  is read once as explicit choices and is not written again.
- **MCP-013.** When the agent calls a tool in state `ask`, the server asks the person at the
  MCP client to approve that one call. It sends an MCP elicitation request in form mode with
  one yes-or-no field (`approve`). The message names the tool, says how many argument values
  there are and shows each of them on its own line: nested keys as dotted paths, the identifying
  arguments (`device`, `check`, `site`, `id`, `name`, `confirm`) first, each value cleaned and cut
  at 120 characters with its full length. A key that the tool does not take is shown by name
  only. More than 40 values, or a message over 4,000 characters, is refused before the person is
  asked, with a sentence to send a smaller change. The call runs only after a clear yes. If the
  client did not declare elicitation, or the person declines or cancels, or there is no answer
  in 120 seconds, or the request fails, the call is refused with `isError`. Every refusal of a
  tool that is off, unknown or not approved tells the agent to ask the operator and not to change
  Anima's settings itself; it holds no command line. The server instructions say the same. The
  hub is not asked. A delete in state `ask` still needs its `confirm` (MCP-007).
- **MCP-014.** `anima mcp permissions show` prints one row for each tool: tool set, tool, kind
  (`read` or `change`), needs sign-in (`yes` or `no`) and state. `--json` gives
  `{ hub, summary, tools: [{ toolset, tool, kind, needsSignIn, state, description }] }`; the
  Settings panel reads this form. `permissions set` takes `--allow`, `--ask` and `--off` with
  comma lists of tool or tool set names; the bulk forms `--allow-all`, `--ask-all`, `--off-all`,
  narrowed by `--toolset <set>` and `--kind read|change`; and `--read-only` (every change tool
  `off`). Bulk forms apply first, then `--allow`, `--ask`, `--off`. An unknown name exits 2 and
  lists the valid names. `--allow-all` that covers change tools needs `--yes` or a yes at the
  terminal. `permissions reset` deletes the saved states of the hub. A `set` or `reset` that
  widens access (a tool or a kind's default moves toward `allow`) needs a terminal on standard
  input; without one it exits 2, whatever `--yes` says, and names the terminal and the desktop
  app's Settings. A change toward `off` works anywhere.
- **MCP-015.** The flags of `anima mcp` apply to one run and only narrow: `--toolsets a,b` turns
  off every tool outside those sets; `--read-only` turns off every change tool. The one
  exception is `--allow-write [sets]`: it allows the change tools (of every set, or of the sets
  named) that have no saved choice and no saved `change` default. A saved `off` or `ask` is
  never weakened by a flag. The token comes last: an API key never gets a tool that needs a
  login token, whatever is saved (MCP-003).
- **MCP-016.** The server instructions, the line on standard error at start, and
  `permissions show` state the current picture in plain words: the tool sets that are on, the
  change tools that are allowed, the tools that ask, and the tools that are off.
- **MCP-017.** Each tool has at least one integration test that calls it over JSON-RPC against
  the scripted hub (`tests/it/mcp/`).

### Settings panel

The desktop application has an "MCP" section on the Settings page. It sets the same states as
`anima mcp permissions` (CMD-030: the command line comes first; this is the graphical client).

- **MCP-018.** The section reads and saves the states through two shell commands that call the
  library of `anima-cli`: the same functions and the same configuration file as
  `permissions show`, `set` and `reset` (MCP-014), with the same atomic replace of the file. The
  shell does not start the `anima` program, does not parse its output, and does not read a token.
  The hub address is checked with the command line's own origin parser. An error that the
  shell returns is a code with fixed text; it holds no path and no file content (LOG-007).
  At the top the section says in plain words what the MCP server is, that the states apply to
  coding agents on this computer for this hub, that the hub token still limits everything, and
  that `anima mcp` starts it. It also says that Allow lets an agent use a tool without asking
  and that text an agent reads can steer it, what Ask does (the agent's app shows each call; if
  it cannot ask, the call is refused), and what MCP stands for.
- **MCP-019.** The section shows one group for each tool set, in the order of Section 7, closed
  by default. The group's name carries the number of tools in each state, in words ("3 allow,
  1 ask, 2 off"). A group opens to one row for each tool: its name in plain words, the tool ID
  in a secondary style, a "Read" or "Change" mark, a "Needs sign-in" mark with a hint where
  `needsSignIn` is true (the tool works only when `anima mcp` runs with the operator's own
  sign-in, never an API key; MCP-003), its description on request, and a choice of Allow, Ask
  or Off. A group header also says how many tools differ from the default ("2 changed from
  default"), when any do. A group with a tool that needs sign-in says in view what that means.
  A change tool that is on Allow shows a mark, in words, that it can change the hub without
  asking.
- **MCP-020.** Bulk actions are "Allow all", "Ask for all", "Read-only (turn off all changes)"
  (every change tool `off`) and "Reset" (MCP-014: `permissions reset`) for all tools. Each group has
  "Allow all", "Ask for all" and "Turn off changes" for its own set. "Allow all" asks for a
  confirmation first, because it turns on change tools without a question for each call, as
  `--allow-all` does (MCP-014). The question names its scope in the title ("Allow every tool on
  this hub?" or "Allow every Checks tool?"), says that the read tools of that scope are turned
  on too, and lists in at most three lines what an agent steered by text it reads could then do
  without asking: with an API key that can write, and with the operator's own sign-in. Only the
  consequences of the tools in the scope are listed; they come from data keyed by tool set,
  taken from "What a compromised agent can do" below. A single tool set to Allow from its row
  needs no question. Every other action is saved at once. A group has no "Reset":
  it would forget the states of every set.
- **MCP-021.** The choice for one tool is a radio group named after the tool. The arrow keys only
  move the focus and do not wrap from the last option to the first; Space or Enter chooses, so a
  key press never passes through Allow. A change shows at once and is saved after the changes
  before it, one at a time. If the shell refuses it, the control goes back to its earlier state
  and the section says: "Not saved", the tool or action that failed, the reason in fixed words,
  and that the control went back. The message stays until the operator dismisses it
  or the same control is saved. From the first paint the section holds a placeholder of ten closed
  groups, as tall as the list, so that loading, a failed read and the loaded list take the same
  height and nothing below moves (SET-004, MOT-007); there is no delay before it shows. If the file
  cannot be read, the section shows an error without a path and a "Try again" button in that
  place.
- **MCP-022.** Every control can be reached with the keyboard and shows a focus ring. The state
  of a tool is a word, never a colour alone. After a change the section shows one quiet line
  ("Saved. Devices: every tool set to ask (7 tools).") in a polite live region, also for a bulk
  action; a failure is in an alert region. There is one message for a bulk action, not one for
  each tool.
- **MCP-023.** The section is shown in the desktop application only. The states are a file on
  this computer, which a browser cannot read, and the hub does not hold them: they belong to the
  computer that runs `anima mcp`, not to the account.

### Tools

Read tools are `allow` and change tools are `off` by default (MCP-002). L: needs a login token.

| Set | Read | Change |
| --- | --- | --- |
| `devices` | `anima_fleet_overview`, `anima_devices_search`, `anima_device_get`, `anima_device_metrics` | `anima_device_create`, `anima_site_create`, `anima_device_wake`, `anima_device_update` L, `anima_device_delete` L |
| `checks` | `anima_check_get` (with heartbeats and uptime) | `anima_check_create`, `anima_check_update`, `anima_check_set_paused`, `anima_check_delete` |
| `maintenance` | `anima_maintenance_list` | `anima_maintenance_create`, `anima_maintenance_update`, `anima_maintenance_delete`, `anima_maintenance_set_active` |
| `events` | `anima_events_search`, `anima_audit_list` | |
| `sessions` | `anima_sessions_list` | `anima_session_end` |
| `notifications` | `anima_notifications_overview` (channels with no addresses, settings, deliveries) | `anima_notification_send_test`, `anima_check_set_notifications` |
| `web` | (in `anima_device_get`) | `anima_web_interface_create`, `anima_web_interface_update`, `anima_web_interface_delete` |
| `releases` | `anima_releases_list`, `anima_build_log` | `anima_release_create` L, `anima_build_cancel` L |
| `vpn` | `anima_vpn_status` | `anima_vpn_approve_route` L |
| `retention` | `anima_retention_list` | `anima_retention_set` L |

## 8. The rule for new functions

- **CMD-030.** A new function is built in this order: the hub API, the command line, the MCP
  tool, then the graphical client. A function is not complete until each of the three has it or
  Section 6 or 7 gives the reason that it does not.
- **CMD-031.** A test must compare the root fields of K1 with the fields that the command line
  uses. A K1 field that no command uses, and that is not in a list of exceptions with a reason,
  must fail the gates.

## 9. Open points

1. **A session for an agent.** `anima ssh` gives the operator a terminal on any device. The same
   as an MCP tool would let a coding agent run a command on each device, with the stored
   credential. This plan does not build it (MCP-006). It needs an operator decision and a security
   review.
2. **Who did it.** Decided (operator, 2026-11-13) and built: each audit log entry has an actor
   kind, User, Agent or Hub (design specification Section 3.8.1, LOG-012 to LOG-020). `anima mcp`
   sends the agent declaration with each request (LOG-016). `anima key create` takes the
   "made for" attribute with `--for` (LOG-015). `anima audit list` has the Actor, Token and Key
   columns and `--actor`, and `anima_audit_list` has the fields and an `actor` filter (LOG-020).
   The contract change still needs a programme ruling number (design specification OD-15).
3. **Login token for the MCP server.** The "L" tools work only when the operator starts
   `anima mcp` with a login token. That gives the agent more than INT-003 permits an API key.
   They are `off` until the operator sets them to `allow` or `ask` (MCP-002); the operator can
   decide to remove them.
4. **Distribution.** Decided (operator, 2026-11-13), not built: `anima` is inside the disk image
   and the AppImage of the desktop application, at the same version, and the application puts it
   on the `PATH` (design specification Section 3.11, INS-001 to INS-009). There is no package
   manager channel (INS-008). Three mechanisms are not verified (design specification OD-16,
   OD-17, OD-18).
5. **MCP protocol version.** The current revision is 2026-07-28. The Rust SDK (`rmcp`) can speak
   it and the earlier revisions. Which revision each coding agent uses was not verified.
6. **SNMP secrets after a host change.** The hub keeps a check's SNMP community and v3 passwords
   when the device's host changes. An agent with `anima_device_update` can set the host to an
   address it chooses, and the next SNMP check sends the community there. The hub must ask for
   the SNMP secrets again when the host changes, as it does for an HTTP check URL. This needs a
   hub change and an issue.
7. **`ask` trusts the MCP client.** The server cannot see who answers the approval request.
   Whatever answers it (the person, the client's own rule, or a client that the agent controls)
   decides.
8. **Who writes the start.** The flags of `anima mcp` (`--allow-write`, `--toolsets`) and the
   configuration location (`XDG_CONFIG_HOME`, `--context`) come from whoever writes the MCP
   client's configuration. That includes a project `.mcp.json` in a repository.
9. **The audit log cannot tell who acted.** Decided (operator, 2026-11-13) and built: the actor
   kind tells an agent from the operator (LOG-012, LOG-014), and the token kind with the key name
   tells an API key from a login token (LOG-018). See point 2. One limit stays: a program that
   holds the operator's login token and sends no declaration is recorded as User.
10. **Accepted risk: the operator's own account.** `anima mcp permissions set` and `reset` widen
    access only with a terminal on standard input (MCP-014). A program that runs commands as the
    operator can still edit the configuration file by hand, start the server with
    `--allow-write`, or run the command inside a pseudo-terminal. The check stops an agent from
    widening its access by accident or because a refusal told it how; it does not stop an agent
    that sets out to do it. The hub token stays the real limit (MCP-003).
11. **SET-004 lists the sections.** Closed (operator, 2026-11-13): `docs/design-spec.md` SET-004
    now names the "MCP" section (MCP-018..023). The design specification also records that the
    section list and the jump menu of SET-004 are not built.

### What a compromised agent can do

An agent can be steered by text it reads. What it can do then depends on the token and the
tools that are on:

1. **Read API key.** Read the whole fleet, events, the audit log, sessions with client
   addresses, VPN nodes and build logs.
2. **Write API key, every change tool allowed.** Pause every check and empty its notification
   channels, start maintenance windows, delete checks, maintenance windows and web interfaces,
   end every live session, point web interfaces at other addresses, point HTTP checks at
   internal URLs, send wake packets and test notifications.
3. **Login token, everything allowed.** All of the above, and also delete every device with its
   stored credentials, approve a VPN route, cancel builds, set every retention period to the
   minimum and start a release.

## 10. Work

| Task | Content | Builder | Tag |
| --- | --- | --- | --- |
| T1 | Crate, `hub/`, contexts, sign-in, token store, output, exit codes, `api`, `device list\|get`, the K1 coverage test | Opus | security |
| T2 | `device` (the rest), `site`, `tag`, `web`, `agent` | Sonnet | security (credentials) |
| T3 | `check`, `maintenance` | Sonnet | |
| T4 | `event`, `audit`, `notify`, `retention`, `key`, and the `watch` commands (subscriptions) | Sonnet | |
| T5 | `release`, `builder`, `vpn` | Sonnet | security (keys shown once) |
| T6 | `session`, `ssh`, `files` | Opus | security |
| T7 | `anima mcp`: server, tool sets, read tools, change tools | Opus | security |
| T8 | Review (CUPID, tests), security review, gates | Sonnet, Opus | |
| T9 | Per-tool states `allow`, `ask`, `off`; `permissions show\|set\|reset`; `ask` by elicitation; the `sessions` tool set (MCP-012..017) | Opus | security |
| T10 | The MCP permissions panel in the desktop Settings page, on the library behind `permissions show\|set\|reset` (MCP-018..023) | Sonnet | security (file write) |

Done when: `bun run gates` passes; each row of Section 6 works against a test hub; the coverage
test of CMD-031 passes; an MCP client lists the tools and reads the device list.
