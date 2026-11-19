---
kind: Proposal
state: Draft
evidence: Directions of the operator (2026-11-19, first-hand to the orchestrating session, relayed to the session that wrote this): he wants a hosted hub in a browser at `hub.anima.ing`, agreed to one front door with the account sign-in ("yeah thats fine."), and gave the flow of the sign-in. No ruling has a number yet, and no requirement is approved. Nothing is built and nothing here is verified on a server.
---

# Proposal: a hosted hub in a browser at `hub.anima.ing`

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

From: the session that recorded the defects of the first use of a hosted hub (DEF-002 to DEF-005), on the word of the orchestrating session, 2026-11-19.
Version: 2. Date: 2026-11-19. Version 2 adds his agreement to the shape and Section 4, the flow.
Status: the header of this document holds the state (PLN-020). History: Draft. It is a proposal: do not build from it (PLN-007).
Map: [work-plan.md](../work-plan.md), row 4.17 (the hosted mode and the shared pool). Parent: [hosted-mode.md](hosted-mode.md). It adds no requirement identifier; if the operator chooses an option, the requirements go into the parent.

## 1. The direction

The operator, 2026-11-19: "I still would love a web hub like the old app so we can reporse hub.anima.ing for the hosted version".

Thus he wants: a person opens `https://hub.anima.ing` in a browser, signs in with the Anima.ing account, and uses the hosted hub of that account, as he used his own hub on the VPS. It is a wish and a direction. It is not a ruling on a design.

The same day the orchestrating session put the shape to him: one front door at `hub.anima.ing`, the sign-in with the Anima.ing account, and the browser passed through to the hub of the account. His answer: "yeah thats fine." On the flow: "It should be you have to sign in or when you sign in/up you are redirected to hub.anima.ing does that makes sense". That agrees with option A of Section 3. He did not choose between its forms A1 and A2, and did not answer the questions of Section 7. No ruling records these words yet; the session that wrote this took no ruling number for them, because they came as a direction for a proposal.

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

### Recommendation

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

The second factor in this path:

- By R-159 and R-163 the sign-in stays one step, and a second factor is needed before the first device with remote access is added (HMD-006).
- Recommended here (question 2): the gateway asks for the second factor at each browser sign-in. A person who has none sets one up (a passkey first) between the sign-in and the first sight of the hub. The first sign-up in a browser then is: sign up, set up a passkey, "Your hub is starting", the hub.
- If he keeps the letter of HMD-006 instead: the gateway lets a person with no second factor in, and the hub refuses, in a browser session as in the application, each step that adds remote access until the account has one. A browser session of a hub that already has remote access must then still need the second factor, or the rule protects nothing.
- In both cases the second factor is checked by the account (AuthKit), and the gateway reads the result. The hub does not ask for a code.

Signing out: "Sign out" in the web client ends the session of the hub and of the gateway. The account page lists the open browser sessions with "Sign out" for each.

## 5. What it settles besides

- The callback address of the GitHub App ([DEF-003](../work-items/defects/DEF-003.md), HM-D17). With A, `redirect_url` and `setup_url` are below `https://hub.anima.ing/connect/github` for each hosted hub, and the gateway passes the browser to the hub of the session. The person must be signed in at the gateway in that browser, which the flow can ask for. With B, each hub has its own address and the flow does not change. In both cases the choice "local port of the link" of HM-D17 is not needed for a hosted hub, and the one-time ticket stays between the browser, the gateway and the hub. The same holds for each later provider that sends a browser back to the hub (OAuth of a source).
- A webhook of GitHub or of a different provider could reach a hub later. It is not part of this proposal: it is a request with no person and no session, and needs its own design.
- A phone browser: the product without the mobile application.
- A computer without the application: a borrowed computer, or a system for which there is no build (Windows today).
- Not settled: the hub of the one-click installation on the computer of a person has no public address and gets none from this. DEF-003 stays open for it.
- Not settled: the daemon of a computer (R-206) and the path from the hub to a port of a computer (HMD-048). A browser session shows what the hub has; it adds no path to a device.

## 6. What it needs in the other repositories

A list for their owners. Nothing there is changed by this proposal.

- `anima-cloud` (the pool): the gateway, or the route from a gateway to each hub (ACL-022 and AC-D28 change); the wake of a sleeping hub for a browser; the request of a browser session at the hub with the cloud key; `ANIMA_PUBLIC_ORIGIN` in the environment of each hub (it is not set today, DEF-003); the web client in the image of the hub, if the image does not carry it (not verified); limits against abuse.
- `anima-site` (the account), a list for its owner:
  - after each sign-in and each sign-up, send the browser to `hub.anima.ing`, except when the request names a page of the account to come back to;
  - accept the gateway as a place to come back to after the sign-in, and no other host (a fixed list, so that the sign-in cannot send a session to a foreign address);
  - give the gateway what it needs to know the account and its hub, and the result of the second factor; the WorkOS tokens stay at the site and the gateway;
  - the setup of a second factor (a passkey first) in the path of the first browser sign-in;
  - the page of the account reachable from the hub, with the list of the open browser sessions and "Sign out" for each;
  - the hub of an account that has none yet, asked for by a browser and not by the application; the pages for a paused, a deleted and a past-due hub;
  - a browser sign-in counts as a sign-in for the delete after 60 days (HMD-021);
  - the plan of the account, if browser access is a paid function.
- DNS and the certificate: the record `hub.anima.ing` goes from the VPN address of the VPS to the gateway; a certificate for it at the gateway (with B, a wildcard record and certificate). The old hub renews the certificate of this name by DNS-01 today; its token for that must be withdrawn in the same change.
- The old hub on the VPS: it holds the name today. Before the name moves: the releases must be checked, signed and published elsewhere, because that hub holds the updater key (R-117, R-158); a copy of the updater key must exist in the business vault; the data of the old hub that he wants must be exported; and each installed client that still has `hub.anima.ing` as a typed or default address of a self-hosted hub will reach the gateway and must get a plain page there, not an error.
- This repository, if he chooses A or B: the browser session of a hosted hub (K1, the hosted contract, HMD-038 changes, because the hub then has a second way in, which is still the account); the web client for a hosted hub (no hub address page, the hosted mark); the default hub address of the clients, which then means "the hosted service".

## 7. The questions for the operator

Each with the recommendation of the session that wrote this.

| # | Question | Recommendation |
| --- | --- | --- |
| 1 | A browser at one address (A), a name for each hub (B), or the application only (C)? He agreed to the shape of A on 2026-11-19 ("yeah thats fine."). Still his: the form, A1 (each hub checks its own session) or A2 (the hub trusts the gateway) | A1 |
| 2 | The second factor for a browser: at each browser sign-in from the first day, or only when the hub has a device with remote access (the letter of HMD-006)? | At each browser sign-in. A browser session is remote access by itself. This is stricter than "required later" (R-159), so it needs his word |
| 3 | A review of the security of the gateway before it is open to other people? In R-159 he chose no review for the pool. | Yes, for this part: it is the first public door to the hub of each customer |
| 4 | Is browser access for each account, or a function of a paid plan? A visit wakes a free hub. | Each account. A wake by a browser counts as use (HM-D15, R-164) |
| 5 | When does the name move? The old hub signs each release today. | The gateway is built and checked under a second name first. `hub.anima.ing` moves after the releases are signed elsewhere and he has what he wants from the old hub. Until then the old hub keeps the name |
| 6 | Connect GitHub until this is built: wait, or build the choice "local port of the link" of HM-D17 now? | Wait, if he chooses A or B and it is among the next work. Build only the refusal with plain words (DEF-003), so that no ticket goes to the old hub |
| 7 | Does an open browser session count as use, so that a free hub does not stop while a person works in the browser, and does a browser sign-in count for the 60 days? R-164 names only a device on the link. | Yes to both. The session of the gateway has a short life with no activity, so a forgotten tab does not keep a hub awake |
| 8 | May a browser be the first sign-in of an account, before any application? The hub then has no computer of the person. | Yes. The first page then offers the application |
