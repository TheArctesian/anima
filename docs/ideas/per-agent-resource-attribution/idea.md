# Per-Agent Resource Attribution

- From: the Anima Product Ideas bot, from competitor research, 2026-11-17
- Source: the vault note "Product Ideas from Research" (idea 6) and the private working note "Per-Agent Resource Attribution"
- Status: Framed
- Serves: Row 4.8 The other extensions (inferred; joins AU-1 to AU-4 usage counts and machine metrics). Not confirmed.
- Owner if it becomes work: Open. AU-1 to AU-4 sit in `extensions-plan.md`.
- Effort: S to M (estimate). Recommendation: should (the research verdict, not a decision)

## The idea

Show which agent session is using the CPU, memory and disk on each machine.

## Evidence

- Competitor research, checked 2026-11-17 (vault note "Product Ideas from Research"): Netdata and Beszel show load for each process or container but do not know about agents. T3 Code shows only its own memory, and users report memory growth. The public pages are not re-checked here, and their dates are not verified.

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md) and [deferred.md](../../deferred.md).

- ownership.md: metrics belong to the design spec; AI coding sessions to the extensions plan.
- extensions-plan.md: AU-1 to AU-4 count prompts, tool calls and tokens, not resources.
- register.md and deferred.md: no per-process item found.
- Feeds placement (JOB-013 in `jobs.md`).
- Checked on 2026-11-18 against the repository at commit `866f667f`. This is a keyword search, not a full read.

## Narrowest version, and when to stop

- Narrowest version: Per-process CPU and memory for the process tree of each detected agent session on one machine, shown on the device page.
- Stop if: the daemon cannot link a process to an agent session without reading session files beyond AIS-022's limits.

## Open questions

- Is a per-process reading allowed in the daemon's push batch? (yes/no)
- Should it cover the agent process tree only, not all processes? (yes/no)

## Reviews

- 2026-11-18: round 1. The bot framed the idea from the research note. The Serves row is not confirmed. The operator has not picked this idea.

## Source text

The research note gives this table row (copied from the vault note "Product Ideas from Research"):

> | 6 | Per-agent resource attribution: which agent session is using the CPU, memory and disk on each machine | S to M | Not planned |
