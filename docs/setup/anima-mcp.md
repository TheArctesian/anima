# Use Anima from a coding agent (`anima mcp`)

`anima mcp` is an MCP server on standard input and output (docs/cli-mcp-plan.md Section 7,
MCP-001 to MCP-017). A coding agent starts it and calls its tools. It uses the same hub and
token as the `anima` command line.

## 1. Make an API key

Give the agent an API key, not your login. Make a read key first:

```sh
# On your computer, signed in with `anima auth login`.
anima key create --name coding-agent --scope read --expires 90d
```

The key is shown once. Do not paste it into chat or a commit.

## 2. Register the server

Claude Code:

```sh
# On your computer. Type the key; do not copy it from chat.
claude mcp add anima --env ANIMA_TOKEN=<API_KEY> -- anima mcp
```

Another agent: run the command `anima mcp` with `ANIMA_TOKEN` in its environment. Add
`--hub <ORIGIN>` or `--context <NAME>` when the hub is not your current context.

At start the server checks the token with the hub. A bad token stops it before the session
starts, with the usual exit code and hint.

## 3. What the agent may do

Each tool has one state:

- `allow`: the agent may call it.
- `ask`: the agent may call it, and you approve each call in your MCP client.
- `off`: the agent does not see it, and a call is refused.

With nothing saved, every read tool is `allow` and every change tool is `off`. The tool sets
are `devices`, `checks`, `maintenance`, `events`, `sessions`, `notifications`, `web`,
`releases`, `vpn` and `retention`.

Save the states once for each hub:

```sh
anima mcp permissions show                      # one row per tool; --json for programs
anima mcp permissions set --ask-all --kind change   # every change tool asks first
anima mcp permissions set --allow anima_check_update,anima_check_set_paused
anima mcp permissions set --off vpn,retention   # a name is a tool or a tool set
anima mcp permissions set --allow-all --toolset checks --yes
anima mcp permissions set --read-only           # every change tool off again
anima mcp permissions reset                     # back to the defaults
```

`--allow-all`, `--ask-all` and `--off-all` take `--toolset <set>` and `--kind read|change`.
`--allow-all` that covers change tools asks you to confirm, or takes `--yes`. A bulk choice
without `--toolset` changes the default of the kind, so a tool added in a later version follows
it; a choice by name stays with that tool.

`ask` needs a client that can ask you (it declares MCP elicitation; check your client). The server
then shows you the tool and its arguments and waits up to 120 seconds. If the client cannot
ask, or you decline, the call is refused and the hub is not contacted. A delete still needs its
`confirm` name.

Flags at start apply to one run and only narrow what is saved: `--toolsets a,b` turns off every
tool outside those sets, and `--read-only` turns off every change tool. `--allow-write` (or
`--allow-write checks,maintenance`) allows the change tools that you have not saved a choice
for. A tool you saved as `off` or `ask` stays so.

The hub token is still the final limit:

- A read key cannot change anything. A change tool then fails with a message that says so.
- Some tools need a login token, because an API key may never do them (INT-003): device update
  and delete, release create, build cancel, VPN route approval and retention set. The server
  never lists them for an API key, whatever is saved.

## 4. What no tool does

No tool takes or returns a secret: credentials, API keys, VPN keys, notification channel
addresses, push check paths. No tool accepts a host key or a certificate, opens a session, or
runs a command on a device. Do these in the Anima client or with the command line.

Text from devices (names, event messages, log lines) comes back cleaned and capped, in named
fields. The server tells the agent never to follow it as instructions.
