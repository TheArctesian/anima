---
kind: Proposal
state: Approved
evidence: Approved: R-163 (operator, 2026-11-18) approves HMD-001 to HMD-036, and each of HMD-037 to HMD-041 and HMD-043 to HMD-050 has its ruling; approved does not mean built. The move of this document to `docs/plans/` (PLN-018, stage 3) is not done yet: it changes the links in 15 code files and in `contracts/schema.graphql`, so it needs the full gates and a commit of its own. Direction decided before (HM-R1 to HM-R9, operator, 2026-11-16). R-127 names task HM-T1 (the first-run screens of Sign in, with a stand-in cloud). R-128 approves HMD-037 (the first run is only the sign-in). R-129 makes the account the only way into a hosted hub and approves HMD-038 to HMD-041; HM-T2 to HM-T4 build the hub and client side. R-130 approves HMD-043 (the real cloud client in the shell, HM-T6) and HM-T7 (the hosted mode on a pool hub). HM-T1 to HM-T3, HM-T6 and HM-T7 are built and merged to main (operator, 2026-11-17: "Yes, merge it"); HM-T4 is built in the private repositories. HT-022, the first real sign-in, is Passed (operator, 2026-11-17), so HM-T1 to HM-T7 are Done. HM-R6: a free hub sleeps. HM-R7: Apple Pay and Google Pay. HM-R8: no staff access to hubs. HM-R9: WorkOS AuthKit (decides HM-D3). R-159 decides HM-D4 (a second factor is required before the first device with remote access) and HM-D7 (no review by a person from outside); R-163 approves HMD-001 to HMD-036 with the text of the revision (Section 6.7 gives what is built: 15 Built, 5 Changed, 16 Not built) and decides HM-D14. HMD-051 is proposed. R-164 decides HM-D15 (a free hub sleeps after 15 minutes with no connected device and is deleted after 60 days with no sign-in). Open: HM-D2, HM-D8, HM-D11 and HM-D13.
---

# Proposal: the hosted mode as the default first run

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the business planning session of 2026-11-16, after the operator tried release 0.2.0 on a
second Mac (report of the S12 packaging session).
Version: 7. Date: 2026-11-18. Version 2 added HM-R6 (a free hub sleeps) and the cloud design of the
business vault (Section 3, fact 5). Version 3 added HM-R7 (Apple Pay and Google Pay), HM-R8 (no
staff access to hubs), HMD-030 and HMD-031. Version 4 adds HM-R9 (WorkOS AuthKit, which decides
HM-D3), the device-code sign-in of Section 4.1 and HMD-032 to HMD-036. HM-D4 is not ruled.
Version 12 (2026-11-20) has the new text of HMD-073 in two forms and Section 6.12, for one answer of the operator.
Version 11 (2026-11-20) records the approval of HMD-052 to HMD-072 and HMD-074 (R-266) and the decision of HM-D19 (R-267); HMD-073 is to be written again.
Version 10 (2026-11-20) adds Section 6.11 (the row `Hub`: its metrics and its use; HMD-071 to HMD-074, proposed), the open decision HM-D19 and tasks HM-T34 to HM-T36.
Version 9 records R-239 (a hosted hub in a browser at `hub.anima.ing`; HM-D17 decided) and R-240 (HM-D18 decided: the link runs as a unit too), the proposed requirements HMD-058 to HMD-070 (Sections 6.8 and 6.9), the list for one approval (Section 6.10) and tasks HM-T24 to HM-T33.
Version 8 records R-206 (the desktop application includes the daemon; HM-D16 decided), the proposed requirements HMD-052 to HMD-057 (Section 6.8), tasks HM-T16 to HM-T23, and the open decisions HM-D17 and HM-D18.
Version 7 holds the approval of HMD-001 to HMD-036 (R-163): the approved text is in Sections 6.1 to 6.6, HM-D14 is decided and HMD-051 is proposed.
Version 6 adds Section 6.7 (the revision of HMD-001 to HMD-036 for approval), R-159 and the open decisions HM-D14 and HM-D15.
Version 5 adds the task table of Section 8, task HM-T1 (R-127), HMD-037 (R-128), HMD-038 to HMD-041 with tasks HM-T2 to HM-T4 (R-129), and HMD-043 with tasks HM-T6 and HM-T7 (R-130).
Status: the header of this document holds the state (PLN-020). History: Draft. The direction is decided by the operator (Section 2). The requirements
HMD-001 to HMD-041 and HMD-043 to HMD-050 are approved (Section 6; R-163 for HMD-001 to HMD-036; no requirement has the number HMD-042); HMD-052 to HMD-072 and HMD-074 are approved (R-266); HMD-051 and HMD-073 are proposed.
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

- **HMD-011.** The server stack must run `anima-link`, and the hub must run in link mode (Section 3, fact 3). A hosted hub must have no public port other than the relay. Approved (R-163). R-241 adds the public path of the observability extensions (SEC-008; [extensions-plan.md](../plans/extensions-plan.md) Section 4.17.6): on a hosted hub it goes through the relay, so the relay stays the one public port [NEEDS CLARIFICATION: OD-OBS19]. Not approved (task OB1).
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
| HMD-016 | Not built | A self-hosted hub has a backup and a restore on a second server (FND-006; HT-024 is Deferred, R-169). A hub of the pool has no export. |
| HMD-017 | Not built | No card "Your hub" and no limits. HM-D2 is open. |
| HMD-018 | Not built | No usage line exists (HM-D2). R-159 (e) is a different case: a paid account that is past due. |
| HMD-019 | Not built | Built in part, in the Stripe sandbox only: the site saves a card and opens the customer portal (private site, Stripe plan). Not built: the application does not show the plan and does not open the site for a change. Not checked: Apple Pay and Google Pay, and the live mode. R-159 (f) is the move to live; the private plans own it. |
| HMD-020 | Built | An account has one hub: `POST /v1/hub` gives the hub that exists and makes no second one. |
| HMD-021 | Not built | No pause and no deletion of a free hub that is not used. R-164 (HM-D15) confirms the rule; the private plan of the pool service owns the build. |
| HMD-022 | Built | HM-T1 (a428809e, 6659c3c7); HT-022 Passed. The Linux startup correction with no chosen hub is built on `fix/linux-start-sign-in` (`665fb16a`), not merged (2026-11-19). Its local AppImage showed account sign-in in a virtual X11 display without `LD_PRELOAD`; 4,835 client tests, client lint and type checks, and Clippy passed. The full gates stopped at `builder::tests::carried::tree::s12_d_1_the_work_folder_is_made_anew_with_mode_0700_and_a_planted_link_is_not_followed` (`BuilderError::Bundle`, under the machine's umask 002). The operator tested sign-in and resume on the local preview. HT-045 Passed (2026-11-19). |
| HMD-023 | Built | HM-T1: one state for each step, and an error with a retry (`apps/client/src/routes/welcome/hosted/`). |
| HMD-024 | Not built | The private service does not sleep a hub by use (its plan, T7: not started, with an open decision for what "use" is). R-164 (HM-D15) says what "use" is: a device connected on the link. |
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

HM-D15 (a free hub that is not used: HMD-021, HMD-024) is decided by R-164. Open:

- HM-D2 (the usage line: HMD-017, HMD-018) and HM-D8 (the numbers of HMD-024 and HMD-026).
- HM-D11 and HM-D13; no requirement of this list waits for them.

### 6.8 The daemon of a computer at a hosted hub (R-206; the text is approved, R-266)

R-206 decides that the desktop application includes the daemon, and R-240 the four points that it left open. The text of each requirement below is proposed by the session that recorded the rulings. His words cover the substance; the exact text is in the list for one approval (Section 6.10). Section 7.1 has the facts.

- **HMD-052.** The desktop application for macOS and the AppImage for Linux must carry `anima-daemon`, built from the commit of the application and signed as the other programs of the bundle. A local build (`trial-bundle.sh`) must carry it too. Approved (R-266).
- **HMD-053.** When the application opens a hosted hub, at the sign-in and at each start, it must enrol the daemon for the computer it runs on and install it as a unit of the user: a `LaunchAgent` on macOS, a systemd user unit on Linux. It must not ask for an administrator password. It must do nothing when the daemon of this computer is installed and enrolled at this hub. When a step fails, the device of this computer must say which step failed, with a button that tries again; the application must not try again by itself more than once for each start. Approved (R-266).
- **HMD-054.** The daemon of HMD-053 must reach the hub only through the Unix socket of the link of this computer (K8 Section 1, R-172). The link of HMD-058 must make the socket. The daemon must send nothing to a TCP port. Approved (R-266).
- **HMD-055.** When the application starts and the daemon that it carries is newer than the installed one, it must replace the installed copy and start the unit again, with no new enrolment. It must never install an older daemon over a newer one. Approved (R-266).
- **HMD-056.** When the person signs out of the hosted hub on a computer, or turns the daemon off in Settings, the application must stop and remove the unit, remove the copy of the daemon and the token of the daemon, and revoke the daemon at the hub. When the hub cannot be reached, it must still remove what is on the computer and say that the hub still lists the daemon. Approved (R-266).
- **HMD-057.** The installation of HMD-053 must turn on only what the daemon reports today: the metrics, the tool inventory (CAP-029) and the use counts of the AI agent programs (AIS-018, counts only). It must turn on no collector source: a source that reads the text of a file stays off until the person turns it on, with one of the three folder modes of COL-006 (R-166). The first run must say in one sentence what the daemon reports. Approved (R-266).
- **HMD-058.** On a computer that is signed in to a hosted hub, the link of the computer must run as a unit of the user (a `LaunchAgent` on macOS, a systemd user unit on Linux) that starts at the login, so that the daemon reports while the person is logged in and the application is closed (R-240 a). The application must use that link and must not start a second one. The installation, the update and the removal of the unit must follow HMD-053, HMD-055 and HMD-056. No administrator password. Approved (R-266).

### 6.9 A hosted hub in a browser at `hub.anima.ing` (R-239; the text is approved, R-266)

R-239 holds the eight answers of the operator. [hosted-web-hub.md](hosted-web-hub.md) has the options, the flow and the lists for the other repositories. The text of each requirement below is proposed; the exact text is in the list for one approval (Section 6.10).

- **HMD-059.** `https://hub.anima.ing` must be the one address of the hosted hubs in a browser. It must show nothing of a hub to a browser that is not signed in: it must send that browser to the sign-in of the Anima.ing account and take it back after the sign-in. Approved (R-266).
- **HMD-060.** After each sign-in and each sign-up at the account site, the site must send the browser to `hub.anima.ing`, unless the request names a page of the account to come back to. Approved (R-266).
- **HMD-061.** The gateway at `hub.anima.ing` must only route: it finds the hub of the account and passes the browser to it. Each hub must set and check its own browser session (form A1). A hub must make a browser session only on a request of the cloud for the account of the hub, signed with the cloud key, as it makes an invite of a device. This is a second form of the way in by the account; HMD-038 stays: the hub has no password that a person knows. Approved (R-266).
- **HMD-062.** A browser session of a hosted hub must end after 30 minutes with no request and at the latest 12 hours after the sign-in. The account page must list the open browser sessions and must be able to end each one. The sign-in must limit the attempts. At first the sign-in is by password or by the sign-on provider, with no second factor (R-239, answer 2). HMD-006 stays: when it is built, it applies in a browser as in the application. Approved (R-266).
- **HMD-063.** With a session of the gateway, the browser must show one of: "Your hub is starting" while the hub is made or wakes; the hub; a page for a paused or a deleted hub with one button; the hub with the notice to pay for an account that is past due; a page with plain words and "Try again" when no session of the hub can be made. Approved (R-266).
- **HMD-064.** An open browser session must count as use of the hub for HMD-024, and a browser sign-in as a sign-in for HMD-021. Approved (R-266).
- **HMD-065.** Browser access must be part of each hosted plan, the free plan too. Approved (R-266).
- **HMD-066.** A browser may be the first sign-in of an account. The hub is then made on that visit, and its first page must offer the application. Approved (R-266).
- **HMD-067.** Before a person other than the operator can use `hub.anima.ing`, these must be true: the review of the security of the gateway and of the browser session of the hub is done and its findings of the class "must correct" are corrected; HMD-062 is built; the gateway keeps no body of a request and gives the staff no session (HM-R8). A second factor at the browser sign-in is a later step; it is not a condition of this requirement. Approved (R-266).
- **HMD-068.** The connection of a hub to GitHub must be made once for the account: after it, each device and each browser of the account uses it, and no second sign-in to GitHub is asked for it. On a hosted hub the flow must work. Its pages are below `https://hub.anima.ing/connect/github`, which is the one callback address of each hosted hub, and the gateway passes each request to the hub of the signed-in account. Approved (R-266).
- **HMD-069.** The pool must give each hosted hub `ANIMA_PUBLIC_ORIGIN` with the value `https://hub.anima.ing`. Approved (R-266).
- **HMD-070.** A hub that runs in the link mode with no public origin of its own must refuse to start the Connect GitHub flow, with words that say that this hub has no address for a browser, and must make no ticket. (Until HMD-069 is true for a hosted hub, and for the hub of the one-click installation.) Approved (R-266).

### 6.10 The list for one approval (answered, R-266)

Answered on 2026-11-20: "Approve all but HMD-073". HMD-052 to HMD-072 and HMD-074 are approved as written. The card named two numbers for his eye, the 30 minutes with no request and the 12 hours of a browser session (HMD-062); he changed neither. HMD-073 is not approved: R-267 changes it. The table below is the list as it was put to him.

His words (R-206, R-239, R-240) decide the substance of each requirement below. The rules ask for his approval of the text before a task is built from it (PLN-007). One answer can approve all: "Approve HMD-052 to HMD-074 as written", or the same with the numbers that he wants changed.

| Requirement | In one line | The ruling that covers its substance | A point of the text that is not his word |
| --- | --- | --- | --- |
| HMD-052 | The application carries `anima-daemon`, the local build too | R-206 | None |
| HMD-053 | Enrolment and installation at the sign-in, a unit of the user, no administrator password, no question | R-206, R-240 b | At most one try again for each start |
| HMD-054 | The daemon reaches the hub only through the socket of the link | R-206 (A names the socket), R-172 | None |
| HMD-055 | The daemon is updated with the application, never to an older one | R-206 | "Never an older one" |
| HMD-056 | Removal at the sign-out and by the switch in Settings | R-240 b (the switch) | The removal at the sign-out; what occurs when the hub cannot be reached |
| HMD-057 | Only metrics, tool inventory and use counts; no collector source; one sentence | R-240 b (the sentence), R-166 | The list of what is turned on |
| HMD-058 | The link runs as a unit of the user | R-240 a | Lingering is not turned on, on Linux |
| HMD-059 | One address, always a sign-in | R-239, his words on the flow | None |
| HMD-060 | The site sends the browser to `hub.anima.ing` after a sign-in or a sign-up | R-239, his words on the flow | The exception for a named page of the account |
| HMD-061 | The gateway routes, each hub checks its own session | R-239 (1) | The cloud key as the proof of the request |
| HMD-062 | The life of a session, the list of sessions, the limit of attempts; no second factor at first | R-239 (2) | The numbers 30 minutes and 12 hours. The orchestrating session told him that these protections stay; he did not object |
| HMD-063 | The pages for each state of the hub | R-239, the flow | The words of each page |
| HMD-064 | A browser session counts as use and as a sign-in | R-239 (7) | None |
| HMD-065 | Each hosted plan, the free one too | R-239 (4) | "The free one too" is the reading of the orchestrating session of his words "all hosted plans"; it told him so |
| HMD-066 | A browser may be the first sign-in | R-239 (8) | None |
| HMD-067 | What must be true before other people use it | R-239 (2), (3) | The list of conditions |
| HMD-068 | GitHub is connected once for the account, and it works on a hosted hub | R-239 (6) | The two requirements are the reading of the orchestrating session of his words; it told him so |
| HMD-069 | The pool sets the public origin of each hosted hub | Follows from HMD-068 | All of it |
| HMD-070 | A refusal with plain words until then | Not his word. A step for the days before the gateway exists, so that no ticket goes to the old hub | All of it. It changes K-E1 by one refusal |
| HMD-071 | The daemon of the hub reports only figures of its container | His words on the hub's row; HMD-046 (R-142) | The whole text: it is the way to meet HMD-046 with nothing of the server in it |
| HMD-072 | The disk is the bytes of the data of the hub, with no total | The same | It changes HMD-046 in one point: no disk percentage on the row |
| HMD-073 | What the row shows, and that the plan and the hours are on the account page | The same; HM-D19 is open | All of it |
| HMD-074 | The hub learns nothing of another hub or of the server | The rule that the orchestrating session gave | All of it |

### 6.11 The row `Hub` of a hosted hub: its metrics and its use (HMD-071, HMD-072 and HMD-074 approved, R-266; HMD-073 to be written again)

The operator, 2026-11-20, first-hand to the orchestrating session, after the metrics of his Mac came back: "and the hub that should have metrics (and usage maybe more importantly as well)". The row `Hub` of a hosted hub shows no metrics today.

What exists:

- HMD-046 (approved, R-142) asks for the processor, the memory and the disk of the hub on that row, "read by the daemon in the container of the hub"; its task HM-T10 is not started.
- The daemon reads six files of `/proc`, `df` and `lsblk`. In a container these can show the server and not the hub: the memory and the processors of the server, its drives, its network counts. The pool session read its records for this section (2026-11-20; nothing was run on a server): whether a hub sees its own figures in `/proc` was never checked; the memory limit of a hub is a hard limit of its container; the processor of a hub has no hard limit that the container can read (it has a count of processors that it may use, and a share under load); the quota of the disk is enforced, but the file system that the hub sees is the one of the whole pool, so its total and its free space are not those of the hub, and the free space moves when other hubs write.
- The pool and the account site already hold the plan of a hub, and the sums of its hours awake and asleep with the amount for each month. The hub holds none of it and asks the pool nothing.

Proposed requirements. They change how HMD-046 is met, not what it asks for, except in one point: the disk.

- **HMD-071.** The daemon in the container of a hosted hub must report only figures of that container. Memory: the use of the container against its own limit. Processor: the processor time of the container against the count of processors that the container may use, and against a hard limit where the container has one. It must not send a figure of the server: no memory or processor count of the server, no load average, no list of drives, no network count of an interface of the server. Approved (R-266).
- **HMD-072.** For the disk, the hub must report the bytes of its own data (its database and its files), which it measures itself. It must not report the total or the free space of the file system: they are those of the pool, and the free space shows what other hubs do. Approved (R-266).
- **HMD-073.** Proposed again after R-267; not approved. The text has one part that is the same in each form, and one part with two forms, A and B, between which the operator chooses (Section 6.12).
  - Each form. The row `Hub` must show memory as used of its limit and the processor as a share of the processors that the hub may use. The page of the hub must show, besides those: the size of the data of the hub (HMD-072); what the hub keeps (the count of devices, the count of checks, the days of history); and, for the account and the month of now: the name of the plan, the disk that the plan counts as used against the limit of the plan, the hours awake and asleep, and the amount so far. The figures of the account are for the account and the month, not for one hub; an account has one hub today. Beside the amount the page must say: "So far this month, before tax. This is an estimate, not a bill. Your invoice arrives after the month ends." On a plan that is not charged the page must show the hours and "Nothing is charged on this plan" in the place of an amount. For an account that owes, the page must point to the account page and must not show what is owed as part of this month. When the figures of the account cannot be read, the page must say "Not available just now" in their place, and the rest of the page must show. The figures must be those of the account of the hub only, read-only; nothing of another account or of the server. A fault of the hub must not be able to change what the account is charged: the amount to pay comes from the usage record of the pool, which no hub reads.
  - Form A, the hub is told. The pool must give the hub the figures of its account with the key of the cloud, when the hub is made, when it wakes, and every 15 minutes while it runs. The hub must keep the last figures in memory only, must show the time of those figures, and must show "Not available just now" when it has none. The hub must never ask the pool for them. Each client reads them from the hub.
  - Form B, the hub is told nothing. The page must take the figures of the account from the account, not from the hub: in the desktop application through a route of the account site, with the sign-in of the account that the application holds; in a browser at `hub.anima.ing` through a path of the gateway on the same origin, for the account of its session. Both must give the answer of the pool with no change and in one form, which the contract defines once. The hub holds no figure of the plan, of the hours or of an amount.
- **HMD-074.** A hosted hub must learn nothing about another hub or about the server through its metrics. A test must give the daemon the files of a server beside those of a container and show that no figure of the server is in a batch. Approved (R-266).

What "use" of the hub can mean, for the question HM-D19:

| Meaning | Where it is today | Where it would show |
| --- | --- | --- |
| Use against the limits of the plan: memory, processor, the size of the data | Nowhere; HMD-071 and HMD-072 give it | The row `Hub` and its page |
| The hours awake and asleep (R-164: a free hub sleeps), and the amount for the month | The pool has the sums for each account and month; whether the account page shows them is for the session of the site | The account page |
| The use of the AI agent programs on his computers | The page Agent usage | Agent usage; the page of the hub could link to it |
| What the hub keeps: the count of devices and checks, the days of history | In the hub, not shown as one figure | The page of the hub, if he wants it |

### 6.12 HMD-073: the two forms, for one answer of the operator (2026-11-20)

R-267: he selected "Hours and amount on the hub's page", on a card that said "The hub must be told billing figures". The orchestrating session asked for the design with the least exposure and for the alternative in which the hub is told nothing. The sessions of the pool and of the site answered on 2026-11-20; nothing of this is built in any repository.

Facts from those two sessions:

- The pool computes the amount: it meters, prices and makes the invoice. It already has, for an account and a month, the seconds awake and asleep and the amount before tax. The site holds none of these figures and does not call that answer yet.
- The pool knows the disk limit of a hub. The bytes that the quota counts as used are probably in a listing that the pool already reads; that was never read on a server with the quota, so it is not verified.
- In form A the pool can push to each running hub at make, at wake and on a timer; a sleeping hub gets figures at its wake. The person is the administrator inside the container of the hub, so "in memory only" is a rule for the program, not something that can be enforced there; that is acceptable because the figures are their own.
- In form B the gateway can answer a path on the same origin for the account of its session, and the site can add one route for the application, in the pattern of its two existing routes. The request names no account and no hub, so nothing of another account can be asked for.
- In both forms the charge is safe: it comes from the usage record of the pool.

| | Form A: the hub is told | Form B: the hub is told nothing |
| --- | --- | --- |
| What he selected | This one | The alternative |
| Where a figure of money is | In the pool, and a copy in the memory of each running hub | In the pool only |
| A faulty or changed hub | Can show a wrong amount under the name of the business; the person cannot tell | Cannot: the figure comes from where the invoice comes from |
| How old the figures are | Up to 15 minutes; none after a restart until the next push; the page shows their time | Read when the page opens; a short cache |
| A hub that sleeps | Has no figures until it wakes | The account can still be shown what it used |
| The command line and an MCP tool | Can read the figures from the hub, as each other figure | Cannot, unless they sign in to the account, which they do not do today (OD-25) |
| What is new | A call of the pool to the hub (K1, the hosted contract), a timer in the pool | A route of the site for the application, a path of the gateway for the browser, one answer form for both, one more read for the token of the gateway |
| A client with no sign-in of the account | Sees the figures | Sees none, which is right for a hub that is not hosted |

The disk: HMD-072 (approved) says that the hub reports the bytes of its own data with no total. With the limit of the plan on the page, two figures are shown and must not be mixed: "data of your hub" (what the hub measures: its database and its files) and "disk used of the plan" (what the quota counts, the system of the hub included, against the limit). The second comes from the pool in each form. So HMD-072 stays as it is for the first figure, and the row `Hub` still shows no percentage of a file system. A percentage of the plan is shown only on the page, from the figure of the pool. This is true only if the pool can read the used bytes; if it cannot, the page shows the limit and the data of the hub, and no percentage.

Recommendation of the session that wrote this, with the sessions of the pool and of the site: form B for all the figures of the account, the disk of the plan too. The pool session's point: the disk figures can travel by B as well, so nothing forces a figure into the hub. Form A is buildable and safe for the charge; what it buys is one source for each client, the command line too, and a hub that could warn near its disk limit with no page open. A mixed form (B for the hours and the amount, A for the two disk figures only) gives that warning and keeps money out of the hub; it costs both sets of new parts.

The one question for him: "HMD-073 in form A, as you selected (the hub is told), in form B (the page asks the account, the hub is told nothing), or mixed (the hub is told only its two disk figures)?"

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
| HM-D15 | What occurs to a free hub that is not used? | HMD-021 pauses a free hub after 60 days with no sign-in and deletes it after a warning. HMD-024 stops it after 15 minutes with no connected device. R-159 gives a rule only for a paid account that is past due (deleted 30 days after the first failed payment). The private service does not sleep a hub by use yet: its plan has an open decision for what "use" is. | Decided (R-164, operator, 2026-11-18: "A: sleep, delete at 60"). HMD-021 and HMD-024 stand as approved. "Use" means that a device is connected on the link of the hub (the presence of HMD-047). The site decides and instructs the pool, as for a failed payment. The private plan of the pool service owns the build (its T7 and AC-D32). Not answered: HMD-027 names a heartbeat of the hub as a source of that decision, and this answer names only the presence on the link; the text of HMD-027 is not changed. |
| HM-D16 | Can the daemon of a computer of a person (`anima-daemon`, not the daemon in the container of the hub, HMD-046) enrol at a hosted hub and push to it, and by which path? | HMD-028 names "a daemon on a device" of a hosted hub. K8 says that the daemon connects only to the hub (MET-020). A computer reaches a hosted hub through the link, at a loopback address. No document shows such an enrolment, and no check ran one. The checks HT-002, HT-018, HT-026 and P-13 need it on a hosted hub ([sort-2026-11-18.md](../research/human-checks/sort-2026-11-18.md), groups 3 and 4; R-169). The sign-in of the command line to a hosted hub is a different open decision: OD-25 of the design specification. | Decided (R-206, operator, 2026-11-19; relayed twice): the desktop application includes the daemon and installs it on the computer it runs on (choice A). Section 7.1 has the facts; Section 6.8 has the proposed requirements, and tasks HM-T16 to HM-T23 the work. What he did not rule on is HM-D18. History: open on 2026-11-18; task W0 (R-172) changed the path to a Unix socket of the link (K8 Section 1). |
| HM-D17 | At which address does the browser meet a hosted hub for Connect GitHub? | The hub builds the link and the manifest of its GitHub App from `ANIMA_PUBLIC_ORIGIN`, whose default is `https://hub.anima.ing`. A hosted hub has no public address, so the browser opens the old hub ([DEF-003](../work-items/defects/DEF-003.md)). | Decided (R-239, operator, 2026-11-19; relayed): at the gateway, `https://hub.anima.ing`, which is the one callback address; the result reaches the hub of the person; the connection is made once for the account. HMD-068 and HMD-069 have the proposed text, Section 7.2 the design, tasks HM-T29 and HM-T30 the work. Not decided by it: the hub of the one-click installation, which gets no public address (DEF-003 stays open for it). |
| HM-D18 | What R-206 left open for the daemon of a computer: (a) while the application is closed; (b) a question before the installation; (c) a computer with no application; (d) the SSH pull. | R-206 decides that the application includes the daemon. The link of the application stopped with the application. | Decided (R-240, operator, 2026-11-19; relayed): (a) the link runs as a background unit of the user too, so the daemon reports while the person is logged in and the application is closed (HMD-058, task HM-T24), and this comes before the daemon ships; (b) no question: the installation at the sign-in, one sentence at the first run, a switch in Settings; (c) a one-line installation for a server, later (task HM-T33); (d) the SSH pull stays as a fallback, with no change. |
| HM-D19 | What does "usage" of the hub mean on the row `Hub`? | His words: "and the hub that should have metrics (and usage maybe more importantly as well)". Section 6.11 lists four meanings and where each figure is. No meaning of an amount to pay is assumed here. | Decided (R-267, operator, 2026-11-20; relayed). He selected three of the four meanings: "Use against plan limits", "What the hub keeps", and "Hours and amount on the hub's page". Not selected: a copy of the use of the AI agent programs on the page of the hub; the page links to Agent usage. The third is against the recommendation of this row: the card said "The hub must be told billing figures". Thus HMD-073 is written again: the row and the page show memory and processor against the limits of the hub and the size of the data against the limit of the plan; the page also shows what the hub keeps (devices, checks, days of history) and the hours awake and asleep with the amount of the month. How those figures reach the page with the least exposure, and whether the page can take them from the account with no figure in the hub, is asked of the sessions of the pool and of the site; the new text comes back to him. HMD-072 ("no total and no percentage" for the disk) changes with it if the limit of the plan is shown. |
| HM-D20 | Which address does the web client of a hosted hub give for the download of the application, and are the words of its two new texts right? | HMD-066: the first page of a hub with no computer must offer the application. The web client has no address of a download page today. Built with no link (task HM-T27): the empty list says "Install the Anima.ing application on a computer and sign in there. The computer then shows here." The sign-in page of a hosted hub in a browser says "Signed out", "Signing in" or "Sign in again", with one button. | A link to the download page of the account site, when the operator names its address; the words as built until he changes them. |

### 7.1 HM-D16: the daemon of a computer at a hosted hub (decided, R-206)

Decided on 2026-11-19 (R-206): the desktop application includes `anima-daemon` and installs it on the computer it runs on. This section holds the facts and what is still open (HM-D18). It adds no requirement: the proposed requirements are in Section 6.8 and the tasks in Section 8.

Why: at his first use of a hosted hub the operator found that the page Agent usage is empty and that no control installs a daemon ([DEF-002](../work-items/defects/DEF-002.md)), and that the uptime and the usage of his computer do not show without SSH.

Facts, from the code of `dev` at `3353f648`. The fork that put the question to him answered from the plans; each point here was read in the code:

- Most of the installation exists. The shell of the application copies the daemon that it carries to `~/.local/bin`, gives it an enrolment code on standard input, and has it install its own unit: a `LaunchAgent` on macOS, a systemd user unit on Linux (`apps/desktop/src-tauri/src/daemon.rs`, `crates/daemon/src/units.rs`). Nothing runs as root and no administrator password is asked. The button is "Install on this computer" in the panel "Daemon" of a device (MET-019, D-10 d).
- No application carried the daemon on 2026-11-19 (task HM-T16 changes that). `tauri.release.conf.json` named it as a sidecar, but no script and no workflow passed that file to the build. `trial-bundle.sh`, `check-mach-o.sh` and "What a release carries" of [releases.md](../procedures/releases.md) list six programs, and `anima-daemon` is not one of them. Thus "Install on this computer" is offered nowhere.
- The shell and the install command accept only an `https` origin on port 443 (MET-020). The daemon also accepts `unix:<path>` of a socket that `anima-link device -daemon-socket <path>` makes (task W0, R-172; K8 Section 1). The link that the application starts gets no such argument.
- "Install on this computer" is offered only when the host of the device is the VPN address of this computer (R-060). On a hosted hub the host is the link address ([DEF-005](../work-items/defects/DEF-005.md)).
- The daemon has no command that removes its unit, its copy or its token.
- The link of the application stops when the application stops (`-stop-with-stdin`). The socket of the daemon goes with it.
- On a hosted hub this computer is a device already (HMD-044), so the enrolment code has a device to belong to.

What R-206 did not decide was HM-D18. The operator decided it on 2026-11-19 (R-240):

| Point | His answer | What follows |
| --- | --- | --- |
| a. While the application is closed | "B: yes, link runs too" | The link of the computer is a unit of the user that starts at the login, as the daemon is. The application uses that link and does not start a second one. HMD-058; task HM-T24, before HM-T19. The recommendation was to build with the gap first; he decided against it |
| b. A question before the installation | "A: no question" | HMD-053 as written: the installation at the sign-in, one sentence at the first run (HMD-057), a switch in Settings (HMD-056) |
| c. A computer with no application | "A: yes, later" | Task HM-T33, after the installation by the application works |
| d. The SSH pull | "A: keep as fallback" | No change |

What a link as a unit changes, to be answered in task HM-T24 and its review (HM-T23):

- The link program must be outside the bundle: an application that is moved or updated must not break the unit. Its copy is `<application>/link/program/anima-link`, in the folder of the link that the application makes with mode 0700, and not `~/.local/bin` as the daemon's: the link refuses a program whose folder its group can write, and on Ubuntu and its derivatives `~/.local/bin` is often mode 775 (DEF-007; the operator, 2026-11-20: "the isntall should embed it do it properly"). A unit of an earlier application that runs `~/.local/bin/anima-link` is moved to the new copy at the next start, and the earlier copy is deleted.
- The key of the link is then used by a program that runs with no application open. Its state folder stays readable only by the user.
- One link for each computer and each hub: the application must find the running unit (its local port and its socket) and must not start a child of its own. When the unit is not running, the application starts it, not a second link.
- The update and the removal of the link go with those of the daemon (HMD-055, HMD-056).
- On Linux a unit of the user stops at the logout unless lingering is on. "While the person is logged in" is what R-240 asks for, so lingering is not turned on.

Security questions that the tasks must answer before the installation with no question is built (task HM-T23):

- The token of the daemon is on a computer where jobs of other people may run later. SF-004 and task W0 moved the path to a socket that only the user of the daemon can open, but a program of the same user can open it, and K8 says so. Where the token is kept on macOS and on Linux, and what a job of the same user can do with it, must be written down before a runner uses such a computer.
- The copy of the daemon in `~/.local/bin` can be replaced by a program of the same user. The shell checks the folders of the path before it hands over the code (`daemon/trust.rs`); it does not check the file after the installation.
- An installation with no question starts a program that stays after the application is closed. The first run must say so (point b).
- What the daemon reads: HMD-057.

Not part of this decision: the sign-in of the command line to a hosted hub (OD-25 of the design specification).

### 7.2 HM-D17: Connect GitHub on a hosted hub (decided, R-239)

Decided on 2026-11-19 (R-239, answer 6). His words: "The github app should only need to be signed into once. This should be able to work". The orchestrating session read them as two requirements and told him so: the connection to GitHub is made once for the account, not for each hub or device; and it must work on a hosted hub. HMD-068 has the proposed text. This section is the design; it adds no requirement.

What "once" means here:

- An account has one hosted hub. That hub makes one GitHub App by the manifest flow and gets one installation of it (K-E1 Section 7a). The key of the App stays in the hub. Each device and each browser of the account talks to that hub, so each uses the same connection. The person meets GitHub once: "Create GitHub App" and "Install".
- If an account has more than one hub later, "once" would need one App that several hubs use, so its key would have to leave a hub. That is against K-E1 and is not designed here. Today it does not occur.
- A hub that is deleted and made again is a new hub: it has no key, and the person connects again. To keep the connection over a deletion, the key would have to be kept outside the hub; not designed.

The steps on a hosted hub, with the values of each:

| Step | Who | The value | Who can read it |
| --- | --- | --- | --- |
| 1. The person starts Connect GitHub in the application or in the web client | Client to hub | A one-time ticket: one use, 5 minutes, valid only at the hub that made it | The client, the hub |
| 2. The browser opens `https://hub.anima.ing/connect/github` with the ticket | Browser to gateway | The ticket in the address. The session cookie of the gateway, if the browser has one | The gateway reads the ticket: TLS ends there. It must not log the query of an address |
| 3. No session of the gateway: the sign-in of the account, then back to the same address | Browser, account site, gateway | The session cookie of the gateway | The gateway |
| 4. The gateway passes the request to the hub of the account of the session | Gateway to hub | The ticket | The hub claims it and sets the cookie of the flow (`__Host-anima-connect`) for this browser |
| 5. The hub sends the browser to GitHub with the manifest | Browser to GitHub | The state S1. In the manifest: `redirect_url` and `setup_url` below `https://hub.anima.ing/connect/github` | GitHub |
| 6. GitHub sends the browser back to `/connect/github/created` | Browser to gateway to hub | The code of GitHub (one use) and S1 | The gateway reads the code. Who has the code can get the key of the App from GitHub once. This is the one new trust of this design, and the review (HMD-067) must look at it |
| 7. The hub trades the code at GitHub | Hub to GitHub | The key of the App, the webhook secret | The hub only |
| 8. The person presses "Install"; GitHub sends the browser back to `/connect/github/installed` | Browser to gateway to hub | The installation number and the state S2; neither is trusted, the hub asks GitHub with the App | The gateway, the hub |
| 9. Later use | Hub to GitHub | Installation tokens that the hub makes with the key, each valid 1 hour | The hub only |

A ticket of hub X that reaches the gateway in a browser of account Y goes to the hub of Y, which does not know it and refuses. So a ticket is of use only with the session of its own account.

The hub side changes little: `ANIMA_PUBLIC_ORIGIN` is `https://hub.anima.ing` (HMD-069), and the check of `Origin` on `POST /connect/github/open` then holds as it is. The cookie of the flow is per browser, and each browser reaches one hub, so the shared origin does not mix two flows.

Two cases stay outside:

- The days before the gateway exists: HMD-070, a refusal with plain words, as a small step of its own (task HM-T30). It is not the end state.
- The hub of the one-click installation on the computer of a person: it has no public address and gets none from the gateway. DEF-003 stays open for it; the choice "local port of the link" of the earlier form of this section is the candidate, with its three doubts not verified.

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
| HM-T10 | The metrics of the hub and its distribution on the row `Hub` (HMD-046): the hub makes an enrolment for its own daemon at the start, the image of the pool carries the daemon, the row shows the distribution and offers no protocol. Needs a new hub image. | R-142 | Not started. Section 6.11 changes how it is built: the daemon needs a mode for a container (task HM-T34) before the hub enrols it (task HM-T35), because the daemon of today would send figures of the server. |
| HM-T11 | The check of the presence on the link (HMD-047). The link contract gets port 446 (`ReversePort`): the link program of a computer keeps one stream open to it and writes `here` each 15 seconds; the hub link records the time of the last line and gives it as `seen` in `devices.list`. The hub reads that list each 15 seconds and keeps the PUSH check `Connected` on each device whose host is the link address of a computer that the link saw: Up while the last line is 45 seconds old or newer, and one Down report when the computer left. The hub adds the check to the device that the application added (HMD-044), when the computer is first connected, so that an application with an old link program keeps a device with no check. The hub does not count the time in which it slept or did not run: at a start and after a sleep the checks wait one whole interval again, and the scheduler takes a wait of more than 30 seconds for a tick as a new start. The check is an ordinary check: no contract change and no new hub function, so no `anima` command, no MCP tool (CMD-030) and no client change. It reaches hubs only with a new hub image of the pool, and computers only with a new application. | R-143 | In progress: built in 5a051aec; waits for HT-029. |
| HM-T12 | The path from the hub to a local port of a computer (HMD-048), for TCP: the port of the hub link, the streams that the computer keeps open, the list of open ports on the computer, the socket that the hub dials through, one connect function in the hub, and the TCP checks and SSH on it. The link contract gets port 447 (`OpenPort`): the hub link asks for a port on the stream of the reverse port, and the computer gives the connection on a new stream to port 447, only for a port of its list (the file `open-ports` of its link program; `anima-link ports` shows and changes it, 22 and 5900, 3389 on Windows only). The hub link serves the socket `dial.sock` beside its control socket, and the hub connects through it with `targets::connect` when the address is a link address. The TCP checks, the TLS checks and each SSH connection of the hub use that function. A check that needs ICMP or UDP on a computer of the link is Unknown and says that the link carries TCP only. No contract change of the hub and no new hub function, so no `anima` command, no MCP tool (CMD-030) and no client change. It reaches hubs only with a new hub image of the pool, and computers only with a new application. Until HM-T13, the command `anima-link ports` is the only way to turn a port on. | R-143 | In progress: built in f69d228d, 0b79ad39; waits for HT-031. |
| HM-T13 | The switches of the open ports (HMD-049): the shell command, the screen in the application (one or two clicks), and the text that names the system setting. | R-143 | Not started. |
| HM-T14 | VNC and RDP through the path (HMD-048): the gateway reaches a computer on the link through the hub. | R-143 | Built on the branch `hosted/hm-t14-desktop-through-link` (tests with the fake dial socket of HM-T12 and the tunnel tests of the hub; no person checked it): the check is HT-070. guacd connects by TCP only and no packet goes to a link address, so for a device whose host is a link address the hub opens a port on its own loopback address for the session, gives guacd that port in place of the address of the device, and carries each connection of it through the link with the one connect function of HM-T12. The computer still decides which of its ports are open (HMD-049): a port that is not on gives guacd a closed connection. The port closes when the session ends. This works where guacd runs on the same computer as the hub, as `anima-local-hub` runs it in the link mode; a hub with no link opens no such port. No contract change of the hub and no new hub function, so no `anima` command, no MCP tool (CMD-030) and no client change. It reaches hubs only with a new hub image of the pool. Not built: the RDP port 3389 of a Windows computer needs the link program for Windows (HM-T15); while the port of the hub is open, another program on the server of the hub could connect to it too (on a hub of the pool the hub is the only user of its server, and VNC and RDP ask for their own sign-in). |
| HM-T15 | The link program for Windows (HMD-050): `link/build.sh` builds it, the Windows application carries it. Built and not tested on Windows. | R-143 | Not started. |
| HM-T16 | The bundle carries `anima-daemon` (HMD-052): `trial-bundle.sh` (the list of programs and `externalBin`), the bundle step of the builder (`src/builder/carried/rust_programs.rs`), `check-mach-o.sh` and the check of the AppImage in the release workflow (their lists of programs), and "What a release carries" of [releases.md](../procedures/releases.md). The release map (`deploy/release/applications.txt`) needs no change: it lists only what is out, and the daemon crates are in for the desktop application. The file `tauri.release.conf.json`, which named the daemon and was used by nothing, is deleted. | R-206 | Built on the branch `hosted/hm-t16-daemon-in-bundle`, not checked: no bundle was made with it. The check is HT-049. Not Done before a release or a trial bundle shows `anima-daemon` in the application and "Install on this computer" is offered on a hub with an `https` address. |
| HM-T17 | The link makes the socket of the daemon (HMD-054): `anima-link device` runs with `-daemon-socket <path>` in a folder that only the user can reach, and the shell reports the path. Since R-240 the link is the unit of HM-T24, so this task is a part of HM-T24. | R-206, R-240 | Built on the branch `hosted/hm-t17-daemon-socket`, not checked: the check is HT-061. The shell makes the folder of the daemon with mode 0700 and gives the unit of the link `-daemon-socket <folder>/link.sock`; a folder that is not private gives no socket and the link still starts; a unit of before the socket is started again at the next start of the application. |
| HM-T18 | The installation accepts a link origin (HMD-053, HMD-054): the shell (`daemon/request.rs`) and the panel accept `unix:<path>` of HM-T17, and "Install on this computer" is offered when the device is this computer by its link address (DEF-005). Then K8 Section 3 and MET-019 name the path. | R-206 | Built on the same branch, not checked (HT-061): the shell enrols the daemon at `unix:<socket>` when the web view names the local address of the link (`http://127.0.0.1:<port>`) and the link of that port made the socket; the web view never names a path. Not built: the button "Install on this computer" for a device with a link address, because task HM-T19 installs with no button; K8 Section 3 and MET-019 are not changed yet. |
| HM-T19 | The enrolment and the installation at the sign-in and at each start (HMD-053), with the state of each step on the device of this computer. | R-206, R-240 b | Built on the branch `hosted/hm-t19-daemon-at-sign-in`, not checked: the check is HT-061. When the application opens a hosted hub, it finds the device of this computer by its link address; if its metrics source is not the daemon, it makes an enrolment code and has the shell install the daemon that the application carries, enrolled at the socket of the link, with no question. It runs once for each start; while the person is not signed in, this computer is not a device yet, or the link has no socket yet, it waits 30 seconds and looks again, five times in all. The link of a computer that was joined before is handed to its unit 45 seconds after the shell learns that the hub is a hosted one: at a sign-in, and at each start of an application that is signed in already (the first form of this task did it only at a sign-in, so on the first real run, 2026-11-19, nothing started). Each step writes a line to the log of the application. Different from HMD-053 as proposed, and not built: the device does not say which step failed and has no button to try again (a failure is quiet, and the next start tries again). Built before the security review HM-T23, against the order of this table, because the operator asked for the metrics first (2026-11-19, first-hand to the orchestrating session: "I also would really like the metrics back"); HM-T23 is still to do. |
| HM-T20 | The update of the daemon with the application (HMD-055). | R-206 | Not started. It waits for HM-T19. |
| HM-T21 | The removal (HMD-056): a command of the daemon that removes its unit, its copy and its token, and the call of it at the sign-out and from Settings, with the revocation at the hub. | R-206 | Not started. |
| HM-T22 | The words and the switch (HMD-057): the sentence of the first run, the switch in Settings, and on the page Agent usage the control that installs the daemon on this computer in the place of the sentence of DEF-002. The mockup of the page changes in the same commit (PLN-019). | R-206 | In progress. Built on the branch `hosted/hm-t22-usage-states`, not checked by a person (the check is HT-061, step 6): the state of the install of the daemon of this computer (idle, installing, installed, failed with a reason in words) as one value in the client, and the page Agent usage of a hosted hub, which says that state in the place of "Coming soon". Not built: the sentence of the first run and the switch in Settings (HMD-056, HMD-057), a button that tries again (a new start of the application tries again), and the same state on the device page. A finding of the first real run: the two units of a Mac write their log lines nowhere; a file for them needs a second value in the unit text of the daemon, whose rule for that text is strict, so it is a task of its own. |
| HM-T23 | The security review of the daemon on a computer of a person (Section 7.1, "Security questions"; SF-004; W0): the place of the token, the copy in `~/.local/bin`, a job of the same user. Its findings go to [security-findings.md](../security-findings.md). | R-206 | Not started. It comes before HM-T19. |
| HM-T24 | The link as a unit of the user (HMD-058, R-240 a), in four parts. (a) The link installs its own unit, as the daemon does: `anima-link unit install -state DIR -listen 127.0.0.1:PORT [-daemon-socket PATH]`, `unit remove` and `unit show` (`link/unit`). The unit names the program that runs the command; a program inside an application or a mounted image is refused, and a program that a different user can replace is refused. (b) The shell copies the link program out of the bundle, installs the unit, and uses the running link in place of a child of its own; the leave of a hub removes the unit. (c) The update of the link program with the application (HMD-055). (d) The check by a person. It comes before HM-T19. | R-240 | In progress. Part (a) is on `dev` and waits for HT-050. Parts (b) and (c) are built on the branch `hosted/hm-t24b-shell-link-unit` and wait for HT-059: when a hosted hub gives the device its access, the shell notes that a unit must keep the link; a join that follows copies the link program to `<application>/link/program` (until DEF-007: `~/.local/bin`), installs the unit, waits until its link answers, and then watches it in place of a child; a computer that is on the link already is marked and handed over at the next start of the application, so that the sign-in of now is not cut; when the unit cannot be made, the application runs the link itself as before. At each start the unit gets the link program of the application when the bytes differ (it follows the application in both directions: there is no version to compare, so the rule "never an older one" of HMD-055 is not met for the link). The leave of a hub removes the unit and the copy. Not built: the socket of the daemon (HM-T17), because the folder of the daemon does not exist before its enrolment; part (d) is HT-059. The link program does not build for Windows today (`link/statedir` uses calls that Windows does not have), which is not from this task: HMD-050 says that it must. |
| HM-T25 | The contract of the browser session of a hosted hub (HMD-061, HMD-062): the call of the cloud with the cloud key (K1 and the hosted contract), the cookie, the life of the session, the list and the end of sessions. | R-239 | Built on the branch `hosted/hm-t26-browser-session`: K1 (`contracts/schema.graphql`) has `hostedBrowserTicket(cloudKey)` and `hostedBrowserSignIn(ticket)`, with their codes; the database contract has the column of the idle rule. Different from the shape that was agreed with the session of the pool in one point: the cookie has `SameSite=Strict`, as the cookie of each hub has, not `Lax` (the account site and the gateway are one site, so the browser sends it after the hand-over). No list of sessions in the hub: the account page lists the sessions of the gateway (HMD-062), which is work of the pool and of the site. |
| HM-T26 | The hub side of HM-T25, with its command and its MCP tool where the plan of the command line asks for them (CMD-030). | R-239 | Built on the same branch, not checked with a gateway: the check is HT-066. The ticket is one use and 60 seconds, kept as a hash in memory. The trade is refused unless the request comes from the public origin of the hub. The session is a browser token of at most 12 hours that the hub refuses after 30 minutes with no request; an open socket counts as a request (it writes the use of its token at each check of its lease). A hosted hub also answers a request that names the host of its public origin and comes from that origin; a hub that is not hosted does not. The two calls have no command and no MCP tool, as `hostedDeviceInvite` has none: the reason is in the coverage test of the command line. Not built here: `anima-local-hub run --public-origin` and `--web-root` are on the branch `hosted/github-on-hosted-hub`; a hosted hub without them uses the default public origin, which is the address of the gateway. |
| HM-T27 | The web client on a hosted hub (HMD-059, HMD-063, HMD-066): no hub address page and no password page, "Sign out" that ends the session of the gateway too, the page that offers the application when the hub has no computer. The image of a hosted hub carries the web client. | R-239 | Built on the branch `hosted/hm-t27-web-client-door`, tested with a stand-in for the gateway, not checked with the gateway of the pool: the check is HT-NEW-webclientgateway. The web client asks one time for each page load whether a gateway is in front of its hub (the path `/_door/client`, which only the gateway answers, with 404 and the code `no_such_route`); no contract of the hub changed. Behind the gateway: (1) a refusal of the gateway (`sign_in_needed`, `hub_not_ready`, `hub_failed`, `too_many_requests`) is read as its code and shown in plain words, where it was "Can't read the hub's answer" before; (2) a browser that must sign in goes to a page with no form (`/hub-sign-in`), which goes to the entry of the gateway (`/_door/enter`) by itself one time in a minute and offers "Try again" after that, so a hub that takes no session makes no loop; (3) "Sign out" ends the session of the hub, then sends one `POST` to `/_door/sign-out` and shows "Signed out" with no new page load, because the policy of the hub allows no form (`form-action 'none'`) and a new page would start a sign-in at once; (4) the hub is marked as a hosted one in the browser, so no screen names a VPN for it; (5) the empty list of devices says to install the application and to sign in there (HMD-066). It needs a new image of the hub, because the image carries the web client. Not built: a link to the download of the application in that sentence (no address for it is decided: HM-D20); Settings still shows the rows of the hub password and of the second factor, which a hosted hub does not use; the place a browser wanted before a session ended is not kept (it opens the list of devices); the state "paused or deleted hub" and the notice to pay of HMD-063 are pages of the gateway, not of the web client. The words of the new page and of the sentence are not approved by the operator. After the operator saw the hosted hub on 2026-11-20 (branch `hosted/account-row`): on a hosted hub, Settings says "Signed in as owner with your Anima.ing account" and links to `https://cloud.anima.ing/account` ("Manage account and sign-in methods"); the hub shows no email, because the ticket of the gateway names no person (HMD-061) and the hub needs none. The Sign out of the phone layout ends the session of the gateway too, as the row of a wide window does. |
| HM-T28 | The use by a browser (HMD-064). Agreed with the session of the pool on 2026-11-19: the gateway sees each request and reports the use itself, so the hub reports nothing about browser sessions. Not built: the account site has no route for that report yet, so nothing tells the site that a browser session is open. The sleep rule of HMD-024 is not built either (private pool plan, T7), so no hub is stopped today. What stays for the hub: the presence of a device on the link (private pool plan, AC-D32). | R-239 (7) | No work in this repository. It waits for the route of the site and the report of the gateway; HMD-064 is not met before both exist. |
| HM-T29 | Connect GitHub on a hosted hub (HMD-068): nothing in the flow of the hub changes; a test runs the flow with the public origin of the gateway, and the words of the application say that the browser must be signed in at `hub.anima.ing`. The live check is an entry of [need-human-testing.md](../need-human-testing.md). | R-239 (6) | Not started. It waits for the gateway and for `--public-origin` at the pool. One more part since HM-T30: the hub must tell the application whether the browser flow can start (a field of the GitHub source in K-E1, so that a hub of before, which has no such field, counts as "cannot"), and the application then drops its own rule of DEF-003. |
| HM-T30 | The refusal of HMD-070: K-E1 gets the refusal, the hub makes no ticket, the application shows the sentence. | Not named by a ruling | Built on the branch `hosted/hm-t30-connect-refusal` (tests with the hub's routes and a database; no person checked it). A hub in the link mode that was given no `ANIMA_PUBLIC_ORIGIN` answers `CONFLICT` to `startGithubConnect` and `openGithubConnect` and makes no attempt and no ticket; the command line and an MCP tool get the same refusal. It needs a new image of the hub. The application has its own rule for the same case since DEF-003 (it offers no browser step on a hub at a port of this computer); that rule must stay until each hub refuses by itself, and it must then give way to a fact that the hub tells, so that the flow shows again when the pool gives a hub its public origin: task HM-T29. |
| HM-T31 | The default hub address of the clients (NET-005, DNS-001, INS-013): after the name moves, `hub.anima.ing` is the hosted service. What the application, the command line and the daemon do with that default must be decided and written before the name moves. For the devices of the operator that condition is answered (R-269, 2026-11-20): a device that is still on the old hub loses its hub at the move, he accepts that, and the name may move before this task is built. The task stays open for the clients of other people. | R-239 (5), R-269 | Built on the branch `hosted/hm-t31-old-clients` (tests of the three readers; no person checked it, and no old client met the real gateway). The rule: the default address stays `https://hub.anima.ing` in the command line and in the daemon, and a client that gets the refusal of the gateway there says in plain words that the address is the hosted service and gives the way on. (1) The command line: the answer `sign_in_needed` of the gateway is `UNAUTHENTICATED` with the words that this address is the hosted hub in a browser, that a hub of one's own is named with `--hub` or `ANIMA_HUB`, and that a hosted hub is opened in a browser or with the application; before, it said that the answer could not be read. (2) The daemon: `enrol` and `run` say the same for the status 401 with that body, and name `--hub` and the application. (3) The desktop application cannot read the refusal (the gateway sends no header that lets a page of a different origin read it), so it goes by the address: when the hub of its settings is `https://hub.anima.ing` and no answer comes, it says that the address is now the hosted hubs, and its sign-in leads to the first-run choices (the account, or a hub address) and not to the password page. It does not hold the move of the name (R-269). Not built: the sign-in of the command line to a hosted hub (OD-25 of the design specification); a change of the default address itself; the released application 0.2.1, which cannot be changed. The words are not approved by the operator. |
| HM-T32 | The review of the security of the gateway and of the browser session (HMD-067; R-239, answer 3). It gates the opening to other people, not the build and not the use by the operator. | R-239 (3) | Not started. It waits for HM-T26 and for the gateway. |
| HM-T33 | A one-line installation of the link and of the daemon for a computer with no application (R-240 c). | R-240 | Not started. Later, after HM-T19. |
| HM-T34 | The mode of the daemon for the container of a hosted hub (HMD-071, HMD-072, HMD-074): the figures of the container from its own control group, the bytes of the data of the hub, and nothing of the server. K8 changes (a batch with no drive list, a disk with no total), so it needs the approval of the text first (CON-001). Before the build, one read on a hub of the pool, by the operator or the orchestrating session, of what the container shows of its limits. | Not named by a ruling | Not started. HMD-071, HMD-072 and HMD-074 are approved (R-266); HMD-072 may change with the new HMD-073. It waits for one read on a hub of the pool of what the container shows of its limits. |
| HM-T35 | The hub of the pool runs its own daemon (HMD-046): the hub enrols it at its start with no step of a person, and `anima-local-hub run` starts it as one more member, only when the program is there, so that an image of before still runs. The image of the pool then carries the program (one line in the build of the image, in the private repository) and needs a new image. | R-142 (HMD-046) | Not started. It waits for HM-T34. |
| HM-T36 | The row `Hub` and its page show the three figures as HMD-073 says, and offer no protocol (HMD-046). The interface session owns the page Overview; the mockup of the row changes in the same commit (PLN-019). | Not named by a ruling | Not started. It waits for HM-T34 and for HM-D19. |
| HM-T37 | The repair of the unit of the link while the application runs (HMD-058): something outside the application can remove the unit or end its link (a cleaning tool). The loss that was seen on 2026-11-20 had a cause in the repository: the test `a_failed_swap_runs_the_link_before_again_on_its_own_port` of the shell called the join of the shell with a folder of the test, the shell took each installed unit for the unit of its folder, and ran the real `anima-link unit remove` of the computer. Corrected in the same branch: a unit is the unit of the application only when its file names the folder of the link of this application, so a test, or a second copy of the application with a different folder, removes no unit of a different folder; each removal by the application writes a line with its reason to the log. When the link that the application watches does not answer for about 30 seconds and its folder is marked for a unit, the shell installs and starts the unit again, makes the copy of the link program again when it is gone, and writes one line to the log. At most three times for each run of the application. | Not named by a ruling: a correction of HM-T24 part b, asked by the orchestrating session on 2026-11-20 after the observation | Built on the branch `hosted/hm-t24e-link-repair` (tests with a stand-in link program: the count of the looks, the unit that is installed again, a folder with no mark, a program that refuses, a unit of a different folder). Not checked on a real computer: the check is a part of HT-059. Not built: the same for the unit of the daemon, and a word on the device page while the link is down for this reason. |
| HM-T1 | Step 6 without the cloud: the first-run screens of "Sign in" in the client (HMD-022, HMD-023, HMD-037). One state for each step of Section 4.1, then the device list, and an error with a retry for cancelled, expired, no network and refused. The cloud is behind the interface `HostedAccount` in `apps/client/src/lib/hosted/`. After the sign-in, its call `access` gives one opaque `HubAccess` (what `link_join` takes, and the hub access token of this device). The app joins with it silently (`join.ts`: the link join, the token in the key store, the address). Two implementations: a stand-in for the tests and for `VITE_ANIMA_HOSTED=stand-in` (`?fail=step:problem` makes it fail once), and the real cloud, which is a stub that refuses each call. Not built: the cloud API (step 3), the hub side (step 4), the device key, the AuthKit device flow, the key store for the refresh token (HMD-034), and the opening of the system browser. Mockup: [first-run/index.html](../mockup/first-run/index.html). | R-127, R-128 | Done: built in a428809e, 6659c3c7 (screens), 875ddfc8, d2a9564b (mockup); merged to main with HM-T2 to HM-T7; HT-022 Passed (operator, 2026-11-17). |

## 9. Changes that this proposal needs in other documents

| Document | Change |
| --- | --- |
| [one-click-plan.md](../plans/one-click/one-click-plan.md) Section 1.3 | Hosted row: the shared pool, not one VPS for each customer (HM-R4). Done in the commit of this proposal, with a pointer to this file. |
| [one-click-plan.md](../plans/one-click/one-click-plan.md) OR-5 | The directory gives access to a hosted hub (HM-R2). Done in the same commit. |
| [shared-pool-hosting.md](shared-pool-hosting.md) | Remove Headscale from the instance stack; one relay for each pool host (Section 3, fact 4). Not done. |
| Design specification | A delta for the account login of a hosted hub, NET-001 and the first run. Not written. |
| S6 task breakdown | "Sign in" as the primary choice. The client does it (HM-T1); the S6 task breakdown does not yet name it. |

The update of a hosted hub (2026-11-18): the operator said that the method of today, delete and make again, "needs a better method". The pool service in the private repository owns the method. What the hub program must permit is decision OD-ROL-16 of [daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md), which is decided (R-189, 2026-11-18: "B: volume, copy, replace"): a volume for each hub, a copy of the whole root of the stopped hub, then the new image on the same volume. That section lists what the pool plan must take. Until it exists the operator deletes his own hub by hand.

## Council follow-up: first use and the service promise (2026-11-18)

R-152 names the preparation below. It changes no approved requirement or contract. Read current `main` and the claims before taking a task. A source review is not a Passed human check. The task owner records the tested build and the result; unknown results stay unknown.

| Task | Work and acceptance | Depends on | State |
| --- | --- | --- | --- |
| HM-C1 | Compare the published installer and its source commit with the hosted functions it carries. Read the release record and HT-022; keep the operator's existing sign-in proof. Produce one packet: artifact, version, checksum, included functions, missing proof. Do not infer that a development build is the published build. | Release owner; [releases.md](../procedures/releases.md); HT-022 | Not started. Preparation only (R-152) |
| HM-C2 | Prepare one pilot worksheet: ordinary download, install, sign in, own computer visible, first useful session, quit and reopen the same hub. Record assistance, exclusions and the first failed step. Successful SSH, the ended session record and a fresh read after reopen are separate observations. HM-C1 supplies the artifact; this is one pilot, not one per agent. | HM-C1; the session path in HM-T12 to HM-T15; HM-D7; HMD-015; operator permission for the participant and target | Not started. Preparation only; live execution is blocked |
| HM-C3 | Compare the explanation at sign-in with what the hub can read: private transport, server-readable credentials and client-sealed records are separate. Give the owner proposed wording with code evidence. Do not claim that a staff-access policy prevents technical access. | HMD-015; HM-D7; custody decisions of the owner | Not started. Preparation only; publishing new wording waits for owner review |
| HM-C4 | Prepare three fixed timelines for hub sleep and daemon backfill. For each, separate readings stored later from checks performed at the time. A reconciliation timestamp is discovery time unless both actual sleep endpoints are known. List the missing source of an account-scoped sleep interval. | HMD-028; private sleep/wake owner; no new public interface is approved | Not started. Preparation only; display changes are blocked |

HM-D2 and HM-D8 also need the case where an account keeps a device connected for the whole measurement period. A powered-on computer alone does not prove that its account link stayed connected. The private pool plan owns the cost and capacity calculation.
