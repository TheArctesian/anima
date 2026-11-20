---
kind: Plan
state: In progress
evidence: Version 0.4 (2026-11-19): R-241 (operator, first-hand, 2026-11-19, 16:24 to 16:36 PT) decides OD-OBS1 to OD-OBS14 and merges the observability suite into Section 4.17 (OBS-001 to OBS-066, not approved; task OB1); the plan has 19 extensions (OD-OBS11). Version 0.5 (2026-11-19): R-242 (first-hand, 16:50 to 17:08 PT) decides OD-OBS7 and OD-OBS15 to OD-OBS20 (Dub for short links, automatic image updates, OD-E6 reversed when the extension works); OD-OBS21 to OD-OBS25 are open (the managed stacks against R-226 to R-238). R-196 (2026-11-18) withdraws AIS-009 to AIS-017 (Section 4.12.1) and adds a note at EXT-009. Version 0.3 (2026-11-18). The operator approved the engine items and Projects one at a time, not the plan as one document. On `main`: E0-1, E0-2, E0-8 without its launch briefs, the Projects extension (PRO-001 to PRO-004) and Connect GitHub (PRJ-039 to PRJ-042) (Section 4, tiers 2a and 3a). E0-4 is on `main` since 2026-11-17, without its client component. E0-5 and E0-6 are on `main` in part since the merge of the branch `engine/e0-5-places` (2026-11-18): their task table says what is built. E0-3, E0-7 after its contract and the other extensions are not on `main`. The operator decided T0 of E0-5 and of E0-6, the contract ruling and the gate on 2026-11-18 (R-144 to R-148). The index of the work items holds the state of each item ([work-items/](../work-items/README.md), "Engine", "Extensions" and "Decisions").
---

# Anima Extensions Plan

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

| Item | Value |
| --- | --- |
| Document | Extensions plan |
| Version | 0.4 |
| Date | 2026-11-19 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | The header of this document holds the state (PLN-020). History: In progress. Version 0.4 (2026-11-19) adds Section 4.17, the observability suite (R-241), changes the count of the extensions from 15 to 19 (EXT-009, Section 1.1) and adds XP-17 to XP-20 after XP-02 in XP-005. The operator did not approve the plan as one document: version 0.1 of 2026-11-03 was "For review". The operator approved the engine items and Projects one at a time, and they are built (Section 0). Version 0.2 adds Section 0 and the tasks of E0-5 to E0-7, moves the Bots proposal to its own document, and records OD-E18. It changes no requirement, except the folder name in ENG-012. Version 0.3 records the decisions of 2026-11-18 (R-144 to R-148): it changes FIL-001 and REM-008, decides OD-E1 and OD-E18, and adds OD-E19 and task T6 of E0-5. |
| Related | [design-spec.md](../spec/design-spec.md), [implementation-plan.md](../archive/phase-1/implementation-plan.md) (Phase 1, done and in the archive; XP-002 uses its task loop) |

This document obeys the writing rules in Section 0 of the design specification.

## 0. How to build from this plan

Read this section first. It says where the state is and what is next. It holds no state (PLN-016).

- The state of each engine item and each extension is in the index of the work items: [work-items/README.md](../work-items/README.md). The state of this plan is in its header and in the register.
- The order is Section 5: the engine (E0) first, then one workflow for each extension. Row 4.4 and row 4.8 of [priorities.md](../plan/priorities.md) give the tier.
- The tasks of an engine item are in a table in Section 5.1. Take the first task that is "Not started" and that no open decision blocks. Claim it first (PLN-022).
- An extension (XP-01 to XP-14) has no task table until its launch brief is written (ENG-012). Write the brief and its task table first, in this plan, then build.
- A requirement is in Section 2 (engine), Section 3 (client) or Section 4 (one section for each extension). The open decisions are in Section 7. A decision that is open blocks the tasks that name it.
- These parts are drafts and are not for a build: community extensions (Sections 2.8.1, 2.12.1 and 3.4), the project map (Section 4.2.1, XP-14), Bots ([bots.md](../proposals/bots.md)) and the default panels ([default-panels.md](../proposals/default-panels.md), which proposes a change to EXT-001, EXT-005 and EXT-008).

| Item | What is next | Blocked by |
| --- | --- | --- |
| E0-1, E0-2 | Nothing. Compare each with the text of its work item, then set its state in the index. | — |
| E0-3 | T1 and T2 (the shared redaction crate, R-166) are Done in `f03429d1`; no claim was made, their task rows hold the state. T3 and T4 (the collector) are later: its tier is 3b. | Row 4.8 for T3 and T4 |
| E0-4 | An agent: T9 (compare the hub and the tests with each rule of time and references, then set its state). Then T8, the live check, which the operator does. T7 is later work. | OD-E16 and OD-E17 for the parts that they name |
| E0-5 | An agent: T5 with the operator (HT-034, HT-040, HT-063). T2, T3 and T4 are built. | OD-E26 for a Dawarich server with no `https` |
| E0-6 | An agent: the client-sealed records of T2 (K-E5 Section 3), then T4. The operational keys of T2 are built and wait for HT-035. T3 waits for the operator. | OD-E23 for T3 |
| E0-7 | An agent: `Subscription.reminderFirings` with `anima reminder watch` and the count of the open applications (the rest of T3 and T4). Then T5 with the operator (HT-043). T1 and T2 are built; T3 and T4 are built apart from the subscription. | — |
| E0-8 | The launch briefs (ENG-012). | The engine items that a brief names |
| Writing (XP-03) | It is the sample extension of the engine (ENG-010). | E0-4 to E0-7 |

## 1. Purpose and scope

### 1.1 Purpose

This plan adds 19 extensions to Anima (15 until R-241 added the four of Section 4.17; OD-OBS11). With the extensions, Anima also keeps a record of the life and work of the operator. The device manager stays the foundation.

Anima has three layers. The model is the Obsidian application: a small foundation, and functions that each person sets on or off.

| Layer | Contents | Can be set off |
| --- | --- | --- |
| Foundation | The device manager of the design specification (devices, checks, sessions, the VPN, the hub) and the engine of Section 2 | No |
| Extensions | The 19 extensions of this plan. The Anima project builds them, and they are part of each installation. | Yes, each one (Section 2.8) |
| Community extensions | Extensions from other people. See EXT-009 and the draft direction in Section 2.8.1. | Yes |

Each hub has one operator (spec Section 1.4). Other people can install their own Anima. Each operator selects the extensions that they want, and a new installation has no extension on (EXT-001). Thus Anima must not need an extension that an operator does not want. (operator, 2026-11-13)

- **EXT-000.** Build only for the accounts, devices and services that the operator uses. Do not build a general connector catalogue, a "planned" source, or support for a service that the operator does not use. When the operator starts to use a new service, add it then.

The "Ideas from" column shows where an idea came from. Anima takes ideas from Auracle and HQ. It does not copy their scope or their code.

| No. | Extension | Ideas from | Section |
| --- | --- | --- | --- |
| 1 | Map | Auracle Places | 4.1 |
| 2 | Projects: deployments, project map and analytics | HQ Projects | 4.2 |
| 3 | Writing | HQ research notebook, Auracle note libraries | 4.3 |
| 4 | Secrets | HQ Secrets (`PLAN-secrets.md`) | 4.4 |
| 5 | Integrations (on the Settings page) | Auracle connection health states | 4.5 |
| 6 | People (a directory across all accounts) | Auracle identity resolution | 4.6 |
| 7 | Money | Auracle Money, HQ Finances | 4.7 |
| 8 | History | Auracle History (`life/` evidence) | 4.8 |
| 9 | Health | Auracle wearables and clinical records | 4.9 |
| 10 | Documents (legal and identity documents) | New | 4.10 |
| 11 | Reminders | Auracle reminders, with device notifications only | 4.11 |
| 12 | AI coding sessions | HQ coding sessions, Auracle assistants | 4.12 |
| 13 | Wardrobe | New | 4.13 |
| 14 | Library | New, with Auracle reading connectors | 4.14 |
| 15 | Model gateway | Tailscale Aperture, OpenRouter | 4.15 |


### 1.2 Changes to the design specification

The extensions change these parts of the design specification. Operator checkpoint 3 of the implementation plan applies. The operator must approve each change.

| Spec part | Change |
| --- | --- |
| 1.1 Purpose | Add a fourth function: a private record of the life and work of the operator. |
| 1.4 Out of scope | Remove nothing. Multiple operators stay out of scope. |
| 5.3 Navigation bar | Add the extension pages (Section 3.3 of this plan). |
| 5.4 Design principles, Purpose | Each screen helps the operator find a problem, get into a device, or find a fact about their own life. |
| 4.1 Network (SEC-008) | Add the model gateway routes of the hub. They are available on the VPN only (MGW-001). |
| Marketing website (SITE-006) | The website is no longer the only public part of Anima. Add the webhook receiver (Section 2.12, OD-E15). |
| 6.9 Integrations | Join with extension 5 (Section 4.5). Add the model use counts to the Prometheus endpoint (MGW-013). |
| 7 Settings page | Add an Integrations section (Section 4.5). |
| 13 Repository structure | Add the new domains and the collector (Section 6). |

### 1.3 Terms

These terms add to Section 2 of the design specification.

| Term | Meaning |
| --- | --- |
| Extension | One optional group of pages and hub domains. The operator can set each extension on or off. |
| Source | One external system that gives data to Anima, for example Strava or a bank. |
| Source account | One connection to a source. A source can have more than one account. |
| Sync run | One attempt to get data from a source account. |
| Evidence | One record on the timeline. It comes from a source and has a time. |
| Place | A named area on the map with a centre and a radius. |
| Belonging | A physical object that the operator owns, for example a book or a jacket. |
| Collector | The Anima process on an operator computer that reads local sources. |
| Sealed | Encrypted at rest with a data key (Section 2.4). |
| Client-sealed | Encrypted on the client. The hub keeps only the ciphertext. |
| Secret handle | A text that names one secret and does not contain its value, for example `anima://secrets/global/OPENROUTER_API_KEY` (SCR-011). |
| Model gateway | The hub function that sends model requests from the operator computers to an upstream and adds the key (Section 4.15). It is not the session gateway (`guacd`). |
| Upstream | One place that answers model requests: a model provider, OpenRouter, or a model server on an operator device. |

## 2. Engine (Phase E0)

All extensions use a shared engine. Build the engine first, as one programme. Then build each extension in its own independent workflow (Section 5). An extension workflow must not need a change to the engine or to a different extension.

```text
                   +-------------------- Hub --------------------+
 Collector  ------>|  sources/ --> provider normalisers           |
 (each Mac/PC)     |     |             |                          |
 Uploads    ------>|     v             v                          |
 OAuth pull <----->|  domain tables (health, money, people, ...)  |
                   |     |                                        |
                   |     v                                        |
                   |  timeline/ (Evidence)   places/   files/     |
                   |     |                      |        |        |
                   |     +--> reminders/ <------+        |        |
                   |                                     v        |
                   |  vault/ (data keys, client-sealed records)   |
                   +----------------------------------------------+
```

### 2.1 Sources (the connector framework)

The sources are the services in Section 9. Each source uses the same small shape, so that the hub schedules, stores and shows them in the same way. Keep the shape small: add a field only when a real source needs it.

- **SRC-001.** Each source must declare: an identifier, the acquisition method, the scopes, and the data that it gives.
- **SRC-002.** The acquisition methods are: OAuth pull, token pull, collector push and file upload.
- **SRC-003.** Each source account must keep: the source, a label, an instance key, the credential reference, a sync state, the last sync time, the last error and a failure count.
- **SRC-004.** The sync state must be one JSON value. Each provider normaliser must read and write only its own part of this value.
- **SRC-005.** The hub must record each sync run with the trigger, the start time, the end time and the counts: inserted, changed, unchanged and removed.
- **SRC-006.** A provider normaliser must be a pure function. It receives the provider response and the sync state. It returns records, a new sync state and a list of problems. The hub does the I/O (CQ-020).
- **SRC-007.** A backfill cursor must move only after the hub stores the records of that page.
- **SRC-008.** The hub must store source tokens with the credentials domain (SEC-003). Do not make a second credential store.
- **SRC-009.** Each source account must have a health state: OK, Starting, Stalled, Never synced, Paused or Needs reconnect. The state comes from the failure count and the last sync time.
- **SRC-010.** A source must ask for read-only scopes. If the source has no read-only scope, write the reason in the source declaration.
- **SRC-011.** The hub must keep the original record of the provider in the domain table. Canonical values come from these records. Thus a change to a mapping can run again on old data.
- **SRC-012.** A sync that runs again on the same data must write nothing. The hub compares before it writes.

The hub is not on the public internet (SEC-008). This has two results:

- **SRC-020.** OAuth must use the loopback redirect of the desktop client with PKCE (RFC 8252). The client sends the code to the hub. The hub exchanges the code. Verify for each provider that it accepts a loopback redirect.
- **SRC-021.** A source can receive webhooks through the webhook receiver (Section 2.12). A source that has no webhook, or that the receiver does not serve, is polled. The default poll interval is 15 minutes. A source can set a longer interval for its rate limit. (Changed: Anima did not receive webhooks. operator, 2026-11-13)

### 2.2 Collector

Many sources exist only on an operator computer: AI session files, an Obsidian vault, iMessage, photos. The hub cannot read them.

- **COL-001.** The collector must be a part of `anima-daemon`. It must not be a separate program (R-166). Thus a computer has one program to install, and the collector runs when the client is closed. The additions to the daemon contract (K8) and the ingest protocol (K-E2) need a contract review before the daemon reads the text of a file.
- **COL-002.** The collector must send records to the hub through the VPN only.
- **COL-003.** Each collector source must use its own ingest token. An ingest token can only add records for one source account. It cannot read data.
- **COL-004.** The collector must keep a read position for each file, so that it sends only new data.
- **COL-005.** The collector must redact secrets before it sends text (Section 2.5).
- **COL-006.** The operator must turn on each collector source. Each source has one root folder and a folder rule with one of three modes: all folders of the source, all folders but a list that the operator excludes, or only a list that the operator includes. The default mode is all folders of the source (R-166). "All folders" means the folders below the root of that source. It never means the computer. The collector must not read outside the root of the source, and it must not follow a symbolic link out of the folders that the rule permits.
- **COL-007.** If the hub is not available, the collector must keep the records in a local queue and send them later. The queue must have a size limit.
- **COL-008.** The parsers of the collector must be in a library crate with no I/O, so that tests can use fixture files.

### 2.3 Timeline

The timeline joins the records of all sources. History, Map and Reminders read it. Auracle calls this "Evidence".

- **TML-001.** The timeline must not copy data. Each domain gives an adapter that maps its table rows to Evidence.
- **TML-002.** Each Evidence entry must have: a record reference (`kind:id`, RLK-005), the source, the category, the start time, an optional end time, the basis and a list of uncertainty notes.
- **TML-003.** The basis must be one of: observed, planned, reported by the operator, or inferred.
- **TML-004.** The categories are a closed list: sleep, workout, visit, travel, code, message, post, media, reading, spending, document, device, session and note. Add a category only with a change to this plan.
- **TML-005.** When two sources record the same event, the timeline must keep both. The lower-ranked entry must point to the higher-ranked entry with `superseded_by`. Do not delete it.
- **TML-006.** An adapter must write an uncertainty note when it does not know a value, for example "end time not given by the source".
- **TML-007.** The event log of the device manager (Section 3.8 of the spec) must also be a timeline source.

The rules TML-008 to TML-012 and RLK-005 to RLK-006 come from issue #7 (operator, 2026-11-16).

- **TML-008.** An event instant that Anima machines exchange or store must be one count: microseconds since 1970-01-01T00:00:00Z (UTC), as a signed 64-bit integer. A text form, such as RFC 3339 in the API, must give the same count when it is read again. A time zone changes only how an instant is shown. It must not change the instant or an old source time.
- **TML-009.** Evidence must keep the event time that the source gives as its start time. When the hub receives a record later than the event, the received time is a separate value. Do not put a received time in place of an event time. When the event time is not known or is approximate, the adapter must write an uncertainty note.
- **TML-010.** A calendar date, a recurring schedule and its time zone are not instants. Keep their meaning. Do not change a date to an instant at midnight.
- **TML-011.** The timeline must not claim an exact order between entries from the clocks of different machines. Do not change old source events to hide clock skew. The policy for clock synchronization is not decided (OD-E16).
- **TML-012.** A replay of the same source record must not make a second entry. A record of the same event from a different source is a separate entry (TML-005).

### 2.4 Vault and data classes

The hub runs on the Hostinger VPS (OD-9). A VPS breach must not expose the most sensitive data. Thus each data type has a class.

| Class | Data | Protection |
| --- | --- | --- |
| A | Identity documents, recovery codes | Client-sealed. The hub cannot read it. |
| B | Secrets, health, money, messages, location, AI session text | Sealed on the hub with a data key for each extension. |
| C | Projects, books, clothes, writing, the people directory, own posts | Database encryption at rest only. |

- **VLT-001.** The vault key for Class A must be in the operating system key store of each client (CLI-003). The hub must never receive it.
- **VLT-002.** The operator must be able to add a client to the vault with a QR code from a client that already has the key. Recovery uses a printed recovery key.
- **VLT-003.** Class B data keys must be different from the credential key (SEC-004). Each data key must be in a separate file or Docker secret.
- **VLT-004.** The hub must record a reveal of a Class A record in the audit log. Do not log the value (LOG-007).
- **VLT-005.** A reveal of a Class A record must ask for the TOTP code again if the last TOTP code is older than 5 minutes.
- **VLT-006.** Search on sealed text must use a search index that is also sealed, or search on the client. Do not keep plain text copies for search.
- **VLT-007.** Secrets are Class B, not Class A. The hub must be able to open a secret, because the model gateway adds keys to requests (MGW-003) and each operator computer gets secrets from the hub (SCR-006, SCR-012). The operator accepts that a breach of the hub exposes the secrets. Fast rotation in one place decreases this risk (SCR-014). (operator, 2026-11-13)

### 2.5 Redaction

- **RED-001.** Anima must have one redaction function. The collector, the uploads and the AI session ingestion use it.
- **RED-002.** The redaction function must remove: API keys (Anthropic, OpenAI, GitHub, AWS, Google, Slack, Stripe), JWTs, bearer tokens, private keys, passwords in URLs and `password=` assignments.
- **RED-003.** The function must redact before it seals or clips the text.
- **RED-004.** Tests must use a fixture of known secret shapes. A new shape needs a new fixture line.

### 2.6 Files

Photos, PDFs and export archives use one file store.

- **FIL-001.** The hub must store files by content hash, as sealed pieces in a folder of the hub (a named Docker volume on a server), behind one store interface. The database keeps the metadata. (R-145; [bulk-data-placement.md](../proposals/bulk-data-placement.md) can change the place of the pieces later.)
- **FIL-002.** The hub must seal each file with the data key of its class.
- **FIL-003.** The hub must remove the location metadata (EXIF GPS) from a photo copy that it shows. It can keep the location as a field of the record, if the operator permits it.
- **FIL-004.** An export archive is the unit of deletion. If the operator deletes an archive, the hub must delete all records that came from it.
- **FIL-005.** The upload limit is 256 MB for each file. The hub must stream large archives. It must not load them into memory.

### 2.7 Places

Map, Wardrobe, Library and Reminders use places. The places domain owns them. The other domains keep only a `PlaceId`.

- **PLC-001.** A place must have: a name, a centre, a radius, a category, an icon and a list of other names.
- **PLC-002.** A place can have child places, for example "Home" > "Bedroom" > "Wardrobe". The radius applies only at the top level.
- **PLC-003.** The operator name of a place must have priority over a name from a geocoder.
- **PLC-004.** The operator must be able to merge two places and split a place. A merge must move all references to the remaining place.
- **PLC-005.** The hub must not delete a place that has references. It must ask the operator for a replacement place first.
- **PLC-006.** Each Anima site (VPN-002) must be able to link to a place, so that the map can show sites and devices.

### 2.8 Extension switches

- **EXT-001.** The operator must be able to set each extension on or off on the page Extensions, which the navigation bar shows directly above Settings (R-255). The default is off.
- **EXT-002.** When an extension is off, the hub must not poll its sources. The navigation bar must not show its page.
- **EXT-003.** When the operator sets an extension off, the hub must keep its data. A separate "Delete data" action removes the data, after a confirmation.
- **EXT-004.** Each extension must state its data class in its section of this plan.
- **EXT-005.** The foundation must operate with all extensions off. It must not read a table of an extension. Projects is an extension, not a part of the foundation.
- **EXT-006.** Two relations between extensions are permitted. "Needs": the extension cannot do its function without the other. "Works with": the extension shows more when the other is on, and operates without it. If an extension can do its primary function without the other, the relation must be "works with". (operator, 2026-11-13)
- **EXT-010.** Each extension must declare what it needs in its section of this plan and in the extension registry. The table below is the full list. A needed extension must need nothing, and the foundation must need no extension (EXT-005).
- **EXT-011.** When the operator sets an extension on, the hub must also set on each extension that it needs. The client must tell the operator before the change, for example "Model gateway also sets Secrets on".
- **EXT-012.** The hub must refuse to set an extension off while an extension that is on needs it. The message must name that extension. The hub must not set a second extension off without an instruction.
- **EXT-007.** For "works with": when two extensions are on, each can show the records of the other. They must join through record links and the shared subject keys (Section 2.10), not through a table of the other extension. When one of the two is off, the other must not show an empty section or an error for it.

| Extension | Needs | Works with |
| --- | --- | --- |
| Model gateway | Secrets | Projects |
| Project design and map (Section 4.2.1) | Projects | — |
| Secrets | — | Projects |
| AI coding sessions | — | Projects |
| Projects | — | Secrets, Model gateway, AI coding sessions, Writing |
| Status pages, Error tracking, Product analytics, Short links | — | As the table of Section 4.17 (R-241) |
| All other extensions | — | As their seams in Section 5.4 |

- **EXT-008.** At the first start, the client must ask the operator what Anima must do, and must offer sets of extensions. One set is the developer set: Projects, Secrets, Model gateway and AI coding sessions. The operator can select a set, select single extensions, or select none. The operator can change the selection in Settings at any time.
- **EXT-009.** Community extensions are a later phase. Do not start them until the 19 extensions of this plan are built (15 until R-241, OD-OBS11: "OD-OBS11: no, they count toward the 15."). The first function for other people must be a new source (Section 2.1), because a source gives records only and gets only its own token. The hub must not run the code of other people until a design for isolation and permissions is approved. (operator, 2026-11-13)

  Note (R-196, operator, 2026-11-18; OD-AXT4 and OD-AXT8 of [agent-tools-as-extensions.md](../proposals/agent-tools-as-extensions.md)): the host of agent tools may start before the 15 extensions are built, with three limits. Its catalogue holds only manifests that Anima.ing writes; no code of a tool runs in the hub; and it starts only after the operator approves its isolation and permissions design (Section 8.3 of that proposal). "Agent tools" is one new switch of the registry and is not one of the 15. Community extensions still wait for EXT-009. The requirements of that proposal are not approved.

#### 2.8.1 Community extensions

Draft for review, 2026-11-15. This direction follows EXT-009.

- Give authors a sample source, a manifest reference and sample provider responses for tests. Let an author load a local project on a development hub. Let an operator install a versioned package from a file without a public listing. Decide the package format and execution model before implementation. Consider a public gallery after private installation works. [Zed development](https://zed.dev/docs/extensions/developing-extensions), [VS Code private packages](https://code.visualstudio.com/docs/configure/extensions/extension-marketplace#_install-from-a-vsix).
- Use **Settings > Extensions** to install, enable, update, disable and remove packages. Use **Settings > Integrations** to connect accounts, grant access and see sync health. A connector can supply records after the operator connects an account. [Home Assistant setup](https://www.hacs.dev/docs/use/repositories/type/integration/), [Grafana data sources](https://grafana.com/docs/grafana/latest/datasources/).
- Show the publisher, version, review status and requested access before the operator approves a community extension. Explain which data and actions the granted access permits. Allow private or unreviewed packages with an explicit risk acknowledgement. Record the approval for the package version and the permission grants. Ask again when an update requests more access. [Obsidian community plugins](https://obsidian.md/help/community-plugins), [Chrome permissions](https://developer.chrome.com/docs/extensions/develop/concepts/declare-permissions).
- Define which access the runtime can enforce. Limit each source to the credential for its own connection. Include a way to disable an extension and revoke its grants. A review badge describes the checks on a release. Operator approval applies within the approved isolation and permissions design.
- Consider an optional security assurance service. Candidates include review of a specific release and help with incident response. Evaluate insurance with an insurer or broker as a separate decision. Scope, eligibility and coverage are open.

For background sync, see Section 2.12.1. For UI extensions, see Section 3.4. For extension actions and executable views, see Section 3.4.4. For token-only themes, see [Theme authoring and installation](ui-site-language-plan.md#2b-theme-authoring-and-installation).

### 2.9 Vision and text extraction

Wardrobe, Library, Documents, Health and Money extract fields from photos and PDFs.

- **VIS-001.** Each extraction result is a proposal. The operator must confirm it before Anima saves it.
- **VIS-002.** Extraction for Class A data must run on the client, for example with the Apple Vision framework on macOS. Do not send a Class A image to a hosted model.
- **VIS-003.** Extraction for Class B and C data can use a hosted model only if the operator permits it for that extension (OD-E4).
- **VIS-004.** The extraction function must return typed fields and a confidence value for each field.

### 2.10 Record links

Extensions refer to each other only through record links. Thus no extension waits for a different extension.

`RLK-001` to `RLK-006` had the prefix `LNK` until 2026-11-17 (R-125). `LNK` is the link of the one-click plan. The migration `2030_links_record_link.sql` still says `LNK` in a comment, because a migration that ran does not change.

- **RLK-001.** The engine must keep one `record_link` table: a source reference, a target reference (`kind:id`, RLK-005) and a link kind.
- **RLK-002.** An extension can link to a record of an extension that does not exist yet. Until the target exists, the link keeps a text label, for example the name of a person.
- **RLK-003.** Each record page must show its links with one shared component. An extension does not write code for the links of a different extension.
- **RLK-004.** The engine must give shared subject keys that every extension can use without an import: a GitHub repository name, a `PlaceId`, a `PersonId` and a date.
- **RLK-005.** A record reference is `kind:id`. Only the domain that owns the record reads the `id`. A reference is not the address of a row on one machine. A reference to the same record must not change when the record is imported again. How a second machine names the same record is not decided (OD-E17).
- **RLK-006.** A link whose target does not exist keeps its target reference and its label. It must not stop the work of its source extension, and it must not copy the record of a different domain. What a link does when its target is deleted is not decided (OD-E17).

### 2.11 Reminder engine and Integrations section

Many extensions send reminders (expiry dates, monthly uploads, failed deployments) and connect sources. Thus the engine contains:

- **ENG-001.** The reminder rule evaluation, the place state and the delivery (Section 4.11, REM-001 to REM-010). An extension adds triggers through the trigger contract (K-E6). It does not change the reminder code.
- **ENG-002.** The Settings > Integrations section (Section 4.5). An extension adds its sources to the source registry. It does not change the Integrations page.
- **ENG-003.** The navigation groups and the extension switches (Sections 2.8 and 3.3). An extension adds one entry to the extension registry.

### 2.12 Webhook receiver

Some services tell Anima about an event with a webhook: a push to GitHub, a finished deployment. The hub is not on the public internet (SEC-008). Thus a small public receiver on the same machine takes the webhook, and the hub gets it from the receiver. GitHub is the first user. Later sources and jobs use the same receiver.

- **WHK-001.** The webhook receiver must be the only part that a service can reach. It must accept a POST request on one address for each source account and must refuse all other requests. The hub must stay off the public internet (SEC-008).
- **WHK-002.** The hub must get the deliveries from the receiver through a connection that the hub opens. The receiver must not open a connection to the hub.
- **WHK-003.** The hub must check the signature of each delivery with the webhook secret of that source account before it uses the delivery. The webhook secret stays on the hub, with the credentials domain (SRC-008). The receiver must not have a key of a source and must not be able to read data of the hub.
- **WHK-004.** The receiver must keep a delivery only until the hub confirms it, and for a maximum of 7 days. It must refuse a body that is larger than a fixed limit.
- **WHK-005.** The hub must use a delivery one time only. It must keep the delivery identifier of the service and must ignore a second delivery with the same identifier.
- **WHK-006.** A webhook is a signal to read, not the record. After a delivery, the hub reads the changed item from the API of the service. A source that uses webhooks must also poll on a slow interval, so that a lost delivery is repaired.
- **WHK-007.** A source declares the events that it wants (SRC-001). The provider normaliser of the source stays a pure function (SRC-006).
- **WHK-008.** The Integrations section must show for each source account: the webhook address, the time of the last delivery and the count of refused deliveries.
- **WHK-009.** The receiver must be a separate program on the machine of the hub (OD-E15). It must listen on one public port with TLS. It must run as its own user, with no access to the hub database, the credential key or the data keys. It must write each delivery to a queue that the hub reads on the same machine. The host firewall must open only this port to the public internet, and the hub port must stay closed (NET-003, DNS-005).
- **WHK-010.** The receiver must limit the request rate for each sender address and must answer a request for an unknown address with 404 and no other text.

#### 2.12.1 Community sources in the background

Draft for review, 2026-11-15. This direction follows EXT-009. It uses Sections 8 to 10 of the [source contract](../../contracts/extensions/sources.md) and the receiver of Section 2.12.

| Software | Background model | What Anima can use |
| --- | --- | --- |
| Home Assistant | A data update coordinator polls once for several entities. It handles backoff and requests reauthentication when credentials fail. [Fetching data](https://contributors.home-assistant.io/core/integrations/fetching-data/). | One schedule and health state per connected account. |
| Nextcloud | Apps can register timed jobs or queue work once after an event. A background job can prevent parallel runs. [Background jobs](https://docs.nextcloud.com/server/stable/developer_manual/basics/backgroundjobs.html). | Use the same runner for timed work and work from events. |
| Chrome | Extension service workers wake for events and can stop when idle. Extensions persist state needed after a restart. [Service worker lifecycle](https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/lifecycle). | Persist pending work and cursors. Run a source only when it has work. |
| GitHub and Slack | They expect a quick response to each delivery. GitHub recommends queuing the work. Slack can retry an event if acknowledgement fails. [GitHub webhooks](https://docs.github.com/en/webhooks/using-webhooks/best-practices-for-using-webhooks), [Slack Events API](https://docs.slack.dev/apis/events-api/). | Acknowledge receipt after storing the delivery. Process it separately. |
| Shopify and Stripe | Shopify recommends reconciliation against its API. Both document duplicate deliveries. Stripe also documents that events can arrive out of order. [Shopify deliveries](https://shopify.dev/docs/apps/build/webhooks/verify-deliveries), [Stripe webhooks](https://docs.stripe.com/webhooks#event-delivery-behaviors). | Fetch current data and repair missed deliveries by polling. |

**Scheduled sync.** The source declares its provider, records, read scopes, interval and optional event types. Anima saves the schedule for each enabled source account. The hub runs it even when the client is closed. A community source uses the 15-minute default or a longer interval (SRC-021). The operator can pause the account or request a sync in **Settings > Integrations**.

The first community sources should use declarative requests and mappings that Anima validates. Anima supplies the supported authentication, pagination and response handling. A connector can propose a cursor or record as output. The hub validates the output and commits it through the source intake. A connector that needs executable code must wait for the isolation design of EXT-009. That design must limit execution time and memory. Give each invocation only the input and state for one account.

Schedule, webhook and manual requests use the same account claim and run limits. The hub persists pending work and the next due time. After a restart it resumes from the last committed cursor. It runs one catch-up sync for a due account. It keeps a concurrency limit across accounts and respects provider rate limits. Apply Retry-After and backoff to all triggers for the account. If credentials fail, pause automatic requests until the operator reconnects the account.

**Webhooks.** Keep the public receiver separate from the private hub (WHK-001 to WHK-004, WHK-009). The receiver accepts only configured account routes. It bounds the queue and accepts a delivery only after it stores it. A successful HTTP response confirms queue receipt. The hub checks authenticity later. The receiver runs no connector code and holds no provider credentials.

The hub uses an Anima-supported verifier and the account's webhook secret to verify the original delivery. It checks the account and the granted event types. It checks freshness when the provider signs a timestamp. In one transaction, it records the provider delivery ID for that account and sets pending sync work. Then it confirms the delivery to the receiver. On a storage failure, the hub leaves the delivery queued.

The hub drops invalid or duplicate deliveries without running the connector. It keeps delivery outcomes without copying bodies or secrets into logs (LOG-007).

A verified delivery requests a read for that account. It cannot invoke an arbitrary action. Coalesce a burst into one pending sync. Keep a follow-up request if an event arrives after the current run claims its work. Fetch current records from the provider. Keep a separate slow reconciliation deadline so a stream of webhooks cannot postpone it (WHK-006).

**Background permissions.** The operator grants access for a specific package and source account. Background work uses this grant without an operator session. Show requested access before enabling it (Section 2.8.1). Require a new grant before adding access on update.

| Grant | What it permits |
| --- | --- |
| Provider reads | Anima makes declared read requests for that account to approved HTTPS endpoints. It attaches the account credential only at its approved destination. The connector does not receive the token or webhook secret. |
| Schedule and events | Anima can run the account on its approved interval or after a verified delivery of a granted event type. The connector cannot open a listener or register a public handler. |
| Records and cursor | The connector supplies bounded records of declared types and its own sync state. Anima validates the records before storage. The grant supplies no database access or access to other accounts. |

The first background contract grants no provider writes, shell, host files, general network access, session access or UI actions. Anima checks the active package and grant before each provider request and in the transaction that stores results. Pause, disable or revocation stops new work and clears pending work for the affected grant. A stale run cannot commit after the hub invalidates its claim. When the operator resumes, reconcile from the saved cursor.

In **Settings > Integrations**, show the last trigger, last successful sync, next run and current reason for delay. Use the existing run history and webhook counters (ITG-003, WHK-008). For example, a certificate event requests a refresh of one account. The connector can supply the new expiry date. The event cannot schedule device maintenance.

## 3. Client

### 3.1 Pages

Each built-in extension adds one route in `apps/client/src/routes/`. The route composes components from `$lib/<domain>/` (CQ-044).

### 3.2 Shared components

| Component | Folder | Used by |
| --- | --- | --- |
| Map canvas (MapLibre GL) | `$lib/places/` | Map, Wardrobe, Library, Reminders, Devices |
| Timeline (month, week, day, hour) | `$lib/timeline/` | History, Health, Projects |
| Source tile and connect flow | `$lib/sources/` | Settings > Integrations and each extension empty state |
| Photo capture and upload | `$lib/files/` | Wardrobe, Library, Documents, Health, Money |
| Proposal review (VIS-001) | `$lib/files/` | All extraction flows |
| Markdown editor (CodeMirror 6) | `$lib/writing/` | Writing, Projects, notes on each record |

### 3.3 Navigation bar

The navigation bar has groups. Each group shows only the extensions that are on.

| Group | Items |
| --- | --- |
| Infrastructure | Devices, Projects, Model gateway |
| Life | History, Map, People, Health, Money |
| Belongings | Library, Wardrobe |
| Vault | Secrets, Documents |
| Work | Writing |
| Bottom, above Settings | Reminders |

Integrations is not a navigation item. It is a section of the Settings page (Section 4.5).

- **NAV-013.** The operator must be able to select the start page. The default stays Devices (NAV-004).
- **NAV-014.** A group with no active items must not show.
- **NAV-012.** The client must have a command palette (Cmd+K) that opens each page and each record type.

NAV-013 and NAV-014 had the numbers NAV-010 and NAV-011 until 2026-11-17. The design specification owns those two numbers (PLN-016).

### 3.4 Community UI extensions

Draft for review, 2026-11-15. This part follows the community source contract (EXT-009).

#### 3.4.1 Reference designs

| Software | How contributions work | What Anima can use |
| --- | --- | --- |
| VS Code | The manifest declares commands, menu placement and views. The extension registers command handlers in code. Menu conditions control when a command appears. Tree views use a data provider. A webview can render a custom editor tab or sidebar view. [Commands](https://code.visualstudio.com/api/extension-guides/command), [Contribution points](https://code.visualstudio.com/api/references/contribution-points), [Tree views](https://code.visualstudio.com/api/extension-guides/tree-view), [Webviews](https://code.visualstudio.com/api/extension-guides/webview). | Use one command ID across surfaces. Give authors a native view for structured data. |
| Obsidian | Plugins call `addCommand`, `addRibbonIcon` or `addSettingTab` from plugin code. A custom `ItemView` registers with `registerView`. The plugin can open the view in a workspace tab or a sidebar leaf. [Plugin API](https://github.com/obsidianmd/obsidian-api/blob/master/obsidian.d.ts), [Custom views](https://github.com/obsidianmd/obsidian-developer-docs/blob/main/en/Plugins/User%20interface/Views.md). | Let a command open a view. Keep the surface distinct from the action that opens it. |
| JupyterLab | A plugin registers a command with a label and an execution function. It can add the same command to the palette or a toolbar. The application shell places widgets in named areas. [Extension points](https://jupyterlab.readthedocs.io/en/stable/extension/extension_points.html). | Keep command behavior in one place. Let the shell own placement. |
| Grafana | An app declares its navigation pages in `plugin.json`. It can register links or components at named extension points. Grafana or another plugin owns the point that displays them. [App pages](https://grafana.com/developers/plugin-tools/tutorials/build-an-app-plugin), [Register content](https://grafana.com/developers/plugin-tools/how-to-guides/ui-extensions/register-an-extension). | Give extension pages their own routes. Publish a short list of places that accept contributions. |
| Shopify | A target places a block on a resource page or adds an action to a menu. The target determines the context API and the UI components available to the extension. [Admin UI extensions](https://shopify.dev/docs/api/admin-extensions/latest). | Give a Device or Project panel only the context for that surface. |
| Slack | An app publishes a Block Kit view to its Home tab or opens a modal with a structured view. Slack renders the controls. [App Home](https://docs.slack.dev/surfaces/app-home/), [Modals](https://docs.slack.dev/surfaces/modals/). | Build pages and panels from validated data and host controls. |
| Chrome | The manifest declares a side panel page, a toolbar action or keyboard commands. Chrome owns their placement. An extension page can use the browser APIs available to it. [Side panel](https://developer.chrome.com/docs/extensions/reference/api/sidePanel), [Toolbar action](https://developer.chrome.com/docs/extensions/reference/api/action), [Commands](https://developer.chrome.com/docs/extensions/reference/api/commands). | Let the host reserve top-level controls. Use the package manifest for discovery. |
| Figma | A plugin can declare menu commands. Its custom UI runs in an iframe and exchanges messages with the plugin code that has access to the document. [Manifest](https://developers.figma.com/docs/plugins/manifest/), [How plugins run](https://developers.figma.com/docs/plugins/how-plugins-run/). | If a custom view later needs code, define the message interface separately from the view. |

#### 3.4.2 The first client contract

An extension package declares its UI in a versioned manifest. Each contribution has an ID scoped to its package. The client validates the manifest against the supported UI contract version before it enables the contributions. Anima renders labels, tables, cards and links with its own controls and semantic tokens (TOK-010). The package supplies no HTML, CSS or client code for these surfaces.

| Contribution | What the author declares | Where it appears |
| --- | --- | --- |
| Command | ID, label, required context and target page or record. A context can be global, the open device or the open project. | The existing command palette. A panel or page can link to the same command ID. The session shortcut follows design specification NAV-011. |
| Panel | ID, title, a named slot, a host query and a card or table definition. | A read-only section below the built-in content on a Device or Project page. Anima gives the panel the ID of the open record. |
| Page | ID, title, host query and layout made of cards, lists or tables. | A route under `/extensions/<package>/<page>`. The operator can open it from the palette or the package details. |

The first commands navigate to pages or records. Section 3.4.4 describes commands that can make changes after this first contract.

The host supplies the current record ID. The manifest can select an approved query and bind its typed arguments to that context. The hub checks the extension switch, the query grant and the record scope for each request. A query can return approved fields from the extension's connected sources. A panel filters to records linked to the open device or project (Section 2.10).

The client loads panels when it displays their host page. It displays loading, empty and error states in the contributed section. A panel failure leaves the host page usable.

The host controls titles, navigation, focus and keyboard access. The operator chooses which contributed pages to pin if pinning is added. Disabling the extension or revoking its access removes its commands and panels. A saved link to an unavailable page shows the reason in Anima. Disabling an extension keeps its data (EXT-003).

#### 3.4.3 Author and operator example

A certificate inventory connector supplies expiry records. An author adds three declarations to the same package.

| ID | Declaration |
| --- | --- |
| `open-inventory` | A global command named **Open certificate inventory**. It opens the `inventory` page. |
| `device-certificates` | A **Certificates** panel in the Device details slot. The panel uses the open Device ID to show linked certificates. |
| `inventory` | A **Certificate inventory** page. It uses the package's approved certificate query and displays name, expiry date and status in a table. |

The author previews the package on a development hub with sample records. The preview uses the same manifest validation and controls as an installed package. With no account, the page shows a link to **Settings > Integrations**. The operator installs and approves the package in **Settings > Extensions**, then connects the account. The source imports the certificate records. The panel displays records linked to the open device. The command opens the full inventory from any Anima page where the palette is available. If the operator disables the package, the command and panel disappear.

#### 3.4.4 Commands that make changes and custom views

Draft for review, 2026-11-15. Add named actions after the first client contract. An extension can use these actions from a host-rendered panel. A custom view can use the same actions after its isolation design is approved.

| Software | Execution and permission model | Implication for Anima |
| --- | --- | --- |
| VS Code | The desktop extension host has the app's permissions. Webviews use message passing and can restrict scripts and local resources. [Runtime security](https://code.visualstudio.com/docs/configure/extensions/extension-runtime-security), [Webview security](https://code.visualstudio.com/api/extension-guides/webview#security). | Apply limits to the action handler as well as the view. |
| Obsidian | Community plugins inherit app access. Restricted mode disables them. The directory scans releases and displays a safety scorecard. [Plugin security](https://obsidian.md/help/Extending%2BObsidian/Plugin%2Bsecurity). | Show review status and give the operator a disable control. Enforce grants when the extension runs. |
| Zed | Extension code compiles to WebAssembly. Zed exposes host capabilities for operations such as starting processes and downloading files. Users can restrict those capabilities. [Development](https://zed.dev/docs/extensions/developing-extensions), [Capabilities](https://zed.dev/docs/extensions/capabilities). | Define a small set of host operations. Check each operation at the host boundary. |
| Chrome | Extensions declare API and host permissions. Optional permissions can be granted later. Extension scripts and pages use messages to request work from other extension contexts. [Permissions](https://developer.chrome.com/docs/extensions/develop/concepts/declare-permissions), [Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/messaging). | Grant access to a specific operation and resource. Validate each message at the receiver. |
| Shopify | Checkout UI code runs in a worker without the page DOM. Shopify renders provided controls and grants selected capabilities, including network access. Admin targets expose APIs for their context. [Checkout security](https://shopify.dev/docs/api/checkout-ui-extensions/latest), [Admin targets](https://shopify.dev/docs/api/admin-extensions/latest). | Use host-rendered controls when they fit. Give a custom UI only the context it needs. |
| Figma | Plugin code accesses the document inside a restricted runtime. An iframe renders custom UI and exchanges messages with that code. A manifest can restrict its network access. [How plugins run](https://developers.figma.com/docs/plugins/how-plugins-run/). | Give the view a message interface. Keep authority to change records in the host. |
| Slack | Slack signs requests for slash commands and interactions. The app verifies the request and uses an OAuth token with granted scopes for Slack API calls. [Verify Slack requests](https://docs.slack.dev/authentication/verifying-requests-from-slack/), [OAuth scopes](https://docs.slack.dev/reference/scopes/). | Verify who asked for an action. Check separately what that caller can do. |
| Grafana | Grafana documented an opt-in frontend sandbox. Its docs mark the preview deprecated. [Frontend sandbox](https://grafana.com/docs/grafana/latest/administration/plugin-management/plugin-frontend-sandbox/). | Make the isolation boundary part of the contract for executable community views. |

**Actions that the hub runs.** A manifest declares an action ID, a named Anima operation and the typed inputs it uses. Anima publishes the eligible operations, the resources they accept and the fields they return. The extension can ask the host to invoke one operation. The hub uses its existing action code to make the change. Community actions have no interface for arbitrary GraphQL, shell commands, credentials, sessions or trust decisions.

The operator grants each operation for specific resources in **Settings > Extensions**. For example, the operator can grant **Sync source account** for one connected account. A grant does not start work. Selecting a host-rendered command can start an action that needs no confirmation. A custom view can request a change. The operator confirms that request in Anima's own UI.

For every action that needs confirmation, Anima shows the package, resolved target and intended change. Apply any additional confirmation the built-in action requires.

The trusted client sends the request with the operator's session. The extension receives no session credential. The hub checks the operator, active package version, operation grant and target before it starts work. It binds confirmation to the action, target and inputs. It accepts a repeat submission of the same request at most once. A disable, revocation or package change invalidates pending requests. If work already started, Anima reports the actual result and uses the operation's existing cancellation rules.

Record the operator, package, version, operation, target and outcome within the audit rules (LOG-007, LOG-012 to LOG-020). Return only the fields the operation permits. New hub functions follow CMD-030 and CMD-031. MCP tools keep the trust, secret and session limits (MCP-004 to MCP-006).

For example, **Sync now** on a Certificate Inventory page can run the existing source sync for its approved account. A later **Schedule maintenance** action can open Anima's confirmation for one approved device. Anima resolves the device and shows the start and end before applying the change.

**Views that run client code.** The first candidate is a sandboxed frame on a separate origin. Put it inside an extension page with an Anima header. The header identifies the package and keeps access to its settings. Load HTML and scripts from the approved package only. The frame has no access to the host DOM, session credentials, shared storage or Tauri IPC.

Block external loads, direct network requests, popups and downloads. The host handles navigation requests. The frame reaches the hub through the message interface only. [Browser frame isolation](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/iframe#sandbox), [Tauri capabilities](https://v2.tauri.app/reference/acl/capability/).

Use a versioned message interface for approved queries, action requests and navigation requests. Bind the interface to the loaded package version and view instance. The host validates the sender and message shape. It checks grants and scope for each request. The frame gets only the data needed for the view. Close the interface when the view closes, the package changes or the operator revokes access.

Before shipping, prove that the boundary works on each supported desktop and web platform. Check access to the host DOM, network, storage and IPC. Check that stale or invalid requests fail and broken views leave Anima usable. Use host-rendered views where the boundary cannot be enforced. Keep plugin-defined background execution out of this client contract. A hub runtime for third-party code needs a separate design under EXT-009.

## 4. Extensions

Each section gives: purpose, sources, data, interface, requirements and data class.

### 4.1 Map

**Purpose.** Show where the operator was, the places that they use, and where their sites and belongings are. **Class:** B.

**Sources.**

| Source | Method | Note |
| --- | --- | --- |
| OwnTracks or Overland (iPhone) | Collector-style push to an ingest token | Works before the Anima iOS client (Phase 5) |
| Google Maps Timeline export | Export upload | History before Anima |
| Strava GPS streams | From the Health sources | Workout routes |
| Photo locations | From Wardrobe, Library and a photo folder | Places for belongings |
| Anima iOS client | Push | Phase 5 |

**Data.** `location_fix` (time, latitude, longitude, accuracy, source, device), `visit` (place, arrival, departure), and the places of Section 2.7.

- **MAP-001.** The client must use MapLibre GL with vector tiles that the hub serves from a PMTiles file. The client must not request tiles from a third-party server. Each tile request shows an area to the tile server.
- **MAP-002.** The hub must decide where the map can draw a line. A trail must break at a gap of more than 20 minutes, a change of source or device, or a speed of more than 400 km/h.
- **MAP-003.** The map must show a gap between two trails as a dashed line. It must not show a gap as travel.
- **MAP-004.** The hub must find visits from clusters of fixes. The hub must suggest a place for a visit that has no place.
- **MAP-005.** Reverse geocoding must use a self-hosted geocoder (Photon or Nominatim). The hub must send only place centres to it. Do not send a raw fix.
- **MAP-006.** The map must have layers: trails, visits, places, sites and devices, belongings. The operator can set each layer on or off.
- **MAP-007.** The map must have a time range control. The default is the last 7 days.
- **MAP-008.** A place panel must show: visits, time spent, the belongings at the place and the reminders of the place.
- **MAP-009.** The map must keep to 3000 points for each trail on the screen. The hub must reduce the points for a large range.

### 4.2 Projects: deployments and analytics

**Purpose.** Show each project of the operator, where it is deployed, if each deployment is healthy, and how many people use it. For Vercel, Neon, Hostinger and PostHog, Projects is a view only (PRJ-003). For GitHub, Anima can also make changes (PRJ-017). Projects does not run recipes or agents. **Class:** C.

Projects is an extension as all others are (EXT-005). An operator who uses Anima only for devices does not set it on. Only the project design and map needs it (EXT-010). Secrets, the model gateway and AI coding sessions get more functions when Projects is on, and they operate without it.

All projects are on GitHub. They are hosted on Vercel, Neon or the Hostinger VPS. PostHog gives the analytics.

**Sources.** The hub reads the REST API of each service. These are the same APIs that the command line tools (`vercel`, `neonctl`, the Hostinger CLI) use. Anima does not run the command line tools.

| Service | Method | Gives |
| --- | --- | --- |
| GitHub | GitHub App of the operator, installed on the account of the operator (PRJ-017) | Repositories, default branch, latest commits, branches, open pull requests, issues, Actions run status, releases |
| Vercel | Access token | Projects with their linked GitHub repository, deployments (production and preview), build state, commit, domains, Web Analytics (visitors and page views) where it is on |
| Neon | API key | Projects, branches, databases, compute state (active or idle), storage and usage against the plan limits |
| Hostinger | API token (hPanel > Dev Tools > API) | VPS state, CPU, memory, disk and network metrics, Docker projects on the VPS |
| PostHog | Personal API key with read scopes | Pageviews, unique users, top events and top pages for each PostHog project |
| Anima | Devices and checks | Uptime of each production domain and of the VPS |

**Data.** `project` (slug, name, GitHub repositories), `deployment` (project, service, service ID, environment, URL, state, commit, created), `deployment_link` (project, service, service resource ID), `analytics_day` (project, source, day, pageviews, unique users).

- **PRJ-001.** The hub must import each GitHub repository that the connection can read, as one project for each repository. The operator can leave a repository out. The hub must not import a left-out repository again. A project can have more than one repository. (Changed by OD-W11 of the work plan: the operator accepted each project. operator, 2026-11-13)
- **PRJ-002.** The hub must link a Vercel project to an Anima project by its linked GitHub repository. The operator links Neon projects, Hostinger resources and PostHog projects by hand, one time.
- **PRJ-003.** The hub must use only read requests to Vercel, Neon and Hostinger. The HTTP client for each of these services must have an allow list of methods and paths. It must refuse every other request. The tokens of these services can also change resources, so this rule protects the accounts (VPS-005).
- **PRJ-004.** The project page must show each deployment with: the service, the environment, the state, the commit and its message, the time and the URL.
- **PRJ-005.** The project page must show the deployment history: the last 20 deployments for each environment, with failed builds marked.
- **PRJ-006.** For each production domain, the hub must suggest an HTTP(S) check (Section 6.4 of the spec). The project page shows the uptime and the TLS expiry of each domain.
- **PRJ-007.** For a Neon database, the page must show the compute state, the storage and the usage against the plan limit. A usage above 80 percent of a limit must show as Degraded.
- **PRJ-008.** The Hostinger VPS must link to its Anima device. The page shows the VPS metrics from the Hostinger API next to the Anima checks.
- **PRJ-009.** Analytics: only PostHog and Vercel Web Analytics give analytics. The hub must read them once each hour and keep one row for each project, source and day. The page shows pageviews and unique users for 7, 30 and 90 days, and the top pages. Verify which Vercel Web Analytics data the Vercel REST API gives.
- **PRJ-015.** A project with no PostHog project and no Vercel Web Analytics must not show an analytics section. It must not show zero values. Neon and Hostinger give status and usage only, not analytics.
- **PRJ-010.** Each item must have an "Open in" link to the dashboard of its service: GitHub, Vercel, the Neon console, hPanel or PostHog. For Vercel, Neon, Hostinger and PostHog, changes happen there, not in Anima.
- **PRJ-012.** The Projects page must show all projects as cards. Each card shows the production state, the last deployment time, the uptime and, if the project has analytics (PRJ-015), the users of the last 7 days. The Projects page has a **Sort by** dropdown with **Grouped by status**, **Last commit**, **Date created**, **Commit count**, and **Stars**. The default groups by status and orders projects by name within each group. Last commit and commit count use the default branch. Date created uses the repository creation date. The page can filter by GitHub’s primary language and repository size. The hub applies sorting and filters before paging, using the first repository in the current view. Unknown sort values follow known values.
- **PRJ-013.** A failed production deployment must make an event log entry and can trigger a reminder (Section 4.11).
- **PRJ-014.** The project page must also show the AI coding sessions (Section 4.12), the drafts (Section 4.3), the secrets (Section 4.4) and the model use (Section 4.15) of the project. It must show a section only when its extension is on (EXT-007).
- **PRJ-017.** Anima must connect to GitHub as a GitHub App that the operator makes in their own account (Section 9.2). The App has read and write permission on each repository permission that GitHub offers. This is an exception to SRC-010. Reason: the operator wants Anima to be their GitHub, not only a view of it. (operator, 2026-11-13)
- **PRJ-018.** The hub must keep the App ID, the installation ID and the private key of the App with the credentials domain (SRC-008, SEC-003). The hub signs a short token with the private key and gets an installation token from GitHub. An installation token is valid for one hour. The hub must keep it in memory only and must not write it to the database or to a log (LOG-007).
- **PRJ-019.** The App must send its webhook events to the webhook receiver (Section 2.12). Until Section 2.12 is built, the webhook of an App that Anima makes stays off (PRJ-039); the receiver then sets its own address and secret with `PATCH /app/hook/config` (PRJ-042), and the operator may need to tick "Active" once on GitHub. (operator, 2026-11-14) The hub must also read GitHub on a slow interval, because a webhook delivery can be lost (WHK-006). No OAuth redirect is necessary for an installation token, so SRC-020 does not apply to GitHub. (operator, 2026-11-13)
- **PRJ-027.** A full permission does not make a function. Each change that Anima can make on GitHub must be one named function with its hub API, its `anima` command and its MCP tool (CMD-030). The MCP tool of a change is off until the operator sets it on (MCP-002). OD-E14 gives the first functions.
- **PRJ-028.** The hub must record each change that it makes on GitHub in the audit log: the function, the repository, the target and the actor kind (LOG-012).
- **PRJ-029.** A change that cannot be undone must ask for a confirmation that names the target (CMD-024, MCP-007). Of the functions of OD-E14 these are: merge a pull request (the confirmation names `owner/name#n`, the head commit and the method), run a workflow again, run its failed jobs again, cancel a run (each names the run, the workflow and the commit, PRJ-034), and start a workflow by hand (names the workflow file and the branch). An issue, a comment, closing or opening an issue and a commit status under `anima/` can be undone or corrected and ask nothing. Later functions such as delete a branch, delete a repository, change a repository setting or force a push are in this class too. The API binds each of them to what the operator saw: the merge and the run controls carry the head commit, and the hub refuses the change when it moved (K-E1 Section 13.4). (operator, 2026-11-14; ruling number to be assigned)
- **PRJ-039.** A "Connect GitHub" flow that fills the form of the GitHub App for the operator, with the App manifest flow of GitHub. Anima sends the form with its values (a name that starts with `anima-`, 23 repository permissions, "Only on this account"): `metadata` read and the other 22 write, the set of the App the operator made by hand. The operator selects Create and then Install: three presses, nothing typed. The hub keeps the App ID, the installation ID and the private key; it does not keep the client secret or the webhook secret. The operator downloads no key file. The default label is the login of the owner of the App. The graphical client offers the personal account only; the API and the command line take an organisation. GitHub shows its "Confirm access" prompt when the hub sends the form; Anima cannot avoid it and the client says so. GitHub has no call to delete an App, so the hub cannot delete one that it made and does not use: it shows the link to the settings page of the App. (operator, 2026-11-14; ruling number to be assigned) Each operator still makes their own App (PRJ-017): one shared App would share its private key. (operator, 2026-11-13: "otherwise the user has to do many clicks")
- **PRJ-040.** The manual route stays: `anima source connect github --from-file <path\|->` with the key from a file, for the command line and for a machine with no browser. `--from-file` selects the manual route; without it the command starts the flow of PRJ-039. (operator, 2026-11-14; ruling number to be assigned)
- **PRJ-041.** An MCP tool can start the flow of PRJ-039 and a second tool can read its state. The tools are `anima_github_connect_start`, `anima_github_connect_state` (read) and `anima_github_connect_cancel`. The start tool returns the fixed link `https://<hub>/connect/github`. The link must hold no secret and no one-time value, and the hub must require a signed-in operator at that link: the page does nothing until the operator presses Continue in a browser signed in to Anima, or continues from the desktop client. No tool returns the one-time link, and the hub refuses `openGithubConnect` for a request that declares an agent. (operator, 2026-11-14) The operator, not the agent, selects Create and Install. The start tool is off until the operator sets it on (MCP-002), and each start is in the audit log. Confirm against Section 7 of the CLI and MCP plan before the build: the tool takes and returns no secret, and the person makes the trust decision.
- **PRJ-042.** Verified with the documentation of GitHub on 2026-11-14 (operator, 2026-11-14; ruling number to be assigned): the manifest is sent only as a form POST (field `manifest`) to `https://github.com/settings/apps/new` or the organisation form; `state` is optional and the hub uses it; the code that GitHub returns lives one hour and is exchanged with `POST /app-manifests/{code}/conversions` with no authentication; there is no `DELETE /app` (the only delete is `DELETE /app/installations/{id}`); `PATCH /app/hook/config` with an App JWT sets the webhook `url` and `secret`, and the documentation has no way to activate a hook by API; creating an App triggers GitHub's "Confirm access" prompt; "All repositories" was preselected in a real install. Still not confirmed, for the live check with the operator's account: (1) the webhook fields: whether `hook_attributes` with `active: false` is accepted with the hub's address, else whether omitting it is, else a placeholder address; (2) `redirect_url` and `setup_url` on a host reachable only through the VPN; (3) that GitHub lists all 23 permission names as read and write; (4) that the code is single use (a second exchange fails); (5) whether `state` is in the query of `setup_url`; (6) whether one press of Install is enough with "All repositories" preselected; (7) whether a form POST from another site asks for sign-in or only "Confirm access"; (8) that `/settings/apps/<slug>/advanced` shows "Delete GitHub App"; (9) whether `webhook_secret` is null when the hook is off (informational); (10) that exactly three presses are needed when access was confirmed recently. The first run with the operator's account, on 2026-11-14 against the hub at bc2f1cd7, showed (operator, 2026-11-14; ruling number to be assigned): (1) GitHub accepts the manifest as the hub sends it, with the webhook off; (2) both addresses work on a host reachable only through the VPN, because the browser makes the requests; (5) `state` is in the query of `setup_url`, since the hub served the page of the bound browser with a matching state; (7) GitHub asked for the authenticator code before it made the App; (10) the operator pressed Create, typed the authenticator code, selected "All repositories" and pressed Install, so the count is three presses and one code when access was not confirmed recently. The operator's report on (6) is not clear: "All repositories" was either preselected or one more press. The browser's last page was the waiting page and not the connected page, although the connection was stored; the fix makes that page wait for the end of the attempt. Not checked in that run: (3), (4), (8) and (9).
- **PRJ-016.** The subject key of a project is the name of its GitHub repository (RLK-004). Other extensions keep the repository name with their records. Thus their records show under the project when Projects is set on later, with no change to the stored records.

#### 4.2.2 Pages for GitHub work

The ideas come from the Projects pages of HQ. HQ also has parts that Anima must not copy; PRJ-038 lists them.

- **PRJ-030.** The project page has these tabs only: Overview (PRJ-004 to PRJ-009), Work (PRJ-031 to PRJ-035), Code (the files of a repository, read only), History (commits, branches and releases on GitHub), and Design and map (Section 4.2.1, only when that extension is on). A tab must not show before its function is built. (operator, 2026-11-14; ruling number to be assigned; replaces "three tabs only: Overview, Work, and Design and map")
- **PRJ-031.** The Work tab must show, in this order: a banner when the default branch fails its checks; the workflow runs that run now; the open pull requests; the open issues. Each list is one data path from the hub. Do not build a second path for the same list.
- **PRJ-032.** Each list must show when the hub last read GitHub, for example "Read 20 s ago", and must show "Stale" when the last read failed. Loading, error, not connected and empty must be four different states with different text. (operator, 2026-11-14; ruling number to be assigned: the hub reads GitHub live for these lists, K-E1 Section 13, so the label says "Read", not "Synced".)
- **PRJ-033.** The pull request list must show for each row if the pull request can merge (no conflict, checks passed). The operator can select rows and merge them in one action. The action must ask for a confirmation and must then show which merges succeeded and which failed.
- **PRJ-034.** A workflow run must have these controls: run again, run only the failed jobs again, cancel. Each confirmation must show the commit.
- **PRJ-035.** The operator must be able to start a workflow by hand, with a branch and the named inputs of that workflow.
- **PRJ-036.** The Projects page can show one list of items that need the operator across all projects: failed default branches, pull requests that wait for the operator, and issues assigned to the operator. The operator can mark an item "Done" or "Snooze 1 day". The item must come back when it has new activity. Deferred to the webhook slice: it needs tables, which come with the webhook receiver (Section 2.12). (operator, 2026-11-14; ruling number to be assigned)
- **PRJ-037.** A project card can show one small chart for each of these weekly counts: merged pull requests, issues opened and closed, failed workflow runs. Each chart has its own scale. Deferred to the webhook slice, as PRJ-036. (operator, 2026-11-14; ruling number to be assigned)

Rulings for the second Projects slice (Work, Code and History; the first changes of OD-E14). Each is a decided default of the slice design; the hub, command line and MCP plan follow them (K-E1 Section 13, CLI and MCP plan Sections 6 and 7):

- **Login-only changes.** Merge one or many, run again, run the failed jobs again, cancel a run, start a workflow and write a commit status are closed to API keys: login or browser token only, as `createRelease` and `cancelBuildJob` are. Each makes code run with the repository's secrets, stops it, or can satisfy a required check. A write key may make an issue, add a comment and close or open an issue. (operator, 2026-11-14; ruling number to be assigned)
- **Pull requests from forks (PRO-013).** A pull request from a fork merges one at a time, only after the operator types its head repository, never in bulk, and never by an AI agent: the hub refuses it for a request that declares an agent, and no MCP tool can merge one. A workflow run of a fork is not run again from Anima; it can be cancelled. (operator, 2026-11-14; ruling number to be assigned)
- **Private code.** The content of a file (`codeFile`) is closed to every API key, and no MCP tool returns file content: the file may hold a secret that the hub cannot see, and it is the strongest carrier of instructions to an agent. A read key can list the names in a directory. An agent that needs the code has the checkout. (operator, 2026-11-14; ruling number to be assigned)
- **Workflow inputs (PRJ-035).** The operator types the inputs as name and value pairs; GitHub checks them, and the client links to the workflow file. The hub parses no YAML: a parser would be a new dependency that reads untrusted text. (operator, 2026-11-14; ruling number to be assigned)
- **Commit status as an operator command.** `setCommitStatus` exists for the builds of Anima (PRO-010); CMD-030 gives it a command and a tool now. Its context is fenced to `anima/`, so Anima cannot write the status of another system. (operator, 2026-11-14; ruling number to be assigned)
- **PRJ-036 and PRJ-037** are deferred to the webhook slice. (operator, 2026-11-14; ruling number to be assigned)
- **Project groups (an idea of the operator, not planned work; no requirement ID yet).** The operator selects two or more projects on the Projects page, groups them and names the group. Pressing the group opens a page laid out like the Projects page that shows only the group's members. A project may be in more than one group, and a group may hold groups. (operator, 2026-11-14, relayed by another session: "yes and yes") Read from the operator's words, not ruled: a group works like a folder, so a grouped project leaves the main Projects page and an ungrouped one stays there. Notes for the builder: a group is Anima's own record and nothing is written to GitHub; membership is many-to-many; a group must not hold itself through a chain of groups; CMD-030 applies, so the hub function, an `anima project group` command and an MCP tool come before the client page.
- **Facts about GitHub's API** that these functions depend on are listed for verification in K-E1 Section 13 ("to verify"); none is asserted until checked against GitHub's documentation.
- **PRJ-038.** Do not build these parts of HQ: a list of projects in code (projects come from GitHub, PRJ-001); a tab or a panel with placeholder text; marketing text for a project; a second route to the same list; settings for density or pane width; a note that explains what a count does not mean. The team, the cost and the agent sections of HQ are out of scope, because Anima has one operator.

#### 4.2.1 Project design and map (later, XP-14)

This part comes after the other extensions. The operator keeps most of it by hand. The automatic part (Section 4.2) does not depend on it.

**Data.** `project_design` (project, design tokens, fonts, logo, screenshots), `project_map` (project, nodes, edges, notes, revision).

- **PRJ-020.** Each project can have a design section: its colour tokens, fonts, spacing scale, logo and screenshots.
- **PRJ-021.** The hub can import design tokens from a token file in the repository, for example a `design/tokens/` folder in the W3C Design Tokens format. The operator selects the file one time. Otherwise the operator enters the tokens by hand.
- **PRJ-022.** The design section must show each colour token as a swatch with its name and value, and each font as a sample.
- **PRJ-023.** Each project can have a project map: a diagram of its parts, for example the client, the API, the database and the hosting. The operator draws and edits the map in Anima.
- **PRJ-024.** A map node can link to a deployment, a Neon project, the Hostinger VPS or an Anima device. A linked node shows its live state colour. A node with no link has no state.
- **PRJ-025.** The map editor must use `@xyflow/svelte`. The hub must keep each saved revision of the map, as WRT-004.
- **PRJ-026.** The Projects page can show a map of all projects. It joins the project maps on their shared nodes, for example the Hostinger VPS.

### 4.3 Writing

**Purpose.** Write and keep notes, drafts and documents in Anima. **Class:** C. The operator can set a document to Class B.

**Sources.** Anima itself. An Obsidian vault folder through the collector (read-only index).

**Data.** `draft` (title, body in Markdown, tags, project, place), `draft_revision` (immutable).

- **WRT-001.** The editor must be CodeMirror 6 with Markdown. It must have a preview mode.
- **WRT-002.** The client must save a draft automatically 2 seconds after the last change.
- **WRT-003.** Each save must send the `expected_revision`. If the draft changed on a different client, the hub must refuse the save. The client must show both versions.
- **WRT-004.** The hub must keep every revision. The operator must be able to see and restore an old revision.
- **WRT-005.** The collector can index a selected Obsidian vault as read-only drafts. Anima must not write to the vault in this version.
- **WRT-006.** A draft can link to a project, a place, a device or a timeline entry with `[[` links.
- **WRT-007.** The operator must be able to export a draft as a Markdown file.

### 4.4 Secrets

**Purpose.** Keep global secrets and project secrets on the hub, and give them to a command, to a pipe or to the model gateway. **Class:** B (VLT-007).

This is not a password manager for websites. Use a dedicated password manager for that.

**Data.** `secret` (scope, project, environment, name, sealed value, rotation days, last rotation), `secret_access` (audit).

| Scope | Belongs to | Examples | Secret handle |
| --- | --- | --- | --- |
| Global | The hub | An OpenRouter key, an Anthropic key | `anima://secrets/global/<name>` |
| Project | One project and one environment | A database URL, a payment test key | `anima://secrets/<project>/<environment>/<name>` |

- **SCR-001.** The hub must seal each value with the data key of Secrets before it writes the value to the database (VLT-003, VLT-007). The database must not contain a plain text value. (Changed: the value was client-sealed. operator, 2026-11-13)
- **SCR-002.** The secret list must show names, scopes, environments and rotation status only. A value shows only after the operator selects "Reveal". The hub must record each reveal in the audit log (SCR-007). VLT-005 does not apply to a secret.
- **SCR-003.** Each secret can have a rotation period. The status values are: OK, Expiring (30 days or fewer), Due (14 days or fewer), Overdue, Unrecorded and Unset.
- **SCR-004.** The hub must send a reminder 30, 7 and 1 days before a rotation date (Section 4.11).
- **SCR-005.** The client must compare the secrets of a project with the `.env.example` file of its repository. It must show the missing names and the extra names.
- **SCR-006.** A command line tool (`anima run <project> --env <environment> -- <command>`) must get the secrets of the project from the hub and give them to the command as environment variables. Without a project (`anima run -- <command>`), it gives the global secrets. It must not write them to a file.
- **SCR-007.** The hub must record each read of a value in the audit log: scope, project, environment, names, time, computer and actor kind (LOG-012). A read is a reveal, a run, a piped read or a use by the model gateway. Do not record values (LOG-007).
- **SCR-008.** The import function must read a `.env` file that the operator selects. The client sends the values to the hub, and the hub seals them. Anima must not read a `.env` file without an operator action.
- **SCR-009.** Each secret must have exactly one scope: global or project. A global secret has no project and no environment. The scope does not change who can read the value. (operator, 2026-11-13)
- **SCR-010.** When a project has a secret with the same name as a global secret, the project secret must be used for that project. `anima run` and the model gateway (MGW-004) obey this rule.
- **SCR-011.** Each secret must have a secret handle of the form in the table above. A secret handle contains no value. The hub can show a secret handle in a list, in a log and in an MCP tool result.
- **SCR-012.** `anima run` must replace each secret handle in the environment of the command with its value. It must not replace a secret handle in an argument, because other processes can read the arguments.
- **SCR-013.** `anima secrets read <secret handle>` must write the value alone on standard output as one line (CMD-020), so that the operator can pipe it. When standard output is a terminal, the command must refuse and must name the `--reveal` flag. With `--reveal`, it writes the value to the terminal.
- **SCR-014.** When the operator changes a value, the next run, the next piped read and the next model gateway request must use the new value. No restart is necessary. Thus the operator rotates a key in one place.
- **SCR-015.** The MCP tools of Secrets must return names, scopes, rotation status and secret handles only. No tool takes a value and no tool returns one (MCP-004). Thus an AI agent can put a secret handle in a command, and it does not see the value.
- **SCR-016.** The project scope is available only when the Projects extension is on. When Projects is off, the client and the command line must show the global scope only, with no project field. If the operator sets Projects off, the hub keeps the project secrets (EXT-003) and a secret handle still opens them.

### 4.5 Integrations

**Purpose.** Show and control each source and each output in one place. **Class:** C. Credentials stay in the credentials domain. This section is part of the engine (ENG-002).

Integrations is a section of the Settings page (route `settings/integrations`). It is not a navigation item. The section has two tabs: Sources (data into Anima) and Outputs (spec Section 6.9: the Prometheus endpoint, API keys and notification channels).

- **ITG-008.** The Settings page must show Integrations as a section in its own sub-navigation. Each extension page must link to its sources in this section, for example "Manage sources" on the Health page.
- **ITG-001.** The Sources tab must show one row for each source in Section 9, grouped by extension.
- **ITG-002.** Each row must show the health state (SRC-009) and the last sync time, for example "Synced 4 min ago" or "Needs reconnect".
- **ITG-003.** The source detail must show: the accounts, the scopes, the last 20 sync runs with counts, the problems, and the buttons "Sync now", "Pause" and "Disconnect".
- **ITG-004.** "Disconnect" must ask if the operator wants to keep or delete the data of that account.
- **ITG-005.** For an upload source (for example a monthly bank statement), the row must show the date of the last file and an "Upload" button.
- **ITG-007.** When a source stays Stalled for 24 hours, the hub must write an event log entry and send a reminder (Section 4.11).

### 4.6 People

**Purpose.** A directory of the people that the operator knows. Each person shows once, with all their accounts and the history of contact with the operator across those accounts. This is not a social feed. **Class:** C for the directory, B for message text.

**Sources.**

| Source | Method | Gives |
| --- | --- | --- |
| macOS Contacts | Collector (needs the Contacts permission), or a vCard upload | Names, phone numbers, email addresses, birthdays, photos. This is the base of the directory. |
| iMessage | Collector (macOS) | Who the operator messages, and when. Message text (Class B). |
| Instagram | "Download your information" export (JSON), uploaded | Followers, following, direct messages (Class B). The Instagram API is only for business and creator accounts. |
| X | Account archive, uploaded | Followers, following, direct messages (Class B). The X API is paid, so do not use it. |
| GitHub | The GitHub App of Projects (PRJ-017) when Projects is on. Verify that an installation token can read these lists. | Followers, following, collaborators on the projects of the operator |

**Data.** `person` (name, photo, tags, birthday, notes), `person_account` (person, service, account ID, handle, URL, how found), `contact_event` (person, service, direction, time, kind; message text sealed), `merge_suggestion`.

- **PPL-001.** The People page must be a directory: a searchable list of persons. Each row shows the name, the photo, an icon for each linked account and the last contact time.
- **PPL-002.** The operator must be able to filter by service, by tag and by last contact, and sort by name or by last contact.
- **PPL-003.** A person page must show: all linked accounts with links to them, the contact history across all services on one timeline, the contact count for each month, the notes of the operator, and the linked records (belongings lent, places, drafts that mention the person).
- **PPL-004.** The hub must join accounts to a person only by a stable identifier: a phone number, an email address or a service account ID. A display name or a handle that looks the same must not join two accounts. It can only make a merge suggestion.
- **PPL-005.** The page must have a review list of merge suggestions. The operator accepts, rejects or edits each one. The hub must not ask again about a rejected pair.
- **PPL-006.** The operator must be able to link an account to a person by hand, and to split an account from a person.
- **PPL-007.** Anima must ingest only the accounts and messages that involve the operator. It must not ingest the feeds or the posts of other people.
- **PPL-008.** The own posts of the operator on Instagram and X go to the timeline (History) with the category "post". There is no feed page.
- **PPL-009.** A newer Instagram or X export must merge with the older data by external ID. It must not add copies.
- **PPL-010.** The Instagram import must refuse an HTML export and tell the operator to select the JSON format.
- **PPL-011.** The hub must send a reminder each 3 months to make new Instagram and X exports (Section 4.11).
- **PPL-012.** Photos and videos in an export must go to the file store (Section 2.6). The location in a photo can suggest a visit on the map.
- **PPL-013.** The operator can set a "keep in touch" interval for a person. The hub sends a reminder when the last contact is older than the interval. Birthdays from Contacts can also make reminders.
- **PPL-014.** A person can link to belongings (a book lent to a friend, LIB-005) and to places.

### 4.7 Money

**Purpose.** Show balances, transactions, investments, recurring costs and net worth across the accounts of the operator. Anima must never move money. **Class:** B.

**Sources.** These are the accounts of the operator. Use an API only where the institution gives a read-only API to a private person. Use a statement upload for the others.

| Institution | Account type | Method | Currency | Note |
| --- | --- | --- | --- | --- |
| Wise | Multi-currency | Read-only personal API token | Many | Balances and transactions |
| HSBC HK | Bank | Statement upload (PDF e-statement; CSV if online banking gives it) | HKD | No personal API. Verify the export formats. |
| Revolut | Bank | Statement upload (PDF or Excel from the app) | Many | No personal API |
| Bank of America | Bank | SimpleFIN Bridge, or CSV/QFX download | USD | Verify that SimpleFIN supports it |
| Chase | Credit card | SimpleFIN Bridge, or CSV/QFX download | USD | Verify that SimpleFIN supports it |
| Gemini | Credit card | Statement upload | USD | No card API found |
| Gemini | Crypto exchange | API key with the Auditor role only | Crypto, USD | The Auditor role is read-only and cannot have other roles |
| Coinbase | Crypto exchange | API key with the View permission only | Crypto, USD | |
| MetaMask | Crypto wallet | Public addresses with the Etherscan API (all EVM chains) | Crypto | Never ask for a seed phrase or a private key |
| Interactive Brokers | Investments | Flex Web Service: a token and Flex Query IDs | Many | Read-only reports: positions, trades, cash, dividends |
| Robinhood | Investments | SimpleFIN Bridge, or statement upload | USD | Verify that SimpleFIN supports it |
| Acorns | Investments | SimpleFIN Bridge, or statement upload | USD | Verify that SimpleFIN supports it |

**Data.** `money_account`, `money_transaction`, `balance_day`, `holding` (account, asset, quantity, cost, day), `asset` (stock, fund or token, with its symbol and chain), `price_day`, `recurring_cost`, `receipt`, `statement_file`, `fx_rate`.

- **MON-001.** Each source must use read-only access. A source that needs write access is not permitted.
- **MON-002.** A spend must have a negative amount. The hub must mark transfers between own accounts. It must not delete them.
- **MON-003.** The operator can change the category, the payee and the note of a transaction. The hub must keep the operator values in separate columns, so that a new sync does not overwrite them.
- **MON-004.** The hub must record a balance for each account each day. Some sources do not keep a balance history.
- **MON-005.** The hub must find recurring costs from the payee, the interval and the amount.
- **MON-006.** The hub must convert currencies with the daily reference rates of the European Central Bank. The operator selects the display currency.
- **MON-007.** The hub must match a receipt to a transaction in a window from 10 days before to 1 day after the receipt date.
- **MON-008.** The overview must show: net worth, cash, the spend of this month by category, subscriptions and other recurring costs, and the average monthly spend over 3, 6 or 12 months.
- **MON-011.** Anima is a personal application. Money must use sources that a private person can use without a company account. Do not add company accounting sources.
- **MON-012.** Each statement upload must have a parser for that institution and that file format. The parser must be a pure function with a fixture test made from a redacted real statement. If no parser matches, the client can use extraction (Section 2.9) with operator confirmation.
- **MON-013.** A statement upload must not make duplicate transactions when two statements overlap. Match on the date, the amount and the description.
- **MON-014.** The hub must send a monthly reminder to upload each statement source, for example "HSBC HK statement for October" (Section 4.11).
- **MON-015.** The hub must show investments and crypto as holdings with a value on each day. Prices come from the source when it gives them (IBKR, Coinbase, Gemini). Other prices come from one price source (OD-E10).
- **MON-016.** Net worth must add all accounts in the display currency. Credit card balances are negative.
- **MON-009.** A source failure must show on the screen as a stale value with its age. It must not stop the page.
- **MON-010.** Anima must not give investment or financial advice. It shows only data and simple totals.

### 4.8 History

**Purpose.** Show what happened, from all sources, on one timeline. **Class:** the highest class of the entries on the screen.

- **HIS-001.** The page must show a month, a week, a day and an hour scale. The default is the current week.
- **HIS-002.** The page must read the timeline (Section 2.3). It must not read domain tables directly.
- **HIS-003.** Each entry must show its source, its category and its basis. An inferred entry must look different from an observed entry.
- **HIS-004.** The page must show a coverage strip: the number of sources with data for each hour. An hour with no data must show as "Unknown". It must not show as idle.
- **HIS-005.** The operator must be able to filter by category and by source.
- **HIS-006.** An entry must open its source record, for example the workout, the AI session or the device event.
- **HIS-007.** A day view must show the map trail of that day next to the timeline.
- **HIS-008.** The operator can add an entry by hand. Its basis is "reported by the operator".

### 4.9 Health

**Purpose.** Keep health and fitness data from all devices and reports in one place. Anima does not diagnose or give medical advice. **Class:** B.

**Sources.**

| Source | Method | Note |
| --- | --- | --- |
| Strava | OAuth pull | Workouts and GPS. Rate limit: 100 requests per 15 minutes. |
| WHOOP | OAuth pull | Sleep, recovery, strain, workouts |
| Oura | OAuth pull | Sleep, readiness, activity, heart rate |
| Garmin Connect | Token pull through the Garmin bridge (Section 4.9.1). Fallback: FIT file and export upload. | Unofficial. The official Health API needs partner approval. |
| Apple Health | `export.zip` upload. Later the iOS client (Phase 5). | |
| InBody | CSV export upload. Photo of the result sheet. | Body composition |
| Medical reports | PDF and photo upload with extraction | Lab results |
| FHIR records | FHIR bundle upload | Where a provider gives an export |

**Data.** `health_record` (source, collection, source record ID, revision, time kind, values), `health_reading` (canonical: metric, value, unit, time, source), `lab_result` (test, value, unit, reference range, date, report file), `body_scan`.

- **HLT-001.** The hub must map each provider metric to a canonical metric with a canonical unit. The map must be one table in the code with tests.
- **HLT-002.** When two sources give the same metric, a ranking for each metric decides the canonical value. The operator can change the ranking.
- **HLT-003.** The hub must keep the provider records (SRC-011) and recompute canonical values for a day when a record of that day changes.
- **HLT-004.** A medical report must keep the original file. Each extracted lab result must link to its page in the file.
- **HLT-005.** The page must show: sleep, recovery and heart rate trends, workouts, body composition, and lab results with their reference ranges.
- **HLT-006.** A lab result outside its reference range must show the range and the value only. Anima must not say what the result means.
- **HLT-007.** Places can compare health metrics: days at a place against days at other places, for sleep, resting heart rate and steps.

#### 4.9.1 Garmin bridge

Garmin gives its official Health API only to approved business partners. Thus Anima uses the open-source library `python-garminconnect` (https://github.com/cyberjunky/python-garminconnect). From version 0.3.0, the library logs in with the Garmin mobile SSO flow and gets OAuth access and refresh tokens. It does not use `garth`. It needs Python 3.12 or later.

The hub is in Rust. Thus the library runs in a small separate container, the Garmin bridge.

```text
hub (sources/) --HTTP on the Docker network--> garmin-bridge (Python) --HTTPS--> Garmin Connect
     |  sealed tokens in, refreshed tokens out
     v
credentials domain
```

- **GAR-001.** The bridge must be a container on the Docker network of the hub. Do not publish its port (as GW-002).
- **GAR-002.** The bridge must be stateless. The hub keeps the Garmin tokens in the credentials domain (SRC-008). The hub sends the tokens with each request. The bridge returns the data and the refreshed tokens. Verify that the library can load and save tokens without a file; if it cannot, use a temporary folder that the bridge deletes after each request.
- **GAR-003.** The bridge must expose only read functions: activities and their original files, sleep, HRV, Body Battery, stress, heart rate, steps, body composition and weigh-ins. It must not expose a function that uploads, edits or deletes Garmin data.
- **GAR-004.** The operator types the Garmin email and password in the client one time. The hub sends them to the bridge for the login and then forgets them. The hub keeps only the tokens. Do not store the Garmin password.
- **GAR-005.** If Garmin asks for an MFA code, the bridge must return "MFA required" (the library mode `return_on_mfa`). The client asks the operator for the code. The bridge completes the login with `resume_login`.
- **GAR-006.** The poll interval for Garmin must be 60 minutes or more. The first backfill must go back in pages of 7 days, with a pause between pages.
- **GAR-007.** If Garmin refuses the tokens, the source must go to "Needs reconnect" (SRC-009). The bridge must not try the password login again by itself.
- **GAR-008.** The `garminconnect` version must be pinned in the bridge image. An upgrade is a separate change with a test against recorded responses.
- **GAR-009.** "Disconnect" must tell the operator to also revoke the Anima access in the Garmin account settings. The library cannot revoke a token at Garmin.
- **GAR-010.** The bridge must convert the Garmin response to provider records only. The canonical mapping stays in the Rust provider normaliser (SRC-006), with the other health sources.

Risks: the library is not official. Garmin can change its login flow or block the account access at any time. The FIT and export upload stays available as the fallback.

### 4.10 Documents (legal and identity documents)

**Purpose.** Keep copies of important documents and their expiry dates: passport, driving licence, visas, residence permits, insurance, vehicle and property documents. **Class:** A.

**Data.** `identity_document` (kind, holder, issuer, country, number, issue date, expiry date, client-sealed fields), files for the front, the back and the pages.

- **IDD-001.** The client must seal the document images and the fields before upload (VLT-001). The hub keeps only the kind, the expiry date and the ciphertext.
- **IDD-002.** The kind and the expiry date are not sealed, so that the hub can send expiry reminders. The operator must be told this.
- **IDD-003.** Field extraction must run on the client (VIS-002). The client must read the MRZ of a passport when it is visible.
- **IDD-004.** The list must show the kind, the holder and the expiry status. The number must show as the last 4 characters only, until the operator selects "Reveal".
- **IDD-005.** The hub must send expiry reminders. Defaults: passport and visa 9, 6 and 3 months; other documents 60, 30 and 7 days. The operator can change them.
- **IDD-006.** The client must not put a document image in the operating system clipboard or a temporary file without an operator action.
- **IDD-007.** The operator must be able to export a document as a PDF file for a form or an application.

### 4.11 Reminders

The rule evaluation and the delivery are part of the engine (ENG-001). The Reminders extension (XP-10) is the Reminders page: the list and the rule editor.

**Purpose.** Tell the operator something at the correct time, place or event, with a device notification only. **Class:** C. A reminder that refers to Class A or B data shows only a neutral text on the lock screen.

**Triggers.**

| Trigger | Example |
| --- | --- |
| Time | Each weekday at 08:00 |
| Place | Arrive at or leave "Gym" |
| Source event | When a source gives a matching record: "a Strava workout arrives", "a new AI session in project X", "device Y goes Down" |
| Source state | When a source connects, stalls or needs reconnect |
| Threshold | Sleep less than 6 hours; spend more than £500 in a week |
| Expiry | Document, secret rotation, TLS certificate, API token |

Reminders deliver only to the devices of the operator.

| Delivery | When |
| --- | --- |
| Client notification (Tauri notification plugin) | A desktop client is open |
| The ntfy channel of the notifications, to the ntfy app on iPhone and macOS (REM-008) | No client is open |
| Anima iOS push | Phase 5 |

- **REM-001.** The rule evaluation must be a pure function. It receives the rule, the new records, the place state and the current time. It returns a list of reminders to send (CQ-021).
- **REM-002.** The hub must evaluate source-event rules when a sync run stores new records, and time rules on a 1-minute sweep.
- **REM-003.** A rule can have an "unless" condition, for example "unless a workout is already recorded today".
- **REM-004.** Each rule must have a cooldown and a daily limit. The defaults are 60 minutes and 3.
- **REM-005.** The hub must check the rule again just before it sends. If the operator changed the rule, the hub must not send the old reminder.
- **REM-006.** Place rules must use the place state (inside or outside). A new fix inside the place radius is an arrival only if the last state was outside.
- **REM-007.** The operator can snooze or acknowledge a reminder from the notification.
- **REM-008.** A reminder must go to an open application and to the ntfy channel that the operator set in the notifications (NOT-002), on any ntfy server. Each topic name must be random. On a hosted hub Anima runs no ntfy server of its own (R-148). Until 2026-11-18 this requirement was: the hub runs ntfy on the Docker network and publishes it only on the VPN interface (NET-003).
- **REM-009.** The notification text on the lock screen must not contain Class A or Class B values.
- **REM-010.** Device status notifications (NOT-001) must use the same delivery code.

### 4.12 AI coding sessions

**Purpose.** Keep a record of the AI coding sessions of the operator, by repository. **Class:** B for the text. C for the counts.

**Sources.**

| Source | Method | Files |
| --- | --- | --- |
| Claude Code | Collector | `~/.claude/projects/*/*.jsonl` |
| Codex | Collector | `~/.codex/sessions/**/rollout-*.jsonl` |
| ChatGPT | Data export upload (`conversations.json` in the zip) | Web and app conversations. Not linked to a project unless the operator links it. |

**Data.** `assistant_session` (source, external ID, project, repository, branch, worktree, title, model, start, end, counts of prompts, tool calls and tokens), `assistant_turn` (role, kind, sealed text, truncated flag).

- **AIS-001.** The collector must read each session file from its last read position (COL-004). A session that changes must update the stored session. It must not add a copy.
- **AIS-002.** The collector must link a session to a repository by the git remote of its working folder. The repository name is the subject key (RLK-004). When Projects is on, the session shows under the project that has this repository (PRJ-016). It must map worktree paths (`.claude/worktrees/*`, `.codex/worktrees/*`) to the main repository.
- **AIS-003.** The collector must keep sub-agent sessions as child sessions of their parent session.
- **AIS-004.** The collector must redact the text (Section 2.5). It must not send tool output. It can send the tool name and the tool input, clipped to 2000 characters.
- **AIS-005.** The hub must keep a maximum of 64 000 characters for each turn and mark a clipped turn.
- **AIS-006.** The page must show: sessions by repository and by day (by project when Projects is on), the model, the duration, the counts, and a search in the prompts of the operator.
- **AIS-007.** Each session must be a timeline entry with the category "code".
- **AIS-008.** The operator must be able to exclude a repository or a folder. The collector must not read sessions from an excluded folder.

#### 4.12.0 Usage counts first (operator direction 2026-11-17)

Status: approved for build (R-135, operator, 2026-11-17: "Approve and build"). The requirements
AIS-018 to AIS-024 are approved. They are built and wait for HT-026 of
[need-human-testing.md](../need-human-testing.md). R-133 and R-134 decided their direction. The map is
[ai-direction.md](../proposals/ai-direction.md), step 1. The tasks are below.

**Purpose.** Show the use of each AI agent program on each machine before the vault and the
collector exist. Counts have no text, so they are Class C and need no vault.

- **AIS-018.** `anima-daemon` must read the session files of Claude Code and Codex (the table of
  sources above) on its machine. For each session it must send only: the source, the external ID,
  the model, the start, the end, the name of the repository (AIS-002; R-134), and the counts of
  prompts, tool calls and tokens. It must not send text, a title or the path of a folder.
- **AIS-019.** The counts must be an optional member of the push batch of K8, as the tool
  inventory is (CAP-029). The daemon makes no new call.
- **AIS-020.** The hub must keep the counts and give them by device, by source, by model and by
  day. The function must have its command and its MCP tool (CMD-030), then its page. The command
  is `anima agents usage` and the MCP tool is `anima_agents_usage`. The hub domain is
  `fleet/agent_usage/`.
- **AIS-021.** The hub must show an estimate of the cost: the tokens times the public price of
  the model (MGW-014 holds the prices). The text must say that it is an estimate and not the bill
  of the subscription.
- **AIS-022.** The daemon must not read a login, the name of an account or a usage limit
  (CAP-025). The function must not show the quota that is left on an account.
- **AIS-023.** The exclusions of AIS-008 apply to the counts. When the collector reads the same
  session (AIS-001), the hub must keep one session, not two.

- **AIS-024.** Only a daemon that runs as the user of the operator reads the session files: the
  launchd agent on macOS and the systemd user unit on Linux. The systemd system unit keeps its
  sandbox and sends no counts (HT-005). For such a device the page and the command must give the
  reason: "This daemon runs as a system unit and cannot see the sessions of a user."

The build order is CMD-030. Each task has its tests. The counts come from a device, so the MCP
result gives them in named fields with a length limit (MCP-008).

| Task | Content | State | Commits |
| --- | --- | --- | --- |
| AU-1 | The contract: the optional member `agentUsage` of the push batch (K8), the query in K1, the table (K2). The daemon: the pure parse of the session files of Claude Code and Codex in `daemon-core`, the reader, the exclusions (AIS-018, AIS-019, AIS-022 to AIS-024) | In progress: built in 7b066fb2, 36739f06, b605aad1. Waits for HT-026. | See the state |
| AU-2 | The hub: `fleet/agent_usage/` keeps the counts of each session and gives them by device, source, model, repository and day, with the estimate of the cost (AIS-020, AIS-021, AIS-023) | In progress: built in 934b0288, ee2a726c. Waits for HT-026. | See the state |
| AU-3 | `anima agents usage` and the MCP tool `anima_agents_usage` (AIS-020, CMD-030, CMD-031) | In progress: built in 2a001a34. Waits for HT-026. | See the state |
| AU-4 | The page in the client, with its fixture for the mock hub and its mockup (AIS-020, AIS-021, AIS-024; PLN-019) | In progress: built in 4b3823cd, 8bd2579e, 51f77f8f. The browser specs did not run and the mockup is not compared. Waits for HT-026. | See the state |
| AU-5 | The security review (the daemon reads files of the home folder), the entry in the human testing list for real machines | In progress: the review found six faults of the daemon, corrected in cbbc1122. The entry is HT-026, which is open. | See the state |

#### 4.12.1 Agent panes (Herdr-style, operator direction 2026-11-16)

**Withdrawn (R-196, operator, 2026-11-18, first-hand; OD-AXT2 of [agent-tools-as-extensions.md](../proposals/agent-tools-as-extensions.md)).** AIS-009 to AIS-017 are withdrawn. Anima.ing hosts Herdr as a terminal tool extension in their place. AIS-018 to AIS-024 (Section 4.12.0) stay. Review: if Herdr Cloud (a hosted relay of the maker of Herdr) ships, or the Herdr project stalls, the operator reviews this decision, and native panes can come back with a new ruling. The open decisions 1 to 3 below are superseded. The text below is kept as a record of the direction of 2026-11-16.

**Purpose.** Do everything [Herdr](https://herdr.dev) does for a herd of coding agents, inside this
extension and free: background terminal panes that stay alive, one view of every agent on every
device, and an alert when an agent needs the operator. The operator's words: "build hrdr into the
agent extension so it can be a tmux like thing ... so they are free but we just do everything they
do". Status: direction only. It has no work package yet and is not in the build order (Section 5).

Herdr (`github.com/herdrdev/herdr`) is Apache 2.0, so its code can be reused in this AGPL project
with its licence notice kept. Whether to embed it or write our own is open decision 1 below.

- **AIS-009.** Withdrawn (R-196): replaced by the Herdr tool extension of agent-tools-as-extensions.md. The daemon (`anima-daemon`) on each device must run a session server that keeps terminal panes alive
  when no client is attached, like tmux. A pane must survive a client disconnect and an agent
  restart.
- **AIS-010.** Withdrawn (R-196): replaced by the Herdr tool extension of agent-tools-as-extensions.md. Each pane must show a state: working, waiting for the operator, done or idle. For
  Claude Code and Codex the state must come from the session files the collector already reads
  (AIS-001). For other programs it must come from terminal activity.
- **AIS-011.** Withdrawn (R-196): replaced by the Herdr tool extension of agent-tools-as-extensions.md. One page must list every pane on every device, those waiting for the operator first,
  with the device, repository, branch and time in the current state. The operator must be able to
  open any pane in the browser terminal from this page.
- **AIS-012.** Withdrawn (R-196): replaced by the Herdr tool extension of agent-tools-as-extensions.md. The hub must send an alert through the notification system that sends check alerts (the NOT requirements in the design spec) when a pane
  changes to waiting or done. One alert per state change, and the operator can mute a pane or a
  device.
- **AIS-013.** Withdrawn (R-196): replaced by the Herdr tool extension of agent-tools-as-extensions.md. The operator must be able to start a new pane on a chosen device, in a chosen
  repository, from the page and from the CLI. When agent runs are approved
  (`docs/proposals/agent-runs.md`, AGN-001 to AGN-008), they must use these panes.
- **AIS-014.** Withdrawn (R-196): replaced by the Herdr tool extension of agent-tools-as-extensions.md. The CLI must offer `anima agents` (list panes and states, across devices) and
  `anima attach <device>/<pane>`.
- **AIS-015.** Withdrawn (R-196): replaced by the Herdr tool extension of agent-tools-as-extensions.md. The operator can turn on a daily digest of finished sessions that wait for review,
  so work can be checked in one sitting instead of watching panes live.
- **AIS-016.** Withdrawn (R-196): replaced by the Herdr tool extension of agent-tools-as-extensions.md. Agent panes must be free on every plan, including self-host. They must not be a
  paid feature of the hosted Solo plan.

- **AIS-017.** Withdrawn (R-196): replaced by the Herdr tool extension of agent-tools-as-extensions.md. The page must warn the operator when more agents need attention than one
  person can follow. It must count the panes that are working or waiting at the same time,
  across devices, and compare the count with a limit the operator sets. The default limit is
  16, the ceiling DHH reports. Over the limit, the page must suggest pausing or queueing
  panes, or switching to the daily digest (AIS-015). It must also say how long the operator
  has been switching between panes without a break. It must not block anything, and the
  operator can turn it off.

**Why.** The human, not the machines, is the limit: DHH reports a ceiling of about 16 agent threads
across 4 to 5 machines, and fewer as agents get faster (Lex Fridman Podcast #501, 1:38:09). The
answer is routing the operator's attention to the agent that needs it (AIS-010 to AIS-012, AIS-015),
from one seat, without a KVM box per machine.

**Open decisions.**

1. Superseded (R-196). Embed Herdr's core or write our own session server. Embedding is faster; our own keeps one
   daemon binary and one protocol.
2. Superseded (R-196). Whether the Herdr phone apps or terminal client can connect to Anima panes.
3. Superseded (R-196). How waiting-for-input is detected for agents that do not write session files.

### 4.13 Wardrobe

**Purpose.** Keep a record of each item of clothing, with a photo and its place. **Class:** C.

**Data.** `garment` (category, sub-category, colours, brand, size, material, season, condition, purchase date, price, place, spot, photos, notes), `outfit` (garments, date, photo).

- **WAR-001.** The operator must be able to add a garment from a photo. The client must cut out the garment from the background. On macOS, use the Vision subject lift.
- **WAR-002.** The extraction must propose the category, the colours and the brand (VIS-001). It must read a care label or a brand label when the photo shows one.
- **WAR-003.** Each garment must have a place (PLC-002), for example "Home > Bedroom > Wardrobe". A new garment uses the last place that the operator used.
- **WAR-004.** If the photo has a location, the client must suggest the nearest place that contains it.
- **WAR-005.** The operator must be able to move many garments to a different place in one action, for example when they pack for travel.
- **WAR-006.** The page must show a grid of cut-out photos with filters for category, colour, season and place.
- **WAR-007.** The operator can record an outfit for a day. The hub must calculate "last worn" and "times worn" for each garment.
- **WAR-008.** A garment can link to its purchase transaction in Money.

### 4.14 Library

**Purpose.** Keep a record of each book, with its place, its reading status and who has it. Also keep the films that the operator watched. **Class:** C.

**Sources.**

| Source | Method | Data |
| --- | --- | --- |
| Photos and barcodes | Client capture | Physical books |
| Open Library | Public API by ISBN | Book metadata and covers |
| Goodreads | Library export (CSV) for the first import. Then the public RSS feed of each shelf. | Shelves, ratings, read dates, reviews |
| Letterboxd | Account data export (zip of CSV files) for the first import. Then the public RSS feed of the diary. | Watched films, diary dates, ratings, reviews, watchlist |
| Kindle highlights | `My Clippings.txt` upload | Highlights |

Goodreads closed its API in 2020. Letterboxd gives its API only to approved applications. Thus Anima uses exports and the public RSS feeds. The RSS feeds need no key.

**Data.** `book` (ISBN, title, authors, edition, format, cover, place, spot, ownership, reading status, rating, lent to, notes), `highlight`, `film` (title, year, Letterboxd URI, watched dates, rating, review, rewatch, physical copy place).

- **LIB-001.** The operator must be able to add a book from a barcode scan, a cover photo or a search.
- **LIB-002.** For a barcode, the hub must read the metadata from Open Library by ISBN. It must send only the ISBN.
- **LIB-003.** The operator can take a photo of a shelf. The extraction must propose one book for each spine. The operator confirms each book (VIS-001).
- **LIB-004.** Each physical book must have a place, as WAR-003 and WAR-004. An e-book or an audiobook has no place.
- **LIB-005.** The ownership values are: owned, lent, borrowed and wanted. A lent book must link to a person (PPL-014) and a date.
- **LIB-006.** The reading status values are: not started, reading, finished and stopped. A finish date makes a timeline entry with the category "reading".
- **LIB-007.** An import from a reading service must match books by ISBN first and then by title and author. The operator confirms the matches that are not by ISBN.
- **LIB-008.** The page must show a grid of covers with filters for place, status, ownership and author.
- **LIB-009.** The page must have two tabs: Books and Films.
- **LIB-010.** The hub must poll the Goodreads and Letterboxd RSS feeds each 6 hours. An RSS entry that matches an existing book or film must update it. It must not add a copy.
- **LIB-011.** Each watched date of a film must make a timeline entry with the category "media". A film on physical media (DVD, Blu-ray) can have a place, as LIB-004.

### 4.15 Model gateway

**Purpose.** Give each operator computer one address for AI model requests. The keys stay on the hub. **Class:** C. The hub keeps counts only, not the text of a request.

The operator installs Anima on a computer (INS-001), and that computer can then use each model that the operator has, with no key on the computer. The idea comes from Tailscale Aperture and from OpenRouter. Anima builds this function in the hub. It does not use those products as the gateway.

**Upstreams.**

| Upstream | Method | Key |
| --- | --- | --- |
| Model provider (Anthropic, OpenAI) | The API of the provider | A global secret |
| OpenRouter | OpenAI-compatible API | A global secret. One key gives many models. |
| Local model server on an operator device (llama.cpp, Ollama, vLLM) | OpenAI-compatible API on the VPN | None |
| Subscription of the operator (for example a Claude plan in Claude Code) | Passthrough (MGW-008) | The authorization that the program sends |

A model that the operator tuned with Unsloth is a local model. A local model server gives it to the gateway. Unsloth is not an upstream.

**Data.** `upstream` (name, kind, base address, device, secret handle, passthrough flag, state), `model_use` (time, device, project, upstream, model, input tokens, output tokens, cost, duration, status), `model_budget` (device, amount, period).

- **MGW-001.** The hub must serve two routes for model requests: one with the form of the Anthropic Messages API and one with the form of the OpenAI API. The routes must be available on the VPN only (SEC-008).
- **MGW-002.** The hub must identify the caller by its VPN address and must find the device with that address. The hub must refuse a request from an address that is not a device of the operator. The caller sends no key of its own.
- **MGW-003.** The hub must remove the placeholder key that a program sends and must add the key of the upstream. The key is a global secret (SCR-009). The hub must record the use (SCR-007).
- **MGW-004.** When Projects is on, a request can name a project in the header `X-Anima-Project`. Then the hub must record the project with the use and must use the project secret with the same name when the project has one (SCR-010).
- **MGW-005.** The hub must not send an upstream key to a caller. The key must not be in a log, an error or a metric (LOG-007).
- **MGW-006.** The hub must send each part of a streamed answer to the caller when it arrives. It must not hold the answer until the end.
- **MGW-007.** A model name has the form `<upstream>/<model>`, for example `openrouter/<model>` or `anthropic/<model>`. The hub selects the upstream from the first part. The operator can give a short name to a model.
- **MGW-008.** When an upstream has the passthrough flag and the request has its own authorization, the hub must send that authorization without change and must not store it. The hub must record the use as for other requests.
- **MGW-009.** The operator must be able to add a local model server as an upstream: a device and a port that answer the OpenAI API. The hub must read the list of models from the server.
- **MGW-010.** A request for a local upstream must not go through the hub when the hub is on the VPS. It must go from the operator computer to the device with the model, on the VPN. The hub still gives the route and records the use (OD-E11).
- **MGW-011.** The operator can set a budget for a device: an amount in USD for each day or each month. When the device is at its budget, the hub must refuse the request with HTTP 429 and a message that names the budget.
- **MGW-012.** The hub must record each request in `model_use`. It must not keep the text of the request or of the answer. The AI coding sessions extension keeps session text (Section 4.12).
- **MGW-013.** The Prometheus endpoint (spec Section 6.9) must give the token counts and the cost, by device, upstream and model.
- **MGW-014.** The hub must calculate the cost from the token counts and a price for each model. When the upstream gives the cost in its answer (OpenRouter), the hub must use that value. A local upstream has a cost of zero. A passthrough request has no cost value.
- **MGW-015.** `anima gateway setup` must set the gateway address and a placeholder key in the configuration of Claude Code and Codex on this computer. It must show each change before it writes it. The desktop application must offer the same function after the installation.
- **MGW-016.** The function must have its commands and its MCP tools (CMD-030): list the upstreams and their state, show the use, show and set the budgets. No tool takes or returns a key (MCP-004).
- **MGW-017.** The page must show: each upstream with its state, the use by device, by model and by day, the cost for the month, and each budget with the amount that is used.
- **MGW-018.** The model gateway needs the Secrets extension (EXT-010), because each upstream key is a global secret. Thus a key has one store and the operator rotates it in one place (SCR-014).

### 4.16 Bots (proposed, operator direction 2026-11-16)

This section is a proposal and is in its own document: [bots.md](../proposals/bots.md) (BOT-001 to BOT-078; R-196 withdrew BOT-001 to BOT-018 and BOT-025 to BOT-028 and parked BOT-047 to BOT-057). It is not approved. Its proposed work package is XP-16, which is not in the tables of Section 5. Do not build from it (PLN-018).

### 4.17 Observability suite: status pages, error tracking, product analytics and short links (R-241)

**Direction.** R-241 (operator, first-hand, 2026-11-19, 16:24 to 16:36 PT) decides OD-OBS1 to OD-OBS14 (Section 7). R-242 (operator, first-hand, 2026-11-19, 16:50 to 17:08 PT) decides OD-OBS7 and OD-OBS15 to OD-OBS20; OD-OBS21 to OD-OBS25 are open. The requirements OBS-001 to OBS-066 of this section (with gaps between the groups) are not approved: that is task OB1. No task may start before OB1, and R-144 holds them until the acceptance run WP-53 (OD-OBS13). Row 4.23 of [priorities.md](../plan/priorities.md) is the parent (tier 3; OD-OBS5). The mockup is [mockup/observability-suite/index.html](../mockup/observability-suite/index.html).

**Purpose.** Give the operator, inside Anima.ing, the tools that a public post of the founder of Coolify (2026-11-19, https://x.com/heyandras/status/2123390206464209146) listed as one program: a public status page, error tracking, product analytics and short links. Uptime checks are not new: they stay CHK-001 to CHK-014 and NOT-001 to NOT-008 in the design specification. The four parts are four extensions, each off by default, and the operator selects which to run (OD-OBS1, OD-OBS2, OD-OBS4). They count toward the extensions of this plan (OD-OBS11): this plan now has 19 extensions.

**Interpretation of the Planner (not words of the operator).** For product analytics and error tracking the operator said: "Everything in this entire extension, and it should just be Post Hog. Post Hog is like industry standard, it's the best. It also has an open source version." and "the thought for this entire thing is that you probably have the space available on another machine to run it. Or you can spin up a VM to run it". The Planner reads this as follows. Product analytics and error tracking use the open-source PostHog, self-hosted, as a stack of containers. The hub selects a machine of the operator with space, or a VM that Anima.ing provisions ([cloud-machines.md](../proposals/cloud-machines.md), CVM, a draft), and installs the stack there. The hub and the daemon manage the stack; they do not hold it in their own program. The MCP layer and the hub send each call to the machine that runs the stack ("if this machine is not the right machine to run it, then the other machine is the right machine to run it"). [NEEDS CLARIFICATION: OD-OBS16. The operator also said "In theory, this is all embedded inside either the hub app, like the installed hub app, or everything else" and chose "inside the hub and daemon ... no separate binary" (OD-OBS4). The PostHog stack is ClickHouse, Kafka, Postgres, Redis and more, and needs about 4 vCPU and 16 GB of memory, so it cannot be inside the hub program.]

| Extension | Key | Work package | Data class | Needs | Works with |
| --- | --- | --- | --- | --- | --- |
| Status pages | `status_pages` | XP-17 | C (the operator publishes it) | — | Projects, Short links |
| Error tracking | `errors` | XP-18 | B | — | Projects, Product analytics |
| Product analytics | `analytics` | XP-19 | B | — | Projects, Error tracking, Short links |
| Short links | `short_links` | XP-20 | C | — | Product analytics, Projects |

#### 4.17.1 Common rules

- **OBS-001.** Each of the four is one entry of the extension registry, off by default (EXT-001). The registry has 19 entries; the code type `static REGISTRY: [Entry; 15]` in `crates/hub/src/engine/extensions/registry.rs` changes to 19 (task OB3; K-E7).
- **OBS-002.** The foundation must not read a table of these extensions (EXT-005). Status pages read check state and the day aggregates (DB-008, DB-009, CHK-011) through the read functions of the checks domain only.
- **OBS-003.** Migrations use 3600 to 3699 (status pages), 3700 to 3799 (error tracking), 3800 to 3899 (product analytics) and 3900 to 3999 (short links). [NEEDS CLARIFICATION: confirm in task OB2 that no open branch takes these ranges.]
- **OBS-004.** No new program of Anima.ing runs for these extensions (OD-OBS4). The hub and the daemon manage the containers of the PostHog stack and of the Dub stack on a machine with room; they do not hold them in their own program (R-242, OD-OBS16).
- **OBS-008.** The stacks obey the rules of the container work types (R-226 to R-238; [hosted-applications-stage-2.md](../proposals/hosted-applications-stage-2.md) Section 4): (a) only the applications program talks to Docker; the daemon gives it each item (R-226); (b) each stack is an entry of the tested catalogue of Anima.ing (OD-HOST-11, R-215) that names each image by a digest (R-233); (c) the final file must pass the profile `contained` (R-228) [NEEDS CLARIFICATION: OD-OBS21; the Compose files of PostHog and Dub as published do not pass]; (d) each port is on `127.0.0.1` (R-229), and the hub reaches it [NEEDS CLARIFICATION: OD-OBS22]; (e) nothing installs before the design of task H30 is approved and the types are version 0.4 of contract K10 (R-237); (f) an API key or an MCP tool may ask for the install of these entries, because they are entries of the tested catalogue, but not for an update from a file of its own (R-234 on `dev`).
- **OBS-005.** Each stored text and each forwarded event goes through the shared redaction crate first (LOG-007, RED-002).
- **OBS-006.** Setting an extension off keeps its data, and "Delete data" removes it (EXT-003). Setting `status_pages` or `short_links` off removes its routes from the public path at once.
- **OBS-007.** Status pages, error tracking, product analytics and short links are built together, in the same waves (OD-OBS3, Section 4.17.9).

#### 4.17.2 Status pages (XP-17)

- **OBS-010.** The operator can make a status page with a title, an optional short text, an address name (slug) and an ordered list of components. A component is one check or one check of the type Group (design specification Section 6.4), with a display name that the operator writes.
- **OBS-011.** For each component the page shows the current state (Operational, Degraded, Down, Maintenance, No data) and 90 daily bars from the day aggregates. A maintenance window (design specification Section 6.5) shows as Maintenance and does not count as down time. A time with no beat, or when the hub slept, shows as No data and never as up.
- **OBS-012.** The page must not show an address, a host name, a port, a device name, a hub name or a check target. It shows only the display names that the operator writes. The hub refuses a display name or incident text that looks like an IP address, a host name with a dot, or a URL, and names the field.
- **OBS-013.** The hub serves the page on the public path (Section 4.17.6, OD-OBS8): HTML, `status.json`, and an SVG badge for each component and for the page. The page loads nothing from another origin, sets no cookie and has no analytics script.
- **OBS-014.** The hub renders the page from a snapshot that the status domain writes at each state change and at least each minute. The public path reads only the snapshot, never the checks or the database.
- **OBS-015.** Incidents. The operator writes an incident with a title, the components it affects, a state (Investigating, Identified, Monitoring, Resolved) and dated updates. When a component goes Down, the client can offer a draft. The hub never publishes an incident that the operator did not write or confirm. The page keeps 90 days of incidents.
- **OBS-016.** Each change of a page, of an incident and of the public path is in the audit log (LOG-012 to LOG-020) with the actor kind.

#### 4.17.3 Error tracking (XP-18)

- **OBS-020.** Error tracking uses the error tracking of the PostHog stack of Section 4.17.4 (OD-OBS6). Anima.ing does not write a second error store. [NEEDS CLARIFICATION: task OB0 checks that the self-hosted PostHog has error tracking on its free features; PostHog says "All paid-plan features are Cloud-only".]
- **OBS-021.** Sources: (a) the PostHog SDKs in the operator's programs, which send to the public error intake of the hub (OD-OBS10, OBS-062); (b) the lines of a container log at level error or fatal, on the path of HOST-001; (c) a restart loop (CHK-014). The hub sends (b) and (c) to the stack as exception events, with the device and the program.
- **OBS-022.** The hub forwards an event to the stack over the private network only. The stack is never public.
- **OBS-023.** The hub sends a notification (NOT-003 channels) for a new issue and for a resolved issue that occurs again, at most one for each issue in each hour. It reads the issues from the API of the stack.
- **OBS-024.** The Errors page of the client lists the issues from the API of the stack, with a link to the device, the log lines (HOST-001, HOST-015) and the page of the issue in PostHog. [NEEDS CLARIFICATION: a list in Anima.ing, or only a link to the PostHog interface. Recommendation: a list, so that MCP can read it.]
- **OBS-025.** An error is not a check. It does not change the state of a check or the uptime.

#### 4.17.4 Product analytics and the PostHog stack (XP-19)

- **OBS-030.** Product analytics is the open-source PostHog, self-hosted (OD-OBS6). The code outside its `ee/` folder is under the MIT licence (the PostHog `LICENSE` file, read 2026-11-19). The stack is the Docker Compose deployment of PostHog ("hobby"); PostHog says that self-hosted deployments are "officially unsupported" and have no tagged releases.
- **OBS-031.** One stack serves error tracking and product analytics. The hub installs it when the operator sets the first of the two extensions on, and removes it, after a confirmation, when the second is set off and the operator asks to delete the data.
- **OBS-032.** Placement. The hub proposes a machine that has Docker and the free room that PostHog names (about 4 vCPU, 16 GB of memory and more than 30 GB of disk, PostHog self-host page, read 2026-11-19), from the metrics of each machine (MET). The operator confirms the machine. If no machine has room, the client offers a VM that Anima.ing provisions (CVM; [NEEDS CLARIFICATION: CVM-001 to CVM-024 are a draft]).
- **OBS-033.** The hub installs and updates the stack as an entry of the tested catalogue, through the container work types `application_apply` and `application_back` (OBS-008; hosted applications stage 2). The stack is a check group: one check for each container, and an HTTP check of its web service.
- **OBS-034.** The images of the PostHog and Dub stacks update automatically (R-242, OD-OBS20: "automatically."; against the recommendation). How the digest of R-233 moves is open [NEEDS CLARIFICATION: OD-OBS23]. The guardrails of OBS-050 to OBS-055 apply to each automatic update. [R-249: automatic updates come later, as a general rule of the catalogue after the copies of stage 3; until then these stacks update by the act of the operator.]
- **OBS-035.** The operator opens the PostHog interface in the client through the tunnel of the hub (as a web interface of the device), not from the public internet.
- **OBS-036.** The Projects extension reads analytics from this stack as from PostHog (PRJ-009), with its base address. Anima.ing moves its own product analytics from PostHog Cloud to this extension once the extension works, that is when the human checks of task OB18 are Passed; OD-E6 is reversed then, not before (R-242, OD-OBS15). `www.anima.ing` and `docs.anima.ing` keep Vercel Web Analytics; SITE-008 and OD-21 do not change (OD-OBS7).
- **OBS-037.** Events from the operator's websites reach the stack only through the public intake of the hub (R-242, OD-OBS18). The intake accepts analytics events as well as errors (OBS-062).
- **OBS-050.** Before each automatic update, the hub makes a copy of the volumes of the stack [NEEDS CLARIFICATION: where the copy goes before the copy of stage 3 (HOST-042) exists; R-238 says that the setting "The hub may delete volumes" does not protect data. Recommendation: a copy on the same machine, kept until the next update passes].
- **OBS-051.** After each automatic update, the hub runs the check group of the stack (OBS-033) [NEEDS CLARIFICATION: the time to wait and the checks that count; recommendation: all containers Up and the HTTP check Up within 10 minutes].
- **OBS-052.** When the check of OBS-051 fails, the hub goes back to the earlier digest with `application_back` and restores the copy of OBS-050 when the update changed the data (a migration of ClickHouse or Postgres) [NEEDS CLARIFICATION: a migration can be one-way; task OB0 tests a way back].
- **OBS-053.** Each automatic update writes one entry in the audit log: the stack, the machine, the earlier and the new digest, the result, and a way back if one ran (LOG-012 to LOG-020) [NEEDS CLARIFICATION: the actor kind of an update that no person asked for; OD-OBS25].
- **OBS-054.** A failed update and a way back each send one notification (NOT-003).
- **OBS-055.** The operator can stop automatic updates for one stack. The default is on (R-242).
- **OBS-038.** Routing. A call of the command line or of an MCP tool for errors or analytics goes to the hub, and the hub sends it to the machine that runs the stack. The caller does not need to know the machine.

#### 4.17.5 Short links (XP-20)

- **OBS-040.** The operator can make a short link: a code and a target address. The code is 4 to 64 characters of `a-z`, `0-9` and `-`. The target is an `https` or `http` address.
- **OBS-041.** The hub answers `GET /l/<code>` on the public path with a redirect (302) to the target. An unknown code gives 404 with no text.
- **OBS-042.** Only the operator makes a link (the client, the command line, or MCP with approval). The public path never makes or changes a link.
- **OBS-043.** When Product analytics is on, each redirect sends one event to the PostHog stack: the code, the time, the referrer domain, the country and the device class. The hub stores no IP address and no full user agent.
- **OBS-044.** A link can have an end date and can be paused. A paused or ended link gives 404.
- **OBS-045.** Short links run Dub, self-hosted, as a stack that the hub manages as it manages PostHog (R-242, OD-OBS17: "run Dub, managed by the hub like PostHog."): placement as OBS-032, the rules of OBS-008, automatic updates as OBS-034 and OBS-050 to OBS-055. Anima.ing writes no engine of its own. Dub holds the links; the public path forwards `GET /l/<code>` to the redirect of Dub, and the client, the command line and MCP make links through the API of Dub by the hub. OBS-040 to OBS-044 and OBS-046 are rules of the hub for these calls. Dub's code is AGPLv3 [NEEDS CLARIFICATION: OD-OBS24; Dub's settings name hosted services: MySQL (PlanetScale), Upstash Redis, Upstash QStash and Tinybird, from `apps/web/.env.example`, read 2026-11-19].
- **OBS-046.** The hub refuses a target that is the address of the hub, of a device or of a private network (NET-001), so that a link does not expose a private address.

#### 4.17.6 The public path of the hub (OD-OBS8, OD-OBS10)

R-241 changes SEC-008, NET-003, DNS-005, SITE-006 (design specification) and HMD-011 ([hosted-mode.md](../proposals/hosted-mode.md)) to permit one public path. These requirements are not approved, so the exception has no effect until OB1 approves them and task OB17, the security review, passes.

- **OBS-060.** The public path is off by default. It is on only while `status_pages`, `short_links`, or the intake of `errors` or `analytics` is on and the operator sets "Public path" on.
- **OBS-061.** It is a second listener of the hub on one public port with TLS, on its own name (not `hub.anima.ing`; DNS-005). It serves only these routes: `GET /s/<slug>` and its files (OBS-013), `GET /l/<code>` (OBS-041), and `POST /i/<project>/...` (OBS-062). Each other request gets 404 with no text.
- **OBS-062.** The intake accepts the capture and exception requests of the PostHog SDKs for each project that the operator adds. Each project has a key; a request with no valid key gets 401. A body is at most 64 KiB, and each key has at most 100 requests each second and each sender address at most 20 [NEEDS CLARIFICATION: the limits are recommendations; task OB0]. The intake redacts (OBS-005) and puts the event on a bounded queue that the forwarder sends to the stack.
- **OBS-063.** The public listener has no route to the API, the GraphQL, the tunnels, the sign-in, the sessions or the files. It holds no session and no credential. It reads only the snapshot of status pages, the table of links and the project keys.
- **OBS-064.** The hub writes an event for each refused request class (counts each minute, not each request) and shows the counts in the client.
- **OBS-065.** On a hosted hub, the public path goes through the relay and the hub has no other public port (R-242, OD-OBS19). When a free hosted hub sleeps (HM-R6), the relay answers 503 with "Status not available" and does not wake the hub.
- **OBS-066.** The public path must not ship before the security review of task OB17 passes with no open finding of severity high.

#### 4.17.7 Interfaces (CMD-030 order)

1. Hub GraphQL: status pages, incidents, the public path setting, the intake projects and keys, the links, the errors and analytics reads, and the placement of the stack.
2. Command line: `anima status-page`, `anima incident`, `anima errors`, `anima analytics`, `anima links`, `anima public-path`.
3. MCP: read tools for each; a change tool (publish an incident, make a link, set the public path) follows the MCP permissions (MCP-014 to MCP-025) and has its JSON-RPC test (MCP-017). The hub routes each call (OBS-038).
4. Client: the Status pages, Errors, Analytics and Links pages, and the placement dialog (the mockup).

#### 4.17.8 Reviews

**Scope.** All four parts are in (OD-OBS1, OD-OBS2). Effort: L or larger for the four together (an estimate, not verified). Out: a separate binary, session recording, feature flags, surveys, alerts by phone call, on-call schedules, the "fix with an agent" idea, and a public link maker.

**Design scores (0 to 10).** Clarity of the status page 8; honesty (No data, a sleeping hub) 8; privacy of the operator on public pages 7; effort to start PostHog (placement) 6, a 10 needs a one-click VM; same look as the product 8; control (nothing public without the operator) 8.

**Engineering.** Types: `StatusPage`, `Component { check_id, display_name: DisplayName }` (refuses an address at construction), `Incident`, `StatusSnapshot`; `PublicRoute` (a closed enum of the routes of OBS-061); `IntakeProject { key_hash }`; `ShortLink { code: LinkCode, target: PublicUrl }`; `StackPlacement { device_id, state }`. Contracts: K-E7 (19 entries), K3 (the new domains), the schema, and the text of the public path; K8 does not change because the SDK events arrive at the hub, not at the daemon. Failure modes: the stack machine is down (the intake queue holds 24 hours, then drops the oldest and counts); a flood (the limits of OBS-062); an open redirect (OBS-042, OBS-046); a leak of a private name (OBS-012); a sleeping hosted hub (OBS-065); an update of PostHog that breaks (OBS-034). Trust boundaries: a visitor and the public listener; an SDK and the intake (the key); the hub and the stack (the private network); the operator and the placement. Tests: the route enum refuses each other path; `DisplayName` and `PublicUrl` refusals; the rate limits; the snapshot has no address; the forwarder retries. Blast radius: SEC-008, NET-003, DNS-005, SITE-006, HMD-011, EXT-009, XP-004, XP-005, the registry size, K-E7.

#### 4.17.9 Tasks

One pull request for each task. All tasks are Not started; none starts before OB1 and WP-53 (R-144). Status pages, error tracking, product analytics and short links are built in the same waves (OD-OBS3).

| Task | Wave | Content | Needs | State | Commits |
| --- | --- | --- | --- | --- | --- |
| OB0 | 0 | A record in `docs/research/`: the PostHog Compose stack on a test machine (resources, a run with no public domain behind the hub, error tracking on the free features, the capture endpoints); the PostHog and Dub files in the profile `contained` (OD-OBS21); Dub with no hosted service (OD-OBS24); an update and a way back across a migration (OBS-052) | — | Not started | — |
| OB1 | 1 | The operator approves OBS-001 to OBS-066 and decides OD-OBS21 to OD-OBS25 | The operator | Not started | — |
| OB2 | 1 | Docs and contracts: K-E7 (19 entries), K3, the schema, the public path text | OB1 | Not started | — |
| OB3 | 2 | Hub: four registry entries, `REGISTRY: [Entry; 19]`, switches, migrations | OB2 | Not started | — |
| OB4 | 2 | Hub: the public listener, the closed route enum, TLS, limits, counts (security) | OB2 | Not started | — |
| OB5 | 3 | Hub: status pages, incidents, the snapshot, `DisplayName`, the public render and badges | OB3, OB4 | Not started | — |
| OB6 | 3 | Hub and daemon: the placement of the PostHog stack, its catalogue entry by digest and its install through the container work types (OBS-008), its check group | OB3; the container work types as version 0.4 of K10 and task H30 (R-237); H31 | Not started | — |
| OB7 | 3 | Hub: the intake projects, keys, the queue and the forwarder to the stack (security) | OB4, OB6 | Not started | — |
| OB8 | 3 | Hub: the error sources of OBS-021 (b), (c); notifications of new and recurring issues | OB6 | Not started | — |
| OB9 | 3 | Hub: analytics reads from the stack; Projects reads its base address (OBS-036) | OB6 | Not started | — |
| OB10 | 3 | Hub: the Dub stack (placement and catalogue entry as OB6), the forward of `GET /l/<code>` to Dub, the refusals of OBS-046, the click events | OB3, OB4, OB6 | Not started | — |
| OB11 | 4 | Command line: `status-page`, `incident`, `errors`, `analytics`, `links`, `public-path` | OB5, OB7 to OB10 | Not started | — |
| OB12 | 4 | MCP: the read tools and the change tools with permissions; routing of OBS-038 | OB11 | Not started | — |
| OB13 | 5 | Client: the four pages, the placement dialog, the public path setting; the mockup compared | OB11 | Not started | — |
| OB14 | 4 | The VM path for the stack (CVM), only after CVM is approved | OB6; CVM | Not started | — |
| OB15 | 1 | Docs: the code change list for `REGISTRY` and XP-004 checked against the code at OB3 | OB2 | Not started | — |
| OB16 | 5 | The privacy page of the docs lists the public path and the stack (UDOC-040 to UDOC-044) | OB13 | Not started | — |
| OB17 | 6 | The security review of the public path, the intake, the redirects and the stack exposure; entries in `need-human-testing.md`. The public path does not ship before it passes | OB4, OB5, OB7, OB10 | Not started | — |
| OB19 | 3 | Hub: automatic updates of the PostHog and Dub stacks with the guardrails OBS-050 to OBS-055 | OB6, OB10; OD-OBS23, OD-OBS25 | Not started | — |
| OB18 | 7 | Human checks: a real outage on the public page; a real error from an SDK in PostHog; a short link and its click; a stack on a second machine | OB17 | Not started | — |

| Task | Pass or fail | Check commands | Out of scope |
| --- | --- | --- | --- |
| OB0 | Pass when the record gives the measured resources, the run behind the hub, the error tracking result and the engine choice; fail otherwise | `bun run quality:documents` | Code |
| OB1, OB2, OB15 | Pass when the ruling or the documents exist and the gates pass; fail otherwise | `bun run quality:register`, `bun run quality:documents` | Code |
| OB3 | Pass when the four switches are off by default, the foundation tests pass with all off, and the registry test lists 19 entries; fail otherwise | `cargo nextest run -p anima-hub engine::extensions`, `bun run gates` | Pages |
| OB4 | Pass when each path outside OBS-061 gets 404, the limits hold under a load test, and the listener has no route to the API (test); fail otherwise | `cargo nextest run -p anima-hub public_path` | Routes of other tasks |
| OB5 | Pass when the snapshot has no address (test), the bars match the aggregates with maintenance, and No data shows for a gap; fail otherwise | `cargo nextest run -p anima-hub status_pages` | — |
| OB6 | Pass when the hub proposes a machine from metrics, installs the stack by the digest of its catalogue entry on a test machine, and its checks are Up; fail otherwise | `cargo nextest run -p anima-hub observability`, the human check of OB18 | The VM path |
| OB7 | Pass when a request with no key gets 401, a large body is refused, an event reaches the stack, and a fixture secret is redacted; fail otherwise | `cargo nextest run -p anima-hub intake` | SDKs other than PostHog |
| OB8, OB9 | Pass when fixture logs make issues in a test stack and one notification each hour, and Projects shows analytics from the stack; fail otherwise | `cargo nextest run -p anima-hub observability` | — |
| OB10 | Pass when a link of Dub redirects through the public path, a paused link gives 404, and a private target is refused; fail otherwise | `cargo nextest run -p anima-hub short_links` | Public link making |
| OB19 | Pass when a test update with a broken image goes back to the earlier digest, the copy is restored, and one audit entry and one notification exist; fail otherwise | `cargo nextest run -p anima-hub observability`, the human check of OB18 | Other catalogue entries |
| OB11, OB12 | Pass when the commands follow the output rules and the MCP tools pass their JSON-RPC tests; fail otherwise | `cargo nextest run -p anima-cli` | — |
| OB13 | Pass when the pages match the mockup at 1040 px, light and dark, and the mockup row names the commit; fail otherwise | `bun run --cwd apps/client test`, `bun run gates` | — |
| OB14 | Pass when a VM is made, the stack runs on it and is removed with it; fail otherwise | The human check | — |
| OB16 | Pass when the privacy page lists each connection; fail otherwise | `bun run --cwd apps/docs build` | — |
| OB17 | Pass when no finding of severity high is open and the `HT-nnn` entries exist; fail otherwise | `bun run quality:documents` | — |
| OB18 | Pass when the operator sets the entries to Passed; fail otherwise | — | — |

Rules for each task:

- Always: work in a worktree from `origin/dev` and merge to `dev`; claim the task (PLN-022); run `bun run quality:register` and `bun run quality:documents` before each commit; keep CMD-030 order; write "Not verified" for a step that you did not run.
- Ask first: a frozen contract change; a DNS record or a public port; a new outbound connection; a VM that costs money; a change of the guardrails of automatic updates (OBS-050 to OBS-055).
- Never: a public route other than OBS-061; the PostHog stack on the public internet; an address or a device name on a public page; a link made from the public path; a stored secret; a fact of the business vault.

Prompt for each task: "Follow AGENTS.md 'Start of a session'. Claim and build task OBn of docs/plans/extensions-plan.md Section 4.17. Read Sections 2.8 and 4.17 first. Build only what the row of OBn says. Run the check commands of the row, then bun run quality:documents and bun run gates."

#### 4.17.10 Not verified

- That the self-hosted PostHog gives error tracking with its free features, and how its capture endpoints behave behind the hub (OB0).
- That the PostHog stack runs with no public domain and no LetsEncrypt certificate (its install asks for a domain and an `A` record).
- The resource numbers of OBS-032: they are PostHog's text, not a measurement.
- The limits of OBS-062 and the size of the queue.
- That the relay of `anima-link` can carry a public route (OBS-065).
- That HOST-003 (stage 2) and CVM will exist when OB6 and OB14 start.
- That PostHog and Dub can run in the profile `contained`. The published PostHog file (`docker-compose.hobby.yml`, read 2026-11-19) uses `extends` with a file, bind mounts of relative paths, `ulimits`, ports 80 and 443, and image tags such as `master` and `latest`; each of these is refused by Section 4.5 of the stage 2 proposal or R-233 as written.
- That Dub runs with no hosted service.
- That a way back works after a migration of the PostHog data.
- The effort of the four parts.
- What the Coolify product will be.

### 4.18 Arcane, hosted as it is (R-245)

**Direction.** R-245 (operator, first-hand, 2026-11-19, 17:08 to 17:12 PT) decides Arcane Q1 to Q4; Q5 is open (OD-ARC1). The requirements ARC-001 to ARC-012 are not approved: that is task AR1. The research is the vault gap report "Arcane" (Arcane `getarcaneapp/arcane`, commit `b154ae6`, read 2026-11-19). Arcane is BSD 3-Clause, a Go server with a SvelteKit client and SQLite, on port 3552; it talks to one Docker API for each environment, and it reaches other machines through its own agents, which dial out to the manager over one gRPC stream (fallback WebSocket). This follows the principle of the operator: "the whole goal of everything is to be able to support other open source applications".

- **ARC-001.** Arcane is an optional extension, off by default (EXT-001). Anima.ing hosts it as it is: no fork, no private patch, the name and the version as the project writes them (AXT-028 to AXT-035). Stage 1 of [hosted-applications-plan.md](hosted-applications-plan.md) stays Anima.ing's own view of containers (R-245, Q1).
- **ARC-002.** Stage 2 of hosted applications (container control) is built as planned now (R-245, Q4). Arcane does not replace it; the two are two ways to manage containers, and the operator selects.
- **ARC-003.** The Arcane manager is an entry of the tested catalogue (OD-HOST-11, R-215), its image named by a digest (R-233), installed through the container work types and the applications program (R-226) on a machine that the operator confirms. Its port is on `127.0.0.1` (R-229). [NEEDS CLARIFICATION: its file mounts the Docker socket, which the profile `contained` refuses (R-228); OD-ARC2.] [R-247 decides OD-ARC2: an exception to the profile, as a proposal that waits for a security review; R-229 for the port stays.]
- **ARC-004.** The client opens the Arcane manager in a web pane of the agent tools (AXT-008 to AXT-011, AXT-015): a child web view through the tunnel of the hub, not an iframe. Only the operator opens it (AXT-012).
- **ARC-005.** Sign-in in the pane is through an Anima.ing OIDC provider (ARC-010). The pane does not ship before that provider ships (R-245, Q3). Arcane's own OIDC client is set to the provider; the local admin of Arcane is turned off after the first sign-in (`ARCANE_DEFAULT_ADMIN_DISABLED`) [NEEDS CLARIFICATION: not verified on a real install; task AR0].
- **ARC-006.** Other machines are reached through Arcane's own agents (R-245, Q2). An agent is a catalogue entry on each machine that the operator grants. The agent's connection to the manager goes over the network of Anima.ing (an SSH `direct-tcpip` channel of the hub, as AXT-009, or the Anima.ing network address of the manager), never over the public internet. The hub carries the bytes and keeps nothing (AXT-015).
- **ARC-007.** The agent token and the encryption key of Arcane are secrets of the operator: sealed by the hub, never in a log (LOG-007), never returned by MCP.
- **ARC-008.** Anima.ing never proxies the raw Docker API through the hub or the daemon. A raw Docker API is the administrator of the machine; it would reverse R-226, SF-003 and SF-018.
- **ARC-009.** The grant screen of the agent says in a plain sentence: "Arcane's agent controls Docker on this machine, and so can do all on this machine."
- **ARC-010.** Anima.ing gets an OIDC provider in the hub: discovery, authorization code with PKCE, ID tokens signed by a key of the hub, one client for each hosted tool, the operator's login as the only user source (design specification OD-2). [NEEDS CLARIFICATION: OD-ARC3, the owner of the provider.]
- **ARC-011.** Arcane actions are not in the `anima` command or in MCP (CMD-030 covers only Anima.ing's own functions). An MCP tool may read the state of the Arcane entry (installed, running), as AXT-019.
- **ARC-012.** No Arcane agent ships before the security review of task AR6 passes with no open finding of severity high.

| Task | Wave | Content | Needs | State | Commits |
| --- | --- | --- | --- | --- | --- |
| AR0 | 0 | A record in `docs/research/`: Arcane manager and one agent on test machines, the agent link through the hub, the OIDC setting, the socket mount against the profile `contained` | — | Not started | — |
| AR1 | 1 | The operator approves ARC-001 to ARC-012 and decides OD-ARC1 to OD-ARC3 | The operator | Not started | — |
| AR2 | 2 | The catalogue entries of the Arcane manager and agent by digest (ARC-003, ARC-006) | AR1; the container work types as version 0.4 of K10 and task H30 (R-237) | Not started | — |
| AR3 | 2 | The agent link over the network of Anima.ing (ARC-006), with the grant of ARC-009 | AR2 | Not started | — |
| AR4 | 3 | The web pane of the manager (ARC-004), after OI3 | AR2; AX5, AX8; OI3 | Not started | — |
| AR5 | 3 | The extension switch, the state read for MCP (ARC-011), the attribution | AR2 | Not started | — |
| AR6 | 4 | The security review: the agent with the socket of Docker (OD-ARC2), the token, the link, the pane; entries in `need-human-testing.md` | AR3, AR4 | Not started | — |
| AR7 | 5 | Human check: the operator signs in to Arcane with Anima.ing and manages a container on a second machine through an agent | AR6 | Not started | — |
| OI1 | 1 | The design of the OIDC provider (ARC-010): keys, clients, consent, logout, the hosted hub | AR1; OD-ARC3 | Not started | — |
| OI2 | 2 | Hub: the provider endpoints and the client records | OI1 | Not started | — |
| OI3 | 3 | The security review of the provider; client: the list of OIDC clients in Settings | OI2 | Not started | — |

Check commands: AR0, AR1 and OI1 are documents (`bun run quality:register`, `bun run quality:documents`); AR2 to AR5 and OI2 run `cargo nextest run -p anima-hub` and `bun run gates`; AR6 and OI3 pass with no open finding of severity high. Always: a worktree from `origin/dev`, a claim (PLN-022), CMD-030 order for Anima.ing's own functions. Ask first: a public port; a change of the profile `contained`. Never: a fork of Arcane; a raw Docker API through the hub; a secret in a log.

## 5. Build order: engine first, then one workflow per extension

The extensions start only when the current device manager build is finished: Phase 1 is accepted, `phase-1` is merged into `main`, and the macOS and Linux desktop applications are built.

- **PH-E-000.** No extension work (contracts, branches or work packages) must start before the device manager build is finished. The operator decides when it is finished.

```text
 Device manager build finished (operator decides)
              |
              v
 E0  Engine programme (one programme orchestrator, work packages as in the implementation plan)
              |  contracts K-E1..K-E8 frozen, vault security review passed,
              |  one launch brief for each extension written
              v
 +------+------+------+------+------+-- ... --+------+
 XP-01  XP-02  XP-03  XP-04  XP-05  ...       XP-14     independent workflows,
 Map    Proj.  Writ.  Secr.  People           Design    any order, any number at a time
              |
              v
 Seam tasks (small follow-ups that join two finished extensions)
```

### 5.1 The engine (E0)

The engine contains Sections 2.1 to 2.11 and Section 3.3: sources, collector, timeline, places, files, vault, redaction, extraction proposals, record links, the reminder engine, Settings > Integrations, the extension switches and the navigation groups. It also does the hub folder regroup (OD-E5).

The engine freezes these contracts. An extension workflow uses them and does not change them (CON-001).

| ID | Contract | File |
| --- | --- | --- |
| K-E1 | Source registry: the declaration of a source, the normaliser signature, the sync state slice | `contracts/extensions/sources.md` |
| K-E2 | Collector parser interface and the ingest protocol with ingest tokens | `contracts/extensions/collector.md` |
| K-E3 | Timeline adapter interface and the category list | `contracts/extensions/timeline.md` |
| K-E4 | Places, files, record links and the shared subject keys (RLK-004) | `contracts/extensions/shared-records.md` |
| K-E5 | Vault: data classes, sealing, client-sealed records, reveal | `contracts/extensions/vault.md` |
| K-E6 | Reminder trigger interface: how an extension gives an event, an expiry date or a threshold | `contracts/extensions/reminders.md` |
| K-E7 | Extension registry: switch, navigation entry, data class, migration range | `contracts/extensions/registry.md` |
| K-E8 | Migration ranges and owned paths for each extension (Section 5.2) | `contracts/extensions/ownership.md` |

- **ENG-010.** The engine must include one sample extension that uses each contract, with tests. Use the Writing extension for this, because it is small and has no outside source. Thus XP-03 is part of E0.
- **ENG-011.** At the end of E0, the security reviewer (Opus) must accept the vault (PH-E-002).
- **ENG-012.** At the end of E0, the programme orchestrator must write one launch brief for each extension in `docs/plans/extensions/XP-NN.md` (the folder was `docs/programme/extensions/` until 2026-11-17; no brief was written there). A brief contains: the goal, the plan section, the owned paths, the contracts that it uses, the sources and keys, the acceptance criteria of Section 5.3, the gates and the builder models.
- **ENG-013.** The operator starts each extension workflow from its launch brief. A workflow needs no other context.

The engine and the grouped items are work items in [work-items/](../work-items/README.md) (they were GitHub issues until 2026-11-16, R-119):

| Work item | Item |
| --- | --- |
| [E0-1](../work-items/engine/E0-1.md) | E0-1: Regroup hub folders |
| [E0-2](../work-items/engine/E0-2.md) | E0-2: Sources framework and Settings > Integrations |
| [E0-3](../work-items/engine/E0-3.md) | E0-3: Collector and redaction |
| [E0-4](../work-items/engine/E0-4.md) | E0-4: Timeline and record links |
| [E0-5](../work-items/engine/E0-5.md) | E0-5: Places, file store and photo field reading |
| [E0-6](../work-items/engine/E0-6.md) | E0-6: Vault and its security review |
| [E0-7](../work-items/engine/E0-7.md) | E0-7: Reminder engine and ntfy delivery |
| [E0-8](../work-items/engine/E0-8.md) | E0-8: Extension registry, nav groups and launch briefs |
| [SPEC](../work-items/extensions/SPEC.md) | SPEC: Approve the design spec changes (Section 1.2) |
| [OD-E1](../work-items/decisions/OD-E1.md) | OD-E1: Decide: where Class B data lives |
| [OD-E4](../work-items/decisions/OD-E4.md) | OD-E4: Decide: hosted vision model |
| [OD-E10](../work-items/decisions/OD-E10.md) | OD-E10: Decide: price source |
| [OD-E3](../work-items/decisions/OD-E3.md) | OD-E3: Check SimpleFIN coverage |
| [KEYS](../work-items/extensions/KEYS.md) | KEYS: Accounts, keys and setup checks (Section 9) |
| [SEAMS](../work-items/extensions/SEAMS.md) | SEAMS: Seam tasks (Section 5.4) |

#### Tasks of E0-4, timeline and record links (PLN-006)

The operator ruled on 2026-11-16: issue #7 governs E0-4. The rules of issue #7 are TML-008 to TML-012 and RLK-005 to RLK-006. The wire unit of an event instant is Unix microseconds. A record reference is `kind:id` with an opaque `id`. The clock policy (OD-E16) and the portable reference and deleted targets (OD-E17) stay open. The work that was built before that ruling stays. The branch `engine/e0-4-timeline-links` was merged into `main` on 2026-11-17. No task has a live check on a real hub; task T8 is that check.

| Task | Content | State | Commits |
| --- | --- | --- | --- |
| T1 | Contracts K-E3 (`contracts/extensions/timeline.md`) and the record-link and subject-key parts of K-E4 (`contracts/extensions/shared-records.md`), with the K1, K2, K3 and K-E8 rows. The places and files parts of K-E4 are a gap for E0-5. | Done | `f4de06f9` |
| T2 | Hub: the timeline domain with no table, the adapter interface, `superseded_by`, and the event log as its first adapter (TML-001 to TML-007) | Done | `f4de06f9` |
| T3 | Hub: the record-link domain with the table `links.record_link`, the label of a target that does not exist, and the shared subject keys (RLK-001, RLK-002, RLK-004) | Done | `f4de06f9` |
| T4 | Hub API: `Query.timeline`, `Query.recordLinks`, `Mutation.linkRecords`, `Mutation.unlinkRecords` | Done | `f4de06f9` |
| T5 | `anima timeline list` and `anima record-link list`, `add`, `remove` (CMD-030). The noun `link` is the device link, so the record links have the noun `record-link`. | Done | `611f68cf` |
| T6 | MCP tools `anima_timeline_list`, `anima_record_links_list`, `anima_record_link_add`, `anima_record_link_remove` (CMD-030) | Done | `611f68cf` |
| T7 | Client: the one shared component that shows the links of a record (RLK-003). Issue #7 makes it later work for the pages that use links (#18, #13, #22, #24, #16), not a part of E0-4. | Not started | |
| T8 | Live check on a real hub: the event log shows in `anima timeline list`, and a link is made and read | Not started | |
| T9 | The rules of issue #7: the instant unit (TML-008), the received time (TML-009), dates (TML-010), no exact order across clocks (TML-011), replays (TML-012), references and missing targets (RLK-005, RLK-006), in K-E3, K-E4, the hub and the tests | Done (2026-11-17). Result: TML-008 is met (`instants.rs` tests, `tml_008_one_span_written_in_two_time_zones_gives_the_same_entries_in_utc`). TML-009 is met (`tml_009_a_record_received_late_keeps_its_event_time_and_sorts_by_it`; the event log gives `receivedAt` null). TML-010 is met: a date is the subject key `date:<YYYY-MM-DD>`, kept as a date (`rlk_004_a_date_key_is_a_date_and_names_itself`); no adapter has a record with only a date yet. TML-011 was not written in K1, in the hub or in the MCP tool; the branch adds it to their descriptions. The event log is the only source and uses the clock of the hub, so no test can show two clocks. TML-012 is met (`tml_012_a_replayed_record_is_one_entry_and_another_source_of_its_event_is_a_second`). RLK-005 is met for the link domain: a reference is `kind:id` with an opaque `id` (`rlk_001_a_reference_is_a_kind_and_an_opaque_identifier`), and the same link again is one row (`rlk_001_the_same_link_again_is_one_row_and_a_new_label_replaces_the_old`); the event log reference `event:<uuid>` does not change. RLK-006 is met (`rlk_002_a_link_to_a_target_that_does_not_exist_keeps_its_label`). The parts of OD-E16 and OD-E17 stay open. | `766310f5`, `50e7fb1f`, `d19b6a50` |

#### Tasks of E0-5, E0-6 and E0-7 (PLN-006)

These rows give the order of the work of each item. The work item holds the scope and the acceptance criteria. Each item is built in the order contract, hub, `anima` command, MCP tool, client (CMD-030). A row that waits for the operator is "Not started" until the operator decides.

| Item | Task | Content | State | Commits |
| --- | --- | --- | --- | --- |
| [E0-5](../work-items/engine/E0-5.md) | T0 | The operator decides: the location provider, the rules to match an event with a sample (tolerance, age, uncertainty), and where bulk data is kept. | Done (R-145, 2026-11-18): samples from Dawarich, then OwnTracks, then the Anima phone apps, through one ingestion contract; 2 minutes for "at the time", 10 minutes for "nearest, inferred", then unknown, and "uncertain" with no accuracy or an accuracy worse than 200 m; sealed pieces in a folder of the hub. Class B is on the hosted hub (R-146). | |
| E0-5 | T1 | Contract: the places and files parts of K-E4 (`contracts/extensions/shared-records.md`), which E0-4 left open. | Done (2026-11-18): K-E4 Sections 8 to 13 (places, files, extraction proposals, location samples, the context of a transfer event, the API), the data keys of K-E5 Section 1 (`contracts/extensions/vault.md`), and the ranges 2040 to 2069 in K-E8. | `0ce1e09b` |
| E0-5 | T2 | Hub: places (PLC-001 to PLC-006), the file store (FIL-001 to FIL-005) and extraction proposals (VIS-001 to VIS-004). | In progress (2026-11-18): built, with the tests of the gates; the live check is T5 (HT-034). Built: the data keys of K-E5 Section 1 (in the code `engine/sealing_keys`); places with merge, split, the refused delete and the site rows; the file store with sealed pieces, upload, download without the location of a JPEG, archives; the proposals. Not built: no domain gives a `Holder` or a `Derived` yet; a piece or a staging copy that a failed upload left is not swept; a link to the ID of a merged place is not counted by a delete; the refusal of a hub extraction for Class A reads the extension that the caller names, because no extension declares its forms yet; an upload does not compare its key with the extension of the owner record. | `3bf91c9a`, `6c799d22`, `1aeda484`, `7d814a50` |
| E0-5 | T3 | Hub: location samples from the selected provider through the source framework of E0-2, and the context of a transfer event at the SSH and SFTP boundary. | In progress (2026-11-19): built, with the tests of the gates; the live check is HT-040. Built: the schema `locations` (migration 2060); one storing path, with each position sealed with the sealing key `locations`; the OwnTracks path `POST /locations/v1/owntracks` with its ingest tokens; `recordLocationSamples` for the Anima phone apps; `locationSamples` with its audit entry; the context of each transfer of the files tunnel, stored in the transaction of its event; `transferContext` with the evidence for the operator as a pure function (2 minutes, 10 minutes, 200 m; R-145). The Dawarich pull (R-261): the acquisition with a key of the account in the source framework (K-E1 Section 7b), the provider `provider-dawarich`, `connectDawarich`, and the intake of `locations`, which stores each point as one sample; the live check is HT-063. Open: OD-E26 (plain `http` to a private address). Notes of the build: in K1 `observedAt` is a `DateTime`, as K-E3 Section 2.1 says, not a count; `recordLocationSamples` takes each signed-in token, and its account is the ID of the operator; the side of the computer of the client is unknown in each context, because the hub does not know that computer as a device; a context is stored for each transfer event, also one that was refused, cancelled or failed. | `d159f940`, `aac02e8d`, `ca42194d` |
| E0-5 | T4 | The `anima` commands and the MCP tools of T2 and T3 (CMD-030). | In progress (2026-11-19): built for T2: `anima place`, `anima file` (with `archive`), `anima extraction` and their MCP tools (no tool carries the bytes of a file). Built for T3: `anima location list\|import\|context`, `anima location token add\|list\|revoke` and the MCP tools `anima_locations_list`, `anima_location_import` and `anima_location_context` (no tool makes or returns an ingest token). `anima source connect dawarich` connects a Dawarich account (R-261); no MCP tool takes its key. | `6c799d22`, `1aeda484`, `8b1f27be` |
| E0-5 | T5 | Live check: the cases of the acceptance criteria of the work item, with an entry in [need-human-testing.md](../need-human-testing.md). | Not started | |
| E0-5 | T6 | A quick way to deploy an OwnTracks server (R-145): the deployment files and the procedure. It changes no server of the operator (VPS-005). | Built (2026-11-18): `deploy/owntracks/` (a Recorder in HTTP mode behind nginx, with a copy of each post to the hub). Not started in Docker by an agent; its check is an entry of T5. | `965d77d6` |
| [E0-6](../work-items/engine/E0-6.md) | T0 | The operator decides: the store, how a server proves its identity, the life of a cache, how a change reaches a cache, and the rules for versions. | Done (R-147, 2026-11-18): the `credentials` domain of the hub; the daemon token of an admitted server; a daemon learns the versions at each report and deletes its cache after 7 days with no contact; one active version, and the old one stops at once. | |
| E0-6 | T1 | Contract: K-E5 (`contracts/extensions/vault.md`). | Done (2026-11-18): Sections 2 to 5, the operational keys with retrieval by a daemon, the client-sealed records of Class A, and the ranges 2070 to 2089 in K-E8. OD-E20 is new. | `41add3e3` |
| E0-6 | T2 | Hub: operational keys in the `credentials` domain, with versions, retrieval by an admitted server and revocation; the store of client-sealed records with the audit of a reveal (VLT-001 to VLT-006). The daemon: the cache of K-E5 Section 2.1. | In progress (2026-11-18): built, with the tests of the gates: the operational keys of K-E5 Section 2 (versions, rotation, revocation, expiry, consumers, the fetch by a daemon, the member `keys` of the push) and the cache of the daemon with `anima-daemon key read`. It waits for HT-035 (two machines, the Keychain of a Mac, 7 days). Not built: the store of client-sealed records and the audit of a reveal (K-E5 Section 3); the answer of `deleteDevice` does not name the keys that the server had (the hub writes them to the audit log and the event log; the answer needs a change of K1, OD-E23). | `4d8e1dfe`, `7d814a50` |
| E0-6 | T3 | The `anima` commands. An MCP tool must not take or return a secret ([cli-mcp-plan.md](cli-mcp-plan.md) Section 7). | Not started. It waits for OD-E23: K-E5 Section 2.2 names the commands `anima key`, and `anima key` is the noun of the API keys. Until then the nine root fields of the operational keys are exceptions of the coverage test (CMD-031), each with this reason. | |
| E0-6 | T4 | The security review of the vault (ENG-011, PH-E-001, PH-E-002). | Not started | |
| E0-6 | T5 | Client: the vault key of Class A in the key store of the operating system, a second client by QR code, and the recovery key (VLT-001, VLT-002). It is built with the first Class A extension (XP-09), which is its first user. | Not started | |
| [E0-7](../work-items/engine/E0-7.md) | T1 | Contract: K-E6 (`contracts/extensions/reminders.md`). Compare the delivery with the notification work of Phase 3 (WP-45, WP-50) first; do not build a second transport. | Done (2026-11-18): the trigger interface, the rules, the pure evaluation, the place state and the delivery through the outbox and the channels of `notifications` (WP-45, WP-50), with one new function `queue_reminder`; the range 2090 to 2099 in K-E8. | `b14a7fa9` |
| E0-7 | T2 | Hub: the pure evaluation of a rule (time, place, source event, source state, threshold, expiry), cooldowns and the check at send time (REM-001 to REM-010). | Built (2026-11-19), with the tests of the gates, in `engine/reminders`: `evaluate` for the six triggers with `unless`, the cooldown and the daily limit; the instants of a time rule in its zone (a skipped local time, a repeated one, a rule that was missed); the place state from the location samples (`place_changes`); the schema `reminders` (migration 2090) with the stored form of a trigger; `notice`, which evaluates the rules that wait for occurrences in the transaction of the caller and stores each firing one time; `claim_send`, the check at send time (REM-005). A firing that the cooldown or the limit stops is in the result with its reason (`Due.stopped`), which K-E6 Section 3 does not show in `Due`. `sweep`, which takes `now`, fires each time rule one time for its latest instant and reads the location samples that arrived since its cursor for the place states and the place rules. The sweep also reads the new events of the event log (a `Record` with the source of the event and its severity as the kind; the event of a firing gives none), the operational keys that end or need a rotation (`keys.expiry`, `keys.rotation`), and the `Triggers` of each extension that is on (the interface is built; no extension gives one yet). It reads the health of the source accounts too: a change of health since the last sweep is a `Record` with the source `sources` (the table `known` keeps the last state of each account). Built, with the tests of the gates; the live check is HT-043. Open: no domain calls `notice` at the moment of storage yet (no extension gives records to remind about), and `notice` does not read the switch of an extension. The branch `engine/e0-7-reminders` starts from `engine/e0-5-locations`. | `44f54958`, `3b00213e`, `c893ec38`, `378c9e25` |
| E0-7 | T3 | Delivery to an open application and to the ntfy channel of the notifications (REM-008, R-148). | In progress (2026-11-19): built, with the tests of the gates, in `notifications`: the kind `reminder` of the outbox and of the deliveries (migration 0853, K1 `NotificationKind.REMINDER`), `queue_reminder`, which queues one row for each channel of a rule or for each channel that is on by default, and sends the title and the body as the engine gives them (no template of a channel is read); an ntfy channel whose topic is shorter than 16 characters gets no row (REM-008). `queue_reminder` takes the vault and gives the count of the rows and of the short topics, which K-E6 Section 5 does not show. The sender of the engine (`send_waiting`): the check at send time, the words for a lock screen (REM-009: for Class A or B the title `Anima` and the body `You have a reminder.`), one outbox row for each channel, and one event; a firing that no channel takes is `suppressed` with the reason `topic_not_random` or `no_channel`. The task of each minute runs the sweep and then the sender (`spawn_minute`, started with the hub). It waits for a live check, which is T5. The K1 fields of K-E6 Section 6 apart from the subscription: the rules, the firings, `testReminderRule`, `snoozeReminder` and `acknowledgeReminder` (a trigger and `unless` are JSON in the stored form). Not built: `Subscription.reminderFirings` and the count of the open applications; the random topic that `anima notification` offers. | `6efa8cdb`, `265cf638`, `3e512f95` |
| E0-7 | T4 | The `anima` commands and the MCP tools (CMD-030). | In progress (2026-11-19): built: `anima reminder list\|show\|add\|set\|remove\|enable\|disable\|test`, `anima reminder firings`, `anima reminder snooze <firing> <minutes>`, `anima reminder ack <firing>`, and one MCP tool for each (`anima_reminders_list`, `anima_reminder_get`, `anima_reminder_add`, `anima_reminder_set`, `anima_reminder_remove`, `anima_reminder_set_enabled`, `anima_reminder_test`, `anima_reminder_firings`, `anima_reminder_snooze`, `anima_reminder_ack`). Not built: `anima reminder watch`, which waits for the subscription of T3. | |
| E0-7 | T5 | Live check, with an entry in [need-human-testing.md](../need-human-testing.md). | Not started | |

#### Tasks of E0-3, collector and redaction (PLN-006)

The work item [E0-3](../work-items/engine/E0-3.md) governs: no collection is built without a feature that uses it, and no new redaction package is necessary without a user of it. E0-3 is tier 3b (row 4.8). The operator decided T0 on 2026-11-18 (R-166): T1 and T2 are built now, and T3 and T4 stay later. A session of 2026-11-18 wrote code for T1 to T3 with no task row. That code is not on `main`. It is the commit `041d4920` on the branch `handoff/2026-11-18-collector-redaction`, on an old base (`cef2e917`), with the old name `anima-agent` (R-126). Its tests were run again at that commit on 2026-11-18: 8 tests of `collector` and 8 tests of `redaction` pass. The hub files that T1 moves did not change between that base and `main` of 2026-11-18.

| Task | Content | State | Commits |
| --- | --- | --- | --- |
| T0 | The operator decides: the form of the collector (OD-E2, with OD-C2 of [compute-plan.md](compute-plan.md)), if the shared redaction crate is built now (OD-E21), and the folder rule of COL-006 (OD-E22). Then change COL-001, COL-006 and K-E2 to agree with the decisions. | Done (R-166, 2026-11-18): the collector is a part of `anima-daemon`; the redaction crate is built now (T1, T2); the folder rule has three modes below the root of a source. COL-001 and COL-006 are changed. K-E2 has no contract file yet (`contracts/extensions/collector.md` does not exist), so no contract is changed: T4 writes it for the daemon. OD-C2 stays open in the compute plan, and its recommendation does not agree with R-166. | |
| T1 | The crate `crates/redaction`: move the text redaction of the hub (`crates/hub/src/fleet/events/redaction/names.rs` and `text.rs`) into it with no change of behaviour; the event redaction of the hub imports it (RED-001, LOG-007). | Done (2026-11-18): built in `f03429d1`, ported from `041d4920`. All 21 gates pass at the merge of `main` `7aa1461f` (6157 Rust tests). No claim was made. | `f03429d1` |
| T2 | Redaction: the provider key shapes of RED-002 in each text, the redaction of a text of many lines with private key blocks, and the fixture of secret shapes (RED-002 to RED-004). This changes what the event log of the hub redacts. | Done (2026-11-18): built in `f03429d1`, ported from `041d4920`. All 21 gates pass at the merge of `main` `7aa1461f` (6157 Rust tests). No claim was made. | `f03429d1` |
| T3 | The crate `crates/collector` with no I/O: the folder rule and the read position of a file (COL-004, COL-006, COL-008). | Not started. Later (R-166, tier 3b): it waits for the first feature that names its source. The saved code has the old name `anima-agent` and its rule does not yet take the root of the source as the limit of each list. | Saved, not on `main`: `041d4920` |
| T4 | The contract review of the additions to K8 (`contracts/daemon-push.md`) and of K-E2, before a program reads the text of a file. Then the reader, the queue and the ingestion (COL-002, COL-003, COL-005, COL-007), for the first feature that names its source. Write the rows of this work when that feature is named. | Not started. Later (R-166, tier 3b). | |

### 5.2 Extension work packages

Each extension owns only its paths. It adds one line to each registry of the engine (K-E1, K-E2, K-E7). The orchestrator of the workflow merges these registry lines.

| ID | Work item | Extension | Section | Hub domain | Client | Other owned paths | Migrations | Tags |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| XP-01 | [XP-01](../work-items/extensions/XP-01.md) | Map | 4.1 | `life/locations/` | `routes/map/`, `$lib/map/` | `providers/location/`, PMTiles and Photon in `deploy/` | 2100–2199 | — |
| XP-02 | [XP-02](../work-items/extensions/XP-02.md) | Projects | 4.2 | `work/projects/` | `routes/projects/`, `$lib/projects/` | `providers/{github,vercel,neon,hostinger,posthog}/` | 2200–2299 | security (tokens, PRJ-003; GitHub App key and write functions, PRJ-017 to PRJ-029) |
| XP-03 | [XP-03](../work-items/extensions/XP-03.md) | Writing | 4.3 | `work/writing/` | `routes/writing/`, `$lib/writing/` | `collector/obsidian/` | 2300–2399 | Part of E0 (ENG-010) |
| XP-04 | [XP-04](../work-items/extensions/XP-04.md) | Secrets | 4.4 | `vault/secrets/` | `routes/secrets/`, `$lib/secrets/` | `crates/anima-cli/` | 2400–2499 | security |
| XP-05 | [XP-05](../work-items/extensions/XP-05.md) | People | 4.6 | `life/people/` | `routes/people/`, `$lib/people/` | `collector/{contacts,imessage}/`, `providers/{instagram,x}/` | 2500–2599 | — |
| XP-06 | [XP-06](../work-items/extensions/XP-06.md) | Money | 4.7 | `life/money/` | `routes/money/`, `$lib/money/` | `providers/{wise,simplefin,gemini,coinbase,etherscan,ibkr}/`, statement parsers | 2600–2699 | security (financial data) |
| XP-07 | [XP-07](../work-items/extensions/XP-07.md) | History | 4.8 | `life/history/` (queries only) | `routes/history/`, `$lib/history/` | — | none | — |
| XP-08 | [XP-08](../work-items/extensions/XP-08.md) | Health | 4.9 | `life/health/` | `routes/health/`, `$lib/health/` | `providers/{strava,whoop,oura,garmin,apple_health,inbody}/`, `deploy/garmin-bridge/` | 2800–2899 | — |
| XP-09 | [XP-09](../work-items/extensions/XP-09.md) | Documents | 4.10 | `vault/identity_documents/` | `routes/documents/`, `$lib/documents/` | on-client extraction in `$lib/documents/` | 2900–2999 | security |
| XP-10 | [XP-10](../work-items/extensions/XP-10.md) | Reminders page | 4.11 | none (engine) | `routes/reminders/`, `$lib/reminders/` | — | none | — |
| XP-11 | [XP-11](../work-items/extensions/XP-11.md) | AI coding sessions | 4.12 | `work/assistant_sessions/` | `routes/ai-sessions/`, `$lib/assistant-sessions/` | `collector/{claude_code,codex}/`, `providers/chatgpt/` | 3100–3199 | security (redaction) |
| XP-12 | [XP-12](../work-items/extensions/XP-12.md) | Wardrobe | 4.13 | `life/belongings/wardrobe/` | `routes/wardrobe/`, `$lib/wardrobe/` | — | 3200–3299 | — |
| XP-13 | [XP-13](../work-items/extensions/XP-13.md) | Library | 4.14 | `life/belongings/library/` | `routes/library/`, `$lib/library/` | `providers/{open_library,goodreads,letterboxd,kindle}/` | 3300–3399 | — |
| XP-14 | [XP-14](../work-items/extensions/XP-14.md) | Project design and map | 4.2.1 | `work/projects/design/` | `$lib/projects/design/`, `$lib/projects/map/` | — | 3400–3499 | Needs XP-02 first |
| XP-15 | none | Model gateway | 4.15 | `fleet/model_gateway/` | `routes/model-gateway/`, `$lib/model-gateway/` | `crates/anima-cli/` | 3500–3599 | security (keys, MGW-005). Needs XP-04 first |
| XP-17 | none | Status pages | 4.17.2 | `fleet/status_pages/` | `routes/status-pages/`, `$lib/status-pages/` | the public path (`fleet/public_path/`, task OB4) | 3600–3699 | security (public path) |
| XP-18 | none | Error tracking | 4.17.3 | `fleet/errors/` | `routes/errors/`, `$lib/errors/` | the intake and the forwarder | 3700–3799 | security (public intake) |
| XP-19 | none | Product analytics | 4.17.4 | `fleet/analytics/` | `routes/analytics/`, `$lib/analytics/` | the PostHog stack placement, `providers/posthog/` (shared with XP-02) | 3800–3899 | — |
| XP-20 | none | Short links | 4.17.5 | `fleet/short_links/` | `routes/links/`, `$lib/links/` | — | 3900–3999 | security (redirects) |

- **XP-001.** An extension workflow must not change an engine path or a path of a different extension. If it needs an engine change, it stops and reports a contract change request to the operator.
- **XP-002.** Each workflow uses the task loop of the implementation plan (Section 3.1): plan, build, gate, review, fix. A task with a security tag needs the security reviewer (AGT-004).
- **XP-003.** Each workflow works in its own worktree on the branch `xp/NN-slug`. It merges into `main` only after the gates pass and the operator confirms.
- **XP-004.** Only XP-14 and XP-15 depend on a different extension (XP-02 and XP-04). All other workflows can run in any order and at the same time. XP-17 to XP-20 share the public path (task OB4) and XP-18 and XP-19 share the PostHog stack (task OB6) of Section 4.17; they run together (R-241, OD-OBS3).
- **XP-005.** Suggested order when the operator runs them one at a time: XP-11, XP-02, XP-17 to XP-20 (together; R-241, OD-OBS12), XP-01, XP-07, XP-10, XP-13, XP-12, XP-05, XP-08, XP-06, XP-04, XP-15, XP-09, XP-14. Low-risk developer data first; the vault-heavy extensions after the engine has some use.

### 5.3 Acceptance criteria for each extension

| ID | The extension is done when |
| --- | --- |
| XP-01 | An OwnTracks or Overland fix and a Google Timeline export show as trails and visits. A gap shows as a dashed line. No tile request leaves the network. |
| XP-02 | With all other extensions off, each GitHub project shows its Vercel, Neon and Hostinger deployments with their state. A test proves that the clients of Vercel, Neon and Hostinger refuse a write request. The hub reads GitHub with an installation token of the App, and a test proves that the private key and the token are not in the database as plain text or in a log. Each GitHub change of OD-E14 works from the command line and is in the audit log, with its outcome: a change that GitHub refused, or whose answer did not arrive, is recorded too (PRJ-028; operator, 2026-11-14; ruling number to be assigned). PostHog and Vercel analytics show where they exist. |
| XP-03 | The operator writes, saves and restores a draft. A save from a second client with an old revision is refused. The Obsidian vault shows read-only. |
| XP-04 | `anima run` gives secrets to a command and replaces a secret handle. `anima secrets read` writes a value to a pipe and refuses a terminal without `--reveal`. A project secret is used before a global secret with the same name. A test proves that the hub database has no plain text value. Each reveal, run and piped read is in the audit log. |
| XP-05 | Contacts, iMessage, Instagram and X accounts of one person show as one person. A display name alone never joins two accounts. |
| XP-06 | All 11 accounts show in net worth, with the API sources synced and each statement parser tested with a redacted statement. Overlapping statements make no duplicates. |
| XP-07 | One week shows entries from at least three extensions. Hours with no data show as "Unknown". |
| XP-08 | Strava, WHOOP, Oura and Garmin data show as canonical metrics with the ranking applied. A lab report PDF gives confirmed lab results. |
| XP-09 | A passport photo gives proposed fields from on-client extraction only. A test proves that the hub cannot read the image. Expiry reminders arrive. |
| XP-10 | The operator makes a time, a place and a source-event rule, and each one arrives as a device notification. |
| XP-11 | Claude Code and Codex sessions from two computers show under their repositories, with Projects off. A known secret in a fixture session is redacted before it leaves the collector. |
| XP-12 | A garment photo gives a cut-out and proposed fields. Moving 10 garments to a different place is one action. |
| XP-13 | A barcode, a shelf photo, a Goodreads CSV and the Letterboxd RSS feed add books and films without duplicates. |
| XP-14 | A project shows its design tokens from a repository file and a hand-drawn map with live state on linked nodes. |
| XP-15 | Claude Code on a computer with no key gets a streamed answer through the gateway from a provider, from OpenRouter and from a local model server. A test proves that no upstream key is in an answer or a log. A device at its budget gets HTTP 429. Each request shows in the use counts. |

### 5.4 Seam tasks

A seam joins two finished extensions, for example the AI sessions of a project on the Projects page. A seam is a small task after both extensions are merged. Neither extension waits for it.

| Seam | Joins |
| --- | --- |
| Project page panels | XP-02 with XP-03, XP-04 and XP-11 (PRJ-014) |
| Lent to a person | XP-13 with XP-05 (LIB-005, PPL-014) |
| Purchase of a garment | XP-12 with XP-06 (WAR-008) |
| Health by place | XP-08 with XP-01 (HLT-007) |
| Day trail in History | XP-07 with XP-01 (HIS-007) |
| Photo location as a visit | XP-05 with XP-01 (PPL-012) |

- **PH-E-001.** A security review (Opus) must accept the vault design before XP-04, XP-06 and XP-09 start.
- **PH-E-002.** The vault review must include a test that the hub cannot read a Class A value. Secrets are Class B (VLT-007), so this test applies to XP-09 and not to XP-04.
- **PH-E-003.** The Phase 1 rule applies: do not start the web and iOS work until Phase 2 is complete.

## 6. Repository changes

New hub domains, each with the shape of Section 13 of the spec:

```text
crates/
├── hub/src/
│   ├── fleet/            The device manager domains (devices, checks, maintenance, metrics, sessions, ...); XP: model_gateway/
│   ├── engine/           E0: sources/, timeline/, places/, files/, links/, reminders/, extensions/
│   ├── vault/            E0: data keys and sealing; XP: secrets/, identity_documents/
│   ├── life/             XP: locations/, people/, money/, history/, health/, belongings/
│   └── work/             XP: projects/, writing/, assistant_sessions/
├── providers/            Pure provider normalisers, one module for each source (XP-owned)
├── redaction/            The redaction function (Section 2.5, E0)
├── collector/            The collector logic with no I/O (E0) and its parsers (XP-owned modules); `anima-daemon` runs it (COL-001)
└── anima-cli/            The `anima run` and `anima secrets` commands (XP-04); the `anima gateway` commands (XP-15)
```

- **REPO-E-001.** Without groups, the hub `src/` folder has more than 12 domains (REPO-005). As the first task of E0, group the domains under the parent nouns above. This is OD-E5.
- **REPO-E-002.** Provider normalisers must not import a hub domain. The hub calls them.
- **REPO-E-003.** Migration number ranges for the new domains must be in contract K2.

## 7. Open decisions

| Number | Decision | Recommendation |
| --- | --- | --- |
| OD-E1 | Hub location for Class B data. | Decided: Class B records are on the hosted hub, sealed with one data key for each extension, different from the credential key (VLT-003). No home hub. (operator, 2026-11-18, R-146) |
| OD-E2 | Collector form: a separate program (`anima-collector`), or a part of `anima-daemon`? | Decided: a part of `anima-daemon`, not a separate program (COL-001). The daemon already reads the session files for the counts (R-135). The additions to K8 and K-E2 need a contract review before the daemon reads the text of a file. OD-C2 of [compute-plan.md](compute-plan.md) recommends two programs and is not changed by this decision. (operator, 2026-11-18, R-166) |
| OD-E3 | Aggregator for the US accounts (BofA, Chase, Robinhood, Acorns) | SimpleFIN Bridge. It is for private persons and costs about 15 USD each year (its pages give different prices). Check each institution on bridge.simplefin.org before payment. HSBC HK and Revolut use statement uploads. |
| OD-E4 | Hosted vision model for Class B and C extraction | Permit a hosted model for Wardrobe and Library photos, with the operator switch (VIS-003). Use only on-device extraction for Documents. |
| OD-E5 | Hub folder grouping (REPO-E-001) | Group by parent nouns as the first task of E0 (`fleet/`, `engine/`, `vault/`, `life/`, `work/`), in a separate change. |
| OD-E6 | Telemetry for projects | Decided: PostHog and Vercel Web Analytics for analytics, Anima checks for uptime, and the service APIs for deployment state. No custom telemetry endpoint. R-242 (OD-OBS15) reverses this when the PostHog extension of Section 4.17 works (OB18 Passed): Anima.ing's own product analytics then moves to it. Until then it stands. |
| OD-E7 | Garmin access | Use the Garmin bridge with `python-garminconnect` (Section 4.9.1). Keep FIT and export uploads as the fallback. Accept the risk that Garmin blocks unofficial access. |
| OD-E8 | Obsidian two-way sync | Read-only in this version (WRT-005). Decide after XP-03. |
| OD-E9 | People sources | Decided: macOS Contacts, iMessage, Instagram, X and GitHub. Bluesky is not needed. |
| OD-E10 | Price source for holdings that the source does not price | IBKR, Coinbase and Gemini give their own prices. For MetaMask tokens and the SimpleFIN holdings, select one free price API. |
| OD-E11 | Path for a request to a local upstream (MGW-010). A program such as Claude Code has one base address, so it cannot select the path itself. | The `anima` program on each computer listens on the loopback address. It sends a request for a local upstream directly to the device on the VPN, sends all other requests to the hub, and reports the use to the hub. Not verified: the added work and the failure cases. Alternative: all requests go through the hub, and local requests are slower. |
| OD-E12 | Secrets on the hub (VLT-007) | Decided: the hub seals and can open each secret. One class of secret with two scopes, global and project (SCR-009). Each read is in the audit log; a piped read refuses a terminal. No other limit. (operator, 2026-11-13) |
| OD-E13 | Is Projects a part of the foundation? | Decided: no. The foundation is the device manager and the engine. Projects is an extension, off by default. Only the project design and map needs it; Secrets, the model gateway and AI coding sessions work with it (EXT-005, EXT-006, EXT-010). Community extensions come after the 15 extensions, sources first (EXT-009). (operator, 2026-11-13) |
| OD-E14 | The first GitHub functions that make a change (PRJ-027) | Decided: make an issue, add a comment, close or open an issue, merge a pull request (also many in one action, PRJ-033), run a workflow again, run its failed jobs again, cancel a run, start a workflow by hand (PRJ-034, PRJ-035), write a commit status for a build of Anima (PRO-010, PRO-020 of the projects proposal). Add others one at a time when the operator needs them. (operator, 2026-11-13) |
| OD-E15 | Where the webhook receiver runs (Section 2.12) | Decided: on the VPS, as a small separate program that is not the hub (WHK-009). It works where the hub runs and needs no other service. This changes SITE-006 ("the website is the only public part of Anima"). To open the port on the VPS needs the operator's confirmation at that time (VPS-005). (operator, 2026-11-13) |
| OD-E16 | Clock synchronization between Anima machines (TML-011, issue #7): how skew is found and reported, the permitted skew, and what an unreliable clock does | Not decided. E0-4 builds no clock check. Decide before a timeline source that uses the clocks of more than one machine. |
| OD-E17 | The portable form of a record reference across machines and replicas, and what a link does when its target is deleted (RLK-005, RLK-006, issue #7) | Not decided. E0-4 uses `kind:id` with an opaque `id` and keeps a link and its label when the target is missing. |
| OD-E18 | PH-E-000 says that no extension work starts before the device manager build is finished. Is PH-E-000 released, changed or still in force? | Decided: released for the engine and Projects; in force for the other extensions until the Phase 3 acceptance run (WP-53). (operator, 2026-11-18, R-144) |
| OD-E19 | More ways to deliver a reminder or a notification: the desktop and iOS applications subscribe to the channel, and message channels as plug-ins, such as those of photon.codes, of the agent of Nous Research and of OpenClaw (operator direction, 2026-11-18, R-148). Which channels, in which order, and which code is copied under which licence? | Not decided. E0-7 builds the delivery to an open application and to ntfy (REM-008). Recommendation: record the channels and the licences of those projects first, then decide the channels one at a time. The Bots proposal ([bots.md](../proposals/bots.md)) names related work. |
| OD-E20 | A reveal of a Class A record asks for the TOTP code again (VLT-005). On a hosted hub the operator signs in with the Anima.ing account and has no TOTP on the hub (R-129). What replaces the check there? | Not decided. `recordReveal` is unavailable on a hosted hub (K-E5 Section 3). Recommendation: the check of the operating system on the client (Touch ID, or the password of the computer) before each reveal. It blocks only the first Class A extension (XP-09). |
| OD-E21 | Is the shared redaction crate built now, before a collector or an upload uses it (E0-3 T1 and T2)? | Decided: yes, the redaction part only (T1 and T2). The event log of the hub uses it at once (LOG-007, RED-002). The collector (T3, T4) stays later. (operator, 2026-11-18, R-166) |
| OD-E22 | The folder rule of COL-006: the operator selects each folder, or each source has one of three modes? | Decided: three modes for each source (all folders of the source, all but a list, only a list), and the default is all folders of the source. "All" means the folders below the one root of that source and never the computer. The operator still turns on each source. COL-006 has this text. (operator, 2026-11-18, R-166) |
| OD-E23 | The commands of the operational keys (E0-6 T3). K-E5 Section 2.2 names them `anima key list\|show\|add\|set\|rotate\|revoke\|remove\|grant\|ungrant`, but `anima key` is the noun of the API keys (`key list\|create\|revoke`). Which noun do the operational keys get? And does the answer of `deleteDevice` name the keys that the server had (K-E5 Section 2.1), which is a change of K1? | Not decided. Recommendation: `anima secret` for the operational keys, with the MCP tools `anima_secrets_list` and `anima_secret_show`, and K-E5 Section 2.2 changed to these names; `deleteDevice` keeps its answer, and the audit entry and the event of the removal name the keys (built). |
| OD-E24 | The Dawarich pull of E0-5 T3 (K-E4 Section 11). K-E4 says that it goes through the source framework with a base address and an API key for each account. The source framework (K-E1) has one acquisition, the GitHub App key, and one fixed API base for each source; and no contract gives the K1 root or the command that connects a Dawarich account. An intake also needs an extension that is on (EXT-002), and `locations` is a part of the engine. Three things to decide: (1) K-E1 gets a second acquisition, a stored API key sent as a bearer, and an API base for each account, which the hub checks with the deny list of the probes before each request (the operator's Dawarich is on a private address of the VPN, so the deny list must permit it); (2) K1 gets `connectDawarich(label, baseUrl, apiKey)` (login or browser token only), with `anima source connect dawarich`; (3) the intake of `locations` runs always, or with a switch. Also: K-E4 Section 13 gets the edge `locations -> access` (`client_addr` only), which the OwnTracks path uses for its audit entry (built). | Decided: (a). The hub pulls from Dawarich with an API key of the server; K-E1 gets the acquisition with a key of the account, and K1 gets `connectDawarich`. The intake is of the engine and the pause of the account is its switch. (operator, 2026-11-19, R-261) |
| OD-E26 | The Dawarich pull sends the key only over `https` with a verified name (K-E1 Section 7b). A Dawarich server in a home network often answers only plain `http` on a private address. Is plain `http` permitted? | Not decided. Recommendation: (b). The options: (a) `https` only, as built: the operator puts a certificate on the server, for example with the reverse proxy that he uses; (b) plain `http` is permitted only when each address of the host is a private or VPN address (10/8, 172.16/12, 192.168/16, 100.64/10, fc00::/7), because the VPN encrypts the path, and the connect command says that the key goes with no TLS; (c) plain `http` to each address. |
| OD-E25 | K-E4 Section 13 says that each change writes one audit entry and one event. For the OwnTracks path this is one audit entry and one event for each fix that the phone sends (built as the contract says; a fix that the hub already has writes none). In the move mode of the app a fix comes each few seconds, so the event log and the audit chain grow fast and other events are hard to see. | Decided: (b). A stored fix writes no event; one audit entry for each ingest token and hour holds the count of its fixes. (operator, 2026-11-19, R-262) |
| OD-OBS1 | Which parts of the observability suite are in scope? | Decided (R-241): "OD-OBS1: all of them, including analytics." Against the recommendation (status pages and error tracking). |
| OD-OBS2 | Are short links (Dub) out of scope? | Decided (R-241): "OD-OBS2: no, include short links." Against the recommendation. Section 4.17.5. |
| OD-OBS3 | In what order? | Decided (R-241): "OD-OBS3: build them all together." Against the recommendation (status pages first). |
| OD-OBS4 | Inside the hub and the daemon as extensions that are off by default, with no separate binary? | Decided (R-241): "OD-OBS4: yes, ship inside the hub and daemon as extensions that are off by default, no separate binary." See OD-OBS16. |
| OD-OBS5 | Which priority row? | Decided (R-241): "OD-OBS5: new row 4.23 at tier 3." |
| OD-OBS6 | Product analytics and error tracking: what engine? | Decided (R-241), in the operator's words in the chat (quoted in R-241): open-source PostHog that the operator can switch on, on a machine with room or a VM that Anima.ing makes. The Planner's interpretation is in Section 4.17. OD-E6 for Anima.ing's own analytics is not answered: OD-OBS15. |
| OD-OBS7 | Do SITE-008 and OD-21 change for a page-view collector? | Decided (R-242): "OD-OBS7: yes, no change to SITE-008 or OD-21." As recommended. |
| OD-OBS8 | How does a status page reach the public? | Decided (R-241): "OD-OBS8: the hub serves the status page." Against the recommendation (static export). Changes SEC-008, NET-003, DNS-005, SITE-006 and HMD-011 (Section 4.17.6). |
| OD-OBS9 | Is the first publish target a GitHub repository branch? | Withdrawn: OD-OBS8 makes the hub serve the page, so there is no publish target. |
| OD-OBS10 | Which error events does the tracker accept? | Decided (R-241): "OD-OBS10: also a public error intake on the hub." Against the recommendation (PostHog SDKs straight to the operator's PostHog, no intake on the hub). Combined with PostHog error tracking: Section 4.17.3, OBS-062 (keys for each project, limits, the security review OB17). |
| OD-OBS11 | Are the new extensions outside the count of EXT-009? | Decided (R-241): "OD-OBS11: no, they count toward the 15." Against the recommendation. The count is 19. |
| OD-OBS12 | Where in the order of XP-005? | Decided (R-241): "OD-OBS12: right after XP-02 (Projects)." |
| OD-OBS13 | Does R-144 hold these extensions until WP-53? | Decided (R-241): "OD-OBS13: yes, R-144 holds these extensions until WP-53." |
| OD-OBS14 | A separate proposal now, or merged into this plan? | Decided (R-241): "OD-OBS14: no, merge it into extensions-plan.md now." Against the recommendation. The proposal `docs/proposals/observability-suite.md` is deleted; its text is Section 4.17. |
| OD-OBS15 | Does Anima.ing's own product analytics move from PostHog Cloud to a self-hosted PostHog (a change to OD-E6)? | Decided (R-242): "OD-OBS15: yes, Anima moves its own analytics to the PostHog extension once it works; OD-E6 is reversed then." Against the recommendation. OBS-036. |
| OD-OBS16 | Do the hub and the daemon manage the stack, rather than hold it in their program? | Decided (R-242): "OD-OBS16: yes, the hub and daemon manage the PostHog containers on a machine with room; no new Anima binary." As recommended. OBS-004, OBS-008. |
| OD-OBS17 | Short links: an engine in the hub, or Dub run as a stack? | Decided (R-242): "OD-OBS17: run Dub, managed by the hub like PostHog." Against the first recommendation; the Planner changed its recommendation to Dub after the principle of the operator. OBS-045; OD-OBS24 is open. |
| OD-OBS18 | Does the public intake also accept analytics events from the operator's websites? | Decided (R-242): "OD-OBS18: yes, the public intake also accepts analytics events from websites." As recommended. OBS-037, OBS-062. |
| OD-OBS19 | On a hosted hub, does the public path go through the relay, with a 503 when a free hub sleeps? | Decided (R-242): "OD-OBS19: yes, the public path goes through the relay and answers 503 while a free hub sleeps." As recommended. OBS-065. |
| OD-OBS20 | Is the PostHog image pinned by digest and updated only when the operator selects Update? | Decided (R-242): "automatically." Against the recommendation. OBS-034, OBS-050 to OBS-055; OD-OBS23 and OD-OBS25 are open. |
| OD-OBS21 | The Compose files of PostHog and Dub do not pass the profile `contained` (R-228): `extends`, relative bind mounts, `ulimits`, ports below 1024. How do they run? | Decided (R-248): "A: test first, with a fallback". Task OB0 tests a form that passes `contained`; if it does not run, the operator starts the stack at the machine and adopts it, or the first version reads a PostHog that he has. No exception to the profile for these two stacks. Before this ruling the cell said: Recommendation: Anima.ing writes a catalogue entry for each in the `contained` form (one file, files by `configs` with `content`, ports above 1024 on `127.0.0.1`, digests), and OB0 tests it. Strength: the hub manages it with no change of R-228. Weakness: Anima.ing keeps two large files up to date; if a key such as `ulimits` is needed, the entry waits for a ruling or the owner starts it at the machine and adopts it (R-211, R-228). |
| OD-OBS22 | With ports on `127.0.0.1` only (R-229), how does the hub reach the stacks (forward events, read the API, forward `/l/<code>`)? | Decided (R-250): "A: design a way through the link". SSH `direct-tcpip` for a hub on the VPN; for a hosted hub the link carries the port when the person turns it on at the machine. The design is task H35 of [hosted-applications-plan.md](hosted-applications-plan.md), with the owner of HMD-049. Before this ruling the cell said: Recommendation: an SSH `direct-tcpip` channel of the hub to the port, as AXT-009. Strength: no port on the network; "Open to the network" stays off. Weakness: each forward goes through the SSH link; the speed of the intake is not measured. |
| OD-OBS23 | Automatic updates and R-233 (digests in the catalogue): what moves? | Decided (R-249): "A: later, as a general rule". Updates to digests that Anima tested, for each entry of the catalogue, opt-in, with a setting at the machine, after the copies of stage 3 exist. Not now and not for two entries only. OBS-034 and task OB19 wait for it. Before this ruling the cell said: Recommendation: Anima.ing moves the digest of the catalogue entry after a test install, and each machine applies the new digest automatically. No machine follows a tag. Strength: R-233 holds; a broken upstream build stops at the test. Weakness: an update reaches machines only after Anima.ing tests it. |
| OD-OBS24 | Dub names hosted services (PlanetScale, Upstash Redis, QStash, Tinybird). Does Anima.ing run Dub only with local replacements? | Open. Recommendation: yes, only local replacements on the same machine, no account at a third party; OB0 proves it, and if it fails the question comes back. Strength: no data leaves the operator's machines. Weakness: Dub may not support it; not verified. |
| OD-OBS25 | An automatic update is a request that no person made. HOST-021 and ETH-004 say that Anima updates nothing by itself, and Section 4.14 of the stage 2 proposal names the actors. May the hub schedule update these two catalogue entries? | Decided in part (R-249): the actor of an update that no person asked for goes into the text of the contract that a security review reads, when the general rule of R-249 is written. Not now. Before this ruling the cell said: Recommendation: yes, as a new actor "the hub schedule" for `application_apply` of the PostHog and Dub entries only, with OBS-050 to OBS-055. Strength: R-242 needs it. Weakness: one more exception to ETH-004. |
| OD-ARC1 | Does the Arcane extension count toward the extensions of EXT-009 (as OD-OBS11)? (yes/no) | Open (skipped on 2026-11-19). Recommendation: yes, the count becomes 20. Strength: one rule for each new extension. Weakness: community extensions wait longer. |
| OD-ARC2 | The Arcane manager and agent mount the socket of Docker. R-226 says that only the applications program talks to Docker, and R-228 has no profile `full`. How do they run? (one choice: the owner starts them at the machine and adopts them (R-211, R-228); a profile for Arcane only, after a ruling; no agent) | Decided (R-247): "B: catalogue entry with an exception", against the recommendation. Arcane is an entry of the tested catalogue with an exception to the profile `contained` for the socket of Docker. The exact exception is a proposal (Section 10 of [hosted-applications-stage-2.md](../proposals/hosted-applications-stage-2.md)) and goes through a security review; nothing that installs Arcane is merged before the operator has decided its findings. The contract is not changed yet. Before this ruling the cell said: Recommendation: the owner starts them at the machine and adopts them, and the grant says what the agent can do (ARC-009). Strength: no change of R-226 or R-228; the owner of each machine decides at the machine. Weakness: no install by one click; an adopted stack updates as R-232 says. |
| OD-ARC3 | Which document owns the OIDC provider (ARC-010)? (one choice: the design specification, sign-in of the hub (OD-2); a new proposal; this section) | Open. Recommendation: the design specification, next to OD-2, with the tasks OI1 to OI3 of this section until it moves. Strength: the sign-in of the hub has one owner. Weakness: one move later. |

## 8. Ideas taken from Auracle and HQ

Auracle and HQ serve many users and many sources. Anima serves one operator. This table lists only the ideas that Anima uses.

| From | Idea | Where in this plan |
| --- | --- | --- |
| Auracle | Each connector owns only its part of the sync state | SRC-004 |
| Auracle | Narrow ingest tokens for device push | COL-003 |
| Auracle | Keep duplicates with `superseded_by`; never drop them | TML-005 |
| Auracle | Each Evidence entry states its basis and its uncertainty | TML-002, TML-006 |
| Auracle | The server decides where a trail can be drawn; gaps are not travel | MAP-002, MAP-003 |
| Auracle | Send only place centres to a geocoder | MAP-005 |
| Auracle | Check a reminder again at send time | REM-005 |
| Auracle | Redact before sealing | RED-003 |
| Auracle | Operator values in separate columns from synced values | MON-003 |
| HQ | Optimistic save with an expected revision, immutable history | WRT-003, WRT-004 |
| HQ | Secret status values and rotation policies; compare with `.env.example` | SCR-003, SCR-005 |
| HQ | `run` command that injects secrets into a command | SCR-006 |
| HQ | A banner for a failed default branch, bulk merge with a result list, run controls, workflow start by hand, a list of items that need the operator, the "Synced" line (in Anima "Read … ago", PRJ-032) | PRJ-031 to PRJ-036 |
| HQ | GitHub App for reads, with a mirror that is also polled | PRJ-017, PRJ-019 |
| HQ | Coding session import keyed by git remote, with worktree mapping | AIS-002 |
| HQ | A provider failure shows as a stale value, not an error page | MON-009 |
| HQ | Cache age by month: current month short, older months long | MON-004 |

Auracle receives webhooks and uses a public OAuth callback on its server. The Anima hub is not on the public internet (SEC-008). Thus Anima uses the loopback redirect (SRC-020) and a separate webhook receiver (Section 2.12).

## 9. Services and keys

Anima is a personal application for one operator. Register each developer application in the personal account of the operator, for personal use, with only the operator as a user. Do not ask for a business review.

- **KEY-001.** Each key and client secret in this section must go into the hub as a Docker secret or into the credentials domain. Do not put a key in a Compose file, a repository or a log (DB-002, LOG-007).
- **KEY-002.** The hub must show in Settings > Integrations which keys are present and which are missing. It must not show the values.
- **KEY-003.** The OAuth redirect for each provider is the loopback address of the desktop client (SRC-020). Register `http://127.0.0.1` or `http://localhost` as the redirect where the provider permits it.

### 9.1 Services that Anima runs itself (no key)

| Service | Used by | Note |
| --- | --- | --- |
| PostgreSQL, `guacd`, Headscale | Device manager | Already in Phase 1 |
| ntfy (self-hosted) | Reminders | For iPhone push, a self-hosted ntfy sends a poll request through `ntfy.sh` (message ID only, no text). The iPhone must be on the VPN to fetch the text. |
| Protomaps PMTiles file | Map | Download a regional or planet build. No account. |
| Photon geocoder | Map | Self-hosted with a country or region extract. No account. |
| Garmin bridge | Health | Section 4.9.1 |
| The collector in `anima-daemon` | AI sessions, Writing, People | On each operator computer (COL-001) |

### 9.2 Developer applications to register

| Service | Where to register | What you get | Used by | Note |
| --- | --- | --- | --- | --- |
| GitHub | GitHub > Settings > Developer settings > GitHub Apps > New GitHub App | App ID, installation ID, private key (`.pem` file); the webhook secret later (WHK-008) | Projects, People | Free. Anima makes the App (PRJ-039): press "Connect GitHub", then "Create GitHub App" and "Install" on GitHub, with no key file. The webhook stays off until the receiver is built (PRJ-019); set its URL and secret then (WHK-008). The manual route (PRJ-040) is the same steps by hand: "Read and write" for each repository permission (PRJ-017), "Only on this account", installed on your account for all repositories. |
| Strava | strava.com/settings/api | Client ID, client secret | Health, Map | Free. A new application allows one athlete, which is enough. Set the callback domain to `localhost`. |
| WHOOP | developer.whoop.com (developer dashboard) | Client ID, client secret | Health | Free. Needs a WHOOP membership. |
| Oura | cloud.ouraring.com > API applications | Client ID, client secret | Health | Free. Needs an Oura membership. Use OAuth, not a personal access token. |
| Etherscan | etherscan.io > API keys | API key | Money (MetaMask) | Free tier. One key covers all EVM chains. |
| Hosted vision model | The model provider console | API key | Wardrobe, Library, Health, Money extraction | Optional (OD-E4). Never for Documents. |

### 9.3 Personal tokens and app passwords (no developer application)

| Service | Where to make it | Used by | Note |
| --- | --- | --- | --- |
| Wise | Wise > Settings > API tokens | Money | Select "Read only". |
| Vercel | Vercel > Account settings > Tokens | Projects | Select the narrowest scope (account, team or project) that covers your projects, and an expiry date. Vercel has no read-only token, so PRJ-003 applies. |
| Neon | Neon console > Account settings > API keys | Projects | Neon has no read-only key, as far as we know. Verify, and use a project-scoped key if Neon gives one. PRJ-003 applies. |
| Hostinger | hPanel > Dev Tools > API | Projects | The token has the same rights as your hPanel user and has no read-only option. Set an expiry date. PRJ-003 and VPS-005 apply. |
| PostHog | PostHog > Settings > Personal API keys | Projects | Select read scopes only (for example `query:read`, `project:read`) and only your projects. |
| Gemini (exchange) | Gemini > Settings > API | Money | Make a key with the Auditor role only. |
| Coinbase | Coinbase Developer Platform > API keys | Money | Make a key with the View permission only. No trade or transfer permission. |
| Interactive Brokers | Client Portal > Performance and Reports > Flex Queries | Money | Make Activity Flex Queries (positions, trades, cash) and a Flex Web Service token. |
| SimpleFIN Bridge | bridge.simplefin.org | Money (BofA, Chase, Robinhood, Acorns) | Paid (about 15 USD each year). Connect each institution in the Bridge. Give Anima one setup token. |
| MetaMask | Copy the public addresses only | Money | No key. Never give a seed phrase or a private key. |
| Garmin Connect | Your Garmin email and password, one time | Health | The bridge keeps only the tokens (GAR-004). MFA is supported. |
| OpenRouter | openrouter.ai > Keys | Model gateway | Keep it as a global secret. Set a credit limit on the key. |
| Anthropic, OpenAI | The console of each provider > API keys | Model gateway | Only for the providers that the operator uses (EXT-000). Keep each key as a global secret. |

### 9.4 Exports to upload (no key)

| Export | Where to get it | Used by |
| --- | --- | --- |
| Google Maps Timeline | Phone: Google Maps > Settings > Timeline > Export | Map |
| Apple Health | iPhone: Health > profile > Export all health data | Health |
| InBody | InBody app or LookinBody export (CSV) | Health |
| Garmin (fallback) | Garmin account > Data management > Export your data, or FIT files | Health |
| Medical reports, lab results | PDF or photo | Health |
| Kindle highlights | `My Clippings.txt` from the device | Library |
| Goodreads | My Books > Import and export > Export library (CSV). Also copy the RSS link of each shelf. | Library |
| Letterboxd | Settings > Data > Export your data (zip). Also your diary RSS: `letterboxd.com/<username>/rss/` | Library |
| Instagram | Accounts Center > Your information and permissions > Download your information (JSON format) | People |
| X | Settings > Your account > Download an archive of your data | People |
| ChatGPT | Settings > Data controls > Export data | AI sessions |
| HSBC HK statements | HSBC HK online banking: e-statements (PDF) or transaction history | Money |
| Revolut statements | Revolut app: account > Statement (PDF or Excel) | Money |
| Gemini credit card statements | Gemini app: card > statements | Money |
| BofA, Chase, Robinhood, Acorns statements | Only if SimpleFIN does not support them | Money |

### 9.5 Accounts that cost money

| Account | Cost | When |
| --- | --- | --- |
| Hostinger VPS | Existing | Now |
| Apple Developer Program | 99 USD each year | Only for the iOS client (Phase 5) and for signed macOS releases |
| Hosted vision model | Pay for use | Only if OD-E4 is yes |
| SimpleFIN Bridge | About 15 USD each year | For the US accounts (OD-E3) |
| OpenRouter and model providers | Pay for use | For the model gateway (Section 4.15). A local model has no cost. |

## Council follow-up: a useful agent action (2026-11-18)

R-152 names the preparation below. It changes no approved requirement or contract. Read current `main` and the claims before taking a task. A source review is not a Passed human check. The task owner records the tested build and the result; unknown results stay unknown.

| Task | Work and acceptance | Depends on | State |
| --- | --- | --- | --- |
| AIS-C1 | Prepare a read-only machine-selection example with the existing capability query and its command/MCP surface. Use fixed reports with capable, stale and missing inventory. Report capability at the probe time; do not infer current reachability, credentials or successful execution. Compare one task with the user's current workflow; recruitment is separate. | CAP-030; existing tool inventory; no new scheduler or model gateway | Not started. Offline preparation only |
| AIS-C2 | Prepare explicit, content-free evidence cases for an agent that needs input. Separate host presence, quiet output, a lost connection and an actual request. Each synthetic case gives pane identity, UTC times, prior state, sequence and the observation that ends the wait. Mark synthetic rules as synthetic. Give missing vendor evidence to the pane owner before proposing an adapter. | Pane owner and its source contract; approval of pane work before adoption | Not started. Preparation only; production protocol and adapter are blocked |
