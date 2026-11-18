# Exposure Check

- From: the Anima Product Ideas bot, from competitor research, 2026-11-17
- Source: the vault note "Product Ideas from Research" (idea 5) and the private working note "Exposure Check"
- Status: Framed
- Serves: No row names it (gap). The CVE watch and network awareness proposal has no priorities row. Closest: row 4.8 (See; inferred). Not confirmed.
- Owner if it becomes work: Open. Candidate: `cve-watch-and-network-awareness.md` (VUL, ADR).
- Effort: M (estimate). Recommendation: should (the research verdict, not a decision)

## The idea

A read-only, local check of what on each machine is reachable from outside, with proof that Anima.ing opens nothing.

## Evidence

- Competitor research, checked 2026-11-17 (vault note "Product Ideas from Research"): ServerOS Panel does a read-only first scan. Pangolin, Cloudflare Tunnel and Tailscale show access rules. Public videos ask whether a homelab is exposed (for example DB Tech, "Is Your Homelab Exposed? How to Check for Free"). The public pages are not re-checked here, and their dates are not verified.

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md) and [deferred.md](../../deferred.md).

- ownership.md: no concern for listening ports; the CVE watch proposal owns VUL and ADR.
- cve-watch-and-network-awareness.md: VUL finds known vulnerabilities; ADR reads interface addresses. No listening-port list.
- register.md and deferred.md: no item found.
- Not checked in detail: `design-spec.md`, `vpn.md` and `projects-and-builds-plan.md` mention exposure or listening ports.
- Checked on 2026-11-18 against the repository at commit `866f667f`. This is a keyword search, not a full read.

## Narrowest version, and when to stop

- Narrowest version: Daemon lists listening ports and which are bound beyond loopback or the private network; the client shows a pass or fail line. No scanning from outside.
- Stop if: the daemon cannot list listening ports on both Mac and Linux without a new permission or a command run (the daemon runs no commands today).

## Open questions

- Is the check limited to the machine itself, with no outside scan? (yes/no)
- Should it ship with the CVE watch rather than alone? (yes/no)

## Reviews

- 2026-11-18: round 1. The bot framed the idea from the research note. The Serves row is not confirmed. The operator has not picked this idea.

## Source text

The research note gives this table row (copied from the vault note "Product Ideas from Research"):

> | 5 | An exposure check: what on each machine is reachable from outside, and proof that Anima.ing opens nothing | M | Partial |
