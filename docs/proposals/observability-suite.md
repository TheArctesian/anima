---
kind: Proposal, with a task table
state: Draft
evidence: Version 1 (2026-11-19). Made from the idea `observability-suite` (`5c9fc563`); this commit deletes the idea folder (PLN-018). The direction in Section 2 is relayed by an agent and is not a ruling: OD-OBS1 to OD-OBS14 ask the operator first-hand. Code state checked on `origin/dev` at `5c9fc563`. No requirement is approved and no task may start. Serves row 4.8 of the priorities (relayed, not confirmed; OD-OBS5).
---

# Proposal: an observability suite (status pages, error tracking and analytics) as extensions of the hub and the daemon

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this proposal (PLN-005).

| Item | Value |
| --- | --- |
| Document | Proposal: public status pages, an error tracker for the programs on the operator's machines, and the decision on product analytics and web analytics, beside the checks, metrics and notifications that exist |
| Version | 1 |
| Date | 2026-11-19 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | The header of this document holds the state (PLN-020). History: Draft. Made from the idea `observability-suite` (PLN-018); the commit that adds this document deletes the idea folder. The four answers of the operator in Section 2 came through two agents. They are the working direction only. Section 9 asks for each of them first-hand. |
| Serves | Row 4.8 of [priorities.md](../plan/priorities.md), The other extensions (relayed). [NEEDS CLARIFICATION: OD-OBS5. Row 4.8 waits for row 4.4 and for R-144 (PH-E-000 is in force for the other extensions until WP-53).] |
| Related | [design-spec.md](../spec/design-spec.md) Section 6.4 (CHK), 6.5 (maintenance windows), 6.8 (NOT), INT-001 to INT-004, SEC-008, NET-003, DNS-005, SITE-006 to SITE-008, LOG-007, DB-008, DB-009; [extensions-plan.md](../plans/extensions-plan.md) Section 2.8 (EXT), 2.12 (WHK), 4.2 (PRJ), 5.2 (XP-005), Section 7 (OD-E6, OD-E15, OD-E18); [hosted-applications-plan.md](../plans/hosted-applications-plan.md) (HOST-001, HOST-015, HOST-016); [hosted-mode.md](hosted-mode.md) (HMD-011, HM-R6); [watchdog.md](watchdog.md) (WDG-035); [agent-tools-as-extensions.md](agent-tools-as-extensions.md) (the precedent for a switch outside the 15) |
| Identifiers | OBS-001 to OBS-056 (with gaps between the groups), OD-OBS1 to OD-OBS14, tasks OB0 to OB17 |

## 1. Goal

Give the operator, inside Anima.ing, the parts of an observability suite that fit a device manager: a public status page from the checks that exist, and a list of the errors of the programs that run on the operator's machines. Decide first-hand if Anima.ing also stores product analytics and web analytics of its own, or continues to read them from PostHog and Vercel Web Analytics (OD-E6).

The trigger is a public post of the founder of Coolify, 2026-11-19, 12:17 UTC (04:17 PT): "I am working on a Posthog, Sentry, Better Stack, Plausible, Dub (etc) alternative that is just one binary on a few MB of RAM and backed by our open source philosophy." (https://x.com/heyandras/status/2123390206464209146, read by Anima Product Ideas through the X connector on 2026-11-19). The post is a teaser. It names no product, repository, licence or date.

## 2. The direction of the operator (relayed, not a ruling)

The operator replied "ya we should also be doing this" to the post (relayed by the Anima Marketing Bot, 2026-11-19, 14:15 PT; the Marketing Bot confirmed that the operator wrote it himself). The operator then answered four questions on a card. Anima Product Ideas relayed the picks verbatim (2026-11-19, 15:20 PT):

1. "Scope: all of them, including analytics"
2. "Order: both together"
3. "Ship: inside the hub and daemon as extensions that are off by default"
4. "Priority: under 4.8 The other extensions"

These words came through agents. They are not rulings (PLN-021). This proposal uses them as the working direction, and OD-OBS1, OD-OBS3, OD-OBS4 and OD-OBS5 ask the operator to confirm each one first-hand. "Both together" does not say which two pieces. This proposal reads it as status pages and error tracking, the two pieces that the card put first. [NEEDS CLARIFICATION: OD-OBS3.]

## 3. Overlap check (PLN-004)

Files read: [plan/ownership.md](../plan/ownership.md), [plan/register.md](../plan/register.md), [deferred.md](../deferred.md), [plan/priorities.md](../plan/priorities.md), [rulings.md](../rulings.md), design-spec.md Sections 3.8, 3.12, 4, 6.4, 6.5, 6.8 and 11, extensions-plan.md Sections 2.8, 2.12, 4.2, 5.2 and 7. Checked 2026-11-19 at `5c9fc563`.

| Piece | Owner now | What exists | What this proposal adds |
| --- | --- | --- | --- |
| Uptime checks | design-spec.md (ownership.md row "Device manager: ... checks, metrics, notifications") | CHK-001 to CHK-014, NOT-001 to NOT-008, INT-001 to INT-003, DB-008 and DB-009 (day aggregates for 2 years). WDG-035 (a watch is a check) is proposed, not approved | Nothing. Do not rebuild a check |
| Status pages, incidents, badges | design-spec.md, INT-004 only: "A later version can add public status pages, incidents and status badges." | No requirement, no row, no item in `deferred.md` | OBS-010 to OBS-019. At approval they replace INT-004 (Section 7) |
| Error tracking | None | No requirement for the errors of the operator's programs. Neighbours: HOST-001 (the last lines of a container log; built, stage 1), HOST-015 (search in those lines; client only), HOST-016 (follow; waits for stage 2), CHK-014 (restart loops), LOG-001 to LOG-028 (the hub's own logs and audit log, not the logs of other programs), RED (the shared redaction crate of E0-3, built, R-166) | OBS-020 to OBS-032 |
| Product analytics | extensions-plan.md Section 4.2 (Projects): PRJ-009, PRJ-012, PRJ-015; OD-E6 "No custom telemetry endpoint" | The hub reads PostHog each hour | OBS-040 to OBS-044, only if OD-OBS6 reverses OD-E6 |
| Web analytics | extensions-plan.md (PRJ-009) for the operator's projects; design-spec.md SITE-008 and OD-21 for www.anima.ing; user-documentation.md (R-207, OD-UD7) for docs.anima.ing | Vercel Web Analytics is read (PRJ-009) and is the one exception on the two websites | OBS-050 to OBS-056, only if OD-OBS7 permits a collector |
| Short links (Dub, in the post) | None | Nothing | Nothing. OD-OBS2 asks if it is out |
| A public program on the VPS | extensions-plan.md Section 2.12 (WHK-009, OD-E15) | The webhook receiver is already decided as a separate public program on the VPS. OD-E15 says: "This changes SITE-006" | A possible path for a public status page or ingest (OD-OBS8, OD-OBS10) |

Result of the check: no plan owns status pages or error tracking. The extension parts belong to extensions-plan.md (ownership.md row "Engine and extensions"). PLN-004 says to change that plan. This proposal is a separate draft for now, because it also changes design-spec.md (INT-004, SITE-006) and extensions-plan.md OD-E6, and both changes need a ruling first. At approval, Section 7 moves the requirements into their owners and this document keeps only the history (OD-OBS14). This is the same path as agent-tools-as-extensions.md.

### 3.1 Check of the claims of Anima Product Ideas

| Claim | Result |
| --- | --- |
| Uptime is covered by CHK-001 to CHK-014, NOT-001 to NOT-008, INT-001 and WDG-035 | Correct for CHK, NOT and INT-001: design-spec.md lines 1206 to 1219, 1330 to 1337 and 1341. Correction: WDG-035 is in [watchdog.md](watchdog.md), whose requirements are not approved (R-162 decides the direction only), and the code has no check type `watch` (`crates/hub/src/fleet/checks/config.rs:29` has 12 types). So WDG-035 is planned, not built |
| Status pages exist only as INT-004, "later version" | Correct (design-spec.md line 1344). Addition: OD-E15 already permits a public program on the VPS (the webhook receiver), and says that it changes SITE-006. The idea did not name it |
| Error tracking is new; nearest are LOG-001 to LOG-028 and HOST-015 and HOST-016 | Correct that it is new: no requirement and no code (no match for `sentry` in `crates/` and `apps/`). Correction of the neighbours: LOG-001 to LOG-028 are the hub's own process log, event log and audit log, not the logs of other programs. The nearest source is HOST-001, which is built: the hub runs `docker logs` for the last lines (`crates/hub/src/work/containers/docker.rs:44`). HOST-015 is client-only search over those lines. HOST-016 waits for stage 2 |
| Row 4.8 brings in EXT-009 and the order of the 15 extensions | Partly correct. EXT-009 holds back community extensions, not extensions that Anima.ing writes. The new extensions do change the number "15" in EXT-009, in extensions-plan.md Section 1.1 and in the code (`static REGISTRY: [Entry; 15]`, `crates/hub/src/engine/extensions/registry.rs:22`). The order is XP-005. Missed: row 4.8 waits for row 4.4, and R-144 (OD-E18) keeps PH-E-000 in force for the other extensions until the acceptance run WP-53 |
| Effort is L or more for all four pieces | An estimate. Not verified. The estimate of this proposal is in Section 8.1 |
| The post names four products (Marketing's note) | The idea already corrected this: it names five, with "Dub" and "(etc)" |

## 4. Current state in the code (origin/dev at `5c9fc563`, 2026-11-19)

- Checks: `crates/hub/src/fleet/checks/` has the scheduler, the probes, push checks, uptime from aggregates (`store/uptime.rs`, CHK-011) and the Prometheus exposition (`exposition/`, INT-001; metric `anima_check_uptime_ratio` with windows 1d, 7d and 30d).
- Notifications: `crates/hub/src/fleet/notifications/` has channels, decision, dispatch, egress and templates (NOT-001 to NOT-008).
- Extensions: `crates/hub/src/engine/extensions/` has the registry of 15 entries, the switches and the GraphQL (EXT-001 to EXT-012, K-E7). Only Projects is built. Migration ranges 2100 to 3599 are used by XP-01 to XP-15.
- Container logs: `crates/hub/src/work/containers/docker.rs` (HOST-001). The daemon has no reader for the systemd journal or for application logs (no match in `crates/daemon/src` except a test of the agent sessions reader).
- Redaction: `crates/redaction/` (E0-3 T1 and T2, R-166).
- Command line and MCP: `anima checks` and the MCP tools `anima_check_create`, `anima_check_update`, `anima_check_delete` and `anima_check_set_paused` exist (`crates/anima-cli/src/mcp/tools/checks.rs`).
- Status pages, error tracking, product analytics and web analytics: nothing in the code (no match for `status_page`, `sentry`, `posthog` or `plausible` in `crates/` and `apps/`).
- The client has no page for any of them (`apps/client/src/routes`).

## 5. Constraints

- **C1.** The hub stays off the public internet (SEC-008, NET-003, DNS-005). A hosted hub has no public port other than the relay (HMD-011). No part of this proposal opens a port on the hub.
- **C2.** The foundation operates with every extension off (EXT-005). An extension reads the checks through the read functions of the checks domain, not through its tables.
- **C3.** Build only for what the operator uses (EXT-000). Each source of errors and each publish target is one that the operator uses now.
- **C4.** No secret, no password, no token and no session content is stored (LOG-007). Each payload goes through the shared redaction crate before it is stored.
- **C5.** The websites `www.anima.ing` and `docs.anima.ing` keep SITE-007 and SITE-008 with their one exception (OD-21, R-207). A status page is not a page of those websites unless a ruling says so.
- **C6.** The free hosted hub sleeps when no device of the account is connected (hosted-mode.md, HM-R6). A check does not run while the hub sleeps. A status page must not show Up for a time with no data.
- **C7.** The order of CMD-030: the hub API, then the command line, then MCP, then the client.

## 6. Requirements (proposed, not approved)

The text in this section is a proposal. A session builds only the parts that a ruling names (PLN-007).

### 6.1 Common rules

- **OBS-001.** Each piece is one extension in the registry, off by default (EXT-001): `status_pages`, `errors`, and, if OD-OBS6 or OD-OBS7 permits, `product_analytics` and `web_analytics`. [NEEDS CLARIFICATION: OD-OBS1, OD-OBS11.]
- **OBS-002.** Each extension declares its data class (EXT-004): `status_pages` Class C (the operator publishes it); `errors` Class B (messages can hold personal data after redaction); `product_analytics` and `web_analytics` Class B.
- **OBS-003.** Relations (EXT-006, EXT-010): none of them needs another extension. `status_pages` and `errors` work with Projects (a project card shows its status component and its open error groups). `errors` works with Hosted applications.
- **OBS-004.** The foundation must not read a table of these extensions (EXT-005). `status_pages` reads check state and day aggregates through the checks domain only.
- **OBS-005.** Migrations use the ranges 3600 to 3999, 100 for each extension. [NEEDS CLARIFICATION: confirm that no other draft takes these ranges before the contract change of task OB2.]
- **OBS-006.** No new program runs next to `anima-daemon` and the hub for status pages or for the errors of programs on the operator's machines. A public program is permitted only where OD-OBS8 or OD-OBS10 says so.
- **OBS-007.** Each stored text goes through the shared redaction crate first (LOG-007, RED-002).
- **OBS-008.** Setting an extension off keeps its data; "Delete data" removes it (EXT-003). Setting `status_pages` off also stops each scheduled publish.

### 6.2 Status pages

- **OBS-010.** The operator can make a status page with a title, an optional short text, and an ordered list of components. A component is one check or one check of the type Group (Section 6.4), with a display name that the operator writes.
- **OBS-011.** For each component the page shows the current state (Operational, Degraded, Down, Maintenance, No data) and 90 daily bars of uptime from the day aggregates (DB-008, DB-009, CHK-011). A maintenance window (design specification Section 6.5) shows as Maintenance and does not count as down time.
- **OBS-012.** The page must not show an address, a host name, a port, a device name, a hub name or a check target. It shows only the display names that the operator writes (SITE-006, DNS-005). The export refuses a display name that looks like an IP address, a host name with a dot, or a URL, and names the field.
- **OBS-013.** The hub writes the page as a static bundle: `index.html`, `status.json`, one SVG badge for each component and one for the whole page, and the style and script files of the page. The bundle loads nothing from another origin and sends no request to the hub (C1). It has no cookie and no analytics.
- **OBS-014.** The bundle holds the time it was made. When the bundle is older than 15 minutes, the page says "Status not current, last updated <time>" and shows no state as Operational. [NEEDS CLARIFICATION: the limit; 15 minutes is a recommendation.] A time when the hub slept or the check had no beat shows as No data, not as up (C6).
- **OBS-015.** Incidents. The operator can write an incident with a title, the components it affects, a state (Investigating, Identified, Monitoring, Resolved) and updates with a time. When a component goes Down, the client can offer a draft incident. The hub must never publish an incident that the operator did not write or confirm.
- **OBS-016.** Publish. The operator can download the bundle at any time. The hub can also publish it on a schedule (default each 5 minutes and at each state change of a component) to one publish target that the operator connects. The connection is outbound only. The first target is a branch of a GitHub repository through the GitHub App of Projects (PRJ-017). [NEEDS CLARIFICATION: OD-OBS8 decides the path; the target is EXT-000 scope, so it is the one that the operator uses.]
- **OBS-017.** A failed publish keeps the last good bundle at the target, retries with the schedule of NOT-001 (1, 5, 15 and 60 minutes), and writes an event. After 60 minutes of failures, it sends one notification.
- **OBS-018.** A status page has a history of incidents for 90 days in the bundle.
- **OBS-019.** The export and the publish are in the audit log (LOG-012 to LOG-020), with the actor kind.

### 6.3 Error tracking

- **OBS-020.** An error event has: the device, the program (a container, a systemd unit, a check, or an application that reports with an SDK), the level, the message, an optional stack, the time, and the release if the program gives one.
- **OBS-021.** The hub groups events into error groups by a fingerprint: the program, the error type, and the message with numbers, identifiers, hex strings, paths and times replaced by placeholders, plus the top frame of the stack when there is one. A group shows the first time, the last time, the count and the devices.
- **OBS-022.** Sources in the first version (the narrow tracker): (a) the lines of a container log at level error or fatal, read on the path of HOST-001; (b) a restart loop (CHK-014); (c) a systemd unit that enters the failed state [NEEDS CLARIFICATION: the daemon has no journal reader; task OB0 measures it]; (d) a watch that fails (WDG-035), only after the watchdog is built.
- **OBS-023.** Events from an SDK (OD-OBS10): if permitted, the daemon accepts the envelope format of the Sentry SDKs on the loopback address only, for each project that the operator adds, with a key for each project. The daemon forwards the events to the hub on the existing daemon push path (K8). The hub never accepts SDK events directly. [NEEDS CLARIFICATION: OD-OBS10, and the subset of the envelope format (task OB0).]
- **OBS-024.** The daemon redacts each event before it sends it (C4). It never forwards request bodies, cookies, or headers other than an allow list (method, path without query, status, user agent class).
- **OBS-025.** A size limit: one event is at most 64 KiB after redaction; the daemon drops a larger event and counts it. Each program has at most 100 events in each minute; the daemon counts the rest and sends the count. [NEEDS CLARIFICATION: the limits are recommendations.]
- **OBS-026.** The hub sends a notification (NOT-003 channels) for a new group and for a resolved group that occurs again. It sends at most one notification for each group in each hour.
- **OBS-027.** A group has the state Unresolved, Resolved or Ignored. The operator can mute a group for a time.
- **OBS-028.** A group links to the device, to the log lines around the first and the last event (HOST-001, HOST-015), and to the check, if one exists.
- **OBS-029.** Retention: events 30 days (as raw heartbeats, DB-009); groups 1 year after the last event. [NEEDS CLARIFICATION: retention.]
- **OBS-030.** Each program has at most 1,000 open groups. Above that, new fingerprints go to one group "Too many distinct errors" for the program, and the client says so.
- **OBS-031.** An error group is not a check. It does not change the state of a check or the uptime.
- **OBS-032.** The "fix with an agent" path (start a coding agent from a down check or an error group) is out of scope. It is a separate idea with no brief.

### 6.4 Product analytics (only if OD-OBS6 is yes)

- **OBS-040.** If OD-OBS6 is no: product analytics stays the hourly read of PostHog (PRJ-009, PRJ-012, PRJ-015, OD-E6). This proposal adds nothing for it, and OBS-041 to OBS-044 are withdrawn.
- **OBS-041.** If OD-OBS6 is yes: the hub stores product events (name, time, an anonymous person key, properties after redaction) for each project that the operator adds.
- **OBS-042.** Events arrive only through the public path that OD-OBS10 permits. The hub has no public ingest (C1).
- **OBS-043.** The page shows events, unique persons, a funnel of up to 5 steps and the retention of a week. The Projects card (PRJ-012) reads the 7-day users from this store when PostHog is not linked.
- **OBS-044.** No person profile, no session recording and no feature flags in the first version.

### 6.5 Web analytics (only if OD-OBS7 permits a collector)

- **OBS-050.** `www.anima.ing` and `docs.anima.ing` keep Vercel Web Analytics and nothing else (SITE-008, OD-21, R-207). OD-OBS7 asks if this changes.
- **OBS-051.** If OD-OBS7 permits: a collector for the operator's other websites counts page views. It stores the path, the referrer domain, the country, the device class and the browser family. It never stores an IP address or a full user agent, and sets no cookie.
- **OBS-052.** The site loads the script from its own origin. The script sends to the public program of OD-OBS10 (c), never to the hub.
- **OBS-053.** A unique visitor is a daily hash of a salt that changes each day, the site and the request data; the salt is never stored after its day.
- **OBS-054.** The page shows page views and unique visitors for 7, 30 and 90 days and the top pages, as PRJ-009 does.
- **OBS-055.** A site with Vercel Web Analytics and the collector shows one source only; the operator picks it (PRJ-015 rule for empty sections).
- **OBS-056.** The privacy page of the docs (UDOC-040 to UDOC-044) lists the collector if it is on.

### 6.6 Interfaces (CMD-030 order)

1. Hub GraphQL: `statusPages`, `statusPage(id)`, `createStatusPage`, `updateStatusPage`, `exportStatusPage`, `writeIncident`; `errorGroups(filter)`, `errorGroup(id)`, `setErrorGroupState`.
2. Command line: `anima status-page list|show|export|publish`, `anima incident write|update|resolve`, `anima errors list|show|resolve|ignore|mute`.
3. MCP: read tools `anima_status_pages_list` and `anima_errors_list`. A change tool (publish, resolve) follows the MCP permissions (MCP-014 to MCP-025); each tool has its JSON-RPC test (MCP-017). No MCP tool writes an incident.
4. Client: the Status pages screen and the Errors page (the mockup in Section 8.2).

### 6.7 Out of scope

Short links (Dub; OD-OBS2), a separate single binary, session recording, feature flags, source maps and release health of the Sentry product, a status page served by the hub, the "fix with an agent" idea, alerts by phone call, and on-call schedules.

## 7. Changes to other documents (at approval, not now)

| Document | Change |
| --- | --- |
| extensions-plan.md | New sections in Section 4 for each approved extension, with OBS requirements moved in; the table of EXT-010; XP-005 (OD-OBS12); EXT-009 and Section 1.1 for the count (OD-OBS11); OD-E6 if OD-OBS6 is yes; WHK if OD-OBS8 (b) or OD-OBS10 (c) |
| design-spec.md | INT-004 replaced by a pointer; SITE-006 if a public program is permitted |
| contracts/extensions/registry.md (K-E7) | New entries; "The fifteen entries" |
| contracts/daemon-push.md (K8) | A new message for error events (OBS-023), if OD-OBS10 permits SDK events |
| contracts/schema.graphql | The types of Section 6.6 |
| priorities.md | Row 4.8 text, or a new row (OD-OBS5) |
| ownership.md | The row of this proposal is replaced by the owners of Section 3 |

## 8. Reviews

### 8.1 Scope review

- **Narrowest useful version.** One status page, exported as static files, from the checks that exist (OB3 to OB6). No new program, no port. Then the narrow error tracker from container logs and restart loops (OB9, OB10, OB12).
- **Effort (estimate, not verified).** Status pages M (export) or L (with a public program). Narrow error tracking M. SDK ingest on the daemon M. Product analytics L. Web analytics M, plus the public program. All four: L+, as Anima Product Ideas estimated.
- **What is in, with the relayed scope.** All four pieces, with analytics behind OD-OBS6 and OD-OBS7.
- **What the Planner recommends.** Status pages and the narrow error tracker. Keep analytics as reads of PostHog and Vercel (OD-E6). Reason: analytics needs a public ingest and reverses a decision, and PRJ-009 already gives the numbers.
- **Stop condition (from the idea).** Stop if a public status page needs the hub on the public internet, or if the first build needs a new program next to `anima-daemon` and the hub.
- **Risk.** Row 4.8 is behind row 4.4 and R-144. Under row 4.8, no task starts before WP-53.

### 8.2 Design scores (0 to 10)

| Dimension | Score | What makes it a 10 |
| --- | --- | --- |
| Clarity for a visitor of the status page | 8 | A test with three people who say the state of each component in under 10 seconds |
| Honesty (No data, stale page) | 8 | A test that makes the hub sleep and shows that the page never says Operational for that time |
| Privacy of the operator (no address on a public page) | 7 | A gate that also checks the incident text for addresses |
| Signal over noise in the errors list | 6 | Measured on a week of the operator's real logs: fewer than 20 new groups each day |
| Same look as the product (tokens) | 8 | The mockup compared with the built screens (PLN-019) |
| Accessibility (contrast, not color only) | 8 | Each state has a word and a shape as well as a color; a screen reader audit |
| Control (nothing public without the operator) | 9 | Each publish and incident is in the audit log |

The mockup is [mockup/observability-suite/index.html](../mockup/observability-suite/index.html): frame 1 is the public status page (light), frame 2 is the Status pages screen in the client (dark), frame 3 is the Errors page (dark). The values come from `design/tokens` (Graphite palette, status roles).

### 8.3 Engineering review

**Types.** Hub (Rust): `StatusPage { id, title, text, components: Vec<Component>, target: Option<PublishTarget> }`, `Component { check_id, display_name: DisplayName }` where `DisplayName` refuses an address at construction (OBS-012), `Incident { id, page_id, title, components, updates: Vec<IncidentUpdate>, state: IncidentState }`, `ExportBundle { files, made_at }`; `ErrorEvent`, `ErrorGroup { fingerprint: Fingerprint, state: GroupState, first, last, count }`, `Fingerprint` (a pure function, golden tested). Daemon (`daemon-core`): `SdkEnvelope` (parse only the permitted items), `ProjectKey`.

**Contracts.** K-E7 (registry) gets new entries. K8 (daemon push) gets one message kind for error events, only if OD-OBS10 permits SDK events; the narrow sources (container logs, restart loops) use hub reads that exist. `contracts/schema.graphql` gets the types of Section 6.6. Each is a frozen contract change and needs the operator's approval (task OB2).

**Failure modes.** The hub sleeps (C6): the page says No data and goes stale (OBS-014). The publish target refuses: the last good bundle stays (OBS-017). An error storm: the limits of OBS-025 and OBS-030. A redaction miss: a secret is stored; the security review OB15 tests the redaction with the fixtures of the crate. A fingerprint that is too fine: many groups; OBS-030 caps them. A clock that is wrong on a device: the event time is the time of the daemon; the hub also keeps the receive time.

**Trust boundaries.** (1) A visitor and the static bundle: no contact with Anima.ing. (2) A local program and the daemon loopback ingest: any local process can post; the project key limits it to known projects, and the size and rate limits protect the daemon. (3) The daemon and the hub: the existing authenticated push (K8). (4) The hub and the publish target: outbound only; the token is in the credentials domain (SRC-008). (5) If permitted, a public program on the VPS: the WHK-009 model (own user, no database access, a queue that the hub reads).

**Tests.** Unit: the fingerprint (golden), `DisplayName` refusals, the stale rule, the bars from aggregates with maintenance. Integration: the export has no external origin (parse the HTML), no request to the hub, and the badges; the publish retries; the daemon ingest refuses non-loopback, oversized and unknown keys. End to end: a down check gives a red bar and a draft incident.

**CMD-030 order.** Hub API (OB4, OB9), then the command line (OB6, OB13), then MCP (in the same tasks, read tools), then the client (OB7, OB14).

### 8.4 Blast radius on shared contracts and rules

| Shared item | Effect |
| --- | --- |
| `REGISTRY: [Entry; 15]` and K-E7 "The fifteen entries" | The count changes, or the new switches stay outside the 15 as "Agent tools" did (OD-OBS11) |
| EXT-009 | The count in its text (OD-OBS11) |
| XP-005 | A place in the order (OD-OBS12) |
| K8 daemon push | One new message, only for SDK events |
| SEC-008, NET-003, DNS-005, HMD-011 | No change with the export path; a change with a public program or a hub page (OD-OBS8) |
| SITE-006 to SITE-008, OD-21 | No change unless OD-OBS7 or OD-OBS8 permits it |
| OD-E6 | Reversed only if OD-OBS6 is yes |
| NOT channels | New notification kinds (OBS-017, OBS-026) on the existing channels |

## 9. Decisions

Each row asks one question with yes, no or one choice, and gives the recommendation of the Planner (PLN-018, PLN-021). The relayed answers of Section 2 are the working direction; each row needs the operator's first-hand answer.

| Number | Question | Recommendation, with strengths and weaknesses | State |
| --- | --- | --- | --- |
| OD-OBS1 | Which pieces are in scope? (one choice: all four, including product and web analytics, as relayed; status pages and error tracking; status pages only) | Status pages and error tracking. Strength: both build on data Anima.ing has, with no public ingest. Weakness: it differs from the relayed "all of them". | Open |
| OD-OBS2 | Are short links (Dub, named in the post) out of scope? (yes/no) | Yes. Strength: no fit with a device manager. Weakness: none known. | Open |
| OD-OBS3 | In what order? (one choice: status pages and error tracking together, as relayed; status pages first) | Status pages first. Strength: the narrowest version ships sooner and tests the export path. Weakness: errors wait about one wave. Section 10 permits both. | Open |
| OD-OBS4 | Do they ship inside the hub and the daemon as extensions that are off by default, with no separate binary? (yes/no) | Yes. Strength: one install, EXT-005 keeps the foundation clean. Weakness: the hub grows. | Open |
| OD-OBS5 | Which priority row? (one choice: under row 4.8, as relayed; a new row 4.23 at tier 3) | A new row 4.23 at tier 3, after row 4.19. Strength: row 4.8 waits for row 4.4 and for WP-53 (R-144); status pages need neither. Weakness: one more row. | Open |
| OD-OBS6 | Do you reverse OD-E6 ("No custom telemetry endpoint") so that Anima.ing stores product analytics events of its own? (yes/no) | No. Strength: PRJ-009 already reads PostHog; no public ingest. Weakness: the operator stays on PostHog. | Open |
| OD-OBS7 | Does SITE-008 and OD-21 change, so that an Anima.ing collector can count page views? (one choice: no change, Vercel Web Analytics stays; a collector for the operator's other sites only; also replace Vercel Web Analytics on www and docs) | No change. Strength: no public collector. Weakness: no own web analytics. | Open |
| OD-OBS8 | How does a status page reach the public? (one choice: static export that the hub publishes outbound to a target; a separate public program on the VPS, as the webhook receiver (OD-E15); the hub serves it) | Static export. Strength: keeps SEC-008, NET-003, DNS-005 and HMD-011 with no change. Weakness: a delay of up to 5 minutes, and a target account. | Open |
| OD-OBS9 | Is the first publish target a branch of a GitHub repository through the Projects GitHub App? (yes/no) | Yes. Strength: an account the operator uses (EXT-000), free static hosting. Weakness: Projects must be on, or a second token. | Open |
| OD-OBS10 | Which events can the error tracker accept? (one choice: only what Anima.ing reads now, container logs and restart loops; also Sentry SDK events on the daemon loopback; also a public ingest on the VPS for websites) | Also SDK events on the loopback. Strength: real app errors with no public port. Weakness: a K8 contract change; websites on Vercel stay out. | Open |
| OD-OBS11 | Are the new extensions outside "the 15 extensions" of EXT-009, as the "Agent tools" switch is (R-196)? (yes/no) | Yes. Strength: community extensions do not wait longer. Weakness: two counts to explain. | Open |
| OD-OBS12 | Where in the order of XP-005? (one choice: right after XP-02 Projects; at the end) | Right after XP-02. Strength: Projects cards show status and errors; low risk data. Weakness: delays the next extension. | Open |
| OD-OBS13 | Does R-144 (PH-E-000 until the acceptance run WP-53) also hold these extensions? (yes/no) | Yes. Strength: one rule for extensions. Weakness: no build before WP-53. | Open |
| OD-OBS14 | Does this stay a separate proposal now, and move into extensions-plan.md and design-spec.md (INT-004) at approval? (yes/no) | Yes. Strength: PLN-004 at approval, one owner each. Weakness: one move later. | Open |

## 10. Tasks

One pull request for each task. The size of each task is a few hours to three days (an estimate). All tasks are Not started. No ruling names a task, so no task starts (PLN-007). Waves follow the relayed "both together": the status page tasks (OB3 to OB8) and the error tasks (OB9 to OB14) can run at the same time.

| Task | Wave | Content | Needs | State | Commits |
| --- | --- | --- | --- | --- | --- |
| OB0 | 0 | A record in `docs/research/`: the subset of the Sentry envelope format; a systemd journal reader on Linux and its macOS equivalent; the size of an export bundle | — | Not started | — |
| OB1 | 1 | The operator decides OD-OBS1 to OD-OBS14 first-hand, then approves OBS-001 to OBS-056 (or names the ones not approved) | The operator | Not started | — |
| OB2 | 1 | Docs only: the changes of Section 7 that the rulings permit; the contract text of K-E7, K8 and the schema | OB1 | Not started | — |
| OB3 | 2 | Hub: the registry entries and switches of the approved extensions; migrations | OB2 | Not started | — |
| OB4 | 2 | Hub: the status pages domain (pages, components, incidents), `DisplayName`, the GraphQL | OB3 | Not started | — |
| OB5 | 3 | Hub: the export bundle (HTML, JSON, SVG badges), the stale rule, the address gate | OB4 | Not started | — |
| OB6 | 3 | `anima status-page`, `anima incident`; MCP `anima_status_pages_list` | OB4 | Not started | — |
| OB7 | 4 | Client: the Status pages screen; the mockup frames 1 and 2 compared | OB5, OB6 | Not started | — |
| OB8 | 4 | Hub: the scheduled publish to the target of OD-OBS9, retries, notification | OB5 | Not started | — |
| OB9 | 2 | Hub: the errors domain (events, groups, fingerprint, retention, caps), the GraphQL | OB3 | Not started | — |
| OB10 | 3 | Hub: the sources of OBS-022 (a), (b) | OB9 | Not started | — |
| OB11 | 3 | Daemon: the loopback SDK ingest, redaction, limits; the K8 message (security) | OB9; OB0 | Not started | — |
| OB12 | 4 | Hub: the notifications for new and recurring groups | OB9 | Not started | — |
| OB13 | 4 | `anima errors`; MCP `anima_errors_list` | OB9 | Not started | — |
| OB14 | 5 | Client: the Errors page; the mockup frame 3 compared | OB13 | Not started | — |
| OB15 | 6 | The security review of the export gate, the loopback ingest and the redaction; entries in `need-human-testing.md` | OB8, OB11, OB14 | Not started | — |
| OB16 | — | Product analytics and web analytics: no task until OD-OBS6 or OD-OBS7 is yes; then the Planner writes the tasks | OD-OBS6, OD-OBS7 | Not started | — |
| OB17 | 7 | The human check: a real outage shows on the published page; a real container error makes a group and one notification | OB15 | Not started | — |

### 10.1 Acceptance, checks and scope of each task

| Task | Pass or fail | Check commands | Out of scope |
| --- | --- | --- | --- |
| OB0 | Pass when the record names the envelope items to accept, the journal method on each platform and a measured bundle size; fail otherwise | `bun run quality:documents` | Code |
| OB1 | Pass when a ruling records each answer first-hand; fail otherwise | `bun run quality:register`, `bun run quality:documents` | A build |
| OB2 | Pass when the changes of Section 7 are made and the gate passes; fail otherwise | `bun run quality:register`, `bun run quality:documents` | Code |
| OB3 | Pass when each new switch is off by default, the foundation tests pass with all off, and the registry test lists the new entries; fail otherwise | `cargo nextest run -p hub engine::extensions`, `bun run gates` | Pages |
| OB4 | Pass when `DisplayName` refuses an address, a host name and a URL (tests), and the GraphQL works; fail otherwise | `cargo nextest run -p hub status_pages` | Export |
| OB5 | Pass when the bundle has no external origin and no hub address (parsed test), the bars match the aggregates with a maintenance window, and a stale bundle shows "Status not current"; fail otherwise | `cargo nextest run -p hub status_pages` | Publish |
| OB6 | Pass when the commands follow the output rules and the MCP tool is read-only; fail otherwise | `cargo nextest run -p anima-cli` | Client |
| OB7 | Pass when the screen matches the mockup at 1040 px (light and dark) and the mockup row names the commit; fail otherwise | `bun run --cwd apps/client test`, `bun run gates` | Errors page |
| OB8 | Pass when a publish writes the bundle to the target, a refused publish keeps the last bundle and retries, and 60 minutes of failures send one notification; fail otherwise | `cargo nextest run -p hub status_pages` | Other targets |
| OB9 | Pass when the fingerprint golden tests pass, the caps of OBS-030 hold, and retention deletes old events; fail otherwise | `cargo nextest run -p hub errors` | Sources |
| OB10 | Pass when a fixture container log and a restart loop make the expected groups; fail otherwise | `cargo nextest run -p hub errors` | SDK events |
| OB11 | Pass when the ingest refuses a non-loopback address, an unknown key, a body above 64 KiB and the 101st event in a minute, and a fixture secret is redacted; fail otherwise | `cargo nextest run -p daemon`, `cargo nextest run -p daemon-core` | A public ingest |
| OB12 | Pass when a new group and a recurring group each send one notification in an hour; fail otherwise | `cargo nextest run -p hub notifications` | New channels |
| OB13 | Pass when the commands follow the output rules and the MCP tool is read-only; fail otherwise | `cargo nextest run -p anima-cli` | Client |
| OB14 | Pass when the page matches the mockup frame 3 and the mockup row names the commit; fail otherwise | `bun run --cwd apps/client test`, `bun run gates` | — |
| OB15 | Pass when the review has no open finding of severity high and the `HT-nnn` entries exist; fail otherwise | `bun run quality:documents` | Fixes outside the findings |
| OB17 | Pass when the operator sets the entries to Passed; fail otherwise | — | — |

Rules for each task:

- Always: work in a worktree from `origin/dev` and merge to `dev` (AGENTS.md); claim the task (PLN-022); run `bun run quality:register` and `bun run quality:documents` before each commit; keep CMD-030 order; write "Not verified" for a step that you did not run.
- Ask first: a frozen contract change (K-E7, K8, the schema); a new outbound connection or publish target; a change to `anima-site`; a public port anywhere.
- Never: a port on the hub or a public route to it (SEC-008, HMD-011); an address, a host name or a device name on a public page; a stored secret (LOG-007); a script from a third party in a bundle; an incident published without the operator; a fact of the business vault (a price, a cost, a funding fact or a trademark finding).

Prompt for each task: "Follow AGENTS.md 'Start of a session'. Claim and build task OBn of docs/proposals/observability-suite.md. Read Sections 5, 6 and 8 first. Build only what the row of OBn says. Run the check commands of the row, then bun run quality:documents and bun run gates."

## 11. Not verified

- The effort of each piece (Section 8.1). Estimates only.
- That a systemd journal reader and a macOS log reader fit the daemon (OBS-022 (c)); OB0 measures it.
- The subset of the Sentry envelope format that the SDKs need (OBS-023); OB0.
- That the GitHub App of Projects can write to a branch for publish with its current permissions (OBS-016).
- That the migration ranges 3600 to 3999 are free in every open branch (OBS-005).
- The limits of OBS-014, OBS-025, OBS-029 and OBS-030. Recommendations, not measurements.
- That each of CHK-001 to CHK-014 is built. The check types are in the code; each requirement was not traced.
- What the Coolify product will be. The post is a teaser, and the replies were not read.

## 12. Review notes

- 2026-11-19: version 1. The Planner made the proposal from the idea `observability-suite` (`5c9fc563`). The idea folder is deleted in this commit. The four relayed answers are written as OD-OBS1, OD-OBS3, OD-OBS4 and OD-OBS5 for first-hand rulings. No ruling is recorded.
