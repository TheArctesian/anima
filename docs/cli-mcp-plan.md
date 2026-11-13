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
two more users of the same API, with no change to the hub:

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
  commands, its table output, and its MCP tools.
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
  field names of K1. Data goes to standard output; messages and errors go to standard error.
- **CMD-021.** Exit codes: 0 success, 1 failure, 2 wrong usage, 3 not found, 4 sign-in needed,
  5 not permitted, 6 conflict, 7 the hub or a dependency is not available.
- **CMD-022.** With `--json`, an error is `{"error":{"code","message","hint"}}` on standard error.
  `code` is the K7 code. `hint` is the next command to try.
- **CMD-023.** A command asks a question only when standard input is a terminal. `--no-input`
  never asks and fails with the flag that is missing.
- **CMD-024.** A command that deletes must ask for confirmation, or get `--yes` (DEV-014).
- **CMD-025.** A secret that a command takes (a device password, a private key) comes from a
  prompt, from standard input (`-`) or from `--from-file`. Never from an argument.
- **CMD-026.** Lists take `--limit` and `--after` (the K1 cursor) and `--all`.
- **CMD-027.** Colour is off when the output is not a terminal or `NO_COLOR` is set.
- **CMD-028.** `anima completion <shell>` prints shell completion.

## 6. Commands

"L" = needs the login token. All other commands also work with an API key (read commands with a
read key; change commands with a write key).

| Noun | Commands |
| --- | --- |
| `auth` | `login`, `logout`, `status` |
| `context` | `list`, `add`, `use`, `remove` |
| `device` | `list`, `get`, `create`, `update` L, `delete` L, `wake`, `test`, `metrics`, `uptime`, `watch` |
| `device credential` | `set ssh\|rdp\|vnc` L, `clear ssh\|rdp\|vnc` L |
| `device trust` | `accept-host-key` L, `accept-certificate` L |
| `site` | `list`, `create`, `set-wake-relay` |
| `tag` | `list` |
| `check` | `list`, `get`, `create`, `update`, `delete`, `pause`, `resume`, `heartbeats`, `defaults get\|set`, `notifications set`, `watch` |
| `maintenance` | `list`, `create`, `update`, `delete`, `start`, `stop` |
| `event` | `list`, `watch` |
| `audit` | `list` |
| `session` | `list`, `end`, `watch` |
| `ssh <device>` L | A terminal on the device through the hub (K4 Section 8). The credential stays on the hub. |
| `files` L | `ls`, `get`, `put` (K4 Section 9) |
| `web` | `list`, `create`, `update`, `delete`, `suggestion accept\|dismiss`, `accept-certificate` L |
| `agent` | `status`, `enrol` L, `revoke` L |
| `notify channel` | `list`, `create`, `update`, `delete`, `test` |
| `notify` | `settings get\|set`, `deliveries` |
| `retention` | `list`, `set` L |
| `release` | `list`, `next-version`, `create` L, `cancel` L, `log` |
| `builder` | `list`, `enrol` L, `revoke` L |
| `vpn` | `status`, `invite` L, `router-key` L, `register` L, `renew` L, `approve-route` L |
| `key` | `list` L, `create` L, `revoke` L |
| `api` | Send a GraphQL document from a file or standard input |
| `mcp` | Start the MCP server (Section 7) |
| `completion` | Shell completion |

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
- **MCP-002.** The default is read-only. `--allow-write` adds the tools that change things.
  `--toolsets` limits the tools to the nouns that the operator names.
- **MCP-003.** The hub token is the real limit. The server shows only the tools that its token
  can use. With an API key, the limits of INT-003 apply and the hub enforces them.
- **MCP-004.** No tool takes a secret and no tool returns one. Thus these are not tools: set a
  device credential, make an API key, enrol an agent or a builder, make a VPN key, make a
  notification channel (its address can hold a secret).
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

### Tools

| Set | Read (default) | Change (`--allow-write`) |
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
2. **Who did it.** The audit log records the operator and the token. It does not record that an
   agent, not a person, made the request. A hub change could add the client name.
3. **Login token for the MCP server.** The "L" tools work only when the operator starts
   `anima mcp` with a login token. That gives the agent more than INT-003 permits an API key.
   The plan keeps them behind `--allow-write`; the operator can decide to remove them.
4. **Distribution.** How `anima` gets to a computer (with the desktop application, Homebrew, the
   R2 release feed) is not planned.
5. **MCP protocol version.** The current revision is 2026-07-28. The Rust SDK (`rmcp`) can speak
   it and the earlier revisions. Which revision each coding agent uses was not verified.

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

Done when: `bun run gates` passes; each row of Section 6 works against a test hub; the coverage
test of CMD-031 passes; an MCP client lists the tools and reads the device list.
