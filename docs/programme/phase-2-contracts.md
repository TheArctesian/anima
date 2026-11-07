# Phase 2 contract deltas (K1–K7)

Status: **Adopted.** The deltas are now in `contracts/` (WP-20, R-065); this file is the record of the proposal.
Plan: [phase-2-plan.md](phase-2-plan.md).

Why this file and not `contracts/`: the CON-002 test (`crates/hub/tests/it/contract.rs`) requires the hub
schema and K1 to be equal. A probe that added `RDP` and `VNC` to `Protocol` failed it with
`Protocol.RDP: in K1, not in the hub`. On approval, WP-20 adds the `@pending` rule (D-1) and the programme
orchestrator copies each section below into its contract file in one commit, with the file header changed
to "frozen for Phase 2". Until then `contracts/` stays the Phase 1 freeze.

Every delta adds; nothing in Phase 1 is renamed or removed.

## K1 — GraphQL additions (`contracts/schema.graphql`)

Each new type, field, enum value and input field carries `@pending` until the hub declares it (P2-002).

```graphql
"A K1 addition that the hub does not serve yet (P2-002). CON-002 ignores it only while the hub lacks it."
directive @pending on OBJECT | FIELD_DEFINITION | ARGUMENT_DEFINITION | ENUM | ENUM_VALUE | UNION | INPUT_OBJECT | INPUT_FIELD_DEFINITION

# ---- shared and existing enums
enum Protocol { SSH  RDP @pending  VNC @pending }
enum SessionEndReason { ...  "The RDP certificate differs from the accepted one (D-5)." SERVER_CERTIFICATE_MISMATCH @pending }
enum EventSource { ...  METRICS @pending }

# ---- devices (WP-21)
enum RdpSecurity @pending { ANY NLA NLA_EXT TLS VMCONNECT RDP }
enum RdpResizeMethod @pending { DISPLAY_UPDATE RECONNECT }
enum MetricsSource @pending { NONE SSH }          # PROXMOX in Phase 4
"RA-020. TELNET joins when Telnet exists."
enum AccessWarning @pending { IGNORED_CERTIFICATE RDP_SECURITY UNENCRYPTED_VNC }

type ServerCertificate @pending {
  fingerprintSha256: String!
  subject: String
  notAfter: DateTime
  firstSeenAt: DateTime!
}

type RdpSettings @pending {
  enabled: Boolean!
  port: Int!                      # default 3389
  username: String!
  domain: String
  security: RdpSecurity!          # default NLA
  ignoreCertificate: Boolean!     # default false
  resizeMethod: RdpResizeMethod!  # default DISPLAY_UPDATE
  audioOut: Boolean!              # default true
  microphone: Boolean!            # default false
  sharedDrive: Boolean!           # default false (RA-021)
  credentialStored: Boolean!
  serverCertificate: ServerCertificate
}

type VncSettings @pending {
  enabled: Boolean!
  port: Int!                      # default 5900
  "macOS Screen Sharing needs one."
  username: String
  credentialStored: Boolean!
}

"What may leave or enter the device in a session (RA-032, RA-033). All default true."
type TransferPolicy @pending { upload: Boolean!  download: Boolean!  copy: Boolean!  paste: Boolean! }

type Device {
  ...
  rdp: RdpSettings! @pending
  vnc: VncSettings! @pending
  "Lower-case aa:bb:cc:dd:ee:ff (RA-031)."
  macAddress: String @pending
  metricsSource: MetricsSource! @pending
  transfer: TransferPolicy! @pending
  accessWarnings: [AccessWarning!]! @pending
  webInterfaces: [WebInterface!]! @pending                       # web_interfaces
  webInterfaceSuggestions: [WebInterfaceSuggestion!]! @pending   # web_interfaces (WEB-002)
  readings: Readings @pending                                    # metrics
  metrics(range: MetricRange!): [MetricSeries!]! @pending        # metrics (MET-003, MET-009)
  gpus: [Gpu!]! @pending                                         # metrics
}

type Site { ...  "The device that sends Wake-on-LAN packets at this site (D-2)." wakeRelay: Device @pending }

input RdpSettingsInput @pending {
  enabled: Boolean!  port: Int = 3389  username: String!  domain: String
  security: RdpSecurity = NLA  ignoreCertificate: Boolean = false
  resizeMethod: RdpResizeMethod = DISPLAY_UPDATE  audioOut: Boolean = true
  microphone: Boolean = false  sharedDrive: Boolean = false
}
input VncSettingsInput @pending { enabled: Boolean!  port: Int = 5900  username: String }
input TransferPolicyInput @pending { upload: Boolean = true  download: Boolean = true  copy: Boolean = true  paste: Boolean = true }
input PasswordCredentialInput @pending { password: String! }

input CreateDeviceInput { ...  rdp: RdpSettingsInput @pending  vnc: VncSettingsInput @pending
  macAddress: String @pending  metricsSource: MetricsSource = NONE @pending  transfer: TransferPolicyInput @pending }
input UpdateDeviceInput { ...  rdp: RdpSettingsInput @pending  vnc: VncSettingsInput @pending
  "An empty string clears the address."
  macAddress: String @pending  metricsSource: MetricsSource @pending  transfer: TransferPolicyInput @pending }

input ConnectionTestInput { ...  protocol: Protocol = SSH @pending  rdp: RdpSettingsInput @pending
  vnc: VncSettingsInput @pending  password: PasswordCredentialInput @pending }
type ConnectionTestResult { ...  serverCertificate: ServerCertificate @pending  certificateChanged: Boolean! @pending
  certificateMatched: Boolean! }  # CR-3, added after freeze

# ---- checks (WP-29). Identifiers fixed forever (CHK-009): docker, snmp, tailscale_ping, systemd.
enum CheckType { ...  DOCKER @pending  SNMP @pending  TAILSCALE_PING @pending  SYSTEMD @pending }
union CheckConfig = ... | DockerConfig | SnmpConfig | TailscalePingConfig | SystemdConfig   # header change, @pending on the four members' types

"Up when running (and healthy, when a health check exists); Degraded when starting; else Down. Through the SSH link (D-3)."
type DockerConfig @pending { container: String!  timeoutMs: Int! }
"Up when ActiveState is active. Through the SSH link (D-3)."
type SystemdConfig @pending { unit: String!  timeoutMs: Int! }
enum SnmpVersion @pending { V2C V3 }
"Community and v3 passwords are never returned (SEC-005)."
type SnmpConfig @pending { port: Int!  version: SnmpVersion!  oid: String!  "Up when equal; null: Up when the OID answers." expected: String
  username: String  timeoutMs: Int! }
"Disco ping through the LocalAPI proxy (D-4). The target defaults to the device host."
type TailscalePingConfig @pending { target: String  timeoutMs: Int! }

input CheckConfigInput { ...  docker: DockerConfigInput @pending  snmp: SnmpConfigInput @pending
  tailscalePing: TailscalePingConfigInput @pending  systemd: SystemdConfigInput @pending }
input DockerConfigInput @pending { container: String!  timeoutMs: Int = 10000 }
input SystemdConfigInput @pending { unit: String!  timeoutMs: Int = 10000 }
input SnmpConfigInput @pending { port: Int = 161  version: SnmpVersion = V2C  oid: String!  expected: String
  community: String  username: String  authPassword: String  privacyPassword: String  timeoutMs: Int = 5000 }
input TailscalePingConfigInput @pending { target: String  timeoutMs: Int = 5000 }

# ---- metrics (WP-28)
enum MetricKind @pending { CPU_PERCENT MEMORY_PERCENT MEMORY_BYTES DISK_PERCENT DISK_BYTES NETWORK_IN_BPS
  NETWORK_OUT_BPS UPTIME_SECONDS LOAD_1 LOAD_5 LOAD_15 GPU_PERCENT VRAM_PERCENT VRAM_BYTES GPU_TEMPERATURE_C GPU_POWER_W }
enum MetricRange @pending { HOUR DAY WEEK }    # MET-003
enum GpuVendor @pending { NVIDIA AMD INTEL APPLE }
type Gpu @pending { index: Int!  name: String!  vendor: GpuVendor!  "False for integrated and Apple GPUs (MET-007)." hasVram: Boolean! }
type MetricPoint @pending { at: DateTime!  value: Float!  min: Float  max: Float }
"Label: mount point, interface name, or 'GPU <index> <name>' (MET-009). Bytes are Float (above 2^31)."
type MetricSeries @pending { kind: MetricKind!  label: String!  gpuIndex: Int  points: [MetricPoint!]! }
"The latest values for the card (MET-004, MET-008). Disk: the fullest mount. VRAM: the GPU with the highest use."
type Readings @pending { at: DateTime!  cpuPercent: Float  memoryPercent: Float  diskPercent: Float  vramPercent: Float  vramGpu: Gpu }
type MetricSample @pending { kind: MetricKind!  label: String!  value: Float! }
type MetricsAdded @pending { deviceId: ID!  at: DateTime!  samples: [MetricSample!]!  readings: Readings! }

# ---- sessions (WP-22, WP-27)
type Session { ...  clientAddr: IpAddr! @pending }       # RA-034: which computer holds it
type SessionTicket { ...  "A Wake-on-LAN packet was sent; the opening waits longer (K4 §1)." wakeSent: Boolean! @pending }
type WakeResult @pending { sentAt: DateTime!  relayName: String! }
type SessionChange @pending { session: Session! }

# ---- web interfaces (WP-30)
enum OpenIn @pending { ANIMA_TAB SYSTEM_BROWSER }
type WebInterface @pending { id: ID!  deviceId: ID!  label: String!  url: String!  openIn: OpenIn!
  "Pinned on accept (WEB-008)." pinnedFingerprintSha256: String  pinnedAt: DateTime }
type WebInterfaceSuggestion @pending { checkId: ID!  url: String!  label: String! }
input WebInterfaceInput @pending { "Defaults to the URL's host." label: String  url: String!  openIn: OpenIn = ANIMA_TAB }

# ---- root additions
type Mutation {
  ...
  setSiteWakeRelay(siteId: ID!, deviceId: ID): Site! @pending
  setRdpCredential(deviceId: ID!, credential: PasswordCredentialInput!): RdpSettings! @pending
  clearRdpCredential(deviceId: ID!): RdpSettings! @pending
  setVncCredential(deviceId: ID!, credential: PasswordCredentialInput!): VncSettings! @pending
  clearVncCredential(deviceId: ID!): VncSettings! @pending
  "The fingerprint must equal the one the hub saw, or CONFLICT."
  acceptServerCertificate(deviceId: ID!, fingerprintSha256: String!): ServerCertificate! @pending
  "A ticket for the files tunnel of a live SSH session (K4 §9)."
  openFiles(sessionId: ID!): SessionTicket! @pending
  wakeDevice(deviceId: ID!): WakeResult! @pending
  createWebInterface(deviceId: ID!, input: WebInterfaceInput!): WebInterface! @pending
  updateWebInterface(id: ID!, input: WebInterfaceInput!): WebInterface! @pending
  deleteWebInterface(id: ID!): ID! @pending
  acceptWebInterfaceSuggestion(checkId: ID!): WebInterface! @pending
  dismissWebInterfaceSuggestion(checkId: ID!): ID! @pending
  "Pins the certificate and writes an audit entry (WEB-008, WEB-009)."
  acceptWebCertificate(id: ID!, fingerprintSha256: String!): WebInterface! @pending
}
type Subscription {
  ...
  "Every start and end of a session, from every client (RA-034)."
  sessionChanged: SessionChange! @pending
  metricsAdded(deviceId: ID): MetricsAdded! @pending
}
```

`Query.sessions(live: true)` already exists and serves the live list. `startSession` takes `RDP` and `VNC`
unchanged. `DisplayInput` is unchanged (GW-012).

## K2 — Database (`contracts/database.md`)

| Range | Domain (package) | Schema | New tables or changes |
| --- | --- | --- | --- |
| 0110–0199 | devices (WP-21) | `devices` | `rdp_settings`, `vnc_settings` (device_id pk fk cascade, columns as K1, defaults in the schema, DB-011); `devices.mac_address macaddr null`, `devices.metrics_source text check in ('none','ssh') default 'none'`, `devices.transfer_upload`, `_download`, `_copy`, `_paste bool default true`; `sites.wake_relay_device_id uuid null fk -> devices.devices on delete set null` |
| 0210–0299 | credentials (WP-21) | `credentials` | `secrets.kind` check adds `rdp_password`, `vnc_password` |
| 0310–0399 | checks (WP-29) | `checks` | `checks.type` identifiers `docker`, `snmp`, `tailscale_ping`, `systemd`; SNMP secrets sealed in the config with context `checks.checks.snmp:<check_id>` |
| 0610–0649 | sessions (WP-22) | `sessions` | `sessions.protocol` check `('ssh','rdp','vnc')`; `end_reason` adds `server_certificate_mismatch`; `server_certificates` (device_id pk fk cascade, fingerprint_sha256, subject null, not_after null, first_seen_at, accepted_by) |
| 0650–0699 | sessions (WP-27) | `sessions` | None expected (transfers go to the audit log) |
| 0700–0799 | metrics (WP-28) | `metrics` | `series` (id uuid pk, device_id fk cascade, kind, label, gpu_index null, unique (device_id, kind, label)); `samples` (series_id, at, value double precision; partitioned by day on `at`; pk (series_id, at); no fk, as heartbeats); `minute`, `hour`, `day` (series_id, bucket, avg, min, max, count; pk (series_id, bucket)); `gpus` (device_id fk cascade, index, name, vendor, has_vram, found_at; pk (device_id, index)); `discovery` (device_id pk fk cascade, found_at, boot_at) for MET-006 |
| 0800–0849 | web_interfaces (WP-30) | `web_interfaces` | `interfaces` (id uuid pk, device_id fk cascade, label, url, open_in check in ('anima_tab','system_browser'), created_at); `pins` (interface_id pk fk cascade, fingerprint_sha256, accepted_by, accepted_at); `dismissed` (check_id pk fk -> checks.checks on delete cascade) |
| 0850–0999 | reserved | — | Phase 3 (notifications, API keys) |

Retention: `metrics.samples` 30 days; `minute` 7 days, `hour` 90 days, `day` 2 years (DB-009). The metrics
retention job removes samples whose series no longer exists (DEV-015, the R-004 pattern). Partition
functions follow the K2 "Partitions" rule (`SECURITY DEFINER`, owner role).

## K3 — Hub domain interfaces (`contracts/hub-domains.md`)

```rust
// devices (WP-21): Device gains the fields of K1 Device.
pub struct RdpSettings { pub enabled: bool, pub port: u16, pub username: String, pub domain: Option<String>,
    pub security: RdpSecurity, pub ignore_certificate: bool, pub resize: ResizeMethod, pub audio_out: bool,
    pub microphone: bool, pub shared_drive: bool }
pub struct VncSettings { pub enabled: bool, pub port: u16, pub username: Option<String> }
pub struct TransferPolicy { pub upload: bool, pub download: bool, pub copy: bool, pub paste: bool }
pub struct MacAddress([u8; 6]);            // parsed at the API edge; Display renders aa:bb:cc:dd:ee:ff
pub enum MetricsSource { None, Ssh }
pub async fn wake_relay(db: &Db, site: SiteId) -> Result<Option<Device>, Error>;

// credentials (WP-21)
pub enum PasswordKind { Rdp, Vnc }         // kinds rdp_password, vnc_password; context as R-010
pub async fn store_password(conn: &mut PgConnection, vault: &Vault, device: DeviceId, kind: PasswordKind, s: Secret) -> Result<(), Error>;
pub async fn load_password(db: &Db, vault: &Vault, device: DeviceId, kind: PasswordKind) -> Result<Option<Secret>, Error>;
pub async fn with_password(db: &Db, devices: &[DeviceId], kind: PasswordKind) -> Result<HashSet<DeviceId>, Error>;
pub async fn clear_password(conn: &mut PgConnection, device: DeviceId, kind: PasswordKind) -> Result<bool, Error>;

// sessions (WP-27): the one SSH path for hub-run work (P2-003)
/// A command made only from static text and hub-rendered values; operator text cannot enter it (CHK-012).
pub struct FixedCommand { /* program: &'static str, args: Vec<Rendered> */ }
impl FixedCommand { pub const fn new(program: &'static str) -> Self; pub fn mac(self, mac: devices::MacAddress) -> Self; }
/// A signed-in SSH connection pinned to the accepted host key, to a target that passed the target rule and deny list (P2-004).
pub struct SshLink { /* russh handle */ }
pub async fn link(db: &Db, vault: &credentials::Vault, policy: &AddressPolicy, device: &Device, now: DateTime<Utc>) -> Result<SshLink, Error>;
impl SshLink { pub async fn run(&mut self, c: &FixedCommand, limit: Duration) -> Result<CommandOutput, Error>; }
pub async fn wake(db: &Db, vault: &credentials::Vault, policy: &AddressPolicy, bus: &events::Bus, device: DeviceId, now: DateTime<Utc>) -> Result<WakeSent, Error>;
pub fn files_router(db: Db, vault: credentials::Vault, bus: events::Bus, clock: Clock, live: Live) -> axum::Router;   // GET /files, K4 §9

// sessions (WP-22)
pub async fn server_certificates(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, ServerCertificate>, Error>;
// tunnel_router now serves RDP and VNC and is mounted; Live also broadcasts SessionChange (D-15).

// checks (WP-29)
pub async fn http_urls(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<(CheckId, String)>>, Error>;   // WEB-002

// metrics (WP-28)
pub struct Collecting { pub vault: credentials::Vault, pub policy: AddressPolicy }
#[derive(Clone)] pub struct Feed { /* broadcast of MetricsAdded */ }    // D-15
pub fn spawn_collector(db: Db, feed: Feed, bus: events::Bus, clock: Clock, collecting: Collecting) -> JoinHandle<()>;   // MET-001
pub async fn readings(db: &Db, devices: &[DeviceId], now: DateTime<Utc>) -> Result<HashMap<DeviceId, Readings>, Error>;
pub async fn series(db: &Db, device: DeviceId, range: MetricRange, now: DateTime<Utc>) -> Result<Vec<MetricSeries>, Error>;
pub async fn gpus(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<Gpu>>, Error>;
// Wiring only: create_partitions_ahead, drop_expired_partitions (as events and checks).

// web_interfaces (WP-30)
pub async fn for_devices(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<WebInterface>>, Error>;
pub async fn suggestions(db: &Db, devices: &[DeviceId]) -> Result<HashMap<DeviceId, Vec<Suggestion>>, Error>;

// events: Source gains Metrics. No new Signal (D-15).
```

`AddressPolicy` moves from `checks` to the leaf module `crates/hub/src/targets.rs` (programme-owned, as
R-003), so `sessions`, `checks` and `metrics` share one deny-list type without new edges. WP-20 makes the move.

Allowed domain dependencies, new edges only (REPO-004):

| Domain | May also import |
| --- | --- |
| `checks` | `sessions` (`link`, `SshLink`, `FixedCommand` only; no cycle: `sessions` does not import `checks`) |
| `metrics` (new) | `devices`, `sessions` (link only), `credentials` (`Vault` only), `events` |
| `web_interfaces` (new) | `checks` (`http_urls` only), `events` |
| `devices/graphql.rs` (composition, R-011) | `metrics`, `web_interfaces` |
| `metrics`, `web_interfaces` `graphql.rs` | `access` (R-022) |

Wake-on-LAN lives in `sessions` (RA-031 is a remote access rule), so a session start can wake a device
without a `sessions` ↔ `power` cycle.

## K4 — Tunnel (`contracts/tunnel.md`)

**§1 Ticket.** `tunnelPath` is `/tunnel` for RDP and VNC, `/files` for a files ticket. `startSession` checks,
per protocol, that the protocol is on. RDP also needs an accepted server certificate unless `security` is
`RDP` or `ignoreCertificate` is on (CONFLICT, as for SSH host keys). When the device is Down and has a MAC
address and its site has a wake relay, `startSession` sends the packet first and sets `wakeSent`. The
opening limit is then 120 seconds instead of 8.

**§2 Refusals.** Add: protocol off, no accepted certificate, target refused by the rule or the deny list
(P2-004): 503, the session ends `GATEWAY_ERROR`.

**§3 Handshake.** `select` with `rdp` or `vnc`. `audio` lists `audio/L8` and `audio/L16` when `audioOut` is
on, else empty. Values (all others empty, `wol-*` always empty, GW-013):

| Argument | RDP | VNC |
| --- | --- | --- |
| `hostname`, `port` | checked target address, settings port | same |
| `username`, `domain` | settings | `username` when set |
| `password` | `load_password(Rdp)` | `load_password(Vnc)` |
| `security`, `ignore-cert` | settings | — |
| `cert-fingerprints` | the accepted `sha256` fingerprint (D-5) | — |
| `resize-method` | `display-update` or `reconnect` | — |
| `disable-audio`, `enable-audio-input` | not `audioOut`, `microphone` | — |
| `enable-drive`, `drive-path`, `create-drive-path` | `sharedDrive`, `<ANIMA_GUACD_DRIVE_ROOT>/<device_id>`, `true` (RA-021) | — |
| `disable-download`, `disable-upload` | not `download`, not `upload` | — |
| `disable-copy`, `disable-paste` | not `copy`, not `paste` (RA-033) | same |

Before the handshake, for RDP with TLS or NLA, the hub reads the server certificate itself (X.224 request,
then TLS) and compares its SHA-256 with the accepted one. A difference ends the session with `error`
`"Certificate changed"` `0x0303`, close 4003, `SERVER_CERTIFICATE_MISMATCH`, an Error event, and the warning
modal. `testConnection` with `protocol: RDP` reads it the same way; only `acceptServerCertificate` records it.

**§4 Relay.** The allow-list adds, each only when the device's policy permits it:

- `clipboard` from the client only when `paste` is on; the hub drops the gateway's `clipboard` streams when
  `copy` is off (with guacd's own flags as a second guard);
- `audio` (2) from the client only when `microphone` is on, with its `blob` and `end`;
- `file` (3), `get` (2) and `put` (4) only when `sharedDrive` is on: `file` and `put` need `upload`, `get`
  of a file needs `download`;
- `argv` for `required`: RDP `username`, `password`, `domain`; VNC `username`, `password` (GW-015).

**§5 End.** Add `SERVER_CERTIFICATE_MISMATCH`, close 4003, Error event.

**§8.7 Terminal.** With `copy` off the client copies nothing from the terminal; with `paste` off it sends
no paste. The hub cannot enforce these for bytes (D-6).

**§9 Files tunnel (new, SFTP, RA-032).**

- Request `GET /files?token=<token>`, subprotocol `anima-files`, refusals as §8.1. The ticket comes from
  `openFiles(sessionId)`: the session must be a live SSH session of the same operator and client address.
  The files tunnel ends when that session ends.
- The hub opens its own SSH connection through the link (pinned key, stored credential) and requests only
  the `sftp` subsystem. The terminal connection (§8) is unchanged.
- Client text messages (JSON, closed set, at most 4 KiB): `list {path}`, `download {path}`,
  `upload {path, size}`, `cancel`, `ping`. Paths: UTF-8, at most 4096 bytes, no NUL. Binary frames from the
  client carry upload data only after `upload`, at most 64 KiB each, `size` bytes in all.
- Hub text messages: `entries {path, entries: [{name, kind: file|dir|link, size, modifiedAt}]}` (at most
  10 000), `download {size}` then binary frames, `done`, `refused {reason: DOWNLOAD_OFF | UPLOAD_OFF |
  NOT_FOUND | DENIED | EXISTS}`, `ping`.
- One transfer at a time. Upload never overwrites (`EXISTS`). The flags are read at each request.
- Backpressure and silence limits as §8.4 and §8.5. Each finished or refused transfer writes the audit entry
  `file.upload` or `file.download` with the path and size and no content, and an Info event (LOG-005, LOG-007).

**§7 Tests.** Add: recorded guacd 1.6.0 `args` for `rdp` and `vnc` give the §3 values; a fake guacd sees no
`clipboard` when paste is off; a changed certificate ends 4003 before any `connect`; files: a fake SSH server
sees only the `sftp` subsystem, each `refused` reason, an upload over `size`, and a ticket for an ended session.

## K5 — Tokens (`contracts/tokens.md`)

Colour, chart: `chart-grid`, `chart-axis`, `chart-threshold` (the 85 % line, MET-004), `chart-series-1` …
`chart-series-6` (Nord frost then aurora, so up to six GPUs or mounts differ). WP-20 adds them to
`design/tokens/` and the K5 table in one commit (R-049 pattern).

## K6 — Fixtures (`contracts/fixtures/phase2.json`)

Synthetic only: a Windows device with RDP (NLA, a certificate fingerprint), a Mac with VNC and a MAC
address, a Linux device with SSH metrics and an NVIDIA GPU, a device with two web interfaces (one pinned),
one check of each new type, a site wake relay, live and ended sessions of each protocol, and 7 days of metric
aggregates. IDs follow the Phase 1 pattern (`0190a000-…-8002-…`).

## K7 — Conventions (`contracts/conventions.md`)

- IDs: add `WebInterfaceId`, `MetricSeriesId`.
- Configuration: `ANIMA_GUACD_DRIVE_ROOT` (guacd's path for shared drives; default `/var/lib/guacd/drives`);
  `ANIMA_TAILSCALE_LOCALAPI` (`host:port` of the LocalAPI proxy, D-4; empty: Tailscale ping checks are Unknown
  with a message). `ANIMA_PROBE_DENY` now also applies to session and metrics targets (P2-004).
- Never log (LOG-007) adds: RDP and VNC passwords, SNMP communities and v3 passwords, file content and paths
  in the process log (paths go only to the audit log).
- K1 header comment, line 4: name `contracts/schema.graphql` as the client's generation source, guarded by
  CON-002 (WP-17 E-2). Comment only.
