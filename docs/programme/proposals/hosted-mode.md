# Proposal: the hosted mode as the default first run

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the business planning session of 2026-11-16, after the operator tried release 0.2.0 on a
second Mac (report of the S12 packaging session).
Version: 2. Date: 2026-11-16. Version 2 adds HM-R6 (a free hub sleeps) and the cloud design of the
business vault (Section 3, fact 5).
Status: Draft. The direction is decided by the operator (Section 2). The requirements are not
approved.
Map: [work-plan.md](../../work-plan.md). Parent plans: [one-click-plan.md](../../one-click-plan.md)
Section 1.3 (the hosted mode) and item S6 (first run), and [other-operators.md](other-operators.md)
(OD-W2). The one-click work has no tier in Section 4 of the work plan; the operator must give it
one (see the notes below that table).

## 1. Purpose

A person installs Anima and signs in. They install nothing else. Anima makes a hub for the
account, and each computer where the person signs in joins that hub through the link.

The operator's words (2026-11-16):

1. "I want Tailscale bundled into the install or whatever else the user shouldn't have to install
   anything else also dangrous cuz if i tell them to install tailscale then they might just build
   their own"
2. "in theory I am equiv to a VPS user so for them I wouldn't mind building it in such that it
   works with just sign in. So I sign into my account and it signs up this infa can cordinate with
   the verecl site which I have planned to do SSO and all the rest of the sign up"
3. "The default behaviour should be the paid one lets figure that out first before we do self
   hosted support"

This proposal plans the hosted mode before the self-host mode (ruling HM-R1). It does not plan
the business: prices, costs and legal terms are in the business vault, and this file gives only
the facts that the build needs.

## 2. Operator rulings (2026-11-16)

| ID | Question | Ruling | Effect |
| --- | --- | --- | --- |
| HM-R1 | Which mode is the default on the first run? | The hosted mode. Plan it before self-host support. | "Sign in" is the primary choice of the first run (S6). |
| HM-R2 | Is a sign-in to the Anima.ing account sufficient for a new device to join the hub? | Yes. | This changes OR-5 of the one-click plan: for a hosted hub, the account directory gives access. Section 5 gives the risk and the controls. |
| HM-R3 | When does a person pay? | A small hosted hub is free below a usage line. A person pays above it. | Each account is a cost from its first minute. See HM-D1. |
| HM-R4 | What does a hosted hub run on at the launch? | The shared pool ([shared-pool-hosting.md](shared-pool-hosting.md)). | The pool must be built and reviewed before the launch. This replaces "one VPS for each customer" in Section 1.3 of the one-click plan. |
| HM-R5 | Does the launch wait for split custody (OI-3 of the one-click plan)? | No. Launch, then build split custody. | The sign-up page must say what a hosted hub can read (HMD-015). |
| HM-R6 | How does a free hub cost less than the paid hubs earn (HM-D1)? | A free hosted hub sleeps when no device of the account is connected. A hub that is always on is the paid step. | Section 6.5. The checks, the notifications and the metrics history stop while a free hub sleeps. |

## 3. Facts (2026-11-16)

1. The desktop application holds the hub, its PostgreSQL, the gateway and `anima-link`. Release
   0.2.0 is notarised, and the update in the application works on a second Mac.
2. The first run shows "Sign in" as not yet available, then "Join a hub" and "Create a hub on
   this computer".
3. The server stack cannot use the link. `deploy/compose.yaml` runs no `anima-link` service and
   the link mode of the hub is off. A hub on a server is thus available only through Headscale
   and the Tailscale client. A hosted hub needs this correction. The self-host mode needs the same
   correction, so the hosted work pays for most of it.
4. A hosted hub that uses only the link needs no Headscale. The pool proposal has Headscale in
   each instance, and one relay port for each address is one of its open problems. With the link,
   one relay for each pool host is sufficient.
5. No account directory, no billing and no pool exist. The site has only a waiting list. The
   design of the cloud is in the note "Hosted Architecture" of the business vault (HCL-001 to
   HCL-036, not approved, not in this repository). The operator decided three points of it: the
   cloud is the private repository `anima-site` on Vercel; its API and its sign-in pages are on
   `cloud.anima.ing`; the open-source client holds only the calls to it, with an override for a
   self-host person. That note has a table of jobs for long operations, one token for each hub,
   a heartbeat from the hub each 60 seconds and an idle stop for rented computers. It has no
   sign-in from the application, no device grant, no call that makes a hub, and no sleep. This
   proposal adds them. "The directory" in this file is that cloud.
6. The memory that one hub uses is not measured. All costs of a hosted hub are estimates.

## 4. The flow

### 4.1 First device

1. The person starts Anima. The first run shows "Sign in" as the primary button. "Join a hub"
   and "Create a hub on this computer" stay as secondary choices (S6 owns the layout).
2. The application makes a device key. It opens the system browser at the site with the public
   key and a one-time challenge. No secret is in the address.
3. The person signs up or signs in on the site (single sign-on).
4. If the account has no hub, the directory tells the pool to make one. The site shows the
   progress.
5. The site sends the person back to the application with a one-time code (a deep link).
6. The application changes the code for a device grant and the link address of the hub. The
   directory signs the grant. The grant names the account, the hub and the device key.
7. The application connects to the hub through the link. The hub checks the grant and adds the
   device key to its list.
8. The application shows the device list. The first step that it offers is "Add this computer".

### 4.2 Each other device

Steps 1 to 3 and 5 to 8 of Section 4.1. The hub is there already, so there is no step 4.

### 4.3 What the person never does

- Install Tailscale, Docker or a database.
- Type an invite, a server address or a relay address.
- Make a second password for the hub. The account is the operator account of a hosted hub.

## 5. The account as the key (HM-R2)

HM-R2 makes one sign-in sufficient. Thus a person who gets into the account gets into the hub,
and, until split custody is built, the hub can open a session to each device. Section 3.1 of the
one-click plan gave this as the lesson from Relay. The controls below decrease the risk. They do
not remove it.

| Control | Requirement |
| --- | --- |
| A second factor on each account | HMD-006 |
| A grant is for one device key, and it has a short life | HMD-004 |
| Each new device makes a notice on all joined devices and an email | HMD-007 |
| The owner can remove a device from any joined device | HMD-008 |
| The hub, not the directory, holds the list of device keys | HMD-005 |
| An optional setting: a joined device must approve each new device | HMD-009 |

The free mode and the self-host mode keep OR-5: no directory can give access to those hubs.

## 6. Proposed requirements

The prefix `HMD` is free on `main`. No requirement is approved. The cloud side of each requirement
is built in the private repository (fact 5); this repository holds the client and the hub side.

### 6.1 Account directory and site

- **HMD-001.** The site must let a person make an account with single sign-on.
- **HMD-002.** An account must own zero or more hubs. The directory must give the hubs of an account only to that account.
- **HMD-003.** When a new account has no hub, the directory must make one hosted hub for it without a step from the person.
- **HMD-004.** The directory must sign a device grant only for a signed-in account. A grant must name the account, the hub and one device key. A grant must stop after 5 minutes.
- **HMD-005.** A hosted hub must accept a device only with a valid grant for that hub. The hub must keep the list of its device keys (LNK-004).
- **HMD-006.** An account must have a second factor (a passkey, TOTP, or the second factor of the sign-on provider) before the directory signs a grant.
- **HMD-007.** When a device joins, the hub must send a notice to each joined device, and the directory must send an email to the account.
- **HMD-008.** The owner must be able to remove a device from each joined device and from the site. The hub must close the open sessions of that device.
- **HMD-009.** The owner must be able to set that a joined device approves each new device. The default is off (HM-R2).
- **HMD-010.** The directory must never hold a device credential, a session or a record of a hub.

### 6.2 Hosted hub

- **HMD-011.** The server stack must run `anima-link`, and the hub must run in link mode (Section 3, fact 3). A hosted hub must have no public port other than the relay.
- **HMD-012.** A hosted hub must run without Headscale and without the Tailscale client.
- **HMD-013.** The pool must make a hub in less than 60 seconds. The site and the application must show the progress.
- **HMD-014.** A hosted hub must have its own data, keys and backup. One hub must never read the data of a different hub.
- **HMD-015.** Before the first hub is made, the site must tell the person in plain words what a hosted hub can read (the table "What a hosted hub can read" of [other-operators.md](other-operators.md)).
- **HMD-016.** The person must be able to export the hub and to move it to a self-host hub or a hub on their computer.

### 6.3 Usage line and payment

- **HMD-017.** Each hosted hub must show its use against its limits (the "Your hub" card of [shared-pool-hosting.md](shared-pool-hosting.md) Section 2).
- **HMD-018.** Above the usage line, the hub must ask the person to pay. It must slow down; it must not delete data.
- **HMD-019.** Payment is on the site, through Stripe. The application must show the plan and open the site for a change.
- **HMD-020.** An account must have no more than one free hub.
- **HMD-021.** A free hub with no sign-in for 60 days must be paused. The directory must warn the person by email before it deletes a paused hub.

### 6.4 First run

- **HMD-022.** "Sign in" must be the primary choice of the first run. The other choices must stay available.
- **HMD-023.** The application must show one state for each step of Section 4.1, and a clear error with a retry for each failure.

### 6.5 Sleep and wake of a free hub (HM-R6)

- **HMD-024.** The pool must stop a free hub when no device of the account was connected for 15 minutes. A stopped hub must use disk only.
- **HMD-025.** When a person opens the application or signs in, the cloud must start the hub of the account. The start is a job (the job table of the cloud). The application must show "Your hub is starting" until the link connects.
- **HMD-026.** A free hub must start in less than 30 seconds. This number is a target; it is not measured.
- **HMD-027.** The cloud, not the hub, must decide to stop a hub, from the heartbeat of the hub and the state of the link. A hub that sends no heartbeat must be stopped by the pool.
- **HMD-028.** While a free hub sleeps, an agent on a device must wait and try again without an error to the person. The application must say in plain words that checks, notifications and history need a hub that is always on.
- **HMD-029.** A paid hub must never sleep. When a person pays, the hub must stay on from that moment without a move of data.

## 7. Open decisions

| ID | Decision | Facts | Proposal |
| --- | --- | --- | --- |
| HM-D1 | How can each free hub cost less than the paid hubs earn? | Estimates: a paid Solo hub earns about $13 a month after its hardware and the payment fee. A free always-on slice costs about $2 to $3. At a conversion of 1% to 3%, one paid hub carries 33 to 99 free hubs, so a free hub must cost less than about $0.13 to $0.40 a month. | Decided: HM-R6. The free hub sleeps. |
| HM-D8 | What does a hub that sleeps cost, and how fast does it start? | Not measured. The cost is the disk of the hub and its share of memory while it is on. | Measure on one pool host in step 5. Change the 15 minutes of HMD-024 and the 30 seconds of HMD-026 from the result. |
| HM-D2 | Where does the usage line sit? | Hub memory is not measured. | Set it after the load test of [shared-pool-hosting.md](shared-pool-hosting.md). |
| HM-D3 | Which identity service does the site use? | The operator planned single sign-on on the Vercel site. | The operator names it. HMD-006 must be possible with it. |
| HM-D4 | Is the second factor (HMD-006) required for each account? | Without it, one stolen sign-on session opens each device of a hub. | Required. |
| HM-D5 | In which region is the first pool host? | The priced hosts are in the EU. No US host is priced. | Price a US host before the launch. |
| HM-D6 | Is the directory and billing code open source? | OR-5: not open source. The operator decided the same for the cloud on 2026-11-16 (fact 5). | Decided. No change. |
| HM-D7 | Who reviews the security of the pool and of the grant path before the launch? | The pool proposal asks for a review before paying customers. HM-R2 adds the grant path. | A review by a person from outside, before the first account of a different person. |

## 8. Work, in order

No task is started. A task breakdown comes after the operator approves the requirements and
decides HM-D3 and HM-D4 (PLN-002).

| Step | Work | Depends on |
| --- | --- | --- |
| 1 | The link in the server stack (HMD-011, HMD-012). Also the base of the self-host mode. | Nothing |
| 2 | The load test of one hub: memory, devices for each slice | Step 1 |
| 3 | The account directory: accounts, hubs, grants (HMD-001 to HMD-010) | HM-D3, HM-D4 |
| 4 | The hub accepts grants; the hosted hub has no separate operator password (HMD-005). A spec delta: the login requirements and NET-001. | Step 3 |
| 5 | One pool host: isolation, limits, backup, the make-a-hub job, sleep and wake (HMD-013, HMD-014, HMD-024 to HMD-029, HM-D8) | Step 2, HM-D5 |
| 6 | The first run: "Sign in" (HMD-022, HMD-023) | Steps 3 and 4 |
| 7 | The usage card, the line, Stripe (HMD-017 to HMD-021) | HM-D1, HM-D2 |
| 8 | The security review (HM-D7), the terms and the text of HMD-015 | Steps 3 to 5 |
| 9 | Split custody (OI-3 of the one-click plan) | After the launch (HM-R5) |

## 9. Changes that this proposal needs in other documents

| Document | Change |
| --- | --- |
| [one-click-plan.md](../../one-click-plan.md) Section 1.3 | Hosted row: the shared pool, not one VPS for each customer (HM-R4). Done in the commit of this proposal, with a pointer to this file. |
| [one-click-plan.md](../../one-click-plan.md) OR-5 | The directory gives access to a hosted hub (HM-R2). Done in the same commit. |
| [shared-pool-hosting.md](shared-pool-hosting.md) | Remove Headscale from the instance stack; one relay for each pool host (Section 3, fact 4). Not done. |
| Design specification | A delta for the account login of a hosted hub, NET-001 and the first run. Not written. |
| S6 task breakdown | "Sign in" as the primary choice. Not done; S6 owns it. |
