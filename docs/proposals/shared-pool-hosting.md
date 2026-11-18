---
kind: Proposal
state: Draft
evidence: Not approved. Launch on the pool is decided (HM-R4, operator, 2026-11-16); the design is not. The AX102 list price was corrected on 2026-11-16 to €259 a month plus €129 setup (about $286).
---

# Shared Pool Hosting

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Proposal, 2026-11-16. The operator decided one point: hosted hubs launch on this pool, not on
one VPS for each customer (HM-R4 in [hosted-mode.md](hosted-mode.md), 2026-11-16). The design below
is not approved, and its figures are estimates. Instead of one dedicated server per customer, run each
customer's hub as an isolated instance on a few large shared servers (the pool), meter each hub's
CPU, memory and disk, show that to the customer as a live gauge, and sell more of the pool, or a
dedicated box, when a hub runs short. That replaces per-device pricing with load-based upgrades.
At pool prices a Solo hub costs about $4.80 a month in hardware before setup, disk, backups and
spare capacity, instead of $9.30. Solo at $19 still keeps a margin, but the margin is thinner
than the earlier $3 to $5 estimate. The price is real
operations work: isolation, shared networking and host upkeep become Anima.ing's problem.

All capacity figures below are estimates. The device figures are the founder's working numbers,
and hub memory is still unmeasured (see the open task in the vault's Hosting Costs note). The free hosted hub that sleeps (HM-R3, HM-R6) is in [hosted-mode.md](hosted-mode.md) and not repeated here.

## 1. What changes

| | Today's plan: one server per customer | Shared pool |
| --- | --- | --- |
| Unit | One VPS per hub (UpCloud 2 GB, about $9.30) | One isolated instance per hub on a large host |
| Host | Rented per customer | A few dedicated servers, for example Hetzner AX102 |
| Hardware cost per Solo hub | $9.30 to $13.90 | About $4.80 before extras (estimate, see section 5) |
| Growing a hub | Resize or replace the VPS | Change the instance's limits, or move it to a bigger slice |
| Who runs the isolation | The cloud provider | Anima.ing |

Each hub keeps its own full stack (hub, Postgres, guacd, localapi, apprise, Headscale), so the
product code does not need to become multi-tenant. Only the hosting layer is shared.

## 2. Metering and the live gauge

- A small daemon on each pool host reads each instance's usage every 10 seconds: CPU time against
  its allowance, memory in use against its limit, disk used against its quota, and relayed
  traffic against the 500 GB monthly allowance in the vault's Pricing note.
- The figures go to the control plane, which keeps a 30-day history for billing and support,
  and to the customer's hub, which stores them as ordinary series in its existing metrics store.
- The hub dashboard shows one "Your hub" card: CPU percent of slice, memory used and limit, disk
  used and quota, and relayed traffic this month, with a 7-day trend line.
- The figures are for the customer's own hub only; no customer sees another's numbers.

## 3. Upgrade prompts instead of per-device pricing

Prompts trigger on sustained load, not on device count:

| Signal | Prompt at | Hard limit behaviour |
| --- | --- | --- |
| CPU | Above 80% of the slice on average for 3 days | CPU is slowed, never stopped |
| Memory | Above 85% of the limit for 1 day | Soft limit first (slows the hub), hard limit only as a last resort |
| Disk | Above 80% of quota | Oldest raw metrics are thinned before anything else; never silent data loss |
| Relayed traffic | 80% of 500 GB | As in the vault's Pricing note: slowed, not cut off |

Each prompt names the cause ("your 34 devices are using most of your hub's CPU") and offers the
next step: a bigger slice, then a dedicated box. One prompt a week at most, and never a pop-up
during a remote session.

## 4. Isolation

| Option | Boundary | Fits Anima.ing because | Against |
| --- | --- | --- | --- |
| Containers with cgroup limits (Docker or Incus system containers) | Shared kernel; CPU weight and cap, memory limit, disk I/O limit | Lightest; easy to meter | A kernel bug or escape exposes every customer; Headscale and the Tailscale client want kernel networking features |
| Lightweight VMs (Firecracker, Cloud Hypervisor, Incus VMs) | Separate kernel per hub | Runs the existing Docker Compose stack unchanged; strong boundary; still metered through cgroups on the host | About 100 to 200 MB of memory overhead per hub (estimate); more tooling |
| Full VMs (KVM or Proxmox) | Separate kernel | What the vault's Hosting Costs note already assumes for shared hosts | Heaviest; slowest to resize |

Recommendation: one lightweight VM per hub, with cgroup limits on the host side as well, so a
noisy hub can only slow itself down. Incus is the first candidate to test, because it manages
both VMs and containers, sets CPU, memory and disk limits per instance and exposes per-instance
metrics. Keep a per-hub disk quota and per-hub I/O limits, since disk is the resource a busy
metrics store exhausts first.

## 5. Packaging and device capacity

Working capacity numbers: a shared CPU slice handles about 10 devices, one dedicated core about
100 to 200, and beyond that the hub needs a bigger instance (vertical scaling) at a higher cost.

Rough pool cost: an AX102 (16 cores, 128 GB) lists at €259 a month plus a €129 setup fee,
about $286 a month at 1.1052 USD/EUR (ECB reference rate of 13 November 2026), from
https://www.hetzner.com/dedicated-rootserver/ax102/. That is about $17.90 per core, or about
$4.80 per hub if one AX102 holds about 60 Solo hubs, limited by CPU, before setup, disk,
backups and spare capacity. The setup fee is about $143, or about $2.40 per hub once, about
$0.20 a month if spread over a year. The earlier figure of about $184 a month (about $3.10
per hub, from the vault's Hosting Costs note) is out of date. If a Solo slice is guaranteed a
quarter of a core (it can burst higher when the host is idle) plus 1.5 GB of memory, the
60-hub packing still holds as a working estimate, limited by CPU.

| Package | Slice | Devices it handles (estimate) | Price | Hardware cost (estimate) |
| --- | --- | --- | --- | --- |
| Solo | Shared, quarter core guaranteed, 1.5 GB, 20 GB disk | About 10 comfortably; the gauge decides beyond that | $19 a month or $199 a year | About $4.80 before extras |
| Solo Plus (name to decide) | One dedicated core, 4 GB, 60 GB disk | 100 to 200 | To decide; about $49 to $59 would keep a margin similar to Solo's | About $20 to $25 |
| Dedicated | Own server or large instance, scaled up | 200 and more | Custom (enterprise) | Quoted per customer |

- The Solo device cap stops being the gate. Keep 25 as a published guide ("built for about 10 to
  25 devices"), and let the gauge and prompts handle hubs that outgrow it.
- Enterprise starts where one dedicated core runs out (about 150 to 200 devices), or earlier for
  anyone needing single sign-on, audit logs, a support contract or a dedicated box for
  compliance.
- Team in the vault's Pricing note is about users, not load. A team hub is a second instance in the pool, and
  can be any slice size. Check the Team price against this before publishing it.
- This replaces the per-device price steps suggested on 2026-11-16 ($2 per device from 11 to 50,
  $1 from 51 to 150). The operator chose load-based upgrades on 2026-11-16, so the per-device steps
  are dropped.

## 5a. Enterprise sign-in, user provisioning and audit logs

Anima.ing will use WorkOS for enterprise single sign-on (SSO), SCIM directory sync and audit logs
rather than building them. WorkOS prices, from its pricing page on 2026-11-16:

| Item | Price |
| --- | --- |
| AuthKit (user sign-in) | Free up to 1 million monthly active users, then $2,500 a month per extra million |
| SSO | $125 a month per connection (one customer's identity provider), falling to $65 above 50 connections |
| Directory sync (SCIM) | $125 a month per connection, same volume steps as SSO |
| Audit logs | $125 a month per streaming connection, plus $99 a month per million events stored |
| Custom domain | $99 a month |

A first enterprise customer with both SSO and directory sync costs about $250 a month in WorkOS
fees, before audit logs. These fees must be built into the enterprise price, not absorbed by
the hosting margin.

## 6. Launch on the pool, and the fallback of one server for each customer

The operator decided on 2026-11-16 that hosted hubs launch on the pool (HM-R4). No hosted customer
exists, so no hub must move. The pool, its isolation and a security review must exist before the
launch.

1. **Make every hub portable from the first day.** Keep the data in restic backups on R2, keep
   the Headscale keys and the identity of the server in the backup, and reach each hub through its
   own subdomain.
2. **Build one pool host.** Rent one dedicated server, set up the isolation layer, run 5 to 10
   test hubs with simulated devices, and confirm the device numbers of each slice.
3. **Launch customers on the pool** when 10 test hubs have run for two weeks without a
   noisy-neighbour problem and the security review is done.
4. **Keep one server for each customer** (for example a VPS at UpCloud) as the Dedicated package
   and as the fallback for regions with no pool host (the US until a US host is priced).
5. **To move a hub between hosts, use a backup and a restore.** Stop the hub, take a final backup,
   restore it into the new instance, switch the subdomain, and check that every daemon connects
   again. The target is less than 10 minutes without service, inside a maintenance window that the
   customer knows.

## 7. Risks and open questions

- **Operations load.** Patching hosts, replacing failed disks, capacity planning and a host
  outage taking down 60 customers at once all land on a one-person company. A VPS for each customer
  (the fallback) hands most of that to the provider.
- **Shared networking.** Hubs on one host share IPv4 addresses. HTTPS can be routed by name, but
  Headscale's built-in relay needs UDP port 3478, which only one hub per address can have. Either
  run one shared relay per pool host or buy extra IPv4 addresses (about $2 each a month).
- **Blast radius.** One host failure affects every hub on it. Needs a restore-elsewhere plan and
  a stated recovery time in the vault's Support note.
- **Noisy neighbours.** CPU throttling is safe; memory and disk I/O are not. Limits must be
  tested under load, not assumed.
- **Capacity numbers are unmeasured.** Hub memory, devices per slice and disk growth per device
  all need a load test before any price goes on the site.
- **Provider terms.** Check that Hetzner and any US host allow reselling capacity as a hosted
  service (overlaps with the reselling task in the vault's Hosting Costs note).
- **Region.** The pool prices are for Hetzner in the EU; no US dedicated host is priced yet.
- **Security review.** A shared pool raises the stakes of an isolation escape; get a review
  before taking paying customers onto it.

## Open

#todo/shared-pool
- [x] Decide between one server per customer and the shared pool for launch (the shared pool, HM-R4, operator, 2026-11-16)
- [ ] Measure what a sleeping free hub costs and how long it takes to start (HM-D8 in [hosted-mode.md](hosted-mode.md))
- [ ] Get a security review of the pool's isolation and of the grant path before the first paying customer (HM-D7)
- [ ] Load test one hub to confirm devices per shared slice and per dedicated core
- [ ] Test Incus VMs with per-hub CPU, memory, disk and I/O limits on one dedicated server
- [ ] Decide how hubs on one host share Headscale's relay and IPv4 addresses
- [ ] Spec the "Your hub" usage card and the upgrade prompt wording
- [ ] Price and name the dedicated-core package, and check it against the Team price in the vault's Pricing note
- [ ] Write the backup-and-restore move procedure and time it

## Council follow-up: recovery evidence (2026-11-18)

R-152 names the preparation below. It changes no approved requirement or contract. Read current `main` and the claims before taking a task. A source review is not a Passed human check. The task owner records the tested build and the result; unknown results stay unknown.

| Task | Work and acceptance | Depends on | State |
| --- | --- | --- | --- |
| SP-C1 | Add a customer-side outcome to the proposed hosted recovery rehearsal: an already enrolled client reconnects, sees the expected records and opens a session to the same test target. Record backup cutoff, recovery time and operator intervention. Finding a restored database alone does not pass this outcome. Produce the rehearsal packet; do not run a recovery or allocate capacity. | Approved hosted restore design; working hosted session route; private pool owner and operator | Not started. Preparation only; the live rehearsal is blocked |
