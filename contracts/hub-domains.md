# K3 — Hub domain interfaces

Status: **frozen for Phase 3** (change only through a programme ruling, CON-001). Phase 2 additions are marked (Phase 2). Phase 3 additions are marked (Phase 3).

These are the only functions and types one hub domain may use from another. Everything else in a domain is private or `pub(crate)` for its own submodules. Signatures are the contract; parameter names and doc comments may vary. `Db` is `sqlx::PgPool`; functions that must join a caller's transaction take `&mut sqlx::PgConnection` instead.

## Folder groups (OD-E5, E0-1, 2026-11-13)

The hub groups its domains under parent nouns so `crates/hub/src/` stays within 12 files (REPO-005, REPO-E-001). A group is a folder and its face, such as `fleet/` and `fleet.rs`; the face only declares the group's modules and holds no code. A group is not a domain: the rules of this contract name each domain by its own name, wherever its folder is, and a domain inside a group keeps the shape of Section 13 of the spec.

| Group | Modules |
| --- | --- |
| `crates/hub/src/fleet/` | The device manager: `access`, `agent_usage` (R-135), `builds`, `checks`, `credentials`, `devices`, `events`, `maintenance`, `metrics`, `notifications`, `retention`, `sessions`, `ssh`, `targets` (a leaf), `trust`, `vpn`, `web_interfaces` |

The leaf modules other than `targets` and the wiring stay at `crates/hub/src/`. A domain path in Rust goes through its group: `crate::fleet::devices`, `anima_hub::fleet::checks`. `quality/hub-modules.json` lists the groups under `groups`; the REPO-004 check reads a module inside a group as a top-level module. Later groups (`engine/`, `vault/`, `life/`, `work/`, Section 6 of `docs/plans/extensions-plan.md`) are added by the task that adds their first code.

## Leaf modules (owned by WP-01; every domain may import them; they import no domain)

| Module | Content |
| --- | --- |
| `crates/hub/src/ids.rs` | All entity ID newtypes (see `conventions.md`): `OperatorId`, `SiteId`, `DeviceId`, `CheckId`, `WindowId`, `SessionId`, `EventId`, and (Phase 2) `WebInterfaceId`, `MetricSeriesId`, and (Phase 3) `ApiKeyId`, `ChannelId`, `DeliveryId`, and (Phase 3, WP-36, R-107) `TokenId`, `ReleaseId`, `BuildJobId`, `BuilderId` |
| `crates/hub/src/engine/instants.rs` (engine, new; operator, 2026-11-16; ruling number to be assigned; K-E3, TML-008) | `micros` and `from_micros`: an event instant as microseconds since the Unix epoch in UTC. It imports no module of the hub |
| `crates/hub/src/engine/sealing_keys.rs` (engine, new; R-148; K-E5 Section 1) | `SealingKeys` and `SealingKey`: the data keys of `ANIMA_DATA_KEY_DIR` (VLT-003). It imports no module of the hub |
| `crates/hub/src/engine/records.rs` (engine, new; operator, 2026-11-16; ruling number to be assigned; K-E4) | `Reference` (`kind:id`) and `Subject` (the shared subject keys, RLK-004). It imports only `ids`, which gains `RecordLinkId`, `PlaceId` and `PersonId` |
| `crates/hub/src/status.rs` | `pub enum Status { Up, Down, Degraded, Maintenance, Unknown }` with `fn worst(a, b)` (CHK-004) |
| `crates/hub/src/clock.rs` | `pub trait Clock: Send + Sync { fn now(&self) -> DateTime<Utc>; }`, `SystemClock`, and `FixedClock` for tests. The only clock read in the hub (CQ-021). |
| `crates/hub/src/config.rs` | `pub struct Config` from the environment (see `conventions.md`) |
| `crates/hub/src/paging.rs` | `PageEnd`, exposed as K1 `PageInfo { hasNextPage, endCursor }` (R-014; the Rust name avoids the banned word "info"). Every connection uses it; cursors stay opaque and per domain. |
| `crates/hub/src/fleet/targets.rs` (Phase 2; in `fleet/` since OD-E5, E0-1, 2026-11-13) | `AddressPolicy`, the target rule and deny list (P2-004). It moves here from `checks` (programme-owned, as R-003), so `sessions`, `checks` and `metrics` share one deny-list type without new edges. WP-20 makes the move. |
| (Phase 3) `crates/hub/src/api_errors.rs` (Q2-06) | `ErrorCode` (the K7 codes), `internal`, `coded`, `invalid`, `FieldError`, `field_names`, `schema_dependency`, generic `pool`, `begin`, `commit`. Each domain keeps its own `to_graphql(Error)` |
| (Phase 3) `crates/hub/src/text.rs` (Q2-11) | `is_invisible(char)` (the widest table), `is_plain(&str)`, `clean(&str, max_chars) -> String` |
| (Phase 3) `crates/hub/src/fingerprint.rs` (Q2-10) | `CertificateFingerprint`: parses 64 hex digits or 32 colon pairs; one canonical value; two renderings |
| (Phase 3) `crates/hub/src/fleet/targets.rs` (changed) | Adds `pub async fn resolve_checked(host: &str, port: u16, policy: &AddressPolicy) -> Result<Vec<SocketAddr>, TargetError>` (moved from `checks/probe/resolve.rs`, P3-003). `sessions/target.rs` is deleted (Q2-09); the loopback test switch stays on `AddressPolicy` |
| `crates/hub/src/scalars.rs` | GraphQL scalars `IpAddr` and `Cidr` (network only, no host bits) for K1 (R-012). `DateTime` is `chrono::DateTime<Utc>` and `JSON` is `async_graphql::Json<T>`. Domains never define their own scalar with a K1 name. |

## Wiring (owned by WP-01: `main.rs`, `schema.rs`)

There is no shared `AppState` type that domains import (it would create import cycles). `main.rs` builds each dependency once and:

- puts them in the GraphQL schema data: `Schema::build(..).data(pool).data(bus).data(vault).data(clock).data(config)`; resolvers read what they need with `ctx.data::<T>()`;
- passes them as explicit arguments to background tasks and routers (`checks::spawn_scheduler(pool, bus, clock, ..)`, `sessions::tunnel_router(..)`).

`Db` below is `sqlx::PgPool`; `Clock` is `Arc<dyn clock::Clock>`.

GraphQL composition (`schema.rs`, wiring only, CQ-033):

```rust
#[derive(async_graphql::MergedObject, Default)]
pub struct Query(access::graphql::Query, devices::graphql::Query, checks::graphql::Query,
                 maintenance::graphql::Query, events::graphql::Query, sessions::graphql::Query);
// Mutation and Subscription merge the same way (MergedSubscription for subscriptions).
```

Each domain exposes `pub mod graphql` with `#[derive(Default)] pub struct Query; pub struct Mutation; pub struct Subscription` (omit the ones it does not need). Resolvers read their dependencies from the GraphQL context data and the `Operator` from the request guard.

(Phase 3) Wiring changes (`main.rs`, `lib.rs`, `schema.rs`, `upkeep.rs`, and new `spawns.rs` and `web_client.rs`):

```rust
// web_client.rs (WP-43): wiring only. The static fallback of the router; explicit routes win.
pub fn web_client(root: &Path, origin: &PublicOrigin) -> axum::Router;   // headers as K7 "HTTP surface"
// lib.rs mounts, each logged and behind the NET-002 layer:
//   /metrics: checks::exposition_router(..) wrapped in access::bearer_layer(.., Scope::Read)
//   fallback: web_client(..) when ANIMA_WEB_ROOT is set
// spawns.rs (P3-007): spawn_scheduler, spawn_collector, metrics::spawn_stale_watch, notifications::spawn_dispatcher.
// upkeep.rs: reads retention::periods daily and passes each period to its domain's job (D-25).
```

## access (WP-05)

```rust
pub use events::{Operator, ClientAddr};   // defined in events (R-019)

/// GraphQL guard: requires a valid token; puts `Operator` in the context (API-003).
pub struct Authenticated;
/// The permitted networks and the refusals written, built once in main.rs and shared by the
/// listener's accept-time check and the layer, so both give one answer (NET-002, R-051).
pub struct Edge;
impl Edge { pub fn new(db: Db, bus: events::Bus, clock: Clock) -> Self; }
/// axum middleware: refuses addresses outside the permitted networks of `edge`, logs the refusal (NET-002).
pub fn permitted_networks_layer(edge: Edge) -> impl tower::Layer<axum::routing::Route> + Clone;
/// The hub's TLS listener: drops refused peers at TCP accept, before TLS (R-051). main.rs and
/// the `health` subcommand use it; no other domain does.
pub mod listen; // listen, Listening, Entrance, TlsFiles, TlsListener, Fault, Error, check_health, HealthError
/// Used by the tunnel: resolve a bearer token to an operator.
pub async fn operator_for_token(db: &Db, token: &str, now: DateTime<Utc>) -> Result<Operator, Error>;
```

(Phase 3, WP-43):

```rust
pub use tokens::Scope;                       // Read, Write, and Builder (WP-36, R-107)
pub enum TokenKind { Login, Browser, ApiKey, Builder }   // Builder: WP-36, R-107
impl Identity { pub const fn kind(&self) -> TokenKind; }   // the guard refuses API keys on the D-23 list
/// For routes outside GraphQL (/metrics): a bearer token of at least `scope`; 401 with no body otherwise.
pub fn bearer_layer(db: Db, clock: Clock, scope: Scope) -> impl tower::Layer<axum::routing::Route> + Clone;
/// The hub's public origin from ANIMA_PUBLIC_ORIGIN; the cookie Origin check and the CSP use it.
#[derive(Clone)] pub struct PublicOrigin(/* scheme, host, port */);
// identify(..) also reads the __Host-anima cookie when Origin equals the public origin (D-3).
// SocketAccess also takes the upgrade's headers, so a WebSocket can sign in by cookie (K7).
```

(Phase 3, WP-36, R-107, D-1) Builder tokens are access tokens of kind `builder` (`TokenKind::Builder`)
and scope `builder` (`Scope::Builder`), always both (migration 0030). They travel only as a bearer, live
180 days and are stored only as their hash. `Authenticated`, `operator_for_token` and the document backstop refuse them
outside the builder fields; an operator token is refused on the builder fields (`Error::OutOfScope`, FORBIDDEN).

```rust
/// GraphQL guard: requires a valid builder token (BLD-022).
pub struct BuilderSignedIn;
impl Identity { pub fn builder_token(&self) -> Option<TokenId>; }
/// A new builder token; `secret()` is the only way to read it and it has no `Debug`.
pub struct IssuedToken { pub id: TokenId, pub expires_at: DateTime<Utc>, /* token */ }
pub async fn issue_builder_token(conn: &mut PgConnection, operator: OperatorId, now: DateTime<Utc>) -> Result<IssuedToken, Error>;
pub async fn revoke_builder_token(conn: &mut PgConnection, id: TokenId) -> Result<bool, Error>;
```

## events (WP-05)

```rust
pub struct Operator { pub id: OperatorId, pub name: String }   // re-exported by access (R-019); see the actor kinds below
pub struct ClientAddr(pub std::net::IpAddr);                     // re-exported by access (R-019)
pub struct HeartbeatSummary { pub at: DateTime<Utc>, pub status: Status, pub response_time_ms: Option<u32>, pub message: Option<String> }   // used by checks (R-019); `message` as stored: cleaned, at most MESSAGE_MAX chars; Clone, not Copy (R-036)
pub enum Severity { Info, Warning, Error }
pub enum Source { Check, Session, Device, Access, Hub, Metrics, Notifications }        // Metrics (Phase 2); Notifications (Phase 3); Proxmox later. No new Signal (D-15)
pub struct NewEvent { pub severity: Severity, pub source: Source,
                      pub device: Option<(DeviceId, String)>, pub message: String }
pub struct NewAudit { pub operator: Operator, pub action: AuditAction,
                      pub target: AuditTarget, pub client: ClientAddr,
                      pub change: serde_json::Value }

pub async fn record(conn: &mut PgConnection, bus: &Bus, at: DateTime<Utc>, e: NewEvent) -> Result<EventId, Error>;
pub async fn audit(conn: &mut PgConnection, at: DateTime<Utc>, a: NewAudit) -> Result<(), Error>;
/// The redaction `record` applies, for text other domains store (WP-36: build log lines, BLD-049).
pub fn redact_text(text: &str) -> String;
// (WP-36) AuditAction adds Cancel; AuditTarget adds Release(ReleaseId), BuildJob(BuildJobId), Builder(BuilderId).

/// Broadcast of new events and status changes for subscriptions (LOG-010).
#[derive(Clone)] pub struct Bus { /* tokio::sync::broadcast */ }
pub enum Signal { Event(EventSummary), DeviceStatus { device: DeviceId, status: Status, at: DateTime<Utc> },   // `at` added by R-028
                  Heartbeat { check: CheckId, device: DeviceId, beat: HeartbeatSummary } }
impl Bus { pub fn publish(&self, s: Signal); pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Signal>; }
```

`Status`, `CheckId` and `DeviceId` come from the leaf modules, and `events` defines the three shared value types above, so `events` imports no domain (R-019).

Wiring only (called from `main.rs`, never from another domain): `create_partitions_ahead`, `drop_expired_partitions`, `check_runtime_role`, `run_daily`, `keep_partitions` (R-016, R-019; WP-05 T2).

(Phase 3, changed, Q2-07):

```rust
pub enum Source { Check, Session, Device, Access, Hub, Metrics, Notifications, Vpn }   // Notifications (Phase 3), Vpn (VP-5)
pub enum AuditTarget { ..., WebInterface(WebInterfaceId), ApiKey(ApiKeyId), Channel(ChannelId), Vpn }   // Vpn: kind() and id() are "vpn" (VP-7)
/// One audit-then-event write for every domain (Q2-07). The entry carries its source and device.
pub struct Trail { /* bus, operator, client address, the time read once */ }
impl Trail {
    pub fn from_context(ctx: &async_graphql::Context<'_>) -> Result<Self, Error>;
    pub async fn write(&self, conn: &mut PgConnection, entry: TrailEntry) -> Result<(), Error>;
}
// Retention jobs take their period (D-25): keep_partitions(db, now, events_keep: chrono::Days).
// CR-9: drop_expired_partitions(db, now, events_keep: chrono::Days) also takes the period (wiring only).
// CR-9: AuditTarget::Setting(&'static str) names a hub setting by its dotted name (retention.events, checks.defaults).
```

(operator, 2026-11-13, LOG-012 to LOG-020; ruling number to be assigned) Actor kinds (LOG-012 to LOG-019):

```rust
pub enum ActorKind { User, Agent, Hub }            // K1 ActorKind
pub enum TokenKind { Login, Browser, ApiKey }      // K1 TokenKind, as the audit log records it
/// Who did an action; never the token (LOG-018). Only `access` builds a request's actor.
pub struct Actor { /* kind, token kind, API key name */ }
impl Actor { pub const fn hub() -> Self; pub fn request(kind: ActorKind, token: TokenKind, api_key_name: Option<String>) -> Self;
             pub fn kind(&self) -> ActorKind; pub fn token(&self) -> Option<TokenKind>; pub fn api_key_name(&self) -> Option<&str>; }
pub struct Operator { pub id: OperatorId, pub name: String, pub actor: Actor }   // replaces the row above
impl Operator { pub fn hub() -> Self;              // nil id, name "Anima", Hub (OD-15)
                pub fn by_hub(self) -> Self; }     // the same operator, for the hub's own work on their behalf
pub struct NewEvent { pub severity: Severity, pub source: Source, pub device: Option<(DeviceId, String)>,
                      pub message: String, pub actor: ActorKind }   // Hub for an event no request caused
```

`NewAudit` takes its actor from `operator.actor`. A writer that no request drives (scheduler, retention, prune,
policy, builder and daemon tokens per LOG-019) uses `Operator::hub()` and `ActorKind::Hub`.
`access` adds `pub struct Declaration` (the agent declaration, `of_headers`, `of_payload`), `pub enum MadeFor
{ User, Agent }` and `pub const ACTOR_HEADER`; the decision itself is private to `access` (LOG-014).

(R-138, LOG-021 to LOG-028) The hash chain of the audit log:

```rust
// audit() also writes the entry's link in `events.audit_chain`, in the caller's transaction, under the
// chain's advisory lock until that transaction ends (LOG-024). Its signature does not change.
pub async fn link_existing(db: &PgPool) -> Result<u64, Error>;   // LOG-026: links the stored entries when the chain is empty
pub async fn check_chain_daily(db: &PgPool, bus: &Bus, now: DateTime<Utc>) -> Result<(), Error>;   // LOG-028: an Error event on a fault
```

Wiring only (called from `main.rs`, `operator.rs` and `upkeep.rs`, never from another domain): `link_existing` before
anything writes an audit entry, and `check_chain_daily` through `run_daily`. The check itself and `Query.auditChain`
are private to `events`.

### Allowed domain dependencies (the import-graph gate enforces this list; REPO-004)

| Domain | May import (besides leaf modules) |
| --- | --- |
| `events` | — |
| `credentials` | — |
| `access` | `events`, `credentials` (Vault only, for TOTP secrets) |
| `devices` | `events`, `credentials` (`Vault`, `Secret`, `SshSecret`, `Error`, `has_ssh`, `with_ssh`, `store_ssh`, `clear_ssh`, and in Phase 2 `PasswordKind`, `store_password`, `clear_password`, `with_password`; R-010, R-017, R-026, WP-21) |
| `maintenance` | `events` (R-029; audit and events for window changes, LOG-005) |
| `checks` | `devices`, `maintenance`, `events`, `credentials` (`Vault`, `Secret`, `Sealed` only; R-028) |
| `sessions` | `access`, `devices`, `credentials`, `events` |
| `sessions` (Phase 3, replaces the row above) | `access`, `devices`, `credentials`, `events`, `ssh`, `trust` |
| `checks` (Phase 2, adds) | `sessions` (`link`, `SshLink`, `FixedCommand` only; no cycle: `sessions` does not import `checks`) |
| `checks` (Phase 3, replaces the Phase 2 row above) | `ssh` (`link`, `SshLink`, `FixedCommand`, `CommandOutput`, `LinkError` only; replaces `sessions`; CR-11), `notifications` (`observe`, `adopt_defaults`, `Observation`, `for_checks` only) |
| `metrics` (Phase 2, new) | `devices`, `sessions` (link only), `credentials` (`Vault` only), `events` |
| `metrics` (Phase 3, replaces the row above) | `devices`, `ssh` (`link`, `CommandOutput`, `LinkError` only; replaces `sessions`; CR-11), `credentials` (`Vault` only), `events`, `access` (`Authenticated`, `Operator`, `ClientAddr` only; daemon tokens live in `metrics`, R-092 D-1); crate `daemon-core` (CR-8) |
| `web_interfaces` (Phase 2, new) | `checks` (`http_urls` only), `devices` (CR-5), `events` |
| `trust` (Phase 3, new) | `devices`, `events` |
| `ssh` (Phase 3, new) | `devices`, `credentials`, `trust`, `events` |
| `notifications` (Phase 3, new) | `events`, `credentials` (`Vault`, `Secret`, `Sealed` only) |
| `retention` (Phase 3, new) | — |
| `builds` (Phase 3, WP-36, new) | `access` (builder tokens, `Identity`, the guards), `events` |
| `vpn` (Phase 3, VP-5, new) | `devices`, `events`, `checks` (`for_devices` only) |
| `extensions` (engine, new; operator, 2026-11-13; ruling number to be assigned; K-E7) | `events` |
| `sources` (engine, new; same ruling; K-E1) | `extensions` (`Key`, `on_keys` only), `credentials` (`Vault`, `Secret`, `Error`, `store_source_secret`, `load_source_secret`, `clear_source_secret` only), `events` |
| `sources` (Connect GitHub, replaces the row above; operator, 2026-11-14; ruling number to be assigned; K-E1 Section 7a) | the same, and `access` for `registration/routes.rs` only: `PublicOrigin`, `SameOrigin`, `identify`, and a cookie reader (K-E1 Section 12) |
| `projects` (work, new; same ruling; XP-02) | `extensions` (`PROJECTS`, `require_on` only), `sources` (`Intake`, `Page`, `Taken`, `Counts`, `request_sync` only), `events` |
| `projects` (second slice, adds; operator, 2026-11-14; ruling number to be assigned; PRJ-027 to PRJ-035, OD-E14; K-E1 Section 13) | `sources` (`Scheduling`, `ask`, `change`, `forget`, `standing`, `Keep`, `Reply`, `LiveFailure` too); `access` (`operator_only` only, and only in the files named `graphql.rs` of its sub-folders: the import gate reads `<domain>/graphql.rs` alone as the graphql module, so this edge is listed under `domains`); crates `provider-github` (already) and `source-exchange` (`Change`, its parts, `Settled`) |
| `timeline` (engine, new; operator, 2026-11-16; ruling number to be assigned; K-E3) | `events` (`before`, `EventSummary`, `Source` only) |
| `links` (engine, new; same ruling; K-E4) | `events` (the trail types) |
| `agent_usage` (R-135, operator, 2026-11-17; AIS-020; AU-2) | `devices` (`Device`, `list` only) |
| `containers` (work, new; R-149, operator, 2026-11-18; HOST-001, HOST-010) | `devices` (`Device`, `get`, `Error` only), `ssh` (`link`, `FixedCommand`, `HexDigits`, `CommandOutput`, `LinkError` only), `credentials` (`Vault` only) |
| `catalogue` (work, new; R-299, R-300; HOST-047; task H38) | — (no domain; the crate `catalogue` and the leaf `ids` only) |
| `metrics` (R-135, adds) | `agent_usage` (`record` only, in the push transaction) |
| `files` (engine, new; R-148; K-E4 Sections 9, 10) | `events` (the trail types, `audit`, `NewAudit`), `extensions` (`Key`, `Sensitivity` only: the data class of the extension of a form, VIS-002), `access` (`identify`, `client_addr`, `Identity`, `Scope` only, and only in `files/routes.rs`: the bearer of `/files/v1`); the leaves `sealing_keys` and `records` |
| `places` (engine, new; R-148; K-E4 Section 8) | `links` (`retarget`, `retarget_named`, `count`, `Titles`, `Unavailable` only), `events` (the trail types), `records` (leaf) |
| `locations` (engine, new; R-145; K-E4 Sections 11 to 13) | `places` (`Place`, `list`, `place_of_site` only), `events` (the trail types, `audit`, `NewAudit`, `ActorKind`), `access` (`client_addr` only, and only in `locations/routes.rs`: the address of a post of the OwnTracks path; OD-E24 names this edge, which K-E4 Section 13 does not list); the leaves `sealing_keys` and `records`. (R-261) `sources` (`Intake`, `Page`, `Taken`, `Counts`, `IntakeError` only: the intake of the Dawarich pull) and `extensions` (`INTEGRATIONS` and `Key` only); the crate `provider-dawarich` |
| `sessions` (R-145; K-E4 Section 12, adds) | `locations` (`record_transfer`, `Transfer`, `Direction`, `Actor`, `Error` only: the files tunnel stores the context of a transfer in the transaction of its trail). No cycle: `locations` does not import `sessions` |
| `reminders` (engine, new; R-148; K-E6; E0-7 T2 and T3) | `places` (`Place`, `Centre`, `list`, `current`), `locations` (`Sample`, `Seal`, `received`, `inside`, `RULE`), `notifications` (`queue_reminder`, `channels_exist`, `Reminder`, `Error`), `credentials` (`Vault`; `operational::keys`, `OperationalKey`, `VersionState`), `events` (`record`, `after`, `NewEvent`, `EventSummary`, `Bus`, the trail types), `extensions` (`on_keys` only: an extension that is off is not read; K-E6 Section 6 does not list this edge); the leaf `records`. The edge to `sources` of K-E6 Section 6 joins with the health of the accounts |
| `notifications` (R-148; K-E6 Section 5, adds) | no new edge. New for the engine: `Reminder`, `Queued`, `queue_reminder(conn, vault, channels, (reminder, now)) -> Queued` and `RANDOM_TOPIC_MIN`. The vault opens the topic of an ntfy channel for the rule of REM-008 |
| `credentials` (operational keys, adds; R-147, R-148; K-E5 Section 2) | `events` (the trail types, `audit`, `NewAudit`, `ClientAddr`, `Operator`); `access` (`Authenticated`, `operator_only`) only in `credentials/graphql.rs`, a graphql edge. A device is a `DeviceId` value only: the consumer rows reference `devices.devices` with a cascading foreign key, so `credentials` imports no `devices` code. `metrics` (its daemon ingest) calls `credentials::operational::{fetch, current_for, record_held}` |

(Phase 3) No cycle: `notifications` imports neither `checks` nor `sessions`. `ssh` and `trust` import neither `sessions` nor `checks`.

GraphQL composition edges (R-011). Only the file `<domain>/graphql.rs` may also import these domains' public faces, to resolve K1 fields that join domains. These edges never permit a `store` import and do not count toward cycles. They live in `quality/hub-modules.json` under `graphql`.

| Domain `graphql.rs` | May also import | For |
| --- | --- | --- |
| `devices` | `access` (R-022), `checks`, `sessions` (replaced in Phase 3 by `trust`, see the Phase 3 row below) | the `Authenticated` guard; `Device.status`, `Device.checks`, `Device.uptime`, `DeviceFilter.status`, `DeviceSort.STATUS`/`UPTIME`, `createDevice(addDefaultCheck)`, `SshSettings.hostKey` |

| `devices` (Phase 2, adds) | `metrics`, `web_interfaces` | `Device.readings`, `Device.metrics`, `Device.gpus`, `Device.webInterfaces`, `Device.webInterfaceSuggestions`; Phase 3: `Device.metricsStale` (MET-018, CR-18), `Device.storage` (MET-029), `Device.batteries` (MET-032) |
| `metrics`, `web_interfaces` (Phase 2) | `access` (R-022) | the `Authenticated` guard on their root fields |
| `builds` (Phase 3, WP-36) | `access` | the `Authenticated` and `BuilderSignedIn` guards |
| `vpn` (Phase 3, VP-5) | `access` | the `Authenticated` guard |
| `checks`, `maintenance`, `events` | `access` (R-022) | the `Authenticated` guard on their root fields (API-003) |
| `devices` (Phase 3) | `trust` (replaces `sessions` for these fields) | `SshSettings.hostKey`, `RdpSettings.serverCertificate` |
| `checks` (Phase 3) | `notifications` | `Check.notifications` |
| `notifications`, `retention`, `trust` (Phase 3) | `access` (R-022) | the `Authenticated` guard |
| `extensions`, `sources`, `projects` (operator, 2026-11-13; ruling number to be assigned) | `access` | the `Authenticated` guard and the login-only guard on `setExtension`, `connectGithubApp`, `removeSourceAccount`; and, from Connect GitHub, on `startGithubConnect`, `openGithubConnect`, `cancelGithubConnect` (they join `API_KEY_REFUSED_ROOTS`); and, from the second Projects slice (operator, 2026-11-14; ruling number to be assigned), on `codeFile`, `mergePullRequest`, `mergePullRequests`, `rerunWorkflowRun`, `rerunFailedWorkflowJobs`, `cancelWorkflowRun`, `dispatchWorkflow`, `setCommitStatus` (they join `API_KEY_REFUSED_ROOTS`) |
| `places` (R-148; K-E4 Section 8) | `access`, `devices` | the `Authenticated` guard on `places`, `place` and the six place mutations; `devices::sites` to check that a site exists in `setSitePlace` |
| `timeline`, `links` (operator, 2026-11-16; ruling number to be assigned; K-E3, K-E4) | `access` | the `Authenticated` guard on `timeline`, `recordLinks`, `linkRecords`, `unlinkRecords` |
| `agent_usage` (R-135) | `access` | the `Authenticated` guard on `agentUsage` |
| `files` (R-148; K-E4) | `access` | the `Authenticated` guard on `attachments`, `deleteAttachment`, `createArchive`, `deleteArchive`, `extractionProposals`, `proposeExtraction`, `confirmExtraction`, `rejectExtraction` |
| `locations` (R-145; K-E4 Section 13) | `access` | the `Authenticated` guard on `locationSamples`, `locationIngestTokens`, `transferContext`, `recordLocationSamples`, `createLocationIngestToken`, `revokeLocationIngestToken`; `operator_only` on `createLocationIngestToken`, which joins `API_KEY_REFUSED_ROOTS` (it returns a secret) |
| `sources` (R-261; K-E1 Section 7b, adds) | `access` | `operator_only` on `connectDawarich`, which joins `API_KEY_REFUSED_ROOTS` (it takes a secret) |
| `containers` (R-149; R-150 adds) | `access` | the `Authenticated` guard on `containers`, `containerLog`, `containerDetail`, `containerStorage`, `containerNetworks`, `containerImageUpdates`, `composeFileCheck` and `composeFromDockerRun`; `containerLog` and `containerDetail` join `API_KEY_REFUSED_ROOTS` (a log, a command or a label can hold a secret) |
| `catalogue` (task H38) | `access` | the `Authenticated` guard on `catalogueEntries`, `catalogueEntry` and `installCatalogueEntry`; none joins `API_KEY_REFUSED_ROOTS`: a key and an agent may ask for the install of an entry of the tested catalogue (R-234) |

`sessions` already depends on `access`. Through these edges a `graphql.rs` uses only `access::Authenticated`; `Operator` and `ClientAddr` come from `events` (R-019).

Outside `graphql.rs`, `devices` never imports `checks` (that edge would make a cycle). Device deletion relies on foreign keys and the checks retention job instead (see below).

Any other edge is a contract change.

## devices (WP-06)

```rust
pub struct Device { pub id: DeviceId, pub site: SiteId, pub name: String, pub host: String,
                    pub ip: Option<IpAddr>, pub kind: String, pub os: OperatingSystem,
                    pub tags: Vec<String>, pub maintenance: bool, pub ssh: SshSettings }
pub struct SshSettings { pub enabled: bool, pub port: u16, pub username: String, pub auth: SshAuth }

pub async fn get(db: &Db, id: DeviceId) -> Result<Device, Error>;
pub async fn list(db: &Db) -> Result<Vec<Device>, Error>;
pub async fn in_maintenance(db: &Db, id: DeviceId) -> Result<bool, Error>;
```

Delete (DEV-015) is one `DELETE` in `devices`: foreign keys cascade to `device_tags`, `ssh_settings`, `credentials.secrets`, `checks.checks` and `trust.host_keys` (`sessions.host_keys` before Phase 3; CR-11). Heartbeats (partitioned, no foreign key) whose check no longer exists are removed by the checks retention job. Maintenance targets are text; a target that names a deleted device matches nothing. Logs and session history keep their copied names.

(Phase 2, WP-21) `Device` gains the fields of K1 `Device`:

```rust
pub struct RdpSettings { pub enabled: bool, pub port: u16, pub username: String, pub domain: Option<String>,
    pub security: RdpSecurity, pub ignore_certificate: bool, pub resize: ResizeMethod, pub audio_out: bool,
    pub microphone: bool, pub shared_drive: bool }
pub struct VncSettings { pub enabled: bool, pub port: u16, pub username: Option<String> }
pub struct TransferPolicy { pub upload: bool, pub download: bool, pub copy: bool, pub paste: bool }
pub struct MacAddress([u8; 6]);            // parsed at the API edge; Display renders aa:bb:cc:dd:ee:ff
pub enum MetricsSource { None, Ssh, Daemon }   // Daemon (Phase 3, CR-15): set only by metrics enrolment
pub async fn wake_relay(db: &Db, site: SiteId) -> Result<Option<Device>, Error>;
```

(Phase 3, WP-47):

```rust
pub struct Device { ..., pub session_limit: Option<std::num::NonZeroU8> }
```

(Phase 3, CR-24) the functions other domains already call, listed here so K3 is complete again:

```rust
pub struct Distribution { /* the Linux distribution read from the device, DEV-016 */ }
pub async fn record_distribution(db: &Db, id: DeviceId, distribution: &Distribution) -> Result<(), Error>;   // metrics collector
pub async fn record_detected_os(db: &Db, id: DeviceId, os: OperatingSystem) -> Result<bool, Error>;   // metrics collector; DEV-025, writes only while os = 'other'
pub async fn lock(conn: &mut PgConnection, id: DeviceId) -> Result<(), Error>;                                  // FOR UPDATE; metrics enrolment
pub async fn lock_metrics_source(conn: &mut PgConnection, id: DeviceId) -> Result<MetricsSource, Error>;       // FOR UPDATE; MET-011, MET-018
pub async fn use_daemon_metrics(conn: &mut PgConnection, id: DeviceId) -> Result<String, Error>;                // daemon enrolment only; R-092 D-7
pub async fn session_limit<'e>(db: impl PgExecutor<'e>, id: DeviceId) -> Result<Option<NonZeroU8>, Error>;   // RA-036
```

A new cross-domain function is listed here in the same commit as its code (CR-24).

## credentials (WP-07)

```rust
/// Pure crypto type, no tables. XChaCha20-Poly1305 with the key from ANIMA_KEY_FILE (SEC-003, SEC-004).
#[derive(Clone)] pub struct Vault { /* key material, zeroized on drop */ }
impl Vault {
    pub fn from_key_file(path: &Path) -> Result<Self, Error>;
    /// `context` is authenticated but not encrypted (AEAD associated data). It binds the
    /// ciphertext to its row, so a sealed value copied to another row does not open (R-010).
    pub fn seal(&self, plaintext: &Secret, context: &[u8]) -> Sealed;     // nonce + ciphertext + key_id
    pub fn open(&self, sealed: &Sealed, context: &[u8]) -> Result<Secret, Error>;
}
/// Secret bytes: zeroized on drop; no Debug, Display, Serialize or Clone (LOG-007, SEC-005).
pub struct Secret(/* zeroize::Zeroizing<Vec<u8>> */);

pub enum SshSecret { Password(Secret), PrivateKey { key: Secret, passphrase: Option<Secret> } }
pub async fn store_ssh(conn: &mut PgConnection, vault: &Vault, device: DeviceId, s: SshSecret) -> Result<(), Error>;
pub async fn load_ssh(db: &Db, vault: &Vault, device: DeviceId) -> Result<Option<SshSecret>, Error>;
pub async fn has_ssh(db: &Db, device: DeviceId) -> Result<bool, Error>;
/// The devices in `devices` that have a stored SSH secret, in one query (API-005, R-017).
pub async fn with_ssh(db: &Db, devices: &[DeviceId]) -> Result<HashSet<DeviceId>, Error>;
/// Deletes every SSH secret of the device. Returns true if a secret was removed (R-010).
pub async fn clear_ssh(conn: &mut PgConnection, device: DeviceId) -> Result<bool, Error>;
```

(Changed, operator, 2026-11-13; ruling number to be assigned; SEC-011.) The device's SSH, RDP and VNC credentials are bound to the device host they were entered for. Each writer gives that host and each reader the host it is about to connect to, as `(device, host)`; a reader gets `Error::EnteredForOtherHost` for any other host and must send nothing. These lines replace the WP-07 and WP-21 signatures of the same functions:

```rust
pub async fn store_ssh(conn: &mut PgConnection, vault: &Vault, (device, host): (DeviceId, &str), s: SshSecret) -> Result<(), Error>;
pub async fn load_ssh(db: &Db, vault: &Vault, (device, host): (DeviceId, &str)) -> Result<Option<SshSecret>, Error>;
pub async fn store_password(conn: &mut PgConnection, vault: &Vault, (device, host): (DeviceId, &str), (kind, s): (PasswordKind, Secret)) -> Result<(), Error>;
pub async fn load_password(db: &Db, vault: &Vault, (device, host): (DeviceId, &str), kind: PasswordKind) -> Result<Option<Secret>, Error>;
pub enum Error { /* as before */ EnteredForOtherHost }
```

Context values (R-010). Each caller uses exactly this byte string, so a value sealed for one row cannot be opened for another:

| Use | `context` |
| --- | --- |
| SSH secret, RDP or VNC password, sealed before SEC-011 (inside `credentials`) | `credentials.secrets:<device_id>:<kind>` |
| (SEC-011) SSH secret, RDP or VNC password with its host: plaintext is the host length (2 bytes, big-endian), the host text, the secret (inside `credentials`) | `credentials.secrets.host:<device_id>:<kind>` |
| TOTP secret (WP-05) | `access.operators.totp:<operator_id>` |
| (Phase 3) Notification channel secrets (WP-45) | `notifications.channels:<channel_id>` |
| (operator, 2026-11-13; ruling number to be assigned; K-E1) Source secret, such as a GitHub App key | `credentials.source_secrets:<account_id>` |
| (R-147; K-E5 Section 2) One version of an operational key | `credentials.operational_key_versions:<version_id>` |

The GraphQL mutations `setSshCredential` and `clearSshCredential` belong to `devices` (they return `SshSettings`). They call `store_ssh` / `clear_ssh` in the same transaction as the audit entry (`events::audit`), and the audit `change` never contains the secret.

`load_ssh` has exactly one caller: `sessions` (and the connection test, which `sessions` owns). The GraphQL API never returns a secret, only `credentialStored: Boolean`.

(Phase 2, WP-21):

```rust
pub enum PasswordKind { Rdp, Vnc }         // kinds rdp_password, vnc_password; context as R-010
pub async fn store_password(conn: &mut PgConnection, vault: &Vault, device: DeviceId, kind: PasswordKind, s: Secret) -> Result<(), Error>;
pub async fn load_password(db: &Db, vault: &Vault, device: DeviceId, kind: PasswordKind) -> Result<Option<Secret>, Error>;
pub async fn with_password(db: &Db, devices: &[DeviceId], kind: PasswordKind) -> Result<HashSet<DeviceId>, Error>;
pub async fn clear_password(conn: &mut PgConnection, device: DeviceId, kind: PasswordKind) -> Result<bool, Error>;
```

(operator, 2026-11-13; ruling number to be assigned; K-E1, PRJ-018.) Source secrets, in the new file `credentials/source.rs`. The row is in `credentials.source_secrets` (K2 migration 0221). `SourceAccountId` and `SourceSecretId` are leaf IDs:

```rust
pub async fn store_source_secret(conn: &mut PgConnection, vault: &Vault, account: SourceAccountId, s: Secret) -> Result<SourceSecretId, Error>;
pub async fn load_source_secret(db: &Db, vault: &Vault, (id, account): (SourceSecretId, SourceAccountId)) -> Result<Secret, Error>;
pub async fn clear_source_secret(conn: &mut PgConnection, (id, account): (SourceSecretId, SourceAccountId)) -> Result<bool, Error>;   // deletes only the row of this id and account
pub enum Error { /* as before */ UnknownSourceSecret }   // load_source_secret: no row has this id
```

A secret opens only with its own account ID (context in the table above). `load_source_secret` has one caller: the token minter in `engine/sources`. For a GitHub App key the plaintext is built by `sources` (K-E1 Section 6); `credentials` treats it as opaque bytes.

(R-147, R-148; K-E5 Section 2.) Operational keys, in `credentials/operational/` (rows in `credentials.operational_keys`, `operational_key_versions` and `operational_key_consumers`, K2 range 2070). The K1 part is `credentials/graphql.rs` (`credentials::graphql::{Query, Mutation}`). Each change runs in the caller's transaction and writes its rows, one audit entry and one event through the `Trail` it takes:

```rust
pub async fn keys(db: &Db, now: DateTime<Utc>) -> Result<Vec<OperationalKey>, operational::Error>;
pub async fn key(db: &Db, name: &str, now: DateTime<Utc>) -> Result<Option<OperationalKey>, operational::Error>;
pub async fn create(conn: &mut PgConnection, vault: &Vault, new: NewKey, trail: Trail<'_>) -> Result<OperationalKey, operational::Error>;
pub async fn update(conn: &mut PgConnection, name: &str, metadata: Metadata, trail: Trail<'_>) -> Result<OperationalKey, operational::Error>;
pub async fn rotate(conn: &mut PgConnection, vault: &Vault, (name, value, rotated_at_provider): (&str, Secret, bool), trail: Trail<'_>) -> Result<Ending, operational::Error>;
pub async fn revoke(conn: &mut PgConnection, name: &str, trail: Trail<'_>) -> Result<Ending, operational::Error>;
pub async fn remove(conn: &mut PgConnection, name: &str, trail: Trail<'_>) -> Result<Ending, operational::Error>;
pub async fn grant(conn: &mut PgConnection, grant: Grant, trail: Trail<'_>) -> Result<OperationalKey, operational::Error>;
pub async fn ungrant(conn: &mut PgConnection, grant: Grant, trail: Trail<'_>) -> Result<OperationalKey, operational::Error>;
pub async fn fetch(db: &Db, vault: &Vault, (device, name): (DeviceId, &str), (now, client): (DateTime<Utc>, ClientAddr)) -> Result<Option<FetchedKey>, operational::Error>;   // None: the one NO_KEY answer
pub async fn current_for(db: &Db, device: DeviceId, now: DateTime<Utc>) -> Result<Vec<KeyVersion>, operational::Error>;
pub async fn record_held(db: &Db, device: DeviceId, held: &[KeyVersion], now: DateTime<Utc>) -> Result<(), operational::Error>;
pub async fn forget_ended(db: &Db, now: DateTime<Utc>) -> Result<u64, operational::Error>;   // the daily retention run of upkeep.rs
pub enum Error { Database, Trail, Sealing, Invalid(Vec<Problem>), NameTaken, UnknownKey, UnknownDevice, NoGrant }
```

`FetchedKey`, `KeyVersion` and `KeyValue` are the K8 types of `daemon_core::wire`. `metrics::daemon_router` takes `((db, bus, clock), feed, vault)`: the vault opens a fetched key.

## checks (WP-09)

```rust
// HeartbeatSummary is events::HeartbeatSummary (R-019).
pub async fn device_status(db: &Db, device: DeviceId, now: DateTime<Utc>) -> Result<Status, Error>;   // CHK-004
/// Started by main.rs; owns the scheduler task, the watchdog (CHK-010) and retention (DB-005).
pub struct Probing { pub vault: credentials::Vault, pub policy: AddressPolicy }   // R-030; main.rs builds AddressPolicy::new(config.probe_deny)
pub fn spawn_scheduler(db: Db, bus: events::Bus, clock: Clock, probing: Probing) -> tokio::task::JoinHandle<()>;   // R-028, R-030
/// PUSH checks (CHK-*): `GET|POST /push/{token}`; the token is the only authentication. main.rs mounts it behind the NET-002 layer (R-028).
pub fn push_router(db: Db, bus: events::Bus, clock: Clock) -> axum::Router;
```

Functions for the devices GraphQL composition (R-011; WP-09 adds them and their use in `devices/graphql.rs`):

```rust
pub async fn statuses(db: &Db, devices: &[DeviceId], now: DateTime<Utc>) -> Result<HashMap<DeviceId, Status>, Error>;
pub async fn uptimes(db: &Db, devices: &[DeviceId], window: UptimeWindow, now: DateTime<Utc>) -> Result<HashMap<DeviceId, f64>, Error>;
pub async fn for_device(db: &Db, device: DeviceId) -> Result<Vec<Check>, Error>;
pub async fn for_devices(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<Check>>, Error>;   // every requested device has an entry (empty when none); checks in name order; API-005 batch for Device.checks (R-037)
/// Adds the default ICMP check in the caller's transaction (spec 6.2).
pub async fn add_default_check(conn: &mut PgConnection, device: DeviceId) -> Result<CheckId, Error>;
/// The last address a check resolved for each device (R-028); `Device.ip` uses it when the host is a name.
pub async fn resolved_ips(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, IpAddr>, Error>;
```

`checks::graphql::Subscription` owns K1 `deviceStatusChanged` and `heartbeatAdded`, built from the Bus (R-028). HTTP check secrets (basic password, bearer token, Authorization and Cookie header values) are sealed by checks with `credentials::Vault`, context `checks.checks.http:<check_id>` (R-028, SEC-003).

Check evaluation is pure: `fn evaluate(prev: &CheckState, probe: ProbeResult, settings: &CheckSettings, now) -> (CheckState, Status)`. `CheckSettings` holds interval, retries, retry interval, degraded threshold and invert; `CheckConfig` is the probe configuration, the K1 union (R-030).

(Phase 2, WP-29):

```rust
pub async fn http_urls(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<(CheckId, String)>>, Error>;   // WEB-002
```

`AddressPolicy` is now `crate::targets::AddressPolicy` (see the leaf modules).

(Phase 3, WP-46; handoff edits by WP-45):

```rust
pub async fn defaults(db: &Db) -> Result<CheckDefaults, Error>;          // D-26; add_default_check uses it
/// GET /metrics, Prometheus text format 0.0.4 (INT-001, INT-002), cached 10 s. lib.rs mounts it behind
/// access::bearer_layer(Scope::Read); checks imports no access item for it.
pub fn exposition_router(db: Db, clock: Clock) -> axum::Router;
// Retention: drop_expired(db, now, heartbeats_keep: chrono::Days) is wiring only (moved to upkeep.rs).
```

## maintenance (WP-09)

```rust
/// Pure (MW-001..004). No clock read.
pub fn is_active(rule: &WindowRule, tz: chrono_tz::Tz, now: DateTime<Utc>) -> bool;
pub async fn active_for(db: &Db, device: DeviceId, check: Option<CheckId>, tags: &[String], now: DateTime<Utc>) -> Result<bool, Error>;
/// Every window active at `now`, in one query (API-005, R-028).
pub async fn active_targets(db: &Db, now: DateTime<Utc>) -> Result<ActiveTargets, Error>;
impl ActiveTargets { pub fn covers(&self, device: DeviceId, check: Option<CheckId>, tags: &[String]) -> bool; }
```

## sessions (WP-10)

```rust
/// Tickets, live tunnels and host-key observations; built once in main.rs, added with `.data(live.clone())` (R-032).
#[derive(Clone)] pub struct Live { /* Arc inside */ }
pub async fn start(db: &Db, clock: &Clock, live: &Live, op: &Operator, client: ClientAddr, req: StartRequest) -> Result<Ticket, Error>;
/// For `SshSettings.hostKey` (R-011; WP-10 adds its use in `devices/graphql.rs`).
pub async fn host_key(db: &Db, device: DeviceId) -> Result<Option<HostKey>, Error>;
/// One query for the `SshSettings.hostKey` loader (API-005, R-032).
pub async fn host_keys(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, HostKey>, Error>;
/// axum route for GET /tunnel, the RDP and VNC tunnel (see tunnel.md). `guacd` is the K7 `host:port` text, resolved at connect time (R-032); `drive_root` is the K7 `ANIMA_GUACD_DRIVE_ROOT` under which guacd keeps shared drives (CR-1). Mounted in `tunnels()`; refuses SSH tickets (R-062).
pub fn tunnel_router(db: Db, vault: credentials::Vault, bus: events::Bus, clock: Clock, guacd: String, drive_root: String, live: Live) -> axum::Router;
/// axum route for GET /terminal, the SSH terminal tunnel (tunnel.md Section 8, R-062). main.rs mounts it as it mounted /tunnel: logged, behind the NET-002 layer.
pub fn terminal_router(db: Db, vault: credentials::Vault, bus: events::Bus, clock: Clock, live: Live) -> axum::Router;
/// Wiring only: at start, ends what a previous process left open (as close_stale) and returns the Live that issues tickets, with how many rows it ended (R-041).
pub async fn open(db: &Db, now: DateTime<Utc>) -> Result<(Live, u64), Error>;
/// Wiring only: at start, end rows a crashed hub left open with HUB_SHUTDOWN, one `session.end` audit entry per row in the same transaction (R-032, R-040).
pub async fn close_stale(db: &Db, now: DateTime<Utc>) -> Result<u64, Error>;
/// Wiring only: call on the shutdown signal, before the runtime stops; upgraded tunnels are not ended by the server's drain.
/// Signals tunnels, withdraws pending and spent tickets and refuses new ones, ends those rows HUB_SHUTDOWN with audit, waits up to 5 s (R-040).
pub async fn shut_down(db: &Db, live: &Live, now: DateTime<Utc>) -> Result<(), Error>;
```

(Phase 2, WP-27): the one SSH path for hub-run work (P2-003).

```rust
/// A command made only from static text and hub-rendered values; operator text cannot enter it (CHK-012).
pub struct FixedCommand { /* program: &'static str, args: Vec<Rendered> */ }
impl FixedCommand { pub const fn new(program: &'static str) -> Self; pub fn mac(self, mac: devices::MacAddress) -> Self; }
/// A signed-in SSH connection pinned to the accepted host key, to a target that passed the target rule and deny list (P2-004).
pub struct SshLink { /* russh handle */ }
pub async fn link(db: &Db, vault: &credentials::Vault, policy: &AddressPolicy, device: &Device, now: DateTime<Utc>) -> Result<SshLink, LinkError>;
impl SshLink { pub async fn run(&mut self, c: &FixedCommand, limit: Duration) -> Result<CommandOutput, LinkError>; }
pub async fn wake(db: &Db, vault: &credentials::Vault, policy: &AddressPolicy, bus: &events::Bus, device: DeviceId, now: DateTime<Utc>) -> Result<WakeSent, Error>;   // replaced below (operator, 2026-11-13)
pub fn files_router(db: Db, vault: credentials::Vault, bus: events::Bus, clock: Clock, live: Live) -> axum::Router;   // GET /files, tunnel.md Section 9
```

(Phase 2, WP-22):

```rust
pub async fn server_certificates(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, ServerCertificate>, Error>;
// tunnel_router now serves RDP and VNC and is mounted; Live also broadcasts SessionChange (D-15).
```

Wake-on-LAN lives in `sessions` (RA-031 is a remote access rule), so a session start can wake a device without a `sessions` and `power` cycle.

(Phase 3, changed: WP-41 moves, WP-47, WP-48):

```rust
/// Everything the tunnels need, built once at start in `lib.rs::tunnels()`, which main.rs calls (replaces the long
/// tunnel_router argument lists; CR-12).
#[derive(Clone)] pub struct Serving { pub db: Db, pub vault: credentials::Vault, pub bus: events::Bus,
    pub clock: Clock, pub live: Live, pub guacd: String, pub drive_root: String }
pub fn tunnel_router(s: &Serving) -> axum::Router;      // signature change only
pub fn terminal_router(s: &Serving) -> axum::Router;    // signature change only
// Session start, files and wake keep their Phase 2 signatures; wake now uses ssh::link.
```

(Changed, operator, 2026-11-13; ruling number to be assigned; LOG-013.) `wake` takes the actor kind of
whoever asked for the wake, and its event records it: the kind of the request for `wakeDevice` and for the
wake before a `startSession`, `ActorKind::Hub` for a wake the hub decides by itself. The stores are one
argument. `wakeDevice` writes the audit entry `device.wake` for each attempt on a known device, sent or not;
a failed attempt's change is `{ "sent": false, "reason": <the fixed sentence> }`. This line replaces the
WP-27 signature:

```rust
pub async fn wake((db, vault, policy, bus): (&Db, &credentials::Vault, &AddressPolicy, &events::Bus), device: DeviceId, by: ActorKind, now: DateTime<Utc>) -> Result<WakeSent, Error>;
```

The files tunnel's refusal events (the device's host key changed, SEC-007; the SSH credential was entered
for another host, SEC-011) record the kind of the request that opened the tunnel, not Hub (LOG-013).

## metrics (Phase 2, WP-28)

```rust
pub struct Collecting { pub vault: credentials::Vault, pub policy: AddressPolicy }
#[derive(Clone)] pub struct Feed { /* broadcast of MetricsAdded */ }    // D-15
pub fn spawn_collector(db: Db, feed: Feed, bus: events::Bus, clock: Clock, collecting: Collecting) -> JoinHandle<()>;   // MET-001
pub async fn readings(db: &Db, devices: &[DeviceId], now: DateTime<Utc>) -> Result<HashMap<DeviceId, Readings>, Error>;
pub async fn series(db: &Db, device: DeviceId, range: MetricRange, now: DateTime<Utc>) -> Result<Vec<MetricSeries>, Error>;
pub async fn gpus(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<Gpu>>, Error>;
/// (Phase 3, operator ruling 2026-11-12) One device's storage drive by drive: its drives with the
/// latest use of their mounts and their activity, then mounts on no known drive (MET-026, MET-029).
pub async fn storage(db: &Db, device: DeviceId, now: DateTime<Utc>) -> Result<Storage, Error>;
// Wiring only: create_partitions_ahead, drop_expired_partitions (as events and checks).

// (Phase 3, WP-54; K8, R-092)
/// POST /daemon/v1/{enrol,push,renew} (K8). main.rs merges it behind the NET-002 layer.
pub fn daemon_router(db: Db, feed: Feed, bus: events::Bus, clock: Clock) -> axum::Router;
/// Marks DAEMON devices stale after 30 s with no batch, with one event each way (MET-018);
/// spawned from spawns.rs (P3-007). Takes the event bus, not the feed (CR-18).
pub fn spawn_stale_watch(db: Db, bus: events::Bus, clock: Clock) -> JoinHandle<()>;
/// Of `devices`, those whose daemon is not fresh: never pushed, revoked or stale (MET-018, CR-18).
pub async fn stale_daemons(db: &Db, devices: &[DeviceId]) -> Result<HashSet<DeviceId>, Error>;

// (operator, 2026-11-15, CAP-020 to CAP-031; ruling number to be assigned)
pub struct Capabilities { pub device: DeviceId, pub inventory: daemon_core::inventory::Inventory, pub found_at: DateTime<Utc> }
pub struct CapabilityFilter { pub tool: Option<String>, pub build_target: Option<String> }
/// The tool inventory each daemon sent, kept when `filter` admits it: `tool` usable, `build_target` installed (CAP-030),
/// each with the hardware values its daemon sent, when it sent them (CAP-032; R-165, task L1-1).
/// Each tool is usable with its version or not usable with a fixed `daemon_core::inventory::Reason` (CAP-024).
/// metrics::graphql owns Query.capabilities and joins each to its Device (devices::list), by device name.
pub async fn capabilities(db: &Db, filter: &CapabilityFilter) -> Result<Vec<Capabilities>, Error>;
/// (R-122) The version of each device's daemon from its last batch; no entry when it sent none.
pub async fn daemon_versions(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, String>, Error>;
/// (MET-032) The batteries each device's daemon last listed, by name; no entry when it has none.
pub async fn batteries(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<Battery>>, Error>;
```

## agent_usage (R-135, operator, 2026-11-17; AIS-018 to AIS-024; AU-2)

```rust
/// Stores an accepted `agentUsage` member of a daemon batch in the batch's transaction (K8 Section 4).
pub async fn record(conn: &mut PgConnection, device: DeviceId, usage: &daemon_core::agent_usage::AgentUsage) -> Result<(), Error>;
/// The sums an Ask asks for, the total, and the usage state of each device the filter admits.
/// agent_usage::graphql owns Query.agentUsage and joins each row to its Device (devices::list).
pub async fn usage(db: &Db, ask: &Ask) -> Result<Report, Error>;
/// The estimate of the cost of tokens of a model at the public list prices of PRICES_AS_OF; None for an unknown model (AIS-021).
pub fn prices::estimate(model: &str, tokens: TokenSums) -> Option<f64>;
```

## web_interfaces (Phase 2, WP-30)

```rust
pub async fn for_devices(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<WebInterface>>, Error>;
pub async fn suggestions(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<Suggestion>>, Error>;
```

## notifications (Phase 3, WP-45)

```rust
/// What checks tells notifications about one heartbeat, inside the heartbeat transaction (D-16).
pub struct Observation {
    pub check: CheckId, pub check_name: String, pub check_type: &'static str,
    pub device: (DeviceId, String), pub site: String,
    pub previous: Status, pub status: Status, pub in_maintenance: bool,
    pub at: DateTime<Utc>, pub message: Option<String>,
    pub certificate_not_after: Option<DateTime<Utc>>, pub domain_not_after: Option<DateTime<Utc>>,
}
/// Pure decision, then outbox rows in the caller's transaction. Sends nothing itself.
pub async fn observe(conn: &mut PgConnection, o: &Observation) -> Result<(), Error>;
/// Turns on the channels marked "on by default" for a new check, in createCheck's transaction (NOT-003).
pub async fn adopt_defaults(conn: &mut PgConnection, check: CheckId) -> Result<(), Error>;
/// For the Check.notifications loader (API-005).
pub async fn for_checks(db: &Db, checks: &[CheckId]) -> Result<HashMap<CheckId, CheckNotifications>, Error>;
pub struct Sending { pub vault: credentials::Vault, pub policy: AddressPolicy }
pub fn spawn_dispatcher(db: Db, bus: events::Bus, clock: Clock, sending: Sending) -> JoinHandle<()>;
// Wiring only: create_partitions_ahead, drop_expired_deliveries(db, now, keep: chrono::Days).
```

## retention (Phase 3, WP-46)

```rust
pub enum Subject { Heartbeats, Metrics, Events, Deliveries }
pub struct Periods { /* days per subject */ }
impl Periods { pub fn keep(&self, s: Subject) -> chrono::Days; }
pub async fn periods(db: &Db) -> Result<Periods, Error>;   // wiring (upkeep.rs) only
```

## ssh (Phase 3, WP-41; moved from sessions, Q2-05)

```rust
pub struct FixedCommand;  pub struct CommandOutput;  pub struct SshLink;  pub enum LinkError;   // as Phase 2 K3
pub async fn link(db: &Db, vault: &credentials::Vault, policy: &AddressPolicy, device: &Device, now: DateTime<Utc>) -> Result<SshLink, LinkError>;
// The russh probe (testConnection, R-034) moves here unchanged; the public-key types are in `trust` (CR-7).
```

## trust (Phase 3, WP-41; moved from sessions, Q2-05)

```rust
pub async fn host_key(db: &Db, device: DeviceId) -> Result<Option<HostKey>, Error>;
pub async fn host_keys(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, HostKey>, Error>;
pub async fn server_certificates(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, ServerCertificate>, Error>;
// trust::graphql owns acceptHostKey and acceptServerCertificate (no K1 change).
// The public-key types (PublicKey, Fingerprint, BadKey, BadFingerprint) are in trust (CR-7).
pub struct Sightings;  pub enum Conflict;  pub enum Error;   // CR-11: what connection tests saw, shared with sessions::Live; accept conflicts
```

## builds (Phase 3, WP-36, R-107)

```rust
#[derive(Clone)] pub struct Updates { /* bounded broadcast of JobUpdate */ }   // BLD-081
#[derive(Clone, Copy)] pub struct BuildQueue<'a> { pub db: &'a Db, pub bus: &'a events::Bus, pub updates: &'a Updates }
pub const LEASE_SWEEP: Duration;   // 30 s (D-2)
/// Wiring only (upkeep.rs): ends every lease that ran out by `now`; the third end fails the job (BLD-006).
pub async fn expire_leases(queue: BuildQueue<'_>, now: DateTime<Utc>) -> Result<usize, Error>;

// Publishing (WP-36 T3, R-108, BLD-060 to BLD-070, D-24 to D-28)
pub struct Bucket;   pub enum BucketError;   // no Debug on Bucket; BucketError names a setting, never a value
impl Bucket { pub fn open(releases: &config::Releases) -> Result<Bucket, BucketError>; }   // main.rs, when ANIMA_RELEASES_ENDPOINT is set
pub struct UploadUrl { pub name: String, pub url: String, pub headers: Vec<(String, String)>, pub expires_at: DateTime<Utc> }   // no Debug
pub struct Grant<'a> { pub builder: &'a Builder, pub job: BuildJobId, pub files: &'a [UploadedFile] }
/// graphql only: presigned PUTs of 15 min for an `uploading` job, only for its target's release files, each with its
/// content type; records the files as its grant (D-26) and the latest URL expiry (D-28).
pub async fn upload_urls(db: &Db, bucket: &Bucket, grant: Grant<'_>, now: DateTime<Utc>) -> Result<Vec<UploadUrl>, Error>;
pub const PUBLISH_SWEEP: Duration;   // 10 s
/// Wiring only (upkeep.rs): verifies, copies and writes the feeds of jobs whose files are in and whose upload URLs
/// expired (D-25, D-28). Reads `clock` for each take, hold, signed request and end.
pub async fn publish_ready(queue: BuildQueue<'_>, bucket: &Bucket, http: &reqwest::Client, clock: &dyn Clock) -> Result<usize, Error>;
/// Wiring only (upkeep.rs), on a hub with no bucket: fails the jobs handed to the publisher with a fixed reason (D-25).
pub async fn publishing_off(queue: BuildQueue<'_>, now: DateTime<Utc>) -> Result<usize, Error>;
```

No other domain imports `builds`.

## engine: extensions and sources (operator, 2026-11-13; ruling number to be assigned; K-E7, K-E1)

Folders `crates/hub/src/engine/extensions/` and `crates/hub/src/engine/sources/` (group `engine/`, added by the task that adds its first code; the group face holds no code). Details are in `contracts/extensions/registry.md` and `contracts/extensions/sources.md`; migrations in `contracts/extensions/ownership.md`.

```rust
// engine::extensions
pub struct Key(/* &'static str */);
pub const PROJECTS: Key;
pub fn registry() -> &'static [Entry];
pub async fn on_keys(db: &Db) -> Result<HashSet<Key>, Error>;
pub async fn require_on(db: &Db, key: Key) -> Result<(), Error>;      // Error::Off maps to CONFLICT

// engine::sources
pub trait Intake { /* K-E1 Section 3 */ }
pub struct Registry { /* declarations and intakes, built once in wiring */ }
#[derive(Clone)] pub struct Feed { /* broadcast of account changes, as vpn::Feed */ }
pub struct Scheduling { pub pool: Db, pub bus: events::Bus, pub clock: Clock, pub vault: credentials::Vault,
                        pub registry: Registry, pub transport: Transport, pub feed: Feed }
pub fn spawn_scheduler(s: Scheduling) -> JoinHandle<()>;
/// Sets `next_sync_at` of the account to now, in the caller's transaction (used by `projects`).
pub async fn request_sync(conn: &mut PgConnection, account: SourceAccountId) -> Result<(), Error>;
```

### engine::sources: Connect GitHub (operator, 2026-11-14; ruling number to be assigned); K-E1 Sections 1, 7, 7a, 10a

```rust
// engine::sources (connect.rs): the one storing path, called by connectGithubApp and by the connect flow
pub async fn establish(s: &Scheduling, trail: Trail<'_>, input: Connection) -> Result<Established, Error>;
pub struct Established { pub id: SourceAccountId, pub created: bool, pub consumer_on: bool }
// engine::sources::registration (new module, in-memory attempt; no table)
pub struct Attempts { /* Arc<Mutex<Option<Attempt>>> and a broadcast sender; held in Reach like Tokens */ }
// engine::sources::acquisition
pub struct AppSigner { pub app_id: u64, /* key pair */ }   // read(app_id, pem); token(now, random); bind(installation_id) -> AppKey
pub fn convert(transport: &Transport, declaration: &Declaration, code: &str, now: DateTime<Utc>) -> Result<Converted, ConvertFailure>;
// crate source-exchange: AppKey.registration: Option<Registration>, Converted, Owner, Installation, ManifestFacts, Webhook (K-E1 Section 1)
// crate source-github: registration/manifest.rs (pure)
```

`sources/graphql/trail.rs` moves to `sources/trail.rs` so `connect.rs` can use `connected`. `graphql/registration.rs` holds the three roots and the subscription. `registration/routes.rs` is the only file that imports `access` beyond the guard.

## engine: timeline, links and the leaf `records` (operator, 2026-11-16; ruling number to be assigned; K-E3, K-E4)

Folders `crates/hub/src/engine/timeline/` and `crates/hub/src/engine/links/`, and the leaf file `crates/hub/src/engine/records.rs`. `records` is a leaf inside the group `engine/`, as `targets` is inside `fleet/`: it imports only `ids` and every domain may import it. Details are in `contracts/extensions/timeline.md` and `contracts/extensions/shared-records.md`. The timeline has no table (TML-001); `links` has migration 2030.

```rust
// engine::records (leaf)
pub struct Reference(/* "kind:id" */);
pub enum Subject { Repository(String), Place(PlaceId), Person(PersonId), Date(NaiveDate) }

// engine::timeline
pub trait Adapter { /* K-E3 Section 6 */ }
pub struct Adapters { /* built once in wiring */ }
pub fn event_log() -> Arc<dyn Adapter>;
pub async fn read(adapters: &Adapters, db: &Db, ask: &Ask) -> Result<Page, Error>;

// engine::links
pub async fn connect(conn: &mut PgConnection, new: NewLink, (at, by): (DateTime<Utc>, &str)) -> Result<(Link, Outcome), Error>;
pub async fn disconnect(conn: &mut PgConnection, id: RecordLinkId) -> Result<Option<Link>, Error>;
pub async fn touching(db: &Db, record: &Reference) -> Result<Vec<Link>, Error>;
pub async fn retarget(conn: &mut PgConnection, from: &Reference, to: &Reference) -> Result<u64, Error>;          // PLC-004
pub async fn retarget_named(conn: &mut PgConnection, ids: &[RecordLinkId], (from, to): (&Reference, &Reference)) -> Result<u64, Error>;   // split
pub async fn count(conn: &mut PgConnection, record: &Reference) -> Result<u64, Error>;   // all links of a record, not the newest 200
pub trait Titles { /* K-E4 Section 4: built; places is its first user */ }

// fleet::events, added for the timeline (TML-007)
pub async fn before(db: &Db, (from, before): (DateTime<Utc>, (DateTime<Utc>, Uuid)), sources: Option<Vec<Source>>, limit: i64) -> Result<Vec<EventSummary>, Error>;
```

`schema.rs` builds `timeline::Adapters` with `timeline::event_log()`. `fleet/events/model.rs` gains `AuditTarget::RecordLink`. `ids.rs` gains `RecordLinkId`, `PlaceId` and `PersonId`. No domain imports `timeline` or `links`.

## engine: files and the leaf `sealing_keys` (R-148; K-E4 Sections 9, 10; K-E5 Section 1)

Folder `crates/hub/src/engine/files/` and the leaf file `crates/hub/src/engine/sealing_keys.rs`. `sealing_keys` is a leaf inside the group `engine/`: it imports no domain, and every domain that seals a record or a file may import it (VLT-003). `files` has migration 2050.

```rust
// engine::sealing_keys (leaf; K-E5 Section 1)
pub struct SealingKeys { /* the folder */ }
pub struct SealingKey { /* 32 bytes, zeroed on drop */ }
impl SealingKeys { pub fn open(folder: Option<&Path>, credential_key_id: &[u8]) -> Result<Self, SealingKeyError>; pub const fn none() -> Self; pub fn key(&self, name: &str) -> Result<SealingKey, SealingKeyError>; }
impl SealingKey { pub fn name(&self) -> &str; pub fn id(&self) -> &str; pub fn cipher(&self) -> XChaCha20Poly1305; pub fn seal(&self, context: &str, plain: &[u8]) -> Vec<u8>; pub fn open(&self, context: &str, sealed: &[u8]) -> Result<Vec<u8>, SealingKeyError>; }

// engine::files
pub trait Pieces { /* K-E4 Section 9 */ }
pub trait Derived { /* K-E4 Section 9 */ }
pub struct Store { /* the pieces, the data keys, the Derived domains; built once in main.rs */ }
pub fn router(db: &PgPool, bus: &Bus, clock: &Arc<dyn Clock>, store: &Store) -> Router;   // /files/v1
pub async fn attachments(db: &Db, owner: &Reference) -> Result<Vec<Attachment>, Error>;
pub async fn remove_attachment(conn: &mut PgConnection, store: &Store, id: AttachmentId) -> Result<Option<Removed>, Error>;
pub async fn delete_archive(conn: &mut PgConnection, store: &Store, id: ArchiveId) -> Result<Option<(Archive, Removed)>, Error>;
pub async fn propose(conn: &mut PgConnection, new: NewProposal, at: DateTime<Utc>) -> Result<Proposal, Error>;
pub async fn confirm(conn: &mut PgConnection, id: ProposalId, fields: Vec<Field>, (at, by): (DateTime<Utc>, &str)) -> Result<Proposal, Error>;
pub async fn reject(conn: &mut PgConnection, id: ProposalId, (at, by): (DateTime<Utc>, &str)) -> Result<Proposal, Error>;
pub async fn confirmed(db: &Db, owner: &Reference) -> Result<Vec<Proposal>, Error>;
```

`main.rs` opens the data keys (`ANIMA_DATA_KEY_DIR`) with the identifier of the credential key (`Vault::key_id`, new) and builds the `Store` of `ANIMA_FILES_DIR`; `schema::Dependencies` gains `files`, and `lib.rs` merges `files::router` beside `/graphql` in a logged span. `fleet/events/model.rs` gains `AuditTarget::Attachment`, `Archive` and `ExtractionProposal`. `ids.rs` gains `AttachmentId`, `ArchiveId` and `ProposalId`. No domain imports `files` now.

## projects (work, XP-02 first slice; operator, 2026-11-13; ruling number to be assigned)

Folder `crates/hub/src/work/projects/` (group `work/`). Face `projects.rs` exposes `pub fn intake() -> Arc<dyn sources::Intake>` and `pub mod graphql`. No domain imports `projects`; `main.rs` and `schema.rs` wire its intake into the sources registry. Every root field calls `extensions::require_on(db, PROJECTS)` first.

### projects: work, code and history (second slice; operator, 2026-11-14; ruling number to be assigned; PRJ-027 to PRJ-035, OD-E14)

Live reads and named changes through the engine (K-E1 Section 13). No table and no migration: 2200 to 2299 is untouched.

```rust
// engine::sources (live.rs), used by projects
pub async fn ask(s: &Scheduling, account: SourceAccountId, request: &Request, keep: Keep) -> Result<Reply, LiveFailure>;
pub async fn change(s: &Scheduling, account: SourceAccountId, change: &Change) -> Result<Settled, LiveFailure>;
pub fn forget(transport: &Transport, account: SourceAccountId, path_prefix: &str);
pub async fn standing(conn: &mut PgConnection, accounts: &[SourceAccountId]) -> Vec<(SourceAccountId, Standing)>;
// crate source-exchange: change.rs (Change, Verb, Rendered, Changes, Settled), parts.rs (checked newtypes);
// Declaration.changes: Option<Changes>
```

Sub-folders of `projects/`, each with its face file: `forge/` (repository row to account and `RepositoryAccess`, `LiveFailure` and `Settled` to K7 codes and fixed words, cursors, the change entries of the audit), `pull_requests/`, `issues/`, `workflows/`, `history/` (commits, branches, releases, checks, commit status), `code/`. `projects/graphql.rs` merges the old and new roots and keeps the names `projects::graphql::Query` and `Mutation`, so `schema.rs` does not change. The fork rule reads the agent declaration from `events::Operator` (as `openGithubConnect` does). `fleet/events/model.rs` gains `AuditTarget::Repository` and `AuditAction::Merge`. The engine still imports no extension.

## containers (work; R-149, operator, 2026-11-18; HOST-001, HOST-010)

Folder `crates/hub/src/work/containers/` (group `work/`). Stage 1 of hosted applications: the containers of a device, read-only. The domain has no table and no store: each read opens an SSH link to the device and runs one fixed `docker` command (CHK-012). It writes no event and no log line (LOG-007). No domain imports `containers`; `schema.rs` merges `containers::graphql::Query`.

```rust
/// What a link needs besides the device.
pub struct Reach<'a> { pub db: &'a Db, pub vault: &'a credentials::Vault, pub policy: &'a AddressPolicy }
/// The containers of a device, running or not, newest first. Nothing is kept.
pub async fn list(reach: Reach<'_>, device: DeviceId, now: DateTime<Utc>) -> Result<Vec<Container>, Error>;
/// The last lines of the log of one container, both output streams, oldest first. Nothing is kept.
pub async fn log(reach: Reach<'_>, device: DeviceId, tail: &Tail, now: DateTime<Utc>) -> Result<ContainerLog, Error>;
/// The only operator values that reach a command: 12 to 64 lower-case hex digits, and 1 to 1000 lines (default 200).
pub struct ContainerId(/* ssh::HexDigits */);  pub struct LineCount(/* u16 */);
pub struct Tail { pub container: ContainerId, pub lines: LineCount }
```

Additions of R-150 (operator, 2026-11-18; HOST-017, HOST-019, HOST-020; task H8), with no new import edge and no table:

```rust
/// The details of one container: command, restart policy, health check, mounts, networks, labels, processes,
/// and the names of the environment variables. The command asks Docker for no value of one (LOG-007).
pub async fn detail(reach: Reach<'_>, device: DeviceId, container: &ContainerId, now: DateTime<Utc>) -> Result<ContainerDetail, Error>;
/// The images and the volumes, with the size of each, the containers that use it and the space a clean-up can free.
pub async fn storage(reach: Reach<'_>, device: DeviceId, now: DateTime<Utc>) -> Result<Storage, Error>;
/// The networks of Docker, each with the containers on it.
pub async fn networks(reach: Reach<'_>, device: DeviceId, now: DateTime<Utc>) -> Result<Vec<Network>, Error>;
/// The one state of a Compose project (K1 `ComposeProjectState`); `list` writes it on each container of a project.
pub enum ProjectState { Running, Unhealthy, PartlyRunning, Failing, Paused, Stopped, Unknown }
```

Addition of R-150 for HOST-021 (task H13), with no new import edge (`targets` is a shared unit):

```rust
/// Whether a newer image exists for each container: the digest on the device against the digest of the same tag at
/// its public registry. A registry that is not read is no error. It updates nothing (ETH-004).
pub async fn image_updates(reach: Reach<'_>, device: DeviceId, now: DateTime<Utc>) -> Result<Vec<ImageUpdate>, Error>;
```

This is the one outbound path of the domain that does not go to a device: HTTPS to the fixed hosts of four public registries (`updates/registry.rs`), each resolved once with `targets::resolve_checked` and reached only at those addresses, with no redirect, no credential and no body read but the token (`updates/egress.rs`, as `checks/probe/rdap/egress.rs`).

Addition of R-150 for HOST-022 (task H29), with no new import edge and one new dependency of the hub (`yaml-rust2`, which `compose-rules` already uses). The sub-module `compose_file` holds two pure functions of the text that the caller gives:

```rust
/// The check of the syntax of a Compose file: is the text YAML, and does it have the form of a Compose file. It is not
/// the check of HOST-028 (`compose-rules`): it does not say that a machine accepts the file.
pub fn check(text: &str) -> Check;
/// The Compose file of one `docker run` command, or the fixed code of why there is none.
pub fn convert(command: &str) -> Result<Converted, Refusal>;
```

They reach no device and no network, run nothing and keep nothing, and the two root queries take no device. Each has the complexity 100 (at most four in one request) and runs on a thread for blocking work. `check` reads at most 256 KiB, 64 levels and 100,000 events of the YAML parser, takes the events one at a time (no recursion in the parser), and reads an alias as one node (it expands nothing). `convert` reads at most 64 KiB with its own reader of shell words (`compose_file/words.rs`), which expands nothing and refuses each form that a shell would run, fill or expand; it takes at most 32 networks and 32 network aliases, and refuses a file larger than 256 KiB. A `Problem` holds a fixed code, a line, a column, the number of its service and the name of a key only when Compose defines that name; it holds no text that the operator wrote (LOG-007), because a value or a name in the wrong place can be a secret. The list of the options that are not converted holds the names of options of Docker only. Neither text reaches a log: the request span records no query text and no variables, and the error boundary of the hub replaces each error text that can quote a value (`events/process_log`).

The list of the containers also asks Docker for two more labels of Compose, the folder and the files of the project of a container (R-211, task H15); `Container` has them as `compose_working_dir` and `compose_files`.

Each read is one fixed command line of `docker` words that only read (`container inspect`, `top`, `system df`, `network ls`, `ps`), joined with `&&`. The only operator value in one is the container identifier.

`ssh` gains for this domain (same ruling): `HexDigits` (1 to 64 hex digits kept as values), `FixedCommand::hex(self, &HexDigits)` and `FixedCommand::number(self, u32)`. Both render from values, so operator text still cannot enter a command.

## catalogue (work; R-299, R-300; HOST-047; task H38)

Folder `crates/hub/src/work/catalogue/` (group `work/`). The catalogue that ships with Anima. The recipes of the crate `catalogue` are built into the hub binary by the build script of that crate; at the first read the hub runs the static test of task H36 on each one and keeps, for the life of the process, only those with no finding. The domain has no table, no store, no event and no log line, and it reaches no machine and no network. No domain imports `catalogue`; `schema.rs` merges `catalogue::graphql::Query` and `catalogue::graphql::Mutation`.

```rust
/// Each entry that passed the static test, in the order of the identifiers.
pub fn entries() -> &'static [Listed];
/// The listed entry with this identifier.
pub fn entry(id: &str) -> Option<&'static Listed>;
/// The answer to the install of an entry: NOT_LISTED, ENGINE_SOCKET (R-247) or INSTALL_NOT_BUILT (task H6), and whether
/// the machine needs "Open to the network" (R-229). It never installs: task H6 is not built.
pub fn install(listed: Option<&Listed>) -> Install;
```

A `Listed` holds the entry, the text of its Compose file (with no secret: each secret is a variable that the install makes) and the names of the variables of its example environment file, never a value. `installCatalogueEntry` checks only the form of the device ID; it reads no device row and writes no audit entry, because it changes nothing.

## guac-protocol crate (WP-08)

```rust
pub struct Instruction { pub opcode: String, pub args: Vec<String> }
pub fn encode(i: &Instruction) -> String;
/// Incremental decoder: feed bytes, get complete instructions. Rejects bad lengths (no panics).
pub struct Decoder; impl Decoder { pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<Instruction>, DecodeError>; }
/// Handshake as a pure state machine (GW-010): select → args → size/audio/video/image/timezone/name → connect → ready.
pub struct Handshake; impl Handshake {
    pub fn new(protocol: &str, params: ConnectParams) -> (Self, Instruction /* select */);
    pub fn on_instruction(&mut self, i: Instruction) -> Result<HandshakeStep, HandshakeError>;
}
```
No I/O in this crate. `sessions` owns the sockets.
