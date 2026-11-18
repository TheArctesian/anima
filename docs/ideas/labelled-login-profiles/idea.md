# Labelled Login Profiles

- From: the Anima Product Ideas bot, from competitor research, 2026-11-17
- Source: the vault note "Product Ideas from Research" (idea 8) and the private working note "Labelled Login Profiles"
- Status: Framed
- Serves: No row names it (gap). The Bots proposal (`bots.md`) has no priorities row. Not confirmed.
- Owner if it becomes work: Open. `bots.md` (BOT-025, BOT-046, OD-BOT decision 8).
- Effort: M (estimate). Recommendation: should (the research verdict, not a decision)

## The idea

Let an operator choose which of their own accounts an agent uses on a machine, one login per labelled profile, never switched, pooled or read.

## Evidence

- Competitor research, checked 2026-11-17 (vault note "Product Ideas from Research"): Orca keeps several logins for each vendor and switches between them. T3 Code allows the folder settings `CLAUDE_CONFIG_DIR` and `CODEX_HOME`. The public pages are not re-checked here, and their dates are not verified. The vendor terms need a check before any build.

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md) and [deferred.md](../../deferred.md).

- ownership.md: no concern for logins.
- bots.md: BOT-025 and BOT-046 ban rotating, switching and pooling; a second login of the same vendor on one machine is open decision 8 and unruled. BOT-021 and BOT-032 constrain the Claude program.
- register.md and deferred.md: no item found.
- Terms: Anthropic allows the user's own login on the unmodified `claude` (see Agent Coverage vs Orca); needs the terms check before any build.
- Checked on 2026-11-18 against the repository at commit `866f667f`. This is a keyword search, not a full read.

## Narrowest version, and when to stop

- Narrowest version: One labelled profile per vendor per machine, chosen by the operator, using the vendor's own folder setting. No rotation.
- Stop if: decision 8 is ruled against, or if the vendor terms are read as banning a second login on one machine.

## Open questions

- Do you rule open decision 8 in favour of labelled profiles, one login each? (yes/no)
- Is the profile limited to Claude Code and Codex first? (yes/no)

## Reviews

- 2026-11-18: round 1. The bot framed the idea from the research note. The Serves row is not confirmed. The operator has not picked this idea.

## Source text

The research note gives this table row (copied from the vault note "Product Ideas from Research"):

> | 8 | Labelled login profiles: choose which of the operator's own accounts an agent uses on a machine, with no switching or pooling | M | Open |
