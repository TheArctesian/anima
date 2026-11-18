---
kind: Proposal, with a task table
state: Draft
evidence: Version 1 (2026-11-18). The operator asked for a plan of the release on macOS, Linux, iOS and Android on 2026-11-18; no ruling. RLS-001 to RLS-014 and OD-REL-1 to OD-REL-7 are proposed, not ruled. It serves Section 4, rows 4.1, 4.14 and 4.18. The tasks RF1 to RF4 are Not started.
---

# Proposal: one release for four platforms (macOS, Linux, iOS, Android)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this proposal (PLN-005).

| Item | Value |
| --- | --- |
| Document | Proposal: the one release path for the four targets, and the order of the work |
| Version | 1 |
| Date | 2026-11-18 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | The header of this document holds the state (PLN-020). History: Draft. A request of the operator, not a ruling. No requirement is approved. |
| Serves | Work plan Section 4, row 4.1 (the release of Anima, tasks R1 to R7), row 4.14 (the mobile clients) and row 4.18 (the update of the daemon and the hub, which holds the release record and decision OD-ROL-15) |
| Related | [releases.md](../procedures/releases.md), [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md), [daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md), [mobile-plan.md](../plans/mobile-plan.md), [s12-packaging-tasks.md](../plans/one-click/s12-packaging-tasks.md), the design specification Sections 3.9 to 3.12 |
| Identifiers | RLS-001 to RLS-014, OD-REL-1 to OD-REL-7, tasks RF1 to RF4 |

This document obeys the writing rules in Section 0 of the design specification.

## 1. The request, and what this document owns

The operator said on 2026-11-18: "we also need to be thinking about IOS, mac, linux and android release now". He asked for a plan of the release of the four platforms.

Two more sentences of the operator of 2026-11-18 apply. Each is a direction. No ruling has them.

- On the file store: "keeping it on R2 is okay".
- On the place of the release work: "We should probably switch the deploy onto the hetzner service anyway shouldn't we". No person decided which server. Section 5 uses this sentence as one input.

Other plans own parts of the release. This document does not copy them (PLN-016). It owns only:

1. The rule that one release is one version and one commit on the four targets (Section 4).
2. The comparison of the four targets: the state, the gap, the keys (Sections 2, 3 and 5).
3. The order of the work across the plans (Section 6).

| Subject | Owner |
| --- | --- |
| What is released now | [releases.md](../procedures/releases.md), "Release record" |
| The update of the desktop application, the builds, the installation, the website links | The design specification Sections 3.9 (UPD), 3.10 (BLD), 3.11 (INS) and 3.12 (SITE) |
| The corrections of the release and the first release (R1 to R8d); the release as work of a project (groups W, B, A) | [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) |
| The release record as a signed list, the daemon files, the place of the signing key (OD-ROL-15, task U2) | [daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md) |
| The mobile clients: the identifier, the key store, the signatures, the first builds (OD-M5, OD-M8, M5-1, M5-2) | [mobile-plan.md](../plans/mobile-plan.md) |
| The content of the desktop bundle | [s12-packaging-tasks.md](../plans/one-click/s12-packaging-tasks.md) |
| Each check that only a person can do | [need-human-testing.md](../need-human-testing.md) |

Why a proposal and not a change to one plan: each plan above owns one target or one part. No plan owns the rule for the four targets together or the order between the plans. Each task that has an owner got its row in that owner in the commit of this version (Section 6).

## 2. The state now

Read on 2026-11-18, on `main` at `7342500c`. "Not verified" means: no document states the fact, and this session did not test it.

| Target | Published | How it is built | How it is signed | How it is delivered | How an installed copy updates | What is checked |
| --- | --- | --- | --- | --- | --- | --- |
| macOS (one universal bundle) | 0.2.1. Source: the release record. | A builder of the operator (macOS on aarch64) claims a job that the operator started in the Builds view (BLD-001, BLD-003). The bundle carries the programs of S12 ([releases.md](../procedures/releases.md), "What a release carries"). | The builder signs with the Developer ID Application identity and notarizes (BLD-083 to BLD-085, R-116). The hub signs the updater file with the updater key (BLD-045, R-117). | `Anima.dmg` and the versioned files in the R2 bucket `anima-releases`, at `releases.anima.ing` (UPD-010, DNS-007). | The application reads its feed at start and each 4 hours, and verifies the signature (UPD-001, UPD-002). | The signature and the notarization of 0.2.0 and 0.2.1 (the release record). Open: the update of an installed application (row REL-1 of [need-human-testing.md](../need-human-testing.md)); HT-008; steps 2 and 5 of Section 6.3 of the S12 tasks. |
| Linux (`linux-x86_64`) | Nothing. The feed answered 404. Source: the release record. | The same Builds flow, on a Linux builder on x86_64 (BLD-003). No release was built there (HT-016 is open). | No signature of the operating system. The hub signs the updater file (BLD-045). | `Anima-x86_64.AppImage` at the release host (UPD-010). A person compares the SHA-256 with `SHA256SUMS` (SITE-004). | The same feed and the same check as macOS (UPD-001, UPD-002). | The automated updater test ([releases.md](../procedures/releases.md), "Proof"). Open: HT-016, row REL-1, and steps 3 and 4 of Section 6.3 of the S12 tasks. |
| iOS | Nothing. Source: [mobile-plan.md](../plans/mobile-plan.md) Section 5. | By hand, on the operator's Mac, for the simulator (mobile plan Section 2). No release recipe exists. The Builds flow has no iOS target (BLD-003). | No build with a signature exists (mobile plan, F14 and Section 2.4). `tauri.ios.conf.json` names the identifier `ing.anima.mobile` and the development team `7GDLG48PV6`. | Decided for the operator: TestFlight (OD-M8). Nothing is built for it. | The mobile shell must not load the updater plugin (MOB-007, UPD-001). The store or TestFlight updates the application. | The simulator only. No real phone (mobile plan Section 2.4). Open: HT-025, and the SSH check on a phone in [need-human-testing.md](../need-human-testing.md). |
| Android | Nothing. Source: [mobile-plan.md](../plans/mobile-plan.md) Section 5. | By hand, on the operator's Mac, for the emulator (mobile plan Section 2). No release recipe exists. The Builds flow has no Android target (BLD-003). | No release key exists in the documents. `tauri.android.conf.json` names the identifier `ing.anima.mobile`. | Decided for the operator: an APK from the release server first, a store later (OD-M8). Nothing is built for it. | Not defined. The mobile shell has no updater (MOB-007). | The emulator only. No real phone. Open: HT-025. |

Facts that apply to each target:

- The releases 0.2.0 and 0.2.1 carry the daemon under its old name `anima-agent`, with no version report. The commits of R-122 and R-126 are not in them (the release record).
- The operator selects the version in the release form. No commit holds it (BLD-001, BLD-002). `tauri.conf.json` and the Cargo workspace say `0.1.0` (INS-012).
- A release starts only from the Builds view of a hub that holds the updater key (UPD-007, R-118). That hub is the hub of the operator on his VPS (R-117). The operator will use only the hosted hub, and the VPS hub can go away later (R-130).
- The website must give one install link for the disk image and one for the AppImage (SITE-002). If the live website does this is not verified. No requirement gives a link for a phone.

## 3. The gap for each target

The goal of each row: a first release that a person who is not the operator can install and use.

"Agent" means that a coding session can build the item from an approved task. "Operator" means that only the operator can do it: an account, a key, a payment, a real device, or a server (UPD-012, VPS-005).

### 3.1 macOS

| Item | Gap | Who |
| --- | --- | --- |
| Build machine | None. One Mac builder exists. | — |
| Signing identity | None. The Developer ID and the notary profile are on the builder (R-116). | — |
| Account | None. The Apple Developer Program team exists (R-116). | — |
| Delivery | A person other than the operator has not installed the disk image from the release host (not verified). The website link is not verified. | Operator: the check. The website is in a private repository. |
| Update | The update of an installed application has no recorded result (row REL-1). | Operator |
| Content | The next release must carry `anima-daemon` with its version report (task U2 sets `ANIMA_VERSION` for the daemon). | Agent: U2, after OD-ROL-15. Operator: the release. |
| Checks by a person | Row REL-1; HT-008; HT-018; steps 2 and 5 of Section 6.3 of the S12 tasks. | Operator |

### 3.2 Linux

| Item | Gap | Who |
| --- | --- | --- |
| Build machine | A Linux builder on x86_64 with the tools of [releases.md](../procedures/releases.md). No release was built on it (HT-016). | Operator: the machine and the release (task R7). Agent: a correction when the build fails. |
| Signing identity | None is necessary. The AppImage has no signature of its own; the updater file has the signature of the hub. | — |
| Account | None. | — |
| Delivery | No file is published. | Operator: task R7 |
| Update | Not verified on a real installation (OD-18, row REL-1). | Operator |
| Checks by a person | HT-016; steps 3 and 4 of Section 6.3 of the S12 tasks; the AppImage launcher (OD-18). No document names the Linux distributions on which the AppImage must start: not verified. | Operator |

### 3.3 iOS

| Item | Gap | Who |
| --- | --- | --- |
| Function before a release | A build that leaves the operator's computers must keep the access token in the key store (MOB-006, task M5-1). A person who has only an account cannot sign in on a phone: the cloud sign-in is off there (task M3-1 says "Not on phones yet."; task M3-4 is new). The link inside the application is not proved end to end (HT-025). | Agent: M5-1, M3-4. Operator: HT-025. |
| Build machine | A Mac with Xcode. The Mac builder can be this machine. No recipe, no script and no procedure exist (task M1-3 holds the build procedure). | Agent: task M5-4 |
| Signing identity | A distribution certificate and a provisioning profile for `ing.anima.mobile`, or the automatic signing of Xcode. A key for the upload. None is in the documents. | Operator makes them. Agent never reads them. |
| Account | The Apple Developer Program team exists (R-116). An application record in App Store Connect for `ing.anima.mobile` is not in the documents: not verified. The rules for testers outside the team, and for a review before such a test, must be read at Apple before a decision. Do not state them from memory. | Operator |
| Delivery | TestFlight (OD-M8) is decided for the operator. For other persons: OD-REL-3. | Operator: the upload and the invitation of testers. Agent: the script that makes the archive. |
| Update | TestFlight or the store. No work in Anima. | — |
| Checks by a person | Install from TestFlight on a real iPhone; join a hub; open a device; a session (the end tests of stages M3 and M4 of the mobile plan). | Operator, then one more person |

### 3.4 Android

| Item | Gap | Who |
| --- | --- | --- |
| Function before a release | The key store: the `keyring` crate has no Android store (F19, task M5-1). The same sign-in gap as iOS (task M3-4). HT-025. | Agent: M5-1, M3-4. Operator: HT-025. |
| Build machine | A computer with the Android SDK, the NDK and JDK 21 (mobile plan Section 2.1). The test build ran on the Mac. A build on the Linux builder is not verified. | Agent: task M5-5 |
| Signing key | One release key for `ing.anima.mobile`. An installed copy accepts an update only when the same key signed it. If the key is lost, each person must remove the application and install it again. The key does not exist in the documents. | Operator makes it and keeps a copy that is not on a network (as UPD-012). |
| Account | None for an APK from the release host. A Google Play developer account for a store. Its price, its identity checks and its rules for a new account must be read at Google before a decision. | Operator |
| Delivery | An APK at the release host (OD-M8). No target of the Builds flow writes it, and the hub gives a builder no address for a file outside its job (BLD-061). A person must permit the installation from a browser on the phone. | Agent: task R10, after OD-REL-4 |
| Update | Not defined (OD-REL-5). | Agent, after the decision |
| Checks by a person | Install the APK from the release host on a real Android phone; install a higher version over it; the end tests of stages M3 and M4. | Operator, then one more person |

## 4. What must be the same on the four targets

These requirements are proposed. None is approved.

- **RLS-001.** A release must have one version and one commit for each target: macOS, Linux, iOS and Android. ROL-001 says this for the programs of the desktop release; this requirement adds the two mobile applications.
- **RLS-002.** A target that did not build keeps its older version (as UPD-009). A release must not wait for a store review. The release record shows the version of each target.
- **RLS-003.** The build number of the iOS application and the version code of the Android application must come from the release version by one pure rule, so that a higher version always gives a higher number. No person types the number. How Tauri makes the two numbers today is not verified.
- **RLS-004.** The daemon in each release must have the name `anima-daemon` and must report the version of the release (R-126, R-122, ROL-051). A release that carries `anima-agent` must not be made after 0.2.1.
- **RLS-005.** The release record in [releases.md](../procedures/releases.md) must have one row for each of the four targets, with the version, the state, the channel and the evidence. A target with no release says "None".
- **RLS-006.** Each file that Anima delivers by itself must be in the R2 bucket `anima-releases` under `<target>/<version>/` (BLD-060, ROL-005; the operator's words of Section 1). This includes the Android APK. A file that a store delivers is not in the bucket.
- **RLS-007.** The bucket must keep the 3 newest published versions of each target, and never delete the version that a feed names. This is BLD-086 of the branch `builds/keep-three-versions`, which is not merged (task R9). The same rule applies to the Android target.
- **RLS-008.** Each release must name, for each target, the lowest version of the operating system on which it starts. `tauri.ios.conf.json` says iOS 17.0. The values for macOS, Linux and Android are not in the documents.
- **RLS-009.** A mobile application of a release must work with the hub of that release and with the hub of the release before it (as ROL-037 for the daemon). A store can give a person a new application before the pool updates the hub, and the other way.
- **RLS-010.** Each signing key has one place, one person who makes it and one copy that is not on a network. Section 5.3 gives the table. No key is in the repository, in a log, in the environment of a build step or in a file that an agent reads (BLD-044, UPD-012, LOG-007).
- **RLS-011.** The builder that makes a mobile application must sign it with a key from the key store of that builder, as the macOS builder does with the Developer ID (BLD-083). The hub does not hold a store key or the Android release key.
- **RLS-012.** The hub must check the size and the SHA-256 of the Android APK and must check that its signature is of the release key of Anima, before it writes the stable name (as BLD-064 to BLD-066). The hub holds only the public certificate of that key.
- **RLS-013.** A release for other persons must not go out before each open check of its target in [need-human-testing.md](../need-human-testing.md) says Passed (PLN-014).
- **RLS-014.** The first release that other persons get on a phone must come after task M5-1. A development build that keeps the token in memory must never leave the operator's devices (MOB-006).

## 5. Who signs, and where the release work runs

### 5.1 The problem

The hub on the VPS of the operator holds the updater key and the R2 token, and signs each release (R-117). The operator accepted that a hub that is compromised can sign an update that each installed application trusts. With the update of machines, the same key also signs the daemon of each machine (OD-ROL-2, R-139). The operator will use only the hosted hub (R-130). Decision OD-ROL-15 of [daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md) asks who signs then. Task U2 waits for it. That plan owns the decision. This section gives the options; the row OD-ROL-15 there holds the question and the result.

The release work has two parts. The build runs on the builders of the operator (one Mac, one Linux computer); this does not change. The checks, the signature and the feed run on the hub that holds the key; this is the part that must have a place.

### 5.2 The options

| Option | What it is | Risks |
| --- | --- | --- |
| A | The VPS hub stays, as the one hub that signs. This is the recommendation that the row OD-ROL-15 had before this proposal. | The operator keeps a server that he wants to stop (R-130). That hub stays a full hub with a database and a sign-in, only to sign. The builders must stay connected to it while the operator uses a different hub each day. If the desktop application can be a builder for a hub that is not the hub of its sign-in is not verified. |
| B | A small server that only makes releases. It runs one hub with publishing on, no devices and no sessions. It holds the updater key and the R2 token. It is not a server of the pool. | One more server to update and to copy. The key moves one time, by the operator (UPD-012). A change to a server needs the operator's confirmation (VPS-005). The same builder question as option A. The trade-off of R-117 stays: if this server is compromised, it can sign. |
| C | A server of the pool, or a hosted hub, signs. | The key that signs for each installed application and each machine of each person is on a host that runs the containers of other persons. One escape from a container gives the key. The security review of the pool is open (HM-D7 of [hosted-mode.md](hosted-mode.md)). |
| D | The account service of the hosted mode (a private repository) signs. | The key is in the environment of a service that a different company runs. The release path leaves the open repository. R-107 made the release a function of Anima. |
| E | The computer of the operator signs: the hub on that computer (S4) holds the key in the key store of the operating system. | It reverses the reason of R-117: a builder releases without the key. A release needs that one computer. The key travels with a portable computer. If the local hub can publish is not verified. The benefit: no server holds the key, and the computer accepts no connection from the internet. |

The operator's sentence about the provider (Section 1) agrees with option B and with option C. The difference is important: option B is a server of its own at a provider; option C is the server that runs the hubs of customers.

### 5.3 Recommendation

This is a recommendation. It is not a decision.

- Select option B: a small server that only makes releases, apart from each server of the pool. It can be at the provider of the pool, as the operator's sentence proposes. It must not be a pool host.
- Until that server exists and one release was published from it, keep option A. Do not remove the VPS hub before then.
- Do not select option C. Option E is the second choice if the operator wants no server for this.
- Keep one updater key (OD-ROL-2 is decided). Do not make a new key for the move: a new public key needs a new installation of each application (UPD-012).

### 5.4 The keys of the four targets

The Tauri updater key signs the desktop update and, later, the release record of the daemon. It signs nothing for a phone. The stores and the phones trust different keys.

| Key | Signs | Where it is now | Where it would be | Who makes it |
| --- | --- | --- | --- | --- |
| Tauri updater key (minisign) | The updater file of macOS and Linux (BLD-045). Later the release record (ROL-004). | A secret file of the VPS hub (R-117). One copy not on a network (UPD-012). One copy as a GitHub secret that nothing reads (R-116). | The release server of Section 5.3, as two secret files. | Operator. It exists. |
| Developer ID Application certificate, and the notary profile | The macOS application and the disk image (BLD-083, BLD-084) | The login keychain of the Mac builder | No change | Operator. It exists. |
| R2 token for the bucket `anima-releases` | Nothing. It permits a write to the bucket (BLD-062). | A secret file of the VPS hub | The release server | Operator |
| Apple distribution certificate and provisioning profile for `ing.anima.mobile` | The iOS application that goes to TestFlight or to the store | They are not in the documents | The login keychain of the Mac builder (RLS-011) | Operator |
| App Store Connect key for the upload | Nothing. It permits an upload. | It is not in the documents | The keychain of the Mac builder, or the operator uploads by hand | Operator |
| Android release key | The APK that the release host delivers | It does not exist in the documents | The key store of the builder that makes the Android build, with one copy not on a network (RLS-010, RLS-011) | Operator |
| Android upload key and the signing of Google Play | The file that goes to Google Play, if a store is selected later | It does not exist | Read the rules of Google Play at the source before a decision | Operator |

## 6. The order of the work

Each row is one task that one session can take. A task that has an owner is in the task table of that owner, and this table only points to it. The state of such a task is in its owner (PLN-016). The tasks RF1 to RF4 have no other owner; their state is here. No task of this proposal can start before a ruling names it (PLN-007), except a task that is already approved in its plan.

| Order | Task | Owner | Content, in short | Needs | Check | State |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | R9 | [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) | Bring the branch `builds/keep-three-versions` to `main` (BLD-086) | — | See the owner | See the owner |
| 2 | R7 | The same plan | The Linux release, and the update check on macOS and Linux | The operator | See the owner | See the owner |
| 3 | RF1 | This proposal | The release record gets its four rows (RLS-005). Add "iOS" and "Android" with "None" and the channel of OD-M8. Change the title of [releases.md](../procedures/releases.md) from desktop releases to releases. | A ruling that approves RLS-005 | `bun run quality:documents` passes. Each target has one row. | Not started |
| 4 | U2 | [daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md) | The release record as a signed list; the daemon files with their version (RLS-004) | OD-ROL-15; R7 | See the owner | See the owner |
| 5 | U12 | The same plan | The place that signs moves as OD-ROL-15 decides | OD-ROL-15; the operator | See the owner | See the owner |
| 6 | M5-1 | [mobile-plan.md](../plans/mobile-plan.md) | The key store on iOS and on Android | — | See the owner | See the owner |
| 7 | M3-4 | The same plan | The sign-in with the Anima.ing account on a phone | HT-022 | See the owner | See the owner |
| 8 | M5-3 | The same plan | The rule for the build number and the version code (RLS-003) | A ruling for RLS-003 | See the owner | See the owner |
| 9 | M5-4 | The same plan | The script that makes the signed iOS archive for TestFlight | M5-1, M5-3; OD-REL-2; the operator's certificate | See the owner | See the owner |
| 10 | M5-5 | The same plan | The script that makes the signed Android APK | M5-1, M5-3; OD-REL-2; the operator's key | See the owner | See the owner |
| 11 | M5-2 | The same plan | Signatures and the first builds for the operator | M5-4, M5-5 | See the owner | See the owner |
| 12 | R10 | [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) | The release publishes the Android APK to the bucket, with the checks of RLS-012 | OD-REL-4; M5-5 | See the owner | See the owner |
| 13 | M5-6 | [mobile-plan.md](../plans/mobile-plan.md) | The update notice of the Android application | OD-REL-5; R10 | See the owner | See the owner |
| 14 | RF2 | This proposal | The check of RLS-009: a test runs the phone client of this release against the hub API of the release before it, from the recorded contract K1. | A ruling for RLS-009; one release that has the mobile shell | The test fails when a query of the phone client is not in the older schema. | Not started |
| 15 | RF3 | This proposal | The entries in [need-human-testing.md](../need-human-testing.md) for the first installation by a person who is not the operator, one for each target: the steps from the download to an open device. | A ruling for RLS-013; for each target, its first release | The operator sets each entry. | Not started |
| 16 | RF4 | This proposal | The first release with one version on the four targets (RLS-001). A human task. | Each row above; the operator | The release record shows one version on four rows. | Not started |

Three groups can run at the same time: the desktop group (rows 1 to 5), the mobile function group (rows 6 and 7), and the mobile release group (rows 8 to 13). Rows 14 to 16 come last.

The tasks of group A of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) later replace the desktop builder with a build that is work of a project (A2, A4). The scripts of M5-4 and M5-5 must thus be programs that a recipe can call, with no part in the desktop shell. Then group A can take them with no second version.

## 7. Open decisions

Decision OD-ROL-15 (who signs) is in [daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md); Section 5 gives its options. It blocks the most: tasks U2 and U12, and with U2 each later task of the update of machines.

| Number | Question | Result |
| --- | --- | --- |
| OD-REL-1 | Do you approve the direction: one release has one version and one commit on macOS, Linux, iOS and Android, and a target that did not build keeps its older version (RLS-001, RLS-002)? | Open. Recommendation: yes. One number makes each report of a person clear, and a store review cannot delay the desktop release. |
| OD-REL-2 | Where does the first signed build of each phone application run: by a script on the Mac of the operator, or as a new target of the Builds flow? | Open. Recommendation: by a script on the Mac first (M5-4, M5-5). The Builds flow has a fixed recipe that group A replaces (OD-W16, task A4); a new target there is work that is deleted later. |
| OD-REL-3 | How do other persons get the iOS application first: as testers in TestFlight, or from the App Store? | Open. Recommendation: testers in TestFlight first, the App Store later. OD-M8 already selects TestFlight for the operator. Read the rules of Apple for testers outside the team, and for the review, at Apple before this decision. |
| OD-REL-4 | Is the Android APK a target of the release that the hub checks and writes to the bucket (RLS-006, RLS-012, task R10), or does the operator put the file there by hand? | Open. Recommendation: a target of the release. The hub gives no person a write to a stable name (BLD-061), and the same checks then protect each target. |
| OD-REL-5 | How does an installed Android application learn of a new version, before a store? | Open. Recommendation: the application reads the feed of its target and shows a notice with a link to the APK. It installs nothing by itself; Android checks that the same key signed the new file. This changes no rule of MOB-007: the updater plugin stays out. |
| OD-REL-6 | Under which name do the phone applications go to a store: the Apple team that exists (R-116; `tauri.conf.json` names a person), or an account of a company? | Open. Recommendation: use the team that exists for TestFlight now. Decide the name before the first store listing, and read the rules of each store for a change of the account at the source. This decision does not block a TestFlight build. |
| OD-REL-7 | Does the first release for other persons on a phone wait for the link in the application (stage M3, HT-025) and for the sign-in on a phone (task M3-4)? | Open. Recommendation: yes. Without them, a person must be on the VPN of a hub and must type its address (MOB-038), which is not the one click principle (OD-M3). |

Order by what they block: OD-ROL-15; then OD-REL-1 (each requirement of Section 4); then OD-REL-2 (M5-4, M5-5); then OD-REL-4 (R10); then OD-REL-5 (M5-6); then OD-REL-3 and OD-REL-7 (the first build for other persons); then OD-REL-6 (a store listing only).

## 8. Not in this proposal

| Item | Reason |
| --- | --- |
| A Windows release, a Linux release for ARM | No machine of the operator has them (OD-ROL-8, ETH-009). |
| A package for a Linux distribution, a Homebrew cask, an apt repository | INS-008. The command line alone has R8a to R8d of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) (R-151). |
| The text, the pictures, the privacy answers and the price of a store listing | They are work of the business, in a private repository. A store listing waits for OD-REL-6. |
| The legal entity, the tax and the payment of a developer account | The same. No price is in this document. |
| The rollout and the rollback of the daemon; the update of a hosted hub by the pool | [daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md); the pool service in a private repository. |
| The install links of the website for a phone | The design specification Section 3.12 owns the website (SITE). A link for a phone is a change there, after the first phone release. |
| A change to a server, an account, a key or a store | Only the operator does these (UPD-012, VPS-005). This proposal changes none. |
| Push notifications, a tablet layout, the phone as a daemon | [mobile-plan.md](../plans/mobile-plan.md) Section 6. |
| A release that starts by itself from a push | OD-12 of the design specification. A release stays an act of the operator. |
