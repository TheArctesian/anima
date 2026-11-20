---
kind: Plan
state: In progress
evidence: The command line and the MCP server are on `main`. The test against a real hub is open ([GH-60](../work-items/device-manager/GH-60.md)). R-120 (operator, 2026-11-16): the MCP server is fully automatic by default (MCP-024 to MCP-026); ruled and not built; tasks T11 to T13 build it (R-156), all Not started. Section 11 (2026-11-18) is the direction for `anima tui`, a read-only terminal screen. R-168 (operator, 2026-11-18, first-hand) decided OD-TUI1 to OD-TUI5 and OD-TUI8 and names task TU1. No TUI requirement is approved and no TU task is built.
---

# Anima command line and MCP plan

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version 0.4 (2026-11-19): Section 12 adds `anima mcp install`, which puts the entry `anima` into the
configuration of each coding agent on this computer, and its control in the Settings section "MCP"
(MCP-027 to MCP-040, tasks MI1 to MI6). The operator asked for it (R-257).

Version 0.3 (2026-11-18): Section 11 adds `anima tui`, a read-only full-screen view of the
hub in a terminal (TUI-001 to TUI-083, tasks TU1 to TU10). The operator decided the direction in
R-168 (first-hand, 2026-11-18). The requirements are not approved. The idea folder
`docs/ideas/terminal-ui/` is deleted by the commit that adds the section (PLN-018).

Version 0.2 (2026-11-16): the MCP server is fully automatic by default (R-120, MCP-024 to
MCP-026). The change is ruled and is not built: the code still starts with every change tool `off`.

Version 0.1 (draft, 2026-11-12). Operator instruction, 2026-11-12: "plan out a CLI first and then
a MCP such that coding agents like yourself can use it. The CLI should mirror all the app
functionality and so show the MCP". And: "all new features first get built into CLI then MCP then
the UI".

Identifiers: `CMD-nnn` for the command line, `MCP-nnn` for the MCP server, `TUI-nnn` for the terminal
screen (Section 11). The prefix `CLI-` is
already used for the graphical client (design specification Section 3.1), so this plan does not
use it. The decisions in this plan need a ruling number in [rulings.md](../rulings.md).

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
  delete did not run. Nothing is sent to the hub after a decline. What a question is about
  (a merge summary, the heads of a bulk merge) is shown on standard error before it is asked,
  with `--json` too: the operator never answers a question about something they were not shown.
- **CMD-025.** A secret that a command takes (a device password, a private key) comes from a
  prompt, from standard input (`-`) or from `--from-file`. Never from an argument.
- **CMD-026.** Lists take `--limit` and `--after` (the K1 cursor) and `--all`. `--all` stops
  with a failure when the hub repeats a cursor, and after 100,000 rows. The next-page hint keeps
  the filters and the `--limit` of the command that printed it.
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
| `audit` | `list` (`--actor user\|agent\|hub`, LOG-020), `verify` (the hash chain, LOG-027, R-138) |
| `session` | `list`, `end`, `watch` |
| `ssh <device>` L | A terminal on the device through the hub (K4 Section 8). The credential stays on the hub. |
| `files` L | `ls`, `get`, `put` (K4 Section 9) |
| `web` | `list`, `create`, `update`, `delete`, `suggestion accept\|dismiss`, `accept-certificate` L |
| `agent` | `status`, `enrol` L, `revoke` L |
| `capability` | `list` (`--tool <name>`, `--target <build target>`): the tool inventory of each device (compute plan CAP-020 to CAP-031) |
| `agents` | `usage` (`--device <id\|name>`, `--source claude-code\|codex`, `--repository <host/owner/name>`, `--since <day>`, `--until <day>`, `--days <1-366>`, `--by device,source,model,repository,day`, repeatable; device and source by default): the sessions, prompts, tool calls and tokens of Claude Code and Codex with an estimate of the cost, which is not the bill of a subscription, and the AIS-024 sentence for a daemon that runs as a system unit (R-135; AIS-020, AIS-021, AIS-024) |
| `snapshot` | `push` (`--remote`), `list`, `delete <ref>`: a commit of the working tree on a scratch ref of the forge (JOB-017, JOB-018). It uses git on this computer and makes no hub request. |
| `notify channel` | `list`, `create`, `update`, `delete`, `test` |
| `notify` | `settings get\|set`, `deliveries` |
| `retention` | `list`, `set` L |
| `release` | `list`, `next-version`, `publishing`, `create` L, `cancel` L, `log` (`--follow`) |
| `builder` | `list`, `enrol` L, `revoke` L |
| `vpn` | `status` (`--watch`), `invite` L, `router-key` L, `register` L, `renew` L, `approve-route` L |
| `timeline` | `list` (`--from`, `--to`, `--category`, `--limit`, `--after`, `--all`; the last 24 hours when no time is given) (K-E3; TML-007) |
| `record-link` | `list <record>`, `add <source> <target> --kind <kind>` (`--label`), `remove <id>`; a record is `kind:id` (K-E4; RLK-001, RLK-002, RLK-004). The noun `link` is the device link of S6 |
| `file` | `list <record>`, `upload <path> --owner <record>` (`--name`, `--key`, default `engine`, `--archive <id>`, `--keep-location`, `--type <media type>`), `download <id> --output <path>` (a new file only; never to a terminal), `remove <id>` (confirms), `archive add --owner <record> --name <name>`, `archive remove <id>` (confirms: it deletes the records that came from the archive) (K-E4 Section 9; FIL-001 to FIL-005). The bytes go through `/files/v1`, not through GraphQL |
| `extraction` | `list` (`--owner`, `--state proposed\|confirmed\|rejected`), `propose --owner --form --extension --origin client\|hub --fields <JSON>` (`--attachment <id>`), `confirm <id> --fields <JSON>`, `reject <id>` (K-E4 Section 10; VIS-001 to VIS-004) |
| `extension` | `list`, `on <key>` L, `off <key>` L (`on` prints "also sets X on" and asks unless `--yes`; `off` shows the hub's CONFLICT when another extension needs it) |
| `source` | `kinds`, `list` (`--source github`), `get <id\|label>` (runs with counts), `connect github` L (`--label`, `--organisation <login>`, `--no-browser`, `--state`, `--cancel`: without `--from-file` the hub makes the App and the operator presses Create and Install on GitHub; on a terminal it asks for the one-time link, prints it and opens it, then prints one line for each phase; off a terminal it never asks for the link and prints the hub page to open in a signed-in browser. `--from-file <path\|->` selects the manual route and requires `--label`, `--app-id` and `--installation-id`: the key comes from a 0600 file or standard input, never an argument, CMD-025. `--app-id` or `--installation-id` without `--from-file`, or `--state` with `--cancel`, is `USAGE`, exit 2. All of it is login-only; exit 6 is `CONFLICT` at start, 7 is `GITHUB_UNREACHABLE`. (operator, 2026-11-14; ruling number to be assigned); PRJ-039, PRJ-040), `sync`, `pause`, `resume`, `remove` L (`--keep-records` or `--delete-records`, one is required; it confirms, CMD-024), `watch [<id\|label>]` |
| `project` | `list` (`--left-out`, `--limit`, `--after`, `--all`), `get <slug>`, `leave-out <owner/name\|id>`, `restore <owner/name\|id>`, `resource confirm <slug> <kind>` |
| `pr` | `list <repo>` (`--state open\|closed\|all`, `--checks`; `pullRequests`; table: number, title, author, branches, draft, can merge, checks), `get <repo> <n>` (`--comments`; `pullRequest`, `issueComments`), `merge <repo> <n>...` L (`--method merge\|squash\|rebase`, required; `--sha`, required with one number whenever no question is asked: with `--yes`, `--no-input` or off a terminal; `--allow-failing-checks` and `--fork <owner/name>`, one number only; `--yes`. One number: `mergePullRequest`; 2 to 20: `mergePullRequests`, never one from a fork. It reads each pull request fresh. For one number it shows title, head, fork, merge state, checks and method, and sends the head it showed or the `--sha` given, which must equal the head it read. For several it shows each number, head (seven digits) and title, asks once and sends the heads it read; with `--yes` no one is asked, so those heads were read a moment before and confirmed by no one (the hub refuses any head that moved after the read). What it shows goes to standard error before the question, with `--json` too; each field from GitHub stays on its own line. One line for each result; exit 1 when any of several is not `MERGED` or `ALREADY_MERGED`) |
| `issue` | `list <repo>` (`--state`; `issues`), `get <repo> <n>` (`--comments`; `issue`, `issueComments`), `create <repo>` (`--title`, `--body-file <path\|->`, `--label`; `createIssue`), `comment <repo> <n>` (`--body-file <path\|->`; `addIssueComment`), `close <repo> <n>` (`--reason completed\|not-planned`) and `reopen <repo> <n>` (`setIssueOpen`). No confirmation; the body is never printed back |
| `workflow` | `list <repo>` (`workflows`), `start <repo> <workflow id\|path>` L (`--branch`, `--input name=value` repeated, at most 25, `--yes`; `dispatchWorkflow`; the question names the inputs, never their values, and no output or refusal shows a value), `runs <repo>` (`--branch`, `--status`, `--jobs`; `workflowRuns`; table with the commit), `run <repo> <run>` (`workflowRun`, with its jobs), `rerun <repo> <run>` L (`--failed`, `--yes`; `rerunWorkflowRun`, or `rerunFailedWorkflowJobs` with `--failed`), `cancel <repo> <run>` L (`--yes`; `cancelWorkflowRun`) |
| `code` | `ls <repo> [path]` (`--ref`; `codeDirectory`; table: kind, size, name), `show <repo> <path>` L (`--ref`; `codeFile`: the text on standard output, cleaned of control characters when it is a terminal; `BINARY` or `TOO_LARGE` is `FAILED`, exit 1, with the GitHub link) |
| `repo` | `commits <repo>` (`--ref`, `--checks`; `commits`), `branches <repo>` (`branches`), `releases <repo>` (`repositoryReleases`), `status set <repo> <sha>` L (`--state pending\|success\|failure\|error`, `--context anima/...`, `--description`, `--url`; `setCommitStatus`; no confirmation: the hub fences the context to `anima/`) |
| `key` | `list` L, `create` L (`--for user\|agent`, required, LOG-015), `revoke` L |
| `api` | Send a GraphQL document from a file or standard input |
| `mcp` | Start the MCP server (Section 7) |
| `completion` | Shell completion |

The `pr`, `issue`, `workflow`, `code` and `repo` commands (operator, 2026-11-14; ruling number to be
assigned; PRJ-027 to PRJ-035, OD-E14) read GitHub live through the hub (K-E1 Section 13). `<repo>` is
`owner/name` or the repository ID (`ProjectRepository.id`). Lists take `--limit`, `--after`, `--all`
(CMD-026) and `--fresh`. Exit codes are CMD-021's: 3 not found, 5 not permitted, 6 conflict,
7 unavailable, 1 for a refused merge or a file it cannot show. `anima run` is reserved for Secrets
(SCR-006) and `release` is Anima's own, so workflow runs live under `workflow` and GitHub releases
under `repo`. `rerun` and `cancel` read the run first, show its commit in the prompt and send that
commit as `expectedHeadSha`; `merge` on a terminal reads the pull request and does the same, and whenever no question is
asked (`--yes`, `--no-input`, or off a terminal) `--sha` is required for one number, so `--yes` never
stands for the head (PRJ-029, PRJ-034). A pull request from a fork merges only alone, with
`--fork` equal to its head repository or that name typed at the terminal; `--yes` never stands for
it (PRO-013).

`watch`, `--watch` and `--follow` keep a subscription open: one line for each change, or one JSON
object per line with `--json`. `release log --follow` ends when the job is published, failed or
cancelled; the others end when you stop them.

Not in the command line, with the reason:

- RDP and VNC displays, and web interface tabs: they draw pixels. `anima session list|end` and
  `anima device wake` still apply.
- The appearance settings, the updater and the builder pipeline of the desktop application: they
  are functions of that application, not of the hub.
- `anima run` (SCR-006) and `anima secrets` (SCR-013): they need the Secrets extension ([XP-04](../work-items/extensions/XP-04.md)). The
  command names are reserved. `anima secrets read` prints a value, so it has no MCP tool; the
  Secrets tools return secret handles only (SCR-015, MCP-004).
- `anima gateway` (MGW-015, MGW-016): it needs the Model gateway extension (extensions plan,
  Section 4.15). The command name is reserved.
- `anima job` (`start`, `status`, `log`, `stop`, `list`) and `anima artifact` (`list`,
  `fetch`): proposed, not approved and not built. They need the jobs proposal (JOB-012 to
  JOB-027). The command names are reserved.
- The operator password and the authenticator setup: they stay on the hub host
  (`anima-hub operator`).

## 7. MCP server

- **MCP-001.** `anima mcp` is an MCP server on standard input and output. It uses the same
  context and token as the command line. A remote HTTP server is not planned: it needs OAuth and
  a hub change.
- **MCP-002.** Each tool has one state: `allow` (listed; the agent may call it), `ask` (listed;
  each call needs the person's approval, MCP-013) or `off` (not listed, and refused if called,
  without a hub request). With nothing saved, every read tool and every change tool is `allow`:
  the server is fully automatic (operator ruling R-120, 2026-11-16; MCP-024). A tool that starts
  routed work follows these states and routes with no question when it is `allow` (R-243, ROU-009). The switch "Ask
  before destructive actions" (MCP-025) puts the destructive tools in state `ask`. Before R-120, a
  change tool was `off` with nothing saved.
- **MCP-003.** The hub token is the real limit. The server shows only the tools that its token
  can use. With an API key, the limits of INT-003 apply and the hub enforces them.
- **MCP-004.** No tool takes a secret and no tool returns one. Thus these are not tools: set a
  device credential, make an API key, enrol an agent or a builder, make a VPN key, make a
  notification channel (its address can hold a secret). `anima_check_update` does not replace
  the config of any HTTP check or SNMP check: the check read does not show an HTTP request body,
  so such a check may hold a secret that a new config would drop.
- **MCP-005.** No tool makes a trust decision: accept a host key or a certificate. A person does
  that, in the client or with the command line.
- **MCP-006.** No tool opens a session or runs a command on a device. (Open point 1.) Exception
  (R-243, "Multiplexer Q2: automatic for all work."): a tool may start work of a fixed type, or an
  agent run, on the machine that routing picks (ROU-001 to ROU-014 of
  [jobs.md](../proposals/jobs.md)). R-244 (OD-ROU1) adds routed sessions and command lines
  (ROU-015, ROU-016). Not approved; no effect before the security review RT7.
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
  `off`). Bulk forms apply first, then `--allow`, `--ask`, `--off`. `--ask-destructive` and
  `--no-ask-destructive` turn on and off the switch of MCP-025; `show` prints its state. An unknown
  name exits 2 and lists the valid names. `--allow-all` that covers change tools needs `--yes` or a
  yes at the terminal. `permissions reset` deletes the saved states of the hub and turns the
  switch off. A `set` or `reset` that widens access from a saved choice (a saved `ask` or `off`
  of a tool or of a kind's default moves toward `allow`, or the switch moves from on to off) needs
  a terminal on standard input; without one it exits 2, whatever `--yes` says, and names the
  terminal and the desktop app's Settings. A change toward `off` or `ask`, and turning the switch
  on, works anywhere. With R-120 the default is already `allow`, so nothing widens from the
  default.
- **MCP-015.** The flags of `anima mcp` apply to one run and only narrow: `--toolsets a,b` turns
  off every tool outside those sets; `--read-only` turns off every change tool. The one
  exception is `--allow-write [sets]`: it allows the change tools (of every set, or of the sets
  named) that have no saved choice and no saved `change` default. With R-120 those tools are
  already `allow`, so the flag changes nothing unless a saved default or choice says `ask` or
  `off`; it stays for old configuration files and old start lines. A saved `off` or `ask` is
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
  or the same control is saved. From the first paint the section holds a placeholder of
  thirteen closed groups, one for each tool set, as tall as the list, so that loading, a failed read and the loaded list take the same
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

### Fully automatic by default (operator ruling R-120, 2026-11-16)

Ruled by the operator on 2026-11-16, and not built. It replaces the earlier defaults of MCP-002 and
of Settings "Reset", and the line under "Tools" below. The operator's words: "have destructive be
gated by an are you sure that can be ignored in the MCP settings, off by default (essentially fully
auto by default)".

- **MCP-024.** With nothing saved, every read tool and every change tool is `allow`. The server is
  fully automatic: an agent can change and delete without a question. A new tool takes this default
  too (MCP-012).
- **MCP-025.** Settings and `anima mcp permissions set` have one switch, "Ask before destructive
  actions". It is off for a new user. When it is on, every tool with `destructiveHint` (MCP-009) is
  in state `ask` (MCP-013) unless the operator chose a state for that tool. The operator's choices
  for single tools still win, and `off` still hides a tool.
- **MCP-026.** Every destructive call that runs without a question must be in the hub's audit log
  with the tool, its arguments (cleaned, as in MCP-013) and the MCP client name, so the operator
  can see what an agent deleted.
- Unchanged: MCP-004 (no secrets), MCP-005 (no trust decisions), MCP-006 (no command on a
  device), MCP-007 (a delete still names the thing in `confirm`) and MCP-008 (device text is
  data). The command line keeps CMD-024: a delete asks at the terminal unless it gets `--yes`.
- **How MCP-014 applies.** The default is already wide, so the rule that widening needs a terminal
  applies only to a change away from a saved `ask` or `off`, and to turning the switch of MCP-025
  off after it was on. The main risk is an agent that obeys text from a device or a web page
  (MCP-008) and then deletes without a question. The audit log (MCP-026) and the switch (MCP-025)
  are the answer to that risk.

### Tools

Every tool is `allow` by default (MCP-002, MCP-024, R-120). The switch of MCP-025 puts the destructive tools in state `ask`. L: needs a login token.

| Set | Read | Change |
| --- | --- | --- |
| `devices` | `anima_fleet_overview`, `anima_devices_search`, `anima_device_get`, `anima_device_metrics`, `anima_capability_list`, `anima_agents_usage`, `anima_container_list`, `anima_container_storage`, `anima_container_networks`, `anima_container_updates` | `anima_device_create`, `anima_site_create`, `anima_device_wake`, `anima_device_update` L, `anima_device_delete` L |
| `checks` | `anima_check_get` (with heartbeats and uptime) | `anima_check_create`, `anima_check_update`, `anima_check_set_paused`, `anima_check_delete` |
| `maintenance` | `anima_maintenance_list` | `anima_maintenance_create`, `anima_maintenance_update`, `anima_maintenance_delete`, `anima_maintenance_set_active` |
| `events` | `anima_events_search`, `anima_audit_list`, `anima_audit_verify` (R-138) | |
| `sessions` | `anima_sessions_list` | `anima_session_end` |
| `notifications` | `anima_notifications_overview` (channels with no addresses, settings, deliveries) | `anima_notification_send_test`, `anima_check_set_notifications` |
| `web` | (in `anima_device_get`) | `anima_web_interface_create`, `anima_web_interface_update`, `anima_web_interface_delete` |
| `releases` | `anima_releases_list`, `anima_release_publishing` (PRO-037), `anima_build_log` | `anima_release_create` L, `anima_build_cancel` L |
| `vpn` | `anima_vpn_status` | `anima_vpn_approve_route` L |
| `retention` | `anima_retention_list` | `anima_retention_set` L |
| `link` (FRN-009) | `anima_link_devices` | None: `createLinkInvite` returns a secret (MCP-004) and `revokeLinkDevice` is a trust decision (MCP-005). The command line has both (`anima link invite`, `anima link revoke`) |
| `extensions` | `anima_extensions_list` | `anima_extension_set` L |
| `timeline` | `anima_timeline_list` | |
| `records` | `anima_record_links_list`, `anima_files_list`, `anima_extractions_list`, `anima_locations_list`, `anima_location_context`, `anima_reminders_list`, `anima_reminder_get`, `anima_reminder_firings` | `anima_location_import`, `anima_reminder_add`, `anima_reminder_set`, `anima_reminder_remove`, `anima_reminder_set_enabled`, `anima_reminder_test`, `anima_reminder_snooze`, `anima_reminder_ack`. The three `anima location token` commands have no tool: an ingest token is a secret (K-E4 Section 13). `anima source connect dawarich` has no tool: it takes the API key of the server (R-261). `anima_record_link_add`, `anima_record_link_remove`, `anima_file_remove`, `anima_file_archive_add`, `anima_file_archive_remove`, `anima_extraction_propose`, `anima_extraction_confirm`, `anima_extraction_reject`. `anima file upload` and `anima file download` have no tool: a tool does not carry the bytes of a file (K-E4 Section 13) |
| `sources` | `anima_sources_overview` (declarations, accounts and their last runs; covers `Query.sources` and `Query.sourceAccounts`) | `anima_source_account_sync`, `anima_source_account_set_paused`, `anima_source_account_remove` L (`confirm` must equal the account label, MCP-007), `anima_github_connect_start` L, `anima_github_connect_cancel` L, and the read tool `anima_github_connect_state` (any token; phase, failure, message, `appSlug`, `ownerLogin`, `installUrl`, `appSettingsUrl`, and the account id, label and health). The start tool returns `{ attempt: { id, phase, expiresAt }, link }` where `link` is the fixed `https://<hub>/connect/github`: no state, no ticket, no attempt id. Alone it does nothing until a signed-in operator presses Continue. The start tool's description tells the agent to give the link to the operator and not to open it. `appSlug`, `ownerLogin` and the label go through `mcp/untrusted.rs` (operator, 2026-11-14; ruling number to be assigned) |
| `projects` | `anima_projects_search`, `anima_project_get` | `anima_repository_set_left_out` (one tool, `leftOut: bool`), `anima_project_resource_confirm` |
| `repositories` (operator, 2026-11-14; ruling number to be assigned; PRJ-027 to PRJ-035, OD-E14) | `anima_pull_requests_list` (`pullRequests`; `detail: true` adds mergeability and checks), `anima_pull_request_get` (`pullRequest`, `issueComments`), `anima_issues_list` (`issues`), `anima_issue_get` (`issue`, `issueComments`), `anima_workflows_list` (`workflows`), `anima_workflow_runs_list` (`workflowRuns`), `anima_workflow_run_get` (`workflowRun`, with jobs), `anima_commits_list` (`commits`), `anima_branches_list` (`branches`), `anima_repository_releases_list` (`repositoryReleases`), `anima_code_directory_list` (`codeDirectory`; names only) | `anima_issue_create` (`createIssue`), `anima_issue_comment_add` (`addIssueComment`), `anima_issue_set_open` (`setIssueOpen`). A write key may call them; they return the issue or the comment without its body. The other GitHub changes have no tool (below) |

Fields with no MCP tool, with the reason (MCP-004): `Mutation.connectGithubApp` takes a secret (`GithubAppInput.privateKey`); `Subscription.sourceAccountChanged` is a subscription, and no subscription has a tool; `Mutation.openGithubConnect` returns a ONE-TIME link that opens the connection in a browser, and a tool may not return one (the hub also refuses it, FORBIDDEN, for a request that declares an agent); `Subscription.githubConnectChanged` is a subscription (operator, 2026-11-14; ruling number to be assigned); `Query.codeFile` returns the content of a file (operator, 2026-11-14; ruling number to be assigned). `Mutation.mergePullRequest` and `Mutation.mergePullRequests`: a merge is a trust decision (MCP-005), and the hub refuses every API key for them; `Mutation.rerunWorkflowRun`, `Mutation.rerunFailedWorkflowJobs` and `Mutation.dispatchWorkflow` run code with the repository's secrets, `Mutation.cancelWorkflowRun` stops a run, and `Mutation.setCommitStatus` can satisfy a required check and open a merge; the hub refuses every API key for each (operator, 2026-11-14; ruling number to be assigned). The command line has all seven, behind a confirmation. Text from GitHub (names, descriptions, paths) goes through `mcp/untrusted.rs` as named, length-limited fields (MCP-008).

**No MCP tool returns file content** (operator, 2026-11-14; ruling number to be assigned). The content of a file in a repository may hold a secret, and the hub cannot tell (MCP-004, as for `anima_check_update`); it is also the strongest carrier of instructions to an agent in the Projects slice. MCP-002 makes every read tool `allow` by default, so the content gets no tool rather than a read tool. An agent that needs the code has the checkout. `anima_code_directory_list` returns names only. Other text from GitHub in the `repositories` results (titles, bodies, comments, logins, branch names, labels, commit messages, check and job names) is given in named fields, cleaned and cut by `mcp/untrusted.rs`, where the key `body` is cut at 4,000 characters. Each tool description says that this text is data, and the server instructions gain one line for text from GitHub.

`anima_container_list` (read, tool set `devices`) gives the Docker containers of one device
(HOST-001, R-149). No tool returns the log of a container: a log can hold a secret (MCP-004), so
`containerLog` is closed to every API key and only `anima containers logs` reads it.
`anima_container_storage` and `anima_container_networks` (read, tool set `devices`) give the images,
the volumes and the networks of one device (HOST-020, R-150); `anima_container_list` also gives the
one state of each Compose project (HOST-019). `anima_container_updates` (read, tool set `devices`)
says for each container whether its public registry has a newer image (HOST-021); it pulls and
updates nothing. No tool returns the details of a container
(`containerDetail`, HOST-017): the command, the labels and the health-check command can hold a
secret as the environment can (MCP-004), so the root is closed to every API key and only
`anima containers inspect` reads it.
`anima_capability_list` (read, tool set `devices`) gives the tool inventory of each device
(CAP-030, CAP-031). `anima_agents_usage` (read, tool set `devices`) gives the use of the AI agent programs on each
device with the estimate of its cost and each daemon's state (R-135; AIS-020, AIS-021, AIS-024); it takes
and returns no secret, and the model, repository and device names are named fields capped by
`mcp/untrusted.rs` (MCP-008). `anima snapshot` has no tool: it changes no hub state and acts on a folder of
the computer that runs `anima mcp`, and an agent with a shell can run the command.

Reserved tools, proposed and not built (jobs proposal JOB-012 to JOB-027):
the read tools `anima_job_status`, `anima_job_log`, `anima_jobs_list`
and `anima_artifacts_list`, and the change tools `anima_job_start`, `anima_job_stop` and
`anima_artifact_fetch`. MCP-006 stays: no tool takes a command line (JOB-015). When these tools
exist, the server instructions must say when to give work to a different machine: this computer
has no free memory, or a tool or a build target is absent. They must tell the agent to read
`anima_capability_list` first.

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
   With R-120 they are `allow` by default like every tool (MCP-002, MCP-024), and the operator
   can set them to `ask` or `off`, or decide to remove them.
4. **Distribution.** Decided (operator, 2026-11-13) and built (branch `fix/bundle-anima`):
   `anima` is inside the disk image and the AppImage of the desktop application, at the same
   version, and Settings > Updates > Command line tool puts it on the `PATH` (design
   specification Section 3.11, INS-001 to INS-009). There is no package manager channel
   for the desktop application, the daemon or the hub (INS-008). The command line alone also has a
   Homebrew formula and a crates.io crate (operator, 2026-11-18, R-151; INS-010 to INS-013; tasks R8a to R8d of
   [projects-and-builds-plan.md](projects-and-builds-plan.md); not built). The mechanisms are decided by the implementation and need a real release to
   confirm the signature, the notarization and the AppImage launcher (design specification
   OD-16, OD-17, OD-18).
5. **MCP protocol version.** The current revision is 2026-07-28. The Rust SDK (`rmcp`) can speak
   it and the earlier revisions. Which revision each coding agent uses was not verified.
6. **SNMP secrets after a host change.** Fixed (design specification SEC-010, security review
   2026-11-13). Before the fix, the hub kept a check's SNMP community and v3 passwords when the
   device's host changed, so an agent with `anima_device_update` could set the host to an
   address it chose, and the next SNMP check sent the community there. Now the hub binds the
   SNMP secrets to the device host for which they were entered, inside the encrypted value. At
   each probe it compares that host with the host it sends to. If they are different, it sends
   nothing, removes the secrets, records a failed attempt and writes a Warning event that tells
   the operator to enter the secrets again. The check is made at the point of use, so it
   protects against every way a host can change. Secrets stored before the fix are bound to the
   host that their device had when migration 0324 ran. The stored SSH, RDP and VNC credentials
   of a device are fixed the same way (design specification SEC-011): after a host change the
   hub sends none of them until the operator enters it again or changes the host back.
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
    `--allow-write`, or run the command inside a pseudo-terminal. With R-120 the default is already wide, so the check
    protects only a saved `ask` or `off`. The check stops an agent from
    widening its access by accident or because a refusal told it how; it does not stop an agent
    that sets out to do it. The hub token stays the real limit (MCP-003).
11. **SET-004 lists the sections.** Closed (operator, 2026-11-13): `docs/spec/design-spec.md` SET-004
    now names the "MCP" section (MCP-018..023). The design specification also records that the
    section list and the jump menu of SET-004 are not built.

### What a compromised agent can do

An agent can be steered by text it reads. What it can do then depends on the token and the
tools that are on:

1. **Read API key.** Read the whole fleet, events, the audit log, sessions with client
   addresses, VPN nodes and build logs. It can also spend the GitHub budget of each connected
   account down to the reserve: every `repositories` read tool asks GitHub live, and once 500
   or fewer requests are left, every live read stops, the operator's too, until GitHub's reset
   (`UNAVAILABLE`; contracts/extensions/sources.md, Budget). The sync keeps its share. The
   limit is the hour GitHub allows; nothing in Anima limits one key's share.
2. **Write API key, every change tool allowed.** Pause every check and empty its notification
   channels, start maintenance windows, delete checks, maintenance windows and web interfaces,
   end every live session, point web interfaces at other addresses, point HTTP checks at
   internal URLs, send wake packets and test notifications.
3. **Login token, everything allowed.** All of the above, and also delete every device with its
   stored credentials, approve a VPN route, cancel builds, set every retention period to the
   minimum and start a release.
   With a write key or a login token it can also start a GitHub connection (`anima_github_connect_start`), which does nothing until the operator presses Continue on the hub page and then Create and Install on GitHub, behind GitHub's own confirmation. It cannot finish the flow, and it is given no one-time link (operator, 2026-11-14; ruling number to be assigned).
4. **Projects, the first GitHub changes** (operator, 2026-11-14; ruling number to be assigned). With a
   write key and the issue tools on, it can make issues and comments and close or open issues on
   every repository the GitHub App can reach. It cannot merge a pull request, run a workflow again,
   cancel a run, start a workflow, write a commit status or read the content of a file, with any
   token: those have no tool.
   - **It can carry text out.** It can copy text it read from a private repository (an issue, a
     comment, a pull request, a commit message, a file in its own checkout) into an issue or a
     comment in any repository the GitHub App can reach, a public one included. What limits it:
     the issue tools are `allow` by default (MCP-024, R-120) and the operator can set them to `ask`
     or `off`; `ask` shows each call's values before it runs (MCP-013), the App reaches only the repositories it was installed on,
     and the audit log records each change with the key that made it (PRJ-028, LOG-018). Nothing
     checks where the text came from.
   - **A comment can start a workflow.** An issue or a comment made through the GitHub App is an
     event like any other: a workflow of that repository that listens for `issue_comment`,
     `issues` or a label event runs, with the repository's secrets, and its steps may read the
     text the agent wrote. So the issue tools can start code that `workflow start` would need a
     login token and a confirmation for. This was not tested against GitHub.
   - **The issue tools are on without being named.** A tool with no choice of its own takes its
     kind's default (MCP-012), which is `allow` (MCP-024, R-120). An operator who did not name
     them has them on. This is the model as decided; the risk is accepted here, not changed. `anima mcp
     permissions show` and the line at start (MCP-016) list them.
5. **The operator's login token, no declaration.** A program that obtains the operator's stored
   login token (from the key store or the token file, or from `ANIMA_TOKEN`) and sends no agent
   declaration is recorded as User (LOG-016; open point 9). It can do everything the operator
   can through the hub, with no MCP state in the way: merge a pull request, a fork's included
   (the fork's name is typed into a request like any other value; PRO-013 binds the command
   line, not the hub), run and start workflows, write commit statuses and delete devices.
   Nothing in Anima tells it from the operator.

4. **`anima_job_start` allowed (proposed, not built).** Run the build steps and the test steps
   of a project on each machine that takes work, with the content of a scratch ref that the
   agent wrote. Build scripts are code. Thus this is code execution inside the limits of a job: a
   container, or the build user on a Mac, with no daemon token, no home folder and no project
   secret (JOB-020). It can also fill the disk of a machine with artifacts up to the limit of
   the device policy. `anima_capability_list` shows the tools and versions of each machine.

## 10. Work

The tasks of the terminal screen (TU1 to TU10) are in Section 11.8, not in this table.

The state of each task is in the table (PLN-006). T1 to T10 are Done: each commit is the first commit that adds the code of the task to `main`, found on 2026-11-18 (`git log --diff-filter=A`). Their behaviour was not tested again. T11 to T13 are the R-120 work (R-156); they are Not started.

| Task | Content | Builder | Tag | State | Commits |
| --- | --- | --- | --- | --- | --- |
| T1 | Crate, `hub/`, contexts, sign-in, token store, output, exit codes, `api`, `device list\|get`, the K1 coverage test | Opus | security | Done | `6cc9d6ad` |
| T2 | `device` (the rest), `site`, `tag`, `web`, `agent` | Sonnet | security (credentials) | Done | `9c6833aa` |
| T3 | `check`, `maintenance` | Sonnet |  | Done | `6615375e` |
| T4 | `event`, `audit`, `notify`, `retention`, `key`, and the `watch` commands (subscriptions) | Sonnet |  | Done | `b42f2de7` |
| T5 | `release`, `builder`, `vpn` | Sonnet | security (keys shown once) | Done | `04f64e10` |
| T6 | `session`, `ssh`, `files` | Opus | security | Done | `f3303c43` |
| T7 | `anima mcp`: server, tool sets, read tools, change tools | Opus | security | Done | `f5819b94` |
| T8 | Review (CUPID, tests), security review, gates | Sonnet, Opus |  | Done, unverified (no commit found) | — |
| T9 | Per-tool states `allow`, `ask`, `off`; `permissions show\|set\|reset`; `ask` by elicitation; the `sessions` tool set (MCP-012..017) | Opus | security | Done | `f5819b94` |
| T10 | The MCP permissions panel in the desktop Settings page, on the library behind `permissions show\|set\|reset` (MCP-018..023) | Sonnet | security (file write) | Done | `f27a56a1` |
| T11 | R-120, the library and the command: the built-in state of the change tools is `allow`, and the switch "Ask before destructive actions" (MCP-024, MCP-025) in `permissions show\|set\|reset` | Opus | security | Not started | — |
| T12 | R-120, the audit: each destructive call that runs without a question is in the hub's audit log with the tool, the cleaned arguments and the MCP client name (MCP-026) | Opus | security | Not started | — |
| T13 | R-120, the desktop Settings panel: the switch "Ask before destructive actions", off for a new user (MCP-025), on the library of T11 | Sonnet |  | Not started | — |

### Tasks of R-120 (T11 to T13)

R-120 (operator, 2026-11-16) is ruled and not built. The code starts with every change tool `off` (`crates/anima-cli/src/mcp/permissions.rs`, `Permissions::built_in`). Build in the order T11, T12, T13. One pull request for each task.

**T11. The default and the switch.**
- Why: MCP-024 and MCP-025 are ruled and the code does the opposite.
- Current state: `built_in(Kind::Change)` is `Off`. No switch "Ask before destructive actions" exists in the library or in `permissions set`.
- Change: make the built-in state of a change tool `allow`. Add the switch of MCP-025 to the saved permissions, `permissions show`, `set` and `reset`. When it is on, each tool with `destructiveHint` is `ask` unless the operator chose a state for that tool. Keep `off` hidden and the choices for single tools first (MCP-025). Keep MCP-014 as MCP-014 says for a change away from a saved `ask` or `off`.
- Acceptance: pass when a new user has `allow` for every read and change tool, the switch is off, turning it on makes each destructive tool `ask`, a saved tool state still wins, and `off` still hides a tool; fail otherwise.
- Check: `cargo nextest run -p anima-cli mcp`, `cargo clippy -p anima-cli --all-targets -- -D warnings`, `bun run gates`.
- Out of scope: the audit (T12), the Settings panel (T13), CMD-024 (the command line keeps its question).
- Always: write the test of each state first. Ask: before a change to MCP-004 to MCP-008, which do not change. Never: let a tool take or return a secret.
- Done: the commit that merges the change and sets T11 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task T11 of docs/plans/cli-mcp-plan.md (R-120, MCP-024, MCP-025). Change the built-in state of the change tools to allow and add the switch 'Ask before destructive actions' to the permissions library and to the permissions show, set and reset commands. Run bun run gates."

**T12. The audit of an unasked destructive call.**
- Why: MCP-026 says that the operator must see what an agent deleted.
- Current state: not verified. The audit log has the actor `agent` (LOG-020). Whether an entry holds the tool, the cleaned arguments and the MCP client name is not checked.
- Change: check the entry that a destructive MCP call writes. Add what MCP-026 names and the hub does not yet hold. Use the clean-up of MCP-013 for the arguments. Write no secret (MCP-004, LOG-007).
- Acceptance: pass when a test calls a destructive tool in state `allow` and finds one entry with the tool, the cleaned arguments and the MCP client name, and finds none when the call was refused; fail otherwise.
- Check: `cargo nextest run -p anima-cli -p anima-hub audit`, `bun run gates`.
- Out of scope: the hash chain (R-138); the Settings panel.
- Always: one entry for each call. Ask: before a new field in K1 (CON-001). Never: log a token or a secret argument.
- Done: the commit that merges the audit change and sets T12 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task T12 of docs/plans/cli-mcp-plan.md (R-120, MCP-026). First check what the audit entry of a destructive MCP call holds. Add the tool, the cleaned arguments and the MCP client name. Write no secret. Run bun run gates."

**T13. The Settings switch.**
- Why: the operator sets the switch of MCP-025 in the desktop Settings page as well as in the command line.
- Current state: `apps/client/src/lib/mcp/` has the panel of T10 with per-tool states and no switch.
- Change: add the switch "Ask before destructive actions", off for a new user, on the library of T11 (MCP-018 to MCP-023 keep their rules). Update the Settings mockup in the same commit when it changes (PLN-019).
- Acceptance: pass when the switch shows off for a new user, turning it on writes the saved state, and the panel shows each destructive tool as `ask`; fail otherwise.
- Check: `bun run --filter '*' test`, `bun run gates`.
- Out of scope: the audit; per-tool changes.
- Always: use the tokens. Ask: before a new route. Never: write the file from the page without the library of T10.
- Done: the commit that merges the switch and sets T13 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task T13 of docs/plans/cli-mcp-plan.md (R-120, MCP-025). Add the switch 'Ask before destructive actions' to the MCP section of the Settings page, on the library of T11. Run bun run gates."

Done when: `bun run gates` passes; each row of Section 6 works against a test hub; the coverage
test of CMD-031 passes; an MCP client lists the tools and reads the device list.

## 11. The terminal screen: `anima tui`

Status: Direction decided (PLN-007). Version 0.3 (2026-11-18). R-168 (operator, 2026-11-18, first-hand, Grok Bot chat) decided OD-TUI1 to OD-TUI5 and OD-TUI8: `anima tui` is a subcommand; version 1 is a read-only dashboard that hands off to `anima ssh`; the work starts after R8a to R8d; the screen has row 4.20; the design specification wording is changed; a bare `anima` stays exit 2; the screen does not wait for OD-25; Windows is out of scope; and TU1 may run before R8d. Before R-168, Anima Product Ideas had relayed the first answers from its own chat; the operator confirmed them himself. **The requirements TUI-001 to TUI-083 are not approved.** R-168 decides the open decisions only. The one task that a ruling names is TU1 (a measurement, a record only). TU2 to TU10 are not free to build. OD-TUI6 and OD-TUI7 are open until TU1. The idea folder `docs/ideas/terminal-ui/` is deleted in the commit that adds this section (PLN-018).

This section is part of this plan and not a new document. The plan already owns the command line (CMD), and a terminal screen shows functions that the command line already has (PLN-004). It uses a new identifier prefix, `TUI-nnn`, so that the numbers of the other proposals of the command line are not used (CMD-032 to CMD-038 belong to [shell-prompt.md](../proposals/shell-prompt.md)). A search of every branch on 2026-11-18 found no `TUI-` identifier, no `TU` task and no row 4.20.

### 11.1 Purpose and scope

A person who works only in a terminal cannot use the desktop application: for example over SSH, on a server with no desktop, or on a small machine. Today that person gets one-shot commands and `--watch` lines, not one live screen. `anima tui` is a full-screen, read-only view of the hub in the terminal. It shows what is wrong, and it hands the terminal to `anima ssh` when the person wants to enter a device.

| In version 1 | Later, with its own approval | Never in this section |
| --- | --- | --- |
| Devices with live status. The checks and the last events of the selected device. All events. Open sessions. Filter, sort, help. A key that hands the terminal to `anima ssh`. | A terminal pane inside the screen (TUI-042). Views of the Needs You inbox or the agent usage counts. A command that changes a device. A mouse. | A second program. A new hub function. Any change to K1 to K7. A change tool of any kind. A login form inside the screen. Windows. |

Why Ratatui. It is the common Rust library for full-screen terminal programs: version 0.30.2, published 2026-06-19, MIT licence, minimum Rust 1.88, 72.8 million downloads (crates.io, read on 2026-11-18). The repository pins Rust 1.100.0 (`rust-toolchain.toml`), so the minimum is met. `anima-cli` already has `crossterm`, a raw-mode guard, a live subscription client and `anima ssh`. The competitor tools that the idea named (Herdr, Agent Manager, Homebutler) are in the history of the deleted idea folder.

### 11.2 Rules

"The screen" means `anima tui`.

**The command and its limits**

- **TUI-001.** The screen must be the subcommand `anima tui` of the program `anima`. It must not be a second program or a second binary (CMD-001). The code must be one folder, `crates/anima-cli/src/tui/`. A screen is not a hub noun, so this folder is the one exception to the naming of CMD-002. A GraphQL document that the screen needs and that no command has goes in the folder of its noun (for example `checks/`), as CMD-002 says.
- **TUI-002.** The screen must start only when standard input and standard output are both terminals (CMD-023, CMD-027). If one is not, `anima tui` must fail with a usage failure, exit code 2 (CMD-021), and the hint must name a command that works in a pipe, for example `anima device list --json`. The flag `--json` is a usage failure for this command.
- **TUI-003.** The screen is part of the command line. It is not the client, and it adds no function that the hub, the command line and the MCP server do not already have. It must call only the operations of K1. It must not change the hub, K1 or any other contract (CMD-005).
- **TUI-004.** The screen must be read-only. Its code must send no mutation of K1. A test must read each GraphQL document of the folder and fail when one is a mutation. The one action that is not a view is the hand-off of TUI-040, and that code is the code of `anima ssh`.
- **TUI-005.** A bare `anima` with no subcommand must stay a usage failure, exit code 2. It must not open the screen. A test must pin this: no test pins it today. (Why: the screen must be a choice, and a script that runs `anima` by mistake must not hang on a terminal.)

**What the screen shows**

- **TUI-010.** The devices view must list each device: a status (symbol and word, TUI-032), the name, the kind, the site and the tags. It must show a device in maintenance as maintenance. The default order must be the worst status first, then the name, so that a problem is at the top. The person can change the order to the name.
- **TUI-011.** The detail of the selected device must show its host, address, operating system, kind, site and tags, its checks (name, type, status, paused) and its last ten events. The checks must come from a document that does not select the `config` of a check, because the path of a push check holds its token (the document of `check list` has the same limit).
- **TUI-012.** The events view must show new events at the top, as they arrive. It must keep at most 500. The person can set the lowest severity and can limit the view to the selected device.
- **TUI-013.** The sessions view must list the open sessions: the device, the protocol, the operator name and the time since the start. It must update when a session starts or ends.
- **TUI-014.** A header must show the hub origin and the context name, the state of the live connection (live, reconnecting, or stale with its age), and the count of devices that are up, degraded, down and in maintenance. It must never show a token. A footer must show the keys that work now.
- **TUI-015.** The person can filter the devices by a part of the name with `/`. The filter works on the list that the screen holds. It sends no request.

**Live data**

- **TUI-020.** The screen must read the hub only through `hub/` (CMD-003): the same typed operations and the same subscription client as the `watch` commands. It must add no network code. Each GraphQL document must be a `.graphql` file that the gates check against K1 (CMD-004).
- **TUI-021.** The screen must load each list once at the start. Then it must keep the lists live with three subscriptions: `deviceStatusChanged`, `eventAdded` and `sessionChanged`. It must read the list of devices and the list of sessions again every 60 seconds, because a new or deleted device has no subscription. It must read the checks and the last events of the selected device again every 15 seconds, and when the selection changes. [NEEDS CLARIFICATION: the two intervals are a first guess. TU1 measures the load on a hub with 50 devices and the person decides.]
- **TUI-022.** The client of one subscription opens one socket (`hub/subscription.rs` runs one operation for each socket). The screen therefore opens three sockets and must close all three when it ends. [NEEDS CLARIFICATION: whether the hub limits the sockets of one token. The documents do not say; TU1 reads the hub code and tests it.]
- **TUI-023.** When a socket closes, the screen must open it again after 1 second, then 2, 4, 8, 16 and 30 seconds, and keep 30 seconds. After each new connection the screen must read every list again. It must not assume that it saw each event. During the wait the header says "reconnecting". The data on the screen stays, and its age is shown.
- **TUI-024.** The screen must keep a bounded amount of data: at most 1,000 devices, 500 events and 200 sessions. If the hub has more devices, the screen must say "Showing the first 1,000 of N" and must not fail. [NEEDS CLARIFICATION: 1,000 devices is a first guess. The largest known hub has far fewer.]

**The terminal**

- **TUI-030.** The screen must use the alternate screen and raw mode. It must restore the terminal on every exit: a normal exit, an error, a panic, `SIGINT`, `SIGTERM` and `SIGHUP`. It must use the guard `raw_mode.rs` and the signal code of `anima ssh`, and not copy them. It must not turn on the mouse, bracketed paste or the focus events. It must not write to the clipboard (OSC 52) and must not write a link (OSC 8).
- **TUI-031.** The screen must redraw when the window changes size. If the window is smaller than 80 columns by 24 lines, it must show one line, "Make the window at least 80 by 24", and nothing else, and it must not fail. [NEEDS CLARIFICATION: 80 by 24 is the usual minimum. The layout of 11.4 was drawn at this size.]
- **TUI-032.** The screen must use only the 16 ANSI colours of the terminal, so that the colour scheme of the terminal applies (RA-010 gives the Nord scheme of the application terminal). It must never carry a meaning by colour alone: a status is a symbol and a word (`▲ up`, `▼ down`, `◆ degraded`, `■ maintenance`, `? unknown`; for a terminal that cannot draw them, `+`, `!`, `~`, `m`, `?`). When `NO_COLOR` is set, it must use no colour (CMD-027).
- **TUI-033.** The keys must be: up and down or `k` and `j` to select; `Tab` or `1`, `2`, `3` to change the view; `/` to filter; `s` to change the order; `e` to limit the events to the selected device; `t` to hand off (TUI-040); `r` to read the lists again now; `?` for help; `Esc` to close a filter or the help; `q` or `Ctrl-C` to quit. No key sends a change to the hub.
- **TUI-034.** Each string that comes from the hub (a name, a tag, an event message, a check message) must be cleaned of control characters with `output::text::clean` before it is drawn (CMD-022), and must be cut to the width of its column with an ellipsis, by the display width of the characters. The screen must never write a string from the hub to the terminal as it is.

**The hand-off to `anima ssh`**

- **TUI-040.** The key `t` on a device must hand the terminal to `anima ssh` for that device. The screen must leave the alternate screen and restore the terminal, run the code of `anima ssh` in the same process (not a child process), and, when the session ends, enter the alternate screen again and read every list again. The hand-off needs the login token (INT-003: an API key must not call `startSession`). With an API key or no login token, the screen must stay open and show the footer line "Opening a terminal needs a login token: run `anima auth login`".
- **TUI-041.** If `anima ssh` fails, the screen must come back and show the failure as a line in the footer, with the hint of the failure. It must not show a stack or leave the terminal in raw mode.
- **TUI-042.** A terminal pane inside the screen is not in version 1. It needs a terminal emulator in the program, and it needs the rules of [ssh-terminal.md](../proposals/ssh-terminal.md) (OSC 52 off, links only after a confirmation). [NEEDS CLARIFICATION: its own proposal and approval.]

**Sign-in and the hub**

- **TUI-050.** The screen must find its hub and its token as CMD-010 to CMD-015 say (`--hub`, `ANIMA_HUB`, a context, `ANIMA_TOKEN`, the key store). If there is no token, or the hub is not reachable, the command must fail before it enters the alternate screen, with exit code 4 or 7 (CMD-021) and a message on standard error. The screen has no password form.
- **TUI-051.** If the hub refuses the token while the screen runs (close code 4403), the screen must show "Signed out. Run `anima auth login`", stop all reconnects, keep the last data with its age, and accept only `q`. After `q` the exit code must be 4.
- **TUI-052.** The screen must work with a read API key (`anima auth login --with-token`). This is the only way for a hosted hub until OD-25 is decided: the standalone command line has no other way to sign in to a hosted hub (HMD-032, HMD-038, R-129; OD-25 is not ruled). The screen must not wait for OD-25. Only the hand-off (TUI-040) needs the login token.

**Platforms**

- **TUI-060.** The screen is for macOS and Linux. The crate uses Unix signals (`tokio::signal::unix`) and Unix file modes (`private_file.rs`), so the whole of `anima-cli` does not build on Windows today. The standalone files of R8b are a macOS universal file and a Linux x86_64 file only. Windows is out of scope. A Linux aarch64 file is also out of scope in R8b: a small machine of that kind uses `cargo install anima-cli` (INS-011).

**Build and code**

- **TUI-070.** The screen must add the crate `ratatui` 0.30 (MIT, compatible with the licence of this repository) to the workspace and to `anima-cli`, and no other direct dependency, unless TU1 shows that one is needed. The version is the line `0.30` in `Cargo.toml`, and `Cargo.lock` pins it. INS-011 holds: every dependency of `anima-cli` must be on crates.io. TU1 measures the growth of the standalone `anima` file and the build time. [NEEDS CLARIFICATION: how much growth the operator accepts (OD-TUI9). Stop if it is more.]
- **TUI-071.** The code must follow the repository rules: at most 300 code lines in a file, at most 12 files in a folder (REPO-005), no weak names (CQ-030, so no `utils`, `helpers`, `handler`, `info`, `item`), no `unsafe` code, and Clippy `pedantic` as an error.

**Tests**

- **TUI-080.** The change of state must be a pure function: `update(state, action)` gives a new state and a list of effects, and does no input or output. The drawing of each view must be a pure function of the state: `draw(&state, &mut Frame)`. Tests must call both with no terminal and no network.
- **TUI-081.** Each view must have a test with the Ratatui `TestBackend` that compares the drawn buffer with the expected lines. The states are: empty, loaded, selected, filtered, stale, reconnecting, signed out, too small, a long name and a hostile name (a string with an escape sequence must not reach the buffer). The tests need no new dev dependency: `Buffer::with_lines` is enough.
- **TUI-082.** A test of the end-to-end suite (`crates/anima-cli/tests/it/end_to_end/`, which runs against the real hub with `RealHub`) must start `anima tui` on the pseudo-terminal of `tests/it/kit/terminal.rs`, find a device name from the real hub on the drawn screen, press `q`, and prove that the terminal mode is restored (`assert_modes_restored`, as the sign-in tests do). A second test must press `t` against the test SSH server of `tests/it/kit/tunnel_hub.rs` and prove that the hand-off runs and returns.
- **TUI-083.** Each failure of 11.5 must have a test. A human check must exist for a real terminal (macOS Terminal, a Linux console over SSH, `tmux`, a resize, a window below 80 by 24). The entry in [need-human-testing.md](../need-human-testing.md) is added in the commit of task TU10 (PLN-014). A task is Done only after the operator sets it to Passed.

### 11.3 The change to the design specification (made, R-168)

The design specification Section 2 says that the command line "is not the client". A terminal screen does not conflict with this, because of TUI-003: it adds no function, it uses K1 like the command line, and the `CLI-nnn` rules of the graphical client (the client uses the same user interface code on all platforms) do not apply to it. The words must still say it, or a reader can think that the command line became a client. The operator approved the change (OD-TUI2, R-168), and the commit that records R-168 made it. The table gives the old and the new text.

| Place | Before | Now |
| --- | --- | --- |
| Section 2, term "Command line" | "The `anima` program. The operator and scripts use it in a terminal (Section 6.10). It is not the client." | "The `anima` program. The operator and scripts use it in a terminal (Section 6.10). Its full-screen view, `anima tui`, is part of it. Neither is the client." |
| Section 6.10 | No text about a full-screen view. | One sentence: "`anima tui` is a read-only full-screen view of the hub in a terminal. It shows functions that the command line already has (TUI-003)." |
| [ownership.md](../plan/ownership.md) of the plans, row of the command line | Concerns CMD and MCP. | Concerns CMD, MCP and TUI. |

### 11.4 The screen

The wireframe is 80 columns wide. The mockup [docs/mockup/terminal-ui/index.html](../mockup/terminal-ui/index.html) shows the same screens with the terminal tokens (`design/tokens/terminal.tokens.json`, the Nord scheme). Ratatui draws the box lines. The signs are examples.

```text
 anima  hub.anima.ing (home)                      ● live    ▲ 14  ◆ 1  ▼ 2  ■ 1
  1 Devices  2 Events  3 Sessions                           sorted: worst first 
┌ Devices (18) ─────────────────────────┐┌ web-02 ─────────────────────────────┐
│ ▼ down   web-02     server  home      ││ host   web-02.lan   10.0.0.12       │
│ ▼ down   nas-01     server  home      ││ os     Linux        kind server     │
│ ◆ deg.   build-mac  mac     office    ││ site   home    tags  prod, web      │
│ ■ maint. backup-01  server  home      ││                                     │
│ ▲ up     ollama     server  home      ││ Checks                              │
│ ▲ up     pve-01     server  home      ││ ▼ ping      down     60 s           │
│ ▲ up     router     network home      ││ ▲ https     up       60 s           │
│ ▲ up     studio     mac     office    ││                                     │
│ ...                                   ││ Events                              │
│                                       ││ 14:02 ERR  ping failed 3 times      │
│                                       ││ 13:58 WARN https slow (2.4 s)       │
│                                       ││ 13:41 INFO came back up             │
└───────────────────────────────────────┘└─────────────────────────────────────┘
 ↑↓ select  / filter  s order  t terminal  r refresh  Tab view  ? help  q quit
```

The other states, in words:

- **Stale.** The header shows `◌ reconnecting (retry in 8 s)` and the age of the data, for example `data 41 s old`. The lists stay.
- **Signed out.** The header shows `✖ signed out`. One line of the footer says "Signed out. Run `anima auth login`." Only `q` works.
- **Empty hub.** The list says "No devices yet. Add one in the application or with `anima device create`."
- **Too small.** One line: "Make the window at least 80 by 24".
- **No colour.** The same text, with no colour. The symbols and the words carry the meaning (TUI-032).

### 11.5 Review of the scope and the design

**Scope.** The narrowest version of the idea is kept: read-only, three views, one hand-off. These parts were considered and left out, each with its reason.

| Part | Decision | Reason |
| --- | --- | --- |
| A terminal pane inside the screen | Later (TUI-042) | It needs a terminal emulator in the program and the rules of the SSH proposal. It doubles the size of the work. The hand-off gives the same use at a fraction of it. |
| A command that changes a device (wake, pause a check) | Never in this section | The operator answered "read-only" (OD-TUI1). A change tool also needs the confirmation rules of CMD-024 inside a full-screen program. |
| A mouse | Later, not planned | Mouse capture takes away the selection and copy of the terminal. |
| A login form | Never in this section | `anima auth login` exists. A password form in a full-screen program needs the hidden prompt in a second place (CMD-010). |
| Views of the Needs You inbox, the agent usage and the audit log | Later | Each is an own idea or a built command; add a view after version 1 is used. |
| Open `anima tui` when `anima` has no subcommand | No | See OD-TUI4: a script must not hang. |

**Design review.** Each score is for the draft of this section, from 0 (bad) to 10 (right). The last column says what would make it 10, and what the draft already did about it.

| Dimension | Score | What would make it 10, and what the draft did |
| --- | --- | --- |
| Information hierarchy | 8 | Worst status first (TUI-010) puts a problem at the top; the header counts show the state of the hub at a glance. To reach 10 the counts would link to a filter of that status (a key `f`); left for after the first use. |
| Consistency with the command line and the application | 8 | The names, the status words and the filters are those of `anima device list`; the order "worst first" is the one of `--sort status`. The application shows the same status colours; the screen uses the terminal's ANSI scheme (TUI-032) and so follows the terminal, not the application. |
| States: empty, loading, error | 8 | Empty hub, reconnecting, stale, signed out and too small each have a line (11.4). A first load shows "Loading devices" until the first answer; a slow hub shows it for as long as it takes. A load that fails shows the failure of the hub. |
| Keyboard model | 8 | Few keys, one meaning each, shown in the footer, `?` for help, `q` and `Ctrl-C` both quit. Not 10: the keys `j` and `k` and `/` are the habit of `vim` and `less`; a person who does not know them has the arrows and the footer. |
| Colour and accessibility | 8 | No meaning by colour alone (symbol and word); 16 ANSI colours so that the terminal scheme and its contrast apply; `NO_COLOR` works. A screen reader does not read a full-screen program well; that cannot be fixed here, and `anima device list` stays the way for such a person. The help says so. |
| Small windows | 7 | 80 by 24 is the minimum. Below it, one line. Between 80 and about 100 columns the detail pane is narrow and cuts names with an ellipsis. To reach 10 the detail pane would hide under 100 columns and show on `Enter`; the task TU5 does this. |
| Terminal compatibility | 7 | Alternate screen and raw mode work in `tmux`, over SSH and in the common terminals. No mouse, no OSC 52, no OSC 8 avoids the unsafe ones. Not tested in a real terminal yet: HT entry in TU10. |
| Not generic | 7 | The one use that others lack is the hand-off to a terminal through the hub, with the credential kept on the hub. It is drawn into the wireframe (`t terminal`). |

**Result of the review.** Two changes came from it and are in the rules: the detail pane hides under 100 columns (task TU5), and the stale state shows the age of the data (TUI-014, TUI-023).

### 11.6 Engineering

**What already exists, and is reused.** `hub/` (the request, the subscription socket, the error codes). The typed documents of `device list`, `event list`, `check list`, `session list` and the four `watch` documents. `raw_mode.rs` (restores on every exit and on a panic). The signal code of `anima ssh` (`sessions/terminal/console.rs`). `output::text::clean`. `environment.rs` (which streams are terminals). The test kit: `RealHub`, the pseudo-terminal `Terminal`, `assert_modes_restored`, `subscription_hub.rs` and the test SSH server in `tunnel_hub.rs`.

**Data flow.**

```text
 keys ─────────┐                               ┌── effects: read a list, open a socket,
 window size ──┤                               │   hand off to ssh, quit
 timer tick ───┼─► Action ─► update(state) ─►──┤
 hub answers ──┘      ▲                        └── new State ─► draw(&State, Frame) ─► terminal
 socket events ───────┘
 (feed tasks send Action values over one channel; a draw loop selects over keys, the tick and the channel)
```

- The draw loop runs on the `current_thread` runtime of `main`. It selects over the `crossterm` event stream, a one-second tick and one `mpsc` channel. [NEEDS CLARIFICATION: the `event-stream` feature of `crossterm` must work on a `current_thread` runtime with the key reader of the hand-off. TU1 proves it.]
- Three feed tasks, one for each subscription, and one polling task send `Action::Hub(..)` values. Each task owns its socket and its backoff (TUI-023). Dropping the screen ends them.
- The folder `src/tui/`: `state.rs` (types: `View`, `Screen`, `Snapshot`, `Connection`), `action.rs`, `update.rs` (pure), `keys.rs`, `feed/` (loads, subscriptions, backoff), `view/` (`header.rs`, `devices.rs`, `detail.rs`, `events.rs`, `sessions.rs`, `help.rs`, one file each), `terminal.rs` (the guard, the loop) and `handoff.rs`. The 300-line and 12-file limits shape this layout (TUI-071).
- A device is a value of the K1 type, and a status is the K1 enum, not a string. The screen has no second copy of a K1 type that it can avoid.

**Failure modes.** Each row has a test (TUI-083) or a human check.

| Failure | What the person sees | What the code does |
| --- | --- | --- |
| No token at the start | Message on standard error, exit 4 | Fails before the alternate screen (TUI-050). |
| Hub not reachable at the start | Message on standard error, exit 7 | Fails before the alternate screen. |
| Hub refuses the token while running | "Signed out", last data with age | Stops all reconnects, only `q`, exit 4 (TUI-051). |
| A socket closes | "reconnecting", data stays with its age | Backoff 1 to 30 s, then all lists again (TUI-023). |
| The hub is slow | Data ages; the header shows the age | A request has the timeout of `hub/`; a late answer is dropped if the view changed. |
| A hub with many devices | "Showing the first 1,000 of N" | Bounded lists (TUI-024). |
| A hostile device name or message | A cleaned, cut string | `output::text::clean`, width cut (TUI-034). |
| Window too small | One line | No draw of the views (TUI-031). |
| The program panics | The terminal is normal again, then the panic text | The guard restores before the panic prints (TUI-030). |
| `SIGTERM`, `SIGHUP`, closed terminal | The terminal is normal again | The signal code of `anima ssh` (TUI-030). |
| `anima ssh` fails in the hand-off | The screen returns; one line with the failure and its hint | TUI-041. |
| The login token is missing for the hand-off | One footer line, the screen stays | TUI-040. |

**Trust boundaries.**

| Boundary | Rule |
| --- | --- |
| Hub to screen | Every string is untrusted. It is cleaned and cut before it is drawn (TUI-034). The screen writes no link and no clipboard sequence (TUI-030). |
| Token | The token stays in `hub/` and in the key store (CMD-012, CMD-014). The screen has no code that reads it, shows it or writes it to a log. |
| Checks | The screen never selects the `config` of a check, because a push check's path is a token (TUI-011). |
| Screen to hub | Queries and subscriptions only (TUI-004). A test lists the documents. |
| Hand-off | The credential of the device stays on the hub (`anima ssh`, K4 Section 8). The screen does not see it. The screen changes nothing about how the hub relays the terminal. |

**Blast radius.** What a build of this section touches, and what it must not.

| Place | Change | Risk and check |
| --- | --- | --- |
| `crates/anima-cli/src/tui/` | New folder | The 300-line and 12-file limits (`bun run quality:limits`). |
| `crates/anima-cli/src/cli/program.rs`, `cli.rs`, `lib.rs` | One variant, one arm, one `mod` line | The command tree test and the completion output change by one command; the K1 coverage test (CMD-031) is not affected. |
| `Cargo.toml` (workspace), `crates/anima-cli/Cargo.toml`, `Cargo.lock` | The crate `ratatui` | **The `crossterm` features.** `ratatui-crossterm` 0.1.2 depends on `crossterm` 0.29 with its default features on. The workspace sets `default-features = false` for `crossterm`. Cargo unites the features, so the build turns on `events`, `bracketed-paste`, `windows` and `derive-more` for `anima-cli`, and adds `signal-hook` and `signal-hook-mio`. `anima-cli` is the only crate that uses `crossterm`, so the effect stays in this crate. TU1 must read the result and check that the signals of `anima ssh` still work. |
| The standalone `anima` file (R8b) and the crate (R8a) | The file and the crate grow | Not measured. TU1 measures it (TUI-070). The Homebrew channel is the reason for the limit. |
| The MCP server (`src/mcp/`) | None | A test must show that `anima mcp` starts and lists the same tools. |
| The hub, K1 to K7, the application, the daemon | None | TUI-003. The gates `cargo nextest run --workspace` and `bun run gates` must pass unchanged. |
| The design specification | Section 2 and Section 6.10, one sentence each (11.3, R-168) | Made in the commit of R-168. |
| `docs/` | This section, the row 4.20, the mockup, the ownership row | The documents gate. |

**NOT in scope** (kept short): a mouse, a login form, a change tool, a terminal pane, Windows, Linux aarch64, the views of other ideas, a theme file, a configuration file for keys.

### 11.7 Open decisions

PLN-018 asks for one question for each row that the operator answers with yes, no or one choice, and a recommendation. R-168 decided OD-TUI1 to OD-TUI5 and OD-TUI8. OD-TUI6 and OD-TUI7 are open.

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| OD-TUI1 | Do you confirm four answers that Anima Product Ideas relayed from its chat: `anima tui` is a subcommand; version 1 is a read-only dashboard that hands the terminal to `anima ssh` (a terminal pane later); it starts after R8a to R8d of R-151; and it has a new priorities row of its own (row 4.20)? | Yes. It matches what was relayed. | Decided: yes, R-168 (operator, first-hand: "OD-TUI1: yes, I confirm."). |
| OD-TUI2 | Do you approve the wording change of the design specification Section 2 and Section 6.10 (11.3): the full-screen view is part of the command line, and neither is the client? | Yes. The `CLI-nnn` rules of the graphical client stay for the application only. | Decided: yes, R-168 ("OD-TUI2: yes."). Made in the design specification. |
| OD-TUI3 | Does a bare `anima` with no subcommand stay a usage failure, exit code 2 (TUI-005)? | Yes. A script that runs `anima` by mistake must not wait on a screen. A person types `anima tui`. | Decided: yes, R-168 ("OD-TUI3: yes."). |
| OD-TUI4 | Does the screen wait for OD-25 (sign-in of the standalone command line to a hosted hub)? | No. It works with a read API key now (TUI-052). Only the hand-off waits for a login token. | Decided: no, do not wait, R-168 ("OD-TUI4: no, don't wait."). |
| OD-TUI5 | Is Windows out of scope for the screen and for `anima ssh` (TUI-060)? | Yes. The crate does not build on Windows today, and R8b has no Windows file. | Decided: yes, R-168 ("OD-TUI5: yes, Windows out of scope."). |
| OD-TUI6 | Are three subscriptions plus a re-read every 60 seconds the way to keep the data live (TUI-021)? The other ways are: only a re-read every few seconds (simpler, more load), or a subscription for each list including checks (four sockets). | Three subscriptions, a re-read of the lists every 60 seconds and of the selected device every 15 seconds. TU1 measures the load before the numbers are fixed. | Open. |
| OD-TUI7 | Is a growth of the standalone `anima` file up to a limit that you name after TU1 measures it acceptable (TUI-070)? | Ask after TU1. The measure comes first, so the question has a number. | Open until TU1. |
| OD-TUI8 | May TU1, the measurement of size and dependencies, run before R8d, as a record in `docs/research/` (PLN-007 allows a measurement in the state Draft)? | Yes. It builds no product code, and its result can change OD-TUI7 before R8 ends. | Decided: yes, R-168 ("OD-TUI8: yes."). TU1 may run before R8d. |

### 11.8 Tasks of the terminal screen (TU1 to TU10)

TU1 is free to build now: R-168 decided OD-TUI8 (yes) and names TU1. It writes a record in `docs/research/terminal-ui/` and no product code. TU2 to TU10 are not free to build: they need the result of TU1 (go), R8d Done (R-151, OD-TUI1) and an approval of the requirements by ruling (PLN-007). Build in the waves below. One pull request for each task. Each task starts with a claim ([claims/](../claims/README.md), PLN-022) and ends with its state in this table (PLN-008, PLN-009).

| Task | Wave | Content | Builder | Tag | State | Commits |
| --- | --- | --- | --- | --- | --- | --- |
| TU1 | 1 | Spike and decision record: dependency count, file size, build time, the `crossterm` features, the event stream on a `current_thread` runtime, the socket limit of the hub, the load of the polling | Opus | | Not started. Free to build (R-168, OD-TUI8) | — |
| TU2 | 2 | The skeleton: the dependency, `anima tui`, the guard, the refusal off a terminal, an empty frame, `q`, the `TestBackend` helper, the bare `anima` test | Sonnet | | Not started. Waits for TU1, R8d and approval of the requirements | — |
| TU3 | 3 | State, actions and the pure `update`: types, keys, selection, filter, order, stale | Sonnet | | Not started. Waits for TU1, R8d and approval of the requirements | — |
| TU4 | 4 | The feed: the first loads, the three subscriptions, the polling, the backoff, the loss of sign-in | Opus | security | Not started. Waits for TU1, R8d and approval of the requirements | — |
| TU5 | 4 | The devices view and the detail pane, with the header and the footer, and the hiding of the detail under 100 columns | Sonnet | | Not started. Waits for TU1, R8d and approval of the requirements | — |
| TU6 | 5 | The events view | Sonnet | | Not started. Waits for TU1, R8d and approval of the requirements | — |
| TU7 | 5 | The sessions view | Sonnet | | Not started. Waits for TU1, R8d and approval of the requirements | — |
| TU8 | 5 | The hand-off to `anima ssh` | Opus | security | Not started. Waits for TU1, R8d and approval of the requirements | — |
| TU9 | 6 | The end-to-end tests on a pseudo-terminal, each failure of 11.6, the hostile-text test | Opus | security | Not started. Waits for TU1, R8d and approval of the requirements | — |
| TU10 | 7 | The help text, the completion, the entry in need-human-testing.md, the mockup comparison, the state of this section | Sonnet | | Not started. Waits for TU1, R8d and approval of the requirements | — |

Waves: TU4 and TU5 can run at the same time, because they need only TU3 and touch different folders. TU6, TU7 and TU8 can run at the same time. TU8 needs only TU5.

**TU1. Spike and decision record.**
- Why: three facts decide the cost and none is measured: the growth of the file, the effect of the `crossterm` features, and the load on the hub.
- Needs: nothing more. R-168 (OD-TUI8) lets it run before R8d. A copy of the repository on a branch of its own. It produces no product code.
- Change: add `ratatui` 0.30 to a throwaway branch. Build the release `anima` for the host before and after, and write both sizes (stripped), the build time and the number of new crates (`cargo tree`). Read the features that `crossterm` has after the change. Run a small `event-stream` loop on the `current_thread` runtime and press keys and resize the window. Read the hub code for a limit of sockets for each token, and test three sockets against the loopback hub. Count the requests of the polling of TUI-021 for a hub with 50 devices. Write the result in `docs/research/terminal-ui/spike.md` (a record, PLN-010) and put the numbers in OD-TUI6 and OD-TUI7.
- Acceptance: pass when the record holds each number named above, says whether `anima ssh` signals still work, and says "go" or "stop" against the limit of OD-TUI7 (or "ask" if the operator has not named one); fail otherwise.
- Check: `cargo build --release -p anima-cli`, `cargo tree -p anima-cli --edges normal | wc -l`, `ls -l target/release/anima`, `bun run quality:documents`.
- Out of scope: any code that stays in `crates/`. The Homebrew files.
- Always: write the numbers, not words. Ask: before a version of `ratatui` other than 0.30. Never: merge the throwaway branch.
- Done: the commit that merges the record and sets TU1 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task TU1 of docs/plans/cli-mcp-plan.md (Section 11.8). On a throwaway branch add ratatui 0.30 to crates/anima-cli and measure: the release file size before and after, the build time, the new crates, the crossterm features after unification, an event-stream loop on a current_thread runtime with key and resize events, the hub's limit of sockets for one token, and the polling load for 50 devices. Write only docs/research/terminal-ui/spike.md with the numbers and a go, stop or ask line. Merge no product code."

**TU2. The skeleton.**
- Why: every later task needs the command, the guard and a way to test a frame.
- Needs: TU1 (go), R8d Done, an approval of TUI-001, TUI-002, TUI-005, TUI-030 and TUI-071 by ruling.
- Change: add `ratatui` 0.30 to the workspace and to `anima-cli`. Add `src/tui/` with `terminal.rs` (the guard that uses `RawMode` and the alternate screen, and the signal code of `anima ssh`) and `anima tui` as a variant of the `program` group. Off a terminal it fails with a usage failure (TUI-002). It draws one empty frame with the header text and ends on `q` or `Ctrl-C`. Add the `TestBackend` helper. Add a test that a bare `anima` is exit 2.
- Acceptance: pass when `anima tui` on a pseudo-terminal draws a frame and `q` restores the terminal, `anima tui | cat` and `anima tui --json` exit 2 with the hint of TUI-002, a bare `anima` exits 2, and `anima mcp` lists the same tools as before; fail otherwise.
- Check: `cargo nextest run -p anima-cli`, `cargo clippy -p anima-cli --all-targets -- -D warnings`, `bun run gates`.
- Out of scope: any hub request. Any view.
- Always: the guard first, with a test that a panic restores the terminal. Ask: before a new dependency other than `ratatui`. Never: copy `raw_mode.rs`.
- Done: the commit that merges the skeleton and sets TU2 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task TU2 of docs/plans/cli-mcp-plan.md (TUI-001, TUI-002, TUI-005, TUI-030, TUI-071). Add ratatui 0.30, the folder crates/anima-cli/src/tui/ with the terminal guard (reuse raw_mode.rs and the signal code of anima ssh), the subcommand anima tui, the refusal off a terminal, one empty frame, q and Ctrl-C, a TestBackend helper, and a test that bare anima exits 2. No hub request and no view."

**TU3. State, actions and `update`.**
- Why: a pure state machine makes each later task testable with no terminal and no network (TUI-080).
- Needs: TU2.
- Change: add `state.rs`, `action.rs`, `keys.rs` and `update.rs`. The types hold devices, events, sessions, the view, the selection, the filter, the order, the connection state and the age of the data. `update` handles the keys of TUI-033, the filter, the order, the bounds of TUI-024 and the stale rule. It sends effects as values and does no input or output.
- Acceptance: pass when tests show, for each key and each state change, the new state and the effects; the filter works on the held list; the lists keep their bounds; and no function of the folder reads the clock or the network (the tick is an action); fail otherwise.
- Check: `cargo nextest run -p anima-cli tui`, `cargo clippy -p anima-cli --all-targets -- -D warnings`, `bun run quality:limits`.
- Out of scope: drawing. The hub.
- Always: write the test of a rule first. Ask: before a key that is not in TUI-033. Never: let `update` do input or output.
- Done: the commit that merges the state and sets TU3 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task TU3 of docs/plans/cli-mcp-plan.md (TUI-010, TUI-012, TUI-015, TUI-024, TUI-033, TUI-080). In crates/anima-cli/src/tui/ add state.rs, action.rs, keys.rs and update.rs: a pure update(state, action) that returns a new state and effects. Test each key and each bound. No drawing, no network, no clock."

**TU4. The feed.**
- Why: the screen is only useful when its data is live and survives a lost connection (TUI-020 to TUI-023, TUI-051).
- Needs: TU3.
- Change: add `feed/`. The first loads use the typed documents of `device list`, `event list`, `session list` and a new narrow document in `checks/` for the checks of a device that does not select `config` (TUI-011). Three subscriptions use `hub/subscription.rs`. A polling task reads the lists again at the intervals of OD-TUI6. A socket that closes opens again with the backoff of TUI-023 and then all lists are read again. A refused token (4403) ends all tasks and sends the action of TUI-051. Every task ends when the screen ends.
- Acceptance: pass when tests with a fake hub show: the first loads become actions; an event becomes an action; a closed socket opens again after 1, 2, 4, 8, 16, 30 and 30 seconds (with a fake clock) and then all lists are read again; a 4403 stops all tasks; and dropping the screen closes all sockets; fail otherwise.
- Check: `cargo nextest run -p anima-cli tui`, `bun run gates`.
- Out of scope: drawing. Any new network code outside `hub/`.
- Always: bound each channel and each list. Ask: before a K1 field is needed. Never: log or show a token; select `config` of a check.
- Done: the commit that merges the feed and sets TU4 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task TU4 of docs/plans/cli-mcp-plan.md (TUI-020 to TUI-024, TUI-051). In crates/anima-cli/src/tui/feed/ load devices, events, sessions and the checks of the selected device (a new graphql document in checks/ with no check config), keep them live with the deviceStatusChanged, eventAdded and sessionChanged subscriptions of hub/subscription.rs, poll at the intervals of OD-TUI6, reconnect with 1 to 30 second backoff and reread all lists after each reconnect, and stop on a 4403. Send Action values over one bounded channel. Test with a fake hub and a fake clock. Never log or show a token."

**TU5. The devices view and the detail pane.**
- Why: this is the screen that the person sees first (TUI-010, TUI-011, TUI-014, TUI-031, TUI-032, TUI-034).
- Needs: TU3.
- Change: add `view/header.rs`, `view/devices.rs`, `view/detail.rs` and the footer. Draw the wireframe of 11.4. Use only the 16 ANSI colours. Draw a status as a symbol and a word. Clean and cut each string. Hide the detail pane under 100 columns and show it on `Enter`. Draw the "too small", "empty" and "signed out" states.
- Acceptance: pass when `TestBackend` tests compare the buffer for: empty, loaded, selected, filtered, stale, signed out, too small, a long name and a hostile name (no escape sequence in the buffer), and with `NO_COLOR` no colour in the buffer; fail otherwise.
- Check: `cargo nextest run -p anima-cli tui`, `bun run quality:limits`, `bun run quality:names`.
- Out of scope: the events and sessions views. The hub.
- Always: draw from the state only. Ask: before a colour other than the 16. Never: write a hub string without `output::text::clean`.
- Done: the commit that merges the view and sets TU5 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task TU5 of docs/plans/cli-mcp-plan.md (TUI-010, TUI-011, TUI-014, TUI-031, TUI-032, TUI-034, TUI-081). In crates/anima-cli/src/tui/view/ draw the header, the footer, the devices list and the detail pane of the wireframe in Section 11.4 as pure functions of the state. 16 ANSI colours only, status as symbol and word, strings cleaned and cut, detail pane hidden under 100 columns. Test each state with TestBackend, including a hostile name."

**TU6. The events view.**
- Why: TUI-012.
- Needs: TU3, TU5 for the header and the footer.
- Change: add `view/events.rs`: the list at the top-new order, the severity filter, the limit to the selected device.
- Acceptance: pass when `TestBackend` tests show the order, the bound of 500, each severity filter and the device limit; fail otherwise.
- Check: `cargo nextest run -p anima-cli tui`, `bun run quality:limits`.
- Out of scope: the other views.
- Always: clean each message. Ask: before a key that is not in TUI-033. Never: draw a raw message.
- Done: the commit that merges the view and sets TU6 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task TU6 of docs/plans/cli-mcp-plan.md (TUI-012, TUI-034). Add crates/anima-cli/src/tui/view/events.rs: events newest first, at most 500, severity filter, limit to the selected device. Pure draw function, TestBackend tests, cleaned messages."

**TU7. The sessions view.**
- Why: TUI-013.
- Needs: TU3, TU5.
- Change: add `view/sessions.rs`: the open sessions with the device, protocol, operator name and the time since the start; it follows the `sessionChanged` actions.
- Acceptance: pass when `TestBackend` tests show a start, an end, an empty list and the bound of 200; fail otherwise.
- Check: `cargo nextest run -p anima-cli tui`, `bun run quality:limits`.
- Out of scope: ending a session (that is `anima session end`, a change).
- Always: clean each string. Ask: before any key that ends a session. Never: add one.
- Done: the commit that merges the view and sets TU7 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task TU7 of docs/plans/cli-mcp-plan.md (TUI-013). Add crates/anima-cli/src/tui/view/sessions.rs: open sessions with device, protocol, operator and time since the start, updated by the sessionChanged actions, at most 200. Pure draw function, TestBackend tests. No key that ends a session."

**TU8. The hand-off to `anima ssh`.**
- Why: this is the one use that goes beyond a view (TUI-040, TUI-041).
- Needs: TU5.
- Change: add `handoff.rs`. On `t`, if the login token is present: leave the alternate screen and restore the terminal, run `SshArgs::run` for the selected device in the same process, then enter the alternate screen again and read all lists again. If the token is missing or is an API key, show the footer line of TUI-040. If `anima ssh` fails, come back and show the failure and its hint.
- Acceptance: pass when a test with the test SSH server shows: `t` leaves and restores the terminal, the session runs and ends, and the screen returns with fresh lists; a missing token leaves the screen open with the footer line; a failure returns and shows the hint; the terminal mode is restored in all three cases; fail otherwise.
- Check: `cargo nextest run -p anima-cli tui handoff`, `cargo nextest run -p anima-cli end_to_end`, `bun run gates`.
- Out of scope: a terminal pane (TUI-042). A change to `anima ssh`.
- Always: restore first, then run. Ask: before any change to `sessions/terminal/`. Never: start a child process; read a credential of the device.
- Done: the commit that merges the hand-off and sets TU8 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task TU8 of docs/plans/cli-mcp-plan.md (TUI-040, TUI-041). Add crates/anima-cli/src/tui/handoff.rs: on the key t leave the alternate screen, restore the terminal, run the code of anima ssh for the selected device in the same process, then return, enter the screen again and reread the lists. Without a login token show the footer line and stay. If ssh fails, return and show the failure. Test with the test SSH server of tests/it/kit/tunnel_hub.rs. No child process."

**TU9. End-to-end tests and failure modes.**
- Why: the screen is only proven when it runs on a pseudo-terminal against a real hub (TUI-082, TUI-083).
- Needs: TU4, TU6, TU7, TU8.
- Change: add tests in `tests/it/end_to_end/` that start `anima tui` on the pseudo-terminal of `tests/it/kit/terminal.rs` against `RealHub`, find a device name, press `q` and prove that the mode is restored; refuse a token; close a socket and see the reconnect; send a hostile device name and prove that no escape sequence reaches the terminal; and check each row of the failure table of 11.6.
- Acceptance: pass when each row of the failure table has a test that passes, and the hostile name test fails when `output::text::clean` is removed; fail otherwise.
- Check: `cargo nextest run -p anima-cli end_to_end`, `bun run gates`.
- Out of scope: new features. A real terminal (TU10).
- Always: test against the real hub code, as the suite does. Ask: before a change to the test hub. Never: a test that needs a network outside the machine.
- Done: the commit that merges the tests and sets TU9 to Done.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task TU9 of docs/plans/cli-mcp-plan.md (TUI-082, TUI-083). In crates/anima-cli/tests/it/end_to_end/ start anima tui on the pseudo-terminal of tests/it/kit/terminal.rs against RealHub: a device name shows, q restores the mode, a refused token, a closed socket and its reconnect, a hostile device name with an escape sequence, and each row of the failure table of Section 11.6. Prove that the hostile test fails when output::text::clean is removed."

**TU10. Help, completion, the human check and the state.**
- Why: a person must find the screen and the operator must be able to check it on real terminals (TUI-083, PLN-014, PLN-019).
- Needs: TU9.
- Change: write the `--help` text and the `?` help screen (with the note about screen readers). Check that `anima completion` lists `tui`. Add the entry `HT-nnn` in need-human-testing.md (macOS Terminal, a Linux console over SSH, `tmux`, a resize, below 80 by 24, a hand-off). Compare the screen with the mockup and write the commit in the mockup index. Set the state of Section 11 and the header of this plan.
- Acceptance: pass when `anima tui --help` and the `?` screen exist, the completion lists `tui`, the entry exists with its steps, and the mockup row names the commit of the comparison; fail otherwise.
- Check: `bun run quality:documents`, `bun run gates`.
- Out of scope: a new feature.
- Always: take the next free `HT-` number on `main` at that time. Ask: before the state Done (the check must be Passed, PLN-009). Never: set Done before the operator sets the entry to Passed.
- Done: the commit that merges these changes. The task is Done after the operator sets the entry to Passed.
- Prompt: "Follow AGENTS.md 'Start of a session'. Claim and build task TU10 of docs/plans/cli-mcp-plan.md (TUI-083). Write the help of anima tui and the ? screen, check the completion, add the next free HT entry for real terminals to docs/need-human-testing.md, compare the screen with docs/mockup/terminal-ui/index.html and write the commit in docs/mockup/README.md, and update the state of Section 11."


## 12. Install into coding agents: `anima mcp install`

The operator, 2026-11-19, of the Settings section "MCP": "also would like an MCP install here that
install to every coding agent" (R-257). Today a person adds `anima mcp` to each agent by
hand. This section adds one command that does it, and one control in Settings that calls the same
library (CMD-030: the command line first).

### 12.1 Requirements

- **MCP-027.** `anima mcp agents` must list each coding agent that it knows, with one state:
  `not_installed` (the agent is not on this computer), `not_set_up`, `set_up` (an entry `anima` starts
  the `anima` of MCP-033), `different` (an entry `anima` is there and starts a different program or has
  different arguments) or `unreadable` (its file cannot be read as its format). An agent is on this
  computer when its program is on `PATH` or its configuration folder exists (12.2). The command reads
  only; it changes no file and starts no program of an agent that 12.2 sets up through a file.
- **MCP-028.** `anima mcp install` must set up each agent that is on this computer and is `not_set_up`
  or `different`. `--agent <id>` (repeatable) limits it to the agents named. `anima mcp uninstall`
  takes the entry out again, with the same `--agent`.
- **MCP-029.** `--dry-run` must print, for each agent, each file that the command would write with the
  backup it would make, or each command line it would run, and must change nothing. Without `--dry-run`
  and without `--yes` the command prints the same list and asks once before it does anything
  (CMD-024). With `--json` the list and the result are JSON.
- **MCP-030.** Where an agent is set up through a file, the command changes only the member `anima` of
  the servers object. Each other byte of the file stays as it was: comments, the order of the members,
  trailing commas, line ends and white space. A file that is not there is made with only the objects
  that the entry needs. The command never formats or sorts the file.
- **MCP-031.** A file is written through a temporary file in the same folder, which is then renamed
  over it, so a reader never sees half of it. The new file keeps the permission bits of the old one; a
  new file is `0600`.
- **MCP-032.** Before the first change of a file, the command keeps one copy of the file as it was,
  beside it, named `<file>.before-anima`. A copy that is already there is not replaced: it is the file
  as it was before Anima first changed it. `uninstall` leaves the copy.
- **MCP-033.** The entry starts `anima` by an absolute path with the one argument `mcp`. The path is
  the program that runs the command, or the value of `--command`. The command refuses a path that is
  not absolute, that is not a regular file that can be run, or whose file or folder a different user
  can write. The entry holds no environment variable, no token, no key and no hub address: `anima mcp`
  uses the context and the sign-in that the `anima` command already holds (MCP-002).
- **MCP-034.** The command must refuse a configuration file, and change nothing of it, when: the file
  or a folder of its path below the home folder is a symbolic link; the file is not a regular file; a
  different user owns it or can write it or its folder; it is larger than 1 MiB; or it cannot be read
  as its format (MCP-030). It says which agent and why, goes on with the other agents, and ends with a
  failure. It never follows a path that the file itself names.
- **MCP-035.** Where an agent has its own command to add and remove a server at user scope (12.2), the
  command runs that program with a fixed list of arguments and no shell. It runs the program that
  `PATH` gives, by its absolute path, and prints that path in the list of MCP-029. It passes no
  secret. It does not read or parse the agent's file.
- **MCP-036.** `uninstall` removes only an entry that is `set_up`: one that starts the `anima` of
  MCP-033. An entry `anima` that is `different` was not made by this program, or was changed by a
  person: `uninstall` leaves it and says so, unless `--force` is given. The objects that held the
  entry stay, also when they are then empty.
- **MCP-037.** The command changes no permission of a tool. The states of `anima mcp permissions`
  (MCP-012) already apply to each agent that starts `anima mcp` for the hub.
- **MCP-038.** An agent that starts `anima mcp` while the command line is not signed in gets no tool:
  the program ends before any protocol output with the failure of CMD-013 ("You are not signed in to
  ...") on standard error, as it does today (MCP-003). The install does not sign in and does not need
  a sign-in. On a hub that Anima.ing hosts, the sign-in of the standalone command line waits for
  OD-25; until then `anima mcp` works there with an API key in the token store, which the install
  never copies.
- **MCP-039.** The Settings section "MCP" of the desktop application must have a part "Coding agents"
  above the tool groups: one row for each agent that is on this computer, with its state in words, a
  button "Set up" or "Remove" for the row, and one button "Set up all". Before a change it shows the
  list of MCP-029 and a button that confirms. It reads and changes through shell commands that call
  the library of MCP-027 and MCP-028, as the permissions do (MCP-018); it starts no `anima` program.
  One sentence says: "This changes no permission: the tool groups below apply to every agent." When
  the command line tool is not installed (INS-002), the part says so and offers that step first
  (Settings, Updates), because the entry needs the path of the installed `anima`.
- **MCP-040.** Each way of 12.2 has a test that proves it against a temporary home folder: the bytes
  that stay, the copy, the refusal of a symbolic link, the dry run. No test and no gate reads or
  writes a real configuration of an agent. What only a real agent can show is one entry of
  [need-human-testing.md](../need-human-testing.md) (HT-062).

### 12.2 The agents and the way for each

Read from the documentation of each product on 2026-11-19. "File" means MCP-030 to MCP-034; "own
command" means MCP-035. A path is below the home folder.

| Id | Agent | On this computer when | Way | Place | Entry | Documentation |
| --- | --- | --- | --- | --- | --- | --- |
| `claude-code` | Claude Code | `claude` is on `PATH` | Own command: `claude mcp add --transport stdio --scope user anima -- <anima> mcp`; `claude mcp remove anima`; state from `claude mcp get anima` | `~/.claude.json` (the program's own file, which also holds account state: not written by Anima) | | https://code.claude.com/docs/en/mcp |
| `claude-desktop` | Claude Desktop | its folder exists | File | macOS `Library/Application Support/Claude/claude_desktop_config.json`, Linux `.config/Claude/claude_desktop_config.json`; key `mcpServers` | `{"command": <anima>, "args": ["mcp"]}` | https://modelcontextprotocol.io/docs/develop/connect-local-servers |
| `codex` | Codex CLI | `codex` is on `PATH` | Own command: `codex mcp add anima -- <anima> mcp`; `codex mcp remove anima`; state from `codex mcp get anima --json` | `~/.codex/config.toml` (TOML: not written by Anima) | | https://learn.chatgpt.com/docs/extend/mcp?surface=cli |
| `cursor` | Cursor | `.cursor` exists | File | `.cursor/mcp.json`; key `mcpServers` | `{"type": "stdio", "command": <anima>, "args": ["mcp"]}` | https://cursor.com/docs/context/mcp |
| `vscode` | VS Code, agent mode | `code` is on `PATH` or `.copilot` exists | File | `.copilot/mcp-config.json`; key `mcpServers` | `{"type": "stdio", "command": <anima>, "args": ["mcp"]}` | https://code.visualstudio.com/docs/agents/reference/mcp-configuration |
| `gemini` | Gemini CLI | `gemini` is on `PATH` | Own command: `gemini mcp add -s user anima <anima> mcp`; `gemini mcp remove -s user anima`; state from `gemini mcp list` | `~/.gemini/settings.json` | | https://geminicli.com/docs/tools/mcp-server/ |
| `opencode` | opencode | `opencode` is on `PATH` or `.config/opencode` exists | File (it has no command that removes) | `.config/opencode/opencode.jsonc` when it exists, else `opencode.json`; keys `mcp`, `servers` | `{"type": "local", "command": [<anima>, "mcp"]}` | https://opencode.ai/docs/mcp-servers/ |
| `zed` | Zed | `.config/zed` exists | File | `.config/zed/settings.json`; key `context_servers` | `{"command": <anima>, "args": ["mcp"], "env": {}}` | https://zed.dev/docs/ai/mcp |

Not in the first build, with the reason: Devin Desktop (formerly Windsurf) and Cline have their own
commands and can follow as rows; Amp documents no command that removes and uses the literal key
`amp.mcpServers`; Goose is YAML with no command for a script. What the documentation does not state:
how to know that an application is installed other than by its folder; the exact place of the user
scope in `~/.claude.json`; the path of the profile file of VS Code (so the file of Copilot is used);
whether the settings file of Zed is at that path on each system; whether opencode still reads the
earlier place `mcp.<name>`. Each is a line of the check HT-062.

### 12.3 Security

The command writes the configuration of other programs, and its entry makes those programs start
`anima`. The review of 12.5 (task MI5) must answer, with the code:

1. A hostile file that is already there: can it make the writer change bytes outside the member
   `anima`, write outside the file, or use more than a bounded amount of memory or stack? (MCP-030,
   MCP-034: a size limit, a depth limit, no path taken from the file.)
2. Symbolic links: a file or a folder of the path that points elsewhere (MCP-034), and the window
   between the check and the rename (MCP-031: the rename replaces the name and follows no link).
3. A configuration in a shared folder: a folder or a file that a different user can write (MCP-034).
4. The `anima` of the entry: a fixed absolute path that no different user can replace (MCP-033); what
   an agent starts when that file is later removed or replaced by an update.
5. The own command of an agent (MCP-035): which program runs, with which arguments, and what a
   hostile `PATH` could do; no shell.

### 12.4 Open decisions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| OD-MI1 | Does "Set up all" also replace an entry `anima` that a person wrote and that starts a different program (state `different`)? | Yes, after the list shows it as "replaces your entry", with the copy of MCP-032. `uninstall` leaves such an entry unless `--force`. | Open. |
| OD-MI2 | Which agents are in the first build: the eight of 12.2? | Yes. Devin Desktop and Cline follow when a person asks. | Open. |
| OD-MI3 | May the command run the own program of an agent (`claude`, `codex`, `gemini`) to set it up, or must it only write files? | Run the own program: it is what the documentation of each names, and Anima then never writes a file that holds account state. | Open. |
| OD-MI4 | Does the application set up the agents by itself at the first sign-in? | No. It writes the files of other programs: it happens only after a person selects the button and sees the list. | Open. |

### 12.5 Tasks (MI1 to MI6)

R-257 names MI1 to MI6. The builds follow the recommendations of 12.4 until the operator
decides differently.

| Task | Content | State | Commits |
| --- | --- | --- | --- |
| MI1 | The member editor for JSON with comments (MCP-030) and the safe file write with the copy (MCP-031, MCP-032, MCP-034), with their tests | In progress | `37d557ea` and `5b10f6a6` on the branch `mcp/install`: built and tested; reviewed (12.6) |
| MI2 | The agents of 12.2, their states (MCP-027) and `anima mcp agents`, `install`, `uninstall` with `--agent`, `--dry-run`, `--command`, `--force` (MCP-028, MCP-029, MCP-033, MCP-035, MCP-036), with tests against a temporary home folder (MCP-040) | In progress | `37d557ea` and `5b10f6a6` on the branch `mcp/install`: built and tested against a temporary home folder; not tried with a real agent (HT-062); reviewed (12.6) |
| MI3 | The mockup of the part "Coding agents" in [docs/mockup](../mockup/README.md) and on the screens canvas (PLN-019) | In progress | The mockup [mcp-agents/index.html](../mockup/mcp-agents/index.html) is drawn and is on the screens canvas; the operator did not see it |
| MI4 | The shell commands of the desktop application and the part "Coding agents" of the Settings section (MCP-039) | In progress | `3ed276d2` on the branch `mcp/install`: built, with tests of the client part and of the answer of the shell. Not seen: the part is in the desktop application only, and no desktop build was made. The shell commands were not in the review of 12.6 |
| MI5 | The security review of 12.3 by a reviewer that did not write the code; its findings and the corrections, before the merge | In progress | The review of the command line part is done and its corrections are `5b10f6a6` (12.6). The Settings part (MI4) is built and is not reviewed yet: the shell adds the caller check and takes the installed `anima` and the login `PATH` |
| MI6 | The entry HT-062: each agent of 12.2 on a real Mac and a real Linux computer | Not started | |

### 12.6 The security review of the command line part (2026-11-19)

A reviewer that did not write the code read it and tried it in a temporary folder: break attempts
and three million made-up inputs to the member editor. It changed no file of the repository and read
no real configuration. It found no panic, no read past the end, no unbounded stack or time, no path
taken from the content of a file, no secret written or printed, and that the rename follows no link.
Its findings and what was done (`5b10f6a6`):

| # | Finding | Done |
| --- | --- | --- |
| 1 | Taking out the member could join the next member of the same line to a comment of the line before: that member was then lost | Corrected, with a test |
| 2 | The number for `O_NOFOLLOW` was right for macOS and Linux x86_64 only | Corrected: the value of each system, from `rustix` |
| 3 | For an agent with its own command, the entry of a person was removed before the add, and the list did not show the removal | Corrected: the removal runs only for the state `different` and is a line of the list. Open: `claude mcp remove anima` names no scope (the documentation shows none); OD-MI1 decides if a replacement happens at all |
| 4 | The program of an agent ran in the folder of the caller and could read the project configuration there | Corrected: it runs in the home folder. Open: no time limit on that program |
| 5 | The check of the `anima` path looked at one folder only, not at the folders above it or at what a link points at, and took the owner of the home folder as the user | Corrected: each folder above the path and above the real file; the user is the user of the process; a home folder of a different user is refused. Open: a program that root owns passes, which is intended for a system install |
| 6 | Each file with a group write bit was refused, as the usual mask of Ubuntu and Fedora gives | Corrected: a group write bit is refused only when the group is not the private group of the user; a new folder is `0700` |
| 7 | The state of an agent with its own command was read by a search for the path as a part of the text | Partly: the path must be a whole word. Open: the other arguments of that entry are not read; the output of `claude mcp get` and `gemini mcp list` has no documented form |
| 8 | A key that is there twice: this program changed the first, a reader takes the last | Corrected: refused |
| 9 | Comments inside an empty object or beside the member were dropped; Windows line ends became mixed; a missing comma was accepted | Corrected, with tests. An object that held only space comes back as an empty object with no space |
| 10 | A link at the name of the copy made the write go on with no copy; a failed write left half a copy | Corrected: a link there is refused; the copy appears whole or not at all |
| 11 | The library took any path of `anima` unchecked | Corrected: it takes only a path that the check returned |
| 12 | A path with `$` or braces is a variable in the files of some agents | Corrected: refused, as is a path that is not text |
| 13 | A file that an agent saved between the read and the write was overwritten; under `sudo` the files became root's | Corrected: the file is read again before the rename and a change is refused |
| 14 | Notes | Open: a file that starts with a byte order mark is refused; the home folder itself is not checked; access lists are not read; the path of the running program differs by system (the Settings part must pass the installed path, MCP-039); the copy keeps the other entries of the file for as long as it stays |
