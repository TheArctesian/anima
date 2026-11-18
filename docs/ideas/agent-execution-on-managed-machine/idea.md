# Agent Here, Execution on a Managed Machine

- From: the Anima Product Ideas bot, from competitor research, 2026-11-17
- Source: the vault note "Product Ideas from Research" (idea 4) and the private working note "Agent Here, Execution on a Managed Machine"
- Status: Framed
- Serves: No row names it (gap). Closest: row 4.7 Jobs, builds and AI agent runs (inferred); the cloud machines proposal is `cloud-machines.md`. Not confirmed.
- Owner if it becomes work: Open. Candidates: `bots.md` (BOT-010), `agent-runs.md` (AGN), `cloud-machines.md`.
- Effort: L (estimate). Recommendation: should (the research verdict, not a decision)

## The idea

A bot option where the agent runs on the user's own machine but executes commands on a managed machine, with an allow-listed network and API keys the agent cannot see.

## Evidence

- Competitor research, checked 2026-11-17 (vault note "Product Ideas from Research"): Conductor, Cloudroom, Omnara, cmux and Dots sell a version of an agent that runs on the user's computer and runs code somewhere else. The public pages are not re-checked here, and their dates are not verified.
- A blog post of Simon Willison, "External sandboxes for coding agents" (2026-11-17), is about the same pattern. The research note did not read the post. The design of an allow-listed network and hidden keys is the note's reading of the usual pattern, not his.

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md) and [deferred.md](../../deferred.md).

- ownership.md: AI agent runs belong to `agent-runs.md`; cloud machines to `cloud-machines.md` (proposal).
- bots.md: BOT-010 gives a bot its own computer. agent-runs.md: AGN-003 and AGN-004 (no coordinator credentials, a tool allow list).
- No network allow list or key hiding in any of them. deferred.md line 774 mentions an "MV-008 allow-list" that does not exist; it is a different subject and was not read in detail.
- register.md: `cloud-machines.md` is a proposal, not approved (state not rechecked 2026-11-18).
- Checked on 2026-11-18 against the repository at commit `866f667f`. This is a keyword search, not a full read.

## Narrowest version, and when to stop

- Narrowest version: One bot on one managed machine with a command log and no network access except a short allow list. Prove that a prompt-injected agent cannot reach the user's keys.
- Stop if: the cloud machines proposal is not approved, or if a test shows a key cannot be hidden from the agent on the machine.

## Open questions

- Should this wait until the cloud machines proposal is approved? (yes/no)
- Is a network allow list required in version one? (yes/no)

## Reviews

- 2026-11-18: round 1. The bot framed the idea from the research note. The Serves row is not confirmed. The operator has not picked this idea.

## Source text

The research note gives this table row (copied from the vault note "Product Ideas from Research"):

> | 4 | "Agent here, execution on a managed machine": a bot option that runs commands on a managed machine with an allow-listed network and hidden API keys | L | Partial |
