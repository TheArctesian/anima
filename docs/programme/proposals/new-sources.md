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

## Spotify (Remember: a new timeline source)

Recently played tracks and top artists. The History page (XP-07, #18) shows them. History only
reads the timeline (HIS-002) and owns no source, so this source needs an owner: a new small
extension, or the Library extension (XP-13, #24). The operator decides.

- Not verified: the API gives only the last 50 plays. If so, the hub must read it frequently,
  and the full history comes from the data export as an upload source (ITG-005).
- Not verified: the OAuth scopes and the rules for a developer application.

## Screen Time (Remember: a new timeline source)

The same owner question as Spotify applies.

- macOS: the collector reads the local usage database. Not verified: the location and format of
  the database, and the permission that the collector needs to read it.
- iOS: no known method. Do not plan it until a method is verified.

## Needs

- One verification task for each provider, with a link to the provider documentation.
- A decision on the owner of the Spotify and Screen Time sources.
- Then: a change to the extensions plan, and a checklist line on #13, on the issue of the owner
  and on #31 (accounts and keys).
