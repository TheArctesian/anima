# Remote MCP over HTTP

- From: the Anima Product Ideas bot, from competitor research, 2026-11-17
- Source: the vault note "Product Ideas from Research" (idea 7) and the private working note "Remote MCP over HTTP"
- Status: Framed
- Serves: Row 4.17 The hosted mode and the shared pool (inferred; it relies on the hosted account, R-129). Not confirmed.
- Owner if it becomes work: Open. MCP-001 and MCP-002 sit in `cli-mcp-plan.md`.
- Effort: L (estimate). Recommendation: should (the research verdict, not a decision)

## The idea

A remote MCP server over HTTP with the Anima.ing sign-in and per-tool access levels, so cloud assistants can reach a user's machines.

## Evidence

- Competitor research, checked 2026-11-17 (vault note "Product Ideas from Research"): MCP is common among the competitors. ServerOS Panel meters it, LabTether has scoped keys, Everwas is read-only by default and logs each call, OpenRMM asks for approval on each action, and T3 Code exposes MCP. The public pages are not re-checked here, and their dates are not verified.
- Repository: MCP-001 says a remote HTTP server is not planned. The hosted account now exists (R-129).

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md) and [deferred.md](../../deferred.md).

- ownership.md: the CLI and the MCP server belong to `cli-mcp-plan.md`.
- cli-mcp-plan.md: MCP-001 says a remote HTTP server is not planned because it needs OAuth and a hub change. This idea reverses that line. MCP-002 already has allow, ask and deny per tool. R-120: MCP is fully automatic by default.
- Hosted account exists (R-129, 2026-11-17).
- register.md and deferred.md: no remote MCP item found.
- Checked on 2026-11-18 against the repository at commit `866f667f`. This is a keyword search, not a full read.

## Narrowest version, and when to stop

- Narrowest version: Read-only tools over HTTP for one signed-in user, behind OAuth with the existing Anima.ing account, with the audit trail of every call.
- Stop if: OAuth plus the hub change costs more than the hosted-hub launch can absorb, or if the assistants people use do not accept the server.

## Open questions

- Do you reverse MCP-001 for the hosted hub only? (yes/no)
- Should the first version be read-only tools? (yes/no)

## Reviews

- 2026-11-18: round 1. The bot framed the idea from the research note. The Serves row is not confirmed. The operator has not picked this idea.

## Source text

The research note gives this table row (copied from the vault note "Product Ideas from Research"):

> | 7 | Remote MCP over HTTP with the Anima.ing sign-in and per-tool access levels | L | Reverses a plan line |
