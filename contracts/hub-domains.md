# K3 — Hub domain interfaces

Status: **frozen for Phase 2** (change only through a programme ruling, CON-001). Phase 2 additions are marked (Phase 2).

These are the only functions and types one hub domain may use from another. Everything else in a domain is private or `pub(crate)` for its own submodules. Signatures are the contract; parameter names and doc comments may vary. `Db` is `sqlx::PgPool`; functions that must join a caller's transaction take `&mut sqlx::PgConnection` instead.

## Leaf modules (owned by WP-01; every domain may import them; they import no domain)

| Module | Content |
| --- | --- |
| `crates/hub/src/ids.rs` | All entity ID newtypes (see `conventions.md`): `OperatorId`, `SiteId`, `DeviceId`, `CheckId`, `WindowId`, `SessionId`, `EventId`, and (Phase 2) `WebInterfaceId`, `MetricSeriesId` |
| `crates/hub/src/status.rs` | `pub enum Status { Up, Down, Degraded, Maintenance, Unknown }` with `fn worst(a, b)` (CHK-004) |
| `crates/hub/src/clock.rs` | `pub trait Clock: Send + Sync { fn now(&self) -> DateTime<Utc>; }`, `SystemClock`, and `FixedClock` for tests. The only clock read in the hub (CQ-021). |
| `crates/hub/src/config.rs` | `pub struct Config` from the environment (see `conventions.md`) |
| `crates/hub/src/paging.rs` | `PageEnd`, exposed as K1 `PageInfo { hasNextPage, endCursor }` (R-014; the Rust name avoids the banned word "info"). Every connection uses it; cursors stay opaque and per domain. |
| `crates/hub/src/targets.rs` (Phase 2) | `AddressPolicy`, the target rule and deny list (P2-004). It moves here from `checks` (programme-owned, as R-003), so `sessions`, `checks` and `metrics` share one deny-list type without new edges. WP-20 makes the move. |
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

## events (WP-05)

```rust
pub struct Operator { pub id: OperatorId, pub name: String }   // re-exported by access (R-019)
pub struct ClientAddr(pub std::net::IpAddr);                     // re-exported by access (R-019)
pub struct HeartbeatSummary { pub at: DateTime<Utc>, pub status: Status, pub response_time_ms: Option<u32>, pub message: Option<String> }   // used by checks (R-019); `message` as stored: cleaned, at most MESSAGE_MAX chars; Clone, not Copy (R-036)
pub enum Severity { Info, Warning, Error }
pub enum Source { Check, Session, Device, Access, Hub, Metrics }        // Metrics (Phase 2); Proxmox later. No new Signal (D-15)
pub struct NewEvent { pub severity: Severity, pub source: Source,
                      pub device: Option<(DeviceId, String)>, pub message: String }
pub struct NewAudit { pub operator: Operator, pub action: AuditAction,
                      pub target: AuditTarget, pub client: ClientAddr,
                      pub change: serde_json::Value }

pub async fn record(conn: &mut PgConnection, bus: &Bus, at: DateTime<Utc>, e: NewEvent) -> Result<EventId, Error>;
pub async fn audit(conn: &mut PgConnection, at: DateTime<Utc>, a: NewAudit) -> Result<(), Error>;

/// Broadcast of new events and status changes for subscriptions (LOG-010).
#[derive(Clone)] pub struct Bus { /* tokio::sync::broadcast */ }
pub enum Signal { Event(EventSummary), DeviceStatus { device: DeviceId, status: Status, at: DateTime<Utc> },   // `at` added by R-028
                  Heartbeat { check: CheckId, device: DeviceId, beat: HeartbeatSummary } }
impl Bus { pub fn publish(&self, s: Signal); pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Signal>; }
```

`Status`, `CheckId` and `DeviceId` come from the leaf modules, and `events` defines the three shared value types above, so `events` imports no domain (R-019).

Wiring only (called from `main.rs`, never from another domain): `create_partitions_ahead`, `drop_expired_partitions`, `check_runtime_role`, `run_daily`, `keep_partitions` (R-016, R-019; WP-05 T2).

### Allowed domain dependencies (the import-graph gate enforces this list; REPO-004)

| Domain | May import (besides leaf modules) |
| --- | --- |
| `events` | — |
| `credentials` | — |
| `access` | `events`, `credentials` (Vault only, for TOTP secrets) |
| `devices` | `events`, `credentials` (`Vault`, `Secret`, `SshSecret`, `Error`, `has_ssh`, `with_ssh`, `store_ssh`, `clear_ssh`; R-010, R-017, R-026) |
| `maintenance` | `events` (R-029; audit and events for window changes, LOG-005) |
| `checks` | `devices`, `maintenance`, `events`, `credentials` (`Vault`, `Secret`, `Sealed` only; R-028) |
| `sessions` | `access`, `devices`, `credentials`, `events` |
| `checks` (Phase 2, adds) | `sessions` (`link`, `SshLink`, `FixedCommand` only; no cycle: `sessions` does not import `checks`) |
| `metrics` (Phase 2, new) | `devices`, `sessions` (link only), `credentials` (`Vault` only), `events` |
| `web_interfaces` (Phase 2, new) | `checks` (`http_urls` only), `events` |

GraphQL composition edges (R-011). Only the file `<domain>/graphql.rs` may also import these domains' public faces, to resolve K1 fields that join domains. These edges never permit a `store` import and do not count toward cycles. They live in `quality/hub-modules.json` under `graphql`.

| Domain `graphql.rs` | May also import | For |
| --- | --- | --- |
| `devices` | `access` (R-022), `checks`, `sessions` | the `Authenticated` guard; `Device.status`, `Device.checks`, `Device.uptime`, `DeviceFilter.status`, `DeviceSort.STATUS`/`UPTIME`, `createDevice(addDefaultCheck)`, `SshSettings.hostKey` |

| `devices` (Phase 2, adds) | `metrics`, `web_interfaces` | `Device.readings`, `Device.metrics`, `Device.gpus`, `Device.webInterfaces`, `Device.webInterfaceSuggestions` |
| `metrics`, `web_interfaces` (Phase 2) | `access` (R-022) | the `Authenticated` guard on their root fields |
| `checks`, `maintenance`, `events` | `access` (R-022) | the `Authenticated` guard on their root fields (API-003) |

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

Delete (DEV-015) is one `DELETE` in `devices`: foreign keys cascade to `device_tags`, `ssh_settings`, `credentials.secrets`, `checks.checks` and `sessions.host_keys`. Heartbeats (partitioned, no foreign key) whose check no longer exists are removed by the checks retention job. Maintenance targets are text; a target that names a deleted device matches nothing. Logs and session history keep their copied names.

(Phase 2, WP-21) `Device` gains the fields of K1 `Device`:

```rust
pub struct RdpSettings { pub enabled: bool, pub port: u16, pub username: String, pub domain: Option<String>,
    pub security: RdpSecurity, pub ignore_certificate: bool, pub resize: ResizeMethod, pub audio_out: bool,
    pub microphone: bool, pub shared_drive: bool }
pub struct VncSettings { pub enabled: bool, pub port: u16, pub username: Option<String> }
pub struct TransferPolicy { pub upload: bool, pub download: bool, pub copy: bool, pub paste: bool }
pub struct MacAddress([u8; 6]);            // parsed at the API edge; Display renders aa:bb:cc:dd:ee:ff
pub enum MetricsSource { None, Ssh }
pub async fn wake_relay(db: &Db, site: SiteId) -> Result<Option<Device>, Error>;
```

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

Context values (R-010). Each caller uses exactly this byte string, so a value sealed for one row cannot be opened for another:

| Use | `context` |
| --- | --- |
| SSH secret (inside `credentials`) | `credentials.secrets:<device_id>:<kind>` |
| TOTP secret (WP-05) | `access.operators.totp:<operator_id>` |

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
/// axum route for GET /tunnel (see tunnel.md). `guacd` is the K7 `host:port` text, resolved at connect time (R-032). Not mounted until RDP or VNC uses it (R-062).
pub fn tunnel_router(db: Db, vault: credentials::Vault, bus: events::Bus, clock: Clock, guacd: String, live: Live) -> axum::Router;
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
pub async fn link(db: &Db, vault: &credentials::Vault, policy: &AddressPolicy, device: &Device, now: DateTime<Utc>) -> Result<SshLink, Error>;
impl SshLink { pub async fn run(&mut self, c: &FixedCommand, limit: Duration) -> Result<CommandOutput, Error>; }
pub async fn wake(db: &Db, vault: &credentials::Vault, policy: &AddressPolicy, bus: &events::Bus, device: DeviceId, now: DateTime<Utc>) -> Result<WakeSent, Error>;
pub fn files_router(db: Db, vault: credentials::Vault, bus: events::Bus, clock: Clock, live: Live) -> axum::Router;   // GET /files, tunnel.md Section 9
```

(Phase 2, WP-22):

```rust
pub async fn server_certificates(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, ServerCertificate>, Error>;
// tunnel_router now serves RDP and VNC and is mounted; Live also broadcasts SessionChange (D-15).
```

Wake-on-LAN lives in `sessions` (RA-031 is a remote access rule), so a session start can wake a device without a `sessions` and `power` cycle.

## metrics (Phase 2, WP-28)

```rust
pub struct Collecting { pub vault: credentials::Vault, pub policy: AddressPolicy }
#[derive(Clone)] pub struct Feed { /* broadcast of MetricsAdded */ }    // D-15
pub fn spawn_collector(db: Db, feed: Feed, bus: events::Bus, clock: Clock, collecting: Collecting) -> JoinHandle<()>;   // MET-001
pub async fn readings(db: &Db, devices: &[DeviceId], now: DateTime<Utc>) -> Result<HashMap<DeviceId, Readings>, Error>;
pub async fn series(db: &Db, device: DeviceId, range: MetricRange, now: DateTime<Utc>) -> Result<Vec<MetricSeries>, Error>;
pub async fn gpus(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<Gpu>>, Error>;
// Wiring only: create_partitions_ahead, drop_expired_partitions (as events and checks).
```

## web_interfaces (Phase 2, WP-30)

```rust
pub async fn for_devices(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<WebInterface>>, Error>;
pub async fn suggestions(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<Suggestion>>, Error>;
```

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
