---
kind: Proposal
state: Direction decided
evidence: Direction decided: R-207 (operator, 2026-11-19, relayed) holds his eight answers: one front door at `hub.anima.ing` in the form A1, no second factor at first, a security review before other people, each hosted plan, the name moves straight away, GitHub connected once, a browser session counts as use, a browser may be first. The requirements are HMD-059 to HMD-070 of hosted-mode.md, proposed and not approved. Nothing is built and nothing here is verified on a server.
---

# Proposal: a hosted hub in a browser at `hub.anima.ing`

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the session that recorded the defects of the first use of a hosted hub (DEF-002 to DEF-005), on the word of the orchestrating session, 2026-11-19.
Version: 3. Date: 2026-11-19. Version 3 records R-207: Section 7 has his answers, Section 6 the task lists for the other repositories and what comes off the old hub before the name moves. Version 2 added his agreement to the shape and Section 4, the flow.
Status: the header of this document holds the state (PLN-020). History: Draft; Direction decided (R-207). It is a proposal: build only a task of [hosted-mode.md](hosted-mode.md) Section 8 whose requirement is approved (PLN-007).
Map: [work-plan.md](../work-plan.md), row 4.17 (the hosted mode and the shared pool). Parent: [hosted-mode.md](hosted-mode.md). It adds no requirement identifier: the requirements are HMD-059 to HMD-070 of the parent (its Section 6.9), and the tasks of this repository are HM-T25 to HM-T32 there.

## 1. The direction

The operator, 2026-11-19: "I still would love a web hub like the old app so we can reporse hub.anima.ing for the hosted version".

Thus he wants: a person opens `https://hub.anima.ing` in a browser, signs in with the Anima.ing account, and uses the hosted hub of that account, as he used his own hub on the VPS. It is a wish and a direction. It is not a ruling on a design.

The same day the orchestrating session put the shape to him: one front door at `hub.anima.ing`, the sign-in with the Anima.ing account, and the browser passed through to the hub of the account. His answer: "yeah thats fine." On the flow: "It should be you have to sign in or when you sign in/up you are redirected to hub.anima.ing does that makes sense". That agrees with option A of Section 3. He did not choose between its forms A1 and A2, and did not answer the questions of Section 7. R-207 records these words and his eight answers to the questions of Section 7.

## 2. What exists

Read in the documents and the code of `dev` on 2026-11-19. Not verified on a server.

The old hub in a browser:

- The hub serves the web client at its own origin (`ANIMA_WEB_ROOT`; [web-client.md](../procedures/setup/web-client.md)). The web client is the same user interface code as the application (CLI-001).
- The browser signs in at the hub with the password and the TOTP code of the operator (`signInBrowser`) and gets the cookie `__Host-anima`. The hub compares the header `Origin` of each such request with `ANIMA_PUBLIC_ORIGIN`.
- It was never on the public internet. `hub.anima.ing` is a DNS record to the VPN address of the hub (`deploy/dns/`), the hub listens only on its VPN interface (NET-003) and permits only the VPN ranges (NET-001). A browser reached it only on a computer or a phone that was on the VPN. The VPN was the outer wall, and the password with TOTP the inner one.

A hosted hub today:

- It has no public address. It answers on a port of its own container on a server of the pool. A computer reaches it only through the link, at a local port of that computer, after the sign-in of the application (HMD-037, HMD-048).
- It has no way in other than the account: it refuses `login` and `signInBrowser`, and a device gets its token by a claim through the link (HMD-038 to HMD-040, R-129, R-130). No path gives a browser a session.
- The service of the pool reaches a hub only with one fixed command in the container of the hub; it has no network route to the hub (private pool plan: ACL-022, AC-D28). The pool server gets HTTPS and a name of its own for the calls of the site (private pool plan, task T24). Thus the pool can route nothing from the internet to a hub today.
- A free hub sleeps (HM-R6). Something must wake it before a browser can use it.

The second factor:

- R-159: "C: required later". The sign-in stays one step. HMD-006: an account must have a second factor before the first device with remote access is added to its hub.
- R-163 approved the definition of remote access (HM-D14): a second factor is needed before a port of a computer of the link is turned on, and before SSH, VNC, RDP or a credential is added to a device of the hub.
- HMD-006 is not built ([hosted-mode.md](hosted-mode.md) Section 6.7).
- The staff of Anima.ing must not open a session on a hub of a customer (HM-R8, HMD-031).

What holds the name `hub.anima.ing` today:

- The old hub on the VPS, with its certificate (DNS-01) and its DNS record.
- The default address of each client: the application (`DEFAULT_HUB_ADDRESS`, NET-005, DNS-001), the command line (INS-013) and the daemon. The name is in about 200 places of the code and its tests.
- The default of `ANIMA_PUBLIC_ORIGIN` of each hub, which is the cause of [DEF-003](../work-items/defects/DEF-003.md).
- The old hub also checks, signs and publishes each release: it holds the updater key (R-117, R-158; [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md)).

## 3. The options

For each: what is open to the public internet, what a stolen browser session gives, and what the rule of the second factor needs before it may be turned on.

### A. One address: a gateway at `hub.anima.ing`

The pool, or a server in front of it, answers at `https://hub.anima.ing`. It signs the person in with the Anima.ing account (AuthKit), finds the hub of the account, wakes it, and passes the browser to it. Each hub serves its own web client, so the client always agrees with its hub.

Two forms of the pass decide how safe it is:

- A1, the gateway routes and the hub authenticates. The gateway asks the hub for a browser session of the account, as the site asks for an invite of a device today (`hostedDeviceInvite`, with the cloud key). The hub sets its own cookie and checks it on each request. A fault of the routing then shows the sign-in refusal of a different hub, not its content.
- A2, the gateway authenticates for the hub. The hub trusts each request that the gateway passes. Less to build. A fault of the gateway opens each hub. Not recommended.

| | |
| --- | --- |
| Open to the internet | One name, one certificate, one program: the gateway. No hub listens on a public interface. But through the gateway each hub of the pool can be reached by a person who holds a session for it |
| A stolen session | The whole hub of that account from any computer: the devices, the terminal and the screen sessions, the use of the stored credentials. It is remote access in the full sense of HM-D14. Today a stolen token of a device is of use only on a computer that is on the link of the hub |
| The second factor | By the letter of HMD-006, only when the hub has a device with remote access. That is too weak here: a browser session can add such a device. This option needs a second factor (a passkey or TOTP) at each browser sign-in from the first day, a short life of the session, a list of the open sessions in the account with "Sign out", and a limit of the sign-in attempts. HMD-006 must be built first |
| For | It is what he asked for. One name that a person can remember. `ANIMA_PUBLIC_ORIGIN` is the same true value for each hosted hub, so the default is right again. The certificate and the DNS are one record. A phone and a computer with no application get the product |
| Against | The first public door to the hubs of all customers; the gateway is the one place whose fault matters most. TLS ends at the gateway, so a program of the business reads each request of each hub; the business hosts the hub and its database already, so it is not a new trust, but it is a new place where content is in memory, and HM-R8 must be kept by the design of the gateway (no log of bodies, no staff session). The pool needs a network route from the gateway to each hub, which changes ACL-022. The hub needs a new path for a browser session (K1 and the hosted contract change). A sleeping hub must wake on a request from the internet, so a visitor with a session costs money. All hubs share one origin in the browser: what the client keeps in the storage of the browser is shared by two accounts on one browser |

### B. A name for each hub

Each hub gets a name, for example `https://<hub>.hub.anima.ing`. The pool routes by the name. Each hub is its own origin. `hub.anima.ing` itself is only the door: it signs the person in and sends the browser to the name of the hub.

| | |
| --- | --- |
| Open to the internet | One name for each hub, behind one wildcard record and certificate at the pool. The sign-in page of each hub can be reached by each person who knows or guesses its name |
| A stolen session | As A, for that one hub |
| The second factor | As A |
| For | A clean separation in the browser: one origin, one hub, its own cookie and storage. Two checks by construction: the name routes, the hub authenticates. Each hub has a true public origin of its own, so the Connect GitHub flow (K-E1 Section 7a) works with no change |
| Against | It is not literally "the hosted hub at `hub.anima.ing`": the person lands at a different name. A wildcard certificate whose key is at the pool. More names to guard (a name of a deleted hub must never go to a new hub). The same changes of the pool and of the hub as A1 |

### C. No browser access: the application only

| | |
| --- | --- |
| Open to the internet | Nothing new |
| A stolen session | Nothing new: a device token is of use only on the link |
| The second factor | As decided (R-159, R-163) |
| For | No new surface and no new work in the pool. The design of today stays whole |
| Against | It is not what he asked for. A phone needs the mobile application; a computer with no application has no way in. Connect GitHub needs a different answer (HM-D17) |

### Recommendation, and what he decided

Decided (R-207): option A in the form A1. The text below is the recommendation as it was put to him. He did not take one part of it: the second factor at each browser sign-in (Section 7, answer 2).

A1: one address, the gateway routes and each hub authenticates. It is the only option that gives him the name he asked for, and with A1 it keeps two checks as B does. Three conditions before it is turned on for a person other than the operator: HMD-006 is built and a browser sign-in always needs the second factor; the gateway keeps no body of a request and gives the staff no session (HM-R8); and a review of the security of the gateway and of the browser session of the hub, by a person or a second model, although R-159 chose no review for the pool: this is the first public door.

If the shared origin of A is judged a risk after that review, B is the fallback, with `hub.anima.ing` as its door.

## 4. The flow

From his words (Section 1): `hub.anima.ing` always needs a sign-in, and a person who signs in or signs up at the account site is sent on to `hub.anima.ing`. Written for option A1. Each "must" here is a proposed behaviour, not a requirement.

The two entries:

1. A person opens `https://hub.anima.ing` and has no session there. The gateway shows nothing of a hub. It sends the browser to the sign-in of the account (AuthKit, at the account site), with the place to come back to. After the sign-in the browser comes back to `hub.anima.ing` with a session of the gateway.
2. A person signs in or signs up at the account site (`anima.ing`), for any reason. After it, the site sends the browser to `hub.anima.ing`. The page of the account (plan, payment, devices, sessions) stays reachable from the hub and from the site; a link that names a page of the account, for example a checkout, must still end at that page and not at the hub.

Then, at `hub.anima.ing`, with a session of the gateway, one of these:

| State of the account | What the browser shows | Notes |
| --- | --- | --- |
| The first sign-up: no hub yet | "Your hub is starting", in the browser, as the application shows it (HMD-025, HMD-037). The gateway asks for the hub as the application does (`POST /v1/hub`), waits, then passes the browser to the hub | Today the application makes the hub at its first sign-in. With this flow a browser can be first. The computer of the person is then not a device yet: the hub shows itself (HMD-045) and a way to get the application |
| A hub that runs | The hub | The gateway asks the hub for a browser session of the account, the hub sets its cookie, and the web client loads |
| A free hub that sleeps (HMD-024, R-164: it stops 15 minutes after the last device left the link) | "Your hub is starting", then the hub | The visit wakes it, as the opening of the application does (HMD-025). Two rules of today do not know a browser: "use" is a device connected on the link, so the pool would stop the hub 15 minutes later while the person works in the browser; and the delete after 60 days counts sign-ins of the application. An open browser session must count as use, and a browser sign-in as a sign-in (question 7) |
| A hub that is paused, or deleted after 60 days (HMD-021) | A page that says so, with what is kept and one button: start a new hub, or wake the paused one | Never a blank error |
| A paid account that is past due (R-159 e; HMD-051, proposed) | The hub, with the notice to pay and a link to the account site, until the hub is deleted after 30 days; then the page of the row above | HMD-018: slow down, do not delete data, above the usage line |
| No session of the hub can be made (the hub does not start, the pool does not answer) | A page with plain words and "Try again" | It names no server and shows no detail of the pool |

The second factor in this path (R-207, answer 2: "later password and SSO for now"):

- At first a browser sign-in is by password or by the sign-on provider, with no second factor. The first sign-up in a browser is: sign up, "Your hub is starting", the hub.
- What stays in the design, because it costs a person no step (HMD-062): a session ends after 30 minutes with no request and at the latest after 12 hours; the account page lists the open sessions with "Sign out"; the sign-in limits the attempts.
- HMD-006 is not changed: an account needs a second factor before the first device with remote access is added. It is not built. When it is built, the hub refuses that step in a browser session as in the application until the account has one.
- A second factor at the browser sign-in is a planned later step. Until then a stolen browser session, or a stolen password of an account that has no sign-on provider, is the whole hub of that account from any computer, for the life of the session. The review (answer 3) must say whether that is acceptable for other people; HMD-067 has the conditions.

Signing out: "Sign out" in the web client ends the session of the hub and of the gateway. The account page lists the open browser sessions with "Sign out" for each.

## 5. What it settles besides

- The callback address of the GitHub App ([DEF-003](../work-items/defects/DEF-003.md), HM-D17). With A, `redirect_url` and `setup_url` are below `https://hub.anima.ing/connect/github` for each hosted hub, and the gateway passes the browser to the hub of the session. The person must be signed in at the gateway in that browser, which the flow can ask for. With B, each hub has its own address and the flow does not change. In both cases the choice "local port of the link" of HM-D17 is not needed for a hosted hub, and the one-time ticket stays between the browser, the gateway and the hub. The same holds for each later provider that sends a browser back to the hub (OAuth of a source).
- A webhook of GitHub or of a different provider could reach a hub later. It is not part of this proposal: it is a request with no person and no session, and needs its own design.
- A phone browser: the product without the mobile application.
- A computer without the application: a borrowed computer, or a system for which there is no build (Windows today).
- Not settled: the hub of the one-click installation on the computer of a person has no public address and gets none from this. DEF-003 stays open for it.
- Not settled: the daemon of a computer (R-206) and the path from the hub to a port of a computer (HMD-048). A browser session shows what the hub has; it adds no path to a device.

## 6. The task lists for the other repositories

For their owners. Nothing there is changed by this proposal; the orchestrating session hands the lists on. The requirement numbers are those of [hosted-mode.md](hosted-mode.md) Section 6.9.

### 6.1 `anima-cloud` (the pool and the gateway)

1. The gateway at `https://hub.anima.ing`: TLS for the name, the session of the gateway, the sign-in by the account site, the route to the hub of the account (HMD-059, HMD-061). It routes only: it does not check a session of a hub.
2. A network route from the gateway to each hub, and nothing else to a hub. ACL-022 and AC-D28 of the private plan change: today the service reaches a hub only with one fixed command in its container.
3. The request of a browser session at the hub for the account, signed with the cloud key (the hub side is task HM-T26 of this repository; the contract is HM-T25).
4. `ANIMA_PUBLIC_ORIGIN=https://hub.anima.ing` in the environment of each hosted hub (HMD-069). It is not set today.
5. The web client in the image of a hosted hub (`ANIMA_WEB_ROOT`). Not verified whether the image carries it.
6. The wake of a sleeping hub on a visit, and "Your hub is starting" in the browser while it wakes or is made (HMD-063, HMD-066).
7. A browser session counts as use: the pool does not stop a free hub while a browser session is open (HMD-064; the hub reports it, task HM-T28).
8. No body of a request and no query of an address in a log (the ticket and the code of GitHub pass in an address, Section 7.2 of the parent). No session for the staff (HM-R8).
9. A limit of requests for each address and each account.
10. The pages for a paused, a deleted and a past-due hub, and for a hub that does not start (HMD-063), or their hand-over to the account site.
11. A name of a deleted hub or account must never route to a different hub.

### 6.2 `anima-site` (the account)

1. After each sign-in and each sign-up, send the browser to `https://hub.anima.ing`, except when the request names a page of the account to come back to (HMD-060).
2. Accept the gateway as a place to come back to after the sign-in, from a fixed list of hosts, so that the sign-in cannot send a session to a foreign address.
3. Give the gateway what it needs to know the account and its hub. The WorkOS tokens stay at the site and the gateway.
4. The hub of an account that has none yet, asked for by a browser and not by the application (HMD-066).
5. The account page reachable from the hub, with the list of the open browser sessions and "Sign out" for each (HMD-062).
6. The limit of the sign-in attempts (HMD-062), if AuthKit does not give it.
7. A browser sign-in counts as a sign-in for the delete after 60 days (HMD-064, HMD-021).
8. Browser access on each hosted plan, the free one too (HMD-065).
9. Later, not now: the setup of a second factor in the path of the browser sign-in.

### 6.3 DNS and the certificate

1. The record `hub.anima.ing` goes from the VPN address of the VPS to the gateway.
2. A certificate for the name at the gateway.
3. The token with which the old hub renews the certificate of this name by DNS-01 is withdrawn in the same change.

### 6.4 The old hub on the VPS, before the name moves

R-207, answer 5: "B: move it straight away". The releases are signed in the release workflow of GitHub now ([releases.md](../procedures/releases.md), "The release workflow"), so that reason to wait is gone. For the operator, what must be taken off the old hub first; the session that wrote this did not look at the server:

1. The copy of the updater key that the old hub holds: make sure that the business vault has the key, then remove it from the hub.
2. His data: an export or a backup of the database of the hub and of its key file, if he wants any of it (devices, credentials, checks, projects, events). A hosted hub has no import of it today.
3. The GitHub App of the old hub: its callbacks are at `hub.anima.ing`. Delete the App at GitHub, or it stays as an App whose address is now the gateway.
4. Each client that has `https://hub.anima.ing` as the address of his own hub: his devices with the application, the command line and the daemons. After the move they reach the gateway. Sign them in to the hosted hub, or remove them, first.
5. The other names of the VPS do not move with this one: the VPN login server (`hs.anima.ing`) and the push channel. Each device that is still on the VPN keeps working until he turns the VPS off.
6. The tokens of the old hub at Cloudflare (DNS-01, and the publishing token if it has one): withdraw them.

### 6.5 This repository

The tasks HM-T25 to HM-T32 of [hosted-mode.md](hosted-mode.md) Section 8. Not free today: each waits for the approval of its text (Section 6.10 there), and the contract of the browser session (HM-T25) needs it by CON-001.

## 7. The questions, and his answers (R-207)

Each question was put to him on 2026-11-19 with the recommendation of the session that wrote this. The answers are relayed, not first-hand to that session.

| # | Question | Recommendation | His answer |
| --- | --- | --- | --- |
| 1 | The form of the gateway: A1 (each hub checks its own session) or A2 (the hub trusts the gateway)? | A1 | "A1: each hub checks" |
| 2 | The second factor at each browser sign-in from the first day? | Yes | "later password and SSO for now". No second factor at first. The protections that cost no step stay (HMD-062); the orchestrating session told him so |
| 3 | A review of the security of the gateway before it is open to other people? | Yes | "A: yes, before others". It gates the opening to other people, not the build and not his own use |
| 4 | Browser access for each account, or a paid function? | Each account | "all hosted plans". Read by the orchestrating session as each hosted plan, the free one too; that reading is ours |
| 5 | When does the name move? | After the releases are signed elsewhere, under a second name first | "B: move it straight away". No second name. Section 6.4 lists what comes off the old hub first |
| 6 | Connect GitHub until this is built? | Wait; build only the refusal | "The github app should only need to be signed into once. This should be able to work". Read as: once for the account, and it works on a hosted hub. The design is Section 7.2 of the parent. The refusal (HMD-070) is a separate small step for the days before the gateway, and needs his approval |
| 7 | Does an open browser session count as use, and a browser sign-in for the 60 days? | Yes | "A: yes, it counts" |
| 8 | May a browser be the first sign-in? | Yes | "A: yes" |

What is still his: the approval of the text of HMD-059 to HMD-070 ([hosted-mode.md](hosted-mode.md) Section 6.10), with the numbers 30 minutes and 12 hours of HMD-062; and when a second factor at the browser sign-in is built.
