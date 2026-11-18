# Needs You Inbox

- From: the Anima Product Ideas bot, from competitor research, 2026-11-17
- Source: the vault note "Product Ideas from Research" (idea 3) and the private working note "Needs You Inbox"
- Status: Framed
- Serves: Row 4.8 The other extensions (inferred; AIS-012 and AIS-017 are in `extensions-plan.md`), with the phone app in row 4.14 Mobile clients. Not confirmed.
- Owner if it becomes work: Open. AIS-012 and AIS-017 sit in `extensions-plan.md`; BOT-006 in `bots.md`.
- Effort: M (estimate). Recommendation: should (the research verdict, not a decision)

## The idea

An inbox where an agent's question can be answered from the phone, a message can be queued for a busy agent, and work carries on after a usage limit resets.

## Evidence

- Competitor research, checked 2026-11-17 (vault note "Product Ideas from Research"): Muster has an Asks page, shows Claude Code questions as forms on the phone, queues messages for a busy session, and resumes after a usage limit resets. Agent Manager answers any agent without attaching to it. cmux has notification rings. T3 Code has a thread inbox with snooze. The public pages are not re-checked here, and their dates are not verified.
- Pain in plain words: an agent stalls on one machine while the operator works on another. The repository already counts two to sixteen agents that need attention (AIS-017).

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md) and [deferred.md](../../deferred.md).

- ownership.md: AI coding sessions belong to the extensions plan; mobile belongs to `mobile-plan.md`.
- extensions-plan.md: AIS-012 sends one alert on a state change; AIS-017 warns when more agents need attention than one person can handle. No answer form, no queue.
- bots.md: BOT-006 asks before risky actions. BOT-047 to BOT-057 cover fall back to a local model after a limit.
- Constraint: CAP-025, AGN-014 and AIS-022 bar reading usage limits, so resume must infer the limit from terminal output.
- register.md and deferred.md: no inbox item found.
- Checked on 2026-11-18 against the repository at commit `866f667f`. This is a keyword search, not a full read.

## Narrowest version, and when to stop

- Narrowest version: Answer-from-phone for one agent (Claude Code): the alert of AIS-012 opens a form with the question and a reply box. Queue and resume come later.
- Stop if: agent terminal output cannot be parsed into a question reliably enough to show a form, or if the alert of AIS-012 is enough in a week of real use.

## Open questions

- Should the first version cover answering only, with no queue and no resume? (yes/no)
- Should the resume after a limit be dropped in favour of the local-model fallback (BOT-047 to BOT-057)? (yes/no)

## Reviews

- 2026-11-18: round 1. The bot framed the idea from the research note. The Serves row is not confirmed. The operator has not picked this idea.

## Source text

The research note gives this table row (copied from the vault note "Product Ideas from Research"):

> | 3 | A "needs you" inbox: answer an agent's question from the phone, queue a message for a busy agent, and carry on after a usage limit resets | M | Partial |
