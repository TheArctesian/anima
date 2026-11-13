# Proposal: three new sources (Cloudflare, Spotify, Screen Time)

From: the planning session of 2026-11-12. The operator named these services in a side chat.
Status: a proposal. Nothing here is verified against the documentation of a provider. Do not
write requirements from this file until each "Not verified" line is checked.
Map: [work-plan.md](../../work-plan.md), categories See and Remember.

## Why a proposal and not a plan change

EXT-000 says to build only for the services that the operator uses, and to add a source when the
operator starts to use it. The operator uses all three. Each one then goes into the extensions
plan: a row in its Section 9 (services and keys), a provider folder in Section 5.2, and
requirements in the section of its extension. That change needs the approval of #26 or a ruling.

## Cloudflare (Projects, XP-02, #13)

A read-only source for zones, DNS records, Workers and Pages deployments, R2 usage and
analytics. PRJ-003 applies: an allow list of methods and paths.

- Not verified: the read-only token scopes for each of these.
- Not verified: which analytics the API gives without a paid plan.
- The operator already uses Cloudflare for the DNS of `anima.ing` and for R2 releases. The token
  for this source must be a different token, with read scopes only.

## A new extension: Activity (Remember)

History (XP-07, #18) only reads the timeline (HIS-002) and owns no source. The operator decided
on 2026-11-12 that Spotify and Screen Time go to an extension of their own. This file calls it
Activity: what the operator did on their devices. It gets the next free numbers: XP-15, and a
migration range and paths in the table of Section 5.2 of the extensions plan. History shows its
entries on the timeline.

### Spotify

Recently played tracks and top artists.

- Not verified: the API gives only the last 50 plays. If so, the hub must read it frequently,
  and the full history comes from the data export as an upload source (ITG-005).
- Not verified: the OAuth scopes and the rules for a developer application.

### Screen Time

- macOS: the collector reads the local usage database. Not verified: the location and format of
  the database, and the permission that the collector needs to read it.
- iOS: no known method. Do not plan it until a method is verified.

## Needs

- One verification task for each provider, with a link to the provider documentation.
- Then: a change to the extensions plan, a checklist line on #13 and #31 (accounts and keys),
  and one new issue for the Activity extension with the labels `extensions`, `life` and
  `priority: later`.
