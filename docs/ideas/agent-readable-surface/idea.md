# Agent-Readable Surface

- From: the Anima Product Ideas bot, from competitor research, 2026-11-17
- Source: the vault note "Product Ideas from Research" (idea 1) and the private working note "Agent-Readable Surface"
- Status: Framed
- Serves: No row names it (gap). The CLI and the MCP server are not a category (priorities.md Section 3). Closest: row 4.16 One-click install (loose: that row is installing Anima.ing itself, which the prompt would drive; inferred, not confirmed).
- Owner if it becomes work: Open. Owner document for the CLI and MCP is `cli-mcp-plan.md` (ownership.md, CMD and MCP rows).
- Effort: S (estimate). Recommendation: should (the research verdict, not a decision)

## The idea

Publish a skill file, an `llms.txt` and a one-prompt install that a coding agent can follow, so an agent can set up and use Anima.ing without a human reading the docs.

## Evidence

- Competitor research, checked 2026-11-17 (vault note "Product Ideas from Research"): RustDesk publishes an `llms.txt`. NetBird offers one-click prompts for ChatGPT and Claude on a compare page. Herdr and Rex ship a skill file for agents. Muster installs when the user tells a coding agent to follow its `INSTALL.md`. The public pages are not re-checked here, and their dates are not verified.

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md) and [deferred.md](../../deferred.md).

- `ownership.md`: no concern covers an agent-readable surface; the CLI and MCP belong to `cli-mcp-plan.md` (CMD, MCP).
- register.md: no plan or proposal on a skill file or `llms.txt`.
- deferred.md: no item found (searched `llms.txt`, `skill file`, 2026-11-18).
- Related, already ruled: R-151 (2026-11-18) lets the CLI ship by Homebrew and cargo, which this idea's install prompt would use.
- Not checked: one hit for these words in `docs/research/code-quality/phase-2.md` was not read.
- Checked on 2026-11-18 against the repository at commit `866f667f`. This is a keyword search, not a full read.

## Narrowest version, and when to stop

- Narrowest version: One `INSTALL.md` and one skill file that tell an agent to install the `anima` CLI, run a check, and use the MCP tools; plus an `llms.txt` on the marketing site. Measure: a fresh agent session installs and lists machines with no human help.
- Stop if: a fresh agent cannot finish the install and a first query unaided in three tries, or if the CLI install channel is not shipped.

## Open questions

- Is the one-prompt install allowed to name the Homebrew and cargo channels (R-151) before they exist? (yes/no)
- Should the skill file live in the open-source repo rather than only on the site? (yes/no)

## Reviews

- 2026-11-18: round 1. The bot framed the idea from the research note. The Serves row is not confirmed. The operator has not picked this idea.

## Source text

The research note gives this table row (copied from the vault note "Product Ideas from Research"):

> | 1 | Agent-readable surface: a skill file, `llms.txt`, and a one-prompt install a coding agent can follow | S | Not planned (no skill file or `llms |
