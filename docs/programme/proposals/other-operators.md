# Proposal: Anima for other persons (self-hosting and a hosted option)

From: the planning session of 2026-11-12.
Status: a direction that the operator decided (OD-W2). It has no requirements yet and no work is
planned. It changes text in two plans that say Anima is for one person only.
Map: [work-plan.md](../../work-plan.md), Section 1 and principles ETH-001, ETH-002 and ETH-009.

## Decision (OD-W2, operator, 2026-11-12)

The operator's words: "All cuz we can monetise it as well with hosting it for them but also ways
for them to host it on whatever service they want so self host option with a hosted option for
whoever wants it and we can charge it like SASS coolify model".

| Subject | Decided |
| --- | --- |
| Self-hosting | A person can run their own hub on a service of their choice. |
| Hosted option | A paid service runs a hub for a person who wants that. |
| Model | As Coolify: the same software, free to host yourself, paid when it is hosted for you. |
| One hub for each person | Not changed (ETH-001). A hosted hub is one hub for one person, not one hub that many persons share. |

Anima is for the operator first. Nothing in this file moves work in the priority order.

## What does not agree with this today

| Text | Says | Change that it needs |
| --- | --- | --- |
| Extensions plan 1.1 | "Anima has one user: the operator. It is not a product for other people." | New wording. The operator approves it. |
| EXT-000, CMP-001 | Build only for the services, devices and models that the operator uses. No general catalogue. | Stays for what is built. ETH-009 adds: a different person must be able to add a source without a change to the engine. |
| Design specification 1.4 | "Multiple operators with different permissions" is out of scope. | Stays. Each person has their own hub. |
| SEC-008, NET-003 | The hub must not be on the public internet. | A hosted hub needs a decision: each person joins a VPN of their hub, or the hosted hub has a public address with more protection. |
| OD-9, design specification 4.2 | The hub is on the Hostinger VPS, with `anima.ing` names. | Call it the reference setup (ETH-002). No code must assume it. |

## What a hosted hub can read

A service that runs a hub for a person can read what that hub can read: the device credentials
(SEC-003, SEC-004) and Class B and Class C data. It cannot read Class A data, which is sealed on
the client (VLT-001), or bulk data that the client sealed
([bulk-data-placement.md](bulk-data-placement.md)).

This is a cost against the purpose line "in one place that you own". The hosted option must say
it to the person in plain words. A later design can decrease it, for example with a key that
only the person holds.

## Rules to obey now, so that later work is not blocked

These are not new work. They are checks on work that is done for other reasons.

- Do not write a name, an address or an account of the operator into code. Use configuration
  (ETH-002).
- Keep provider-specific code behind the source registry of the engine (K-E1), so that a person
  with different services adds a source and changes nothing else (ETH-009).
- Each developer application (OAuth) in Section 9.2 of the extensions plan is registered to the
  operator. A different person needs their own, or the hosted service supplies one.

## Open points

- The licence. The workspace is `AGPL-3.0-or-later`. It permits a paid hosted service by the
  operator, and it requires each other host to publish their changes. Confirm that this is the
  intent.
- Who operates the hosted service, and where. It holds credentials of other persons, so it needs
  its own security review and its own legal terms.
- The VPN for a hosted person: Headscale for each person, or one shared control server.
- Billing, sign-up and support. None is designed.
- The first-run setup proposal ([first-run-setup.md](first-run-setup.md)) and in-app enrolment
  ([in-app-enrolment.md](in-app-enrolment.md)) are the first steps that a different person needs.

## Needs

- A ruling number in `programme.md`.
- Approval of the new wording for the extensions plan 1.1 (with #26).
- No issue yet. Make one with the label `priority: later` when the operator wants to start.
