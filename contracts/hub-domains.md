# K3 — Hub domain interfaces

Status: **frozen for Phase 1** (change only through a programme ruling, CON-001).

These are the only functions and types one hub domain may use from another. Everything else in a domain is private or `pub(crate)` for its own submodules. Signatures are the contract; parameter names and doc comments may vary. `Db` is `sqlx::PgPool`; functions that must join a caller's transaction take `&mut sqlx::PgConnection` instead.

## Leaf modules (owned by WP-01; every domain may import them; they import no domain)

| Module | Content |
| --- | --- |
| `crates/hub/src/ids.rs` | All entity ID newtypes (see `conventions.md`): `OperatorId`, `SiteId`, `DeviceId`, `CheckId`, `WindowId`, `SessionId`, `EventId` |
| `crates/hub/src/status.rs` | `pub enum Status { Up, Down, Degraded, Maintenance, Unknown }` with `fn worst(a, b)` (CHK-004) |
| `crates/hub/src/clock.rs` | `pub trait Clock: Send + Sync { fn now(&self) -> DateTime<Utc>; }`, `SystemClock`, and `FixedClock` for tests. The only clock read in the hub (CQ-021). |
| `crates/hub/src/config.rs` | `pub struct Config` from the environment (see `conventions.md`) |

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
pub struct Operator { pub id: OperatorId, pub name: String }
pub struct ClientAddr(pub std::net::IpAddr);

/// GraphQL guard: requires a valid token; puts `Operator` in the context (API-003).
pub struct Authenticated;
/// axum middleware: refuses addresses outside permitted networks, logs the refusal (NET-002).
pub fn permitted_networks_layer(db: Db, bus: events::Bus, clock: Clock) -> impl tower::Layer<axum::routing::Route> + Clone;
/// Used by the tunnel: resolve a bearer token to an operator.
pub async fn operator_for_token(db: &Db, token: &str, now: DateTime<Utc>) -> Result<Operator, Error>;
```

## events (WP-05)

```rust
pub enum Severity { Info, Warning, Error }
pub enum Source { Check, Session, Device, Access, Hub }        // Proxmox, Metrics later
pub struct NewEvent { pub severity: Severity, pub source: Source,
                      pub device: Option<(DeviceId, String)>, pub message: String }
pub struct NewAudit { pub operator: Operator, pub action: AuditAction,
                      pub target: AuditTarget, pub client: ClientAddr,
                      pub change: serde_json::Value }

pub async fn record(conn: &mut PgConnection, bus: &Bus, at: DateTime<Utc>, e: NewEvent) -> Result<EventId, Error>;
pub async fn audit(conn: &mut PgConnection, at: DateTime<Utc>, a: NewAudit) -> Result<(), Error>;

/// Broadcast of new events and status changes for subscriptions (LOG-010).
#[derive(Clone)] pub struct Bus { /* tokio::sync::broadcast */ }
pub enum Signal { Event(EventSummary), DeviceStatus { device: DeviceId, status: Status },
                  Heartbeat { check: CheckId, device: DeviceId, beat: HeartbeatSummary } }
impl Bus { pub fn publish(&self, s: Signal); pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Signal>; }
```

`Status`, `CheckId` and `DeviceId` come from the leaf modules, so `events` imports no domain.

### Allowed domain dependencies (the import-graph gate enforces this list; REPO-004)

| Domain | May import (besides leaf modules) |
| --- | --- |
| `events` | — |
| `credentials` | — |
| `access` | `events`, `credentials` (Vault only, for TOTP secrets) |
| `devices` | `events`, `credentials` (`Vault`, `SshSecret`, `has_ssh`, `store_ssh`, `clear_ssh`; R-010) |
| `maintenance` | — (pure rules and its own store; callers pass the device, check and tags) |
| `checks` | `devices`, `maintenance`, `events` |
| `sessions` | `access`, `devices`, `credentials`, `events` |

`devices` never imports `checks` (that edge would make a cycle). Device deletion relies on foreign keys and the checks retention job instead (see below).

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

## checks (WP-09)

```rust
pub struct HeartbeatSummary { pub at: DateTime<Utc>, pub status: Status, pub response_time_ms: Option<u32> }
pub async fn device_status(db: &Db, device: DeviceId, now: DateTime<Utc>) -> Result<Status, Error>;   // CHK-004
/// Started by main.rs; owns the scheduler task, the watchdog (CHK-010) and retention (DB-005).
pub fn spawn_scheduler(db: Db, bus: events::Bus, clock: Clock) -> tokio::task::JoinHandle<()>;
```

Check evaluation is pure: `fn evaluate(prev: &CheckState, probe: ProbeResult, cfg: &CheckConfig, now) -> (CheckState, Status)`.

## maintenance (WP-09)

```rust
/// Pure (MW-001..004). No clock read.
pub fn is_active(rule: &WindowRule, tz: chrono_tz::Tz, now: DateTime<Utc>) -> bool;
pub async fn active_for(db: &Db, device: DeviceId, check: Option<CheckId>, tags: &[String], now: DateTime<Utc>) -> Result<bool, Error>;
```

## sessions (WP-10)

```rust
pub async fn start(db: &Db, clock: &Clock, op: &Operator, client: ClientAddr, req: StartRequest) -> Result<Ticket, Error>;
/// axum route for GET /tunnel (see tunnel.md).
pub fn tunnel_router(db: Db, vault: credentials::Vault, bus: events::Bus, clock: Clock, guacd: SocketAddr) -> axum::Router;
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
