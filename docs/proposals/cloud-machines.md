---
kind: Proposal
state: Draft
evidence: Not approved. CVM-001 to CVM-024. 2026-11-18: CVM-008 is marked for re-planning, because R-196 withdrew the overload warning AIS-017. 2026-11-20: the section "Supply" was added; it adds no requirement.
---

# Proposal: cloud computers and GPU add-ons

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the planning session of 2026-11-16. The operator asked to offer cloud VMs in a plan, so
a person without several devices can add a computer for a monthly fee, including a machine
with a good GPU (even an H100), as a wrapper around existing GPU rental, with a one-click
offer when a job can no longer run where it is.
Version: 2. Date: 2026-11-20. Version 2 adds the section "Supply (checked 2026-11-20)".
Status: the header of this document holds the state (PLN-020). History: Draft. The requirements are not yet approved. No ruling.
Map: [work-plan.md](../work-plan.md), category Use, and the rented-resource rules (ETH-006,
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

The rented computer installs the daemon on first boot, joins Headscale, and becomes an
ordinary device. The "this job cannot run here" offer belongs on the existing "Needs
attention" list or on the failed job. It names what is missing, suggests the operator's own
machines first, and is limited to one offer a week for the same cause. It must never pitch
more machines to a person who is already over the overload warning (AIS-017, withdrawn by R-196; see CVM-008).

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
| AWS, Azure, Google Cloud | Need a formal partner programme | Not a supplier (operator, 2026-11-20). Their prices are a comparison only. |

Anima.ing must not start a GPU add-on on a provider that forbids resale.

## Supply (checked 2026-11-20)

The operator said on 2026-11-20 to think about supply first: which supplier has the machines,
whether its terms allow resale, what it costs, and how many Anima.ing can get. This section
records what four research passes found on the suppliers' own pages on that day. It adds no
requirement. It does not change the prices of the section "Prices"; see "What changed" below.

Operator direction of the same day:

- AWS, Azure and Google Cloud are not to be suppliers. Their prices are a comparison only.
- Mac computers are left out of the offer for now. The findings are kept below.

### Three supply routes

1. **Rent and resell.** No money first, a thin margin, and the terms of the supplier must
   allow it.
2. **Run our own software on rented machines.** Databases, sandboxes and application hosting.
   The margin is here, and the cost is our own operations work.
3. **Own the hardware.** Found worth it only for Mac computers.

### The supply map

| Product | Supplier | Resale | Cost to Anima.ing | State |
| --- | --- | --- | --- | --- |
| Linux, small and always on | Hetzner Cloud CPX22 (2 vCPU, 4 GB) | Allowed by the standard terms | €16.99 to €19.99 a month | Ready |
| Linux, larger | Hetzner Cloud CCX33 (8 dedicated vCPU, 32 GB) | Allowed | €117.99 to €138.99 a month | Ready; stock not confirmed |
| Linux, for minutes or hours | Hetzner Cloud, by the hour, one hour minimum | Allowed | About €0.027 an hour for CPX22 | Ready; a new account has a low machine limit |
| Sandboxes | Hetzner dedicated servers, divided by the pool service | Allowed | From about €78.70 a month for a server | Our own work |
| PostgreSQL | PostgreSQL with pgBackRest, and Patroni for a standby, on Hetzner | The licence allows a paid hosted service | About €18 to €21 a month, or €38 to €45 with a standby | Our own work |
| Cache | Valkey on Hetzner | The licence (BSD-3) allows it | A part of a machine | Our own work |
| Object storage | Hetzner Object Storage | Allowed | €6.49 a month with 1 TB | Ready; 100 buckets for each account |
| More disk | Hetzner volumes | Allowed | About €0.057 a GB a month (not from Hetzner's own page) | Ready; Hetzner snapshots do not include volumes |
| Windows Server | Vultr ($16 a vCPU a month for the licence) or OVHcloud | Needs written permission (Vultr) or the partner programme (OVHcloud) | Machine and licence | Agreement first |
| GPU | RunPod or Hyperstack | Needs a signed reseller agreement | H100: $2.50 (Hyperstack) to $2.89 (RunPod) an hour | Agreement first |
| Mac | Own Mac mini computers in a data centre | They are ours; Apple must be told first | $899 to $1,800 one time, about $60 a month each | Left out for now |

### Who allows resale

| Kind | Allowed by the standard terms | Only with an agreement | Forbidden |
| --- | --- | --- | --- |
| Linux | Hetzner (§7.1 of its terms). DigitalOcean (§4.4, to confirm for whole machines) | Vultr, OVHcloud | Scaleway, Latitude.sh, AWS |
| GPU | DigitalOcean, Hetzner. Nebius has no ban; ask for it in writing | RunPod, CoreWeave, Hyperstack, GMI Cloud | Lambda, Vast.ai, Crusoe, Voltage Park, TensorDock, Together, SF Compute |
| Data services | Hetzner Object Storage | Neon, Tigris, Wasabi (platform or partner programmes) | Supabase, Upstash, Redis Cloud, Cloudflare R2 |
| Mac | — | Scaleway and Flow (no ban found; ask) | MacStadium |

Each supplier makes the reseller responsible for all that its customers do. Prepayment, a
card check and a spending cap (CVM-009, CVM-010, CVM-014) must be in place before the first sale.

### What changed since version 1

- **Hetzner is the only Linux supplier whose terms plainly allow resale.** Each "Ready" row of
  the map is on Hetzner. A second Linux supplier needs an agreement.
- **Hetzner raised prices on 1 April and 15 June 2026**, and its cheapest plans (CX, CAX) show
  as not available for new orders. The price of about $59 a month in "Prices" must be
  calculated again from today's figures before it is used.
- **A new Hetzner account has a machine limit.** Hetzner raises it only after one month and a
  paid invoice. Ask early.
- **Hetzner is not a supplier of GPUs by the hour**: a GPU server has a setup fee of about one
  month.
- **The margin of plain GPU resale is thin**: an estimate of 20 to 40% before costs and 10 to
  25% after card fees, idle minutes and fraud. This is an estimate of the research, not a
  figure from a supplier.
- **A small PostgreSQL costs Anima.ing about one third of the market price** ($60 to $145 a
  month elsewhere). The figure does not include our own engineering and on-call time.

### Software that must not be used for a hosted service

- MinIO community edition: the repository was archived on 2026-04-25 and gets no security
  corrections. Use Garage or SeaweedFS if Anima.ing runs object storage itself.
- Dragonfly and Redpanda: their licences forbid a hosted cache or queue service.
- Redis 7.4 to 7.8: the licence forbids it. Redis 8 allows it under AGPLv3; Valkey is simpler.

### Windows

A supplier can rent Windows Server with its licence. No supplier can supply a licence for
Windows 10 or 11 as a hosted machine. Anima.ing can offer Windows Server with the desktop,
not Windows 11. No page says in writing that the licence of a supplier covers the customers of
a reseller; get that in writing first.

### Mac (left out for now)

- Apple's macOS licence allows the lease of a whole Mac to one customer for at least 24 hours,
  for developer work only (builds, signing, tests). One Mac cannot be divided between
  customers. The lessor must tell Apple first, and each customer must accept the macOS licence.
- To own Mac mini computers in a data centre pays back in about 7 to 13 months against rent.
- MacStadium forbids resale. Hetzner no longer has Mac computers. Stock is low at small hosts.

### Recommended order of supply

1. Build the first offer from the rows "Ready" and "Our own work": Linux machines, sandboxes,
   PostgreSQL, cache and object storage, all on Hetzner.
2. At the same time start three agreements, because each takes weeks: a second Linux supplier
   (Vultr or OVHcloud), a GPU reseller agreement (RunPod or Hyperstack), and a higher machine
   limit at Hetzner.
3. Windows Server follows the second Linux supplier. Mac waits for a decision of the operator.

### Not verified

- No supplier has confirmed resale to Anima.ing in writing. For Hetzner this document relies
  on its published terms.
- Whether Hetzner's CCX plans can be ordered today: Hetzner's page and a third-party article
  disagree.
- Hetzner's volume price and its default machine limit.
- The commercial terms of each reseller programme; none is published.
- How often GPUs are sold out.
- Scaleway, Vultr (terms), Fluidstack and some other pages refused the research, so some of
  their figures are from search results.
- Euro and dollar figures were not converted.

### Sources

- Hetzner: [terms](https://www.hetzner.com/legal/terms-and-conditions/),
  [cloud prices](https://www.hetzner.com/cloud/regular-performance/),
  [price change](https://www.hetzner.com/pressroom/standardization-and-price-adjustment-of-our-server-products/),
  [object storage](https://www.hetzner.com/storage/object-storage/),
  [GPU servers](https://www.hetzner.com/dedicated-rootserver/matrix-gpu/).
- Linux: [Vultr use policy](https://www.vultr.com/legal/use-policy/),
  [Scaleway instance terms](https://www-uploads.scaleway.com/Conditions_Particulieres_Instances_v07042026_1db2d811d7.pdf),
  [Megaport agreement, used by Latitude.sh](https://www.megaport.com/legal/global-services-agreement/),
  [DigitalOcean terms](https://www.digitalocean.com/legal/terms-of-service-agreement).
- Windows: [Microsoft SPLA guidance](https://www.microsoft.com/licensing/guidance/SPLA),
  [Vultr Windows](https://www.vultr.com/servers/windows/).
- GPU: [RunPod terms](https://www.runpod.io/legal/terms-of-service),
  [RunPod prices](https://www.runpod.io/pricing),
  [Hyperstack prices](https://www.hyperstack.cloud/gpu-pricing),
  [NexGen Cloud channel programme](https://www.nexgencloud.com/channel-partnerships),
  [Nebius agreement](https://docs.nebius.com/legal/agreement),
  [Lambda terms](https://lambda.com/legal/terms-of-service), [Vast.ai terms](https://vast.ai/terms).
- Data services: [PostgreSQL licence](https://www.postgresql.org/about/licence/),
  [Redis licences](https://redis.io/legal/licenses/), [Valkey](https://github.com/valkey-io/valkey),
  [MinIO](https://github.com/minio/minio), [pgBackRest](https://github.com/pgbackrest/pgbackrest),
  [Tigris partner programme](https://www.tigrisdata.com/docs/partner-integrations/).
- Mac: [macOS licence](https://www.apple.com/legal/sla/docs/macOS27.pdf),
  [MacStadium terms](https://docs.macstadium.com/docs/billing-legal).

## Requirements

- **CVM-001.** Anima.ing may offer a rented computer as an add-on. The computer must install
  the daemon on first boot, join the customer's Headscale network, and become an ordinary
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
- **CVM-008.** Re-planning (R-196): the overload warning AIS-017 is withdrawn, so this requirement has no source now. The text was: "The offer must not appear when the overload warning (AIS-017) is already on." [NEEDS CLARIFICATION: what replaces the condition: a count of agents that wait for the operator from the idea `agent-state-hooks` after version 1 of [agent-tools-as-extensions.md](agent-tools-as-extensions.md) (OD-AXT10), a limit that the operator sets, or no condition.]
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

- **ETH-007:** For a rented resource, the hub only reads. An action uses the CLI tool of the
  service, on the operator's computer or in a job. Anima.ing's own systems would have to
  create and destroy these machines (CVM-015), not the customer's hub. That is a new kind of
  rented resource: Anima.ing is the service, not AWS in the customer's own account.
- **CMP-003:** Compute work must stay on the private network. Anima must not join a public
  compute network, for example c0mpute. A private rented machine on the customer's own
  network should not count (CVM-018), but the wording should say so.
- **MET-012:** Enrolment codes expire after 15 minutes and work only once. A slow GPU start
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
