---
kind: Proposal
state: Draft
evidence: Not approved. CVM-001 to CVM-024.
---

# Proposal: cloud computers and GPU add-ons

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the planning session of 2026-11-16. The operator asked to offer cloud VMs in a plan, so
a person without several devices can add a computer for a monthly fee, including a machine
with a good GPU (even an H100), as a wrapper around existing GPU rental, with a one-click
offer when a job can no longer run where it is.
Version: 1. Date: 2026-11-16.
Status: Draft. The requirements are not yet approved. No ruling.
Map: [work-plan.md](../../work-plan.md), category Use, and the rented-resource rules (ETH-006,
ETH-007). It also serves jobs and the "Needs attention" list. A rented computer can take a
machine profile ([machine-profiles.md](machine-profiles.md)) and can keep a bot running when
the operator's own computer is off.
Identifiers: CVM-001 to CVM-024.

## Problem

Anima is built around machines the operator already has. A person with one laptop cannot add
a second computer in one click. When a job needs more memory, more disk or a larger GPU than
those machines have, the product has no next step inside Anima.ing.

People in industry already rent GPUs from AWS, Azure, Google or a neocloud. The product
should not pretend to be a new cloud. It should be a wrapper: one click, the machine joins
the operator's network, and it looks like any other device.

Bedrock and Azure Foundry rent models, not machines, so they are not the comparison. The
comparison is GPU rental, for example RunPod or Lambda, and ordinary cloud computers, for
example Hetzner Cloud.

## Recommendation

Offer the add-on in two parts:

1. **Monthly machines from Hetzner first.** Hetzner's terms already allow Anima.ing to let a
   customer use a machine that it rents. A mid-size cloud computer at about $59 a month is
   the first product.
2. **GPUs by the hour later**, and only after a written reseller agreement. A full-time H100
   costs a few thousand dollars a month if it stays on, so GPU add-ons must be billed by the
   hour with automatic shutdown, never as a flat monthly fee.

The rented computer installs the agent on first boot, joins Headscale, and becomes an
ordinary device. The "this job cannot run here" offer belongs on the existing "Needs
attention" list or on the failed job. It names what is missing, suggests the operator's own
machines first, and is limited to one offer a week for the same cause. It must never pitch
more machines to a person who is already over the overload warning (AIS-017).

## Prices (checked 2026-11-16)

Euro figures in this section use 1.1052 USD per EUR, the ECB reference rate of 13 November
2026.

### GPU rental, per GPU-hour

RunPod's Secure Cloud (its reliable data centres) is the cheapest source that this draft
uses for a resale comparison. Community Cloud is cheaper and less reliable; do not use it
as the cost basis. The RunPod page was updated 12 November 2026:
[runpod.io/pricing](https://www.runpod.io/pricing).

| GPU | RunPod Secure Cloud | Community Cloud | AWS (about) | Azure / Google |
| --- | --- | --- | --- | --- |
| RTX 4090 | $0.89 | $0.34 | — | — |
| L40S 48 GB | $1.09 | $0.79 | — | — |
| H100 80 GB | $2.89 (PCIe Secure Cloud) | $1.99 | about $6.88 per GPU-hour | Only in 8-GPU machines, about $11 to $12 per GPU-hour |

An H100 left on all month at $2.89 per hour is about $2,100 before Anima.ing's margin. That
is why it must not be sold as a monthly plan.

### Suggested Anima.ing prices, about cost plus 30%

| Add-on | Suggested price | How it is shown |
| --- | --- | --- |
| RTX 4090 | about $1.15 per hour | Hourly |
| L40S 48 GB | about $1.45 per hour | Hourly |
| H100 80 GB | about $3.80 per hour | Hourly, and "about $91 a day if you leave it on". Never as a monthly plan. |
| Hetzner cloud computer | about $59 a month | Monthly |
| 10 TB of storage | about $35 a month | Monthly. A Hetzner Storage Box BX31 is €20.80 a month (net), from [hetzner.com/storage/storage-box](https://www.hetzner.com/storage/storage-box/). |

These suggested prices are a starting point for a ruling. They are not published prices.

## Providers and resale

| Provider | Resale | Role in this draft |
| --- | --- | --- |
| Hetzner | Terms already allow Anima.ing to let a customer use a machine that it rents | First provider, for monthly computers and for storage |
| RunPod | Resale only with written consent, through its reseller programme | First candidate for GPUs, after that consent |
| Lambda | Bans reselling | Do not use |
| Vast.ai | Bans reselling | Do not use |
| AWS, Azure, Google Cloud | Need a formal partner programme | Later, if a programme is signed; not required for the first version |

Anima.ing must not start a GPU add-on on a provider that forbids resale.

## Requirements

- **CVM-001.** Anima.ing may offer a rented computer as an add-on. The computer must install
  the agent on first boot, join the customer's Headscale network, and become an ordinary
  device.
- **CVM-002.** Monthly always-on machines come first, from Hetzner.
- **CVM-003.** GPU machines come later, billed by the hour, and only after a written reseller
  agreement with a provider that permits resale.
- **CVM-004.** A GPU add-on must never be sold as a monthly plan. The client must show a
  daily cost if the machine stays on, for example "about $91 a day" for an H100 at the
  suggested price.
- **CVM-005.** When a job cannot run on the machines the customer already has, the "Needs
  attention" list or the failed job may offer a rented computer. The offer must name what is
  missing and compare it with a named existing machine, for example "needs 80 GB of video
  memory, studio has 24 GB".
- **CVM-006.** The offer must list the customer's own machines first (ETH-006). A rented
  computer is a later choice.
- **CVM-007.** At most one offer a week for the same cause. The offer is not a banner, and it
  must not appear during a remote session.
- **CVM-008.** The offer must not appear when the overload warning (AIS-017) is already on.
- **CVM-009.** GPU time is prepaid. A machine that would spend more than the prepaid balance
  must not start.
- **CVM-010.** The customer must set a spending cap before the first start of a billed
  machine.
- **CVM-011.** A GPU machine must stop after 30 minutes idle by default. The customer can
  change this limit.
- **CVM-012.** Removing a rented machine must wipe its disks. The customer must confirm the
  wipe.
- **CVM-013.** Mining and similar abuse are banned. A machine used for them must be stopped
  and removed.
- **CVM-014.** A card must pass a bank check (for example 3-D Secure or a similar issuer
  check) before the first GPU start.
- **CVM-015.** Anima.ing's own control plane, not the customer's hub, creates, starts, stops
  and destroys rented machines (see ETH-007 below). The hub only sees the machine as a
  device once it has joined. The control plane is the cloud service in the private repository
  `anima-site` on Vercel, with its API at `cloud.anima.ing` (operator, 2026-11-16; see fact 5 of
  [hosted-mode.md](hosted-mode.md)). Its design is the note "Hosted Architecture" of the business
  vault (HCL-001 to HCL-036). It is not approved and it is not in this repository.
- **CVM-016.** Enrolment of a rented machine must not depend on a 15-minute code alone
  (MET-012). Use a method that survives a slow start: a one-time join token with a longer
  limit that is bound to that machine, or a repeatable enrolment that the control plane
  drives.
- **CVM-017.** Optional storage, for example 10 TB, may be offered as a monthly add-on. It
  uses a Hetzner Storage Box or an equivalent, and is attached only to the customer's
  machines.
- **CVM-018.** A private machine that Anima.ing rents for one customer, and that joins only
  that customer's network, is not a "public compute network" under CMP-003. The wording of
  CMP-003 should say so. The machine must not join any other customer's network.

## Two ways to pay for cloud computers (operator direction, 2026-11-16)

The operator wants two options, like a model provider's flat plan next to its pay-per-use API:

1. **Pay per use.** The customer pays for each computer by the hour while it exists, from a
   prepaid balance, with the same cap and idle stop as GPUs. Hetzner's cloud servers are billed
   by the hour up to a monthly cap (to check against current Hetzner terms), so the cost
   follows use.
2. **A flat subscription.** One monthly price. The customer can start as many cloud computers as
   they want, within fair-use limits, and Anima.ing carries the risk that some customers use
   more than the price covers.

- **CVM-019.** Cloud computers (not GPUs) can be paid per use: billed by the hour from a prepaid
  balance, under the spending cap of CVM-010 and an idle stop that the customer sets.
- **CVM-020.** A flat cloud subscription may be offered. It allows any number of cloud computers
  of the sizes it names, within the fair-use limits of CVM-021. The price is to be set from the
  finance model (see Needs).
- **CVM-021.** The flat subscription must publish its fair-use limits in plain words before
  purchase: for example the most computers running at once, the largest size, and the total
  computer-hours a month (figures to decide). Past a limit, new computers start on pay per use
  only if the customer has turned that on; otherwise they wait. A running computer is never
  stopped mid-task for passing a limit; the next start waits instead.
- **CVM-022.** GPUs are never part of the flat subscription. They stay prepaid and hourly
  (CVM-003, CVM-004, CVM-009), because one H100 left on costs about $2,100 a month.
- **CVM-023.** The client must show the customer their use against the subscription's limits,
  in the same gauge style as hosted hubs ([shared-pool-hosting.md](shared-pool-hosting.md)), and
  suggest pay per use when they use little, or the subscription when pay per use would cost more.
  This suggestion follows the one-a-week rule of CVM-007.
- **CVM-024.** When a customer reaches their monthly pay-per-use spending cap (CVM-010,
  CVM-019), Anima.ing must not cut service off silently. Machines that are already running keep
  running to the end of their current task or until the customer stops them. Only new starts are
  held. The client must show a clear notice, framed as a security control and not as a billing
  error. It must say that a safety limit stopped further machine starts, and that the limit
  exists to protect the customer from surprise bills and from abuse of their account, for
  example a leaked key starting machines. The notice must give an explicit way forward: raise
  the cap, remove it, or contact support for help. Raising or removing the cap must need the
  same sign-in check as other account security changes. The same notice must also go by email or
  push to anyone who has turned on alerts, so a bot or job that was waiting on a start does not
  fail without anyone knowing.

Risks of the flat subscription:

- A few heavy users can cost more than the price. The fair-use limits and the start limit for
  new accounts are the control; Accounting must model how many heavy users the price can carry.
- "As many as you want" invites crypto mining and resale. CVM-013 still applies, and the
  subscription must forbid resale of the machines.
- The concurrency limit needs a hard check in the control plane (CVM-015), not only in the
  client.

## Conflicts that need a ruling

- **ETH-007.** For a rented resource, the hub only reads. An action uses the CLI tool of the
  service, on the operator's computer or in a job. Anima.ing's own systems would have to
  create and destroy these machines (CVM-015), not the customer's hub. That is a new kind of
  rented resource: Anima.ing is the service, not AWS in the customer's own account.
- **CMP-003.** Compute work must stay on the private network. Anima must not join a public
  compute network, for example c0mpute. A private rented machine on the customer's own
  network should not count (CVM-018), but the wording should say so.
- **MET-012.** Enrolment codes expire after 15 minutes and work only once. A slow GPU start
  can miss that window (CVM-016).

## Needs

- A ruling on ETH-007, CMP-003 and MET-012 as they apply here. The business vault proposes an
  answer to each: the cloud creates and deletes rented machines and the hub only reads
  (ETH-007), a machine rented for one customer joins only that customer's network (CMP-003), and
  the cloud triggers the enrolment code after the machine has booted (MET-012, HCL-011, HCL-012,
  HCL-018). These answers are proposed, not ruled.
- Written consent from a GPU provider before any GPU add-on is offered.
- A published price list, after the suggested prices are ruled on.
- A finance model for the flat subscription (price, fair-use limits, how many heavy users it can
  carry), from Accounting.
- After approval: a work item in `docs/work-items/` (PLN-012) in the Use category, and a security review of
  enrolment, wipe and spending caps.
