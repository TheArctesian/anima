# Importers

- From: the Anima Product Ideas bot, from competitor research, 2026-11-17
- Source: the vault note "Product Ideas from Research" (idea 9) and the private working note "Importers"
- Status: Framed
- Serves: No row names it (gap). Closest: row 4.16 One-click install, first-run flow (inferred). Not confirmed.
- Owner if it becomes work: Open. Candidates: `first-run-setup.md`, `add-this-machine.md`.
- Effort: M (estimate). Recommendation: should (the research verdict, not a decision)

## The idea

Bring in an existing setup from `~/.ssh/config`, a Tailscale or Headscale tailnet, Termius or Termix hosts, Portainer endpoints and Uptime Kuma monitors.

## Evidence

- Competitor research, checked 2026-11-17 (vault note "Product Ideas from Research"): Termix is for people who leave Termius. Nexterm and ServerOS Panel import hosts or what runs on them. Uptime Kuma holds the monitors that many self-hosters already have. The public pages are not re-checked here, and their dates are not verified.

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md) and [deferred.md](../../deferred.md).

- ownership.md: machine profiles are owned by `machine-profiles.md` (REP); no importer concern.
- Machine profiles write an `~/.ssh/config` block (export) but nothing imports.
- register.md and deferred.md: no importer item found. Hits for Portainer in rulings.md R-149 concern a container UI, not an importer.
- Checked on 2026-11-18 against the repository at commit `866f667f`. This is a keyword search, not a full read.

## Narrowest version, and when to stop

- Narrowest version: Import hosts from `~/.ssh/config` and a tailnet list into the first-run "Add this machine" flow. Others later.
- Stop if: the first-run flow already gets five machines in under five minutes without it.

## Open questions

- Is the first version ssh config and tailnet only? (yes/no)
- Should the importer read the file on the machine only, with no upload? (yes/no)

## Reviews

- 2026-11-18: round 1. The bot framed the idea from the research note. The Serves row is not confirmed. The operator has not picked this idea.

## Source text

The research note gives this table row (copied from the vault note "Product Ideas from Research"):

> | 9 | Importers: bring in an existing setup from `~/.ssh/config`, a Tailscale or Headscale tailnet, Termius or Termix hosts, Portainer endpoints and Uptime Kuma monitors | M | Not planned |
