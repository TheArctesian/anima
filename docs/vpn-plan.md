# Anima VPN Plan

| Item | Value |
| --- | --- |
| Document | VPN plan: status, key expiry, policy checks and enrolment |
| Version | 0.4 (build decisions BD-1 to BD-17 folded in; [vpn-build.md](vpn-build.md) wins where they differ) |
| Date | 2026-11-10 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Accepted by the operator, 2026-11-10 (Section 9) |
| Base | Branch `phase-3` at `b4f7051d`. All paths in this plan are on that branch. |
| Related | [design-spec.md](design-spec.md) Sections 4.1, 4.2, 6.7, 7, 13; `docs/setup/vpn.md` Sections 5, 6, 7, 8, 11, 13; `docs/programme/proposals/in-app-enrolment.md`, `add-this-machine.md` |

This document obeys the writing rules in Section 0 of the design specification.

Version 0.4 folds in the decisions of vpn-build.md Section 1 (BD-1 to BD-17). Where a passage below
contradicts a BD row, the passage now points to that row. The requirement text of VPN-008 to
VPN-014 is in design-spec.md Section 4.2 and Section 7 (SET-003); it is not repeated here.

Version 0.2 applies an adversarial review of version 0.1 against the code. The main changes:
VPN-011 no longer claims to detect nodes that the hub cannot see; the path rules follow the
real meaning of the Tailscale fields; key warnings are stored so that each one is written
once; the reader starts from `upkeep.rs` (R-072); the hub does no ping of its own; the broker
has its own controls, because hub-side controls do not stop a hub that an attacker controls;
and pre-auth keys stay out of command lines.

## 1. Purpose and scope

### 1.1 Purpose

This plan puts ideas from the 11 Headscale web interfaces on the Headscale list
(`headscale.net/stable/ref/integration/web-ui/`) into Anima. For each idea it gives the
location in the code, the existing code that it uses, and the rulings that it needs.

The plan has two tiers:

- **Tier A (read-only).** The hub reads the state of its own VPN node. Deploy tools check the
  policy and the node list. Nothing changes the VPN. Tier A needs no change to
  `docs/setup/vpn.md` Section 11.
- **Tier B (changes).** In-app enrolment, renewal, route approval and device paths. Tier B
  needs the VPS broker of R-055, and operator rulings that change vpn.md Section 11, R-079 and
  VPS-005.

### 1.2 What the research showed

| Idea | Source | Tier |
| --- | --- | --- |
| A status node that opens no port and reads its peers | Headplane agent | A |
| Each subnet route is advertised, approved or serving | unraid-headscale-admin, Ouroboros, headscale-admin | A (serving), B (advertised) |
| Key expiry: tagged nodes never expire, client countdown | headscale-admin, HeadControl | A |
| Compare the tailnet with the expected list of nodes | Headscale Manager, vpn.md Section 11 | A (tooling, not the hub) |
| A reachability table for the policy | Headscale Manager | A |
| `headscale policy check` as a dry run | MunMunMiao Headscale UI | A |
| Pin one Headscale version, contract tests against it | all; Headplane, MunMunMiao, headscale-piying | A |
| Make the `tailscale up` command for a site | headscale-admin | B |
| Type the registration ID, the hub registers the node | Ouroboros | B |
| Approve a router's route in the app | headscale-admin, Headplane | B |

### 1.3 Ideas that Anima does not take

| Idea | Reason |
| --- | --- |
| A Headscale API key in the client, the browser or the hub | vpn.md Section 11: no API keys. CLI-004. |
| A user interface that writes the policy or restarts Headscale | VPS-005, R-079. The policy is code with tests (VPN-004). |
| Public `/register/*` paths and CORS changes on Headscale | VPS-002, VPS-003 |
| A Tailscale client in the browser for remote access | All traffic goes through DERP. Credentials go to the browser (CLI-004). Only Tailscale SSH. guacd stays (spec Section 3.2). |
| Encrypted token storage in IndexedDB for the web client | D-3 already settles this: an `HttpOnly` cookie, which scripts cannot read, is better. |
| Users, roles and per-user policy "bubbles" | One operator (spec 1.4). VPN-004 is by role, not by user. |
| Support for more than one Headscale version | headscale-admin stopped this. Anima pins one version (vpn.md 13.1). |

## 2. Rulings and constraints

| ID | Source | Effect on this plan |
| --- | --- | --- |
| OD-10, vpn.md Section 11 | `docs/setup/vpn.md` | No Headscale API key. Admin only through `headscale` on the VPS unix socket. Tier A does not call Headscale. |
| vpn.md Section 11, steps 3 and 5 | `docs/setup/vpn.md` | Pre-auth keys are single use, last one hour, and stay out of command lines and logs. Use `--auth-key=file:` (vpn.md lines 486, 916). |
| R-055 | `docs/programme/programme.md` | The broker is deferred. The proposal has exactly three verbs. Tier B waits for decision (b). |
| R-061 | `programme.md`, `add-this-machine.md` | Device grants: one `/32`, from `tag:hub`, TCP only. Router test rows wait in `ROUTER_ROWS`. |
| R-076 | `programme.md` | No grant on TCP 3389. This overrides the `3389` in `add-this-machine.md`. |
| R-079 | `programme.md` | The app never changes VPN rules. Tier B item B5 needs a change to R-079. |
| VPS-005, R-024 | spec 4.2, `operator-changes.md` | Each live VPS or Headscale change needs operator confirmation in chat and a row in `operator-changes.md`. Key renewal is already "Operator change 14, recurring (confirm in chat first: VPS-005)" (vpn.md Section 7). |
| D-4 (Phase 2), R-065 | `phase-2-plan.md` | The LocalAPI proxy allows only `GET /localapi/v0/status[?peers=]` and `POST /localapi/v0/ping`. WP-42 owns `deploy/localapi/allow.ts` (Q2-15). Tier A needs no change to it. |
| WP-23 H-5, LOG-007 | WP-23 report | The hub must not log raw status output. It contains peer names and logins. |
| R-072 | `programme.md` | `main.rs` is at its limit. Periodic jobs go in `upkeep.rs` (a `wiring` module), not in `main.rs`. |
| R-082 | `phase-3-plan.md` | Changes to `main.rs` and `schema.rs` are hand-off lines to the programme orchestrator. |
| SET-002 | spec Section 7 | The Settings page shows only the listed rows. A VPN section needs a SET delta. |
| K1, K2 | `contracts/schema.graphql`, `contracts/database.md` | Frozen for Phase 3 (CON-001). New fields, event sources and migration ranges need a programme ruling. `0940`–`0999` is reserved for Phase 4. |
| `quality/hub-modules.json` | imports gate | A new domain and each new import is a contract change. |
| PKG-001, R-082 | `phase-3-plan.md` Section 1 | Only WP-54 hand-offs and WP-52 (for D-8, D-24) change `deploy/headscale/`. Tier A items A3 to A5 need a ruling. |
| Limits, `quality/baseline.json` is `{}` | AGENTS.md | 300 code lines for each file, 12 files for each folder. See Section 6. |
| Programme location | `docs/programme/handoff.md` | The Linux-machine session runs Phase 3. New packages go to issue #1 (R-007) until the programme accepts them. |

## 3. Facts from the code

These facts set the design. Items 1 to 3 and 6 to 11 were read on `phase-3`. Items 4 and 5 are
from the Tailscale source (`ipnstate.go`); their behaviour must be checked in A1-T1.

1. The hub calls the LocalAPI proxy only from `crates/hub/src/checks/probe/tailscale.rs`
   (`tailscale_ping`, CHK-009). It reads only `Err` and `LatencySeconds`. Nothing reads
   `status`. Nothing implements VPN-005.
2. The proxy already allows `GET /localapi/v0/status?peers=true` (`deploy/localapi/allow.ts`).
   The hub already has its address: `Config.tailscale_localapi` (`ANIMA_TAILSCALE_LOCALAPI`).
   The proxy writes one log line for each request (`server.ts`).
3. The proxy reads the tailscaled of the hub's own VPN node. Headscale makes two nodes peers
   only when a grant permits traffic between them in one direction or both. Under the current
   `policy.hujson`, the hub's peers are: `operator@` clients (grant to `tag:hub`), routers whose
   site route is approved (grant from `tag:hub` to the site subnet), and device `/32` hosts.
   **A node of another user, a node with a tag that no grant names, and a router with no
   approved route are not peers. The hub cannot see them.**
4. A peer in `status` has, among other fields: `HostName`, `DNSName`, `OS`, `TailscaleIPs`,
   `Tags`, `PrimaryRoutes`, `Online`, `LastSeen`, `Active`, `CurAddr`, `Relay`, `PeerRelay` and
   `KeyExpiry`. The meanings that matter:
   - `CurAddr` is the direct endpoint. It is empty when traffic is relayed.
   - `Relay` is the peer's home DERP region. It is set also when the path is direct.
   - `PeerRelay` is set when traffic goes through a peer relay, not DERP.
   - `Active` is true only while there is recent traffic. Without traffic, the path is not known.
   - `LastSeen` is present only when the peer is offline.
   - `KeyExpiry` is absent (or zero) for a key that does not expire.
   - `HostName` and `OS` come from the peer. They are not trusted text.
   - The status has no Tailscale version and no advertised routes that are not approved.
5. A disco ping returns the first reply. The first reply often comes through DERP, also when a
   direct path will follow. One ping is not a measure of the path.
6. `Site { id, name, subnet }` is in `crates/hub/src/devices/model.rs`. A device has `ip` only
   when its host is an IP literal (around line 151). A device with a DNS-name host has no `ip`.
7. `upkeep.rs` starts the periodic jobs. `events::run_daily` runs each job at start and then
   each day, so a job that writes an event writes it again after each restart.
8. The policy tooling is in `deploy/headscale/src/policy.ts` (parser) and `tailnet.ts` (pure
   evaluator, `allows`, `runPolicyTests`). The test inventory is written by hand in
   `deploy/headscale/tests/policy.test.ts`. No tool reads a live inventory. No tool runs
   `headscale policy check`; the operator runs it by hand (vpn.md Sections 8 and 13.1). No
   harness starts a Headscale container.
9. Headscale is `0.29.7`, pinned by tag and digest in `deploy/headscale/compose.yaml` and by
   `tests/server.test.ts`. `policy.mode` is `file`.
10. No subnet router is enrolled. `operator-changes.md` has no router row, and `ROUTER_ROWS`
    still waits in `policy.test.ts`. A capture from the live hub cannot show `PrimaryRoutes`.
11. TOTP is asked only at sign-in. The `access` domain has no second TOTP step for one
    mutation. WP-43 owns `access/`.
12. The client has no VPN interface except the NET-004 banner (`$lib/nav/`), the site picker in
    Add device (`$lib/devices/form/site.ts`) and the Tailscale ping check form. The VPN-002 site
    filter is not built. The schema already has `DeviceFilter.siteId`.

## 4. Tier A: read-only

### 4.1 A1. Hub domain `vpn`

A new hub domain reads the status of the hub's VPN node, keeps the latest picture in memory, and
publishes changes. It stores only the marks for warnings that it has written.

**New requirements (spec delta, Section 4.2).** VPN-008 to VPN-014 are free; the highest
existing ID is VPN-007.

VPN-008 to VPN-011 are in design-spec.md Section 4.2. They include BD-5 (role from tags), BD-6
(a router serves a site when `PrimaryRoutes` holds the subnet exactly), BD-13 (the last known
path is kept for 3 reads) and BD-14 (a node matches a device by address or `DNSName`, never by
`HostName`).

The hub writes no event when a router goes offline. The Tailscale ping check (CHK-009) already
does this, and its events go to notifications.

**Files.** The domain is a face file and a folder. The gate does not count a module face that has
a folder (`quality/src/limits.ts`).

```text
crates/hub/src/
├── vpn.rs                 Face: spawn_reader, latest, Feed, Error (thiserror)
└── vpn/
    ├── model.rs           VpnPicture, VpnNode, SitePath, Path {Direct, PeerRelay, Relayed{region}, Unknown},
    │                      KeyState {Never, Valid, Expiring, Expired, TaggedWithExpiry}
    ├── peers.rs           Serde types for the status subset; parse; limit untrusted text. Pure.
    ├── picture.rs         Pure: (status, sites, devices, now) -> VpnPicture. Matches PrimaryRoutes to
    │                      Site.subnet. Matches a peer to a device by TailscaleIPs = Device.ip, or by
    │                      DNSName = Device.host (case-insensitive; BD-14, never HostName). Applies VPN-009 to VPN-011.
    ├── warnings.rs        Pure: (picture, stored marks, now) -> warnings to write (VPN-010, VPN-011).
    ├── store.rs           The only SQL: the warning marks table.
    ├── localapi.rs        I/O edge: GET status only. reqwest, no proxy, no redirects, 1 MiB cap.
    ├── reader.rs          The loop: read, build, publish, write new warnings and their marks.
    ├── feed.rs            tokio watch channel of Option<Arc<Picture>> (BD-2).
    ├── graphql.rs         VpnQuery, VpnSubscription.
    ├── tests.rs
    └── tests/             Fixtures from the A4 harness (invented names and addresses), picture, warning
                           and store tests, a fake proxy as checks/tests/tailscale/proxy.rs.
```

The hub does not send pings from `vpn`. The ping stays in `checks` (CHK-009). Thus `checks` does
not import `vpn`, and `vpn` has one LocalAPI call. The two clients share no code; each is about
15 lines and they call different paths.

**Database (K2 delta).** One table in a new schema, numbered `1000–1019` (BD-1; `1000_events_source_vpn.sql`,
`1001_vpn_schema.sql`):

```sql
create schema vpn;
create table vpn.warnings (
  node_key   text        not null,  -- the node's public key, not a secret
  kind       text        not null check (kind in ('expiry_30','expiry_14','expiry_7','expiry_1',
                                                    'tagged_expiry','not_in_anima')),
  key_expiry timestamptz,           -- a renewal gives a new expiry, so the marks start again
  written_at timestamptz not null,
  unique nulls not distinct (node_key, kind, key_expiry)  -- BD-4: no primary key
);
```

**How it uses existing code.**

| Existing code | Use |
| --- | --- |
| `config.rs` `tailscale_localapi` | The proxy address. No new setting. The interval is a constant. |
| `devices` face (`get`, `list`) and the site list | Input to `picture.rs`. `vpn` imports only the `devices` face, never its store. |
| `events::record` | Warning events, as `checks/scheduler/notices.rs`. |
| `metrics/feed.rs` | The pattern for `vpn/feed.rs`. |
| `upkeep.rs` | Starts the reader (R-072). The reader runs each 60 s; it is not a `run_daily` job. |
| `targets::AddressPolicy` | Not used for the proxy address (operator infrastructure, as today). |

**Wiring.**

| File | Change |
| --- | --- |
| `crates/hub/src/lib.rs` | `pub mod vpn;` |
| `crates/hub/src/upkeep.rs` | Start the reader with the proxy address, the pool, the bus and the feed. |
| `crates/hub/src/main.rs` | BD-3: move the start of the four background loops into `upkeep::spawn` first, then pass `config.tailscale_localapi` and the feed. Hand-off lines (R-082). |
| `crates/hub/src/schema.rs` | `VpnQuery`, `VpnSubscription` in the tuples; a `Dependencies` field for the feed; `.data(feed)`. Hand-off lines (R-082). |
| `quality/hub-modules.json` | `"vpn": ["devices", "events", "checks"]`; `vpn` graphql: `["access"]` (`checks` is a domain import for `hasPingCheck` and `vpn::Error`); `upkeep` may import `vpn`. |
| `contracts/schema.graphql` (K1) | New types with `@pending` first; the `EventSource` value `VPN`. |
| `crates/hub/src/events/model.rs` + migration | The `vpn` event `Source`. The migration drops and adds the CHECK with the full list, as `0701_events_source_metrics.sql`, and is numbered `1000`, after 0852 (BD-1). |
| `tests/it/hub.rs`, `tests/it/wiring.rs` | Build the new `Dependencies` field. |
| `crates/hub/tests/it/main.rs` | `mod vpn;`, as `tests/it/vpn.rs` with a `tests/it/vpn/` folder, because `tests/it` is at 12 (vpn-build.md Section 2). |
| Notifications (WP-45, WP-47) | Add the `vpn` source to the notification rules when they land. |

**GraphQL (K1 delta).**

```graphql
type VpnPicture { readAt: DateTime!, hub: VpnNode!, sites: [SitePath!]!, nodes: [VpnNode!]! }
type SitePath   { site: Site!, router: VpnNode, path: VpnPath!, relayRegion: String,
                  hasPingCheck: Boolean! }
type VpnNode    { name: String!, address: String!, os: String, tags: [String!]!, online: Boolean!,
                  lastSeen: DateTime, keyExpiry: DateTime, keyState: KeyState!, device: Device,
                  inAnima: Boolean! }
enum VpnPath    { DIRECT PEER_RELAY RELAYED UNKNOWN }
enum KeyState   { NEVER VALID EXPIRING EXPIRED TAGGED_WITH_EXPIRY }
extend type Query        { vpn: VpnPicture }       # null when the proxy is not set
extend type Subscription { vpnChanged: VpnPicture! }
```

`SitePath.site` and `VpnNode.device` resolve through the `devices` loaders.
`SitePath.hasPingCheck` resolves through the `checks` face from the graphql part only. Do not add
fields to `Site` or `Device`, so `devices` does not import `vpn`.

**Load and logs.** At 60 s the proxy writes 1,440 lines each day for the status reads. If this is
too much, ask WP-42 to log successful status reads at debug level only (a change to
`deploy/localapi/server.ts`).

**Rollback.** Unset `ANIMA_TAILSCALE_LOCALAPI` and the reader does not start; `vpn` returns null.
The migration only adds a schema and a CHECK value.

**Tasks.**

| Task | Content | Done when |
| --- | --- | --- |
| A1-T1 | Make fixtures from the A4 harness: one hub-like node, one router with an approved route, one client, one node with an unknown tag. Capture `status` from the hub-like node, idle and with traffic. | The fixtures show the meanings in Section 3, item 4, and that the unknown-tag node is absent. |
| A1-T2 | `peers.rs`, `picture.rs`, `warnings.rs` with tests (pure). | Tests cite VPN-008 to VPN-011. |
| A1-T3 | `store.rs`, migration, `localapi.rs`, `reader.rs`, `feed.rs`, `upkeep.rs` wiring. | The hub starts with and without the proxy. No status text in any log. Each warning is written once across a restart. |
| A1-T4 | `graphql.rs`, K1 delta, integration tests. | Contract tests pass. |
| A1-T5 | A capture from the live hub after the first router enrols (vpn.md Section 5). | The live capture parses. |

### 4.2 A2. Client

A new client domain `$lib/vpn/`. Tier A adds no navigation destination, because spec 5.3 has
only Devices and Settings. The new Settings section needs a SET delta:

- **SET-003.** The Settings page must show a "VPN" section: the path of each site, the nodes
  that are not in Anima, and the key expiry of each node. When the hub has no LocalAPI proxy,
  the section must say that VPN status is off.

```text
apps/client/src/lib/vpn/
├── api.ts               Documents and fragments (codegen, API-008)
├── client.ts            vpnApi(hub): picture(), watch()
├── picture.ts           Pure: path label, key countdown text, sort order, "not in Anima" grouping
├── picture.test.ts
├── VpnRows.svelte       Settings section: composes the parts below
├── sites/               SitePaths.svelte (one row for each site; "Add a ping check" when there is none) + test
├── keys/                NodeKeys.svelte (key expiry list, VPN-010; listRow transitions, press()) + test
└── device/              DeviceVpn.svelte (device Overview facts: address, path, key expiry, OS) + test
```

The sub-folders keep `$lib/vpn` under 12 files.

| Place | Change |
| --- | --- |
| `routes/settings/+page.svelte` | Add `<VpnRows />` after `SignOutRow` (SET-003). |
| `routes/devices/[id]/overview/OverviewPanel.svelte` | Add `<DeviceVpn />` before `MetricsSection` when the device is a VPN node. |
| `$lib/devices/query.ts`, `$lib/devices/toolbar/` | The VPN-002 site filter: a `site` URL parameter, `filterOf` sends `siteId`, a Site menu beside Status. This needs no hub change. |
| `$lib/events/` | The events page filter gets the `vpn` source. |
| `$lib/nav/` | No change. The NET-004 banner stays the signal that the client is off the VPN. |

Rules for the new components: values from tokens only; motion from `$lib/motion` with a
reduced-motion form (spec 5.7.5); each row has an accessible name, and the path is text, not
colour only. The same section works in the web client (WP-44), because all data comes from the
hub. iOS (Phase 5) uses the same components.

**Tasks.** A2-T1 site filter (independent; it can start now). A2-T2 `$lib/vpn` client and the
Settings section, after A1-T4 and the SET-003 ruling. A2-T3 device Overview facts.

### 4.3 A3. Policy and inventory tooling in `deploy/headscale`

These are deploy tools that run on the operator's computer. They need a ruling under PKG-001 and
R-082, because only WP-54 and WP-52 change `deploy/headscale/` in Phase 3.

- **VPN-012.** The policy tests must check the full reachability table, not only pairs. For each
  pair of identity classes (client, hub, router of site X, device at site X, a device `/32`) and
  each protocol, the table must agree with the four rules of VPN-004.
- **VPN-013.** A script must compare a live node list with the expected inventory, and run the
  policy tests against the live nodes. It must report: a node of a user other than `operator`, a
  tag that is not in `tagOwners`, a tagged node with an expiry, a client key that expires after
  more than 180 days, a router with an advertised route that is not approved, and a node that is
  not in the expected list. The operator runs it after each enrolment, renewal and upgrade
  (vpn.md Sections 5, 6, 7, 13.1). **This is the OD-10 detection.**

```text
deploy/headscale/
├── src/reach.ts           Pure: (policy, inventory) -> Reach table. Uses tailnet.ts allows().
├── src/inventory.ts       Pure: parse `headscale nodes list -o json` and `nodes list-routes -o json`
│                          (0.29.7 shapes only) -> TailnetNode[]; compare with the expected inventory.
├── bin/reach.ts           Prints the table as Markdown, to diff before and after a policy change.
├── bin/inventory.ts       Reads the two JSON files; prints each VPN-013 finding.
├── tests/reach.test.ts    VPN-012: the four rules as table properties over all pairs and protocols.
├── tests/inventory.test.ts
└── test/contract.sh       Manual. Not a gate. See A4.
```

`reach.ts` reuses `allows()`; it does not add an evaluator. The hand-written inventory in
`policy.test.ts` moves to a fixture that both tests use.

**Changes to vpn.md.**

- Section 8, step 1: run `bun deploy/headscale/bin/reach.ts` before and after the edit and
  read the diff.
- Section 11, detection: run `hsctl nodes list -o json > nodes.json` and
  `hsctl nodes list-routes -o json > routes.json`, copy both to the Mac, run
  `bun deploy/headscale/bin/inventory.ts nodes.json routes.json`, then delete both files. The
  files hold node names and addresses and no keys. Do not commit them.
- Sections 5, 6, 7: run the inventory check after each enrolment and renewal.

### 4.4 A4. Headscale contract harness

- **VPN-014.** At each Headscale upgrade, a harness must start the pinned Headscale image with
  the repository configuration and policy, and check: `headscale policy check` accepts the
  policy, and the node and route output of the pinned version parses with `inventory.ts`.

`deploy/headscale/test/contract.sh` follows `deploy/test/smoke.sh` and `apprise-smoke.sh`: a
compose project named with `$$`, a `trap` that runs `down -v`, "Needs Docker. Not a gate." It
starts Headscale and four `tailscale/tailscale` containers (hub-like, router, client, unknown
tag). Each joins with a pre-auth key that the harness makes and passes as a file. No key is
printed. The router container advertises a test subnet, and the harness approves it. The harness
also writes the A1-T1 fixtures. vpn.md 13.1 step 1 adds the harness before the upgrade.

### 4.5 A5. Version pin in one place

The Headscale version appears in `compose.yaml`, `tests/server.test.ts` and, after A3, in
`inventory.ts`. Add `deploy/headscale/src/version.ts` with `HEADSCALE_VERSION = "0.29.7"`. The
tests and `inventory.ts` read it, and a test checks that `compose.yaml` uses the same tag. An
upgrade then changes one constant, one digest and the fixtures. Anima does not add capability
flags, because it supports one version.

## 5. Tier B: changes through the broker (after rulings)

Tier B starts only after the operator rules on R-055 (b), on each broker verb, and on changes to
vpn.md Section 11, R-079 and VPS-005. The embedded VPN client of R-055 (a) is a separate decision
and is not in this plan.

### 5.1 Threat model

The controls in the hub (TOTP, the audit log, the D-23 API-key deny list) do not stop an attacker
who controls the hub, because that attacker can call the broker directly. Thus each limit that
must hold also when the hub is lost must be in the broker:

- The broker accepts only its verbs, with the validation in Section 5.2.
- The broker limits the rate: at most 3 pre-auth keys and 3 registrations each 24 hours.
- The broker limits the size of the tailnet: it refuses a new key or registration when the node
  count is at the expected maximum, which the operator sets on the VPS.
- The broker checks the result of each change (Section 5.2) and undoes a wrong result.
- The broker writes its own log on the VPS (journald), which the hub cannot read or change. The
  operator reads it with the inventory check (VPN-013).

### 5.2 The broker

A small program on the VPS. It is the only path from the hub to Headscale. It holds no API key:
it runs `headscale` on the unix socket, as `hsctl` does.

```text
deploy/broker/
├── verbs.ts        Pure: request -> exact argv or a typed refusal. One function for each verb.
├── outcome.ts      Pure: (verb, node list after the change) -> accept or undo.
├── policy-edit.ts  Pure: add or remove one generated device grant in a marked section (B5).
├── limits.ts       Pure: rate and node-count limits.
├── server.ts       I/O: unix socket server, runs argv, returns typed JSON. Logs verb and result only.
├── Dockerfile      Or a systemd socket unit; decide in B0.
└── tests/          verbs, outcome, policy-edit, limits, a fake headscale binary (pattern: vps-host.ts)
```

| Verb | In R-055 | Validation | Effect and check |
| --- | --- | --- | --- |
| `nodes` | no (read only) | none | `nodes list -o json`, `nodes list-routes -o json`. Gives advertised versus approved routes and the true expiry. |
| `personal-key` | yes | limits | One pre-auth key: user `operator`, untagged, single use, 1 hour. When a node uses it, `outcome.ts` checks: no tags, expiry 180 days or less. Else expire and delete the node. |
| `router-key <site>` | yes | `site` is a policy host; limits | One pre-auth key: `tag:router,tag:site-<site>`, single use, 1 hour. `outcome.ts` checks the tags and that the key does not expire. |
| `approve-route <node> <site>` | yes | the node has the site tag, advertises exactly the site subnet, and the subnet is the policy host | `nodes approve-routes` for that subnet only. |
| `register <auth-id>` | no | format of the ID; limits | `auth register --user operator --auth-id <id>`. Then `outcome.ts` does the vpn.md Section 7 step 3 checks (no tags, expiry 180 days or less), else undoes. |
| `open-device <id> <address> <ports>` / `close-device <id>` | no (`add-this-machine.md`) | R-061: `/32` in `100.64.0.0/10` of an `operator` node; TCP 22 or 5900 only (R-076); source `tag:hub` | `policy-edit.ts`, the policy tests rules, `policy check`, install, `kill -HUP`, roll back on refusal. |

B0 must accept or drop each verb that is not in R-055. Recommendation: accept `nodes` (read only)
and `register` (renewal is the most frequent manual step); decide `open-device` with R-079.

The broker returns a pre-auth key once and never logs it. The hub redaction already hides
`authkey` keys (`events/redaction/names.rs`).

### 5.3 Hub and client for Tier B

| Part | Location | Notes |
| --- | --- | --- |
| Broker client | `crates/hub/src/vpn/broker.rs` | I/O edge; a config value `ANIMA_VPN_BROKER` (unix socket path). |
| Mutations | `crates/hub/src/vpn/graphql.rs` | `createVpnInvite`, `createRouterKey(siteId)`, `approveSiteRoute(siteId)`, `registerVpnNode(authId)`. Signed-in operator (sign-in already asks for the authenticator code, VP-9); each is on the D-23 API-key deny list; each writes an audit entry through `events/trail.rs` (LOG-005) with a new `AuditTarget::VpnNode`. These are second controls; the broker controls in Section 5.1 are the first. |
| Picture | `vpn/picture.rs` | Merge the broker `nodes` result: an advertised route that is not approved shows as "waiting for approval". |
| Enrolment | `apps/client/src/lib/vpn/enrol/` | A modal on `$lib/forms/modal/Modal.svelte`. For a Linux router it shows the key once, with the vpn.md Section 5 steps: put the key in a file that only root can read, then `tailscale up --login-server https://hs.anima.ing --auth-key=file:/run/anima-preauth --advertise-routes=<subnet> …`, then delete the file. The key is never in a command line. Then the modal waits on `vpnChanged` for the node and asks "Approve route <subnet>?". |
| Client computers | same modal | A Mac uses the interactive login (vpn.md Section 6B): the operator types the auth ID from the new machine's screen into the app, and the hub calls `registerVpnNode`. Then Add device opens with the address filled in (add-this-machine). |
| Renewal | `$lib/vpn/keys/` | "Renew" on a client key. A client with a key that has expired is off the VPN, so it cannot approve itself: the operator uses the app on another client (vpn.md Section 7, the expiries are staggered). The app says this. |
| Agent | WP-54 | After the node joins, the same modal can show the `anima-agent` enrolment code (MET-012). |

### 5.4 Tier B tasks

| Task | Content |
| --- | --- |
| B0 | Spec delta for the rulings in Section 9: R-055 (b), the verbs (VP-7), vpn.md Section 11, the VPS-005 and R-024 change (VP-4). R-079 changes only if B6 is built. |
| B1 | Broker with the `nodes` verb only (read), limits and its own log. Deploy with operator confirmation (VPS-005). |
| B2 | Hub `vpn/broker.rs` and the picture merge. "Waiting for approval" in the client. |
| B4 | `personal-key`, `router-key`, `register`, `outcome.ts`, enrolment modal, renewal. |
| B5 | `approve-route`. |
| B6 | `open-device`, `close-device` with add-this-machine (R-058, R-061 follow-up), if B0 accepts them. |

## 6. Full folders and large files to avoid

| Kind | Items |
| --- | --- |
| Hub folders at 12 | `tests/it`, `tests/it/checks/api`, `checks/scheduler/tests`, `metrics/tests`, `sessions/tests`, `sessions/files/tests` |
| Hub folders at 11 | `checks/probe`, `checks/graphql`, `checks/tests/probe`, `tests/it/devices`, `tests/it/sessions` |
| Hub files near 300 | `events/store.rs` (299), `devices/store.rs` (296), `devices/graphql.rs` (291), `main.rs` (300; BD-3), `checks/graphql/form.rs` (283) |
| Client folders at 12 | `lib/devices/form/form`, `lib/checks/form`, `lib/desktop`, `routes/devices/[id]`, `lib/motion`, `lib/metrics`, `lib/forms/modal` |
| Client files near 300 | `DeviceForm.svelte`, `lib/devices/api.ts` |

These counts are from `b4f7051d`. Phase 3 packages add files: `crates/hub/src` gets `text.rs`,
`fingerprint.rs` and `web_client.rs` (10 counted files at `c6517bce`), and `tests/it` is at 12
(the tests go in `tests/it/vpn/`). The counts that vpn-build.md Section 2 gives at `c6517bce`
are correct. Count again before each task starts (vpn-build.md Section 0, rule 2).

This is why Tier A uses a new `vpn` domain and does not add a check type: a new check type
touches `checks/probe`, `checks/graphql` and `checks/graphql/form.rs`, which are near their limits.

## 7. Order and dependencies

```text
A2-T1 site filter ──────────────────────────────────────────── (independent, can start now)
A5 version pin ── A3 reach + inventory ── A4 harness ──┐         (needs a deploy/headscale ruling)
                                                       └── A1-T1 fixtures ── A1-T2 pure core ── A1-T3 reader + store
                                                                                     ── A1-T4 GraphQL ── A2-T2/T3 client
first router enrols (vpn.md Section 5) ── A1-T5 live capture
B0 rulings ── B1 broker nodes ── B2 merge ── B4 keys + register ── B5 routes ── B6 device paths
```

## 8. Acceptance criteria

| Item | Criterion |
| --- | --- |
| A1 | With the proxy set, `vpn` returns each site with its router and a path. With a ping check on the router, the path is `DIRECT` or `RELAYED` within 2 reads, never `UNKNOWN`. No log line holds status text (search the process log). |
| A1 | A tagged node with an expiry shows `TAGGED_WITH_EXPIRY` and makes one warning event. A client key 29 days from expiry makes one event; a restart of the hub does not make a second one. A renewed key makes new events at the next thresholds. |
| A1 | A peer that is not the hub, a router or a device shows as not in Anima, and makes one event. A device with a DNS-name host is matched to its node. |
| A1 | A peer `HostName` of 200 characters or with control characters is cut and cleaned in the picture and in events. |
| A2 | The site filter limits the device list; the URL keeps it. The Settings section shows sites, keys and nodes not in Anima, or "VPN status is off". The device Overview shows the VPN facts. Each new motion has a reduced-motion form. Each path is readable without colour. |
| A3 | A policy change that lets a router reach another site fails `reach.test.ts`. `inventory.ts` reports each VPN-013 finding from a fixture that holds all of them. |
| A4 | `contract.sh` passes on 0.29.7, writes the A1-T1 fixtures, prints no key and leaves no container or volume. |
| B | Each verb refuses every input outside its rule (table tests). The broker refuses the 4th key in 24 hours and a key above the node limit, also when the hub asks with a valid session. A node registered with a tag is undone. A pre-auth key is in no log and in no command line shown by the app. Each change writes an audit entry in the hub and a line in the broker log. |

## 9. Operator rulings (2026-11-10)

| Number | Decision | Ruling |
| --- | --- | --- |
| VP-1 | Who builds it, and where | This session builds Tier A and Tier B now, in branch `claude/nostalgic-hawking-79449a` on top of `phase-3`. Not a package for the Linux-machine session, not Phase 4. |
| VP-2 | A `vpn` event source | Yes, if the setup is one click. Tier A needs no setup step: the compose file already sets `ANIMA_TAILSCALE_LOCALAPI` (default `localapi:8080`). The broker installs with one command (B1). |
| VP-3 | Tier B (R-055 b) | Yes. |
| VP-4 | VPS-005 for broker changes | Accepted as a spec change to VPS-005 and R-024: "A change that the operator starts in the app through a broker verb needs no confirmation in chat. The broker log replaces the `operator-changes.md` row for it. Policy changes (B6) still need confirmation in chat." |
| VP-5 | Contract changes (K1, K2, `hub-modules.json`) and changes to `deploy/headscale/` outside WP-54 and WP-52 | Approved. |
| VP-6 | Proxy log volume | Accepted. No change to `deploy/localapi/server.ts`. |
| VP-7 | Broker verbs | R-055's three, plus `nodes` and `register`. `open-device` and `close-device` (B6) only with a change to R-079. |
| VP-8 | Broker form | A systemd socket unit on the VPS. |
| VP-9 | Step-up TOTP | No. The authenticator code at sign-in is enough. Task B3 is removed. The broker controls in Section 5.1 are the main control for Tier B. |
| VP-10 | A sixth broker verb `renew` | Yes (2026-11-10). `renewVpnKey` is built in B4. |
| VP-11 | Tests on the VPS | Yes: the VPS is a development system, so the broker test install and runs may use it. Each live change is still announced in chat before it runs. |
| VP-12 | `renew` after a long expiry | The broker's `renew` refuses a node whose key expired more than 30 days ago (exactly 30 days is still allowed): `not-allowed`, "the key expired more than 30 days ago; enrol the computer again" (2026-11-11). A key that has not expired, or never expires, still renews. |
| VP-13 | The VPS as a device (2026-11-11) | Yes, through the narrowest exception. `ANIMA_PROBE_DENY` drops `${ANIMA_HUB_ADDRESS}/32` (100.64.0.1); the Docker networks and the VPS public address stay denied. The VPS firewall gets one exception before its DNS-005 drops: only the hub container's own source address (`hub_source`, 172.30.0.10 on `gateway`, the network of its default route, pinned by `gw_priority`) may reach `hub_vpn_address` (100.64.0.1), on 22/tcp and ICMP echo with the existing rate limit. Every other container, source and port (Headscale, 443, 9090, the broker, Apprise) stays blocked. VNC to the VPS is out of scope. Operator steps: `deploy/README.md`, "The VPS as a device". Amends VPS-001, DNS-005, R-030 and P3-003. Follow-up (lead's decision under the operator's "full device" ruling, 2026-11-11): sshd admits the unprivileged user `anima-probe` only from the hub's address (`AllowUsers <admin> anima-probe@172.30.0.10`, keys only), and `harden.sh check` reports any other entry; guacd gets the fixed address `ANIMA_GUACD_IP` (default 172.30.0.20), so it can never take 172.30.0.10. |

The live deployment of the broker on the VPS still needs the operator's confirmation in chat for
each change (VPS-005), until the VP-4 change is in the spec.

## 10. Handoff (2026-11-11)

The build session moved to a fresh session to keep turns short. State at this point:

- **Branch.** `claude/nostalgic-hawking-79449a` (on top of origin/phase-3) holds Tier A, Tier B,
  VP-12, the tailnet-site fix and VP-13. Gates passed 12/12 at 129cfa87. Not pushed, not merged
  into phase-3: both need the operator's word.
- **Live on the dev VPS.** Broker at 8dce3f80 (audit ok); hub image `anima-hub:5a8039de` with the
  VPN work, calling the broker (`docs/programme/operator-changes.md`, 22:58Z and 23:11Z rows).
  The desktop app was rebuilt and opened on the Mac.
- **Built, not yet on the VPS.** VP-12 (new broker binary), the tailnet-site fix (new hub image),
  VP-13 (firewall, sshd, compose), and the VP-13 security review fixes (F1–F6, I1–I3; build note
  in `docs/vpn-build.md`). A builder was finishing those fixes and the full gates on this branch
  at handoff (commits e0c6015d, 05ef3d7b and any after them); check `git log` and rerun the gates
  before deploying.
- **Next, in order.**
  1. Confirm the review fixes pass the full gates (`env -u DATABASE_URL BASE_REF=origin/phase-3 bun run gates`).
  2. Redeploy to the VPS as one batch the operator confirms once, then runs with their sudo
     password: broker (`bun deploy/headscale/bin/broker-build.ts`, then `harden.sh --sha256=… broker`),
     firewall (`--dry-run firewall`, `firewall`, `confirm` within 180 s), sshd and the
     `anima-probe` account (README "The VPS as a device (VP-13)" step 3), then the hub (fresh
     backup, new image from HEAD, new compose; the `gateway` and `acme` networks are recreated).
     Verify per the README and log each change in `docs/programme/operator-changes.md`.
  3. Rebuild and reopen the desktop app; the operator adds the VPS device at 100.64.0.1 with
     ping and TCP 22 checks.
- **Open, no action yet.** A dark-mode token for small accent text (m2); moving the broker off
  docker.sock (issue #1); the operator may delete `~anima-admin/anima-broker.XNT2lY` on the VPS and
  the local `r2.env`; R2 publishing (`ANIMA_RELEASES_ENDPOINT`, `ANIMA_UPDATER_PUBLIC_KEY`) is a
  later operator step.
- **Standing rules.** Every live VPS, Headscale or cloud change is announced in chat and confirmed
  by the operator (VPS-005, VP-11); the operator types sudo; never read `admin_pw.env`, `.env*`
  or key files except the R2 values the operator gave for the hub deploy.
