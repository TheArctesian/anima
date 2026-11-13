# Use Anima from a coding agent (`anima mcp`)

`anima mcp` is an MCP server on standard input and output (docs/cli-mcp-plan.md Section 7,
MCP-001 to MCP-011). A coding agent starts it and calls its tools. It uses the same hub and
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

The server is read-only by default. It offers tool sets: `devices`, `checks`, `maintenance`,
`events`, `notifications`, `web`, `releases`, `vpn`, `retention`.

Save what the agent may do, once for each hub:

```sh
anima mcp permissions set --toolsets devices,checks,events --write checks
anima mcp permissions show
anima mcp permissions set --read-only        # no change tools again
```

Flags at start override the saved values for one run: `--toolsets a,b`, `--allow-write` (every
set), `--allow-write checks,maintenance` (only those sets), `--read-only`.

The hub token is still the final limit:

- A read key cannot change anything. A change tool then fails with a message that says so.
- Some tools need a login token, because an API key may never do them (INT-003): device update
  and delete, release create, build cancel, VPN route approval and retention set. The server
  lists them only when it runs with your login token and their set may change things.

## 4. What no tool does

No tool takes or returns a secret: credentials, API keys, VPN keys, notification channel
addresses, push check paths. No tool accepts a host key or a certificate, opens a session, or
runs a command on a device. Do these in the Anima client or with the command line.

Text from devices (names, event messages, log lines) comes back cleaned and capped, in named
fields. The server tells the agent never to follow it as instructions.
