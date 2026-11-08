# Phase 3 contract deltas (K1–K7)

Status: **Adopted** (R-081); the Phase 3 freeze. WP-40 moves it into `contracts/`. Plan: [phase-3-plan.md](phase-3-plan.md).

Why this file and not `contracts/`: CON-002 keeps the hub schema equal to K1. After approval, WP-40 and the
programme orchestrator copy each section below into its contract file in one commit, with the file headers
changed to "frozen for Phase 3". Until then `contracts/` stays the Phase 2 freeze.

Every delta adds; nothing in Phases 1 and 2 is renamed or removed on the wire or in a table name. There is one
exception, in K2 and K3 only: the Q2-05 split (D-31) moves Rust paths from `sessions` to `ssh` and `trust`, and
moves two tables to the schema `trust`. No GraphQL name, column or wire value changes.

## K1 — GraphQL additions (`contracts/schema.graphql`)

Each new type, field, enum value, argument and input field carries `@pending` until the hub declares it
(P3-002). Header line 3 becomes "frozen for Phase 3".

```graphql
# ---- shared and existing enums
enum EventSource { ...  NOTIFICATION @pending }

# ---- access (WP-43). D-3, D-4, D-23.
enum TokenScope @pending { READ WRITE }

"Set by signInBrowser together with the __Host-anima cookie. The token itself is never returned (D-3)."
type BrowserSignIn @pending {
  operator: Operator!
  expiresAt: DateTime!
}

type ApiKey @pending {
  id: ID!
  "1..64 characters, unique."
  name: String!
  scope: TokenScope!
  "The first 12 characters of the secret, for example anima_Ab12Cd, so the operator can tell keys apart."
  prefix: String!
  expiresAt: DateTime!
  createdAt: DateTime!
  lastUsedAt: DateTime
}

type ApiKeyCreated @pending {
  key: ApiKey!
  "Shown once. The hub keeps only its SHA-256 (INT-003, P3-004)."
  secret: String!
}

input ApiKeyInput @pending {
  name: String!
  scope: TokenScope!
  "Required; at most one year from now (D-23)."
  expiresAt: DateTime!
}

# ---- devices (WP-47). RA-036, RA-040.
type Device {
  ...
  "Most live sessions at once (RA-036); null: no limit. Viewers and file tunnels do not count."
  sessionLimit: Int @pending
  "RA-040; default false."
  recordSessions: Boolean! @pending
}
input CreateDeviceInput { ...  sessionLimit: Int @pending  recordSessions: Boolean = false @pending }
input UpdateDeviceInput { ...  "1..100; 0 clears the limit." sessionLimit: Int @pending  recordSessions: Boolean @pending }

# ---- checks (WP-46). CHK-009 identifier: domain_expiry (fixed forever).
enum CheckType { ...  DOMAIN_EXPIRY @pending }
union CheckConfig = ... | DomainExpiryConfig   # header change; @pending on the member type

"Degraded below warnDays, Down after expiry, Unknown when RDAP gives no expiry date (D-29)."
type DomainExpiryConfig @pending { domain: String!  warnDays: Int!  timeoutMs: Int! }
input DomainExpiryConfigInput @pending { domain: String!  warnDays: Int = 30  timeoutMs: Int = 10000 }
input CheckConfigInput { ...  domainExpiry: DomainExpiryConfigInput @pending }

type Check {
  ...
  "DOMAIN_EXPIRY only: whole days until the registration expires."
  domainDaysLeft: Int @pending
  notifications: CheckNotifications! @pending          # notifications (composition, K3)
}

"The values the client pre-fills in a new check (D-26). Bounds as checks.checks."
type CheckDefaults @pending { intervalSeconds: Int!  retries: Int!  retryIntervalSeconds: Int!  degradedThresholdMs: Int! }
input CheckDefaultsInput @pending { intervalSeconds: Int!  retries: Int!  retryIntervalSeconds: Int!  degradedThresholdMs: Int! }

# ---- notifications (WP-45). NOT-001..007, D-16..D-22.
enum ChannelKind @pending { WEBHOOK NTFY EMAIL APPRISE }
enum NotificationKind @pending { DOWN UP STILL_DOWN CERTIFICATE_EXPIRY DOMAIN_EXPIRY TEST }
enum WebhookMethod @pending { POST PUT }
enum SmtpSecurity @pending { TLS STARTTLS }
enum AppriseMode @pending { STATELESS_URLS CONFIG_KEY }

"The URL can hold a credential, so only its origin is returned (D-17)."
type WebhookSettings @pending { origin: String!  method: WebhookMethod!  "Names only; values are sealed." headerNames: [String!]! }
"The topic and token are sealed: on a public server the topic is the credential."
type NtfySettings @pending { server: String!  priority: Int!  tokenStored: Boolean! }
type EmailSettings @pending { host: String!  port: Int!  security: SmtpSecurity!  username: String  from: String!  to: [String!]!  passwordStored: Boolean! }
"The URLs or the key are sealed."
type AppriseSettings @pending { server: String!  mode: AppriseMode!  tags: [String!]! }
union ChannelSettings @pending = WebhookSettings | NtfySettings | EmailSettings | AppriseSettings

type NotificationTemplate @pending { title: String!  body: String! }

type NotificationChannel @pending {
  id: ID!
  name: String!
  kind: ChannelKind!
  "On for each new check (NOT-003)."
  onByDefault: Boolean!
  settings: ChannelSettings!
  "Null: the installation's default template (NOT-007)."
  template: NotificationTemplate
  lastDelivery: Delivery
  createdAt: DateTime!
  updatedAt: DateTime!
}

type Delivery @pending {
  id: ID!
  channelId: ID
  channelName: String!
  kind: NotificationKind!
  at: DateTime!
  ok: Boolean!
  "A fixed sentence, for example \"The target did not answer in 8 seconds.\" Never the target's reply."
  message: String
}
type DeliveryEdge @pending { cursor: String!  node: Delivery! }
type DeliveryConnection @pending { edges: [DeliveryEdge!]!  pageInfo: PageInfo! }

type CheckNotifications @pending {
  channels: [NotificationChannel!]!
  "Send again after each N Down heartbeats; 0: never (NOT-005)."
  resendEveryBeats: Int!
}

type NotificationSettings @pending {
  "Days before expiry, descending, 1..365, at most 5 values (NOT-006, D-22). Default [21, 14, 7]."
  certificateDays: [Int!]!
  template: NotificationTemplate!
  "The placeholders a template may use (D-21)."
  placeholders: [String!]!
}

"On update, a secret field that is null keeps the stored value; an empty string clears it where it is optional."
input WebhookSettingsInput @pending { url: String  method: WebhookMethod = POST  headers: [HttpHeaderInput!] }
input NtfySettingsInput @pending { server: String = "https://ntfy.sh"  topic: String  token: String  priority: Int = 3 }
input EmailSettingsInput @pending { host: String!  port: Int = 587  security: SmtpSecurity = STARTTLS
  username: String  password: String  from: String!  to: [String!]! }
input AppriseSettingsInput @pending { server: String!  mode: AppriseMode = STATELESS_URLS  urls: [String!]  key: String  tags: [String!] = [] }
input ChannelSettingsInput @oneOf @pending { webhook: WebhookSettingsInput  ntfy: NtfySettingsInput
  email: EmailSettingsInput  apprise: AppriseSettingsInput }
input NotificationTemplateInput @pending { title: String!  body: String! }
input NotificationChannelInput @pending { name: String!  onByDefault: Boolean = false
  settings: ChannelSettingsInput!  "Null: the default template." template: NotificationTemplateInput }
input CheckNotificationsInput @pending { channelIds: [ID!]!  resendEveryBeats: Int = 0 }
input NotificationSettingsInput @pending { certificateDays: [Int!]!  template: NotificationTemplateInput! }

# ---- retention (WP-46). HUB-005, D-25.
enum RetentionSubject @pending { HEARTBEATS METRICS EVENTS RECORDINGS DELIVERIES }
type RetentionPeriod @pending { subject: RetentionSubject!  days: Int!  minDays: Int!  maxDays: Int!  defaultDays: Int! }

# ---- sessions (WP-47, WP-48). RA-035, RA-040, RA-041.
enum ShareAccess @pending { VIEW CONTROL }
enum RecordingState @pending { OFF ON STOPPED }
enum RecordingFormat @pending { GUACAMOLE ASCIICAST }

type SessionShare @pending {
  id: ID!
  sessionId: ID!
  access: ShareAccess!
  maxViewers: Int!
  "Viewers connected now."
  viewers: Int!
  createdAt: DateTime!
  expiresAt: DateTime!
}

type ShareLink @pending {
  share: SessionShare!
  "Shown once: <public origin>/shared#<token>. The token is only in the fragment (D-7)."
  url: String!
}

input ShareInput @pending {
  access: ShareAccess = VIEW
  "5..1440."
  expiresInMinutes: Int = 60
  "1..10."
  maxViewers: Int = 3
}

"What a viewer gets from joinShare. It names no session ID and no operator."
type WatchTicket @pending {
  token: String!
  "/watch"
  tunnelPath: String!
  expiresAt: DateTime!
  protocol: Protocol!
  access: ShareAccess!
  deviceName: String!
  recording: Boolean!
}

type Recording @pending {
  id: ID!
  sessionId: ID!
  deviceId: ID
  deviceName: String!
  protocol: Protocol!
  format: RecordingFormat!
  startedAt: DateTime!
  endedAt: DateTime
  "Bytes of plaintext; Float above 2^31."
  sizeBytes: Float!
  "False when the hub stopped before the last part (K4 §11)."
  complete: Boolean!
  expiresAt: DateTime!
}
type RecordingEdge @pending { cursor: String!  node: Recording! }
type RecordingConnection @pending { edges: [RecordingEdge!]!  pageInfo: PageInfo! }

type Session {
  ...
  recording: RecordingState! @pending
  "Live sessions only; empty after the end."
  shares: [SessionShare!]! @pending
}
type SessionTicket { ...  "The session is recorded; the client shows the badge (RA-040)." recording: Boolean! @pending }

# ---- root additions
type Query {
  ...
  apiKeys: [ApiKey!]! @pending
  checkDefaults: CheckDefaults! @pending
  notificationChannels: [NotificationChannel!]! @pending
  notificationSettings: NotificationSettings! @pending
  deliveries(channelId: ID, first: Int = 50, after: String): DeliveryConnection! @pending
  retention: [RetentionPeriod!]! @pending
  recordings(deviceId: ID, first: Int = 50, after: String): RecordingConnection! @pending
}

type Mutation {
  ...
  "Signed out (D-4). Sets the __Host-anima cookie; refused unless Origin is the public origin."
  signInBrowser(input: LoginInput!): BrowserSignIn! @pending
  "Login or browser token only (D-23)."
  createApiKey(input: ApiKeyInput!): ApiKeyCreated! @pending
  revokeApiKey(id: ID!): ID! @pending

  setCheckDefaults(input: CheckDefaultsInput!): CheckDefaults! @pending

  createNotificationChannel(input: NotificationChannelInput!): NotificationChannel! @pending
  updateNotificationChannel(id: ID!, input: NotificationChannelInput!): NotificationChannel! @pending
  deleteNotificationChannel(id: ID!): ID! @pending
  "Sends at once and waits at most 8 seconds (NOT-004)."
  sendTestNotification(id: ID!): Delivery! @pending
  setCheckNotifications(checkId: ID!, input: CheckNotificationsInput!): CheckNotifications! @pending
  setNotificationSettings(input: NotificationSettingsInput!): NotificationSettings! @pending

  setRetention(subject: RetentionSubject!, days: Int!): RetentionPeriod! @pending

  "The session must be live and the caller's (RA-035)."
  shareSession(sessionId: ID!, input: ShareInput!): ShareLink! @pending
  endShare(id: ID!): ID! @pending
  "Signed out (D-7). Failed attempts count in the login throttle by client address."
  joinShare(token: String!, name: String, display: DisplayInput!): WatchTicket! @pending
  "A ticket for the playback tunnel (K4 §12)."
  openRecording(id: ID!): SessionTicket! @pending
  "Deletes the key first, then the file (D-13)."
  deleteRecording(id: ID!): ID! @pending
}
```

The Subscription type does not change. `sessionChanged` also fires when a share starts or ends, or when a viewer
joins or leaves, so `Session.shares` stays current. A change of `Session.recording` fires it too.

## K2 — Database (`contracts/database.md`)

| Range | Domain (package) | Schema | New tables or changes |
| --- | --- | --- | --- |
| 0010–0029 | access (WP-43) | `access` | `api_tokens.kind text not null default 'login' check (kind in ('login','browser','api_key'))`; `api_tokens.name text null` (required and unique for `api_key`, 1..64 characters); `api_tokens.display_prefix text null` (12 characters, `api_key` only); a check that `browser` and `login` rows have scope `write`; index `(kind, operator_id)`. Prune: expired `login` and `browser` rows at once, expired `api_key` rows 30 days after expiry |
| 0120–0129 | devices (WP-47) | `devices` | `devices.session_limit int null check (session_limit between 1 and 100)`; `devices.record_sessions bool not null default false` |
| 0320–0349 | checks (WP-46) | `checks` | `checks.type` adds `domain_expiry`; `checks.checks.domain_not_after timestamptz null` (beside `cert_not_after`); `checks.defaults` (`id bool pk default true check (id)`, `interval_seconds`, `retries`, `retry_interval_seconds`, `degraded_threshold_ms`, each with the schema default and the bounds of `checks.checks`), seeded with one row |
| 0503 | events (WP-45, handoff) | `events` | `event_log_source_check` adds `notification` (as 0701 did for `metrics`) |
| 0660–0679 | sessions (WP-48) | `sessions` | `recordings` (`id uuid pk`, `session_id uuid not null unique fk -> sessions.sessions`, `device_id uuid null` (no foreign key, copied), `device_name text`, `protocol text`, `format text check (format in ('guacamole','asciicast'))`, `key_sealed bytea not null` (vault, context `sessions.recordings:<id>`), `started_at`, `ended_at null`, `size_bytes bigint not null default 0`, `complete bool not null default false`, `stopped_reason text null check (stopped_reason in ('size_limit','disk_full','write_failed'))`, `expires_at timestamptz not null`); index on `expires_at` |
| 0850–0899 | notifications (WP-45) | `notifications` | `channels` (`id uuid pk`, `name text unique`, `kind text check (kind in ('webhook','ntfy','email','apprise'))`, `on_by_default bool default false`, `settings jsonb` (the non-secret fields, typed in Rust), `secrets bytea null` (sealed JSON of the kind's secret fields, context `notifications.channels:<id>`), `title_template text null`, `body_template text null`, `created_at`, `updated_at`); `check_channels` (`check_id fk -> checks.checks on delete cascade`, `channel_id fk on delete cascade`, pk both); `check_settings` (`check_id pk fk -> checks.checks on delete cascade`, `resend_every_beats int not null default 0 check (0..1000)`); `check_state` (`check_id pk fk -> checks.checks on delete cascade`, `notified_status text null`, `down_beats int not null default 0`, `expiry_marks jsonb not null default '[]'` (threshold and `notAfter` pairs sent)); `outbox` (`id uuid pk`, `channel_id fk on delete cascade`, `check_id uuid null`, `kind`, `payload jsonb` (rendered placeholder values, never a secret), `created_at`, `attempts int default 0`, `next_attempt_at`, `sent_at null`, `given_up_at null`); `deliveries` (`id uuid`, `channel_id uuid null`, `channel_name text`, `kind`, `at timestamptz`, `ok bool`, `message text null`; partitioned by day on `at`, no foreign key, as the logs); `settings` (one row: `certificate_days int[] default '{21,14,7}'`, `title_template`, `body_template`) |
| 0900–0919 | retention (WP-46) | `retention` | `periods` (`subject text pk check (subject in ('heartbeats','metrics','events','recordings','deliveries'))`, `days int not null`, a per-subject bounds check as D-25), seeded with 30, 30, 90, 30, 30 |
| 0920–0939 | trust (WP-41) | `trust` | `create schema trust`; `alter table sessions.host_keys set schema trust`; `alter table sessions.server_certificates set schema trust`. Metadata only (DB-010). Grants and the R-040 revocation move with the tables; migration 0001's default privileges give `anima_runtime` USAGE on the new schema |
| 0940–0999 | reserved | — | Phase 4 |

Retention (Phase 3): the periods in `retention.periods` replace the fixed values of the K2 retention table for
`checks.heartbeats`, `metrics.samples`, `events.event_log`, `sessions.recordings` and
`notifications.deliveries`. The aggregates, `events.audit_log` (1 year, LOG-006) and `access.login_attempts`
(90 days) stay fixed. `notifications.deliveries` partitions follow the K2 "Partitions" rule (`SECURITY DEFINER`
functions owned by the owner role). Outbox rows are deleted 7 days after `sent_at` or `given_up_at`.

## K3 — Hub domain interfaces (`contracts/hub-domains.md`)

### Leaf modules (new and changed)

| Module | Content |
| --- | --- |
| `crates/hub/src/api_errors.rs` (Q2-06) | `ErrorCode` (the K7 codes), `internal`, `coded`, `invalid`, `FieldError`, `field_names`, `schema_dependency`, generic `pool`, `begin`, `commit`. Each domain keeps its own `to_graphql(Error)` |
| `crates/hub/src/text.rs` (Q2-11) | `is_invisible(char)` (the widest table), `is_plain(&str)`, `clean(&str, max_chars) -> String` |
| `crates/hub/src/fingerprint.rs` (Q2-10) | `CertificateFingerprint`: parses 64 hex digits or 32 colon pairs; one canonical value; two renderings |
| `crates/hub/src/targets.rs` (changed) | Adds `pub async fn resolve_checked(host: &str, port: u16, policy: &AddressPolicy) -> Result<Vec<SocketAddr>, TargetError>` (moved from `checks/probe/resolve.rs`, P3-003). `sessions/target.rs` is deleted (Q2-09); the loopback test switch stays on `AddressPolicy` |
| `crates/hub/src/ids.rs` | Adds `ApiKeyId`, `ChannelId`, `DeliveryId`, `ShareId`, `RecordingId` |

Wiring (`main.rs`, `lib.rs`, `schema.rs`, `upkeep.rs`, and new `spawns.rs` and `web_client.rs`):

```rust
// web_client.rs (WP-43): wiring only. The static fallback of the router; explicit routes win.
pub fn web_client(root: &Path, origin: &PublicOrigin) -> axum::Router;   // headers as K7 "HTTP surface"
// lib.rs mounts, each logged and behind the NET-002 layer:
//   /metrics: checks::exposition_router(..) wrapped in access::bearer_layer(.., Scope::Read)
//   /watch:   sessions::watch_router(&serving)
//   /recording: sessions::playback_router(&serving)
//   fallback: web_client(..) when ANIMA_WEB_ROOT is set
// spawns.rs (P3-007): spawn_scheduler, spawn_collector, notifications::spawn_dispatcher.
// upkeep.rs: reads retention::periods daily and passes each period to its domain's job (D-25).
```

### events (changed, Q2-07)

```rust
pub enum Source { Check, Session, Device, Access, Hub, Metrics, Notifications }   // Notifications (Phase 3)
pub enum AuditTarget { ..., WebInterface(WebInterfaceId), ApiKey(ApiKeyId), Channel(ChannelId),
                       Share(ShareId), Recording(RecordingId) }
/// One audit-then-event write for every domain (Q2-07). The entry carries its source and device.
pub struct Trail { /* bus, operator, client address, the time read once */ }
impl Trail {
    pub fn from_context(ctx: &async_graphql::Context<'_>) -> Result<Self, Error>;
    pub async fn write(&self, conn: &mut PgConnection, entry: TrailEntry) -> Result<(), Error>;
}
// Retention jobs take their period (D-25): keep_partitions(db, now, events_keep: chrono::Days).
```

### access (WP-43)

```rust
pub use tokens::Scope;                       // Read, Write
pub enum TokenKind { Login, Browser, ApiKey }
impl Identity { pub const fn kind(&self) -> TokenKind; }   // the guard refuses API keys on the D-23 list
/// For routes outside GraphQL (/metrics): a bearer token of at least `scope`; 401 with no body otherwise.
pub fn bearer_layer(db: Db, clock: Clock, scope: Scope) -> impl tower::Layer<axum::routing::Route> + Clone;
/// The hub's public origin from ANIMA_PUBLIC_ORIGIN; the cookie and joinShare links use it.
#[derive(Clone)] pub struct PublicOrigin(/* scheme, host, port */);
// identify(..) also reads the __Host-anima cookie when Origin equals the public origin (D-3).
// SocketAccess also takes the upgrade's headers, so a WebSocket can sign in by cookie (K7).
```

### devices (WP-47)

```rust
pub struct Device { ..., pub session_limit: Option<std::num::NonZeroU8>, pub record_sessions: bool }
```

### checks (WP-46; handoff edits by WP-45)

```rust
pub async fn defaults(db: &Db) -> Result<CheckDefaults, Error>;          // D-26; add_default_check uses it
/// GET /metrics, Prometheus text format 0.0.4 (INT-001, INT-002), cached 10 s. lib.rs mounts it behind
/// access::bearer_layer(Scope::Read); checks imports no access item for it.
pub fn exposition_router(db: Db, clock: Clock) -> axum::Router;
// Retention: drop_expired(db, now, heartbeats_keep: chrono::Days) is wiring only (moved to upkeep.rs).
```

### notifications (Phase 3, WP-45)

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

Sealing contexts (R-010) add:

| Use | `context` |
| --- | --- |
| Notification channel secrets (WP-45) | `notifications.channels:<channel_id>` |
| Recording key (WP-48) | `sessions.recordings:<recording_id>` |

### retention (Phase 3, WP-46)

```rust
pub enum Subject { Heartbeats, Metrics, Events, Recordings, Deliveries }
pub struct Periods { /* days per subject */ }
impl Periods { pub fn keep(&self, s: Subject) -> chrono::Days; }
pub async fn periods(db: &Db) -> Result<Periods, Error>;   // wiring (upkeep.rs) only
```

### ssh (Phase 3, WP-41; moved from sessions, Q2-05)

```rust
pub struct FixedCommand;  pub struct CommandOutput;  pub struct SshLink;  pub enum LinkError;   // as Phase 2 K3
pub async fn link(db: &Db, vault: &credentials::Vault, policy: &AddressPolicy, device: &Device, now: DateTime<Utc>) -> Result<SshLink, LinkError>;
// The russh probe (testConnection, R-034) and the public-key types move here unchanged.
```

### trust (Phase 3, WP-41; moved from sessions, Q2-05)

```rust
pub async fn host_key(db: &Db, device: DeviceId) -> Result<Option<HostKey>, Error>;
pub async fn host_keys(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, HostKey>, Error>;
pub async fn server_certificates(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, ServerCertificate>, Error>;
// trust::graphql owns acceptHostKey and acceptServerCertificate (no K1 change).
```

### sessions (changed: WP-41 moves, WP-47, WP-48)

```rust
/// Everything the tunnels need, built once in main.rs (replaces the long tunnel_router argument lists).
#[derive(Clone)] pub struct Serving { pub db: Db, pub vault: credentials::Vault, pub bus: events::Bus,
    pub clock: Clock, pub live: Live, pub guacd: String, pub drive_root: String, pub recording: Recording }
/// Recording storage (K4 §11); `root: None` makes recording unavailable.
#[derive(Clone)] pub struct Recording { pub root: Option<PathBuf>, pub max_bytes: u64 }
pub fn tunnel_router(s: &Serving) -> axum::Router;      // signature change only
pub fn terminal_router(s: &Serving) -> axum::Router;    // signature change only
pub fn watch_router(s: &Serving) -> axum::Router;       // GET /watch, K4 §10 (WP-47)
pub fn playback_router(s: &Serving) -> axum::Router;    // GET /recording, K4 §12 (WP-48)
// Wiring only: drop_expired_recordings(db, root, now, keep: chrono::Days) -> Result<u64, Error>.
// Session start, files and wake keep their Phase 2 signatures; wake now uses ssh::link.
```

Allowed domain dependencies after Phase 3 (REPO-004). Rows not shown are unchanged.

| Domain | May import (besides leaf modules) |
| --- | --- |
| `trust` (new) | `devices`, `events` |
| `ssh` (new) | `devices`, `credentials`, `trust`, `events` |
| `sessions` | `access`, `devices`, `credentials`, `events`, `ssh`, `trust` |
| `checks` | `devices`, `maintenance`, `events`, `credentials`, `ssh` (replaces `sessions`), `notifications` (`observe`, `adopt_defaults`, `Observation`, `for_checks` only) |
| `metrics` | `devices`, `ssh` (replaces `sessions`), `credentials`, `events` |
| `notifications` (new) | `events`, `credentials` (`Vault`, `Secret`, `Sealed` only) |
| `retention` (new) | — |

No cycle: `notifications` imports neither `checks` nor `sessions`. `ssh` and `trust` import neither `sessions`
nor `checks`.

GraphQL composition edges (R-011), added:

| Domain `graphql.rs` | May also import | For |
| --- | --- | --- |
| `devices` | `trust` (replaces `sessions` for these fields) | `SshSettings.hostKey`, `RdpSettings.serverCertificate` |
| `checks` | `notifications` | `Check.notifications` |
| `notifications`, `retention`, `trust` | `access` (R-022) | the `Authenticated` guard |

## K4 — Tunnel (`contracts/tunnel.md`)

**§1 Ticket.** `tunnelPath` adds `/watch` (a viewer's ticket from `joinShare`) and `/recording` (a playback
ticket from `openRecording`). Both keep every Section 1 property: 32 bytes, SHA-256 in memory, 30 seconds,
single use, bound to the client address and the path.

`startSession` adds, in this order:

1. When `devices.session_limit` is set and the device has that many live sessions, it refuses with CONFLICT
   ("This device has reached its session limit."). It counts under a transaction advisory lock on the device,
   so two starts cannot both pass. Viewers and file tunnels do not count (RA-036).
2. When `devices.record_sessions` is on, recording must be possible (a root that is set and writable, with
   1 GiB free). If it is not, the hub refuses with UNAVAILABLE ("Recording is on for this device, but the hub
   cannot record now."), D-14. The ticket then has `recording: true`, and the `session.start` audit change
   holds `"recorded": true`.

**§3 Handshake.** guacd's own recording stays off: `recording-path`, `recording-name`, `create-recording-path`,
`typescript-path` and `recording-include-keys` are always empty (D-11). A viewer's join (Section 10) sends
`select` with `$<gateway connection id>` (the ID from `ready`, GW-014), the viewer's `size` from
`joinShare.display`, `audio` empty, `video` empty, `image` as for the owner, `timezone` UTC, `name` the cleaned
viewer name or `viewer`, and `connect` with `read-only` = `true` for VIEW and empty for CONTROL. All other join
arguments are empty. WP-47 task 1 records guacd 1.6.0's join `args`.

**§4 Relay.** The relay passes each gateway instruction it sends to the owner to the session's recorder
(Section 11) when recording is on. It also passes it to the session's viewers (Section 10), which have their
own guacd connections. Client instructions are never recorded.

**§5 End.** When a session ends, every share of it ends, and each viewer's socket closes with 4004 (Section
10). The recorder finishes before the session row is written.

**§8 Terminal.** While a share exists, each output frame also goes to the replay buffer (64 KiB, memory only,
D-10) and to each viewer. When recording is on, each output frame and each `resize` go to the recorder. A
viewer that is slow never slows the owner: its queue holds at most 8 frames, and when that is full the viewer
closes with 4002.

**§10 Watch tunnel (new, RA-035).**

- `joinShare(token, name, display)` is a signed-out GraphQL root (D-7). The hub hashes the token and looks it
  up among the live shares. A miss counts as a failed login attempt for the client address (the Phase 1
  throttle). It refuses when the share expired, ended or has `maxViewers` viewers. Its error is the same
  FORBIDDEN text for every case. `name` is cleaned with `text::clean` to 32 characters.
- A share token is 32 bytes from the OS random source, base64url (43 characters). The hub keeps only its
  SHA-256, in memory, with the share. It is shown once, in `ShareLink.url` after `#`.
- Request `GET /watch?token=<ticket>`. Subprotocol `guacamole` for RDP and VNC, `anima-watch` for SSH. The
  refusals are those of Section 2. The watch ticket names its share; it does not name the owner's session to
  the viewer.
- RDP and VNC: the hub opens its own guacd connection with the join handshake (Section 3) and relays as
  Section 4: the internal UUID first, the BMP guard, and `error` with the fixed text. From the viewer it
  forwards only `sync`, `ack`, `nop`, `disconnect` and `size` (as Section 4) for VIEW, and also `key`, `mouse`
  and `touch` for CONTROL. Never `clipboard`, `file`, `put`, `get`, `audio`, `blob`, `end` or `argv`. Any
  other instruction closes 1002.
- SSH: hub to viewer: a text message `{"type":"joined","cols":C,"rows":R}`, then the replay buffer as binary
  frames, then live output as binary frames, `{"type":"resize","cols":C,"rows":R}` when the owner resizes,
  `{"type":"ping"}`, and `{"type":"end","reason":"SHARE_ENDED"}` before the close. Viewer to hub: `ping` for
  VIEW; for CONTROL also binary input, written to the owner's shell as the owner's input is (limits as §8.5).
  A viewer never sends `resize`: the owner sets the size.
- Close codes: 4004 for a share that ended (expiry, `endShare`, owner session end); 4002 for viewer silence
  (15 s) or a full viewer queue; 1002 for a protocol error. A viewer's end never ends the owner's session.
- Audit: `share.create` (access, expiry, maximum viewers), `share.join` (viewer client address and cleaned
  name, target the share), `share.end` (cause). Each also writes an Info event with `Source::Session`.

**§11 Recording (new, RA-040).**

- What: Guacamole format, every gateway instruction relayed to the owner after `ready`, as relayed (after the
  BMP guard and the `error` text rewrite), not the internal UUID instruction. asciicast v2 for SSH: the header
  `{"version":2,"width":C,"height":R,"timestamp":<unix seconds>}`, then `[t,"o",text]` for output and
  `[t,"r","CxR"]` for resize. Output bytes go through a streaming UTF-8 decoder (bad bytes become U+FFFD). No
  input event is written.
- File: `<ANIMA_RECORDING_ROOT>/<recording_id>.anrec`, created with `create_new`, mode 0600. Header: the magic
  bytes `ANREC1\0`, one format byte, the 16-byte recording ID, a 19-byte STREAM nonce prefix. Body: parts of at
  most 64 KiB of plaintext, each written as a 4-byte big-endian length and the XChaCha20-Poly1305 STREAM
  (BE32) ciphertext. The last part carries the STREAM "last" flag. The associated data of every part is the
  header bytes.
- Key: 32 random bytes per recording, sealed with the vault (context `sessions.recordings:<recording_id>`) in
  `sessions.recordings.key_sealed`, and wiped from memory after the last part. The row is written before the
  first part.
- Writes: a part is written when it is full, or 5 seconds after its first byte. The relay never waits on the
  disk: the recorder has a bounded queue (64 parts). When the queue is full, recording stops with
  `write_failed`.
- Stop: at `ANIMA_RECORDING_MAX_BYTES` (`size_limit`), below 1 GiB free (`disk_full`) or after a write error
  (`write_failed`). The recorder writes the last part, sets `stopped_reason`, and the session's
  `recording` becomes `STOPPED`. The hub publishes `sessionChanged` and writes a Warning event. The session
  goes on.
- End: the recorder writes the last part, sets `ended_at`, `size_bytes` and `complete = true`. A recording with
  no last part (hub crash) stays `complete = false`.
- Never in a log: recording content, the key, and the file path in the process log.

**§12 Playback tunnel (new, RA-041).**

- `openRecording(id)` returns a ticket for `/recording` (the `sessionId` is the recording's session) and writes
  the audit entry `recording.view`.
- Request `GET /recording?token=<ticket>`, subprotocol `anima-recording`, refusals as §8.1.
- Hub to client: `{"type":"recording","format":"GUACAMOLE"|"ASCIICAST","sizeBytes":N,"complete":B}`, then the
  plaintext in binary frames of at most 64 KiB, then `{"type":"done"}`. A part that fails to decrypt ends with
  `{"type":"failed","reason":"DAMAGED"}`, after the parts that verified. A missing file ends with
  `{"reason":"MISSING"}`. Client to hub: `ping`, `cancel`.
- Backpressure and silence as §8.4 and §8.5. At most 2 playbacks at once for each hub.

**§7 Tests** add:

- `startSession` beyond the limit gives CONFLICT, and two parallel starts at the limit give one ticket.
- A recorded fake-guacd session gives a file that has no plaintext byte run of the relayed instructions, and
  that decrypts to exactly the relayed stream. A swapped, truncated or reordered part fails. A file with a
  deleted row cannot be opened.
- A viewer of a fake-guacd session sends a `$id` join with `read-only=true`, and its `key` is dropped under
  VIEW. A terminal viewer gets the replay buffer, then live output, then 4004 when the owner ends.
- `joinShare` with a wrong token counts in the throttle. A watch ticket from another address gets 403.
- Playback of a recording with no last part ends `complete: false` after the verified parts.

## K5 — Tokens (`contracts/tokens.md`)

- No new semantic or component names. The light theme changes only semantic values (TOK-011).
- The token source gains a `light` mode. `tokens.css` gains `:root[data-theme="light"] { … }`, and
  `@media (prefers-color-scheme: light) { :root[data-theme="system"] { … } }`. `data-theme="dark"`, or no
  attribute, keeps today's values. `prefers-contrast: more` applies on top of either mode (MAT-006).
- If D-27 takes option B: the primitive row adds `nord-11-deep`, `nord-12-deep`, `nord-13-deep`,
  `nord-14-deep` and `nord-15-deep`. Each is the Nord colour mixed toward `nord-0` until it reaches 3:1 against
  `nord-6`. Only the light mode's `status-*` and `chart-*` values use them. Components never name them.
- The Rust output (`crates/theme`) does not change.

## K6 — Fixtures (`contracts/fixtures/phase3.json`)

Synthetic only. No real secret, address, mailbox or URL: hosts are `example.net` or 10.x addresses, and secret
fields are absent (SEC-005). IDs follow `0190a000-0000-7000-80NN-0000000000MM`, with NN segments 17
notification channels, 18 deliveries, 19 API keys, 20 recordings, 21 shares, 22 checks. They never collide with
`phase1.json` (00 to 04) or `phase2.json` (11 to 16).

- One channel of each kind: a webhook with origin `https://hooks.example.net`; ntfy on `https://ntfy.example.net`;
  email to `ops@example.net`; Apprise in stateless mode. One of them is `onByDefault`.
- Deliveries: each kind, ok and failed, with the fixed messages.
- Check notifications for the Phase 1 HTTPS check: two channels, `resendEveryBeats: 5`.
- Notification settings with the default template.
- Two API keys, one READ and one WRITE, one of them expired. Hashes only, no secret.
- A device with `sessionLimit: 2` and `recordSessions: true`.
- A live VNC session with one VIEW share and one viewer. A live SSH session with `recording: ON`.
- Recordings: one complete VNC recording, one incomplete SSH recording, one with `stopped_reason: size_limit`.
  Each has metadata only, plus two small synthetic plaintext streams for player tests: a Guacamole dump of a
  drawn rectangle and an asciicast of `echo` output.
- One `DOMAIN_EXPIRY` check on `example.net` with `domainDaysLeft: 12`.
- Retention periods at their defaults, and check defaults at the schema defaults.
- One AMD and one Apple GPU device, to add to the Phase 2 NVIDIA device.

## K7 — Conventions (`contracts/conventions.md`)

- **IDs:** add `ApiKeyId`, `ChannelId`, `DeliveryId`, `ShareId`, `RecordingId`.
- **Configuration:**

  | Variable | Meaning | Default |
  | --- | --- | --- |
  | `ANIMA_PUBLIC_ORIGIN` | The origin clients use (`https://hub.anima.ing`). The cookie Origin check, share link URLs and the CSP use it | `https://hub.anima.ing` |
  | `ANIMA_WEB_ROOT` | The folder with the web client's static build. Empty: no web client, and unknown paths get 404 | empty (compose sets `/usr/share/anima/web`) |
  | `ANIMA_RECORDING_ROOT` | The folder for recording files (K4 §11). Empty: recording is unavailable | empty (compose sets `/var/lib/anima/recordings`) |
  | `ANIMA_RECORDING_MAX_BYTES` | The plaintext limit of one recording | `2147483648` |

  `ANIMA_PROBE_DENY` now also applies to notification and RDAP targets (P3-003).

- **Authentication transport** adds:
  - **Browser:** the cookie `__Host-anima` (`Secure; HttpOnly; SameSite=Strict; Path=/`, `Max-Age` 7 days),
    set only by `signInBrowser` and cleared by `logout`. The hub uses the cookie only when the request has no
    `Authorization` header and its `Origin` equals `ANIMA_PUBLIC_ORIGIN`. That applies to `POST /graphql` and
    to the `GET /graphql` WebSocket upgrade. A socket that signed in by cookie may send an empty
    `connection_init` payload. A cookie with any other Origin, or with none, is ignored: the request is
    signed out.
  - **API keys:** the same `Authorization: Bearer` header. The secret is `anima_` plus 43 base64url characters.
    The kind rules of D-23 are enforced in the access guard, before any resolver, with FORBIDDEN.
  - **Share links:** the only bearer value in a URL, and only in the fragment, which browsers never send.
- **HTTP surface (Phase 3).** Every route sits behind the NET-002 layer and is logged by its template only.

  | Route | Authentication | Owner |
  | --- | --- | --- |
  | `POST /graphql`, `GET /graphql` (WebSocket) | Bearer header or browser cookie; signed out only for one `login`, `signInBrowser` or `joinShare` | `access`, each domain |
  | `/terminal`, `/tunnel`, `/files`, `/watch`, `/recording` | Single-use ticket (K4 §1) | `sessions` |
  | `GET\|POST /push/{token}` | Push token | `checks` |
  | `GET /metrics` | Bearer token, READ or higher | `checks`, behind `access::bearer_layer` |
  | `GET /health` | None; body `ok` | wiring |
  | Any other `GET` (the web client) | None; static files only, `index.html` for unknown paths | `web_client.rs` |

  Static file headers: `Content-Security-Policy: default-src 'self'; script-src 'self' <SvelteKit hashes>;
  connect-src 'self' wss://<public host>; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline';
  font-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'`;
  `X-Content-Type-Options: nosniff`; `Referrer-Policy: no-referrer`; `Cross-Origin-Opener-Policy: same-origin`;
  `Strict-Transport-Security: max-age=31536000`; `Cache-Control: no-cache` for `index.html` and
  `public, max-age=31536000, immutable` under `/_app/immutable/`. CORS (`CLIENT_ORIGINS`) does not change.

- **Prometheus series (INT-001, INT-002).** Content type `text/plain; version=0.0.4; charset=utf-8`. Labels
  `check_id`, `check_name`, `check_type`, `device_id`, `device_name`, `site`. Label values are cleaned with
  `text::clean` and escaped as the format requires. No label holds a status, a time, an address or a message.

  | Series | Type | Value |
  | --- | --- | --- |
  | `anima_check_status` | gauge | 0 Down, 1 Up, 2 Degraded, 3 Maintenance, 4 Unknown (also paused) |
  | `anima_check_up` | gauge | 1 for Up or Degraded, else 0 |
  | `anima_check_response_time_seconds` | gauge | The last heartbeat; absent when it has none |
  | `anima_check_uptime_ratio` | gauge | 0..1, with label `window` = `1d`, `7d` or `30d`, from the aggregates (CHK-011) |
  | `anima_check_certificate_days_left` | gauge | Only for checks with a certificate |
  | `anima_check_domain_days_left` | gauge | Only for domain expiry checks |
  | `anima_device_status` | gauge | As `anima_check_status`; labels `device_id`, `device_name`, `site` |

- **Never log (LOG-007)** adds: API key secrets, browser tokens and cookie values, share tokens and watch
  tickets, notification secrets (whole webhook URLs, webhook header values, ntfy topics and tokens, SMTP
  passwords, Apprise URLs and keys), email recipient addresses in the process log, recording content, recording
  keys and recording file paths in the process log, the replay buffer, and a notification target's reply body.
- **Errors:** no new codes. A token-kind refusal and a READ refusal are FORBIDDEN. A refused notification target
  is a delivery with `ok: false` and the fixed message "The target address is not permitted.", never an error
  that names the address.
- **Tests:** as before. A test that proves a Phase 3 requirement names the ID, for example
  `fn not_005_resend_after_n_down_beats()` or `it('RA-035: viewer is read-only', …)`.
