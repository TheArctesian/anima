# Agent State Hooks

- From: the Anima Product Ideas bot, from competitor research, 2026-11-17
- Source: the vault note "Product Ideas from Research" (idea 2) and the private working note "Agent State Hooks"
- Status: Framed
- Serves: Row 4.8 The other extensions (inferred: AI coding sessions are an extension in the Remember category; AIS-010 was in `extensions-plan.md` (R-196, 2026-11-18, withdrew AIS-009 to AIS-017; the agent panes are now the Herdr tool extension of [agent-tools-as-extensions.md](../../proposals/agent-tools-as-extensions.md))). Not confirmed.
- Owner if it becomes work: Open. AIS-010 is withdrawn (R-196). R-196 (OD-AXT10) says that state hooks come after version 1 of [agent-tools-as-extensions.md](../../proposals/agent-tools-as-extensions.md); [NEEDS CLARIFICATION: that proposal or a new one owns it].
- Effort: S (estimate). Recommendation: should (the research verdict, not a decision)

## The idea

One tiny protocol that any agent or script can use to report working, blocked or done, plus installable hook snippets for the top agents, so every agent is watched without an adapter per agent.

## Evidence

- Competitor research, checked 2026-11-17 (vault note "Product Ideas from Research"): Herdr has an open status protocol (OSC 7501) that any script can use, and Rex speaks it. Muxflow uses reviewable hooks for Codex, Claude Code and pi. Orca has a status adapter for 20 of its 34 agents (vault note "Agent Coverage vs Orca", 2026-11-17). Muster pushes one message for each stop and each question. The public pages are not re-checked here, and their dates are not verified.

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md) and [deferred.md](../../deferred.md).

- ownership.md: AI coding sessions belong to the extensions plan.
- extensions-plan.md (not one of the three files): AIS-010 (withdrawn by R-196) read state from Claude Code and Codex session files and from terminal activity; no hook protocol there.
- register.md and deferred.md: no hook protocol item found (the `hook` hits in deferred.md are a panic hook and a load-failure hook, unrelated).
- Constraint: AIS-022 bars reading a login, account name or usage limit.
- Checked on 2026-11-18 against the repository at commit `866f667f`. This is a keyword search, not a full read.

## Narrowest version, and when to stop

- Narrowest version: One documented status message and one hook snippet for Claude Code, read by the daemon next to the existing session-file and terminal-activity detection.
- Stop if: the screen of Herdr (which replaces AIS-010, R-196) already shows blocked and done correctly for Claude Code and Codex in testing.

## Open questions

- Should the first version be a protocol document only, with no hook snippets? (yes/no)
- Which agent gets the first snippet? (one choice: Claude Code / Codex)

## Reviews

- 2026-11-18: round 1. The bot framed the idea from the research note. The Serves row is not confirmed. The operator has not picked this idea.

## Source text

The research note gives this table row (copied from the vault note "Product Ideas from Research"):

> | 2 | Agent state hooks: one tiny protocol any agent or script can use to report working, blocked or done, plus installable hook snippets for the top agents | S | Partial |
