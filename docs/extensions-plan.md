# Anima Extensions Plan

| Item | Value |
| --- | --- |
| Document | Extensions plan |
| Version | 0.1 (draft) |
| Date | 2026-11-03 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | For review |
| Related | [design-spec.md](design-spec.md), [implementation-plan.md](implementation-plan.md) |

This document obeys the writing rules in Section 0 of the design specification.

## 1. Purpose and scope

### 1.1 Purpose

This plan adds 15 extensions to Anima. With the extensions, Anima also keeps a record of the life and work of the operator. The device manager stays the foundation.

Anima has three layers. The model is the Obsidian application: a small foundation, and functions that each person sets on or off.

| Layer | Contents | Can be set off |
| --- | --- | --- |
| Foundation | The device manager of the design specification (devices, checks, sessions, the VPN, the hub) and the engine of Section 2 | No |
| Extensions | The 15 extensions of this plan. The Anima project builds them, and they are part of each installation. | Yes, each one (Section 2.8) |
| Community extensions | Extensions from other people. Not in this plan (EXT-009). | Yes |

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

- **COL-001.** The collector must be a separate Rust binary (`anima-collector`). It must start at login with launchd on macOS and systemd on Linux. It must run when the client is closed.
- **COL-002.** The collector must send records to the hub through the VPN only.
- **COL-003.** Each collector source must use its own ingest token. An ingest token can only add records for one source account. It cannot read data.
- **COL-004.** The collector must keep a read position for each file, so that it sends only new data.
- **COL-005.** The collector must redact secrets before it sends text (Section 2.5).
- **COL-006.** The operator must select each local folder that the collector reads. The collector must not follow symbolic links out of a selected folder.
- **COL-007.** If the hub is not available, the collector must keep the records in a local queue and send them later. The queue must have a size limit.
- **COL-008.** The parsers of the collector must be in a library crate with no I/O, so that tests can use fixture files.

### 2.3 Timeline

The timeline joins the records of all sources. History, Map and Reminders read it. Auracle calls this "Evidence".

- **TML-001.** The timeline must not copy data. Each domain gives an adapter that maps its table rows to Evidence.
- **TML-002.** Each Evidence entry must have: a reference (`table:id`), the source, the category, the start time, an optional end time, the basis and a list of uncertainty notes.
- **TML-003.** The basis must be one of: observed, planned, reported by the operator, or inferred.
- **TML-004.** The categories are a closed list: sleep, workout, visit, travel, code, message, post, media, reading, spending, document, device, session and note. Add a category only with a change to this plan.
- **TML-005.** When two sources record the same event, the timeline must keep both. The lower-ranked entry must point to the higher-ranked entry with `superseded_by`. Do not delete it.
- **TML-006.** An adapter must write an uncertainty note when it does not know a value, for example "end time not given by the source".
- **TML-007.** The event log of the device manager (Section 3.8 of the spec) must also be a timeline source.

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

- **FIL-001.** The hub must store files in a named Docker volume by content hash. The database keeps the metadata.
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

- **EXT-001.** The operator must be able to set each extension on or off in Settings. The default is off.
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
| All other extensions | — | As their seams in Section 5.4 |

- **EXT-008.** At the first start, the client must ask the operator what Anima must do, and must offer sets of extensions. One set is the developer set: Projects, Secrets, Model gateway and AI coding sessions. The operator can select a set, select single extensions, or select none. The operator can change the selection in Settings at any time.
- **EXT-009.** Community extensions are a later phase. Do not start them until the 15 extensions of this plan are built. The first function for other people must be a new source (Section 2.1), because a source gives records only and gets only its own token. The hub must not run the code of other people until a design for isolation and permissions is approved. (operator, 2026-11-13)

### 2.9 Vision and text extraction

Wardrobe, Library, Documents, Health and Money extract fields from photos and PDFs.

- **VIS-001.** Each extraction result is a proposal. The operator must confirm it before Anima saves it.
- **VIS-002.** Extraction for Class A data must run on the client, for example with the Apple Vision framework on macOS. Do not send a Class A image to a hosted model.
- **VIS-003.** Extraction for Class B and C data can use a hosted model only if the operator permits it for that extension (OD-E4).
- **VIS-004.** The extraction function must return typed fields and a confidence value for each field.

### 2.10 Record links

Extensions refer to each other only through record links. Thus no extension waits for a different extension.

- **LNK-001.** The engine must keep one `record_link` table: a source reference, a target reference (`table:id`, as TML-002) and a link kind.
- **LNK-002.** An extension can link to a record of an extension that does not exist yet. Until the target exists, the link keeps a text label, for example the name of a person.
- **LNK-003.** Each record page must show its links with one shared component. An extension does not write code for the links of a different extension.
- **LNK-004.** The engine must give shared subject keys that every extension can use without an import: a GitHub repository name, a `PlaceId`, a `PersonId` and a date.

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

## 3. Client

### 3.1 Pages

Each extension adds one route in `apps/client/src/routes/`. The route composes components from `$lib/<domain>/` (CQ-044).

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

- **NAV-010.** The operator must be able to select the start page. The default stays Devices (NAV-004).
- **NAV-011.** A group with no active items must not show.
- **NAV-012.** The client must have a command palette (Cmd+K) that opens each page and each record type.

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
- **PRJ-012.** The Projects page must show all projects as cards. Each card shows the production state, the last deployment time, the uptime and, if the project has analytics (PRJ-015), the users of the last 7 days.
- **PRJ-013.** A failed production deployment must make an event log entry and can trigger a reminder (Section 4.11).
- **PRJ-014.** The project page must also show the AI coding sessions (Section 4.12), the drafts (Section 4.3), the secrets (Section 4.4) and the model use (Section 4.15) of the project. It must show a section only when its extension is on (EXT-007).
- **PRJ-017.** Anima must connect to GitHub as a GitHub App that the operator makes in their own account (Section 9.2). The App has read and write permission on each repository permission that GitHub offers. This is an exception to SRC-010. Reason: the operator wants Anima to be their GitHub, not only a view of it. (operator, 2026-11-13)
- **PRJ-018.** The hub must keep the App ID, the installation ID and the private key of the App with the credentials domain (SRC-008, SEC-003). The hub signs a short token with the private key and gets an installation token from GitHub. An installation token is valid for one hour. The hub must keep it in memory only and must not write it to the database or to a log (LOG-007).
- **PRJ-019.** The App must send its webhook events to the webhook receiver (Section 2.12). The hub must also read GitHub on a slow interval, because a webhook delivery can be lost (WHK-006). No OAuth redirect is necessary for an installation token, so SRC-020 does not apply to GitHub. (operator, 2026-11-13)
- **PRJ-027.** A full permission does not make a function. Each change that Anima can make on GitHub must be one named function with its hub API, its `anima` command and its MCP tool (CMD-030). The MCP tool of a change is off until the operator sets it on (MCP-002). OD-E14 gives the first functions.
- **PRJ-028.** The hub must record each change that it makes on GitHub in the audit log: the function, the repository, the target and the actor kind (LOG-012).
- **PRJ-029.** A change that cannot be undone (delete a branch, delete a repository, change a repository setting, force a push) must ask for a confirmation that names the target (CMD-024, MCP-007).
- **PRJ-016.** The subject key of a project is the name of its GitHub repository (LNK-004). Other extensions keep the repository name with their records. Thus their records show under the project when Projects is set on later, with no change to the stored records.

#### 4.2.2 Pages for GitHub work

The ideas come from the Projects pages of HQ. HQ also has parts that Anima must not copy; PRJ-038 lists them.

- **PRJ-030.** The project page must have three tabs only: Overview (PRJ-004 to PRJ-009), Work, and Design and map (Section 4.2.1, only when that extension is on). A tab must not show before its function is built.
- **PRJ-031.** The Work tab must show, in this order: a banner when the default branch fails its checks; the workflow runs that run now; the open pull requests; the open issues. Each list is one data path from the hub. Do not build a second path for the same list.
- **PRJ-032.** Each list must show when the hub last read GitHub, for example "Synced 2 min ago", and must show "Stale" when the last read failed. Loading, error, not connected and empty must be four different states with different text.
- **PRJ-033.** The pull request list must show for each row if the pull request can merge (no conflict, checks passed). The operator can select rows and merge them in one action. The action must ask for a confirmation and must then show which merges succeeded and which failed.
- **PRJ-034.** A workflow run must have these controls: run again, run only the failed jobs again, cancel. Each confirmation must show the commit.
- **PRJ-035.** The operator must be able to start a workflow by hand, with a branch and the named inputs of that workflow.
- **PRJ-036.** The Projects page can show one list of items that need the operator across all projects: failed default branches, pull requests that wait for the operator, and issues assigned to the operator. The operator can mark an item "Done" or "Snooze 1 day". The item must come back when it has new activity.
- **PRJ-037.** A project card can show one small chart for each of these weekly counts: merged pull requests, issues opened and closed, failed workflow runs. Each chart has its own scale.
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
| ntfy on the hub, to the ntfy app on iPhone and macOS | No client is open |
| Anima iOS push | Phase 5 |

- **REM-001.** The rule evaluation must be a pure function. It receives the rule, the new records, the place state and the current time. It returns a list of reminders to send (CQ-021).
- **REM-002.** The hub must evaluate source-event rules when a sync run stores new records, and time rules on a 1-minute sweep.
- **REM-003.** A rule can have an "unless" condition, for example "unless a workout is already recorded today".
- **REM-004.** Each rule must have a cooldown and a daily limit. The defaults are 60 minutes and 3.
- **REM-005.** The hub must check the rule again just before it sends. If the operator changed the rule, the hub must not send the old reminder.
- **REM-006.** Place rules must use the place state (inside or outside). A new fix inside the place radius is an arrival only if the last state was outside.
- **REM-007.** The operator can snooze or acknowledge a reminder from the notification.
- **REM-008.** The hub must run ntfy on the Docker network and publish it only on the VPN interface (NET-003). Each topic name must be random.
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
- **AIS-002.** The collector must link a session to a repository by the git remote of its working folder. The repository name is the subject key (LNK-004). When Projects is on, the session shows under the project that has this repository (PRJ-016). It must map worktree paths (`.claude/worktrees/*`, `.codex/worktrees/*`) to the main repository.
- **AIS-003.** The collector must keep sub-agent sessions as child sessions of their parent session.
- **AIS-004.** The collector must redact the text (Section 2.5). It must not send tool output. It can send the tool name and the tool input, clipped to 2000 characters.
- **AIS-005.** The hub must keep a maximum of 64 000 characters for each turn and mark a clipped turn.
- **AIS-006.** The page must show: sessions by repository and by day (by project when Projects is on), the model, the duration, the counts, and a search in the prompts of the operator.
- **AIS-007.** Each session must be a timeline entry with the category "code".
- **AIS-008.** The operator must be able to exclude a repository or a folder. The collector must not read sessions from an excluded folder.

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
| K-E4 | Places, files, record links and the shared subject keys (LNK-004) | `contracts/extensions/shared-records.md` |
| K-E5 | Vault: data classes, sealing, client-sealed records, reveal | `contracts/extensions/vault.md` |
| K-E6 | Reminder trigger interface: how an extension gives an event, an expiry date or a threshold | `contracts/extensions/reminders.md` |
| K-E7 | Extension registry: switch, navigation entry, data class, migration range | `contracts/extensions/registry.md` |
| K-E8 | Migration ranges and owned paths for each extension (Section 5.2) | `contracts/extensions/ownership.md` |

- **ENG-010.** The engine must include one sample extension that uses each contract, with tests. Use the Writing extension for this, because it is small and has no outside source. Thus XP-03 is part of E0.
- **ENG-011.** At the end of E0, the security reviewer (Opus) must accept the vault (PH-E-002).
- **ENG-012.** At the end of E0, the programme orchestrator must write one launch brief for each extension in `docs/programme/extensions/XP-NN.md`. A brief contains: the goal, the plan section, the owned paths, the contracts that it uses, the sources and keys, the acceptance criteria of Section 5.3, the gates and the builder models.
- **ENG-013.** The operator starts each extension workflow from its launch brief. A workflow needs no other context.

The engine and the grouped items are GitHub issues with the `extensions` label:

| Issue | Item |
| --- | --- |
| #4 | E0-1: Regroup hub folders |
| #5 | E0-2: Sources framework and Settings > Integrations |
| #6 | E0-3: Collector and redaction |
| #7 | E0-4: Timeline and record links |
| #8 | E0-5: Places, file store and photo field reading |
| #9 | E0-6: Vault and its security review |
| #10 | E0-7: Reminder engine and ntfy delivery |
| #11 | E0-8: Extension registry, nav groups and launch briefs |
| #26 | SPEC: Approve the design spec changes (Section 1.2) |
| #27 | OD-E1: Decide: where Class B data lives |
| #28 | OD-E4: Decide: hosted vision model |
| #29 | OD-E10: Decide: price source |
| #30 | OD-E3: Check SimpleFIN coverage |
| #31 | KEYS: Accounts, keys and setup checks (Section 9) |
| #32 | SEAMS: Seam tasks (Section 5.4) |

### 5.2 Extension work packages

Each extension owns only its paths. It adds one line to each registry of the engine (K-E1, K-E2, K-E7). The orchestrator of the workflow merges these registry lines.

| ID | Issue | Extension | Section | Hub domain | Client | Other owned paths | Migrations | Tags |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| XP-01 | #12 | Map | 4.1 | `life/locations/` | `routes/map/`, `$lib/map/` | `providers/location/`, PMTiles and Photon in `deploy/` | 2100–2199 | — |
| XP-02 | #13 | Projects | 4.2 | `work/projects/` | `routes/projects/`, `$lib/projects/` | `providers/{github,vercel,neon,hostinger,posthog}/` | 2200–2299 | security (tokens, PRJ-003; GitHub App key and write functions, PRJ-017 to PRJ-029) |
| XP-03 | #14 | Writing | 4.3 | `work/writing/` | `routes/writing/`, `$lib/writing/` | `collector/obsidian/` | 2300–2399 | Part of E0 (ENG-010) |
| XP-04 | #15 | Secrets | 4.4 | `vault/secrets/` | `routes/secrets/`, `$lib/secrets/` | `crates/anima-cli/` | 2400–2499 | security |
| XP-05 | #16 | People | 4.6 | `life/people/` | `routes/people/`, `$lib/people/` | `collector/{contacts,imessage}/`, `providers/{instagram,x}/` | 2500–2599 | — |
| XP-06 | #17 | Money | 4.7 | `life/money/` | `routes/money/`, `$lib/money/` | `providers/{wise,simplefin,gemini,coinbase,etherscan,ibkr}/`, statement parsers | 2600–2699 | security (financial data) |
| XP-07 | #18 | History | 4.8 | `life/history/` (queries only) | `routes/history/`, `$lib/history/` | — | none | — |
| XP-08 | #19 | Health | 4.9 | `life/health/` | `routes/health/`, `$lib/health/` | `providers/{strava,whoop,oura,garmin,apple_health,inbody}/`, `deploy/garmin-bridge/` | 2800–2899 | — |
| XP-09 | #20 | Documents | 4.10 | `vault/identity_documents/` | `routes/documents/`, `$lib/documents/` | on-client extraction in `$lib/documents/` | 2900–2999 | security |
| XP-10 | #21 | Reminders page | 4.11 | none (engine) | `routes/reminders/`, `$lib/reminders/` | — | none | — |
| XP-11 | #22 | AI coding sessions | 4.12 | `work/assistant_sessions/` | `routes/ai-sessions/`, `$lib/assistant-sessions/` | `collector/{claude_code,codex}/`, `providers/chatgpt/` | 3100–3199 | security (redaction) |
| XP-12 | #23 | Wardrobe | 4.13 | `life/belongings/wardrobe/` | `routes/wardrobe/`, `$lib/wardrobe/` | — | 3200–3299 | — |
| XP-13 | #24 | Library | 4.14 | `life/belongings/library/` | `routes/library/`, `$lib/library/` | `providers/{open_library,goodreads,letterboxd,kindle}/` | 3300–3399 | — |
| XP-14 | #25 | Project design and map | 4.2.1 | `work/projects/design/` | `$lib/projects/design/`, `$lib/projects/map/` | — | 3400–3499 | Needs XP-02 first |
| XP-15 | none | Model gateway | 4.15 | `fleet/model_gateway/` | `routes/model-gateway/`, `$lib/model-gateway/` | `crates/anima-cli/` | 3500–3599 | security (keys, MGW-005). Needs XP-04 first |

- **XP-001.** An extension workflow must not change an engine path or a path of a different extension. If it needs an engine change, it stops and reports a contract change request to the operator.
- **XP-002.** Each workflow uses the task loop of the implementation plan (Section 3.1): plan, build, gate, review, fix. A task with a security tag needs the security reviewer (AGT-004).
- **XP-003.** Each workflow works in its own worktree on the branch `xp/NN-slug`. It merges into `main` only after the gates pass and the operator confirms.
- **XP-004.** Only XP-14 and XP-15 depend on a different extension (XP-02 and XP-04). All other workflows can run in any order and at the same time.
- **XP-005.** Suggested order when the operator runs them one at a time: XP-11, XP-02, XP-01, XP-07, XP-10, XP-13, XP-12, XP-05, XP-08, XP-06, XP-04, XP-15, XP-09, XP-14. Low-risk developer data first; the vault-heavy extensions after the engine has some use.

### 5.3 Acceptance criteria for each extension

| ID | The extension is done when |
| --- | --- |
| XP-01 | An OwnTracks or Overland fix and a Google Timeline export show as trails and visits. A gap shows as a dashed line. No tile request leaves the network. |
| XP-02 | With all other extensions off, each GitHub project shows its Vercel, Neon and Hostinger deployments with their state. A test proves that the clients of Vercel, Neon and Hostinger refuse a write request. The hub reads GitHub with an installation token of the App, and a test proves that the private key and the token are not in the database as plain text or in a log. Each GitHub change of OD-E14 works from the command line and is in the audit log. PostHog and Vercel analytics show where they exist. |
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
├── collector/            anima-collector binary (E0) and its parsers (XP-owned modules)
└── anima-cli/            The `anima run` and `anima secrets` commands (XP-04); the `anima gateway` commands (XP-15)
```

- **REPO-E-001.** Without groups, the hub `src/` folder has more than 12 domains (REPO-005). As the first task of E0, group the domains under the parent nouns above. This is OD-E5.
- **REPO-E-002.** Provider normalisers must not import a hub domain. The hub calls them.
- **REPO-E-003.** Migration number ranges for the new domains must be in contract K2.

## 7. Open decisions

| Number | Decision | Recommendation |
| --- | --- | --- |
| OD-E1 | Hub location for Class B data. The hub is on the VPS (OD-9). | Seal Class B with separate data keys (VLT-003). Before XP-05, XP-06 and XP-08, decide if a home hub must keep Class B data and the VPS hub only the device manager. |
| OD-E2 | Collector form: separate binary or part of the desktop client | Separate binary (COL-001). It runs when the client is closed and on computers without the client. |
| OD-E3 | Aggregator for the US accounts (BofA, Chase, Robinhood, Acorns) | SimpleFIN Bridge. It is for private persons and costs about 15 USD each year (its pages give different prices). Check each institution on bridge.simplefin.org before payment. HSBC HK and Revolut use statement uploads. |
| OD-E4 | Hosted vision model for Class B and C extraction | Permit a hosted model for Wardrobe and Library photos, with the operator switch (VIS-003). Use only on-device extraction for Documents. |
| OD-E5 | Hub folder grouping (REPO-E-001) | Group by parent nouns as the first task of E0 (`fleet/`, `engine/`, `vault/`, `life/`, `work/`), in a separate change. |
| OD-E6 | Telemetry for projects | Decided: PostHog and Vercel Web Analytics for analytics, Anima checks for uptime, and the service APIs for deployment state. No custom telemetry endpoint. |
| OD-E7 | Garmin access | Use the Garmin bridge with `python-garminconnect` (Section 4.9.1). Keep FIT and export uploads as the fallback. Accept the risk that Garmin blocks unofficial access. |
| OD-E8 | Obsidian two-way sync | Read-only in this version (WRT-005). Decide after XP-03. |
| OD-E9 | People sources | Decided: macOS Contacts, iMessage, Instagram, X and GitHub. Bluesky is not needed. |
| OD-E10 | Price source for holdings that the source does not price | IBKR, Coinbase and Gemini give their own prices. For MetaMask tokens and the SimpleFIN holdings, select one free price API. |
| OD-E11 | Path for a request to a local upstream (MGW-010). A program such as Claude Code has one base address, so it cannot select the path itself. | The `anima` program on each computer listens on the loopback address. It sends a request for a local upstream directly to the device on the VPN, sends all other requests to the hub, and reports the use to the hub. Not verified: the added work and the failure cases. Alternative: all requests go through the hub, and local requests are slower. |
| OD-E12 | Secrets on the hub (VLT-007) | Decided: the hub seals and can open each secret. One class of secret with two scopes, global and project (SCR-009). Each read is in the audit log; a piped read refuses a terminal. No other limit. (operator, 2026-11-13) |
| OD-E13 | Is Projects a part of the foundation? | Decided: no. The foundation is the device manager and the engine. Projects is an extension, off by default. Only the project design and map needs it; Secrets, the model gateway and AI coding sessions work with it (EXT-005, EXT-006, EXT-010). Community extensions come after the 15 extensions, sources first (EXT-009). (operator, 2026-11-13) |
| OD-E14 | The first GitHub functions that make a change (PRJ-027) | Recommendation: make an issue, add a comment, close or open an issue, merge a pull request (also many in one action, PRJ-033), run a workflow again, run its failed jobs again, cancel a run, start a workflow by hand (PRJ-034, PRJ-035), write a commit status for a build of Anima (PRO-010, PRO-020 of the projects proposal). Add others one at a time when the operator needs them. Not decided. |
| OD-E15 | Where the webhook receiver runs (Section 2.12) | Decided: on the VPS, as a small separate program that is not the hub (WHK-009). It works where the hub runs and needs no other service. This changes SITE-006 ("the website is the only public part of Anima"). To open the port on the VPS needs the operator's confirmation at that time (VPS-005). (operator, 2026-11-13) |

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
| HQ | A banner for a failed default branch, bulk merge with a result list, run controls, workflow start by hand, a list of items that need the operator, the "Synced" line | PRJ-031 to PRJ-036 |
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
| `anima-collector` | AI sessions, Writing, People | On each operator computer |

### 9.2 Developer applications to register

| Service | Where to register | What you get | Used by | Note |
| --- | --- | --- | --- | --- |
| GitHub | GitHub > Settings > Developer settings > GitHub Apps > New GitHub App | App ID, installation ID, private key (`.pem` file), webhook secret | Projects, People | Free. Set the webhook URL to the address that Anima shows (WHK-008) and set a webhook secret. Select "Read and write" for each repository permission (PRJ-017). Select "Only on this account". Then install the App on your account for all repositories. The operator does these steps; Anima does not make the App. |
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
