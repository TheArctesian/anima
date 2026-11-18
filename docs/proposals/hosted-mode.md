---
kind: Proposal
state: Approved
evidence: Approved: R-163 (operator, 2026-11-18) approves HMD-001 to HMD-036, and each of HMD-037 to HMD-041 and HMD-043 to HMD-050 has its ruling; approved does not mean built. The move of this document to `docs/plans/` (PLN-018, stage 3) is not done yet: it changes the links in 15 code files and in `contracts/schema.graphql`, so it needs the full gates and a commit of its own. Direction decided before (HM-R1 to HM-R9, operator, 2026-11-16). R-127 names task HM-T1 (the first-run screens of Sign in, with a stand-in cloud). R-128 approves HMD-037 (the first run is only the sign-in). R-129 makes the account the only way into a hosted hub and approves HMD-038 to HMD-041; HM-T2 to HM-T4 build the hub and client side. R-130 approves HMD-043 (the real cloud client in the shell, HM-T6) and HM-T7 (the hosted mode on a pool hub). HM-T1 to HM-T3, HM-T6 and HM-T7 are built and merged to main (operator, 2026-11-17: "Yes, merge it"); HM-T4 is built in the private repositories. HT-022, the first real sign-in, is Passed (operator, 2026-11-17), so HM-T1 to HM-T7 are Done. HM-R6: a free hub sleeps. HM-R7: Apple Pay and Google Pay. HM-R8: no staff access to hubs. HM-R9: WorkOS AuthKit (decides HM-D3). R-159 decides HM-D4 (a second factor is required before the first device with remote access) and HM-D7 (no review by a person from outside); R-163 approves HMD-001 to HMD-036 with the text of the revision (Section 6.7 gives what is built: 15 Built, 5 Changed, 16 Not built) and decides HM-D14. HMD-051 is proposed. Open: HM-D2, HM-D8, HM-D11, HM-D13 and HM-D15.
---

# Proposal: the hosted mode as the default first run

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the business planning session of 2026-11-16, after the operator tried release 0.2.0 on a
second Mac (report of the S12 packaging session).
Version: 7. Date: 2026-11-18. Version 2 added HM-R6 (a free hub sleeps) and the cloud design of the
business vault (Section 3, fact 5). Version 3 added HM-R7 (Apple Pay and Google Pay), HM-R8 (no
staff access to hubs), HMD-030 and HMD-031. Version 4 adds HM-R9 (WorkOS AuthKit, which decides
HM-D3), the device-code sign-in of Section 4.1 and HMD-032 to HMD-036. HM-D4 is not ruled.
Version 7 holds the approval of HMD-001 to HMD-036 (R-163): the approved text is in Sections 6.1 to 6.6, HM-D14 is decided and HMD-051 is proposed.
Version 6 adds Section 6.7 (the revision of HMD-001 to HMD-036 for approval), R-159 and the open decisions HM-D14 and HM-D15.
Version 5 adds the task table of Section 8, task HM-T1 (R-127), HMD-037 (R-128), HMD-038 to HMD-041 with tasks HM-T2 to HM-T4 (R-129), and HMD-043 with tasks HM-T6 and HM-T7 (R-130).
Status: the header of this document holds the state (PLN-020). History: Draft. The direction is decided by the operator (Section 2). The requirements
HMD-001 to HMD-041 and HMD-043 to HMD-050 are approved (Section 6; R-163 for HMD-001 to HMD-036; no requirement has the number HMD-042); HMD-051 is proposed.
Map: [work-plan.md](../work-plan.md). Parent plans: [one-click-plan.md](../plans/one-click/one-click-plan.md)
Section 1.3 (the hosted mode) and item S6 (first run), and [other-operators.md](other-operators.md)
(OD-W2). This proposal serves row 4.17 of [priorities.md](../plan/priorities.md): tier 1
(operator, 2026-11-16).

Note (operator, 2026-11-17, R-130): the operator will use only the hosted version, and his
self-hosted hub can go away later. This changes no requirement.

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

R-129 (operator, 2026-11-17: "SSO will be the only way", for hosted hubs only) gives the method:

- **HMD-038.** A hosted hub must have no way in other than the Anima.ing account. It must refuse `login` and `signInBrowser`. Its one operator account must have a password that no person knows. Approved (R-129).
- **HMD-039.** A hosted hub must make a link invite and a one-time claim only for a caller that sends the key of `ANIMA_HOSTED_CLOUD_KEY_FILE`. The hub must keep only the hash of a claim. A claim must work once, until its invite expires, and a new claim for a device must end its open claim. Approved (R-129).
- **HMD-040.** A hosted hub must give a login token for a claim only to the device of its link that has the name of the claim, through the link. The cloud must never receive the token of a device. Approved (R-129).
- **HMD-043.** The application must do the WorkOS device sign-in and the calls of the cloud (`POST /v1/hub`, `POST /v1/hub/access` of `cloud.anima.ing`) in the desktop shell. The WorkOS tokens and the device code must not reach the web view, and the refresh token must be in the key store of the system (HMD-034). The client ID is the production client of WorkOS unless the build sets `ANIMA_CLOUD_CLIENT_ID`; with an empty one the sign-in must say that it is not available. The device name must come from the computer name, so that it stays the same across sign-ins. A missing or refused hub token of a hosted hub must lead to the account sign-in, which first renews the session with the stored refresh token. Approved (R-130).
- **HMD-041.** When a device of a hosted hub must sign in again, the application must go to the first-run choices, not to the hub password page. A device that is already on the link must get a new claim and no second invite. Approved (R-129).

- **HMD-044.** When the application opens a hosted hub, at the sign-in and at each start, it must add the computer it runs on to the devices of the hub with no question and no step of the person, unless a device of the hub already has the link address of this computer as its host. The device has the name of the computer, the kind `workstation`, the operating system, and the link address of the computer as its host, in the site `Link`. It has no check until the hub can reach a device through the link (HM-D11). The hub is in the device list by HMD-045. Approved (R-136).
- **HMD-045.** A hosted hub must be a device of its own device list. At each start it must add the device when no device has the host `localhost`: the name `Hub`, the kind `hub`, the operating system Linux, in the site `Link`, with one PUSH check that the hub itself reports as Up while it runs. The device has no SSH, RDP or VNC, and no session reaches the container of the hub. A device that a person renamed stays; a device that a person deleted is added again at the next start. The metrics of the hub are not part of this requirement. Approved (R-137).
- **HMD-046.** The device of HMD-045 must show the metrics of the hub (processor, memory, disk), read by the daemon in the container of the hub. The hub enrols that daemon at its start with no step of a person. The row must show the Linux distribution of the hub, and must not offer to add SSH, RDP or VNC. Approved (R-142).
- **HMD-047.** A hosted hub must keep one check for each device whose host is the link address of a computer on its link: the check is Up while the computer is connected to the link and Down when it is not. The application adds the device of HMD-044 with this check. The hub must not report Down for the time that the hub itself slept or did not run. This check opens no port of the computer. Approved (R-143).
- **HMD-048.** A hosted hub must be able to open a TCP connection to a local port of a computer on its link. The computer keeps streams open to a port of the hub link; the hub link asks for one local port on a stream; the computer connects to that port on its own loopback address only, and only when the port is in its list of open ports. The checks that use TCP, SSH (terminal, files, metrics), VNC and RDP of the hub must use this path for a host that is a link address. Checks that need ICMP or UDP are not possible on this path, and the hub must say so. Approved (R-143).
- **HMD-049.** The list of open ports is kept on the computer, and the hub cannot change it. It can hold port 22 (SSH), port 5900 (VNC) and, on Windows, port 3389 (RDP). It is empty until the person turns a port on. The application must let the person turn a port on or off in one or two clicks, and must say which system setting the port needs (Remote Login, Screen Sharing, Remote Desktop). Anima installs no other program for a session. Approved (R-143).
- **HMD-050.** The link program of the computer (`anima-link join`, `device`) must be built for Windows on amd64 and arm64, and the Windows application must carry it. This part is not tested on a Windows computer until one is available; the plan says "built", not "works". Approved (R-143).

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

HMD-001 to HMD-036 are approved (R-163, operator, 2026-11-18: "Approve both"), with the text of the revision of Section 6.7. Approved does not mean built or checked: Section 6.7 gives the mark of each requirement. HMD-051 is proposed and not approved. The cloud side of each requirement
is built in the private repository (fact 5); this repository holds the client and the hub side.

### 6.1 Account directory and site

- **HMD-001.** The site must let a person make an account with single sign-on. Approved (R-163).
- **HMD-002.** An account must own zero or more hubs. The directory must give the hubs of an account only to that account. Approved (R-163).
- **HMD-003.** When a new account has no hub, the directory must make one hosted hub for it without a step from the person. Approved (R-163).
- **HMD-004.** The directory must ask a hosted hub for a device invite and a one-time claim only for a signed-in account, and only for the hub of that account. The claim names one device and stops when its invite expires (HMD-039). Approved (R-163).
- **HMD-005.** A hosted hub must admit a device only with an invite and a one-time claim that the cloud asked for with the cloud key of that hub (HMD-039, HMD-040). The link of the hub must keep the list of its device keys (LNK-004). Approved (R-163).
- **HMD-006.** An account must have a second factor (a passkey, TOTP, or the second factor of the sign-on provider) before the first device with remote access is added to its hub. The sign-in stays one step. HM-D14 defines "remote access". Approved (R-163).
- **HMD-007.** When a device joins, the hub must send a notice to each joined device, and the directory must send an email to the account. Approved (R-163).
- **HMD-008.** The owner must be able to remove a device from each joined device and from the site. The hub must close the open sessions of that device. Approved (R-163).
- **HMD-009.** The owner must be able to set that a joined device approves each new device. The default is off (HM-R2). Approved (R-163).
- **HMD-010.** The directory must never hold a device credential, a session or a record of a hub. It passes the invite and the one-time claim of HMD-004 to the device and must not keep them. Approved (R-163).

### 6.2 Hosted hub

- **HMD-011.** The server stack must run `anima-link`, and the hub must run in link mode (Section 3, fact 3). A hosted hub must have no public port other than the relay. Approved (R-163).
- **HMD-012.** A hosted hub must run without Headscale and without the Tailscale client. Approved (R-163).
- **HMD-013.** The pool must make a hub in less than 60 seconds. The application must show the progress. Approved (R-163).
- **HMD-014.** A hosted hub must have its own data, keys and backup. One hub must never read the data of a different hub. Approved (R-163).
- **HMD-015.** Before the first hub is made, the site must tell the person in plain words what a hosted hub can read (the table "What a hosted hub can read" of [other-operators.md](other-operators.md)). The text must not say or imply that a person reviewed the isolation of the hubs. Approved (R-163).
- **HMD-016.** The person must be able to export the hub and to move it to a self-host hub or a hub on their computer. Approved (R-163).

### 6.3 Usage line and payment

- **HMD-017.** Each hosted hub must show its use against its limits (the "Your hub" card of [shared-pool-hosting.md](shared-pool-hosting.md) Section 2). Approved (R-163).
- **HMD-018.** Above the usage line, the hub must ask the person to pay. It must slow down; it must not delete data. Approved (R-163).
- **HMD-019.** Payment is on the site, through Stripe. The application must show the plan and open the site for a change. The checkout takes Apple Pay and Google Pay (HMD-030). Approved (R-163).
- **HMD-020.** An account must have no more than one free hub. Approved (R-163).
- **HMD-021.** A free hub with no sign-in for 60 days must be paused. The directory must warn the person by email before it deletes a paused hub. Approved (R-163).
- **HMD-051.** The hub of a paid account whose payment failed must be deleted 30 days after the first failed payment. The account must get warning emails before the deletion. Proposed, not approved: R-159 (e) holds the answer of the operator, and no ruling approves this text. HMD-018 is for a hub above the usage line, which is a different case.
- **HMD-030.** On a mobile device, the checkout must offer Apple Pay and Google Pay first, and a normal card form when no wallet is available (HM-R7). The page of the checkout must be on `cloud.anima.ing`. Card and wallet details must never reach the directory or its logs. Only Stripe identifiers may. Approved (R-163).
- **HMD-031.** The staff tools of the directory must show account, billing and job data only. Staff must not open a session on a customer's hub or read its content (HM-R8). Access to a hub is allowed only on a plan with a support tier that includes it, after the customer opts in, and the hub must write each access to its audit log. Approved (R-163).

### 6.4 First run

- **HMD-022.** "Sign in" must be the primary choice of the first run. The other choices must stay available. The first screen follows frame 1 of the mobile mockup: the doors, the title, "Sign in", then "Sign up" and "Join a hub" side by side under it. The other ways (an invite, a hub address, a hub on this computer) are on the page behind "Join a hub" (operator, 2026-11-17: "1st option is sign in ... the paste invite ... should be split into two buttons, sign up or join a hub"; the line under the title is removed). Approved (R-163).
- **HMD-023.** The application must show one state for each step of Section 4.1, and a clear error with a retry for each failure. Approved (R-163).

### 6.5 Sleep and wake of a free hub (HM-R6)

- **HMD-024.** The pool must stop a free hub when no device of the account was connected for 15 minutes. A stopped hub must use disk only. Approved (R-163).
- **HMD-025.** When a person opens the application or signs in, the cloud must start the hub of the account. The application must show "Your hub is starting" until the link connects. Approved (R-163).
- **HMD-026.** A free hub must start in less than 30 seconds. Measured for a hub with no device only (HM-D8). Approved (R-163).
- **HMD-027.** The cloud, not the hub, must decide to stop a hub, from the heartbeat of the hub and the state of the link. A hub that sends no heartbeat must be stopped by the pool. Approved (R-163).
- **HMD-028.** While a free hub sleeps, a daemon on a device must wait and try again without an error to the person. The application must say in plain words that checks, notifications and history need a hub that is always on. Approved (R-163).
- **HMD-029.** A paid hub must never sleep. When a person pays, the hub must stay on from that moment without a move of data. Approved (R-163).

### 6.6 Sign-in with WorkOS AuthKit (HM-R9)

- **HMD-032.** The application must sign in with the device authorization flow of WorkOS AuthKit, as a public client. The application must hold only the client ID: no WorkOS API key and no client secret. Approved (R-163).
- **HMD-033.** The cloud must accept a WorkOS access token only after it checks the signature (RS256 only), the issuer (`iss`), the client (`client_id`) and the expiry. The expected values of `iss` and `client_id` are read from one real Production token first. The cloud must find the account by the WorkOS user ID, not by the email address. Approved (R-163).
- **HMD-034.** The application must keep the WorkOS refresh token in the key store of the system, never in a file or a log (LOG-007). It must replace the stored token after each exchange, and it must delete the tokens when WorkOS refuses one. Approved (R-163).
- **HMD-035.** The WorkOS API key must be only in the private cloud (HCL-014). The open-source repository must hold only the client ID and the `ANIMA_CLOUD_URL` setting (HCL-002). Approved (R-163).
- **HMD-036.** When the owner removes a device (HMD-008) or signs out on the site, the cloud must end the WorkOS sessions of that device and refuse its refresh token. How WorkOS ends one session is not checked. Approved (R-163).

### 6.7 What is built of HMD-001 to HMD-036 (2026-11-18)

The operator asked for a revision of the list before he approved it (2026-11-18: "B: revise first"), and
approved the revision with R-163 ("Approve both"). Sections 6.1 to 6.6 hold the approved text. This table is the
record of the comparison: each requirement against the code on `main` (3dc5beae), the private plans of the
site and of the pool service, and the rulings R-127 to R-159. The text changed for HMD-004, HMD-005, HMD-006,
HMD-010, HMD-013, HMD-015, HMD-025, HMD-026 and HMD-033.

The marks: **Built** (the function exists; the evidence says what is not checked), **Changed** (a ruling
replaced the design; the evidence says what is built of the new design), **Not built**. An approved requirement
is not built by its approval, and a mark is not a result of a live check: only an entry of
[need-human-testing.md](../need-human-testing.md) is. Count: 15 Built, 5 Changed, 16 Not built. Change a mark
in the commit that builds the requirement.

| Requirement | Mark | Evidence |
| --- | --- | --- |
| HMD-001 | Built | The site signs a person in with WorkOS AuthKit and makes the account at the first sign-in (private site, sign-in and callback routes). HT-022 Passed. |
| HMD-002 | Built | `POST /v1/hub` of the site gives only the hub of the signed-in account; the private service refuses a hub of a different account (its plan, T9 and T23). An account has zero hubs or one hub: more than one is not built. |
| HMD-003 | Built | The first `POST /v1/hub` of an account makes its hub on the plan `free`, with no card and no step (private site plan, HUB-D1). HT-022 Passed. |
| HMD-004 | Changed | R-129 and R-130 replaced the signed grant of 5 minutes: the cloud asks the hub for a link invite and a one-time claim (`hostedDeviceInvite`), with the cloud key of that hub. Built: `crates/hub/src/fleet/link/graphql/hosted.rs`, `claims.rs` (HM-T2, 921ee045); HMD-039 holds the hub side. |
| HMD-005 | Changed | The same rulings. The hub gives a login token only for the claim, to the device of its link that has the name of the claim (`claimDeviceToken`, HMD-040). The link keeps the device keys (`link/devices/`, LNK-004). HT-022 Passed. |
| HMD-006 | Changed | R-159 (HM-D4): "C: required later". Not built: no code asks for a second factor. |
| HMD-007 | Not built | No notice to the joined devices and no email when a device joins: none is in the hub, the client or the private plans. |
| HMD-008 | Not built | Built in part: the owner removes a device of the link from the client, and the hub link closes its streams (`revokeLinkDevice`, LNK-004). Not built: removal from the site. |
| HMD-009 | Not built | No setting and no approval step exist. |
| HMD-010 | Built | The cloud never receives the token of a device (HMD-040), and the private service keeps no invite and no claim (its plan, ACL-022). The cloud passes the invite and the claim to the device one time. |
| HMD-011 | Built | A hub of the pool runs `anima-local-hub` with the link and the hosted mode (HM-T7, 9babe035). Not checked: a scan of the open ports of a pool host. |
| HMD-012 | Built | The same image: the link is the only path, with no Headscale and no Tailscale client (HM-T7). |
| HMD-013 | Built | Measured on the first pool host on 2026-11-17: about 10 seconds from a make to a ready hub (HM-D8). The application shows the progress (HM-T1, HMD-037). The person makes the hub from the application, so the site shows no progress. |
| HMD-014 | Built | Each hub is one isolated instance with its own data, its own keys and limits (private service plan, ACL-003, T5). Not built: a backup of each hub. Not reviewed by a person (HM-D7, R-159). |
| HMD-015 | Not built | The site has no such text. R-159 (HM-D7) adds a limit to the text. |
| HMD-016 | Not built | A self-hosted hub has a backup and a restore on a second server (FND-006, HT-024 Open). A hub of the pool has no export. |
| HMD-017 | Not built | No card "Your hub" and no limits. HM-D2 is open. |
| HMD-018 | Not built | No usage line exists (HM-D2). R-159 (e) is a different case: a paid account that is past due. |
| HMD-019 | Not built | Built in part, in the Stripe sandbox only: the site saves a card and opens the customer portal (private site, Stripe plan). Not built: the application does not show the plan and does not open the site for a change. Not checked: Apple Pay and Google Pay, and the live mode. R-159 (f) is the move to live; the private plans own it. |
| HMD-020 | Built | An account has one hub: `POST /v1/hub` gives the hub that exists and makes no second one. |
| HMD-021 | Not built | No pause and no deletion of a free hub that is not used. HM-D15 is open: the operator did not answer this question. |
| HMD-022 | Built | HM-T1 (a428809e, 6659c3c7); HT-022 Passed. |
| HMD-023 | Built | HM-T1: one state for each step, and an error with a retry (`apps/client/src/routes/welcome/hosted/`). |
| HMD-024 | Not built | The private service does not sleep a hub by use (its plan, T7: not started, with an open decision for what "use" is). HM-D15 is open. |
| HMD-025 | Changed | Built: `POST /v1/hub` wakes a hub that sleeps, and the application shows "Your hub is starting" (HMD-037). The start is one instruction to the private service; there is no job table. |
| HMD-026 | Built | Measured on the first pool host on 2026-11-17: a wake in 0.2 seconds for a hub with no device (HM-D8). Not measured: a hub with devices, and many hubs on one host. |
| HMD-027 | Not built | The private service stops a hub only on an instruction of the site, and no heartbeat exists (its plan, ACL-018, T7). |
| HMD-028 | Not built | No text in the application says what needs a hub that is always on. The wait of a daemon while its hub sleeps is not tested, because no hub sleeps by use. |
| HMD-029 | Not built | No hub sleeps by use, and no path changes a hub from free to paid yet (private site plan, HUB-C1). |
| HMD-030 | Not built | Built in part: the card form is a page of Stripe, so card details do not reach the directory. Not built: a checkout on `cloud.anima.ing` that offers the wallets first. Not checked: Apple Pay and Google Pay. |
| HMD-031 | Not built | No staff tool exists. No path lets staff open a hub. |
| HMD-032 | Built | `apps/desktop/src-tauri/src/cloud/workos.rs`: the device authorization flow with the client ID only (HM-T6, 0518056e). HT-022 Passed. |
| HMD-033 | Changed | R-159 (d): "A: add the checks". Built in the private site: the signature, the subject and the expiry, and the account by the WorkOS user ID. Not built: the checks of `iss` and `client_id`, and RS256 only. The site work is in the private repository. |
| HMD-034 | Built | `apps/desktop/src-tauri/src/cloud/session.rs`: the refresh token is in the key store of the system (HM-T6). HT-022 Passed. |
| HMD-035 | Built | This repository holds the client ID (`ANIMA_CLOUD_CLIENT_ID` at build time) and `ANIMA_CLOUD_URL` only. |
| HMD-036 | Not built | No removal of a device from the site (HMD-008), and no code ends the WorkOS sessions of one device. |

Of HMD-051 (proposed): the site sleeps the hub of an account that is past due (private site plan, HUB-D2). The
deletion after 30 days and the warning emails are not built.

Open, and not answered by R-163:

- HM-D15: a free hub that is not used (HMD-021, HMD-024).
- HM-D2 (the usage line: HMD-017, HMD-018) and HM-D8 (the numbers of HMD-024 and HMD-026).
- HM-D11 and HM-D13; no requirement of this list waits for them.

## 7. Open decisions

| ID | Decision | Facts | Proposal |
| --- | --- | --- | --- |
| HM-D1 | How can each free hub cost less than the paid hubs earn? | Estimates: a paid Solo hub earns about $13 a month after its hardware and the payment fee. A free always-on slice costs about $2 to $3. At a conversion of 1% to 3%, one paid hub carries 33 to 99 free hubs, so a free hub must cost less than about $0.13 to $0.40 a month. | Decided: HM-R6. The free hub sleeps. |
| HM-D8 | What does a hub that sleeps cost, and how fast does it start? | The cost is the disk of the hub and its share of memory while it is on. One hub with no device, on the first pool host on 2026-11-17 (the private service's plan, T5): 308 MiB of memory and 56 MiB of disk in use, a sleep in 1.3 seconds, a wake in 0.2 seconds, and about 10 seconds from a make to a ready hub. Not measured: a hub with devices, and many hubs on one host. | Measure on one pool host in step 5. Change the 15 minutes of HMD-024 and the 30 seconds of HMD-026 from the result. |
| HM-D2 | Where does the usage line sit? | Hub memory is not measured. | Set it after the load test of [shared-pool-hosting.md](shared-pool-hosting.md). |
| HM-D3 | Which identity service does the site use? | The operator planned single sign-on on the Vercel site. | Decided: HM-R9, WorkOS AuthKit. |
| HM-D9 | Which sign-in methods does AuthKit show? | AuthKit includes email and password, social sign-in, passkeys and magic auth (fact 7). The beachhead is agentic coders. The sign-in page of the site has three doors since 2026-11-17: GitHub, Google and email. The session that built the app reports that GitHub, Google and passkeys are not on in WorkOS Production. | GitHub, Google and email with a code; passkeys offered after the first sign-in. |
| HM-D10 | Do the sign-in pages use a domain of Anima.ing, for example `auth.anima.ing`? | A custom domain is $99 a month (fact 7). Without it the browser shows a WorkOS address during sign-in. | Not at the launch. Add it when the cost is small against revenue. |
| HM-D4 | Is the second factor (HMD-006) required for each account? | Without it, one stolen sign-on session opens each device of a hub. AuthKit includes multi-factor authentication at no extra price (fact 7). | Decided (R-159, operator, 2026-11-18: "C: required later"). The sign-in stays one step. A second factor is required before the first device with remote access is added, not for each account at the sign-in. The meaning of "remote access" is HM-D14. |
| HM-D5 | In which region is the first pool host? | The priced hosts are in the EU. No US host is priced. For the build and the first tests the operator rented a cloud server in the US on 2026-11-17; the host for paying customers is not decided. | Price a US host before the launch. |
| HM-D6 | Is the directory and billing code open source? | OR-5: not open source. The operator decided the same for the cloud on 2026-11-16 (fact 5). | Decided. No change. |
| HM-D13 | Does a public client need `client_secret` to refresh a WorkOS token, and does the device request take JSON or a form? | The CLI Auth guide sends the client ID only, as a form. The reference table of the refresh grant marks `client_secret` as required, and the device reference shows JSON. The shell sends forms with the client ID only. Checked against WorkOS Production on 2026-11-17 with no sign-in: the device request as a form with the client ID only answers 200 with a device code, a user code and the page; a refresh as a form with no secret and a wrong token answers 400 `invalid_grant` "Invalid refresh token", not a refusal of the client. | The forms of the shell stay. The refresh of a real token is step 4 of HT-022. Not ruled. |
| HM-D11 | How does the device list offer "Add this computer" for a computer that reaches its hosted hub only through the link (step 8 of Section 4.1)? | The offer needs a VPN address (`apps/client/src/lib/desktop/this-machine.ts`). The link carries streams from the device to the hub only. | Add a path from the hub to the device through the link, then offer the computer by its link key. Not ruled. The operator saw the empty device list after the first real sign-in (2026-11-17: "the device doesn't auto add it should and the hub is not there as the default device"): the computer that signs in must be added with no step, and the hub must be in the list. |
| HM-D12 | What must the phone app have before a phone can use "Sign in"? | Phones use the same claim join (a dependency, not a different design): the link library in the app (MOB-031, MOB-032) with the same `link_join`, `link_status` and `link_leave` commands (MOB-002), which turn on `deviceLink`. Not built: no phone has joined a hub (M1-4), the link in the mobile shell is on hold (M3-1, 2026-11-16), and phones have no key store (M5-1 deferred), so the hub token and the refresh token (HMD-034) are in memory only. On iOS the link stops in the background (MOB-036), so a phone must reconnect with its kept link state and token, with no new claim: the key store is required there. The relay path from a phone is not tested. | Build M5-1 with M3-1. Not ruled. |
| HM-D7 | Who reviews the security of the pool and of the grant path before the launch? | The pool proposal asks for a review before paying customers. HM-R2 adds the grant path. | Decided (R-159, operator, 2026-11-18): option D, "no review; rely on the design and the tests". No person from outside reviews the pool or the path from the account to a device before the launch. Consequence: no text for a customer may say or imply that a person reviewed the isolation (HMD-015). |
| HM-D14 | What is "remote access" in the rule of the second factor (HM-D4, R-159)? | The row `Hub` of a hosted hub is in the device list from the start (HMD-045), and the computer that signs in is added with no step (HMD-044); neither has a session at that moment. A device has remote access when the hub can open a session on it: SSH, VNC or RDP. For a computer of the link that is the moment when its person turns a port on (HMD-049). For a device on a network that is the moment when a protocol or a credential is added to it. | Decided (R-163, operator, 2026-11-18: "Approve both"): an account needs a second factor before (1) a port of a computer of the link is turned on, and (2) SSH, VNC, RDP or a credential is added to a device of the hub. The rows of HMD-044 and HMD-045 and the check `Connected` need none. |
| HM-D15 | What occurs to a free hub that is not used? | HMD-021 pauses a free hub after 60 days with no sign-in and deletes it after a warning. HMD-024 stops it after 15 minutes with no connected device. R-159 gives a rule only for a paid account that is past due (deleted 30 days after the first failed payment). The private service does not sleep a hub by use yet: its plan has an open decision for what "use" is. | Not ruled. The operator did not answer this question. HMD-021 and HMD-024 stay as they are until he does. |

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

Each place where a device that joined through the link needed an invite or the hub password, and
what R-129 (HM-T2, HM-T3) did with it:

| Place | What it did | What changed |
| --- | --- | --- |
| `apps/desktop/src-tauri/src/link/printed.rs` (`invite`), called by `link_join` in `apps/desktop/src-tauri/src/link.rs` | Accepts only text that starts with `anima-invite.1.`. Anything else is `bad-invite`. | No change: the cloud gives a link invite, which the app uses under the hood. |
| `link/invites/` and `link/cmd/anima-link/join.go` | The hub admits a new device key only with a one-time invite code that the hub made (LNK-003, 15 minutes). | No change: the hub makes the invite for the cloud (next row). |
| `createLinkInvite` in `contracts/schema.graphql` (`crates/hub/src/fleet/link/graphql.rs`) | Makes the invite. Needs an operator token. | `hostedDeviceInvite` makes the invite and a claim for the cloud key (HMD-039). `createLinkInvite` does not change. |
| `login` and `signInBrowser` in `contracts/schema.graphql`; `crates/hub/src/fleet/access/login.rs` and `guard.rs` | The only way to get an access token: the operator name, password and TOTP code. Each other call needs the token. | A hosted hub refuses both (HMD-038). `claimDeviceToken` gives the token to the device of the claim, through the link; the cloud never sees it (HMD-040). |
| `crates/hub/src/operator.rs` (`operator set`, `operator totp`) and `apps/client/src/lib/local-hub/account/` | The hub has an operator account only after a person sets a password and a TOTP secret. | A hosted hub makes the operator `owner` at the first claim, with a password that no one knows (HMD-038). The commands of the hub computer stay, but a hosted hub refuses each password sign-in. |
| `apps/client/src/lib/access/guard.ts` (`startGuard`, `signInNeeded`) and `apps/client/src/routes/sign-in/` | When the stored token is missing or the hub answers UNAUTHENTICATED, the client goes to the hub password page. | For an address that the hosted join marked, `goToSignIn` goes to the first-run choices (HMD-041). A device already on the link gets a claim and no second invite. |
| `apps/client/src/lib/desktop/this-machine.ts` (`readThisMachine`) and `apps/client/src/lib/devices/add/this-machine.ts` | The device list offers "Add this computer" only for a computer with an address of the Anima.ing VPN (100.64.0.0/10). A computer that joined only through the link has none. | Not changed: it is not a sign-in. The hub reaches a device to manage it over SSH or the VPN, and a computer of the link has neither path from the hub. Open decision HM-D11. |
| `apps/client/src/routes/welcome/join/+page.svelte` | "Join a hub" ends at the hub password page. | No change: it is the self-host path, not a step of "Sign in". |

Tasks that a ruling names:

| Task | Work | Ruling | State |
| --- | --- | --- | --- |
| HM-T2 | The hub side of HMD-038 to HMD-040: the hosted mode (`ANIMA_HOSTED_CLOUD_KEY_FILE`, which needs `ANIMA_LINK_CONTROL`), `hostedDeviceInvite` and `claimDeviceToken` (K1), the claim book in memory (`crates/hub/src/fleet/link/claims.rs`), the token of the operator `owner` (`crates/hub/src/fleet/access/login/hosted.rs`), and the refusal of the password sign-in. Tests: `crates/hub/tests/it/link/hosted.rs`. | R-129 | Done: built in 921ee045, 2f7a4eff; merged to main; HT-022 Passed (operator, 2026-11-17). |
| HM-T3 | The client side of HMD-040 and HMD-041: the `HubAccess` holds the invite (or none) and the claim; `apps/client/src/lib/hosted/join.ts` joins the link, trades the claim for the token through the link, keeps it, saves and marks the address; a hosted hub signs in again through the first-run choices (`signInPathFor`). | R-129 | Done: built in 921ee045; merged to main; HT-022 Passed (operator, 2026-11-17). |
| HM-T7 | `anima-local-hub run --hosted-cloud-key-file <path>` passes `ANIMA_HOSTED_CLOUD_KEY_FILE` to the hub, so a hub of the pool can run in the hosted mode of HM-T2: `anima-local-hub` starts the hub with a cleared environment, and without the option no hub could be hosted. `crates/anima-local-hub/src/members.rs` (`hosted`), the procedure in [local-hub.md](../procedures/setup/local-hub.md). | R-129 | Done: built in 9babe035; merged to main. The pool's hub image `anima-hub-9babe035` uses it. HT-022 Passed (operator, 2026-11-17). |
| HM-T6 | The app's real cloud (HMD-043): the shell module `apps/desktop/src-tauri/src/cloud/` (the WorkOS device flow, the refresh token in the key store, `/v1/hub`, `/v1/hub/access` with a retry while the hub is not ready, the device name), `apps/client/src/lib/desktop/cloud/shell.ts` in place of the stub, and the silent renewal of a hosted hub (`signInPathFor`). Not built: "Add this computer" for a computer of the link only (HM-D11). | R-130 | Done: built in 0518056e, fdf1c668; merged to main; HT-022 Passed (operator, 2026-11-17). HM-D11 stays open. |
| HM-T4 | The cloud side, in the private repository: keep the cloud key of each hosted hub, call `hostedDeviceInvite` after the WorkOS sign-in, return the invite and the claim to the application. Then the real `cloudAccount` in the client replaces the stub. | R-129 | Done: built in the private repositories by other sessions (anima-cloud fd32c0e and T23, the site's `/v1/hub` on cloud.anima.ing). Checked from this repository on 2026-11-18 (UTC) with `anima-link` of `main` and a test hub of the pool: the join with the invite and the trade of the claim (the entry HT-022 holds the list). HT-022 Passed (operator, 2026-11-17): the site made the hub and gave the access for a real token. |
| HM-T8 | The computer that signs in is a device with no step (HMD-044): the shell gives the device name of this computer (`cloud_device`), the client finds its link address among the link devices of the hub and adds the device in the site `Link` when no device has that host; the device list of a hosted hub does not ask "Add this computer?". No hub function changes, so no `anima` command and no MCP tool are added (CMD-030). Not in this task: the hub as a device, and a path from the hub to the device (sessions, checks), which are the next tasks of HM-D11. | R-136 | In progress: built in 3ef2a1a4; waits for HT-027. |
| HM-T9 | The hosted hub is a device of its own list (HMD-045): at its start in the hosted mode the hub makes sure that the device `Hub` (host `localhost`, kind `hub`) and its PUSH check exist, and reports that check as Up at the start and then on a timer. The hub side only: no contract change, no new hub function (so no `anima` command and no MCP tool, CMD-030), no client change. It reaches hubs only with a new hub image of the pool; a hub that exists keeps its programs. Not in this task: the metrics of the hub (the daemon in the image and its enrolment with no person), which is the next task. | R-137 | In progress: built in ef2b01aa; waits for HT-028. |
| HM-T10 | The metrics of the hub and its distribution on the row `Hub` (HMD-046): the hub makes an enrolment for its own daemon at the start, the image of the pool carries the daemon, the row shows the distribution and offers no protocol. Needs a new hub image. | R-142 | Not started. |
| HM-T11 | The check of the presence on the link (HMD-047). The link contract gets port 446 (`ReversePort`): the link program of a computer keeps one stream open to it and writes `here` each 15 seconds; the hub link records the time of the last line and gives it as `seen` in `devices.list`. The hub reads that list each 15 seconds and keeps the PUSH check `Connected` on each device whose host is the link address of a computer that the link saw: Up while the last line is 45 seconds old or newer, and one Down report when the computer left. The hub adds the check to the device that the application added (HMD-044), when the computer is first connected, so that an application with an old link program keeps a device with no check. The hub does not count the time in which it slept or did not run: at a start and after a sleep the checks wait one whole interval again, and the scheduler takes a wait of more than 30 seconds for a tick as a new start. The check is an ordinary check: no contract change and no new hub function, so no `anima` command, no MCP tool (CMD-030) and no client change. It reaches hubs only with a new hub image of the pool, and computers only with a new application. | R-143 | In progress: built in 5a051aec; waits for HT-029. |
| HM-T12 | The path from the hub to a local port of a computer (HMD-048), for TCP: the port of the hub link, the streams that the computer keeps open, the list of open ports on the computer, the socket that the hub dials through, one connect function in the hub, and the TCP checks and SSH on it. The link contract gets port 447 (`OpenPort`): the hub link asks for a port on the stream of the reverse port, and the computer gives the connection on a new stream to port 447, only for a port of its list (the file `open-ports` of its link program; `anima-link ports` shows and changes it, 22 and 5900, 3389 on Windows only). The hub link serves the socket `dial.sock` beside its control socket, and the hub connects through it with `targets::connect` when the address is a link address. The TCP checks, the TLS checks and each SSH connection of the hub use that function. A check that needs ICMP or UDP on a computer of the link is Unknown and says that the link carries TCP only. No contract change of the hub and no new hub function, so no `anima` command, no MCP tool (CMD-030) and no client change. It reaches hubs only with a new hub image of the pool, and computers only with a new application. Until HM-T13, the command `anima-link ports` is the only way to turn a port on. | R-143 | In progress: built in f69d228d, 0b79ad39; waits for HT-031. |
| HM-T13 | The switches of the open ports (HMD-049): the shell command, the screen in the application (one or two clicks), and the text that names the system setting. | R-143 | Not started. |
| HM-T14 | VNC and RDP through the path (HMD-048): the gateway reaches a computer on the link through the hub. | R-143 | Not started. |
| HM-T15 | The link program for Windows (HMD-050): `link/build.sh` builds it, the Windows application carries it. Built and not tested on Windows. | R-143 | Not started. |
| HM-T1 | Step 6 without the cloud: the first-run screens of "Sign in" in the client (HMD-022, HMD-023, HMD-037). One state for each step of Section 4.1, then the device list, and an error with a retry for cancelled, expired, no network and refused. The cloud is behind the interface `HostedAccount` in `apps/client/src/lib/hosted/`. After the sign-in, its call `access` gives one opaque `HubAccess` (what `link_join` takes, and the hub access token of this device). The app joins with it silently (`join.ts`: the link join, the token in the key store, the address). Two implementations: a stand-in for the tests and for `VITE_ANIMA_HOSTED=stand-in` (`?fail=step:problem` makes it fail once), and the real cloud, which is a stub that refuses each call. Not built: the cloud API (step 3), the hub side (step 4), the device key, the AuthKit device flow, the key store for the refresh token (HMD-034), and the opening of the system browser. Mockup: [first-run/index.html](../mockup/first-run/index.html). | R-127, R-128 | Done: built in a428809e, 6659c3c7 (screens), 875ddfc8, d2a9564b (mockup); merged to main with HM-T2 to HM-T7; HT-022 Passed (operator, 2026-11-17). |

## 9. Changes that this proposal needs in other documents

| Document | Change |
| --- | --- |
| [one-click-plan.md](../plans/one-click/one-click-plan.md) Section 1.3 | Hosted row: the shared pool, not one VPS for each customer (HM-R4). Done in the commit of this proposal, with a pointer to this file. |
| [one-click-plan.md](../plans/one-click/one-click-plan.md) OR-5 | The directory gives access to a hosted hub (HM-R2). Done in the same commit. |
| [shared-pool-hosting.md](shared-pool-hosting.md) | Remove Headscale from the instance stack; one relay for each pool host (Section 3, fact 4). Not done. |
| Design specification | A delta for the account login of a hosted hub, NET-001 and the first run. Not written. |
| S6 task breakdown | "Sign in" as the primary choice. The client does it (HM-T1); the S6 task breakdown does not yet name it. |

## Council follow-up: first use and the service promise (2026-11-18)

R-152 names the preparation below. It changes no approved requirement or contract. Read current `main` and the claims before taking a task. A source review is not a Passed human check. The task owner records the tested build and the result; unknown results stay unknown.

| Task | Work and acceptance | Depends on | State |
| --- | --- | --- | --- |
| HM-C1 | Compare the published installer and its source commit with the hosted functions it carries. Read the release record and HT-022; keep the operator's existing sign-in proof. Produce one packet: artifact, version, checksum, included functions, missing proof. Do not infer that a development build is the published build. | Release owner; [releases.md](../procedures/releases.md); HT-022 | Not started. Preparation only (R-152) |
| HM-C2 | Prepare one pilot worksheet: ordinary download, install, sign in, own computer visible, first useful session, quit and reopen the same hub. Record assistance, exclusions and the first failed step. Successful SSH, the ended session record and a fresh read after reopen are separate observations. HM-C1 supplies the artifact; this is one pilot, not one per agent. | HM-C1; the session path in HM-T12 to HM-T15; HM-D7; HMD-015; operator permission for the participant and target | Not started. Preparation only; live execution is blocked |
| HM-C3 | Compare the explanation at sign-in with what the hub can read: private transport, server-readable credentials and client-sealed records are separate. Give the owner proposed wording with code evidence. Do not claim that a staff-access policy prevents technical access. | HMD-015; HM-D7; custody decisions of the owner | Not started. Preparation only; publishing new wording waits for owner review |
| HM-C4 | Prepare three fixed timelines for hub sleep and daemon backfill. For each, separate readings stored later from checks performed at the time. A reconciliation timestamp is discovery time unless both actual sleep endpoints are known. List the missing source of an account-scoped sleep interval. | HMD-028; private sleep/wake owner; no new public interface is approved | Not started. Preparation only; display changes are blocked |

HM-D2 and HM-D8 also need the case where an account keeps a device connected for the whole measurement period. A powered-on computer alone does not prove that its account link stayed connected. The private pool plan owns the cost and capacity calculation.
