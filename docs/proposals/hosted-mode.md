---
kind: Proposal
state: Direction decided
evidence: Direction decided (HM-R1 to HM-R9, operator, 2026-11-16). R-127 names task HM-T1 (the first-run screens of Sign in, with a stand-in cloud). R-128 approves HMD-037 (the first run is only the sign-in). HM-T1 is built on branch hosted-sign-in-screens, not merged; the cloud API and the grant check of the hub are not built, and the real cloud of the client is a stub. HMD-001 to HMD-036 are not approved. HM-R6: a free hub sleeps. HM-R7: Apple Pay and Google Pay. HM-R8: no staff access to hubs. HM-R9: WorkOS AuthKit (decides HM-D3). HM-D4 (second factor) is proposed, not ruled.
---

# Proposal: the hosted mode as the default first run

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the business planning session of 2026-11-16, after the operator tried release 0.2.0 on a
second Mac (report of the S12 packaging session).
Version: 5. Date: 2026-11-17. Version 2 added HM-R6 (a free hub sleeps) and the cloud design of the
business vault (Section 3, fact 5). Version 3 added HM-R7 (Apple Pay and Google Pay), HM-R8 (no
staff access to hubs), HMD-030 and HMD-031. Version 4 adds HM-R9 (WorkOS AuthKit, which decides
HM-D3), the device-code sign-in of Section 4.1 and HMD-032 to HMD-036. HM-D4 is not ruled.
Version 5 adds the task table of Section 8, task HM-T1 (R-127) and HMD-037 (R-128).
Status: the header of this document holds the state (PLN-020). History: Draft. The direction is decided by the operator (Section 2). The requirements are not
approved.
Map: [work-plan.md](../work-plan.md). Parent plans: [one-click-plan.md](../plans/one-click/one-click-plan.md)
Section 1.3 (the hosted mode) and item S6 (first run), and [other-operators.md](other-operators.md)
(OD-W2). This proposal serves row 4.17 of [priorities.md](../plan/priorities.md): tier 1
(operator, 2026-11-16).

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
| HM-R7 | Which payment methods does the checkout take? | Apple Pay and Google Pay on mobile devices, as well as cards (operator, 2026-11-16). Stripe charges a wallet payment the standard card rate. | HMD-030. The business vault note "Hosted Architecture" (HCL-032 to HCL-036) has the facts. |
| HM-R8 | May Anima.ing staff log into a customer's hub for support? | No. Hub access is offered only on a plan with a support tier that includes it, for example Enterprise, after the customer opts in (operator, 2026-11-16). | HMD-031. |
| HM-R9 | Which identity service does the sign-in use (HM-D3)? | WorkOS AuthKit (operator, 2026-11-16: "lets get the single sign on with workos"). Sign-in is the default; self-host stays the other choice. | Section 4.1 and Section 6.6. The application signs in with the device authorization flow of AuthKit. |

## 3. Facts (2026-11-16)

1. The desktop application holds the hub, its PostgreSQL, the gateway and `anima-link`. On
   2026-11-16 the operator saw the update in the application work on a second Mac. The release
   record in [releases.md](../procedures/releases.md) holds what is released.
2. The first run shows "Sign in" as the primary choice, then "Join a hub" and "Create a hub on
   this computer" (task HM-T1). The screens of "Sign in" work only with a stand-in cloud
   (`VITE_ANIMA_HOSTED=stand-in`). Without it, the client calls a stub that refuses each call, so
   the first run says "Sign-in refused".
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
7. WorkOS AuthKit, read on 2026-11-16 from <https://workos.com/pricing> and
   <https://workos.com/docs/authkit/cli-auth>:
   - It is free for the first 1,000,000 monthly active users, then $2,500 for each further
     1,000,000. Enterprise single sign-on and directory sync are $125 for each connection a month
     (1 to 15 connections). A staging environment is free.
   - It includes email and password, social sign-in, passkeys, multi-factor authentication, magic
     auth and enterprise single sign-on. The list of social providers, TOTP and how to require
     multi-factor authentication for each user are not checked.
   - For an application that cannot get a browser redirect, AuthKit gives the device
     authorization flow (RFC 8628): the application sends only its client ID, shows a code,
     and polls until the person signs in. The response holds an access token and a refresh token.
     The application must replace the stored refresh token after each exchange. The lives of the
     tokens and how to end the sessions of a user are not checked.
   - A custom domain for the sign-in pages is $99 a month. What it covers is not checked.

## 4. The flow

### 4.1 First device

1. The person starts Anima. The first run shows "Sign in" as the primary button. "Join a hub"
   and "Create a hub on this computer" stay as secondary choices (S6 owns the layout).
2. The application makes a device key. It starts the device authorization flow of AuthKit
   (HM-R9) and opens the system browser at the sign-in page. It shows the same short code, so the
   person can check that the browser page belongs to this sign-in.
3. The person signs up or signs in with AuthKit: email, Google, GitHub or a passkey (the
   providers are not decided), with the second factor of HMD-006.
4. The application polls AuthKit until the sign-in is complete, and receives a WorkOS access
   token and refresh token. It keeps the refresh token in the key store of the system (HMD-034).
5. The application sends the access token and the public device key to the cloud
   (`cloud.anima.ing`). The cloud checks the token with WorkOS. If the account has no hub, the
   cloud starts the make-a-hub job; if the free hub sleeps, it starts the wake job. The
   application shows the progress (HMD-023).
6. The cloud signs a device grant and returns it with the link address of the hub. The grant
   names the account, the hub and the device key. No deep link and no custom address scheme are
   necessary.
7. The application connects to the hub through the link. The hub checks the grant and adds the
   device key to its list.
8. The application shows the device list. The first step that it offers is "Add this computer".

### 4.2 Each other device

Steps 1 to 3 and 5 to 8 of Section 4.1. The hub is there already, so there is no step 4.

### 4.3 What the person never does

- Install Tailscale, Docker or a database.
- Type an invite, a server address or a relay address.
- Make a second password for the hub. The account is the operator account of a hosted hub.

R-128 (operator, 2026-11-17: "It should just be signing into your account") makes this section a
requirement:

- **HMD-037.** On a device of a hosted hub, the first run must be only the sign-in to the account. After "Sign in", the application must not show or ask for an invite, a hub address, a relay address, or a hub password or TOTP code, and must not send the person to "Join a hub" or to the hub sign-in page. After the sign-in, one answer of the cloud must give all that the application needs to join the hub and to use it as a signed-in device. The application must join with it and must not show it. Approved (R-128).

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
- **HMD-019.** Payment is on the site, through Stripe. The application must show the plan and open the site for a change. The checkout takes Apple Pay and Google Pay (HMD-030).
- **HMD-020.** An account must have no more than one free hub.
- **HMD-021.** A free hub with no sign-in for 60 days must be paused. The directory must warn the person by email before it deletes a paused hub.
- **HMD-030.** On a mobile device, the checkout must offer Apple Pay and Google Pay first, and a normal card form when no wallet is available (HM-R7). The page of the checkout must be on `cloud.anima.ing`. Card and wallet details must never reach the directory or its logs. Only Stripe identifiers may.
- **HMD-031.** The staff tools of the directory must show account, billing and job data only. Staff must not open a session on a customer's hub or read its content (HM-R8). Access to a hub is allowed only on a plan with a support tier that includes it, after the customer opts in, and the hub must write each access to its audit log.

### 6.4 First run

- **HMD-022.** "Sign in" must be the primary choice of the first run. The other choices must stay available.
- **HMD-023.** The application must show one state for each step of Section 4.1, and a clear error with a retry for each failure.

### 6.5 Sleep and wake of a free hub (HM-R6)

- **HMD-024.** The pool must stop a free hub when no device of the account was connected for 15 minutes. A stopped hub must use disk only.
- **HMD-025.** When a person opens the application or signs in, the cloud must start the hub of the account. The start is a job (the job table of the cloud). The application must show "Your hub is starting" until the link connects.
- **HMD-026.** A free hub must start in less than 30 seconds. This number is a target; it is not measured.
- **HMD-027.** The cloud, not the hub, must decide to stop a hub, from the heartbeat of the hub and the state of the link. A hub that sends no heartbeat must be stopped by the pool.
- **HMD-028.** While a free hub sleeps, a daemon on a device must wait and try again without an error to the person. The application must say in plain words that checks, notifications and history need a hub that is always on.
- **HMD-029.** A paid hub must never sleep. When a person pays, the hub must stay on from that moment without a move of data.

### 6.6 Sign-in with WorkOS AuthKit (HM-R9)

- **HMD-032.** The application must sign in with the device authorization flow of WorkOS AuthKit, as a public client. The application must hold only the client ID: no WorkOS API key and no client secret.
- **HMD-033.** The cloud must accept a WorkOS access token only after it checks the signature, the issuer, the audience and the expiry. The cloud must find the account by the WorkOS user ID, not by the email address.
- **HMD-034.** The application must keep the WorkOS refresh token in the key store of the system, never in a file or a log (LOG-007). It must replace the stored token after each exchange, and it must delete the tokens when WorkOS refuses one.
- **HMD-035.** The WorkOS API key must be only in the private cloud (HCL-014). The open-source repository must hold only the client ID and the `ANIMA_CLOUD_URL` setting (HCL-002).
- **HMD-036.** When the owner removes a device (HMD-008) or signs out on the site, the cloud must end the WorkOS sessions of that device and refuse its refresh token. How WorkOS ends one session is not checked.

## 7. Open decisions

| ID | Decision | Facts | Proposal |
| --- | --- | --- | --- |
| HM-D1 | How can each free hub cost less than the paid hubs earn? | Estimates: a paid Solo hub earns about $13 a month after its hardware and the payment fee. A free always-on slice costs about $2 to $3. At a conversion of 1% to 3%, one paid hub carries 33 to 99 free hubs, so a free hub must cost less than about $0.13 to $0.40 a month. | Decided: HM-R6. The free hub sleeps. |
| HM-D8 | What does a hub that sleeps cost, and how fast does it start? | Not measured. The cost is the disk of the hub and its share of memory while it is on. | Measure on one pool host in step 5. Change the 15 minutes of HMD-024 and the 30 seconds of HMD-026 from the result. |
| HM-D2 | Where does the usage line sit? | Hub memory is not measured. | Set it after the load test of [shared-pool-hosting.md](shared-pool-hosting.md). |
| HM-D3 | Which identity service does the site use? | The operator planned single sign-on on the Vercel site. | Decided: HM-R9, WorkOS AuthKit. |
| HM-D9 | Which sign-in methods does AuthKit show? | AuthKit includes email and password, social sign-in, passkeys and magic auth (fact 7). The beachhead is agentic coders. | GitHub, Google and email with a code; passkeys offered after the first sign-in. |
| HM-D10 | Do the sign-in pages use a domain of Anima.ing, for example `auth.anima.ing`? | A custom domain is $99 a month (fact 7). Without it the browser shows a WorkOS address during sign-in. | Not at the launch. Add it when the cost is small against revenue. |
| HM-D4 | Is the second factor (HMD-006) required for each account? | Without it, one stolen sign-on session opens each device of a hub. AuthKit includes multi-factor authentication at no extra price (fact 7). | Proposed: required. Not ruled. |
| HM-D5 | In which region is the first pool host? | The priced hosts are in the EU. No US host is priced. | Price a US host before the launch. |
| HM-D6 | Is the directory and billing code open source? | OR-5: not open source. The operator decided the same for the cloud on 2026-11-16 (fact 5). | Decided. No change. |
| HM-D7 | Who reviews the security of the pool and of the grant path before the launch? | The pool proposal asks for a review before paying customers. HM-R2 adds the grant path. | A review by a person from outside, before the first account of a different person. |

## 8. Work, in order

A task breakdown comes after the operator approves the requirements and decides HM-D4 (PLN-002).
Before that, only a task that a ruling names is built (PLN-007).

| Step | Work | Depends on |
| --- | --- | --- |
| 1 | The link in the server stack (HMD-011, HMD-012). Also the base of the self-host mode. | Nothing |
| 2 | The load test of one hub: memory, devices for each slice | Step 1 |
| 3 | The account directory: accounts, hubs, grants, and the WorkOS sign-in (HMD-001 to HMD-010, HMD-032 to HMD-036). Start in the WorkOS staging environment. | HM-D4, HM-D9 |
| 4 | The hub accepts grants; the hosted hub has no separate operator password (HMD-005). A spec delta: the login requirements and NET-001. | Step 3 |
| 5 | One pool host: isolation, limits, backup, the make-a-hub job, sleep and wake (HMD-013, HMD-014, HMD-024 to HMD-029, HM-D8) | Step 2, HM-D5 |
| 6 | The first run: "Sign in" (HMD-022, HMD-023) | Steps 3 and 4 |
| 7 | The usage card, the line, Stripe (HMD-017 to HMD-021) | HM-D1, HM-D2 |
| 8 | The security review (HM-D7), the terms and the text of HMD-015 | Steps 3 to 5 |
| 9 | Split custody (OI-3 of the one-click plan) | After the launch (HM-R5) |

What the code of 2026-11-17 still needs before HMD-037 is true with a real hub (steps 3 and 4).
Each item is a place where a device that joined through the link still needs an invite or the hub
password:

| Place | What it does today | What must change, and on which side |
| --- | --- | --- |
| `apps/desktop/src-tauri/src/link/printed.rs` (`invite`), called by `link_join` in `apps/desktop/src-tauri/src/link.rs` | Accepts only text that starts with `anima-invite.1.`. Anything else is `bad-invite`. | If the cloud gives a device grant, the shell must accept its form too. If the cloud gives an invite, no change. |
| `link/invites/` and `link/cmd/anima-link/join.go` | The hub admits a new device key only with a one-time invite code that the hub made (LNK-003, 15 minutes). | Hub side: accept a grant of the cloud (HMD-005), or let the cloud make the invite through the hub (next row). |
| `createLinkInvite` in `contracts/schema.graphql` (`crates/hub/src/fleet/link/graphql.rs`) | Makes the invite. Needs an operator token. | Cloud and hub side: the cloud needs a credential of the hub that may make an invite or a device for the account, without the operator password. |
| `login` and `signInBrowser` in `contracts/schema.graphql`; `crates/hub/src/fleet/access/login.rs` and `guard.rs` | The only way to get an access token: the operator name, password and TOTP code. Each other call needs the token. | Hub side: a hosted hub gives the token of a device that the account admitted (step 4: "no separate operator password"). The cloud returns it in the `HubAccess`. |
| `crates/hub/src/operator.rs` (`operator set`, `operator totp`) and `apps/client/src/lib/local-hub/account/` | The hub has an operator account only after a person sets a password and a TOTP secret. | Hub side: a hosted hub has no such account; the account of Anima.ing is its operator (Section 4.3). |
| `apps/client/src/lib/access/guard.ts` (`startGuard`, `signInNeeded`) and `apps/client/src/routes/sign-in/` | When the stored token is missing or the hub answers UNAUTHENTICATED, the client goes to the hub password page. | Client and cloud side: for a hosted hub, go to "Sign in" (the account) again, not to the password page. Not built. |
| `apps/client/src/lib/desktop/this-machine.ts` (`readThisMachine`) and `apps/client/src/lib/devices/add/this-machine.ts` | The device list offers "Add this computer" only for a computer with an address of the Anima.ing VPN (100.64.0.0/10). A computer that joined only through the link has none. | Client and hub side: offer this computer from its link key, so step 8 of Section 4.1 works. |
| `apps/client/src/routes/welcome/join/+page.svelte` | "Join a hub" ends at the hub password page. | No change: it is the self-host path, not a step of "Sign in". |

Tasks that a ruling names:

| Task | Work | Ruling | State |
| --- | --- | --- | --- |
| HM-T1 | Step 6 without the cloud: the first-run screens of "Sign in" in the client (HMD-022, HMD-023, HMD-037). One state for each step of Section 4.1, then the device list, and an error with a retry for cancelled, expired, no network and refused. The cloud is behind the interface `HostedAccount` in `apps/client/src/lib/hosted/`. After the sign-in, its call `access` gives one opaque `HubAccess` (what `link_join` takes, and the hub access token of this device). The app joins with it silently (`join.ts`: the link join, the token in the key store, the address). Two implementations: a stand-in for the tests and for `VITE_ANIMA_HOSTED=stand-in` (`?fail=step:problem` makes it fail once), and the real cloud, which is a stub that refuses each call. Not built: the cloud API (step 3), the hub side (step 4), the device key, the AuthKit device flow, the key store for the refresh token (HMD-034), and the opening of the system browser. Mockup: [first-run/index.html](../mockup/first-run/index.html). | R-127, R-128 | In progress: built on branch `hosted-sign-in-screens`, not merged. The merge sets Done with its commit. |

## 9. Changes that this proposal needs in other documents

| Document | Change |
| --- | --- |
| [one-click-plan.md](../plans/one-click/one-click-plan.md) Section 1.3 | Hosted row: the shared pool, not one VPS for each customer (HM-R4). Done in the commit of this proposal, with a pointer to this file. |
| [one-click-plan.md](../plans/one-click/one-click-plan.md) OR-5 | The directory gives access to a hosted hub (HM-R2). Done in the same commit. |
| [shared-pool-hosting.md](shared-pool-hosting.md) | Remove Headscale from the instance stack; one relay for each pool host (Section 3, fact 4). Not done. |
| Design specification | A delta for the account login of a hosted hub, NET-001 and the first run. Not written. |
| S6 task breakdown | "Sign in" as the primary choice. The client does it (HM-T1); the S6 task breakdown does not yet name it. |
