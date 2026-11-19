# Observability suite: status pages and error tracking beside the uptime checks, and what to leave out

- From: the Anima Marketing Bot, relayed on 2026-11-19 (14:15 PT). It reports that the operator replied "ya we should also be doing this" to a public post. That reply is relayed, not first-hand in this record.
- Source: the vault note "Observability Suite"
- Status: Framed
- Serves: not known. No row of `docs/plan/priorities.md` names it. Closest: row 4.1 (the device manager owns uptime checks and INT-004). Not confirmed. Open question 4 asks for a new row.
- Owner if it becomes work: Open. `ownership.md` gives checks, metrics and notifications to design-spec.md (CHK, MET, NOT, INT). The Projects extension (`extensions-plan.md` Section 4.2) owns the read-only analytics view. A new proposal is probable.
- Effort: an estimate for each piece is in "Pieces and effort". Recommendation: should for status pages, could for a narrow error tracker, skip for product analytics and web analytics.

## The idea

A public post says that a single small program could replace several hosted tools: product analytics, error tracking, uptime monitoring and web analytics. The question for Anima.ing is which of those pieces belong beside the uptime checks and metrics it already has, in what order, and in which program. Public status pages and an error tracker for the programs that run on the operator's machines fit best. Product analytics and web analytics fit least: the plan already reads them from other services and rules out a telemetry endpoint of its own.

## Evidence

- Public post by the founder of Coolify, 2026-11-19, 12:17 UTC (04:17 PT), read through the X connector on 2026-11-19 (14:20 PT): "I am working on a Posthog, Sentry, Better Stack, Plausible, Dub (etc) alternative that is just one binary on a few MB of RAM and backed by our open source philosophy. Dogfooding internally soon and then you could have it as well. Maybe its time to accelerate the first public version." (https://x.com/heyandras/status/2123390206464209146). It quotes a post of 2026-11-18 (02:24 UTC) that asks if someone can "reverse engineer and open source posthog". At read time: about 16,000 views, 527 likes, 75 replies. The replies were not read.
- The post is a teaser. It names no product, repository, licence or date. It does not say whether the product is a Coolify project. Marketing's note says the post names four products; it names five, with "Dub" and "(etc)".
- Design specification Section 1.1 gives "Monitoring" as one of the three primary functions, and Section 1.2 names Uptime Kuma as the reference for uptime checks, status history and notifications. Checked 2026-11-19 at commit `7c8ece8d`.
- Related idea from competitor research (2026-11-19, from a release of ServerOS Panel dated 2026-11-18): start a coding-agent task from a down monitor, a failed job or a machine's logs. It has no brief yet.

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md), [deferred.md](../../deferred.md), [plan/priorities.md](../../plan/priorities.md), design-spec.md and the plans. Keyword search, not a full read. Checked 2026-11-19 at commit `7c8ece8d`.

| Piece | What the repository already has | What is new |
| --- | --- | --- |
| Uptime checks | Built or planned: CHK-001 to CHK-014 (Section 6.4: ICMP, TCP, TLS, HTTP(S), DNS, push, group, retries, maintenance, restart loops), maintenance windows (Section 6.5), NOT-001 to NOT-008 (webhook, ntfy, email, Apprise), INT-001 (the Prometheus endpoint `/metrics`), WDG-035 (a watch is a check). Row 4.1; the watchdog is row 4.19 | Nothing. Do not rebuild |
| Status pages | INT-004 only: "A later version can add public status pages, incidents and status badges". No row, no plan and no item in `deferred.md` | The pages, incidents and badges. Conflict to rule: SEC-008, NET-003, DNS-005, HMD-011 (the hub is not on the public internet; a hosted hub has no public port but the relay), SITE-006 and SITE-007 (the website does not link to a hub and is static). A public page must be a static export or a separate public surface, not the hub |
| Error tracking | Nothing for application errors. Neighbours: LOG-001 to LOG-028 (the hub's own logs), HOST-015 and HOST-016 (the log of a container, row 4.9), CHK-014 and WDG-001 to WDG-041 (restart loops and watches, row 4.19), the shared redaction crate of E0-3 (R-166). LOG-007 (no secret in a log) applies to anything stored | An error list for the programs on the machines: capture, grouping, a notification, a link to the log. A decision on whether to accept events from the Sentry SDKs |
| Product analytics | Read only: PRJ-009, PRJ-012, PRJ-015 and OD-E6 (`extensions-plan.md` Section 4.2, row 4.6, XP-02) read PostHog each hour. OD-E6: "No custom telemetry endpoint" | A store of events and funnels of its own. This reverses OD-E6 |
| Web analytics | Read only: PRJ-009 reads Vercel Web Analytics. SITE-008 and OD-21 allow Vercel Web Analytics on the Anima.ing website and nothing else | A page-view counter and a public collector. This reverses OD-E6 and needs a public endpoint |

- `register.md`: no plan or proposal for status pages, error tracking or analytics. `deferred.md`: nothing (searched uptime, status page, analytics, Sentry, incident, monitor).
- Rules to respect: EXT-000 (build only for what the operator uses) and EXT-005 (the foundation works with every extension off), so any new piece is switchable.
- Related ideas: the "fix with an agent" idea above (its input would be a down check or an error); Needs You Inbox (alerts to the operator); Public Demo Hub (a public surface); Agent Tools as Extensions (row 4.21, the extension host).
- Not checked: the text of every plan; whether a status page can be a static export that the existing website publishes; what the Coolify product will be.

## Pieces and effort

Effort is an estimate (S about a week, M one to three weeks, L more than three weeks, for one engineer).

| Piece | Effort (estimate) | Fit | Note |
| --- | --- | --- | --- |
| Status pages, incidents, badges | M for a static export; L if it needs a public surface | Closest | Builds on checks that exist. The public route needs a ruling |
| Error tracking, narrow (errors from watches, containers and logs; grouping; notification) | M | Good | Uses data that Anima.ing already reads |
| Error tracking, full (accept Sentry SDK events, releases, source maps) | L | Medium | Needs a public or relayed ingest route |
| Product analytics | L | Poor | PostHog is already read; reverses OD-E6 |
| Web analytics | M | Poor | Needs a public collector; reverses OD-E6 |

## Narrowest version, and when to stop

- Narrowest version: one public status page for a chosen set of checks, written as static files that the operator publishes. It shows the current state and the last 90 days of uptime from the existing aggregate tables (CHK-011). No new binary, no public port. Measure: the operator publishes a page and a visitor sees a real outage that the checks found.
- Stop if: a public status page needs the hub on the public internet (SEC-008), or the first build needs a new program next to `anima-daemon` and the hub.

## Open questions

1. Which pieces are in scope? (one choice: status pages and a narrow error tracker; status pages only; all four pieces. Recommended: status pages and a narrow error tracker.)
2. In what order? (one choice: status pages first, then error tracking; error tracking first; both together. Recommended: status pages first.)
3. Where does it ship? (one choice: inside the existing hub and daemon as extensions that are off by default; a separate binary. Recommended: inside the hub and daemon.)
4. Which priority row does it serve? (one choice: a new row of its own that the Planner adds; under row 4.1; under row 4.8. Recommended: a new row.)

## Reviews

- 2026-11-19: round 1. The bot framed the idea from a relayed message, the public post and an overlap check. No answers yet.

## Source text

Relayed by the Anima Marketing Bot, 2026-11-19 (14:15 PT), with no change except that the operator's name is replaced by "[the operator]":

> New direction from [the operator] (2026-11-19). Coolify's founder Andras Bacsai teased a single-binary open-source app that replaces PostHog, Sentry, Better Stack and Plausible: https://x.com/heyandras/status/2123390206464209146. I flagged it as a possible overlap with our uptime and metrics, and [the operator] replied "ya we should also be doing this".
>
> Please take it as a roadmap item. That means scoping which pieces fit Anima.ing's device-manager core. Better Stack-style uptime and status pages seem the closest fit, followed by error tracking for apps on your machines. Product analytics and web analytics seem furthest.
