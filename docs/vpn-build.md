# Anima VPN Build Specification

| Item | Value |
| --- | --- |
| Document | Task-by-task build specification for [vpn-plan.md](vpn-plan.md) (Tier A and Tier B) |
| Version | 0.2 (review findings 1–35 applied) |
| Date | 2026-11-10 |
| Base | `phase-3` at `03369c72` (checked against `c6517bce`; nothing cited changed), branch `claude/nostalgic-hawking-79449a` (VP-1) |
| Language | ASD-STE100 Simplified Technical English |
| Rulings | vpn-plan.md Section 9 (VP-1 to VP-9) |

This document turns vpn-plan.md into tasks that a builder can do without design decisions.
Each task gives the files, the signatures, the SQL or SDL, the tests (with requirement IDs),
the gates and the commit message. Where this document and vpn-plan.md disagree, this document
is correct; vpn-plan.md gets the same change in task T0.

## 0. Rules for every task

1. Work in this worktree on `claude/nostalgic-hawking-79449a`. Before each task:
   `git fetch origin && git merge --no-edit origin/phase-3`. The Linux session commits to
   `phase-3` at the same time; conflicts are most likely in `deploy/headscale/`, `access/`,
   `contracts/` and the client.
2. Before a task starts, count again: `bun run quality:limits` must say `limits: ok`, and the
   folder counts in Section 2 must still be true. If a count changed, stop and update this
   document first.
3. After each task: `bun run gates` with `BASE_REF=origin/phase-3`. A task is done only when
   all gates pass. Do not merge to `phase-3` (VP-1: this branch is the integration point until
   the operator says otherwise).
4. Commit message form (one commit for each task, or more if the task says so):
   `VPN <task>: <what changed> (<requirement IDs>, <ruling IDs>)`, then a blank line, then the
   body, then `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
   Example: `VPN A1-T2: the pure picture of sites, nodes and paths (VPN-009, VPN-011, VP-1)`.
5. Test names start with the requirement ID: `vpn_009_a_router_with_traffic_and_cur_addr_is_direct`
   (Rust) or `it('VPN-009: a router with traffic and an endpoint is direct', ...)` (TypeScript).
6. Never put a pre-auth key, a node key or raw status text in a log, an event, a test name,
   a fixture or a commit (LOG-007). Fixtures use the value `REDACTED-PREAUTH-KEY`, and node N
   gets the key `nodekey:` + N as 64 hex digits (`nodekey:000…0003`); `mkey:` and `discokey:`
   use the same digits. Each node has its own placeholder, so warning marks do not collide.
8. Every task that changes `contracts/schema.graphql` runs `bun run --cwd apps/client graphql`
   and commits `apps/client/src/lib/hub/generated/**` in the same commit. The client `lint`
   runs `codegen.ts --check` and fails on stale types.
9. A hub value that K1 lacks fails `tests/it/contract.rs`. So a hub enum value and its K1 value
   land in the same commit. K1 uses no `extend` blocks: add fields inside `type Query`,
   `type Mutation`, `type Subscription`.
7. Never run anything against the live VPS, Headscale, Cloudflare or the live hub. Section 9
   lists the live steps; each needs the operator's confirmation in chat (VPS-005).

## 1. Decisions that this document fixes

These close the open points in vpn-plan.md. They are inside the VP-5 approval (contract
changes), so they need no new ruling.

| ID | Decision | Reason |
| --- | --- | --- |
| BD-1 | Migration range `1000–1019` for `vpn` (K2 row "Phase 3, VP-5"). `1000_events_source_vpn.sql`, `1001_vpn_schema.sql`. | Every number below 1000 is assigned or reserved. 1000 is after 0852, so 0701 cannot drop `vpn` again. |
| BD-2 | `vpn::Feed` is a `tokio::sync::watch` channel of `Option<Arc<Picture>>`. It holds the latest picture and wakes subscribers. One type, one `Dependencies` field. | A broadcast channel plus a lock is two types for one job. |
| BD-3 | `main.rs` frees lines by moving the start of the four background loops (scheduler, collector, dispatcher, stale watch) and `reaching()` into `upkeep::spawn`. | `main.rs` is at 300. `upkeep.rs` starts background work (R-072). All inputs are already in `Dependencies`. |
| BD-4 | The warning marks table has no primary key. It has `unique nulls not distinct (node_key, kind, key_expiry)` (PostgreSQL 15+, the hub uses 17). | A primary key makes `key_expiry` not null; `not_in_anima` and `tagged_expiry` marks have no expiry to key on. |
| BD-5 | A node's role comes from its tags, first match: contains `tag:hub` → HUB; else contains `tag:router` → ROUTER; else contains `tag:agent` → AGENT; else no tags → CLIENT; else OTHER. (A router also has `tag:site-<x>`.) | Headscale 0.29.7 puts every tagged node under the user `tagged-devices` (spike), so the user does not tell the role. |
| BD-6 | A router serves a site when its `PrimaryRoutes` holds the site subnet exactly. A router with no matching route is a node with role ROUTER and `inAnima = false`. | The hub sees a router with an unapproved route as a peer with no `PrimaryRoutes` (spike). |
| BD-7 | The broker is TypeScript in `deploy/headscale/broker/`, compiled on the Mac with `bun build --compile` to one Linux binary. systemd runs one process for each connection (`Accept=yes`). | VP-8. The deploy tools are TypeScript; the broker reuses `policy.ts` and `hujson.ts`; the VPS has no Bun. |
| BD-8 | The broker runs `docker compose -f /opt/anima/headscale/compose.yaml exec -T headscale headscale …`, as `hsctl` does. | The Headscale socket is on the container's tmpfs, not on the host. |
| BD-9 | The broker strips `machine_key`, `disco_key` and `pre_auth_key` from every node it returns, and refuses a tag that is not in the policy (`policy.scope.tags`) before it calls Headscale. | Headscale 0.29.7 returns `pre_auth_key.key` in `nodes list -o json`, and accepts unknown tags on pre-auth keys (spike). |
| BD-10 | The broker client in the hub is a tokio `UnixStream`: one JSON line out, one JSON line in, 64 KiB cap, 30 s limit. No HTTP. | One request for each connection; no extra crate. |
| BD-11 | B6 (`open-device`, `close-device`) is not built. | VP-7: only with a change to R-079, which the operator has not made. |
| BD-12 | Rulings go in vpn-plan.md Section 9 now. The programme row in `docs/programme/programme.md` gets the next free R number when the branch merges to `phase-3`. | The Linux session assigns R numbers; two sessions must not pick the same number. |
| BD-13 | A node keeps its last known path (not `UNKNOWN`) for 3 reads after traffic stops: pure `picture::carry(previous: &Picture, next: Picture) -> Picture`. VPN-009 says so. | Tailscale sets `Active` only for about 45 s after traffic. A 60 s ping check and a 60 s read can be out of phase and read `UNKNOWN` each time. |
| BD-14 | A node is matched to a device by its IPv4 address or its `DNSName` (Headscale's unique name), never by `HostName`. `VpnNode.name` is the first label of `DNSName`; cleaned `HostName` only when `DNSName` is empty. | `HostName` is chosen by the peer. A peer that calls itself `nas` must not hide from VPN-011. |
| BD-15 | The hub holds one `vpn::Broker` (a `Dependencies` field `vpn_broker: Option<vpn::Broker>`). It is `Clone` and holds an `Arc<tokio::sync::Mutex<()>>` for each call, so the reader and the mutations never call the broker at the same time. | The socket has `MaxConnections=1`; systemd closes a second connection at once. |
| BD-16 | The reader calls the broker `nodes` verb on each read (60 s). Mutations return after the broker accepts; the client waits on `vpnChanged` for the result. | An approved route or a new node shows in the hub's own status only seconds later. |
| BD-17 | A sixth broker verb `renew <node>` and the mutation `renewVpnKey(name)` are in B4 (operator ruling VP-10, 2026-11-10). | Key renewal is the most frequent manual step (vpn.md Section 7). |

## 2. Facts that limit the work (at `c6517bce`)

| Item | Value | Effect |
| --- | --- | --- |
| `crates/hub/src/main.rs` | 300 code lines | BD-3 frees about 25 lines. No other line may be added before A1-T0. |
| `crates/hub/tests/it` | 12 counted files | The tests go in `tests/it/vpn.rs` with a `tests/it/vpn/` folder (a face does not count). |
| `crates/hub/src` | 10 counted files | `vpn.rs` with `vpn/` is a face; it does not count. |
| `events/store.rs` 299, `checks.rs` 295 | near 300 | Do not add to them. `vpn` uses `events::record` and `checks::for_devices` only. |
| `apps/client/src/lib/events/panel` | 12 files | Edit `view.ts` only; add no file. |
| `apps/client/src/lib/forms/modal`, `lib/api-keys` | 12 files | New modals go in `lib/vpn/enrol/`. |
| `deploy/` | not folder-measured | `quality/src/limits.ts` `FOLDER_ROOTS` has no `deploy/`. File line limits still apply to `.ts`. |
| D-23 deny list | 27 names | Tier B adds 4 (B4) and 1 (B5): 27 → 32. Places: `access/guard.rs:231`, `access/tests/key_roots.rs:20`, `tests/it/api_keys/refused.rs:14`, the K1 doc strings. |
| `contracts/hub-domains.md` | K3 copies `pub enum Source` (line 139) and `AuditTarget` (line 140); imports table lines 153–188 | Change with A1-T3 (Source) and B4 (AuditTarget). |
| `crates/hub/src/text.rs:25` | `pub fn clean(s, max_chars)` removes control and invisible characters | `vpn` uses it; it writes no character list of its own. |
| `ipnet` | `serde` feature is on only through `hickory-resolver` | A1-T2 sets `ipnet = { version = "2", features = ["serde"] }` in the workspace `Cargo.toml`. |
| `deploy/test/compose-hub.test.ts` | lines 68–80 refuse an `ANIMA_*` hub variable that `config.rs` does not read; line 144 pins the hub mounts | The compose change goes in B3, after `config.rs`. |
| `quality/baseline.json` | `{}` | Nothing is excused. |

## 3. Facts from the local Headscale 0.29.7 spike

The fixtures are in the scratchpad (`hsspike/fixtures/`). A1-T1 copies them into the repo with
invented names. The facts:

1. `status.Self` and each `status.Peer[*]` have `PublicKey` (`nodekey:…`), `HostName`,
   `DNSName`, `OS`, `TailscaleIPs` (IPv4 first, then IPv6), `Tags` (absent when none),
   `PrimaryRoutes` (absent when none), `Online`, `Active`, `CurAddr`, `Relay`, `PeerRelay`,
   `LastSeen`, `KeyExpiry` (absent for a tagged node).
2. `LastSeen` is `"0001-01-01T00:00:00Z"` when the node is online. Treat the zero time as absent.
3. `Relay` is the home DERP region (`"sfo"`) for every node. **The spike has no active peer:**
   `status-active.json` differs from `status.json` only in `Self.Addrs`; every peer has
   `Active: false`, `CurAddr: ""`. The rule "`CurAddr` is set only while `Active` is true" is
   from the Tailscale source, not from evidence. A4 must capture a real active peer (A4 step 5).
   The spike also ran with `magic_dns: false`, so its `DNSName` has no suffix; production is
   `<name>.net.anima.ing.` (`base_domain`).
4. `status.Peer` is `null` when the node has no peers.
5. A router whose route is not approved is a peer of the hub, with no `PrimaryRoutes`.
6. Nodes of another user (`intruder`) and nodes with a tag no grant names (`tag:unknown`) are not
   peers of the hub.
7. `headscale nodes list -o json`: tagged nodes have `user.name = "tagged-devices"` and no
   `expiry`. Untagged nodes have `expiry: {seconds, nanos}`. Every node has
   `pre_auth_key.key` in clear text, and `pre_auth_key.id`, when it joined with a key. Zero
   values are left out: `tags`, `expiry`, `approved_routes` are absent when empty, an offline
   node has no `online` key, and `nanos` can be absent.
8. `headscale nodes list-routes -o json` returns only nodes with routes, each with
   `approved_routes`, `available_routes`, `subnet_routes`.
9. Headscale accepts `preauthkeys create --tags tag:unknown`.

Facts from A4 (`deploy/headscale/test/contract.sh`, run twice with Headscale 0.29.7 and
`tailscale/tailscale:v1.104.4@sha256:ed6480b3…e276c`; fixtures in `crates/hub/src/vpn/tests/fixtures/`
and `deploy/headscale/tests/fixtures/`). They close the A4 evidence in Section 10:

10. **An active peer is real.** After `tailscale ping --c 3 <router IP>` (a disco ping; no TSMP
    ping and no `nc` needed) the router peer has `Active: true` and `CurAddr: "172.29.250.4:48037"`
    (the container address and an ephemeral port; `status-active.json`). `Relay` stays `"sfo"`
    and `PeerRelay` stays `""`. After 60 s with no traffic the same peer has `Active: false` and
    `CurAddr: ""` (`status-after-60s.json`): the path state is lost inside 60 s, so BD-13 is
    needed. `DNSName` is `router-home.net.anima.ing.` and `LastSeen` is the zero time while online.
    The hub has 3 peers (`router-home`, `mac`, `mac2`); `stranger` and `odd` are not peers (fact 6).
11. **Auth ID.** The login URL is `http://<server>/register/<auth ID>`. The ID is
    `hskey-authreq-` followed by 24 characters of base64url (`A-Za-z0-9`, `-`, `_`; both `-` and
    `_` were seen). B1's pattern is `^hskey-authreq-[A-Za-z0-9_-]{24}$`. `auth register` refuses an
    ID without the prefix (`auth ID has invalid prefix`). The ID is in the `AuthURL` field of
    `tailscale status --json` while the node waits.
12. **`preauthkeys create --tags … --expiration 1h -o json`** returns
    `{"id": 1, "key": "hskey-auth-…", "expiration": {"seconds", "nanos"}, "created_at": {"seconds",
    "nanos"}, "acl_tags": ["tag:hub"]}`. Numbers are JSON numbers, not strings. `used`,
    `reusable`, `ephemeral` and `user` are left out when false or empty. A key for a user needs
    `--user <numeric id>` (a name is refused); `users create <name> -o json` returns that `id`.
    (`preauthkey.json`.)
13. **`auth register --user operator --auth-id <id> -o json`** returns the node, not a wrapper:
    `id`, `machine_key`, `node_key`, `disco_key`, `ip_addresses`, `name`, `user` (`id`, `name`,
    `created_at`), `last_seen`, `expiry` (`seconds`, `nanos`), `created_at`, `register_method: 2`,
    `given_name`. No `pre_auth_key`, no `online`, no `tags`. The renewal (`tailscale up
    --force-reauth`, then `auth register` with the new ID) returns the same `id` and the same
    addresses with a new `expiry` (`register.json`, `register-again.json`).
    The node shows `register_method: 1` when it joined with a key.
14. **`nodes list -o json`** has `id`, `pre_auth_key.id` and every `seconds` as JSON numbers.
    A tagged node has `user = {id: 2147455555, name: "tagged-devices", display_name: "Tagged
    Devices", created_at.seconds: -62135596800}` and `tags`; an untagged node has `expiry`. The
    default expiry is 180 days after registration. `nodes list-routes -o json` returns only the
    router, with `approved_routes`, `available_routes` and `subnet_routes`. (`nodes.json`,
    `routes.json`.) `deploy/headscale/src/inventory.ts` parses these files (test
    `contract-fixtures.test.ts`).
15. **`headscale policy check` needs the nodes.** On an empty server it fails with `source
    "operator@" resolved to no IP addresses` for each test row. The harness runs it after the
    joins (the repository policy then passes).
16. **A plain-HTTP control server must have a private IP.** With `--login-server
    http://headscale:8080` the first login works, but `tailscale up --force-reauth` and
    `tailscale logout` fail with `register request: Post "https://headscale:8080/machine/register":
    … HTTP: TLS forced: no port 80 dialed`. The same happens with a documentation address
    (`192.0.2.10`). It works with a private address (`172.29.250.10`). The harness therefore gives
    Headscale that address on its own Docker network (Build notes).

## 4. Task order

```text
T0 records ─┬─ A5 version pin ── A3a reach ── A3b inventory ── A4 harness
            ├─ A2-T1 site filter (client only)
            └─ A1-T0 main.rs seam ── A1-T1 fixtures ── A1-T2 pure core ── A1-T3 events + store
                 ── A1-T4 reader ── A1-T5 GraphQL ── A2-T2 Settings section ── A2-T3 device facts
B1 broker core ── B2 broker server + install ── B3 hub broker client + merge
                 ── B4 keys + register + enrolment ── B5 approve route
```

A-tasks on different lines can run in parallel. Tier B starts after A1-T5 (it extends
`vpn/graphql.rs` and the picture).

---

## 5. Task T0: records

**Files.**

| File | Change |
| --- | --- |
| `docs/vpn-plan.md` | Commit it (it is untracked). Version 0.4: apply BD-1 to BD-12; Section 6 counts from Section 2 here; Section 4.1 SQL from A1-T3. |
| `docs/vpn-build.md` | Commit this document. |
| `docs/design-spec.md` Section 4.2 | Add VPN-008 to VPN-014 (text from vpn-plan.md 4.1, 4.3, 4.4, with BD-5 and BD-6). Change VPS-005: add "A change that the operator starts in the app through a broker verb needs no confirmation in chat; the broker log replaces the `operator-changes.md` row for it (VP-4)." |
| `docs/design-spec.md` Section 7 | Add SET-003 (vpn-plan.md 4.2), with this text for the empty state: "When the hub has no VPN picture, the section must say: VPN status is not available. The hub has no LocalAPI proxy (ANIMA_TAILSCALE_LOCALAPI), or it has not answered since the hub started." VPN-009 gets BD-13 (the last known path is kept for 3 reads). |
| `contracts/database.md` | Row `\| 1000–1019 (Phase 3, VP-5) \| vpn \| \`vpn\` \| warnings \|`. An events row like 0852's: "1000: `event_log_source_check` lists all eight sources, adding `vpn`; numbered after 0852". |
| `contracts/hub-domains.md` (K3 table, lines 153–188) | Row for `vpn`: may import `devices`, `events`, `checks`; its graphql part may import `access`. |
| `quality/hub-modules.json` | `"vpn": ["devices", "events", "checks"]` in `domains`; `"vpn": ["access"]` in `graphql`. |
| `docs/setup/vpn.md` Section 11 | One paragraph: Tier B adds the broker; it is the only path from the hub to Headscale; its verbs and limits (Section 7 here). One sentence: "The broker mount works only while SELinux is disabled (R-054); turning SELinux on needs a policy module first." |

`checks` is a domain import (not graphql-only) because the reader reads
`checks::for_devices` for `hasPingCheck` and `vpn::Error` wraps `checks::Error`. This keeps the
resolver free of database calls. The edge makes no cycle (checked).

**Gates.** `bun run quality:imports` passes with no `vpn` module yet: an unlisted file is an
error, a listed module with no file is not (`quality/src/imports.ts:42-48`).

**Commit.** `VPN T0: the build specification, VPN-008 to VPN-014, SET-003 and the K2/K3 rows (VP-1, VP-4, VP-5)`.

---

## 6. Tier A tasks

### A5. One Headscale version

**Files.**

- `deploy/headscale/src/version.ts` (new, 3 code lines):
  ```ts
  /** The one Headscale version that Anima supports (vpn.md 13.1, VPN-014). */
  export const HEADSCALE_VERSION = '0.29.7';
  /** The Tailscale image the contract harness runs, by tag and digest (A4). */
  export const TAILSCALE_TEST_IMAGE = 'tailscale/tailscale:<tag>@sha256:<digest>';
  ```
  Fill the tag and digest when A4 is written: `docker buildx imagetools inspect
  tailscale/tailscale:stable` on the day, and pin what it shows.
- `deploy/headscale/tests/server.test.ts` lines 273–277: build the image regex from
  `HEADSCALE_VERSION` (escape the dots) instead of the literal.
- `deploy/headscale/tsconfig.json`: `"include": ["src", "tests", "bin", "test", "broker"]`.
  (`bin`, `test`, `broker` do not exist yet; `tsc` accepts a missing include. Check this; if
  `tsc` fails, add each folder in the task that makes it.)

**Tests.** `server.test.ts`: `it('VPN-014: compose.yaml pins HEADSCALE_VERSION by tag and digest', …)`.

**Commit.** `VPN A5: one constant for the Headscale version (VPN-014, VP-5)`.

### A3a. Reachability table

**File** `deploy/headscale/src/reach.ts` (about 80 code lines). Pure.

```ts
import type { Policy } from './policy.ts';
import { allows, type TailnetNode } from './tailnet.ts';

/** One class of node or LAN address that VPN-004 names. */
export type Party =
  | { readonly kind: 'client'; readonly node: string }
  | { readonly kind: 'hub'; readonly node: string }
  | { readonly kind: 'agent'; readonly node: string }
  | { readonly kind: 'router'; readonly node: string; readonly site: string }
  | { readonly kind: 'lan'; readonly site: string; readonly address: string }
  | { readonly kind: 'device'; readonly node: string };

export interface Reach { readonly from: Party; readonly to: Party; readonly port: number; readonly proto: 'tcp' | 'udp'; readonly allowed: boolean; }

/** The ports that the table probes: 1, 65535, and for each grant `PortRule` its `first`, `last`,
 *  `first − 1` and `last + 1` (kept inside 1–65535), without repeats, sorted. */
export function probedPorts(policy: Policy): readonly number[];

/** Every party, from the nodes and the policy's site hosts. */
export function parties(policy: Policy, nodes: readonly TailnetNode[]): readonly Party[];

/** The full table: each pair of different parties, each probed port, tcp and udp. */
export function reachTable(policy: Policy, nodes: readonly TailnetNode[]): readonly Reach[];

/** The table as Markdown rows, sorted, one row for each allowed pair; for a diff. */
export function reachMarkdown(table: readonly Reach[]): string;
```

`reachTable` calls `allows(policy, nodes, connection)` once for each row. It adds no evaluator.
`policy.ts` does not change: the ports are in `policy.grants[].ip` (`PortRule { proto, first,
last }`), and the tags and hosts are in `policy.scope.tags` and `policy.scope.hosts`
(`policy.ts:37-71`). There is no `policy.tagOwners`.

`parties` rules: a node whose IP is inside a `device-*` host is `device` (and not also
`client`); other nodes take their role from their tags (BD-5); a router's `site` is `<x>` from
its `tag:site-<x>`; each `site-<x>` host gives one `lan` party at address `.5` of the subnet.
A LAN source with no router of that site in the nodes is always denied (`tailnet.ts:146`).

**File** `deploy/headscale/bin/reach.ts` (about 15 lines): read `config/policy.hujson`, use the
fixture nodes from `tests/policy-fixture.ts` (`nodes`), print `reachMarkdown`. Usage in the
first comment line: `bun deploy/headscale/bin/reach.ts > /tmp/before.md`.

**File** `deploy/headscale/tests/reach.test.ts`. Uses `policy` and `nodes` from
`policy-fixture.ts`. Properties over the whole table (VPN-004, VPN-012):

| Test | Property |
| --- | --- |
| `VPN-012: no router reaches another site` | For each row from a router of site X to a LAN of site Y ≠ X: not allowed. |
| `VPN-012: no LAN reaches a client` | For each row from `lan` to `client`: not allowed. |
| `VPN-012: an agent reaches only the hub on tcp 443` | Each allowed row from `agent` has `to.kind === 'hub'`, tcp, 443. |
| `VPN-012: a device /32 is reached only from the hub on tcp` | Each allowed row to `device` has `from.kind === 'hub'` and tcp. |
| `VPN-012: a client reaches the hub` | Row client → hub tcp 443 is allowed. |
| `VPN-012: a widened grant fails the table` | `withGrants([...(raw.grants as unknown[]), {src: ['tag:site-home'], dst: ['site-office2'], ip: ['tcp:22']}])` (`withGrants` replaces the list, so spread the old one; the fixture has `router-office2`, not `router-office1`); `crossSiteRows(table)` is not empty. Write each property as an exported function (`crossSiteRows`, …) so the tests call the same code. |

**Commit.** `VPN A3a: the policy reachability table and its VPN-004 properties (VPN-012, VP-5)`.

### A3b. Inventory check

**File** `deploy/headscale/src/inventory.ts` (about 150 code lines). Pure.

```ts
/** One node from `headscale nodes list -o json` (0.29.7), without any key. */
export interface ListedNode {
  readonly id: number; readonly name: string; readonly user: string;
  readonly tags: readonly string[]; readonly ips: readonly string[];
  readonly online: boolean; readonly expiry: Date | null;
  readonly preAuthKeyId: number | null;   // the key's id, never the key (used by the broker, B1)
}
export interface Tailnet { readonly nodes: readonly ListedNode[]; readonly routes: readonly ListedRoutes[]; }
export interface ListedRoutes { readonly id: number; readonly approved: readonly string[]; readonly available: readonly string[]; }

export type Finding =
  | { readonly kind: 'foreign-user'; readonly node: string; readonly user: string }
  | { readonly kind: 'unknown-tag'; readonly node: string; readonly tag: string }
  | { readonly kind: 'tagged-with-expiry'; readonly node: string }
  | { readonly kind: 'long-expiry'; readonly node: string; readonly days: number }
  | { readonly kind: 'unapproved-route'; readonly node: string; readonly route: string }
  | { readonly kind: 'unexpected-node'; readonly node: string }
  | { readonly kind: 'missing-node'; readonly node: string }
  | { readonly kind: 'policy-test-failed'; readonly test: string };

export class InventoryError extends Error {}

/** Parses the node list; drops every key field; throws InventoryError on a shape it does not know. */
export function parseNodes(json: unknown): readonly ListedNode[];
export function parseRoutes(json: unknown): readonly ListedRoutes[];
/** The nodes as the policy evaluator sees them. */
export function tailnetNodes(tailnet: Tailnet): readonly TailnetNode[];
/** Every VPN-013 finding. `expected` is the list of node names; `now` is a parameter. */
export function findings(policy: Policy, tailnet: Tailnet, expected: readonly string[], now: Date): readonly Finding[];
```

Parser rules (spike fact 7): absent `online` → false; absent `tags`, `approved_routes`,
`available_routes` → `[]`; absent `expiry` → null; `nanos` optional.

Rules for `findings`:

- `foreign-user`: an untagged node whose user is not `operator`; a tagged node whose user is not
  `tagged-devices` (spike fact 7).
- `unknown-tag`: a tag not in `policy.scope.tags`.
- `tagged-with-expiry`: tags present and `expiry` not null. Exception: none (`tag:agent` too,
  R-105).
- `long-expiry`: untagged and (`expiry` null or more than 180 days after `now`).
- `unapproved-route`: a route in `available` and not in `approved`.
- `unexpected-node` / `missing-node`: compare names with `expected`.
- `policy-test-failed`: run `runPolicyTests(policy, tailnetNodes(...))`; one finding for each
  failed test.

The parser reads seconds from `{seconds, nanos}`. It never copies `pre_auth_key`,
`machine_key`, `node_key`, `disco_key`.

**File** `deploy/headscale/bin/inventory.ts` (about 30 lines):
`bun deploy/headscale/bin/inventory.ts nodes.json routes.json expected.txt`. Prints one line for
each finding (`! foreign-user stranger (user intruder)`), or `inventory: ok`. Exit 1 when there
is a finding. It reads the policy from `config/policy.hujson` beside it.

**Fixtures** `deploy/headscale/tests/inventory-fixture.ts`: the spike `nodes.json` and
`routes.json` shapes with invented names: `hub` (tag:hub), `mac` (operator, expiry +170 d),
`router-home` (tag:router, tag:site-home, route approved), `stranger` (user `intruder`),
`odd` (tag:unknown), plus `router-office1` with `available` ≠ `approved`, `agent-nas`
(tag:agent, with an expiry), `mac2` (operator, no expiry). `pre_auth_key.key` is
`'REDACTED-PREAUTH-KEY'`.

**Tests** `deploy/headscale/tests/inventory.test.ts`:

| Test | Checks |
| --- | --- |
| `VPN-013: each finding kind is reported from the fixture` | `findings(...)` equals the exact list, in order. |
| `VPN-013: a tagged node under tagged-devices is not a foreign user` | `hub`, `router-home` give no `foreign-user`. |
| `VPN-013: an agent node with an expiry is reported (R-105)` | `agent-nas` gives `tagged-with-expiry`. |
| `VPN-013: no key text survives parsing` | `JSON.stringify(parseNodes(raw))` does not contain `REDACTED-PREAUTH-KEY`, `nodekey:`, `mkey:`, `discokey:` (the fixture uses these prefixes, rule 0.6). |
| `VPN-013: an unknown shape is refused` | a node without `ip_addresses` → `InventoryError`. |
| `VPN-013: a clean tailnet prints ok` | fixture without the bad nodes → `[]`. |

**vpn.md changes (same commit).** Section 8 step 1: run `bin/reach.ts` before and after a policy
edit and read the diff. Section 11 detection and Sections 5, 6, 7 (after each enrolment and
renewal): `hsctl nodes list -o json > nodes.json`, `hsctl nodes list-routes -o json > routes.json`,
copy to the Mac with `scp`, run `bin/inventory.ts`, delete both files on both machines. Say that
`nodes.json` holds pre-auth keys in clear text (spike fact 7) and must not be kept, mailed or
committed.

**Commit.** `VPN A3b: the inventory check for OD-10 nodes, keys and routes (VPN-013, VP-5)`.

### A4. Contract harness

**File** `deploy/headscale/test/contract.sh` (manual, "Needs Docker. Not a gate."). Pattern:
`deploy/test/smoke.sh` (`set -eu`, `here`, `repo`, `work=$(mktemp -d)`,
`project="anima-contract-$$"`, `trap cleanup EXIT` with `docker compose -p "$project" down -v`
and `rm -rf "$work"`).

**File** `deploy/headscale/test/contract-config.yaml` (committed): the repository
`config/config.yaml` with these changes only: `server_url: http://headscale:8080`,
`listen_addr: 0.0.0.0:8080`, `derp.server.enabled: false`,
`derp.urls: [https://controlplane.tailscale.com/derpmap/default]`, no `tls_letsencrypt_*` and no
`acme_*` keys, no `${…}` template values; `dns.magic_dns: true` and `base_domain:
net.anima.ing` stay. A test in `server.test.ts` checks that every other key equals the
repository config, so the two cannot drift.

Steps:

1. Write a compose file in `$work`: `headscale` from the image and digest in
   `deploy/headscale/compose.yaml`, `contract-config.yaml`, and the repository `policy.hujson`
   unchanged. Tailscale containers use `TAILSCALE_TEST_IMAGE` (A5) with
   `--tun=userspace-networking`.
2. `headscale policy check --file /etc/headscale/policy.hujson` must pass (VPN-014). Create the
   users `operator` and `intruder`.
3. Start five containers with keys: `hub` (tag:hub), `router` (tag:router, tag:site-home,
   `--advertise-routes=10.10.0.0/24`), `mac` (user operator), `stranger` (user intruder),
   `odd` (tag:unknown). Each key: `preauthkeys create … -o json | tee "$work/<name>.json"
   >/dev/null`; keep the first reply, redacted, as `preauthkey.json` (the shape B2 parses); take
   the key with `bun -e` from the file, write it to `$work/<name>.key`, `docker cp` it into the
   container, `tailscale up --auth-key=file:/tmp/k`. The script never echoes a key.
4. A sixth container `mac2` runs `tailscale up --login-server http://headscale:8080` with no
   key. Read the auth ID from `tailscale status --json` `.AuthURL` (the last path part). Run
   `auth register --user operator --auth-id=<id> -o json` and save the redacted reply as
   `register.json`. Log `mac2` out and in again and register again: `register-again.json`
   (renewal of an existing node, vpn.md Section 7). Record the ID's character set and length
   in Section 3 of this document; B1's pattern uses them.
5. Approve the router route: `nodes approve-routes --identifier=<id> --routes=10.10.0.0/24`.
6. Capture `status` from `hub`: idle (`status-idle.json`), then after `tailscale ping --c 3
   <router>` (`status-active.json`). **Fail unless the router peer has `Active == true` and a
   non-empty `CurAddr`.** If a disco ping does not set `Active`, use `tailscale ping --tsmp`,
   then `tailscale nc <router> 22`; record which one worked in Section 3. Wait 60 s with no
   traffic and capture `status-after-60s.json` (evidence for BD-13). Capture `nodes list -o json`
   and `nodes list-routes -o json`.
7. Pipe every captured file through `bun deploy/headscale/bin/redact.ts` (new, about 40 lines):
   node N gets `nodekey:`/`mkey:`/`discokey:` + N as 64 hex digits, in `PublicKey`, in the
   `Peer` map keys and in `node_key`/`machine_key`/`disco_key`; `pre_auth_key.key` and the key
   in `preauthkey.json` become `REDACTED-PREAUTH-KEY`. Write the files to
   `crates/hub/src/vpn/tests/fixtures/` and `deploy/headscale/tests/fixtures/`.
8. Run `bin/inventory.ts` on the captured files; expect exactly `foreign-user stranger`,
   `unknown-tag odd`.
9. Print `contract: ok`.

`vpn.md` 13.1 step 1: run `deploy/headscale/test/contract.sh` with the new image before the
upgrade.

**Done when.** The script passes twice in a row, `docker ps -a` and `docker volume ls` show
nothing with the project name, and `grep -r REDACTED` finds the placeholder in each fixture.

**Commit.** `VPN A4: the Headscale contract harness and its fixtures (VPN-014, VP-5)`.

### A1-T0. Move the background loops into `upkeep::spawn` (BD-3)

**Files.**

- `crates/hub/src/main.rs`: delete `reaching()` with its doc comment (lines 196–216), lines
  153–154 (`let (probing, collecting, sending) = reaching(...)`), the four `let scheduler/
  collector/dispatcher/stale_watch = …` statements (lines 169–174) and
  `background.extend([...])` (line 191). `let mut background` becomes `let background`. Keep
  `let feed = metrics::Feed::new();`; change `metrics_feed: feed.clone()` to `metrics_feed:
  feed` if `feed` has no other use. Remove the imports that become unused (`checks`,
  `notifications`; the compiler lists them).
- `crates/hub/src/upkeep.rs`: move `reaching()` here unchanged (private), with the imports it
  needs (`credentials::Vault`, `targets::AddressPolicy`), add `let bus = &dependencies.bus;`,
  and at the end of `spawn` add, in this order:
  ```rust
  let (probing, collecting, sending) =
      reaching(&dependencies.vault, &dependencies.policy, dependencies.config.tailscale_localapi.clone());
  jobs.push(checks::spawn_scheduler(pool.clone(), bus.clone(), clock.clone(), probing));
  jobs.push(metrics::spawn_collector(pool.clone(), dependencies.metrics_feed.clone(), bus.clone(), clock.clone(), collecting));
  jobs.push(notifications::spawn_dispatcher(pool.clone(), bus.clone(), clock.clone(), sending));
  jobs.push(metrics::spawn_stale_watch(pool.clone(), bus.clone(), clock.clone()));
  ```
  Change the module doc comment: "Starts the hub's background work: periodic jobs and the
  check, metrics and notification loops (R-072)."

**Check.** The loops already start before `access::Edge::new` (`main.rs:168-176`), and
`Edge::new` only builds a struct (`access/layer.rs:64`), so the order does not change. No test
spawns these loops through `main`.

**Done when.** `main.rs` ≤ 278 code lines, `upkeep.rs` ≤ 160. All hub tests pass unchanged.

**Commit.** `VPN A1-T0: upkeep starts every background loop, so main.rs has room (R-072, R-082)`.

### A1-T1. Fixtures

Copy the A4 fixtures (or, before A4, the redacted spike fixtures with the rule 0.6 keys) to
`crates/hub/src/vpn/tests/fixtures/`: `status-idle.json`, `status-no-peers.json`
(`"Peer": null`). Invented names; addresses in `100.64.0.0/10` and `10.10.0.0/24`; `DNSName`
values with the production suffix (`router-home.net.anima.ing.`).

Until A4 gives real ones, make these by hand from `status-idle.json` (spike fact 3):
`status-active.json` (router peer `Active: true`, `CurAddr: "192.0.2.10:41641"`) and
`status-relayed.json` (router `Active: true`, `CurAddr: ""`, `Relay: "sfo"`). A4 replaces
`status-active.json` with the real capture.

Add a hand-made `status-mixed.json` with: a router with `PeerRelay` set; a router with
`PrimaryRoutes` absent (BD-6); a client whose `KeyExpiry` is 20 days after the test clock; a
tagged node with `KeyExpiry` set; an untagged peer that matches no device; a peer with a
200-character `HostName` holding `\u0007` and `‮`; a peer whose `HostName` is `nas` and
whose `DNSName` is `evil.net.anima.ing.` (BD-14); a peer with `"TailscaleIPs": null`.

No commit of its own; it goes with A1-T2.

### A1-T2. The pure core

Domain face and folder. Code-line budgets in brackets.

**`crates/hub/src/vpn.rs`** (face, about 40):

```rust
//! The VPN as the hub's own node sees it (VPN-005, VPN-008 to VPN-011).
mod feed; pub mod graphql; mod localapi; mod model; mod peers; mod picture; mod reader; mod store; mod warnings;
#[cfg(test)] mod tests;

pub use feed::Feed;
pub use model::{KeyState, Picture, Role, RouteState, SitePath, VpnNode, VpnPath};
pub use reader::{Reading, spawn_reader};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the LocalAPI proxy did not answer")] Request,
    #[error("the LocalAPI proxy answered {0}")] Status(u16),
    #[error("the LocalAPI proxy refused the request")] Refused,
    #[error("the status reply is longer than 1 MiB")] TooLong,
    #[error("the status reply is not the expected shape")] Unreadable,
    #[error(transparent)] Devices(#[from] crate::devices::Error),
    #[error(transparent)] Checks(#[from] crate::checks::Error),
    #[error(transparent)] Events(#[from] crate::events::Error),
    #[error(transparent)] Database(#[from] sqlx::Error),
}
impl Error {
    /// A fixed word for logs; never the reply text (LOG-007).
    #[must_use] pub const fn fault(&self) -> &'static str;
}
```

(Write each `mod` and attribute on its own line; rustfmt decides.)

**`vpn/model.rs`** (about 90): the types the picture is made of. No serde, no SQL.

```rust
pub enum Role { Hub, Router, Agent, Client, Other }                         // BD-5
pub enum VpnPath { Direct, PeerRelay, Relayed, Unknown }                    // VPN-009
pub enum RouteState { Serving, Offline, NoRouter }
pub enum KeyState { Never, Valid, Expiring, Expired, TaggedWithExpiry }      // VPN-010
pub struct VpnNode {
    pub key: String,               // nodekey:…, public; used for warning marks only, not in GraphQL
    pub name: String, pub address: Ipv4Addr, pub os: String, pub role: Role,
    pub tags: Vec<String>, pub routes: Vec<IpNet>, pub online: bool,
    pub last_seen: Option<DateTime<Utc>>, pub key_expiry: Option<DateTime<Utc>>,
    pub key_state: KeyState,
    pub device: Option<(DeviceId, String)>,   // id and name, as events::NewEvent.device
    pub in_anima: bool,
    pub path: VpnPath, pub relay_region: Option<String>,
    pub unknown_reads: u8,                    // reads since the path was last known (BD-13)
}
pub struct SitePath { pub site: SiteId, pub router: Option<usize> /* index in nodes */, pub state: RouteState, pub has_ping_check: bool }
pub struct Picture { pub read_at: DateTime<Utc>, pub fresh: bool, pub hub: VpnNode, pub sites: Vec<SitePath>, pub nodes: Vec<VpnNode> }
```

Each enum derives `Clone, Copy, Debug, PartialEq, Eq, async_graphql::Enum` with
`#[graphql(name = "VpnRole")]`, `"VpnPath"`, `"SiteRouteState"`, `"KeyState"`. The structs derive
`Clone, Debug, PartialEq`. The file starts with the same `#![expect(clippy::unused_async_trait_impl,
reason = "…")]` header as `events/model.rs:2`, which the `Enum` derive needs.

**Workspace `Cargo.toml`**: `ipnet = { version = "2", features = ["serde"] }` (Section 2).

**`vpn/peers.rs`** (about 90): serde types and cleaning. Pure.

```rust
#[derive(Deserialize)] #[serde(rename_all = "PascalCase")]
pub(super) struct Status { #[serde(rename = "Self")] pub own: Peer, #[serde(default)] pub peer: Option<HashMap<String, Peer>> }

#[derive(Deserialize)] #[serde(rename_all = "PascalCase")]
pub(super) struct Peer {
    pub public_key: String, pub host_name: String,
    #[serde(rename = "DNSName")] pub dns_name: String, #[serde(rename = "OS")] pub os: String,
    #[serde(rename = "TailscaleIPs", default, deserialize_with = "null_is_empty")] pub tailscale_ips: Vec<IpAddr>,
    #[serde(default, deserialize_with = "null_is_empty")] pub tags: Vec<String>,
    #[serde(default, deserialize_with = "null_is_empty")] pub primary_routes: Vec<IpNet>,
    pub online: bool, #[serde(default)] pub active: bool,
    #[serde(default)] pub cur_addr: String, #[serde(default)] pub relay: String, #[serde(default)] pub peer_relay: String,
    #[serde(default, deserialize_with = "zero_is_none")] pub last_seen: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "zero_is_none")] pub key_expiry: Option<DateTime<Utc>>,
}
pub(super) fn parse(body: &[u8]) -> Result<Status, Error>;            // Unreadable on any serde error
/// `crate::text::clean(text, max)`, or "?" when that is empty (VPN-008).
pub(super) fn shown(text: &str, max: usize) -> String;               // name 63, os 32, tag 63, region 16
fn null_is_empty<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>;      // JSON null or absent → []
```

`#[serde(default)]` does not accept JSON `null`, and Go writes `TailscaleIPs` with no
`omitempty`; so the three lists use `null_is_empty`. Unknown JSON fields are ignored (no
`deny_unknown_fields`: Tailscale adds fields).

**`vpn/picture.rs`** (about 140): pure.

```rust
pub(super) struct Inputs<'a> {
    pub status: &'a Status, pub sites: &'a [Site], pub devices: &'a [Device],
    pub ping_checked: &'a HashSet<DeviceId>, pub now: DateTime<Utc>,
}
pub(super) fn build(inputs: &Inputs<'_>) -> Picture;
/// The devices of the peers that serve a site, so the reader reads only their checks.
pub(super) fn router_devices(status: &Status, sites: &[Site], devices: &[Device]) -> Vec<DeviceId>;
/// BD-13: a node whose path is Unknown in `next` keeps its path from `previous` while
/// `unknown_reads` < 3; the counter goes up by one each such read and to 0 on a known path.
pub(super) fn carry(previous: &Picture, next: Picture) -> Picture;
fn role(tags: &[String]) -> Role;                                          // BD-5
fn path(peer: &Peer) -> (VpnPath, Option<String>);                          // VPN-009
fn key_state(tags: &[String], expiry: Option<DateTime<Utc>>, now: DateTime<Utc>) -> KeyState;
fn device_of(peer: &Peer, devices: &[Device]) -> Option<(DeviceId, String)>;
```

Rules:

- `name`: the first label of `DNSName` (`shown(…, 63)`); `shown(HostName, 63)` only when
  `DNSName` is empty (BD-14).
- `path`, in this order: not `online` → `Unknown`; `peer_relay` non-empty → `PeerRelay`;
  `active` and `cur_addr` non-empty → `Direct`; `active` → `Relayed` with region
  `shown(relay, 16)` (none when `relay` is empty); else `Unknown`.
- `key_state`: tags non-empty and expiry some → `TaggedWithExpiry`; expiry none → `Never`;
  expiry ≤ now → `Expired`; expiry ≤ now + 30 days → `Expiring`; else `Valid`.
- `device_of` (BD-14), in this order, first match in `devices` order (name order from
  `devices::list`): (1) a device whose `ip` equals the peer's first IPv4; (2) a device whose
  `host` (lower case, trailing dot removed) equals the peer's `DNSName` (lower case, trailing
  dot removed); (3) a device whose `host` has no dot and equals the first label of `DNSName`.
  `HostName` is never used. A device in another site may match; that is correct (a client
  computer has a VPN address).
- A site's router: the first peer (sorted by name) with role `Router` whose `primary_routes`
  contains the site's `subnet` exactly. `state`: router found and online → `Serving`; found and
  offline → `Offline`; none → `NoRouter`. `has_ping_check`: the router has a device and that
  device is in `ping_checked` (the reader puts in it only checks that are not paused).
- `in_anima`: `Hub` → true; `Router` → serves at least one site; any other role → `device`
  is some.
- `nodes`: every peer, sorted by name, then address. `address`: first IPv4 in
  `TailscaleIPs`; a peer with no IPv4 is skipped (Headscale gives each node one).
- `routes` are `net.trunc()` before they reach GraphQL (`scalars::Cidr::new` refuses host bits,
  `scalars.rs:34`).
- `sites`: one `SitePath` for each site, in `sites` order.

**`vpn/warnings.rs`** (about 70): pure.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind { Expiry30, Expiry14, Expiry7, Expiry1, TaggedExpiry, NotInAnima }
impl Kind { pub const fn as_str(self) -> &'static str; }        // 'expiry_30' … 'not_in_anima'
pub(super) struct Warning { pub key: String, pub kind: Kind, pub key_expiry: Option<DateTime<Utc>>,
                            pub device: Option<(DeviceId, String)>, pub severity: Severity, pub message: String }
/// The warnings this picture calls for; the store decides which ones are new.
pub(super) fn due(picture: &Picture, now: DateTime<Utc>) -> Vec<Warning>;
```

Rules (VPN-010, VPN-011):

- `Expiring` or `Expired` untagged key: days = `ceil((expiry − now) / 1 day)`, at least 0. Kind:
  the smallest of 1, 7, 14, 30 that is ≥ days (so after a long stop only the most urgent one is
  written). Severity Warning; 1 day or less → Error. Message:
  `VPN key of {name} expires in {days} days, on {expiry:%Y-%m-%d}` (`expired on …` when 0).
- `TaggedWithExpiry`: kind `TaggedExpiry`, `key_expiry: None` (BD-4: marked once for the node),
  Warning, `VPN node {name} has a tag and a key expiry; tagged keys must not expire (VPN-003)`.
- `in_anima == false`: kind `NotInAnima`, Warning. Router: `VPN router {name} ({address}) serves
  no Anima site`; else `VPN node {name} ({address}) is not in Anima`. `key_expiry` none.
- Includes `picture.hub` for the expiry rules.

**Tests** in `crates/hub/src/vpn/tests.rs` (face) and `vpn/tests/{peers.rs, picture.rs,
warnings.rs}` (A1-T3 adds `store.rs`, A1-T4 adds `reader.rs` and `proxy.rs`: 6 files);
fixtures in `vpn/tests/fixtures/`. These are crate tests, so they reach `pub(super)` items;
`tests/it` cannot. The test clock is
`2030-05-05T12:00:00Z` (as `tests/it/hub.rs` `start()`); fixture expiries are written relative
to it.

| Test | Requirement |
| --- | --- |
| `vpn_008_status_with_no_peers_parses` (`Peer: null`) | VPN-008 |
| `vpn_008_zero_last_seen_is_none` | VPN-008 |
| `vpn_008_unknown_fields_are_ignored` | VPN-008 |
| `vpn_008_host_name_is_cut_to_63_printable_characters` (200 chars, BEL, RLO) | VPN-008 |
| `vpn_008_a_name_of_only_control_characters_becomes_a_question_mark` | VPN-008 |
| `vpn_008_null_tailscale_ips_parse_as_empty` | VPN-008 |
| `vpn_008_the_name_is_the_first_label_of_the_dns_name` | VPN-008, BD-14 |
| `vpn_009_offline_is_unknown_even_with_peer_relay_set` | VPN-009 |
| `vpn_009_a_router_with_its_site_tag_is_a_router` | VPN-009, BD-5 |
| `vpn_009_a_known_path_is_kept_for_three_unknown_reads_then_dropped` (`carry`) | VPN-009, BD-13 |
| `vpn_009_a_paused_ping_check_does_not_count` | VPN-009 |
| `vpn_009_active_with_cur_addr_is_direct` | VPN-009 |
| `vpn_009_active_without_cur_addr_is_relayed_with_region` | VPN-009 |
| `vpn_009_peer_relay_wins` | VPN-009 |
| `vpn_009_idle_is_unknown_even_with_relay_set` (spike fact 3) | VPN-009 |
| `vpn_009_router_without_primary_routes_serves_no_site` | VPN-009, BD-6 |
| `vpn_009_offline_router_gives_offline` | VPN-009 |
| `vpn_009_ping_check_on_the_router_device_is_reported` | VPN-009 |
| `vpn_010_tagged_without_expiry_is_never` | VPN-010 |
| `vpn_010_tagged_with_expiry_is_flagged` | VPN-010 |
| `vpn_010_expiry_in_20_days_is_expiring` | VPN-010 |
| `vpn_010_kind_is_the_smallest_threshold_not_below_the_days_left` (31→none, 30→30, 15→30, 14→14, 8→14, 7→7, 2→7, 1→1, 0→1, −3→1) | VPN-010 |
| `vpn_011_a_peer_matched_by_ip_is_in_anima` | VPN-011 |
| `vpn_011_a_device_whose_host_is_the_magicdns_name_is_matched` (host `NAS.net.anima.ing`, DNSName `nas.net.anima.ing.`) | VPN-011, BD-14 |
| `vpn_011_a_device_whose_host_is_the_first_label_is_matched` (host `nas`) | VPN-011, BD-14 |
| `vpn_011_a_device_with_the_same_label_in_another_domain_is_not_matched` (host `nas.home.arpa` → `in_anima` false) | VPN-011, BD-14 |
| `vpn_011_a_peer_whose_host_name_copies_a_device_is_not_matched` (HostName `nas`, DNSName `evil.net.anima.ing.`) | VPN-011, BD-14 |
| `vpn_011_an_unmatched_client_is_not_in_anima` | VPN-011 |
| `vpn_011_an_agent_node_matched_by_ip_is_in_anima` (R-105) | VPN-011 |

**Commit.** `VPN A1-T2: the pure picture, path, key state and warnings (VPN-008 to VPN-011, VP-1)`.

### A1-T3. Event source, migrations, store

**Migration** `crates/hub/migrations/1000_events_source_vpn.sql`:

```sql
-- The VPN event source (VPN-010, VPN-011, VP-2). Numbered after 0852, which re-adds this check
-- with seven sources; a lower number would let 0852 drop 'vpn' again (K2, BD-1).
alter table events.event_log drop constraint event_log_source_check;

alter table events.event_log
    add constraint event_log_source_check
    check (source in ('check', 'session', 'device', 'access', 'hub', 'metrics', 'notification', 'vpn'));
```

**Migration** `crates/hub/migrations/1001_vpn_schema.sql`:

```sql
-- The vpn domain's tables (K2 range 1000-1019; VPN-010, VPN-011, BD-4).
--
-- One row for each warning event the hub has written, so each is written once, also after a
-- restart. `node_key` is the node's public WireGuard key (not a secret). A renewed key has a new
-- expiry, so its warnings start again. Privileges: migration 0001's default privileges give
-- `anima_runtime` what it needs on the new schema (K2 "Roles").
create schema vpn;

create table vpn.warnings (
    node_key   text        not null check (node_key like 'nodekey:%' and length(node_key) <= 80),
    kind       text        not null check (kind in ('expiry_30', 'expiry_14', 'expiry_7', 'expiry_1',
                                                     'tagged_expiry', 'not_in_anima')),
    key_expiry timestamptz,
    written_at timestamptz not null,
    constraint warnings_once unique nulls not distinct (node_key, kind, key_expiry)
);
```

Check the 0001 default privileges cover INSERT and SELECT on tables in a new schema (0960's
comment says so). If not, add the grant lines that 0960 uses.

**`crates/hub/src/events/model.rs`**: add `Vpn` to `Source` (after `Notifications`), `as_str` →
`"vpn"`, add it to the `from_stored` array. GraphQL name is `VPN` by default. No `match` on
`Source` exists outside `model.rs`. Change by hand: `events/tests.rs:85` `SOURCES: [(Source,
&str); 7]` → 8 with `(Source::Vpn, "vpn")`; a test like `notifications/tests/schema.rs:98` for
the eighth source; `contracts/hub-domains.md:139` (the copy of `Source`).

**K1, same commit (rule 0.9)**: `VPN` in `enum EventSource` (`contracts/schema.graphql:921`),
then codegen (rule 0.8).

**`vpn/store.rs`** (about 40). The only SQL with `vpn.`:

```rust
/// Inserts the mark; true when it is new, so the caller writes the event in the same transaction.
pub(super) async fn mark(conn: &mut PgConnection, warning: &Warning, at: DateTime<Utc>) -> Result<bool, sqlx::Error>;
// insert into vpn.warnings (node_key, kind, key_expiry, written_at) values ($1,$2,$3,$4)
// on conflict on constraint warnings_once do nothing returning 1
```

**Tests** (`#[sqlx::test(migrations = "./migrations")]`, in the crate at
`crates/hub/src/vpn/tests/store.rs`, calling `store::mark` directly; precedent:
`crates/hub/src/metrics/ingest/tests/*.rs`):

- `vpn_010_a_warning_is_marked_once` — `mark` twice → true, false.
- `vpn_010_a_renewed_key_is_marked_again` — same key and kind, new expiry → true.
- `vpn_011_not_in_anima_with_no_expiry_is_marked_once` — `key_expiry` none twice → true, false
  (BD-4).
- `events_source_vpn_is_stored_and_read_back` — `events::record` with `Source::Vpn`, then the
  events query returns `VPN` (in `crates/hub/src/events/tests`, beside the other source tests).

A1-T3 needs `vpn/warnings.rs` (`Warning`) from A1-T2; `store.rs` takes `&Warning`.

**Commit.** `VPN A1-T3: the vpn event source and the warning marks (VPN-010, VPN-011, BD-1, BD-4)`.

### A1-T4. LocalAPI read, reader, feed, wiring

**`vpn/localapi.rs`** (about 45). Pattern: `checks/probe/tailscale.rs` (`client()`,
`read_capped`). Cap `1024 * 1024` (the proxy's own cap). Timeout 10 s on the request
(`Client::builder().timeout(...)`), because no caller cuts this one off.

```rust
pub(super) async fn status(localapi: &str) -> Result<Status, Error>;
// GET http://{localapi}/localapi/v0/status?peers=true ; 200 → parse; 403 → Refused; other → Status(code)
```

**`vpn/feed.rs`** (about 30):

```rust
#[derive(Clone, Debug)]
pub struct Feed { sender: Arc<watch::Sender<Option<Arc<Picture>>>> }
impl Feed {
    #[must_use] pub fn new() -> Self;
    /// The latest picture; None before the first read or when the proxy is not set.
    #[must_use] pub fn latest(&self) -> Option<Arc<Picture>>;
    pub(super) fn publish(&self, picture: Picture);
    #[must_use] pub fn subscribe(&self) -> watch::Receiver<Option<Arc<Picture>>>;
}
impl Default for Feed { fn default() -> Self { Self::new() } }
```

`publish` calls `sender.send_replace(Some(Arc::new(picture)))`. Never `send`: it fails and
stores nothing when no receiver exists, and `new()` drops the first receiver. `latest` is
`sender.borrow().clone()`. Unit test in `vpn/tests.rs`:
`vpn_008_a_picture_published_with_no_subscriber_is_the_latest`.

**`vpn/reader.rs`** (about 110):

```rust
/// Reads the hub's VPN status each 60 s and publishes the picture (VPN-008); None when the
/// LocalAPI proxy is not set (the vpn query then answers null).
pub fn spawn_reader(localapi: Option<String>, pool: PgPool, bus: Bus, clock: Arc<dyn Clock>, feed: Feed)
    -> Option<tokio::task::JoinHandle<()>>;
```

That is 5 parameters (limit 4). Group them: `pub struct Reading { pub pool: PgPool, pub bus: Bus,
pub clock: Arc<dyn Clock>, pub feed: Feed }` and `spawn_reader(localapi: Option<String>, reading:
Reading)`. `Reading` is in `reader.rs`, re-exported from the face.

Inside:

```rust
const INTERVAL: Duration = Duration::from_secs(60);
async fn read_once(localapi: &str, reading: &Reading, last_failed: &mut bool);
pub(super) async fn build_picture(status: &Status, pool: &PgPool, now: DateTime<Utc>) -> Result<Picture, Error>;
pub(super) async fn write_warnings(pool: &PgPool, bus: &Bus, warnings: &[Warning], now: DateTime<Utc>) -> Result<(), Error>;
```

- `build_picture`: `devices::list(pool)`, `devices::sites(pool)` (new, below),
  `picture::router_devices(status, &sites, &devices)`, then
  `checks::for_devices(pool, &router_device_ids)` filtered by
  `check.config.kind() == CheckType::TailscalePing && !check.paused` → `ping_checked`, then one
  `picture::build`, then `picture::carry(previous, built)` when a previous picture exists.
- `write_warnings`: one transaction; for each warning: `store::mark`; if new,
  `events::record(&mut tx, bus, now, NewEvent { severity, source: Source::Vpn, device, message })`.
  `device`: `(id, device name)` from the picture's device list (keep the name in `Warning`).
- `read_once`: `localapi::status` → `build_picture` → `feed.publish` → `warnings::due` →
  `write_warnings`. On a failure of the read, of `build_picture` or of the database: if a
  picture exists, publish a copy with `fresh = false` and the old `read_at`. A `write_warnings`
  failure does not undo the publish; the marks are not written, so the next read tries again.
  Each failure kind logs with its own `fault()` word. Log only on a change of state:
  `tracing::warn!(job = "vpn_reader", fault = error.fault(), "VPN status read failed")` on the
  first failure, `tracing::info!(job = "vpn_reader", "VPN status read again")` on recovery
  (LOG-007; at most two lines for an outage, not 1,440 a day).
- Loop: `tokio::time::interval(INTERVAL)` with `MissedTickBehavior::Delay`; the first tick is at
  once.

**`crates/hub/src/devices.rs`**: add

```rust
/// Every site, in name order (VPN-009).
pub async fn sites(db: &PgPool) -> Result<Vec<Site>, Error> { store::list_sites(db).await }
```

(`list_sites` is `pub(in crate::devices)`; the face is inside `devices`, so this compiles.)

**`crates/hub/src/schema.rs`**: `Dependencies` gets `pub vpn_feed: vpn::Feed,` after
`metrics_feed`. The destructure at line 105 lists it. Add a block `// vpn` then
`.data(vpn_feed)`. (The `Query` and `Subscription` tuples change in A1-T5.)

**`crates/hub/src/main.rs`**: one line in the literal: `vpn_feed: vpn::Feed::new(),` and the
import. (BD-3 made room.)

**`crates/hub/src/upkeep.rs`**: at the end of `spawn`:

```rust
let reading = vpn::Reading { pool: pool.clone(), bus: bus.clone(), clock: clock.clone(), feed: dependencies.vpn_feed.clone() };
jobs.extend(vpn::spawn_reader(dependencies.config.tailscale_localapi.clone(), reading));
```

**`crates/hub/tests/it/hub.rs`**: `dependencies()` literal gets `vpn_feed: vpn::Feed::new(),`.
The four `..dependencies(..)` users need no change.

**`crates/hub/src/lib.rs`**: `pub mod vpn;` after `pub mod trust;`.

**Tests.** No `tokio::time::pause` anywhere: a paused clock with real sockets and a sqlx pool
fires the reqwest and pool timeouts early. No test-only seam either.

Crate tests, `crates/hub/src/vpn/tests/` (`#[sqlx::test(migrations = "./migrations")]`):

- `proxy.rs`: a fake LocalAPI proxy, the same shape as `checks/tests/tailscale/proxy.rs`
  (`start(answer) -> Proxy`, `Reply::json`, `Hang`, `seen()`), reduced to GET. About 50 lines.
  The answer closure may count calls (for "ok, then 500").
- `reader.rs` — each test calls `read_once(&proxy.address(), &reading, &mut failed)` directly;
  `reading.clock` is a `FixedClock`:
  - `vpn_008_without_a_proxy_no_reader_starts` — `spawn_reader(None, …)` is `None`;
    `feed.latest()` is `None`.
  - `vpn_008_a_read_publishes_a_picture` — `status-active.json` → router `Direct`.
  - `vpn_008_a_failed_read_keeps_the_picture_and_marks_it_not_fresh` — ok, then 500.
  - `vpn_008_the_reader_asks_only_for_status` — every `proxy.seen()` head starts with
    `GET /localapi/v0/status?peers=true `.
  - `vpn_010_a_warning_is_written_once_across_a_restart` — two `Reading`s (two `Feed`s) on one
    pool → one event row with source `vpn` for the expiring client.
  - `vpn_010_a_29_day_key_then_a_6_day_key_gives_two_events` — two `Reading`s with two
    `FixedClock`s.
  - `vpn_008_the_loop_publishes_without_a_tick_wait` — `spawn_reader(Some(addr), reading)`, then
    `tokio::time::timeout(5 s, receiver.wait_for(Option::is_some))`. The first `interval` tick
    is at once, so no time must pass.

Integration tests: `crates/hub/tests/it/main.rs` gets `mod vpn;` before `mod web_client;`.
`tests/it/vpn.rs` (face: `mod proxy; mod graphql;`) and `tests/it/vpn/`:

- `proxy.rs`: a copy of the crate test proxy (the first comment says why: `tests/it` is another
  crate and cannot import `#[cfg(test)]` code).
- `graphql.rs` — the A1-T5 tests, and `vpn_008_no_log_line_holds_status_text`: make
  `struct Capture` in `tests/it/logs.rs:22` `pub`; use `process_log::subscriber("info", …)` with
  `tracing::subscriber::set_default` on the current-thread test runtime; drive `spawn_reader`
  with a proxy that answers once ok and then 500; wait as above; then no captured line contains
  the fixture's names, `nodekey:` or `100.64.`.

**Commit.** `VPN A1-T4: the hub reads its VPN status each minute (VPN-008, VPN-010, VPN-011, BD-2, BD-3)`.

### A1-T5. GraphQL and K1

**K1 delta** — `contracts/schema.graphql` (`EventSource` `VPN` is already in, from A1-T3). Add
after the `Site` type section (no `@pending`; the hub declares it in the same commit), then
codegen (rule 0.8):

```graphql
"""The VPN as the hub's own VPN node sees it (VPN-005, VPN-008)."""
type VpnPicture {
  "When the hub last read its VPN status."
  readAt: DateTime!
  "False when the last read failed; the picture is then from readAt."
  fresh: Boolean!
  hub: VpnNode!
  "One entry for each site, in site name order (VPN-009)."
  sites: [SitePath!]!
  "Every peer of the hub, in name order. A node that is not a peer of the hub is not here (VPN-011)."
  nodes: [VpnNode!]!
}

"""How the hub reaches one site (VPN-009)."""
type SitePath {
  site: Site!
  "The subnet router that serves the site subnet, or null."
  router: VpnNode
  state: SiteRouteState!
  path: VpnPath!
  "The DERP region, when the path is RELAYED."
  relayRegion: String
  "True when the router's device has a Tailscale ping check, which keeps the path known."
  hasPingCheck: Boolean!
}

enum SiteRouteState { SERVING OFFLINE NO_ROUTER }
enum VpnPath { DIRECT PEER_RELAY RELAYED UNKNOWN }
enum VpnRole { HUB ROUTER AGENT CLIENT OTHER }
enum KeyState { NEVER VALID EXPIRING EXPIRED TAGGED_WITH_EXPIRY }

"""One node of the VPN. Name and OS come from the node and are cleaned (VPN-008)."""
type VpnNode {
  name: String!
  address: IpAddr!
  os: String!
  role: VpnRole!
  tags: [String!]!
  "The subnet routes Headscale lets this node serve."
  routes: [Cidr!]!
  online: Boolean!
  lastSeen: DateTime
  keyExpiry: DateTime
  keyState: KeyState!
  path: VpnPath!
  relayRegion: String
  "The Anima device on this node, or null."
  device: Device
  inAnima: Boolean!
}
```

Inside `type Query` (line 1501): `"The VPN picture; null when the hub has no LocalAPI proxy or has not read it yet (VPN-005)." vpn: VpnPicture`.
Inside `type Subscription` (line 1643): `"Each new VPN picture, about once a minute." vpnChanged: VpnPicture!`.
No `extend` (rule 0.9). Both are open to API keys (read only), so no D-23 change.

`SitePath.path` and `relayRegion` repeat the router's; they are null-safe for a site with no
router (`path: UNKNOWN`).

**`vpn/graphql.rs`** (about 150):

```rust
#[derive(Default)] pub struct Query;
#[Object(name = "VpnQuery")] impl Query {
    #[graphql(guard = "Authenticated")]
    async fn vpn(&self, ctx: &Context<'_>) -> Result<Option<PictureView>>;
}
#[derive(Default)] pub struct Subscription;
#[Subscription(name = "VpnSubscription")] impl Subscription {
    #[graphql(guard = "Authenticated")]
    async fn vpn_changed(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = PictureView>>;
}
```

- `PictureView(Arc<Picture>)`, `SitePathView { picture: Arc<Picture>, index: usize }`,
  `NodeView { picture: Arc<Picture>, index: Option<usize> /* None = hub */ }` — `#[Object(name =
  "VpnPicture")]` etc. They borrow from the `Arc`, so a picture is never copied for each field.
- `VpnPicture.sites`: one `load_many` on `DataLoader<devices::SitesById>` for all site ids;
  each `SitePathView` holds its loaded `Site`; a site missing from the result (deleted between
  reads) is left out of the list, not an error.
- `VpnNode.device`: `DataLoader<devices::DevicesById>` with `device.0`; missing → null.
- `vpnChanged`: `stream::unfold(feed.subscribe(), …)`: `changed().await`, then
  `borrow_and_update().clone()`; on `Err` (sender gone) end the stream; skip `None`. Pattern:
  `metrics/graphql.rs` `added()`.
- `VpnNode.key` is not a field (public key, but not useful to the client and long).

**`crates/hub/src/schema.rs`**: `vpn::graphql::Query` at the end of the `Query` tuple,
`vpn::graphql::Subscription` at the end of the `Subscription` tuple.

**Integration tests** `tests/it/vpn/graphql.rs`. Each builds `Dependencies { vpn_feed:
feed.clone(), ..dependencies(..) }`, starts `spawn_reader` with the fake proxy, waits with
`timeout(5 s, receiver.wait_for(Option::is_some))`, then queries:

- `vpn_005_vpn_is_null_without_a_proxy`.
- `vpn_009_vpn_returns_each_site_with_router_state_and_path` — create two sites (`home`
  10.10.0.0/24, `office1` 10.20.0.0/24) and a device for the router IP with a Tailscale ping
  check; fixture `status-active.json` → `home`: `SERVING`, `DIRECT`, `hasPingCheck: true`;
  `office1`: `NO_ROUTER`, `UNKNOWN`.
- `vpn_011_a_peer_with_a_device_resolves_device` — `nodes[*].device.id`.
- `vpn_005_vpn_changed_sends_the_next_picture` — `schema.execute_stream(...)` with
  `Identity::for_tests`, as `tests/it/metrics/subscription.rs` does (no WebSocket); poll once so
  it subscribes, then start the reader, then receive one.
- `vpn_005_an_api_key_may_read_vpn` (read-only, D-23 unchanged).
- The contract test (`tests/it/contract`) passes: hub SDL matches K1.

**Commit.** `VPN A1-T5: the vpn query and vpnChanged subscription (VPN-005, VPN-009, K1, VP-5)`.

### A2-T1. Site filter (VPN-002)

**`apps/client/src/lib/devices/query.ts`**: `ListQuery` gets `readonly site: string | null`
(a site id). `DEFAULT_QUERY.site = null`. `parseListQuery`: `site: params.get('site') || null`.
`writeListQuery`: after the tags, `if (query.site !== null) params.set('site', query.site)`.
`filterOf`: `...(query.site === null ? {} : { siteId: query.site })`. The order comment becomes
"(view, q, status, tag, site, sort)".

**`query.test.ts`** line 18: the pinned URL gets `&site=s1` before `&sort=`. New test:
`it('VPN-002: an empty site parameter is no site filter', …)`.

**`toolbar/toolbar.ts`**: `isFiltered` adds `|| query.site !== null`; `withoutFilters` sets
`site: null`. Add:

```ts
/** The Site menu: "All sites" first, then each site by name (VPN-002). */
export function siteOptions(sites: readonly { id: string; name: string }[]): readonly MenuOption[];
export const ALL_SITES = '';
```

**`toolbar/sites.svelte.ts`** (new; same shape as `tags.svelte.ts`): `class Sites { options:
readonly MenuOption[]; error; load(); get note() }`, using `devicesApi(hub).sites()`.
`options` is `[]` while loading, after a failure and when there is no site; else
`siteOptions(sites)` (with "All sites" first). So `empty={sitesNote}` shows when it should.

**`Toolbar.svelte`**: props add `sites: readonly MenuOption[]; sitesNote: string`. A `Menu`
after Tags: `label="Site" icon="nf-md-lan_connect" kind="radio" options={sites}
chosen={[query.site ?? ALL_SITES]} empty={sitesNote} onChoose={(value) => { onQueryChange({
...query, site: value === ALL_SITES ? null : value }); }}`. `note` is the error message, or
"No sites yet".

**`DeviceScreen.svelte`**: `const sites = new Sites(hub)`, `$effect(() => { void sites.load(); })`,
pass `sites={sites.options} sitesNote={sites.note}`.

**Tests** — `toolbar.test.ts`: `VPN-002: the site menu lists All sites then each site`;
`menus.test.ts`: `VPN-002: choosing a site sets the site filter; All sites clears it`;
`screen/page.test.ts`: `VPN-002: the device list asks the hub with siteId` (the `Devices`
request variables hold `filter.siteId`); `VPN-002: Clear filters clears the site`.

Folder `lib/devices/toolbar` 6 → 7.

**Commit.** `VPN A2-T1: filter the device list by site (VPN-002)`.

### A2-T2. Events source and the Settings VPN section (SET-003)

**Codegen.** Already done in A1-T3 and A1-T5 (rule 0.8).

**`lib/events/panel/view.ts`**: add `{ value: 'VPN', label: 'VPN' }` after `HUB`. (METRICS and
NOTIFICATION are missing too; that is a separate fix, issue #1.)

**New folder `apps/client/src/lib/vpn/`** (7 counted files at the end of A2-T3; sub-folders
are counted on their own):

| File | Content |
| --- | --- |
| `api.ts` | Documents and `vpnApi(hub)`, pattern `lib/events/api.ts`. |
| `picture.ts` | Pure text and order. |
| `picture.test.ts` | |
| `live.svelte.ts` | `class VpnLive`: load + subscribe + state. |
| `vpn.fixture.ts` | A picture for tests; uses `fakeHub` from `$lib/devices/hub.fixture`. |
| `VpnSection.svelte` | The Settings section body. |
| `section.svelte.test.ts` | |
| `sites/SitePaths.svelte`, `sites/sites.svelte.test.ts` | One row for each site. |
| `keys/NodeKeys.svelte`, `keys/keys.svelte.test.ts` | Nodes and key expiry. |
| `device/DeviceVpn.svelte`, `device/device.svelte.test.ts` | A2-T3. |

**`api.ts`**:

```ts
void graphql(`fragment VpnNodeFields on VpnNode { name address os role tags routes online lastSeen keyExpiry keyState path relayRegion inAnima device { id name } }`);
void graphql(`fragment VpnPictureFields on VpnPicture { readAt fresh hub { ...VpnNodeFields } sites { site { id name subnet } router { ...VpnNodeFields } state path relayRegion hasPingCheck } nodes { ...VpnNodeFields } }`);
const Vpn = graphql(`query Vpn { vpn { ...VpnPictureFields } }`);
const VpnChanged = graphql(`subscription VpnChanged { vpnChanged { ...VpnPictureFields } }`);
export type VpnPicture = NonNullable<VpnQuery['vpn']>;
export type VpnNode = VpnPicture['nodes'][number];
export type SitePath = VpnPicture['sites'][number];
export function vpnApi(hub: Hub): {
  picture(): Promise<HubResult<VpnPicture | null>>;
  watch(sink: Sink<VpnPicture>): () => void;
};
```

**`picture.ts`** (pure; `now` is a parameter):

```ts
export function pathLabel(path: VpnPath, region: string | null): string;   // 'Direct', 'Peer relay', 'Relayed (sfo)', 'Not known'
export function stateLabel(state: SiteRouteState): string;                  // 'Serving', 'Router offline', 'No router'
export function keyText(node: Pick<VpnNode, 'keyState' | 'keyExpiry'>, now: Date): string;
// NEVER 'Does not expire'; VALID 'Expires on 12 May 2027'; EXPIRING 'Expires in 9 days'; EXPIRED 'Expired 3 days ago';
// TAGGED_WITH_EXPIRY 'Tagged, but expires on …: renew it as a tagged node'
export function keyTone(state: KeyState): 'up' | 'warning' | 'down';
export function sortedNodes(nodes: readonly VpnNode[]): readonly VpnNode[];  // not in Anima first, then expiring, then name
export function readAgo(readAt: string, now: Date): string;                // 'Read 40 s ago'
```

Dates use `Intl.DateTimeFormat(undefined, { dateStyle: 'medium' })`, as the neighbours do (check
`lib/retention` or `lib/api-keys` and use the same helper if one exists).

**`live.svelte.ts`**:

```ts
export class VpnLive {
  picture = $state<VpnPicture | null | undefined>(undefined);   // undefined: loading; null: off
  error = $state<HubError | null>(null);
  constructor(private readonly api: ReturnType<typeof vpnApi>) {}
  async load(): Promise<void>;         // picture(); on ok sets picture, then starts watch() once
  stop(): void;                        // unsubscribes
}
```

**`VpnSection.svelte`**: `const live = new VpnLive(vpnApi(useHub()))`;
`$effect(() => { void live.load(); return () => { live.stop(); }; })`. States:

- loading: placeholder rows (`aria-hidden="true" inert`), same height as one site row and one
  node row (MOT-007);
- `null`: one `SettingRow` "VPN status" with "VPN status is not available. The hub has no
  LocalAPI proxy (ANIMA_TAILSCALE_LOCALAPI), or it has not answered since the hub started."
  (SET-003 text, T0);
- error: the message and a Retry `FormButton`, as `RetentionSection`;
- picture: `<SitePaths sites={…} now />`, `<NodeKeys nodes={…} hub={…} now />`, and when
  `fresh` is false a line "Last read {readAgo}" with `role="status"`.

`now`: a `$state(new Date())` refreshed each 30 s by an `$effect` interval, passed down.

**`SitePaths.svelte`**: `<ul aria-label="Sites">`, each `<li transition:listRow={{ index }}
aria-label={site.site.name}>`: site name, subnet, a `StatusDot`-like dot plus `stateLabel`,
`pathLabel`. When `state === 'SERVING' && !hasPingCheck`: a link "Add a ping check" to
`/devices/{router.device.id}?tab=checks` (tabs are a query parameter,
`routes/devices/[id]/tabs/tabs.ts:54`). When `router.device` is null: text "The router is not
a device in Anima".

**`NodeKeys.svelte`**: `<ul aria-label="VPN nodes">`, one row for each node in `sortedNodes`
plus the hub first: name, role, address, `keyText`, a tone dot (`--status-up`,
`--status-warning`, `--status-down`) beside the word, and "Not in Anima" as text when
`!inAnima`. `listRow` transitions. No buttons in Tier A.

**`routes/settings/+page.svelte`**: after the Hub section, `<SettingsSection id="settings-vpn"
title="VPN" icon="nf-md-lan_connect" description="How the hub reaches each site, and when each
VPN key expires."><VpnSection /></SettingsSection>`. `routes/settings/page.test.ts`: add
`vi.mock('$lib/vpn/VpnSection.svelte', …)` beside the other section mocks (lines 27–33); the
section list in order includes `VPN`.

**Tests** (jsdom, `fakeHub`, pattern `lib/retention/section.svelte.test.ts`):

| Test | Checks |
| --- | --- |
| `SET-003: with no proxy the section says VPN status is off` | `Vpn: { vpn: null }`. |
| `SET-003, VPN-009: each site shows its state and path as text` | text, not only a colour class (UI-003). |
| `VPN-009: a serving site without a ping check offers one` | |
| `VPN-010: keys are listed with their expiry text` | fixed `now`. |
| `VPN-011: a node not in Anima is marked and listed first` | |
| `SET-003: a new picture from vpnChanged replaces the shown one` | `hub.push('VpnChanged', …)`. |
| `SET-003: a failed load shows Retry, and Retry loads again` | |
| `MOT-040: under reduced motion the rows fade and do not move` | pattern `lib/forms/modal/reduced.svelte.test.ts`. |
| `picture.test.ts`: one test for each function and each enum value | |

**Commit.** `VPN A2-T2: the Settings VPN section and the VPN event source (SET-003, VPN-009 to VPN-011)`.

### A2-T3. Device Overview facts

**`lib/vpn/device/DeviceVpn.svelte`**: props `{ node: VpnNode; now: Date }`. A card in the
`overview-card` form (`<section class="overview-card" aria-labelledby="{id}-vpn"><h2 …><Icon
name="nf-md-lan_connect" />VPN</h2><dl class="facts">`): Address, Path (`pathLabel`), Key
(`keyText`), OS. Classes come from `overview/geometry.css` (global).

**`routes/devices/[id]/overview/facts/OverviewFacts.svelte`**: no change. Instead
**`OverviewPanel.svelte`** gets an optional prop `vpn?: VpnNode | null` and renders
`{#if vpn}<DeviceVpn node={vpn} now={now()} />{/if}` between `OverviewFacts` and
`MetricsSection` (the panel's `now` is `() => Date`). `detail/DetailBody.svelte` owns the
`Detail` query (202 code lines); it creates one `VpnLive` and passes
`vpn = live.picture?.nodes.find((node) => node.device?.id === device.id) ?? null`. One
subscription for the page, not one per card.

**Tests** `device/device.svelte.test.ts`: `VPN-005: the device Overview shows address, path and
key expiry`; `VPN-005: a device with no VPN node shows no VPN card`.

Folder `overview` stays 9; `lib/vpn` has 7 counted files (`api.ts`, `picture.ts`,
`picture.test.ts`, `live.svelte.ts`, `vpn.fixture.ts`, `VpnSection.svelte`,
`section.svelte.test.ts`).

**Commit.** `VPN A2-T3: VPN facts on the device Overview (VPN-005)`.

---

## 7. Tier B tasks

### B1. Broker core (pure)

**Folder** `deploy/headscale/broker/`. Each file ≤ 300 code lines, each function ≤ 60 lines,
≤ 4 parameters (`eslint.config.js:8-13`, applied to `deploy/**/*.ts`).

**`broker/protocol.ts`** (about 60): the wire types and the request parser.

```ts
export type Request =
  | { readonly verb: 'nodes' }
  | { readonly verb: 'personal-key' }
  | { readonly verb: 'router-key'; readonly site: string }
  | { readonly verb: 'approve-route'; readonly node: number; readonly site: string }
  | { readonly verb: 'register'; readonly authId: string };
export type Refusal = 'bad-request' | 'unknown-site' | 'rate-limited' | 'node-limit' | 'not-allowed' | 'undone' | 'headscale-failed';
export type Reply<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly refusal: Refusal; readonly detail: string };
/** Parses one request line; bad-request for anything else. Max 1 KiB. */
export function parseRequest(line: string): Request | Refusal;
```

Validation: `site` matches `^[a-z0-9-]{1,32}$`; `node` passes `Number.isSafeInteger` and
`0 < node < 2^31`; `authId` matches `^[A-Za-z0-9][A-Za-z0-9_-]{7,63}$` (no leading `-`; A4 step
4 records a real ID; if it does not match, change the pattern here and in the test). Keys are
read with `Object.hasOwn`; an unknown key → `bad-request`.

**`broker/nodes.ts`** (about 80): the broker's view of Headscale. Reuses
`src/inventory.ts` `parseNodes`/`parseRoutes` (BD-9: they drop every key).

```ts
export interface BrokerNode { id: number; name: string; user: string; tags: string[]; ips: string[];
  online: boolean; expiry: string | null; approved: string[]; available: string[]; }
export interface NodesReply { nodes: BrokerNode[]; sites: { name: string; subnet: string }[]; }
export function nodesReply(tailnet: Tailnet, policy: PolicyView): NodesReply;
```

`preAuthKeyId` (in `ListedNode`) stays inside the broker for `audit`; it is not in the reply.

**`broker/verbs.ts`** (about 120): request → argv. Pure.

```ts
export interface PolicyView { readonly sites: ReadonlyMap<string, string> /* x → subnet of host "site-<x>" */; readonly tags: ReadonlySet<string>; }
export function policyView(policy: Policy): PolicyView;     // from policy.scope.hosts and policy.scope.tags
export type Step = readonly string[];   // argv after `headscale`; one element for each flag=value
export function plan(request: Request, policy: PolicyView, nodes: readonly BrokerNode[]): readonly Step[] | Refusal;
```

| Verb | Steps | Refusal when |
| --- | --- | --- |
| `nodes` | `['nodes','list','-o','json']`, `['nodes','list-routes','-o','json']` | — |
| `personal-key` | `['preauthkeys','create','--user=operator','--expiration=1h','-o','json']` | — |
| `router-key` | `['preauthkeys','create','--tags=tag:router,tag:site-<site>','--expiration=1h','-o','json']` | `site` not in `policy.sites`, or `tag:site-<site>` not in `policy.tags` (BD-9) |
| `approve-route` | `['nodes','approve-routes','--identifier=<node>','--routes=<subnet>']` | node absent; node lacks `tag:site-<site>`; `available` ≠ exactly `[subnet]` |
| `register` | `['auth','register','--user=operator','--auth-id=<authId>','-o','json']` | — |

Each flag and its value are one argv element (`--auth-id=<id>`), so a value can never be read
as a flag. No step has `--reusable` or `--ephemeral`. A4 records the 0.29.7 flag names and
reply shapes (`preauthkey.json`, `register.json`); a test checks each argv against them.

**`broker/limits.ts`** (about 60):

```ts
export interface LimitState { readonly keys: readonly string[]; readonly registrations: readonly string[]; } // ISO times
export interface LimitConfig { readonly perDay: number /* 3 */; readonly maxNodes: number; }
export interface Usage { readonly state: LimitState; readonly nodeCount: number; }
export function admit(verb: Request['verb'], usage: Usage, config: LimitConfig, now: Date): 'ok' | 'rate-limited' | 'node-limit';
export function record(verb: Request['verb'], state: LimitState, now: Date): LimitState;  // drops entries older than 24 h
```

`personal-key` and `router-key` share the `keys` count. `nodes` and `approve-route` are not
limited. `node-limit` when `nodeCount >= maxNodes` for the three creating verbs.

**`broker/outcome.ts`** (about 100):

```ts
export interface Issued { readonly keyId: number; readonly verb: 'personal-key' | 'router-key'; readonly site?: string; readonly at: string; readonly passed: boolean; }
export type Fix = { readonly kind: 'delete'; readonly node: number; readonly reason: string }
                | { readonly kind: 'set-expiry'; readonly node: number; readonly expiry: Date };
export interface Audit { readonly fixes: readonly Fix[]; readonly issued: readonly Issued[] /* updated */; }
/** Checks the nodes that joined with keys this broker issued, and the node just registered. */
export function audit(tailnet: Tailnet, issued: readonly Issued[], registered: number | null, now: Date): Audit;
export function staggered(now: Date, nodes: readonly ListedNode[]): Date;
```

Rules:

- A node is linked to an `Issued` entry by `ListedNode.preAuthKeyId === keyId`.
- An entry with `passed: true` is never checked again. An entry whose node passes the check
  gets `passed: true`. An entry with no node is dropped 2 hours after `at`. So a later operator
  change with `hsctl` (new tags on a router) is never undone by the broker.
- A node from a `personal-key` entry, or the `registered` node: tags must be empty, else
  `delete`. Expiry null or more than 180 days after `now` → `set-expiry` to `staggered`.
- A node from a `router-key` entry: tags, sorted, must equal `['tag:router',
  'tag:site-<site>']`, and `expiry` null; else `delete`.
- `staggered`: try `now + 180 d`, then 150, 120, 90 days; take the first that is not within
  30 days of the expiry of another untagged `operator` node; 90 days if none (vpn.md 6B step 5).
- Fix steps: `['nodes','delete','--identifier=<id>','--force']`,
  `['nodes','expire','--identifier=<id>','--expiry=<iso>']`. A test checks the expiry value is
  never empty (an empty `--expiry` expires the node at once, vpn.md 6B step 4).

**Tests** `deploy/headscale/tests/broker-*.test.ts` (`deploy/` is not folder-measured):

| File | Tests |
| --- | --- |
| `broker-protocol.test.ts` | `VP-7: only the five verbs parse`; a table of 15 bad-request rows (long site, upper case, `../`, negative node, float node, 2^31, authId with a space, authId starting with `-`, extra key, `__proto__` key, 1 KiB + 1 line, …). |
| `broker-verbs.test.ts` | One argv test for each verb, against the A4 shapes; `BD-9: a site with no tag in the policy is refused before Headscale`; `R-055: approve-route refuses a node of another site / with two routes / with no route`; `no step holds --reusable or --ephemeral`; `every value is in the same argv element as its flag`. |
| `broker-limits.test.ts` | `5.1: the 4th key in 24 h is refused, also mixing personal and router keys`; `5.1: a key 24 h + 1 s old no longer counts`; `5.1: node-limit refuses at the limit`; `nodes and approve-route are never limited`. |
| `broker-outcome.test.ts` | `VPN-003: a personal node with a tag is deleted`; `a personal node with no expiry gets one 180 days out`; `a second Mac gets 150 days, a third 120 (stagger)`; `a router node with an extra tag is deleted`; `a router node with an expiry is deleted`; `a node that passed once is not checked again`; `an issued key with no node is dropped after 2 hours`; `nodes from keys the broker did not issue are left alone`; `the expiry value is never empty`. |

**Commit.** `VPN B1: the broker's verbs, limits and outcome checks (R-055, VP-7, BD-9)`.

### B2. Broker process, units and install

**`broker/main.ts`** (about 140): the I/O edge.

- Entry points: no argument → serve one request; `count-nodes` → print the node count;
  `check` → connect to `/run/anima-broker/broker.sock`, send `{"verb":"nodes"}`, print the
  node count or the refusal (the install uses it; no `socat`, no `jq`).
- Serve: read one line (≤ 1 KiB, 5 s limit) from stdin (the socket, systemd `Accept=yes`),
  write one JSON line to stdout, exit. EOF with no line → exit with no reply.
- Config `/etc/anima/broker.conf` (`max_nodes=<n>`; nothing else). Policy
  `/opt/anima/headscale/config/policy.hujson`. State `/var/lib/anima-broker/state.json`
  (`{ keys, registrations, issued, pendingRegister }`), written to a temp file then `rename`.
  `MaxConnections=1` makes one process at a time, so no lock is needed.
- `headscale(step)`: `Bun.spawn(['/usr/bin/docker','compose','-f',
  '/opt/anima/headscale/compose.yaml','exec','-T','headscale','headscale',...step],
  { stdin: 'ignore', env: { PATH: '/usr/bin:/bin', DOCKER_CONFIG: '/var/lib/anima-broker/docker' } })`,
  8 s limit for each step, stdout cap 1 MiB, stderr discarded (it can echo arguments). The
  request has a 45 s deadline (`RuntimeMaxSec=60` is the outer limit).
- Order for each request:
  1. If `pendingRegister` is set (a killed request), audit that node first and clear it.
  2. `parseRequest`; read the nodes (2 steps); `audit` and apply its fixes; save `issued`.
  3. `admit`; on ok, `record` and **write the state before the step** (a kill after the step
     cannot leave a key that was not counted).
  4. `plan`; run the steps.
  5. Key verbs: read `{ key, id, expiration }` from the reply (shape from `preauthkey.json`);
     append `Issued { keyId: id, … }`; write the state; reply `{ key, expiresAt, keyId }`.
  6. `register`: read the node id **from the `auth register` reply** (shape from
     `register.json`); no id → reply `headscale-failed`. Write `pendingRegister`; read the
     nodes; `audit` with `registered` = that id; apply fixes; clear `pendingRegister`; write the
     state. If the node was deleted, reply `undone`; else reply the node
     `{ name, address, keyExpiry }`. Never infer the node from a list difference.
- The key is read from Headscale's JSON reply and written only to stdout.
- Log: one line to stderr (journald) for each request:
  `anima-broker verb=<verb> outcome=<ok|refusal> node=<id|-> fixes=<n>`. Never the key, the auth
  ID or a node name.
- `serve(line, runner, files, now)` is the core; `runner` is the `headscale` function, so tests
  pass a fake. (A parameter, not an interface.)

**Units** `deploy/headscale/broker/anima-broker.socket` and `anima-broker@.service`:

```ini
# anima-broker.socket
[Unit]
Description=Anima broker socket (R-055, VP-8)
[Socket]
ListenStream=/run/anima-broker/broker.sock
SocketUser=10001
SocketGroup=10001
SocketMode=0600
DirectoryMode=0755
Accept=yes
MaxConnections=1
[Install]
WantedBy=sockets.target
```

```ini
# anima-broker@.service
[Unit]
Description=Anima broker request
[Service]
ExecStart=/usr/local/libexec/anima-broker
StandardInput=socket
StandardOutput=socket
StandardError=journal
User=root
StateDirectory=anima-broker
StateDirectoryMode=0700
Environment=DOCKER_CONFIG=/var/lib/anima-broker/docker
RuntimeMaxSec=60
UMask=0077
LimitCORE=0
NoNewPrivileges=yes
CapabilityBoundingSet=
AmbientCapabilities=
PrivateNetwork=yes
RestrictAddressFamilies=AF_UNIX
IPAddressDeny=any
PrivateDevices=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadOnlyPaths=/opt/anima/headscale
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectKernelLogs=yes
ProtectControlGroups=yes
ProtectClock=yes
ProtectHostname=yes
RestrictNamespaces=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes
LockPersonality=yes
SystemCallArchitectures=native
```

Notes: `/run/docker.sock` is a file-system socket, so `PrivateNetwork`, `AF_UNIX` only and an
empty capability set still let root connect, and `ProtectSystem=strict` does not block
`connect()`. `StateDirectory=` makes `/var/lib/anima-broker` writable (no `ReadWritePaths`).
`MemoryDenyWriteExecute` is left out (Bun has a JIT). `SystemCallFilter` is left out until a
run on a Linux VM shows the binary and `docker compose` work with `@system-service`. 10001 is
the hub container's user (`deploy/hub/Dockerfile:40`). `LimitCORE=0` keeps a key out of core
dumps.

**Build** `deploy/headscale/bin/broker-build.ts` (about 20): `bun build
deploy/headscale/broker/main.ts --compile --target=bun-linux-x64-baseline --outfile
deploy/headscale/broker/dist/anima-broker` (an `arm64` argument gives
`bun-linux-arm64`). The first cross-compile downloads the Bun runtime (network). `dist/` is
already in `.gitignore:7`.

**Install** `deploy/headscale/vps/broker.sh`, sourced by `harden.sh`, adds `harden.sh broker`
(one command, VP-2):

1. `require_root`. The operator copies the binary to `/usr/local/libexec/anima-broker.new` with
   `scp` first (the README says how). Missing → refuse; under `--dry-run`, print it and go on.
2. `uname -m` must be `x86_64` (or `aarch64` for the arm64 build); else refuse.
3. `/etc/anima/broker.conf` only if absent: `max_nodes=` (`anima-broker.new count-nodes`) + 5.
4. `install -m 0755` the binary to `/usr/local/libexec/anima-broker`; `write_file` the two
   units (mode 0644); `systemctl daemon-reload`; `systemctl enable --now anima-broker.socket`.
5. `anima-broker check`; print the node count.
6. `--dry-run` prints each step and changes nothing, as the other `cmd_*`.

`harden.sh` `usage()` and the `case` at line 303 get `broker`. `tests/vps-cli.test.ts:14` (every
command with `--dry-run` exits 0) and `:41` (the shellcheck list) get `broker` and `broker.sh`.
New `tests/vps-broker.test.ts` with `vps-host.ts`'s fake root: dry run changes nothing; a
missing binary is refused (not in dry run); an existing `broker.conf` is kept; the units are
written with mode 0644.

**Fake headscale** `deploy/headscale/tests/broker-fake.ts`: a `runner` that keeps nodes in
memory and answers each step with the A4 shapes.

**Tests** `broker-main.test.ts`: `VP-7: register then audit undoes a tagged node and replies undone`;
`VP-7: register reads the node id from the reply, not from a list difference` (a router joins
between the two lists); `5.1: a pendingRegister left by a killed request is audited first`;
`5.1: the count is written before the step` (a runner that throws after the step);
`5.1: the state survives between two processes`; `LOG-007: stderr never holds the key, the auth
ID or a node name`; `BD-9: nodes replies hold no key fields`.

**Done when** (besides the gates): one run on the VPS (VP-11: the VPS is a development system
and may take test installs) shows that `SocketUser=10001` works with no passwd entry for 10001, that
Bun runs under `PrivateTmp`/`ProtectHome`, and that `docker compose exec` works in the sandbox.
Record the result in Section 3.

**Commit.** `VPN B2: the broker process, its systemd socket and the one-command install (R-055, VP-4, VP-8)`.

### B3. Hub broker client and the merge

**`crates/hub/src/config.rs`**: `const VPN_BROKER: &str = "ANIMA_VPN_BROKER";`,
`pub vpn_broker: Option<PathBuf>` (unset or empty → None; not absolute → a config error).
Tests in `config/tests.rs` beside the `tailscale_localapi` ones.

**`deploy/compose.yaml`**, service `hub` (here, after `config.rs`; Section 2):
`ANIMA_VPN_BROKER: ${ANIMA_VPN_BROKER:-/run/anima-broker/broker.sock}` and a long-form bind,
as `localapi` does (`compose.yaml:145-148`): source `/run/anima-broker`, target
`/run/anima-broker`, `read_only: true`, comment "No SELinux label: the VPS runs with SELinux
disabled (R-054)." Mount the folder, not the socket file, so a socket restart is seen. If the
unit is not installed, Docker makes an empty root-owned folder; the hub then reports
`NOT_INSTALLED`. `deploy/test/compose-hub.test.ts:144`: the SEC-001 test lists three mounts.

**`crates/hub/src/vpn/broker.rs`** (about 100; BD-10, BD-15):

```rust
#[derive(Clone)]
pub struct Broker { path: PathBuf, turn: Arc<tokio::sync::Mutex<()>> }
#[derive(Deserialize)] pub(super) struct BrokerNode { id: u64, name: String, user: String, tags: Vec<String>, ips: Vec<IpAddr>, online: bool, expiry: Option<DateTime<Utc>>, approved: Vec<IpNet>, available: Vec<IpNet> }
pub(super) struct BrokerView { pub nodes: Vec<BrokerNode>, pub sites: Vec<(String, IpNet)> }
#[derive(Deserialize)] pub(super) struct NewKey { pub key: PreAuthKey, #[serde(rename = "expiresAt")] pub expires_at: DateTime<Utc> }
#[derive(Deserialize)] pub(super) struct Registered { pub name: String, pub address: Ipv4Addr, #[serde(rename = "keyExpiry")] pub key_expiry: Option<DateTime<Utc>> }
pub(super) enum Refusal { BadRequest, UnknownSite, RateLimited, NodeLimit, NotAllowed, Undone, HeadscaleFailed }
impl Broker {
    #[must_use] pub fn new(path: PathBuf) -> Self;
    pub(super) async fn nodes(&self) -> Result<BrokerView, Error>;
    pub(super) async fn personal_key(&self) -> Result<NewKey, Error>;
    pub(super) async fn router_key(&self, site: &str) -> Result<NewKey, Error>;
    pub(super) async fn approve_route(&self, node: u64, site: &str) -> Result<(), Error>;
    pub(super) async fn register(&self, auth_id: &str) -> Result<Registered, Error>;
}
```

Each call holds `turn` while it connects, writes, reads and closes. `PreAuthKey` is a newtype
with a `Debug` that prints `"[pre-auth key]"` and no `Display` (`credentials::Secret` is a
different type; do not reuse the name). `Error` gets `NotInstalled` (connect gives `NotFound`
or `ConnectionRefused`), `Broker(Refusal)`, `BrokerUnreadable` (also EOF with no reply line).

**Dependencies**: `pub vpn_broker: Option<vpn::Broker>` in `schema.rs` (`.data(vpn_broker)`)
and one line in `main.rs`: `vpn_broker: config.vpn_broker.clone().map(vpn::Broker::new),`
(272 code lines after A1-T0, so there is room). `tests/it/hub.rs` `dependencies()`:
`vpn_broker: None`. `upkeep.rs` puts `dependencies.vpn_broker.clone()` in `Reading`.

**Site name.** The broker takes a policy site name (`home`). The hub maps an Anima site to it by
**subnet**, from `BrokerView.sites`. No match → the client says "This site's subnet is not in
the VPN policy" and offers no key.

**Merge (BD-16).** `reader.rs`: when `reading.broker` is some, each read also calls
`broker.nodes()`. `picture::build` gets `broker: Option<&BrokerView>` in `Inputs` and fills:

- `VpnNode.waitingRoutes: [Cidr!]!` — `available − approved` for the broker node with the same
  IPv4.
- `SitePath.waitingRouter: VpnNode` — a node from the broker list with `tag:router`,
  `tag:site-<name>` and the site subnet in `available` but not in `approved`. It has no path
  (`UNKNOWN`) and its key state comes from the broker `expiry`.
- `VpnPicture.broker: BrokerState!` — `enum BrokerState { NOT_SET NOT_INSTALLED READY FAILED }`.

K1 gets these three fields and the enum (inside the types, rule 0.9), then codegen (rule 0.8).

**Tests**: `tests/it/vpn/broker.rs` with a fake broker (a `UnixListener` in a temp folder that
answers fixed lines): `vp_7_nodes_reply_is_merged_as_waiting_routes`;
`vp_7_a_missing_socket_is_not_installed`; `vp_7_eof_with_no_reply_is_unreadable`;
`vp_7_a_reply_over_64_kib_is_refused`; `vp_7_two_calls_at_once_take_turns` (the fake accepts
one connection at a time and closes a second at once, as `MaxConnections=1`);
`log_007_a_key_reply_debug_prints_no_key`. Folder `tests/it/vpn` stays small (proxy, graphql,
broker, mutations).

**Commit.** `VPN B3: the hub reads the broker's node list and shows routes waiting for approval (R-055, VP-7, BD-10, BD-15)`.

### B4. Keys, registration and enrolment

`vpn/graphql.rs` becomes the face of `vpn/graphql/` in this task: `graphql.rs` (query,
subscription, views) and `graphql/mutations.rs`, so each stays under 300.

**K1** — inside `type Mutation` (`contracts/schema.graphql:1538`), each doc string ending
"Login or browser token only (D-23).":

```graphql
type VpnKey { key: String!, expiresAt: DateTime!, steps: [String!]! }
type VpnRegistration { name: String!, address: IpAddr!, keyExpiry: DateTime }

  "A one-use, one-hour key for a Linux computer of the operator (VPN-003). Login or browser token only (D-23)."
  createVpnInvite: VpnKey!
  "A one-use, one-hour key for the subnet router of a site (VPN-001). Login or browser token only (D-23)."
  createRouterKey(siteId: ID!): VpnKey!
  "Registers the computer that shows this auth ID on its own screen (vpn.md 6B). Login or browser token only (D-23)."
  registerVpnNode(authId: String!): VpnRegistration!
```

  "Sets the key expiry of an untagged operator node to a staggered date 90 to 180 days from now (vpn.md Section 7). Login or browser token only (D-23)."
  renewVpnKey(name: String!): VpnRegistration!

D-23: 27 → 31 (Section 2 lists the places).

**Broker verb `renew` (BD-17, VP-10).** B1/B2 get it in this task: request `{ verb: 'renew',
node: number }`; `plan` refuses a node that is absent, tagged, or not of user `operator`;
steps `['nodes','expire','--identifier=<node>','--expiry=<staggered(now, others)>']` (others =
the operator nodes except this one); not rate limited; the reply is the node
`{ name, address, keyExpiry }`. Tests: `VP-10: renew refuses a tagged node`, `VP-10: renew
staggers against the other Macs`, `VP-10: the expiry value is never empty`. The hub maps
`name` to the broker node id from the latest `BrokerView`.

**`steps`** never hold the key. Router (copied from vpn.md Section 5, lines 916–919, with
`<subnet>` and `--hostname <site>-router`):

1. `sudo bash -c 'umask 077; read -rsp "Pre-auth key: " K; echo; printf "%s" "$K" > /run/anima-preauth'`
2. `sudo tailscale up --login-server https://hs.anima.ing --auth-key=file:/run/anima-preauth --advertise-routes=<subnet> --hostname <site>-router`
3. `sudo rm -f /run/anima-preauth`

No `--advertise-tags`: the key carries the tags, and vpn.md:1043 says to remove that flag.
Personal Linux computer: the same three steps with `--accept-routes` in place of
`--advertise-routes` and `--hostname`. A Mac has no key path in vpn.md (Section 6 uses the
interactive login only), so the modal offers "register" for a Mac.

**Audit.** `events/model.rs` gets `AuditTarget::Vpn` with `kind() = "vpn"`, `id() = "vpn"`
(`audit_log.target_id` is text, `0500_events_logs.sql:36`); `contracts/hub-domains.md:140`
changes with it. Each mutation calls the broker first, then writes one `TrailEntry { source:
Source::Vpn, severity: Info, action: Create (keys) | Accept (register), target:
AuditTarget::Vpn, change: {"site": …} | {"node": …}, message }` in its own transaction. If the
trail write fails, the broker change stays, the error is returned, and the broker log still
has the change.

Broker refusals map to GraphQL errors: `rate-limited`, `node-limit` → `CONFLICT`;
`bad-request`, `unknown-site`, `not-allowed`, `undone` → `VALIDATION` with a message;
`NOT_INSTALLED`, `headscale-failed`, unreadable → `UNAVAILABLE`.

**Client** `apps/client/src/lib/vpn/enrol/`:

| File | Content |
| --- | --- |
| `api.ts` | the three mutations (codegen glob `lib/*/*/api.ts`). |
| `enrol.svelte.ts` | `class Enrolment` like `AgentInstall`: `epoch`, `key`, `left`, `expired`, `busy`, `error`; `makeRouterKey(siteId)`, `makeInvite()`, `register(authId)`, `close()`. Uses `secondsLeft`, `clockText`, `clearTime` from `$lib/metrics/agent/countdown`. |
| `EnrolModal.svelte` | `Modal` with three views: choose (router for site X / Linux computer / Mac or other computer); key (`KeySecret` from `$lib/api-keys/KeySecret.svelte`, the `steps` as numbered `<code>` lines with Copy, a `role="timer"` countdown); register (an input for the auth ID with "Type the ID from the new computer's own screen. Never use an ID from a message or a link."). After a router key, it watches `VpnLive`; when the site has a `waitingRouter`, it shows "Approve route <subnet>" (B5) and "This can take one minute." |
| `enrol.test.ts`, `modal.svelte.test.ts` | |

Entry points: `SitePaths.svelte` "Add a router" when `state === 'NO_ROUTER'`; `NodeKeys.svelte`
"Add a computer to the VPN"; both open `EnrolModal`, only when `picture.broker === 'READY'`;
else a line says why (`NOT_SET`: "Enrolment needs the broker (ANIMA_VPN_BROKER)";
`NOT_INSTALLED`: "The broker is not installed on the VPS (vpn.md Section 11)"). A Renew
button on each untagged operator key calls `renewVpnKey` (BD-17); the text says a computer
whose key has expired is off the VPN, so renew it from another computer.

**Tests** (hub, `tests/it/vpn/mutations.rs`): `d_23_vpn_mutations_refuse_an_api_key` (all
four); `vp_7_create_router_key_returns_the_key_once_and_writes_an_audit_entry_without_it` (the
audit `change` and the event message have no key);
`vp_7_a_rate_limited_broker_gives_a_conflict_error`;
`vp_7_register_undone_gives_a_validation_error_that_says_the_node_had_a_tag`.

(Client): `VP-7: the key is shown once and gone after Close` (textarea value empty after
close, as `KeySecret` F-8); `VP-7: no step holds the key`; `vpn.md 6B: the register view warns
about IDs from messages`; `MOT-040` reduced motion for the modal.

**Commits.** Hub: `VPN B4a: key and registration mutations through the broker (R-055, VP-7, D-23, LOG-005)`.
Client: `VPN B4b: the enrolment modal (VPN-001, VPN-003, VP-2)`.

### B5. Approve a route

K1, inside `type Mutation`: `"Approves the waiting route of the site's router. Login or browser
token only (D-23)." approveSiteRoute(siteId: ID!): Boolean!` (D-23 31 → 32). The hub finds
`SitePath.waitingRouter` in the latest picture, maps the site name by subnet, calls
`broker.approve_route(id, name)` and returns `true` when the broker accepts (BD-16: the client
waits on `vpnChanged`). Audit: `action: Accept, target: Vpn, change: {"site", "router",
"route"}`.

Client: the "Approve route" button in `EnrolModal` and in `SitePaths` (a site with a
`waitingRouter`), with `ConfirmModal`: "Approve <subnet> for <router>? The hub and the
operator's computers can then reach every address in <subnet>." Then "This can take one
minute."

Tests: `r_055_approve_route_refuses_a_router_of_another_site` (broker refusal → validation
error); `vp_7_approve_route_writes_an_audit_entry`; client `R-055: approve asks for
confirmation`.

**Commit.** `VPN B5: approve a site route from the app (R-055, VP-7, D-23)`.

## 8. Gate and review plan

| After | Run | Review |
| --- | --- | --- |
| each task | `BASE_REF=origin/phase-3 bun run gates` | — |
| A1-T5 | full gates | `reviewer` on A1 (CUPID, spec trace, mutation of `path`/`key_state`/`due`) |
| A2-T3 | gates; `bun run --cwd apps/client build` | `reviewer-interface` on Settings and Overview (tokens, reduced motion, WebKit) |
| A4 | `deploy/headscale/test/contract.sh` (manual) | — |
| B2 | gates | `reviewer-security` on `broker/` and the units (input rules, LOG-007, sandbox, limits) |
| B4 | gates | `reviewer-security` on the mutations, D-23 and the modal (key lifetime in the DOM) |

## 9. Live steps (each needs the operator's confirmation in chat)

| Step | When | What |
| --- | --- | --- |
| L1 | after A1-T5 merges and the hub is deployed | Nothing to do: `ANIMA_TAILSCALE_LOCALAPI` is already set. Check `vpn` in the app. |
| L2 | after A3b | Run the inventory check once on the live tailnet (vpn.md Section 11). Read-only on the VPS. |
| L3 | after B2 | `scp` the broker binary; `sudo harden.sh broker`. VPS change (VPS-005). |
| L4 | after B4 | Deploy the hub with the broker mount. Enrol the first router from the app (vpn.md Section 5 replaced by the modal). |
| L5 | after the first router | A1 acceptance with a live capture (vpn-plan.md A1-T5): save the status JSON through the redactor, commit it as a fixture. |

## 10. Open points

The review of version 0.1 (35 findings) is applied. Its open points 1, 2 and 4 are closed:
the start order does not change (A1-T0 Check), the tests need no paused clock and no seam
(A1-T4), and `checks` is a domain edge (T0). What is still open:

1. Closed: the operator approved the `renew` verb (VP-10) and testing on the VPS (VP-11).
2. Closed (A4): the active peer, the 60 s decay, the auth ID format and the reply shapes are
   in Section 3, facts 10 to 14. B1 and B2 read these shapes.
3. **Evidence, B2:** the run of the broker unit (on the VPS, VP-11) (`SocketUser=10001` with no passwd
   entry, Bun under the sandbox, `docker compose exec` in the sandbox).

## 11. Build notes

Deviations that the builders found, with the evidence. Each keeps the intent of the task.

1. **A5/A3a/A3b/A4: base.** The agent worktree started at an older commit than the task named.
   Branch `vpn-tier-a-headscale` was created from `a126c02b` (the task's base) and the Linux-only
   clippy fix `70e07229` was cherry-picked on it (coordinator instruction).
2. **A4: `server_url` is `http://172.29.250.10:8080`, not `http://headscale:8080`.** Section 3,
   fact 16: Tailscale 1.104 forces TLS on a re-login for a host name and for a non-private
   address, so the renewal steps (A4 step 4) fail against plain HTTP. `contract.sh` creates a
   Docker network `172.29.250.0/24` and gives the `headscale` service `172.29.250.10`.
   `contract-config.yaml` and the drift test in `server.test.ts` use the same address. If that
   subnet is taken on the machine, Docker refuses the network and the script fails at once.
3. **A4: `policy check` runs after the joins, not in step 2.** Section 3, fact 15.
4. **A4: the Tailscale image tag is `v1.104.4`.** `stable` is a moving tag; `v1.104.4` has the same
   index digest on the day (`sha256:ed6480b3…`). The harness starts the containers with
   `entrypoint: tailscaled --tun=userspace-networking --state=mem:` (the image's `containerboot`
   would run its own `tailscale up`).
5. **A4: redaction is a pure function in `src/redact.ts`**, with `bin/redact.ts` as the thin
   command (the task named only the bin file), so that it has tests (`redact.test.ts`). Node N is
   the position of the host name in `NODE_NAMES` (`hub`, `router-home`, `mac`, `stranger`, `odd`,
   `mac2`).
6. **A4: the drift test is in `server.test.ts`** as the task says; the file is 276 non-blank lines.
7. **A3b: ids are read as numbers or digit strings.** Headscale writes JSON numbers (fact 14), but
   a protobuf JSON writer uses strings for 64-bit values, so `parseNodes` accepts both.

- **A2-T1.** The spec's `toolbar.test.ts` and `page.test.ts` fixtures build a `ListQuery` literal, so
  they also gain `site: null` (`toolbar.test.ts` Clear-filters case, `page.test.ts` `beforeEach`).
  `Sites` keeps the loaded sites in a private field and derives `options`, so "All sites" is never
  shown alone (`toolbar/sites.svelte.ts`).
- **Gates on macOS.** `bun run gates` stopped at clippy: the Linux-only reaper code in
  `apps/desktop/src-tauri/src/builder/` was dead code off Linux. Fixed in `70e07229`.

- **Merge: hub fixtures.** The hub's tests use the hand-made `status-idle.json` and `status-active.json`. The real A4 captures are kept beside them as `captured-status-*.json`; `contract.sh` writes those names, and `vpn/tests/peers.rs` checks that the hub reads them (router `Direct` when active, `Unknown` after 60 s).

### Hub lane (A1-T0 to A1-T5)

Deviations that the Tier A hub build made from this document, with the smallest fix that keeps
its intent.

1. **A1-T0.** Clippy `too_many_lines` (limit 60) refuses `upkeep::spawn` with the four loops
   added at its end (68 lines). The loops and `reaching()` go in a private
   `upkeep::loops(dependencies)`, and `spawn` ends with `jobs.extend(loops(dependencies))`. The
   start order does not change. `upkeep.rs` is 169 code lines (the target was 160), because
   rustfmt writes each long `spawn_*` call on several lines; the file limit is 300.
2. **A1-T2.** `devices::Site` has a `pub(super)` field (`crates/hub/src/devices/model.rs:164`,
   `wake_relay`), so no code outside `devices` can build a `Site`, and the pure tests cannot
   make one. `picture::Inputs.sites` and `picture::router_devices` take `&[(SiteId, IpNet)]`
   (each site's id and subnet, in site name order) instead of `&[Site]`; the reader maps
   `devices::sites()` to that. The picture reads nothing else of a site.
3. **A1-T2.** `picture::build` returns `Result<Picture, Error>`: `Unreadable` when the hub's own
   node has no IPv4 address (a node that is logged out has `TailscaleIPs: null`). A peer with no
   IPv4 is skipped as the spec says; the hub cannot be skipped.
4. **A1-T2.** `picture::carry` keeps no path for a node that is offline in the new read:
   VPN-009 makes offline always `Unknown`, and BD-13 is about traffic that stopped.
5. **A1-T2.** `pub mod vpn;` goes in `lib.rs` in A1-T2 (the spec puts it in A1-T4), or the
   A1-T2 tests do not compile. Until the reader calls them, `peers`, `picture` and `warnings`
   carry `#[cfg_attr(not(test), expect(dead_code, reason = …))]`; A1-T4 removes the three
   attributes. `quality/hub-modules.json` gets the `vpn` rows of T0 in the same commit.
6. **A1-T2.** Clippy `struct_field_names` refuses `Peer.peer_relay`; the field keeps the spec's
   name with an `#[expect(…, reason = …)]`.
7. **A1-T2.** `warnings::due` gives no warning for a node whose key is not `nodekey:…` of at
   most 80 characters: `vpn.warnings.node_key` refuses it, and one refused insert would undo
   the whole warning transaction for every node.
8. **A1-T2.** `DeviceId` has no `Ord`, so `router_devices` sorts by the UUID before it removes
   repeats.
9. **A1-T3.** `contracts/hub-domains.md:139` (K3's copy of `pub enum Source`) is not changed
   here: T0 owns that file in this build, and its copy at `dd24bbc7` still ends at
   `Notifications`. It needs `, Vpn   // Vpn (VPN-010, VPN-011)` from the T0 owner.
10. **A1-T3.** The eighth-source test (as `notifications/tests/schema.rs:98`) and
    `events_source_vpn_is_stored_and_read_back` are in the new
    `crates/hub/src/events/tests/source.rs`, because `events/tests.rs` is at 274 code lines. The
    read back uses the events store's source filter and checks the GraphQL name `VPN`, not the
    whole `events` query with a signed-in schema. `store.rs` is still unused outside the tests,
    so it gets the same `dead_code` expectation as note 5.
11. **A1-T4.** `picture::carry` runs in `reader::read_and_publish` with the feed's latest
    picture, not inside `build_picture`, so `build_picture` keeps the spec's three parameters.
    The reader's work after the read is `read_and_publish`; `read_once` only logs the change
    of state. Clippy `cognitive_complexity` counts each `tracing` macro high, so the two log
    lines are in `read_failed` and `read_again`.
12. **A1-T4.** A failed read publishes the not-fresh copy only when the latest picture is
    still fresh: one wake-up for an outage, not one a minute with the same picture.
13. **A1-T4.** The fake proxies have no `Hang` reply: no test uses it, and dead code fails
    clippy. `Reply` is a status and a body. The integration proxy keeps `seen()`; the log test
    checks that every request is the status GET.
14. **A1-T4.** `vpn_008_no_log_line_holds_status_text` cannot reach `read_once` from
    `tests/it`, and the reader's second tick is 60 s away. It starts one reader that reads the
    good answer, then a second reader on the same feed whose read gets the 500 answer (which
    also holds the status text), and waits until the picture is not fresh.
15. **A1-T4.** `upkeep::loops` starts the reader (note 1), so its doc comment names it.
16. **A1-T5.** `SitePathView`'s helper that returns the site's `SitePath` is `entry()`, not
    `path()`: the `#[Object]` impl's `path` field resolver has the same name. The `vpn`,
    `vpnChanged` and `routes` resolvers carry the neighbours' reasoned
    `#[expect(clippy::unused_async, clippy::unused_async_trait_impl, …)]`.
17. **A1-T5.** `bun run --cwd apps/client graphql` writes no change in this task: the client
    generates types only for the documents it uses, and no client document reads `vpn` until
    A2-T2. The lint gate's `codegen.ts --check` confirms the generated files are current.
18. **A1-T5.** K1 places the new `Query.vpn` under a `# Phase 3: VPN (VP-5)` comment at the end
    of `type Query`; the enums are written one value per line, as the rest of K1.
### Broker lane (B1, B2)

- **B1: the auth ID pattern is `^hskey-authreq-[A-Za-z0-9_-]{24}$`** (Section 3, fact 11), not the
  pattern in the B1 text. A value that starts with `-` can never match.
- **B1: `personal-key` names the user by id.** `preauthkeys create --user` takes a numeric id (fact
  12; a local Headscale 0.29.7 refuses `--user=operator` with `strconv.ParseUint`). The broker runs
  `users list --output=json` first, takes the id of the user `operator`, and replies
  `headscale-failed` when there is none. `plan` therefore takes a fourth parameter,
  `Moment { now, operatorId }` (`now` is for `renew`).
- **B1: `auth register --user` takes the name.** A local Headscale 0.29.7 answers `--user=1` with
  `looking up user: user not found` and accepts `--user=operator` (the harness also uses the name,
  `contract.sh:126`). The step stays `--user=operator`.
- **B1: `--output=json`, not `-o json`.** The rule "each flag and its value are one argv element"
  then holds for every step without an exception; Headscale accepts both forms (local check).
  `--tags=tag:router,tag:site-<x>` in one element is one string-slice value (local check:
  `acl_tags` lists both).
- **B1: an empty list is `null`.** A local Headscale 0.29.7 prints `null` for `nodes list` and
  `users list` with no entries (`nodes list-routes` prints `[]`). `broker/nodes.ts` `tailnetOf`
  reads `null` as `[]`; `src/inventory.ts` `parseNodes` still refuses `null` (the inventory check
  runs on a tailnet that has nodes).
- **B1: Headscale errors with `--output=json` are `{"error": "…"}` with exit 1.** The runner fails on
  the exit code; the reply readers also refuse a reply with no `id` or `key`.
- **B1: the `renew` verb (BD-17, VP-10) is in B1.** `{ "verb": "renew", "node": <id> }`; `plan`
  refuses an absent, tagged or non-`operator` node with `not-allowed`; not rate limited.
- **B1: `approve-route` also requires `tag:router`** on the node, besides `tag:site-<site>` and the one
  available route.
- **B1: a passed `Issued` entry is dropped 2 hours after `at`**, like an entry with no node. A passed
  entry is never checked again, so dropping it changes no outcome and keeps the state small.
- **B1: reply readers live in `broker/nodes.ts`** (`newKeyOf`, `registeredIdOf`, `operatorIdOf`,
  `registered`), with `tests/broker-nodes.test.ts` against the harness fixtures, and
  `tests/broker-fixture.ts` for node builders. `src/policy.ts` gains `prefixText` (the site subnet
  as text).
- **B2: the core is `broker/serve.ts`, the files are `broker/state.ts`.** `main.ts` keeps the entry
  points, the `headscale` runner, the stdin reader and the `check` client; the split keeps each
  unit to one purpose. `serve(line, runner, files, now)` has the B2 signature and returns the reply
  and the log line; `main.ts` writes them.
- **B2: the order inside a request.** A `bad-request` is answered before any Headscale step. Then
  one audit covers the `pendingRegister` node and the issued keys (B2 steps 1 and 2 share one node
  read). `plan` runs before `admit`, so a refused site or node is not counted against the limit;
  the count is still written before the step. `personal-key` runs `users list` before `plan`.
- **B2: `nodes` is answered from the list the audit read** (read again only after a fix), not by a
  second run of the same two steps. `approve-route` replies `{ node, site }`; `renew` replies the
  node `{ name, address, keyExpiry }` from a fresh list.
- **B2: an unreadable `state.json` or `broker.conf` refuses every verb** with `headscale-failed`; a
  missing `state.json` is the empty state. A state the broker cannot read is never taken as empty,
  so the limits cannot be reset by corrupting it.
- **B2: `harden.sh broker` runs `anima-broker.new count-nodes` always**, also when `broker.conf`
  exists, as the architecture and Headscale check (a binary for another architecture fails to
  start). It reads the units from `../broker/` (on the VPS `/opt/anima/headscale/broker/`), and
  removes `anima-broker.new` after `install`. `harden.sh` gets `UNAME` (`HARDEN_UNAME`) for the
  fake host; `tests/vps-host.ts` copies the two units beside its `vps/` copy.
- **B2: binary check.** `bun deploy/headscale/bin/broker-build.ts` built
  `bun-linux-x64-baseline-v1.3.14`, ELF x86-64, 93 661 312 bytes. In a local `almalinux:10`
  container (amd64): EOF → no reply, exit 0; `bogus` → exit 64; a bad verb → `bad-request`;
  `nodes` with no `broker.conf` → `headscale-failed`; `count-nodes` with no Docker → exit 1 with
  `count-nodes failed (StepError)`. The VPS run (Section 10, point 3) is still open.
- **B2: the service unit is `broker/anima-broker@.unit` in the repository**, installed as
  `anima-broker@.service`, as `backup/headscale-backup.unit` is: `service` is a banned file name
  (`quality:names`).
- **B2: `tests/broker-process.test.ts`** covers `readLine`, the entry points and the files, besides the
  `broker-main.test.ts` rows (which call `serve` with `tests/broker-fake.ts`).

### Hub lane (B3, B4a, B5)

- **B3: the picture keeps the broker's answer.** `Picture` gets `broker: BrokerState` and a
  `pub(super) roster: Option<BrokerView>` (the node list and policy sites of the same read).
  B4 and B5 map a site, a node name or a waiting router to the broker's site names and node ids
  from it, as B4 and B5 say ("the latest picture"). With no LocalAPI proxy there is no picture,
  so the mutations answer `UNAVAILABLE`.
- **B3: the merge is its own pure file, `vpn/waiting.rs`** (waiting routes, the waiting router,
  the policy site name of a subnet). `picture.rs` exposes `role`, `key_state` and `device_by`
  (BD-14 matching by IPv4 or unique name, split out of `device_of`) to it. A waiting router is
  a `VpnNode` with an empty `key` (the broker drops every key, so it gets no warning marks),
  `os` empty and `path` `UNKNOWN`. A policy site name that is not `^[a-z0-9-]{1,32}$` is
  treated as absent: B4 puts it into a command line the operator copies.
- **B3: the error variants.** `vpn::Error` gets `NotInstalled`, `BrokerRefused { refusal, detail }`
  (the spec's `Broker(Refusal)`; the broker's detail, cleaned and cut to 160 characters, is kept
  for the `VALIDATION` message of `undone`), `BrokerUnreadable`, and also `BrokerTimedOut` (the
  30 s limit) and `BrokerConnection` (any other socket error), so each name tells the truth.
  `Refusal` is `pub`, as the field of a `pub` error. A reply parse error never keeps serde's
  text: serde quotes the value it could not read, which can be a key.
- **B3: the reply line is read into a `Zeroizing` buffer**, and `PreAuthKey` holds a
  `Zeroizing<String>`; the envelope `{ok, value | refusal, detail}` denies unknown fields.
- **B3: only `nodes` is in B3.** The other verbs come with the mutations that call them (B4:
  `personal_key`, `router_key`, `register`, `renew`; B5: `approve_route`), so no client method
  is dead code. `NewKey` and `PreAuthKey` are in B3 for the LOG-007 test, with
  `#[cfg_attr(not(test), expect(dead_code, …))]` until B4.
- **B3: `log_007_a_key_reply_debug_prints_no_key` is a unit test** (`src/vpn/tests/broker.rs`,
  with the reply-shape tests and two property tests of the parser): `NewKey` is `pub(super)`.
  The other five B3 tests are in `tests/it/vpn/broker.rs` through the reader and the `vpn`
  query; `vp_7_two_calls_at_once_take_turns` fails (1 refused connection) without the turn.
- **B3: the reader reads the status and the broker at once** (`tokio::join!`). A broker failure
  sets the state only; the reader logs a change of the broker state by its fault word.
- **B3: wiring.** `main.rs` names `vpn_broker` first in the `Dependencies` literal, because
  `config` moves into the next field. `schema::build_schema` went to 61 lines with
  `.data(vpn_broker)`; the devices loaders moved into `with_devices_loaders`. `config/tests.rs`
  went over 300 lines; the two LocalAPI tests and the broker test moved to
  `config/vpn_tests.rs`. `deploy/test/compose.test.ts` skips the broker bind in the SELinux
  label rule, as it does for `localapi` (R-054).
