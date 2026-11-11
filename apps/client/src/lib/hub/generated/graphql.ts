/* eslint-disable */
/** Internal type. DO NOT USE DIRECTLY. */
type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
/** Internal type. DO NOT USE DIRECTLY. */
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
import type { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';
/** RA-020. TELNET joins when Telnet exists. */
export type AccessWarning =
  | 'IGNORED_CERTIFICATE'
  | 'RDP_SECURITY'
  | 'UNENCRYPTED_VNC';

export type ApiKeyInput = {
  /** Required; at most one year from now (D-23). */
  expiresAt: string;
  name: string;
  scope: TokenScope;
};

export type AppriseMode =
  | 'CONFIG_KEY'
  | 'STATELESS_URLS';

export type AppriseSettingsInput = {
  key?: string | null | undefined;
  mode?: AppriseMode | null | undefined;
  server: string;
  tags?: Array<string> | null | undefined;
  urls?: Array<string> | null | undefined;
};

export type BasicAuthInput = {
  password: string;
  username: string;
};

/** Whether the hub can reach the VPN broker on the VPS (VP-7, BD-15). */
export type BrokerState =
  /** The last call failed or its reply was unreadable. */
  | 'FAILED'
  /** The socket is not there: the broker is not installed on the VPS. */
  | 'NOT_INSTALLED'
  /** ANIMA_VPN_BROKER is not set. */
  | 'NOT_SET'
  | 'READY';

export type BuildFileInput = {
  name: string;
  /** 64 hex digits. */
  sha256: string;
  /** In bytes. */
  size: number;
};

/** Section 3.10.1 says which changes are permitted (BLD-010). */
export type BuildJobState =
  | 'BUILDING'
  | 'CANCELLED'
  | 'CLAIMED'
  | 'FAILED'
  | 'PUBLISHED'
  | 'QUEUED'
  | 'UPLOADING';

/** What a release is built for (BLD-001, BLD-003). */
export type BuildTarget =
  /** An app and a DMG, built on macOS on aarch64. */
  | 'DARWIN_UNIVERSAL'
  /** An AppImage, built on Linux on x86_64. */
  | 'LINUX_X86_64';

/** As Tauri names it. */
export type BuilderArch =
  | 'AARCH64'
  | 'X86_64';

export type BuilderOs =
  | 'LINUX'
  | 'MACOS';

export type BuilderPlatformInput = {
  arch: BuilderArch;
  os: BuilderOs;
};

export type ChannelKind =
  | 'APPRISE'
  | 'EMAIL'
  | 'NTFY'
  | 'WEBHOOK';

export type ChannelSettingsInput =
  {   apprise: AppriseSettingsInput; email?: never; ntfy?: never; webhook?: never; }
  |  { apprise?: never;   email: EmailSettingsInput; ntfy?: never; webhook?: never; }
  |  { apprise?: never; email?: never;   ntfy: NtfySettingsInput; webhook?: never; }
  |  { apprise?: never; email?: never; ntfy?: never;   webhook: WebhookSettingsInput; };

export type CheckConfigInput =
  {   dns: DnsConfigInput; docker?: never; domainExpiry?: never; group?: never; http?: never; icmp?: never; push?: never; snmp?: never; systemd?: never; tailscalePing?: never; tcp?: never; tls?: never; }
  |  { dns?: never;   docker: DockerConfigInput; domainExpiry?: never; group?: never; http?: never; icmp?: never; push?: never; snmp?: never; systemd?: never; tailscalePing?: never; tcp?: never; tls?: never; }
  |  { dns?: never; docker?: never;   domainExpiry: DomainExpiryConfigInput; group?: never; http?: never; icmp?: never; push?: never; snmp?: never; systemd?: never; tailscalePing?: never; tcp?: never; tls?: never; }
  |  { dns?: never; docker?: never; domainExpiry?: never;   group: GroupConfigInput; http?: never; icmp?: never; push?: never; snmp?: never; systemd?: never; tailscalePing?: never; tcp?: never; tls?: never; }
  |  { dns?: never; docker?: never; domainExpiry?: never; group?: never;   http: HttpConfigInput; icmp?: never; push?: never; snmp?: never; systemd?: never; tailscalePing?: never; tcp?: never; tls?: never; }
  |  { dns?: never; docker?: never; domainExpiry?: never; group?: never; http?: never;   icmp: IcmpConfigInput; push?: never; snmp?: never; systemd?: never; tailscalePing?: never; tcp?: never; tls?: never; }
  |  { dns?: never; docker?: never; domainExpiry?: never; group?: never; http?: never; icmp?: never;   push: boolean; snmp?: never; systemd?: never; tailscalePing?: never; tcp?: never; tls?: never; }
  |  { dns?: never; docker?: never; domainExpiry?: never; group?: never; http?: never; icmp?: never; push?: never;   snmp: SnmpConfigInput; systemd?: never; tailscalePing?: never; tcp?: never; tls?: never; }
  |  { dns?: never; docker?: never; domainExpiry?: never; group?: never; http?: never; icmp?: never; push?: never; snmp?: never;   systemd: SystemdConfigInput; tailscalePing?: never; tcp?: never; tls?: never; }
  |  { dns?: never; docker?: never; domainExpiry?: never; group?: never; http?: never; icmp?: never; push?: never; snmp?: never; systemd?: never;   tailscalePing: TailscalePingConfigInput; tcp?: never; tls?: never; }
  |  { dns?: never; docker?: never; domainExpiry?: never; group?: never; http?: never; icmp?: never; push?: never; snmp?: never; systemd?: never; tailscalePing?: never;   tcp: TcpConfigInput; tls?: never; }
  |  { dns?: never; docker?: never; domainExpiry?: never; group?: never; http?: never; icmp?: never; push?: never; snmp?: never; systemd?: never; tailscalePing?: never; tcp?: never;   tls: TlsConfigInput; };

export type CheckDefaultsInput = {
  degradedThresholdMs: number;
  intervalSeconds: number;
  retries: number;
  retryIntervalSeconds: number;
};

export type CheckNotificationsInput = {
  channelIds: Array<string>;
  /** Send again after each N Down heartbeats, 0..1000; 0: never (NOT-005). */
  resendEveryBeats?: number | null | undefined;
};

/** Identifiers are fixed forever (CHK-009). */
export type CheckType =
  | 'DNS'
  | 'DOCKER'
  | 'DOMAIN_EXPIRY'
  | 'GROUP'
  | 'HTTP'
  | 'ICMP'
  | 'PUSH'
  | 'SNMP'
  | 'SYSTEMD'
  | 'TAILSCALE_PING'
  | 'TCP'
  | 'TLS';

export type ConnectionTestInput = {
  credential?: SshCredentialInput | null | undefined;
  /** Test a saved device, or test unsaved values from the form (DEV-013). */
  deviceId?: string | null | undefined;
  host?: string | null | undefined;
  password?: PasswordCredentialInput | null | undefined;
  protocol?: Protocol | null | undefined;
  rdp?: RdpSettingsInput | null | undefined;
  ssh?: SshSettingsInput | null | undefined;
  vnc?: VncSettingsInput | null | undefined;
};

export type CreateCheckInput = {
  config: CheckConfigInput;
  degradedThresholdMs?: number | null | undefined;
  deviceId: string;
  intervalSeconds?: number | null | undefined;
  invert?: boolean | null | undefined;
  name: string;
  parentId?: string | null | undefined;
  retries?: number | null | undefined;
  retryIntervalSeconds?: number | null | undefined;
};

export type CreateDeviceInput = {
  /** When true (the default), the hub adds one ICMP check (spec 6.2). */
  addDefaultCheck?: boolean | null | undefined;
  host: string;
  kind?: string | null | undefined;
  macAddress?: string | null | undefined;
  metricsSource?: MetricsSource | null | undefined;
  name: string;
  os?: OperatingSystem | null | undefined;
  rdp?: RdpSettingsInput | null | undefined;
  recordSessions?: boolean | null | undefined;
  sessionLimit?: number | null | undefined;
  siteId: string;
  ssh?: SshSettingsInput | null | undefined;
  tags?: Array<string> | null | undefined;
  transfer?: TransferPolicyInput | null | undefined;
  vnc?: VncSettingsInput | null | undefined;
};

export type DeliveryReason =
  | 'BAD_ADDRESS'
  | 'CONNECTION_FAILED'
  | 'HTTP_STATUS'
  | 'NOT_UNDERSTOOD'
  | 'NO_STARTTLS'
  | 'SECRET_UNREADABLE'
  | 'SETUP_FAILED'
  | 'SMTP_REFUSED'
  | 'TARGET_NOT_PERMITTED'
  | 'TIMED_OUT'
  | 'TLS_FAILED';

export type DeviceFilter = {
  /** Case-insensitive substring of the name. */
  name?: string | null | undefined;
  siteId?: string | null | undefined;
  status?: Array<Status> | null | undefined;
  tags?: Array<string> | null | undefined;
};

/**
 * Ties and orders (R-044). Names compare in byte order.
 * NAME: name.
 * STATUS: worst first in severity order DOWN, DEGRADED, UNKNOWN, MAINTENANCE, UP; then name.
 * UPTIME: lowest 24 h uptime first, null last; then name.
 * The client uses the same order when it re-sorts on live changes.
 */
export type DeviceSort =
  | 'NAME'
  | 'STATUS'
  | 'UPTIME';

export type DisplayInput = {
  dpi?: number | null | undefined;
  heightPx: number;
  timezone?: string | null | undefined;
  /** Only these values come from the client (GW-012). */
  widthPx: number;
};

export type DnsConfigInput = {
  expected?: string | null | undefined;
  name: string;
  recordType?: DnsRecordType | null | undefined;
  resolver?: string | null | undefined;
};

export type DnsRecordType =
  | 'A'
  | 'AAAA'
  | 'CNAME'
  | 'MX'
  | 'NS'
  | 'PTR'
  | 'SRV'
  | 'TXT';

export type DockerConfigInput = {
  container: string;
  timeoutMs?: number | null | undefined;
};

export type DomainExpiryConfigInput = {
  domain: string;
  timeoutMs?: number | null | undefined;
  /** 1 to 365 (R-093). */
  warnDays?: number | null | undefined;
};

export type EmailSettingsInput = {
  from: string;
  host: string;
  password?: string | null | undefined;
  port?: number | null | undefined;
  security?: SmtpSecurity | null | undefined;
  to: Array<string>;
  username?: string | null | undefined;
};

export type EventFilter = {
  deviceId?: string | null | undefined;
  severity?: Array<Severity> | null | undefined;
  since?: string | null | undefined;
  source?: Array<EventSource> | null | undefined;
};

export type EventSource =
  | 'ACCESS'
  | 'CHECK'
  | 'DEVICE'
  | 'HUB'
  | 'METRICS'
  | 'NOTIFICATION'
  | 'SESSION'
  | 'VPN';

export type GpuVendor =
  | 'AMD'
  | 'APPLE'
  | 'INTEL'
  | 'NVIDIA';

export type GroupConfigInput = {
  childIds: Array<string>;
};

export type HttpAuthInput =
  {   basic: BasicAuthInput; bearerToken?: never; }
  |  { basic?: never;   bearerToken: string; };

export type HttpAuthKind =
  | 'BASIC'
  | 'BEARER'
  | 'NONE';

export type HttpConfigInput = {
  acceptedStatus?: Array<StatusRangeInput> | null | undefined;
  auth?: HttpAuthInput | null | undefined;
  body?: string | null | undefined;
  headers?: Array<HttpHeaderInput> | null | undefined;
  jsonExpected?: string | null | undefined;
  jsonQuery?: string | null | undefined;
  keyword?: string | null | undefined;
  maxRedirects?: number | null | undefined;
  method?: HttpMethod | null | undefined;
  timeoutMs?: number | null | undefined;
  url: string;
  verifyTls?: boolean | null | undefined;
};

export type HttpHeaderInput = {
  name: string;
  value: string;
};

export type HttpMethod =
  | 'DELETE'
  | 'GET'
  | 'HEAD'
  | 'OPTIONS'
  | 'PATCH'
  | 'POST'
  | 'PUT';

export type IcmpConfigInput = {
  ipVersion?: IpVersion | null | undefined;
  timeoutMs?: number | null | undefined;
};

export type IpVersion =
  | 'V4'
  | 'V6';

export type KeyState =
  | 'EXPIRED'
  | 'EXPIRING'
  | 'NEVER'
  | 'TAGGED_WITH_EXPIRY'
  | 'VALID';

export type LoginInput = {
  password: string;
  totpCode?: string | null | undefined;
  username: string;
};

export type MetricKind =
  | 'CPU_PERCENT'
  | 'DISK_BYTES'
  | 'DISK_PERCENT'
  /** Memory in use by a unified-memory GPU (Apple), shown as a number, never a VRAM bar (MET-007, R-092 D-6). */
  | 'GPU_MEMORY_BYTES'
  | 'GPU_PERCENT'
  | 'GPU_POWER_W'
  | 'GPU_TEMPERATURE_C'
  | 'LOAD_1'
  | 'LOAD_5'
  | 'LOAD_15'
  | 'MEMORY_BYTES'
  | 'MEMORY_PERCENT'
  | 'NETWORK_IN_BPS'
  | 'NETWORK_OUT_BPS'
  | 'UPTIME_SECONDS'
  | 'VRAM_BYTES'
  | 'VRAM_PERCENT';

export type MetricRange =
  | 'DAY'
  | 'HOUR'
  | 'WEEK';

export type MetricsSource =
  /** Set only by enrolment (MET-011, R-092 D-7). */
  | 'AGENT'
  | 'NONE'
  | 'SSH';

export type NotificationChannelInput = {
  name: string;
  onByDefault?: boolean | null | undefined;
  settings: ChannelSettingsInput;
  /** Null: the default template. */
  template?: NotificationTemplateInput | null | undefined;
};

export type NotificationKind =
  | 'CERTIFICATE_EXPIRY'
  | 'DOMAIN_EXPIRY'
  | 'DOWN'
  | 'STILL_DOWN'
  | 'TEST'
  | 'UP';

export type NotificationSettingsInput = {
  /** Days 1..365, in any order; at most 5 once duplicates are removed; stored largest first (NOT-006, D-22). */
  certificateDays: Array<number>;
  template: NotificationTemplateInput;
};

export type NotificationTemplateInput = {
  body: string;
  title: string;
};

export type NtfySettingsInput = {
  priority?: number | null | undefined;
  server?: string | null | undefined;
  token?: string | null | undefined;
  topic?: string | null | undefined;
};

export type OpenIn =
  | 'ANIMA_TAB'
  | 'SYSTEM_BROWSER';

export type OperatingSystem =
  | 'LINUX'
  | 'MACOS'
  | 'OTHER'
  | 'WINDOWS';

export type PasswordCredentialInput = {
  password: string;
};

export type PrivateKeyInput = {
  key: string;
  passphrase?: string | null | undefined;
};

export type Protocol =
  | 'RDP'
  | 'SSH'
  | 'VNC';

export type RdpResizeMethod =
  | 'DISPLAY_UPDATE'
  | 'RECONNECT';

export type RdpSecurity =
  | 'ANY'
  | 'NLA'
  | 'NLA_EXT'
  | 'RDP'
  | 'TLS'
  | 'VMCONNECT';

export type RdpSettingsInput = {
  audioOut?: boolean | null | undefined;
  domain?: string | null | undefined;
  enabled: boolean;
  ignoreCertificate?: boolean | null | undefined;
  microphone?: boolean | null | undefined;
  port?: number | null | undefined;
  resizeMethod?: RdpResizeMethod | null | undefined;
  security?: RdpSecurity | null | undefined;
  sharedDrive?: boolean | null | undefined;
  username: string;
};

export type RetentionSubject =
  | 'DELIVERIES'
  | 'EVENTS'
  | 'HEARTBEATS'
  | 'METRICS'
  | 'RECORDINGS';

export type SessionEndReason =
  | 'AUTH_FAILED'
  | 'GATEWAY_ERROR'
  | 'HOST_KEY_MISMATCH'
  | 'HUB_SHUTDOWN'
  | 'OPERATOR'
  /** The client sent an instruction that does not decode (GW-011, R-034). */
  | 'PROTOCOL_ERROR'
  | 'REMOTE_CLOSED'
  /** The RDP certificate differs from the accepted one (D-5). */
  | 'SERVER_CERTIFICATE_MISMATCH'
  | 'TIMEOUT';

export type Severity =
  | 'ERROR'
  | 'INFO'
  | 'WARNING';

export type SiteRouteState =
  | 'NO_ROUTER'
  | 'OFFLINE'
  | 'SERVING';

export type SmtpSecurity =
  | 'STARTTLS'
  | 'TLS';

export type SnmpConfigInput = {
  authPassword?: string | null | undefined;
  community?: string | null | undefined;
  expected?: string | null | undefined;
  oid: string;
  port?: number | null | undefined;
  privacyPassword?: string | null | undefined;
  timeoutMs?: number | null | undefined;
  username?: string | null | undefined;
  version?: SnmpVersion | null | undefined;
};

export type SnmpVersion =
  | 'V2C'
  | 'V3';

export type SshAuth =
  | 'PASSWORD'
  | 'PRIVATE_KEY';

export type SshCredentialInput =
  {   password: string; privateKey?: never; }
  |  { password?: never;   privateKey: PrivateKeyInput; };

export type SshSettingsInput = {
  auth: SshAuth;
  enabled: boolean;
  port?: number | null | undefined;
  username: string;
};

export type StartSessionInput = {
  deviceId: string;
  display: DisplayInput;
  protocol: Protocol;
};

export type Status =
  | 'DEGRADED'
  | 'DOWN'
  | 'MAINTENANCE'
  | 'UNKNOWN'
  | 'UP';

export type StatusRangeInput = {
  from: number;
  to: number;
};

export type SystemdConfigInput = {
  timeoutMs?: number | null | undefined;
  unit: string;
};

export type TailscalePingConfigInput = {
  target?: string | null | undefined;
  timeoutMs?: number | null | undefined;
};

export type TcpConfigInput = {
  port: number;
  timeoutMs?: number | null | undefined;
};

export type TlsConfigInput = {
  port?: number | null | undefined;
  serverName?: string | null | undefined;
  timeoutMs?: number | null | undefined;
};

export type TokenScope =
  | 'READ'
  | 'WRITE';

export type TransferPolicyInput = {
  copy?: boolean | null | undefined;
  download?: boolean | null | undefined;
  paste?: boolean | null | undefined;
  upload?: boolean | null | undefined;
};

export type UpdateCheckInput = {
  config?: CheckConfigInput | null | undefined;
  degradedThresholdMs?: number | null | undefined;
  intervalSeconds?: number | null | undefined;
  invert?: boolean | null | undefined;
  name?: string | null | undefined;
  parentId?: string | null | undefined;
  retries?: number | null | undefined;
  retryIntervalSeconds?: number | null | undefined;
};

export type UpdateDeviceInput = {
  host?: string | null | undefined;
  kind?: string | null | undefined;
  /** An empty string clears the address. */
  macAddress?: string | null | undefined;
  maintenance?: boolean | null | undefined;
  metricsSource?: MetricsSource | null | undefined;
  name?: string | null | undefined;
  os?: OperatingSystem | null | undefined;
  rdp?: RdpSettingsInput | null | undefined;
  recordSessions?: boolean | null | undefined;
  /** 1..100; 0 clears the limit. */
  sessionLimit?: number | null | undefined;
  siteId?: string | null | undefined;
  ssh?: SshSettingsInput | null | undefined;
  tags?: Array<string> | null | undefined;
  transfer?: TransferPolicyInput | null | undefined;
  vnc?: VncSettingsInput | null | undefined;
};

export type VncSettingsInput = {
  enabled: boolean;
  port?: number | null | undefined;
  username?: string | null | undefined;
};

export type VpnPath =
  | 'DIRECT'
  | 'PEER_RELAY'
  | 'RELAYED'
  | 'UNKNOWN';

export type VpnRole =
  | 'AGENT'
  | 'CLIENT'
  | 'HUB'
  | 'OTHER'
  | 'ROUTER';

export type WebInterfaceInput = {
  /** Defaults to the URL's host. */
  label?: string | null | undefined;
  openIn?: OpenIn | null | undefined;
  url: string;
};

export type WebhookMethod =
  | 'POST'
  | 'PUT';

/**
 * On update, a secret field that is null keeps the stored value; an empty string clears it where it is optional.
 * When the target changes (webhook origin, ntfy server, email host or port, Apprise server), every stored secret must
 * be sent again or cleared, or the update is VALIDATION. `headers`: null keeps, [] removes all, a list replaces all.
 * The kind of a channel never changes. URLs and servers are https only; a server is an origin only (CR-14).
 */
export type WebhookSettingsInput = {
  headers?: Array<HttpHeaderInput> | null | undefined;
  method?: WebhookMethod | null | undefined;
  url?: string | null | undefined;
};

export type LoginMutationVariables = Exact<{
  input: LoginInput;
}>;


export type LoginMutation = { login: { token: string } };

export type SignInBrowserMutationVariables = Exact<{
  input: LoginInput;
}>;


export type SignInBrowserMutation = { signInBrowser: { expiresAt: string } };

export type LogoutMutationVariables = Exact<{ [key: string]: never; }>;


export type LogoutMutation = { logout: boolean };

export type MeQueryVariables = Exact<{ [key: string]: never; }>;


export type MeQuery = { me: { username: string } };

export type ApiKeysQueryVariables = Exact<{ [key: string]: never; }>;


export type ApiKeysQuery = { apiKeys: Array<{ id: string, name: string, scope: TokenScope, prefix: string, expiresAt: string, createdAt: string, lastUsedAt: string | null }> };

export type CreateApiKeyMutationVariables = Exact<{
  input: ApiKeyInput;
}>;


export type CreateApiKeyMutation = { createApiKey: { secret: string, key: { id: string, name: string, scope: TokenScope, prefix: string, expiresAt: string, createdAt: string, lastUsedAt: string | null } } };

export type RevokeApiKeyMutationVariables = Exact<{
  id: string;
}>;


export type RevokeApiKeyMutation = { revokeApiKey: string };

export type BuildersQueryVariables = Exact<{ [key: string]: never; }>;


export type BuildersQuery = { builders: Array<{ id: string, name: string, online: boolean, platform: { os: BuilderOs, arch: BuilderArch } }> };

export type EnrolBuilderMutationVariables = Exact<{
  name: string;
  platform: BuilderPlatformInput;
}>;


export type EnrolBuilderMutation = { enrolBuilder: { token: string, builder: { id: string } } };

export type RevokeBuilderMutationVariables = Exact<{
  id: string;
}>;


export type RevokeBuilderMutation = { revokeBuilder: string };

export type ClaimBuildJobMutationVariables = Exact<{
  platform: BuilderPlatformInput;
}>;


export type ClaimBuildJobMutation = { claimBuildJob: { id: string, target: BuildTarget, version: string, commit: string } | null };

export type ReportBuildJobMutationVariables = Exact<{
  id: string;
  state?: BuildJobState | null | undefined;
  logLines: Array<string> | string;
  failureReason?: string | null | undefined;
}>;


export type ReportBuildJobMutation = { reportBuildJob: { id: string, state: BuildJobState } };

export type BuildUploadUrlsMutationVariables = Exact<{
  id: string;
  files: Array<BuildFileInput> | BuildFileInput;
}>;


export type BuildUploadUrlsMutation = { buildUploadUrls: Array<{ name: string, url: string, expiresAt: string, headers: Array<{ name: string, value: string }> }> };

export type ReportBuildUploadedMutationVariables = Exact<{
  id: string;
  files: Array<BuildFileInput> | BuildFileInput;
  feed: string;
}>;


export type ReportBuildUploadedMutation = { reportBuildUploaded: { id: string, state: BuildJobState } };

export type HeartbeatAddedSubscriptionVariables = Exact<{ [key: string]: never; }>;


export type HeartbeatAddedSubscription = { heartbeatAdded: { checkId: string, deviceId: string, heartbeat: { at: string, status: Status, responseTimeMs: number | null } } };

export type CheckFieldsFragment = { id: string, type: CheckType, name: string, intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number, invert: boolean, paused: boolean, status: Status, certificateDaysLeft: number | null, domainDaysLeft: number | null, parent: { id: string } | null, lastHeartbeat: { at: string, status: Status, responseTimeMs: number | null, message: string | null } | null, config:
    | { __typename: 'DnsConfig', name: string, recordType: DnsRecordType, resolver: string | null, expected: string | null }
    | { __typename: 'DockerConfig' }
    | { __typename: 'DomainExpiryConfig', domain: string, warnDays: number, timeoutMs: number }
    | { __typename: 'GroupConfig', children: Array<{ id: string, name: string }> }
    | { __typename: 'HttpConfig', url: string, method: HttpMethod, body: string | null, maxRedirects: number, keyword: string | null, jsonQuery: string | null, jsonExpected: string | null, authKind: HttpAuthKind, verifyTls: boolean, timeoutMs: number, headers: Array<{ name: string, value: string | null }>, acceptedStatus: Array<{ from: number, to: number }> }
    | { __typename: 'IcmpConfig', timeoutMs: number, ipVersion: IpVersion }
    | { __typename: 'PushConfig', pushPath: string }
    | { __typename: 'SnmpConfig' }
    | { __typename: 'SystemdConfig' }
    | { __typename: 'TailscalePingConfig' }
    | { __typename: 'TcpConfig', port: number, timeoutMs: number }
    | { __typename: 'TlsConfig', port: number, serverName: string | null, timeoutMs: number }
   };

export type DockerCheckConfigFragment = { __typename: 'DockerConfig', container: string, timeoutMs: number };

export type SystemdCheckConfigFragment = { __typename: 'SystemdConfig', unit: string, timeoutMs: number };

export type SnmpCheckConfigFragment = { __typename: 'SnmpConfig', port: number, version: SnmpVersion, oid: string, expected: string | null, username: string | null, timeoutMs: number };

export type TailscalePingCheckConfigFragment = { __typename: 'TailscalePingConfig', target: string | null, timeoutMs: number };

export type CheckDefaultsQueryVariables = Exact<{ [key: string]: never; }>;


export type CheckDefaultsQuery = { checkDefaults: { intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number } };

export type SetCheckDefaultsMutationVariables = Exact<{
  input: CheckDefaultsInput;
}>;


export type SetCheckDefaultsMutation = { setCheckDefaults: { intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number } };

export type DeviceChecksQueryVariables = Exact<{
  deviceId: string;
}>;


export type DeviceChecksQuery = { device: { id: string, checks: Array<{ id: string, type: CheckType, name: string, intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number, invert: boolean, paused: boolean, status: Status, certificateDaysLeft: number | null, domainDaysLeft: number | null, parent: { id: string } | null, lastHeartbeat: { at: string, status: Status, responseTimeMs: number | null, message: string | null } | null, config:
        | { __typename: 'DnsConfig', name: string, recordType: DnsRecordType, resolver: string | null, expected: string | null }
        | { __typename: 'DockerConfig' }
        | { __typename: 'DomainExpiryConfig', domain: string, warnDays: number, timeoutMs: number }
        | { __typename: 'GroupConfig', children: Array<{ id: string, name: string }> }
        | { __typename: 'HttpConfig', url: string, method: HttpMethod, body: string | null, maxRedirects: number, keyword: string | null, jsonQuery: string | null, jsonExpected: string | null, authKind: HttpAuthKind, verifyTls: boolean, timeoutMs: number, headers: Array<{ name: string, value: string | null }>, acceptedStatus: Array<{ from: number, to: number }> }
        | { __typename: 'IcmpConfig', timeoutMs: number, ipVersion: IpVersion }
        | { __typename: 'PushConfig', pushPath: string }
        | { __typename: 'SnmpConfig' }
        | { __typename: 'SystemdConfig' }
        | { __typename: 'TailscalePingConfig' }
        | { __typename: 'TcpConfig', port: number, timeoutMs: number }
        | { __typename: 'TlsConfig', port: number, serverName: string | null, timeoutMs: number }
       }> } | null };

export type CheckHeartbeatsQueryVariables = Exact<{
  id: string;
  first?: number | null | undefined;
  after?: string | null | undefined;
}>;


export type CheckHeartbeatsQuery = { check: { id: string, heartbeats: { pageInfo: { hasNextPage: boolean, endCursor: string | null }, edges: Array<{ node: { at: string, status: Status, responseTimeMs: number | null, message: string | null } }> } } | null };

export type CreateCheckMutationVariables = Exact<{
  input: CreateCheckInput;
}>;


export type CreateCheckMutation = { createCheck: { id: string, type: CheckType, name: string, intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number, invert: boolean, paused: boolean, status: Status, certificateDaysLeft: number | null, domainDaysLeft: number | null, parent: { id: string } | null, lastHeartbeat: { at: string, status: Status, responseTimeMs: number | null, message: string | null } | null, config:
      | { __typename: 'DnsConfig', name: string, recordType: DnsRecordType, resolver: string | null, expected: string | null }
      | { __typename: 'DockerConfig' }
      | { __typename: 'DomainExpiryConfig', domain: string, warnDays: number, timeoutMs: number }
      | { __typename: 'GroupConfig', children: Array<{ id: string, name: string }> }
      | { __typename: 'HttpConfig', url: string, method: HttpMethod, body: string | null, maxRedirects: number, keyword: string | null, jsonQuery: string | null, jsonExpected: string | null, authKind: HttpAuthKind, verifyTls: boolean, timeoutMs: number, headers: Array<{ name: string, value: string | null }>, acceptedStatus: Array<{ from: number, to: number }> }
      | { __typename: 'IcmpConfig', timeoutMs: number, ipVersion: IpVersion }
      | { __typename: 'PushConfig', pushPath: string }
      | { __typename: 'SnmpConfig' }
      | { __typename: 'SystemdConfig' }
      | { __typename: 'TailscalePingConfig' }
      | { __typename: 'TcpConfig', port: number, timeoutMs: number }
      | { __typename: 'TlsConfig', port: number, serverName: string | null, timeoutMs: number }
     } };

export type UpdateCheckMutationVariables = Exact<{
  id: string;
  input: UpdateCheckInput;
}>;


export type UpdateCheckMutation = { updateCheck: { id: string, type: CheckType, name: string, intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number, invert: boolean, paused: boolean, status: Status, certificateDaysLeft: number | null, domainDaysLeft: number | null, parent: { id: string } | null, lastHeartbeat: { at: string, status: Status, responseTimeMs: number | null, message: string | null } | null, config:
      | { __typename: 'DnsConfig', name: string, recordType: DnsRecordType, resolver: string | null, expected: string | null }
      | { __typename: 'DockerConfig' }
      | { __typename: 'DomainExpiryConfig', domain: string, warnDays: number, timeoutMs: number }
      | { __typename: 'GroupConfig', children: Array<{ id: string, name: string }> }
      | { __typename: 'HttpConfig', url: string, method: HttpMethod, body: string | null, maxRedirects: number, keyword: string | null, jsonQuery: string | null, jsonExpected: string | null, authKind: HttpAuthKind, verifyTls: boolean, timeoutMs: number, headers: Array<{ name: string, value: string | null }>, acceptedStatus: Array<{ from: number, to: number }> }
      | { __typename: 'IcmpConfig', timeoutMs: number, ipVersion: IpVersion }
      | { __typename: 'PushConfig', pushPath: string }
      | { __typename: 'SnmpConfig' }
      | { __typename: 'SystemdConfig' }
      | { __typename: 'TailscalePingConfig' }
      | { __typename: 'TcpConfig', port: number, timeoutMs: number }
      | { __typename: 'TlsConfig', port: number, serverName: string | null, timeoutMs: number }
     } };

export type DeleteCheckMutationVariables = Exact<{
  id: string;
}>;


export type DeleteCheckMutation = { deleteCheck: string };

export type SetCheckPausedMutationVariables = Exact<{
  id: string;
  paused: boolean;
}>;


export type SetCheckPausedMutation = { setCheckPaused: { id: string, type: CheckType, name: string, intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number, invert: boolean, paused: boolean, status: Status, certificateDaysLeft: number | null, domainDaysLeft: number | null, parent: { id: string } | null, lastHeartbeat: { at: string, status: Status, responseTimeMs: number | null, message: string | null } | null, config:
      | { __typename: 'DnsConfig', name: string, recordType: DnsRecordType, resolver: string | null, expected: string | null }
      | { __typename: 'DockerConfig' }
      | { __typename: 'DomainExpiryConfig', domain: string, warnDays: number, timeoutMs: number }
      | { __typename: 'GroupConfig', children: Array<{ id: string, name: string }> }
      | { __typename: 'HttpConfig', url: string, method: HttpMethod, body: string | null, maxRedirects: number, keyword: string | null, jsonQuery: string | null, jsonExpected: string | null, authKind: HttpAuthKind, verifyTls: boolean, timeoutMs: number, headers: Array<{ name: string, value: string | null }>, acceptedStatus: Array<{ from: number, to: number }> }
      | { __typename: 'IcmpConfig', timeoutMs: number, ipVersion: IpVersion }
      | { __typename: 'PushConfig', pushPath: string }
      | { __typename: 'SnmpConfig' }
      | { __typename: 'SystemdConfig' }
      | { __typename: 'TailscalePingConfig' }
      | { __typename: 'TcpConfig', port: number, timeoutMs: number }
      | { __typename: 'TlsConfig', port: number, serverName: string | null, timeoutMs: number }
     } };

export type CheckNotificationsQueryVariables = Exact<{
  id: string;
}>;


export type CheckNotificationsQuery = { check: { notifications: { resendEveryBeats: number, channels: Array<{ id: string }> } } | null };

export type SetCheckNotificationsMutationVariables = Exact<{
  checkId: string;
  input: CheckNotificationsInput;
}>;


export type SetCheckNotificationsMutation = { setCheckNotifications: { resendEveryBeats: number, channels: Array<{ id: string }> } };

export type DevicesQueryVariables = Exact<{
  filter?: DeviceFilter | null | undefined;
  sort?: DeviceSort | null | undefined;
  first?: number | null | undefined;
  after?: string | null | undefined;
}>;


export type DevicesQuery = { devices: { totalCount: number, pageInfo: { hasNextPage: boolean, endCursor: string | null }, edges: Array<{ node: { id: string, name: string, host: string, ip: string | null, tags: Array<string>, maintenance: boolean, status: Status, uptime: number | null, os: OperatingSystem, distribution: string | null, macAddress: string | null, metricsSource: MetricsSource, accessWarnings: Array<AccessWarning>, readings: { at: string, cpuPercent: number | null, memoryPercent: number | null, diskPercent: number | null, vramPercent: number | null, vramGpu: { index: number, name: string, vendor: GpuVendor, hasVram: boolean } | null } | null, webInterfaces: Array<{ id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null }>, rdp: { enabled: boolean }, vnc: { enabled: boolean, username: string | null }, ssh: { enabled: boolean, hostKey: { fingerprintSha256: string } | null }, checks: Array<{ id: string, heartbeats: { edges: Array<{ node: { at: string, status: Status, responseTimeMs: number | null } }> } }> } }> } };

export type DeviceHostsQueryVariables = Exact<{
  after?: string | null | undefined;
}>;


export type DeviceHostsQuery = { devices: { pageInfo: { hasNextPage: boolean, endCursor: string | null }, edges: Array<{ node: { host: string } }> } };

export type TagsQueryVariables = Exact<{ [key: string]: never; }>;


export type TagsQuery = { tags: Array<string> };

export type DeviceStatusChangedSubscriptionVariables = Exact<{ [key: string]: never; }>;


export type DeviceStatusChangedSubscription = { deviceStatusChanged: { deviceId: string, status: Status, at: string } };

export type CardReadingsAddedSubscriptionVariables = Exact<{ [key: string]: never; }>;


export type CardReadingsAddedSubscription = { metricsAdded: { deviceId: string, readings: { at: string, cpuPercent: number | null, memoryPercent: number | null, diskPercent: number | null, vramPercent: number | null, vramGpu: { index: number, name: string, vendor: GpuVendor, hasVram: boolean } | null } } };

export type DeviceDetailFieldsFragment = { id: string, name: string, host: string, ip: string | null, kind: string, os: OperatingSystem, distribution: string | null, tags: Array<string>, maintenance: boolean, status: Status, macAddress: string | null, metricsSource: MetricsSource, accessWarnings: Array<AccessWarning>, day: number | null, week: number | null, month: number | null, site: { id: string, name: string }, ssh: { enabled: boolean, port: number, username: string, auth: SshAuth, credentialStored: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null }, rdp: { enabled: boolean, port: number, username: string, domain: string | null, security: RdpSecurity, ignoreCertificate: boolean, resizeMethod: RdpResizeMethod, audioOut: boolean, microphone: boolean, sharedDrive: boolean, credentialStored: boolean, serverCertificate: { fingerprintSha256: string, subject: string | null, notAfter: string | null, firstSeenAt: string } | null }, vnc: { enabled: boolean, port: number, username: string | null, credentialStored: boolean }, transfer: { upload: boolean, download: boolean, copy: boolean, paste: boolean }, webInterfaces: Array<{ id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null }>, webInterfaceSuggestions: Array<{ checkId: string, url: string, label: string }> };

export type CardReadingsFragment = { at: string, cpuPercent: number | null, memoryPercent: number | null, diskPercent: number | null, vramPercent: number | null, vramGpu: { index: number, name: string, vendor: GpuVendor, hasVram: boolean } | null };

export type DeviceQueryVariables = Exact<{
  id: string;
}>;


export type DeviceQuery = { device: { id: string, name: string, host: string, ip: string | null, kind: string, os: OperatingSystem, distribution: string | null, tags: Array<string>, maintenance: boolean, status: Status, macAddress: string | null, metricsSource: MetricsSource, accessWarnings: Array<AccessWarning>, day: number | null, week: number | null, month: number | null, site: { id: string, name: string }, ssh: { enabled: boolean, port: number, username: string, auth: SshAuth, credentialStored: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null }, rdp: { enabled: boolean, port: number, username: string, domain: string | null, security: RdpSecurity, ignoreCertificate: boolean, resizeMethod: RdpResizeMethod, audioOut: boolean, microphone: boolean, sharedDrive: boolean, credentialStored: boolean, serverCertificate: { fingerprintSha256: string, subject: string | null, notAfter: string | null, firstSeenAt: string } | null }, vnc: { enabled: boolean, port: number, username: string | null, credentialStored: boolean }, transfer: { upload: boolean, download: boolean, copy: boolean, paste: boolean }, webInterfaces: Array<{ id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null }>, webInterfaceSuggestions: Array<{ checkId: string, url: string, label: string }> } | null };

export type SitesQueryVariables = Exact<{ [key: string]: never; }>;


export type SitesQuery = { sites: Array<{ id: string, name: string, subnet: string }> };

export type CreateSiteMutationVariables = Exact<{
  name: string;
  subnet: string;
}>;


export type CreateSiteMutation = { createSite: { id: string, name: string, subnet: string } };

export type CreateDeviceMutationVariables = Exact<{
  input: CreateDeviceInput;
}>;


export type CreateDeviceMutation = { createDevice: { id: string, name: string, host: string, ip: string | null, kind: string, os: OperatingSystem, distribution: string | null, tags: Array<string>, maintenance: boolean, status: Status, macAddress: string | null, metricsSource: MetricsSource, accessWarnings: Array<AccessWarning>, day: number | null, week: number | null, month: number | null, site: { id: string, name: string }, ssh: { enabled: boolean, port: number, username: string, auth: SshAuth, credentialStored: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null }, rdp: { enabled: boolean, port: number, username: string, domain: string | null, security: RdpSecurity, ignoreCertificate: boolean, resizeMethod: RdpResizeMethod, audioOut: boolean, microphone: boolean, sharedDrive: boolean, credentialStored: boolean, serverCertificate: { fingerprintSha256: string, subject: string | null, notAfter: string | null, firstSeenAt: string } | null }, vnc: { enabled: boolean, port: number, username: string | null, credentialStored: boolean }, transfer: { upload: boolean, download: boolean, copy: boolean, paste: boolean }, webInterfaces: Array<{ id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null }>, webInterfaceSuggestions: Array<{ checkId: string, url: string, label: string }> } };

export type UpdateDeviceMutationVariables = Exact<{
  id: string;
  input: UpdateDeviceInput;
}>;


export type UpdateDeviceMutation = { updateDevice: { id: string, name: string, host: string, ip: string | null, kind: string, os: OperatingSystem, distribution: string | null, tags: Array<string>, maintenance: boolean, status: Status, macAddress: string | null, metricsSource: MetricsSource, accessWarnings: Array<AccessWarning>, day: number | null, week: number | null, month: number | null, site: { id: string, name: string }, ssh: { enabled: boolean, port: number, username: string, auth: SshAuth, credentialStored: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null }, rdp: { enabled: boolean, port: number, username: string, domain: string | null, security: RdpSecurity, ignoreCertificate: boolean, resizeMethod: RdpResizeMethod, audioOut: boolean, microphone: boolean, sharedDrive: boolean, credentialStored: boolean, serverCertificate: { fingerprintSha256: string, subject: string | null, notAfter: string | null, firstSeenAt: string } | null }, vnc: { enabled: boolean, port: number, username: string | null, credentialStored: boolean }, transfer: { upload: boolean, download: boolean, copy: boolean, paste: boolean }, webInterfaces: Array<{ id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null }>, webInterfaceSuggestions: Array<{ checkId: string, url: string, label: string }> } };

export type DeleteDeviceMutationVariables = Exact<{
  id: string;
}>;


export type DeleteDeviceMutation = { deleteDevice: string };

export type SetSshCredentialMutationVariables = Exact<{
  deviceId: string;
  credential: SshCredentialInput;
}>;


export type SetSshCredentialMutation = { setSshCredential: { credentialStored: boolean } };

export type ClearSshCredentialMutationVariables = Exact<{
  deviceId: string;
}>;


export type ClearSshCredentialMutation = { clearSshCredential: { credentialStored: boolean } };

export type ClearRdpCredentialMutationVariables = Exact<{
  deviceId: string;
}>;


export type ClearRdpCredentialMutation = { clearRdpCredential: { credentialStored: boolean } };

export type ClearVncCredentialMutationVariables = Exact<{
  deviceId: string;
}>;


export type ClearVncCredentialMutation = { clearVncCredential: { credentialStored: boolean } };

export type DevicePolicyQueryVariables = Exact<{
  id: string;
}>;


export type DevicePolicyQuery = { device: { sessionLimit: number | null, recordSessions: boolean } | null };

export type SetDevicePolicyMutationVariables = Exact<{
  id: string;
  input: UpdateDeviceInput;
}>;


export type SetDevicePolicyMutation = { updateDevice: { sessionLimit: number | null, recordSessions: boolean } };

export type EventsQueryVariables = Exact<{
  filter?: EventFilter | null | undefined;
  first?: number | null | undefined;
  after?: string | null | undefined;
}>;


export type EventsQuery = { events: { pageInfo: { hasNextPage: boolean, endCursor: string | null }, edges: Array<{ node: { id: string, at: string, severity: Severity, source: EventSource, deviceId: string | null, deviceName: string | null, message: string } }> } };

export type EventAddedSubscriptionVariables = Exact<{
  minSeverity?: Severity | null | undefined;
}>;


export type EventAddedSubscription = { eventAdded: { id: string, at: string, severity: Severity, source: EventSource, deviceId: string | null, deviceName: string | null, message: string } };

export type DeviceMetricsQueryVariables = Exact<{
  id: string;
  range: MetricRange;
}>;


export type DeviceMetricsQuery = { device: { id: string, metricsSource: MetricsSource, gpus: Array<{ index: number, name: string, vendor: GpuVendor, hasVram: boolean }>, readings: { at: string, cpuPercent: number | null, memoryPercent: number | null, diskPercent: number | null, vramPercent: number | null, vramGpu: { index: number, name: string, vendor: GpuVendor, hasVram: boolean } | null } | null, metrics: Array<{ kind: MetricKind, label: string, gpuIndex: number | null, points: Array<{ at: string, value: number, min: number | null, max: number | null }> }> } | null };

export type MetricsAddedSubscriptionVariables = Exact<{
  deviceId: string;
}>;


export type MetricsAddedSubscription = { metricsAdded: { deviceId: string, at: string, samples: Array<{ kind: MetricKind, label: string, value: number }>, readings: { at: string, cpuPercent: number | null, memoryPercent: number | null, diskPercent: number | null, vramPercent: number | null, vramGpu: { index: number, name: string, vendor: GpuVendor, hasVram: boolean } | null } } };

export type AgentStatusQueryVariables = Exact<{
  id: string;
}>;


export type AgentStatusQuery = { device: { id: string, metricsSource: MetricsSource, metricsStale: boolean } | null };

export type EnrolAgentMutationVariables = Exact<{
  deviceId: string;
}>;


export type EnrolAgentMutation = { enrolAgent: { code: string, expiresAt: string } };

export type RevokeAgentTokenMutationVariables = Exact<{
  deviceId: string;
}>;


export type RevokeAgentTokenMutation = { revokeAgentToken: { id: string, metricsSource: MetricsSource, metricsStale: boolean } };

export type DeliveryFieldsFragment = { id: string, channelId: string | null, kind: NotificationKind, at: string, ok: boolean, reason: DeliveryReason | null, code: number | null };

export type NotificationChannelFieldsFragment = { id: string, name: string, kind: ChannelKind, onByDefault: boolean, settings:
    | { __typename: 'AppriseSettings', server: string, mode: AppriseMode, tags: Array<string> }
    | { __typename: 'EmailSettings', host: string, port: number, security: SmtpSecurity, username: string | null, from: string, to: Array<string>, passwordStored: boolean }
    | { __typename: 'NtfySettings', server: string, priority: number, tokenStored: boolean }
    | { __typename: 'WebhookSettings', origin: string, method: WebhookMethod, headerNames: Array<string> }
  , template: { title: string, body: string } | null, lastDelivery: { id: string, channelId: string | null, kind: NotificationKind, at: string, ok: boolean, reason: DeliveryReason | null, code: number | null } | null };

export type NotificationSettingsFieldsFragment = { certificateDays: Array<number>, placeholders: Array<string>, template: { title: string, body: string } };

export type NotificationChannelsQueryVariables = Exact<{ [key: string]: never; }>;


export type NotificationChannelsQuery = { notificationChannels: Array<{ id: string, name: string, kind: ChannelKind, onByDefault: boolean, settings:
      | { __typename: 'AppriseSettings', server: string, mode: AppriseMode, tags: Array<string> }
      | { __typename: 'EmailSettings', host: string, port: number, security: SmtpSecurity, username: string | null, from: string, to: Array<string>, passwordStored: boolean }
      | { __typename: 'NtfySettings', server: string, priority: number, tokenStored: boolean }
      | { __typename: 'WebhookSettings', origin: string, method: WebhookMethod, headerNames: Array<string> }
    , template: { title: string, body: string } | null, lastDelivery: { id: string, channelId: string | null, kind: NotificationKind, at: string, ok: boolean, reason: DeliveryReason | null, code: number | null } | null }> };

export type NotificationSettingsQueryVariables = Exact<{ [key: string]: never; }>;


export type NotificationSettingsQuery = { notificationSettings: { certificateDays: Array<number>, placeholders: Array<string>, template: { title: string, body: string } } };

export type CreateNotificationChannelMutationVariables = Exact<{
  input: NotificationChannelInput;
}>;


export type CreateNotificationChannelMutation = { createNotificationChannel: { id: string, name: string, kind: ChannelKind, onByDefault: boolean, settings:
      | { __typename: 'AppriseSettings', server: string, mode: AppriseMode, tags: Array<string> }
      | { __typename: 'EmailSettings', host: string, port: number, security: SmtpSecurity, username: string | null, from: string, to: Array<string>, passwordStored: boolean }
      | { __typename: 'NtfySettings', server: string, priority: number, tokenStored: boolean }
      | { __typename: 'WebhookSettings', origin: string, method: WebhookMethod, headerNames: Array<string> }
    , template: { title: string, body: string } | null, lastDelivery: { id: string, channelId: string | null, kind: NotificationKind, at: string, ok: boolean, reason: DeliveryReason | null, code: number | null } | null } };

export type UpdateNotificationChannelMutationVariables = Exact<{
  id: string;
  input: NotificationChannelInput;
}>;


export type UpdateNotificationChannelMutation = { updateNotificationChannel: { id: string, name: string, kind: ChannelKind, onByDefault: boolean, settings:
      | { __typename: 'AppriseSettings', server: string, mode: AppriseMode, tags: Array<string> }
      | { __typename: 'EmailSettings', host: string, port: number, security: SmtpSecurity, username: string | null, from: string, to: Array<string>, passwordStored: boolean }
      | { __typename: 'NtfySettings', server: string, priority: number, tokenStored: boolean }
      | { __typename: 'WebhookSettings', origin: string, method: WebhookMethod, headerNames: Array<string> }
    , template: { title: string, body: string } | null, lastDelivery: { id: string, channelId: string | null, kind: NotificationKind, at: string, ok: boolean, reason: DeliveryReason | null, code: number | null } | null } };

export type DeleteNotificationChannelMutationVariables = Exact<{
  id: string;
}>;


export type DeleteNotificationChannelMutation = { deleteNotificationChannel: string };

export type SendTestNotificationMutationVariables = Exact<{
  id: string;
}>;


export type SendTestNotificationMutation = { sendTestNotification: { id: string, channelId: string | null, kind: NotificationKind, at: string, ok: boolean, reason: DeliveryReason | null, code: number | null } };

export type SetNotificationSettingsMutationVariables = Exact<{
  input: NotificationSettingsInput;
}>;


export type SetNotificationSettingsMutation = { setNotificationSettings: { certificateDays: Array<number>, placeholders: Array<string>, template: { title: string, body: string } } };

export type RetentionQueryVariables = Exact<{ [key: string]: never; }>;


export type RetentionQuery = { retention: Array<{ subject: RetentionSubject, days: number, minDays: number, maxDays: number, defaultDays: number }> };

export type SetRetentionMutationVariables = Exact<{
  subject: RetentionSubject;
  days: number;
}>;


export type SetRetentionMutation = { setRetention: { subject: RetentionSubject, days: number, minDays: number, maxDays: number, defaultDays: number } };

export type StartSessionMutationVariables = Exact<{
  input: StartSessionInput;
}>;


export type StartSessionMutation = { startSession: { sessionId: string, token: string, tunnelPath: string, expiresAt: string, wakeSent: boolean } };

export type EndSessionMutationVariables = Exact<{
  id: string;
}>;


export type EndSessionMutation = { endSession: { id: string, deviceId: string | null, deviceName: string, protocol: Protocol, startedAt: string, endedAt: string | null, endReason: SessionEndReason | null } };

export type TestConnectionMutationVariables = Exact<{
  input: ConnectionTestInput;
}>;


export type TestConnectionMutation = { testConnection: { reachable: boolean, authenticated: boolean, hostKeyChanged: boolean, message: string | null, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null } };

export type AcceptHostKeyMutationVariables = Exact<{
  deviceId: string;
  fingerprintSha256: string;
}>;


export type AcceptHostKeyMutation = { acceptHostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } };

export type TestRemoteConnectionMutationVariables = Exact<{
  input: ConnectionTestInput;
}>;


export type TestRemoteConnectionMutation = { testConnection: { reachable: boolean, authenticated: boolean, hostKeyChanged: boolean, message: string | null, certificateChanged: boolean, certificateMatched: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null, serverCertificate: { fingerprintSha256: string, subject: string | null, notAfter: string | null, firstSeenAt: string } | null } };

export type AcceptServerCertificateMutationVariables = Exact<{
  deviceId: string;
  fingerprintSha256: string;
}>;


export type AcceptServerCertificateMutation = { acceptServerCertificate: { fingerprintSha256: string, subject: string | null, notAfter: string | null, firstSeenAt: string } };

export type DeviceTransferQueryVariables = Exact<{
  id: string;
}>;


export type DeviceTransferQuery = { device: { transfer: { copy: boolean, paste: boolean } } | null };

export type SetRdpCredentialMutationVariables = Exact<{
  deviceId: string;
  credential: PasswordCredentialInput;
}>;


export type SetRdpCredentialMutation = { setRdpCredential: { credentialStored: boolean } };

export type SetVncCredentialMutationVariables = Exact<{
  deviceId: string;
  credential: PasswordCredentialInput;
}>;


export type SetVncCredentialMutation = { setVncCredential: { credentialStored: boolean } };

export type DeviceSessionsQueryVariables = Exact<{
  deviceId: string;
  first?: number | null | undefined;
  after?: string | null | undefined;
}>;


export type DeviceSessionsQuery = { sessions: { pageInfo: { hasNextPage: boolean, endCursor: string | null }, edges: Array<{ node: { id: string, protocol: Protocol, operatorName: string, startedAt: string, endedAt: string | null, endReason: SessionEndReason | null } }> } };

export type OpenFilesMutationVariables = Exact<{
  sessionId: string;
}>;


export type OpenFilesMutation = { openFiles: { sessionId: string, token: string, tunnelPath: string, expiresAt: string } };

export type LiveSessionsQueryVariables = Exact<{
  first?: number | null | undefined;
}>;


export type LiveSessionsQuery = { sessions: { edges: Array<{ node: { id: string, deviceId: string | null, deviceName: string, protocol: Protocol, operatorName: string, startedAt: string, clientAddr: string } }> } };

export type LiveSessionChangedSubscriptionVariables = Exact<{ [key: string]: never; }>;


export type LiveSessionChangedSubscription = { sessionChanged: { session: { id: string, deviceId: string | null, deviceName: string, protocol: Protocol, operatorName: string, startedAt: string, clientAddr: string, endedAt: string | null, endReason: SessionEndReason | null } } };

export type WakeDeviceMutationVariables = Exact<{
  deviceId: string;
}>;


export type WakeDeviceMutation = { wakeDevice: { sentAt: string, relayName: string } };

export type VpnNodeFieldsFragment = { name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null };

export type VpnPictureFieldsFragment = { readAt: string, fresh: boolean, broker: BrokerState, hub: { name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null }, sites: Array<{ state: SiteRouteState, path: VpnPath, relayRegion: string | null, hasPingCheck: boolean, site: { id: string, name: string, subnet: string }, router: { name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null } | null, waitingRouter: { name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null } | null }>, nodes: Array<{ name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null }> };

export type VpnQueryVariables = Exact<{ [key: string]: never; }>;


export type VpnQuery = { vpn: { readAt: string, fresh: boolean, broker: BrokerState, hub: { name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null }, sites: Array<{ state: SiteRouteState, path: VpnPath, relayRegion: string | null, hasPingCheck: boolean, site: { id: string, name: string, subnet: string }, router: { name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null } | null, waitingRouter: { name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null } | null }>, nodes: Array<{ name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null }> } | null };

export type VpnChangedSubscriptionVariables = Exact<{ [key: string]: never; }>;


export type VpnChangedSubscription = { vpnChanged: { readAt: string, fresh: boolean, broker: BrokerState, hub: { name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null }, sites: Array<{ state: SiteRouteState, path: VpnPath, relayRegion: string | null, hasPingCheck: boolean, site: { id: string, name: string, subnet: string }, router: { name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null } | null, waitingRouter: { name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null } | null }>, nodes: Array<{ name: string, address: string, os: string, role: VpnRole, tags: Array<string>, routes: Array<string>, waitingRoutes: Array<string>, online: boolean, lastSeen: string | null, keyExpiry: string | null, keyState: KeyState, path: VpnPath, relayRegion: string | null, inAnima: boolean, device: { id: string, name: string } | null }> } };

export type CreateVpnInviteMutationVariables = Exact<{ [key: string]: never; }>;


export type CreateVpnInviteMutation = { createVpnInvite: { key: string, expiresAt: string, steps: Array<string> } };

export type CreateRouterKeyMutationVariables = Exact<{
  siteId: string;
}>;


export type CreateRouterKeyMutation = { createRouterKey: { key: string, expiresAt: string, steps: Array<string> } };

export type RegisterVpnNodeMutationVariables = Exact<{
  authId: string;
}>;


export type RegisterVpnNodeMutation = { registerVpnNode: { name: string, address: string, keyExpiry: string | null } };

export type RenewVpnKeyMutationVariables = Exact<{
  name: string;
}>;


export type RenewVpnKeyMutation = { renewVpnKey: { name: string, address: string, keyExpiry: string | null } };

export type ApproveSiteRouteMutationVariables = Exact<{
  siteId: string;
}>;


export type ApproveSiteRouteMutation = { approveSiteRoute: boolean };

export type WebInterfaceSuggestionFieldsFragment = { checkId: string, url: string, label: string };

export type WebInterfaceFieldsFragment = { id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null };

export type CreateWebInterfaceMutationVariables = Exact<{
  deviceId: string;
  input: WebInterfaceInput;
}>;


export type CreateWebInterfaceMutation = { createWebInterface: { id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null } };

export type UpdateWebInterfaceMutationVariables = Exact<{
  id: string;
  input: WebInterfaceInput;
}>;


export type UpdateWebInterfaceMutation = { updateWebInterface: { id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null } };

export type DeleteWebInterfaceMutationVariables = Exact<{
  id: string;
}>;


export type DeleteWebInterfaceMutation = { deleteWebInterface: string };

export type AcceptWebInterfaceSuggestionMutationVariables = Exact<{
  checkId: string;
}>;


export type AcceptWebInterfaceSuggestionMutation = { acceptWebInterfaceSuggestion: { id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null } };

export type DismissWebInterfaceSuggestionMutationVariables = Exact<{
  checkId: string;
}>;


export type DismissWebInterfaceSuggestionMutation = { dismissWebInterfaceSuggestion: string };

export type AcceptWebCertificateMutationVariables = Exact<{
  id: string;
  fingerprintSha256: string;
}>;


export type AcceptWebCertificateMutation = { acceptWebCertificate: { id: string, pinnedFingerprintSha256: string | null, pinnedAt: string | null } };

export const CheckFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CheckFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Check"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"type"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}},{"kind":"Field","name":{"kind":"Name","value":"invert"}},{"kind":"Field","name":{"kind":"Name","value":"paused"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"certificateDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"domainDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"lastHeartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}},{"kind":"Field","name":{"kind":"Name","value":"config"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"IcmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}},{"kind":"Field","name":{"kind":"Name","value":"ipVersion"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TcpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TlsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"serverName"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"HttpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headers"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"body"}},{"kind":"Field","name":{"kind":"Name","value":"maxRedirects"}},{"kind":"Field","name":{"kind":"Name","value":"acceptedStatus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}}]}},{"kind":"Field","name":{"kind":"Name","value":"keyword"}},{"kind":"Field","name":{"kind":"Name","value":"jsonQuery"}},{"kind":"Field","name":{"kind":"Name","value":"jsonExpected"}},{"kind":"Field","name":{"kind":"Name","value":"authKind"}},{"kind":"Field","name":{"kind":"Name","value":"verifyTls"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DnsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"recordType"}},{"kind":"Field","name":{"kind":"Name","value":"resolver"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"PushConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pushPath"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"GroupConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"children"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DomainExpiryConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"warnDays"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]}}]}}]} as unknown as DocumentNode<CheckFieldsFragment, unknown>;
export const DockerCheckConfigFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DockerCheckConfig"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DockerConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"Field","name":{"kind":"Name","value":"container"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]} as unknown as DocumentNode<DockerCheckConfigFragment, unknown>;
export const SystemdCheckConfigFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"SystemdCheckConfig"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"SystemdConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"Field","name":{"kind":"Name","value":"unit"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]} as unknown as DocumentNode<SystemdCheckConfigFragment, unknown>;
export const SnmpCheckConfigFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"SnmpCheckConfig"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"SnmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"version"}},{"kind":"Field","name":{"kind":"Name","value":"oid"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]} as unknown as DocumentNode<SnmpCheckConfigFragment, unknown>;
export const TailscalePingCheckConfigFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"TailscalePingCheckConfig"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TailscalePingConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"Field","name":{"kind":"Name","value":"target"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]} as unknown as DocumentNode<TailscalePingCheckConfigFragment, unknown>;
export const WebInterfaceFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}}]} as unknown as DocumentNode<WebInterfaceFieldsFragment, unknown>;
export const WebInterfaceSuggestionFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceSuggestion"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkId"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"label"}}]}}]} as unknown as DocumentNode<WebInterfaceSuggestionFieldsFragment, unknown>;
export const DeviceDetailFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeviceDetailFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Device"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"distribution"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"auth"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"macAddress"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"accessWarnings"}},{"kind":"Field","name":{"kind":"Name","value":"rdp"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"security"}},{"kind":"Field","name":{"kind":"Name","value":"ignoreCertificate"}},{"kind":"Field","name":{"kind":"Name","value":"resizeMethod"}},{"kind":"Field","name":{"kind":"Name","value":"audioOut"}},{"kind":"Field","name":{"kind":"Name","value":"microphone"}},{"kind":"Field","name":{"kind":"Name","value":"sharedDrive"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"serverCertificate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"subject"}},{"kind":"Field","name":{"kind":"Name","value":"notAfter"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"vnc"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}},{"kind":"Field","name":{"kind":"Name","value":"transfer"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"upload"}},{"kind":"Field","name":{"kind":"Name","value":"download"}},{"kind":"Field","name":{"kind":"Name","value":"copy"}},{"kind":"Field","name":{"kind":"Name","value":"paste"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaceSuggestions"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"}}]}},{"kind":"Field","alias":{"kind":"Name","value":"day"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","alias":{"kind":"Name","value":"week"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"WEEK"}}]},{"kind":"Field","alias":{"kind":"Name","value":"month"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"MONTH"}}]}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceSuggestion"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkId"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"label"}}]}}]} as unknown as DocumentNode<DeviceDetailFieldsFragment, unknown>;
export const CardReadingsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CardReadings"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Readings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"cpuPercent"}},{"kind":"Field","name":{"kind":"Name","value":"memoryPercent"}},{"kind":"Field","name":{"kind":"Name","value":"diskPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramGpu"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"index"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"vendor"}},{"kind":"Field","name":{"kind":"Name","value":"hasVram"}}]}}]}}]} as unknown as DocumentNode<CardReadingsFragment, unknown>;
export const DeliveryFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeliveryFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Delivery"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"channelId"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"ok"}},{"kind":"Field","name":{"kind":"Name","value":"reason"}},{"kind":"Field","name":{"kind":"Name","value":"code"}}]}}]} as unknown as DocumentNode<DeliveryFieldsFragment, unknown>;
export const NotificationChannelFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"NotificationChannelFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"NotificationChannel"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"onByDefault"}},{"kind":"Field","name":{"kind":"Name","value":"settings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebhookSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"origin"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headerNames"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"NtfySettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"server"}},{"kind":"Field","name":{"kind":"Name","value":"priority"}},{"kind":"Field","name":{"kind":"Name","value":"tokenStored"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"EmailSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"security"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}},{"kind":"Field","name":{"kind":"Name","value":"passwordStored"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"AppriseSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"server"}},{"kind":"Field","name":{"kind":"Name","value":"mode"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"template"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"body"}}]}},{"kind":"Field","name":{"kind":"Name","value":"lastDelivery"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeliveryFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeliveryFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Delivery"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"channelId"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"ok"}},{"kind":"Field","name":{"kind":"Name","value":"reason"}},{"kind":"Field","name":{"kind":"Name","value":"code"}}]}}]} as unknown as DocumentNode<NotificationChannelFieldsFragment, unknown>;
export const NotificationSettingsFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"NotificationSettingsFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"NotificationSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"certificateDays"}},{"kind":"Field","name":{"kind":"Name","value":"template"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"body"}}]}},{"kind":"Field","name":{"kind":"Name","value":"placeholders"}}]}}]} as unknown as DocumentNode<NotificationSettingsFieldsFragment, unknown>;
export const VpnNodeFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"VpnNodeFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"VpnNode"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"address"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"role"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"routes"}},{"kind":"Field","name":{"kind":"Name","value":"waitingRoutes"}},{"kind":"Field","name":{"kind":"Name","value":"online"}},{"kind":"Field","name":{"kind":"Name","value":"lastSeen"}},{"kind":"Field","name":{"kind":"Name","value":"keyExpiry"}},{"kind":"Field","name":{"kind":"Name","value":"keyState"}},{"kind":"Field","name":{"kind":"Name","value":"path"}},{"kind":"Field","name":{"kind":"Name","value":"relayRegion"}},{"kind":"Field","name":{"kind":"Name","value":"inAnima"}},{"kind":"Field","name":{"kind":"Name","value":"device"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}}]} as unknown as DocumentNode<VpnNodeFieldsFragment, unknown>;
export const VpnPictureFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"VpnPictureFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"VpnPicture"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"readAt"}},{"kind":"Field","name":{"kind":"Name","value":"fresh"}},{"kind":"Field","name":{"kind":"Name","value":"hub"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnNodeFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"sites"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"subnet"}}]}},{"kind":"Field","name":{"kind":"Name","value":"router"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnNodeFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"state"}},{"kind":"Field","name":{"kind":"Name","value":"path"}},{"kind":"Field","name":{"kind":"Name","value":"relayRegion"}},{"kind":"Field","name":{"kind":"Name","value":"hasPingCheck"}},{"kind":"Field","name":{"kind":"Name","value":"waitingRouter"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnNodeFields"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"nodes"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnNodeFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"broker"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"VpnNodeFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"VpnNode"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"address"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"role"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"routes"}},{"kind":"Field","name":{"kind":"Name","value":"waitingRoutes"}},{"kind":"Field","name":{"kind":"Name","value":"online"}},{"kind":"Field","name":{"kind":"Name","value":"lastSeen"}},{"kind":"Field","name":{"kind":"Name","value":"keyExpiry"}},{"kind":"Field","name":{"kind":"Name","value":"keyState"}},{"kind":"Field","name":{"kind":"Name","value":"path"}},{"kind":"Field","name":{"kind":"Name","value":"relayRegion"}},{"kind":"Field","name":{"kind":"Name","value":"inAnima"}},{"kind":"Field","name":{"kind":"Name","value":"device"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}}]} as unknown as DocumentNode<VpnPictureFieldsFragment, unknown>;
export const LoginDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"Login"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"LoginInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"login"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"token"}}]}}]}}]} as unknown as DocumentNode<LoginMutation, LoginMutationVariables>;
export const SignInBrowserDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SignInBrowser"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"LoginInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"signInBrowser"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"expiresAt"}}]}}]}}]} as unknown as DocumentNode<SignInBrowserMutation, SignInBrowserMutationVariables>;
export const LogoutDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"Logout"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"logout"}}]}}]} as unknown as DocumentNode<LogoutMutation, LogoutMutationVariables>;
export const MeDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Me"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"me"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"username"}}]}}]}}]} as unknown as DocumentNode<MeQuery, MeQueryVariables>;
export const ApiKeysDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"ApiKeys"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"apiKeys"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"scope"}},{"kind":"Field","name":{"kind":"Name","value":"prefix"}},{"kind":"Field","name":{"kind":"Name","value":"expiresAt"}},{"kind":"Field","name":{"kind":"Name","value":"createdAt"}},{"kind":"Field","name":{"kind":"Name","value":"lastUsedAt"}}]}}]}}]} as unknown as DocumentNode<ApiKeysQuery, ApiKeysQueryVariables>;
export const CreateApiKeyDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateApiKey"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ApiKeyInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createApiKey"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"key"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"scope"}},{"kind":"Field","name":{"kind":"Name","value":"prefix"}},{"kind":"Field","name":{"kind":"Name","value":"expiresAt"}},{"kind":"Field","name":{"kind":"Name","value":"createdAt"}},{"kind":"Field","name":{"kind":"Name","value":"lastUsedAt"}}]}},{"kind":"Field","name":{"kind":"Name","value":"secret"}}]}}]}}]} as unknown as DocumentNode<CreateApiKeyMutation, CreateApiKeyMutationVariables>;
export const RevokeApiKeyDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"RevokeApiKey"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"revokeApiKey"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}]}]}}]} as unknown as DocumentNode<RevokeApiKeyMutation, RevokeApiKeyMutationVariables>;
export const BuildersDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Builders"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"builders"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"platform"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"arch"}}]}},{"kind":"Field","name":{"kind":"Name","value":"online"}}]}}]}}]} as unknown as DocumentNode<BuildersQuery, BuildersQueryVariables>;
export const EnrolBuilderDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"EnrolBuilder"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"name"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"platform"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"BuilderPlatformInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enrolBuilder"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"name"},"value":{"kind":"Variable","name":{"kind":"Name","value":"name"}}},{"kind":"Argument","name":{"kind":"Name","value":"platform"},"value":{"kind":"Variable","name":{"kind":"Name","value":"platform"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"builder"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"token"}}]}}]}}]} as unknown as DocumentNode<EnrolBuilderMutation, EnrolBuilderMutationVariables>;
export const RevokeBuilderDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"RevokeBuilder"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"revokeBuilder"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}]}]}}]} as unknown as DocumentNode<RevokeBuilderMutation, RevokeBuilderMutationVariables>;
export const ClaimBuildJobDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ClaimBuildJob"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"platform"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"BuilderPlatformInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"claimBuildJob"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"platform"},"value":{"kind":"Variable","name":{"kind":"Name","value":"platform"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"target"}},{"kind":"Field","name":{"kind":"Name","value":"version"}},{"kind":"Field","name":{"kind":"Name","value":"commit"}}]}}]}}]} as unknown as DocumentNode<ClaimBuildJobMutation, ClaimBuildJobMutationVariables>;
export const ReportBuildJobDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ReportBuildJob"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"state"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"BuildJobState"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"logLines"}},"type":{"kind":"NonNullType","type":{"kind":"ListType","type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"failureReason"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"reportBuildJob"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"state"},"value":{"kind":"Variable","name":{"kind":"Name","value":"state"}}},{"kind":"Argument","name":{"kind":"Name","value":"logLines"},"value":{"kind":"Variable","name":{"kind":"Name","value":"logLines"}}},{"kind":"Argument","name":{"kind":"Name","value":"failureReason"},"value":{"kind":"Variable","name":{"kind":"Name","value":"failureReason"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"state"}}]}}]}}]} as unknown as DocumentNode<ReportBuildJobMutation, ReportBuildJobMutationVariables>;
export const BuildUploadUrlsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"BuildUploadUrls"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"files"}},"type":{"kind":"NonNullType","type":{"kind":"ListType","type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"BuildFileInput"}}}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"buildUploadUrls"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"files"},"value":{"kind":"Variable","name":{"kind":"Name","value":"files"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"headers"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"expiresAt"}}]}}]}}]} as unknown as DocumentNode<BuildUploadUrlsMutation, BuildUploadUrlsMutationVariables>;
export const ReportBuildUploadedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ReportBuildUploaded"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"files"}},"type":{"kind":"NonNullType","type":{"kind":"ListType","type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"BuildFileInput"}}}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"feed"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"reportBuildUploaded"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"files"},"value":{"kind":"Variable","name":{"kind":"Name","value":"files"}}},{"kind":"Argument","name":{"kind":"Name","value":"feed"},"value":{"kind":"Variable","name":{"kind":"Name","value":"feed"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"state"}}]}}]}}]} as unknown as DocumentNode<ReportBuildUploadedMutation, ReportBuildUploadedMutationVariables>;
export const HeartbeatAddedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"HeartbeatAdded"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"heartbeatAdded"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"NullValue"}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"heartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}}]}}]}}]}}]} as unknown as DocumentNode<HeartbeatAddedSubscription, HeartbeatAddedSubscriptionVariables>;
export const CheckDefaultsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"CheckDefaults"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkDefaults"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}}]}}]}}]} as unknown as DocumentNode<CheckDefaultsQuery, CheckDefaultsQueryVariables>;
export const SetCheckDefaultsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SetCheckDefaults"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"CheckDefaultsInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"setCheckDefaults"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}}]}}]}}]} as unknown as DocumentNode<SetCheckDefaultsMutation, SetCheckDefaultsMutationVariables>;
export const DeviceChecksDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"DeviceChecks"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"device"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"checks"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CheckFields"}}]}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CheckFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Check"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"type"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}},{"kind":"Field","name":{"kind":"Name","value":"invert"}},{"kind":"Field","name":{"kind":"Name","value":"paused"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"certificateDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"domainDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"lastHeartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}},{"kind":"Field","name":{"kind":"Name","value":"config"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"IcmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}},{"kind":"Field","name":{"kind":"Name","value":"ipVersion"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TcpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TlsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"serverName"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"HttpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headers"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"body"}},{"kind":"Field","name":{"kind":"Name","value":"maxRedirects"}},{"kind":"Field","name":{"kind":"Name","value":"acceptedStatus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}}]}},{"kind":"Field","name":{"kind":"Name","value":"keyword"}},{"kind":"Field","name":{"kind":"Name","value":"jsonQuery"}},{"kind":"Field","name":{"kind":"Name","value":"jsonExpected"}},{"kind":"Field","name":{"kind":"Name","value":"authKind"}},{"kind":"Field","name":{"kind":"Name","value":"verifyTls"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DnsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"recordType"}},{"kind":"Field","name":{"kind":"Name","value":"resolver"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"PushConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pushPath"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"GroupConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"children"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DomainExpiryConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"warnDays"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]}}]}}]} as unknown as DocumentNode<DeviceChecksQuery, DeviceChecksQueryVariables>;
export const CheckHeartbeatsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"CheckHeartbeats"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"first"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"check"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"heartbeats"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"Variable","name":{"kind":"Name","value":"first"}}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}}]}}]}}]}}]}}]} as unknown as DocumentNode<CheckHeartbeatsQuery, CheckHeartbeatsQueryVariables>;
export const CreateCheckDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateCheck"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"CreateCheckInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createCheck"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CheckFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CheckFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Check"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"type"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}},{"kind":"Field","name":{"kind":"Name","value":"invert"}},{"kind":"Field","name":{"kind":"Name","value":"paused"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"certificateDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"domainDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"lastHeartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}},{"kind":"Field","name":{"kind":"Name","value":"config"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"IcmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}},{"kind":"Field","name":{"kind":"Name","value":"ipVersion"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TcpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TlsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"serverName"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"HttpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headers"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"body"}},{"kind":"Field","name":{"kind":"Name","value":"maxRedirects"}},{"kind":"Field","name":{"kind":"Name","value":"acceptedStatus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}}]}},{"kind":"Field","name":{"kind":"Name","value":"keyword"}},{"kind":"Field","name":{"kind":"Name","value":"jsonQuery"}},{"kind":"Field","name":{"kind":"Name","value":"jsonExpected"}},{"kind":"Field","name":{"kind":"Name","value":"authKind"}},{"kind":"Field","name":{"kind":"Name","value":"verifyTls"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DnsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"recordType"}},{"kind":"Field","name":{"kind":"Name","value":"resolver"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"PushConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pushPath"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"GroupConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"children"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DomainExpiryConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"warnDays"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]}}]}}]} as unknown as DocumentNode<CreateCheckMutation, CreateCheckMutationVariables>;
export const UpdateCheckDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"UpdateCheck"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"UpdateCheckInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"updateCheck"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CheckFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CheckFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Check"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"type"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}},{"kind":"Field","name":{"kind":"Name","value":"invert"}},{"kind":"Field","name":{"kind":"Name","value":"paused"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"certificateDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"domainDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"lastHeartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}},{"kind":"Field","name":{"kind":"Name","value":"config"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"IcmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}},{"kind":"Field","name":{"kind":"Name","value":"ipVersion"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TcpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TlsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"serverName"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"HttpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headers"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"body"}},{"kind":"Field","name":{"kind":"Name","value":"maxRedirects"}},{"kind":"Field","name":{"kind":"Name","value":"acceptedStatus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}}]}},{"kind":"Field","name":{"kind":"Name","value":"keyword"}},{"kind":"Field","name":{"kind":"Name","value":"jsonQuery"}},{"kind":"Field","name":{"kind":"Name","value":"jsonExpected"}},{"kind":"Field","name":{"kind":"Name","value":"authKind"}},{"kind":"Field","name":{"kind":"Name","value":"verifyTls"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DnsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"recordType"}},{"kind":"Field","name":{"kind":"Name","value":"resolver"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"PushConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pushPath"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"GroupConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"children"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DomainExpiryConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"warnDays"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]}}]}}]} as unknown as DocumentNode<UpdateCheckMutation, UpdateCheckMutationVariables>;
export const DeleteCheckDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DeleteCheck"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deleteCheck"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}]}]}}]} as unknown as DocumentNode<DeleteCheckMutation, DeleteCheckMutationVariables>;
export const SetCheckPausedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SetCheckPaused"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"paused"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Boolean"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"setCheckPaused"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"paused"},"value":{"kind":"Variable","name":{"kind":"Name","value":"paused"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CheckFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CheckFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Check"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"type"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}},{"kind":"Field","name":{"kind":"Name","value":"invert"}},{"kind":"Field","name":{"kind":"Name","value":"paused"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"certificateDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"domainDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"lastHeartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}},{"kind":"Field","name":{"kind":"Name","value":"config"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"IcmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}},{"kind":"Field","name":{"kind":"Name","value":"ipVersion"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TcpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TlsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"serverName"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"HttpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headers"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"body"}},{"kind":"Field","name":{"kind":"Name","value":"maxRedirects"}},{"kind":"Field","name":{"kind":"Name","value":"acceptedStatus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}}]}},{"kind":"Field","name":{"kind":"Name","value":"keyword"}},{"kind":"Field","name":{"kind":"Name","value":"jsonQuery"}},{"kind":"Field","name":{"kind":"Name","value":"jsonExpected"}},{"kind":"Field","name":{"kind":"Name","value":"authKind"}},{"kind":"Field","name":{"kind":"Name","value":"verifyTls"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DnsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"recordType"}},{"kind":"Field","name":{"kind":"Name","value":"resolver"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"PushConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pushPath"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"GroupConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"children"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DomainExpiryConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"warnDays"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]}}]}}]} as unknown as DocumentNode<SetCheckPausedMutation, SetCheckPausedMutationVariables>;
export const CheckNotificationsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"CheckNotifications"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"check"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"notifications"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"channels"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"resendEveryBeats"}}]}}]}}]}}]} as unknown as DocumentNode<CheckNotificationsQuery, CheckNotificationsQueryVariables>;
export const SetCheckNotificationsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SetCheckNotifications"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"checkId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"CheckNotificationsInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"setCheckNotifications"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"checkId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"checkId"}}},{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"channels"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"resendEveryBeats"}}]}}]}}]} as unknown as DocumentNode<SetCheckNotificationsMutation, SetCheckNotificationsMutationVariables>;
export const DevicesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Devices"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"filter"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"DeviceFilter"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"sort"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"DeviceSort"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"first"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"devices"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"filter"},"value":{"kind":"Variable","name":{"kind":"Name","value":"filter"}}},{"kind":"Argument","name":{"kind":"Name","value":"sort"},"value":{"kind":"Variable","name":{"kind":"Name","value":"sort"}}},{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"Variable","name":{"kind":"Name","value":"first"}}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"totalCount"}},{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"distribution"}},{"kind":"Field","name":{"kind":"Name","value":"macAddress"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"readings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CardReadings"}}]}},{"kind":"Field","name":{"kind":"Name","value":"accessWarnings"}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"rdp"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}}]}},{"kind":"Field","name":{"kind":"Name","value":"vnc"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"username"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"checks"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"heartbeats"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"IntValue","value":"50"}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}}]}}]}}]}}]}}]}}]}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CardReadings"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Readings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"cpuPercent"}},{"kind":"Field","name":{"kind":"Name","value":"memoryPercent"}},{"kind":"Field","name":{"kind":"Name","value":"diskPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramGpu"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"index"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"vendor"}},{"kind":"Field","name":{"kind":"Name","value":"hasVram"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}}]} as unknown as DocumentNode<DevicesQuery, DevicesQueryVariables>;
export const DeviceHostsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"DeviceHosts"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"devices"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"IntValue","value":"200"}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"host"}}]}}]}}]}}]}}]} as unknown as DocumentNode<DeviceHostsQuery, DeviceHostsQueryVariables>;
export const TagsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Tags"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"tags"}}]}}]} as unknown as DocumentNode<TagsQuery, TagsQueryVariables>;
export const DeviceStatusChangedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"DeviceStatusChanged"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deviceStatusChanged"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"NullValue"}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"at"}}]}}]}}]} as unknown as DocumentNode<DeviceStatusChangedSubscription, DeviceStatusChangedSubscriptionVariables>;
export const CardReadingsAddedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"CardReadingsAdded"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"metricsAdded"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"NullValue"}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"readings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CardReadings"}}]}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CardReadings"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Readings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"cpuPercent"}},{"kind":"Field","name":{"kind":"Name","value":"memoryPercent"}},{"kind":"Field","name":{"kind":"Name","value":"diskPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramGpu"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"index"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"vendor"}},{"kind":"Field","name":{"kind":"Name","value":"hasVram"}}]}}]}}]} as unknown as DocumentNode<CardReadingsAddedSubscription, CardReadingsAddedSubscriptionVariables>;
export const DeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Device"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"device"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeviceDetailFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceSuggestion"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkId"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"label"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeviceDetailFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Device"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"distribution"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"auth"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"macAddress"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"accessWarnings"}},{"kind":"Field","name":{"kind":"Name","value":"rdp"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"security"}},{"kind":"Field","name":{"kind":"Name","value":"ignoreCertificate"}},{"kind":"Field","name":{"kind":"Name","value":"resizeMethod"}},{"kind":"Field","name":{"kind":"Name","value":"audioOut"}},{"kind":"Field","name":{"kind":"Name","value":"microphone"}},{"kind":"Field","name":{"kind":"Name","value":"sharedDrive"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"serverCertificate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"subject"}},{"kind":"Field","name":{"kind":"Name","value":"notAfter"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"vnc"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}},{"kind":"Field","name":{"kind":"Name","value":"transfer"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"upload"}},{"kind":"Field","name":{"kind":"Name","value":"download"}},{"kind":"Field","name":{"kind":"Name","value":"copy"}},{"kind":"Field","name":{"kind":"Name","value":"paste"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaceSuggestions"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"}}]}},{"kind":"Field","alias":{"kind":"Name","value":"day"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","alias":{"kind":"Name","value":"week"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"WEEK"}}]},{"kind":"Field","alias":{"kind":"Name","value":"month"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"MONTH"}}]}]}}]} as unknown as DocumentNode<DeviceQuery, DeviceQueryVariables>;
export const SitesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Sites"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sites"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"subnet"}}]}}]}}]} as unknown as DocumentNode<SitesQuery, SitesQueryVariables>;
export const CreateSiteDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateSite"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"name"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"subnet"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Cidr"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createSite"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"name"},"value":{"kind":"Variable","name":{"kind":"Name","value":"name"}}},{"kind":"Argument","name":{"kind":"Name","value":"subnet"},"value":{"kind":"Variable","name":{"kind":"Name","value":"subnet"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"subnet"}}]}}]}}]} as unknown as DocumentNode<CreateSiteMutation, CreateSiteMutationVariables>;
export const CreateDeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateDevice"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"CreateDeviceInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createDevice"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeviceDetailFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceSuggestion"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkId"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"label"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeviceDetailFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Device"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"distribution"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"auth"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"macAddress"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"accessWarnings"}},{"kind":"Field","name":{"kind":"Name","value":"rdp"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"security"}},{"kind":"Field","name":{"kind":"Name","value":"ignoreCertificate"}},{"kind":"Field","name":{"kind":"Name","value":"resizeMethod"}},{"kind":"Field","name":{"kind":"Name","value":"audioOut"}},{"kind":"Field","name":{"kind":"Name","value":"microphone"}},{"kind":"Field","name":{"kind":"Name","value":"sharedDrive"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"serverCertificate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"subject"}},{"kind":"Field","name":{"kind":"Name","value":"notAfter"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"vnc"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}},{"kind":"Field","name":{"kind":"Name","value":"transfer"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"upload"}},{"kind":"Field","name":{"kind":"Name","value":"download"}},{"kind":"Field","name":{"kind":"Name","value":"copy"}},{"kind":"Field","name":{"kind":"Name","value":"paste"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaceSuggestions"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"}}]}},{"kind":"Field","alias":{"kind":"Name","value":"day"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","alias":{"kind":"Name","value":"week"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"WEEK"}}]},{"kind":"Field","alias":{"kind":"Name","value":"month"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"MONTH"}}]}]}}]} as unknown as DocumentNode<CreateDeviceMutation, CreateDeviceMutationVariables>;
export const UpdateDeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"UpdateDevice"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"UpdateDeviceInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"updateDevice"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeviceDetailFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceSuggestion"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkId"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"label"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeviceDetailFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Device"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"distribution"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"auth"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"macAddress"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"accessWarnings"}},{"kind":"Field","name":{"kind":"Name","value":"rdp"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"security"}},{"kind":"Field","name":{"kind":"Name","value":"ignoreCertificate"}},{"kind":"Field","name":{"kind":"Name","value":"resizeMethod"}},{"kind":"Field","name":{"kind":"Name","value":"audioOut"}},{"kind":"Field","name":{"kind":"Name","value":"microphone"}},{"kind":"Field","name":{"kind":"Name","value":"sharedDrive"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"serverCertificate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"subject"}},{"kind":"Field","name":{"kind":"Name","value":"notAfter"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"vnc"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}},{"kind":"Field","name":{"kind":"Name","value":"transfer"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"upload"}},{"kind":"Field","name":{"kind":"Name","value":"download"}},{"kind":"Field","name":{"kind":"Name","value":"copy"}},{"kind":"Field","name":{"kind":"Name","value":"paste"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaceSuggestions"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"}}]}},{"kind":"Field","alias":{"kind":"Name","value":"day"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","alias":{"kind":"Name","value":"week"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"WEEK"}}]},{"kind":"Field","alias":{"kind":"Name","value":"month"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"MONTH"}}]}]}}]} as unknown as DocumentNode<UpdateDeviceMutation, UpdateDeviceMutationVariables>;
export const DeleteDeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DeleteDevice"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deleteDevice"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}]}]}}]} as unknown as DocumentNode<DeleteDeviceMutation, DeleteDeviceMutationVariables>;
export const SetSshCredentialDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SetSshCredential"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"credential"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"SshCredentialInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"setSshCredential"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"credential"},"value":{"kind":"Variable","name":{"kind":"Name","value":"credential"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}}]}}]} as unknown as DocumentNode<SetSshCredentialMutation, SetSshCredentialMutationVariables>;
export const ClearSshCredentialDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ClearSshCredential"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"clearSshCredential"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}}]}}]} as unknown as DocumentNode<ClearSshCredentialMutation, ClearSshCredentialMutationVariables>;
export const ClearRdpCredentialDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ClearRdpCredential"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"clearRdpCredential"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}}]}}]} as unknown as DocumentNode<ClearRdpCredentialMutation, ClearRdpCredentialMutationVariables>;
export const ClearVncCredentialDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ClearVncCredential"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"clearVncCredential"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}}]}}]} as unknown as DocumentNode<ClearVncCredentialMutation, ClearVncCredentialMutationVariables>;
export const DevicePolicyDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"DevicePolicy"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"device"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sessionLimit"}},{"kind":"Field","name":{"kind":"Name","value":"recordSessions"}}]}}]}}]} as unknown as DocumentNode<DevicePolicyQuery, DevicePolicyQueryVariables>;
export const SetDevicePolicyDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SetDevicePolicy"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"UpdateDeviceInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"updateDevice"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sessionLimit"}},{"kind":"Field","name":{"kind":"Name","value":"recordSessions"}}]}}]}}]} as unknown as DocumentNode<SetDevicePolicyMutation, SetDevicePolicyMutationVariables>;
export const EventsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Events"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"filter"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"EventFilter"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"first"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"events"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"filter"},"value":{"kind":"Variable","name":{"kind":"Name","value":"filter"}}},{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"Variable","name":{"kind":"Name","value":"first"}}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"severity"}},{"kind":"Field","name":{"kind":"Name","value":"source"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceName"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}}]}}]}}]}}]} as unknown as DocumentNode<EventsQuery, EventsQueryVariables>;
export const EventAddedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"EventAdded"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"minSeverity"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Severity"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"eventAdded"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"minSeverity"},"value":{"kind":"Variable","name":{"kind":"Name","value":"minSeverity"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"severity"}},{"kind":"Field","name":{"kind":"Name","value":"source"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceName"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}}]}}]} as unknown as DocumentNode<EventAddedSubscription, EventAddedSubscriptionVariables>;
export const DeviceMetricsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"DeviceMetrics"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"range"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"MetricRange"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"device"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"gpus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"index"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"vendor"}},{"kind":"Field","name":{"kind":"Name","value":"hasVram"}}]}},{"kind":"Field","name":{"kind":"Name","value":"readings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"cpuPercent"}},{"kind":"Field","name":{"kind":"Name","value":"memoryPercent"}},{"kind":"Field","name":{"kind":"Name","value":"diskPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramGpu"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"index"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"vendor"}},{"kind":"Field","name":{"kind":"Name","value":"hasVram"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"metrics"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"range"},"value":{"kind":"Variable","name":{"kind":"Name","value":"range"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"gpuIndex"}},{"kind":"Field","name":{"kind":"Name","value":"points"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"value"}},{"kind":"Field","name":{"kind":"Name","value":"min"}},{"kind":"Field","name":{"kind":"Name","value":"max"}}]}}]}}]}}]}}]} as unknown as DocumentNode<DeviceMetricsQuery, DeviceMetricsQueryVariables>;
export const MetricsAddedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"MetricsAdded"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"metricsAdded"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"samples"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"readings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"cpuPercent"}},{"kind":"Field","name":{"kind":"Name","value":"memoryPercent"}},{"kind":"Field","name":{"kind":"Name","value":"diskPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramGpu"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"index"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"vendor"}},{"kind":"Field","name":{"kind":"Name","value":"hasVram"}}]}}]}}]}}]}}]} as unknown as DocumentNode<MetricsAddedSubscription, MetricsAddedSubscriptionVariables>;
export const AgentStatusDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"AgentStatus"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"device"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"metricsStale"}}]}}]}}]} as unknown as DocumentNode<AgentStatusQuery, AgentStatusQueryVariables>;
export const EnrolAgentDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"EnrolAgent"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enrolAgent"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"code"}},{"kind":"Field","name":{"kind":"Name","value":"expiresAt"}}]}}]}}]} as unknown as DocumentNode<EnrolAgentMutation, EnrolAgentMutationVariables>;
export const RevokeAgentTokenDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"RevokeAgentToken"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"revokeAgentToken"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"metricsStale"}}]}}]}}]} as unknown as DocumentNode<RevokeAgentTokenMutation, RevokeAgentTokenMutationVariables>;
export const NotificationChannelsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"NotificationChannels"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"notificationChannels"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"NotificationChannelFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeliveryFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Delivery"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"channelId"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"ok"}},{"kind":"Field","name":{"kind":"Name","value":"reason"}},{"kind":"Field","name":{"kind":"Name","value":"code"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"NotificationChannelFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"NotificationChannel"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"onByDefault"}},{"kind":"Field","name":{"kind":"Name","value":"settings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebhookSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"origin"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headerNames"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"NtfySettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"server"}},{"kind":"Field","name":{"kind":"Name","value":"priority"}},{"kind":"Field","name":{"kind":"Name","value":"tokenStored"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"EmailSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"security"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}},{"kind":"Field","name":{"kind":"Name","value":"passwordStored"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"AppriseSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"server"}},{"kind":"Field","name":{"kind":"Name","value":"mode"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"template"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"body"}}]}},{"kind":"Field","name":{"kind":"Name","value":"lastDelivery"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeliveryFields"}}]}}]}}]} as unknown as DocumentNode<NotificationChannelsQuery, NotificationChannelsQueryVariables>;
export const NotificationSettingsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"NotificationSettings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"notificationSettings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"NotificationSettingsFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"NotificationSettingsFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"NotificationSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"certificateDays"}},{"kind":"Field","name":{"kind":"Name","value":"template"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"body"}}]}},{"kind":"Field","name":{"kind":"Name","value":"placeholders"}}]}}]} as unknown as DocumentNode<NotificationSettingsQuery, NotificationSettingsQueryVariables>;
export const CreateNotificationChannelDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateNotificationChannel"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"NotificationChannelInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createNotificationChannel"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"NotificationChannelFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeliveryFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Delivery"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"channelId"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"ok"}},{"kind":"Field","name":{"kind":"Name","value":"reason"}},{"kind":"Field","name":{"kind":"Name","value":"code"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"NotificationChannelFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"NotificationChannel"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"onByDefault"}},{"kind":"Field","name":{"kind":"Name","value":"settings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebhookSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"origin"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headerNames"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"NtfySettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"server"}},{"kind":"Field","name":{"kind":"Name","value":"priority"}},{"kind":"Field","name":{"kind":"Name","value":"tokenStored"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"EmailSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"security"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}},{"kind":"Field","name":{"kind":"Name","value":"passwordStored"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"AppriseSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"server"}},{"kind":"Field","name":{"kind":"Name","value":"mode"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"template"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"body"}}]}},{"kind":"Field","name":{"kind":"Name","value":"lastDelivery"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeliveryFields"}}]}}]}}]} as unknown as DocumentNode<CreateNotificationChannelMutation, CreateNotificationChannelMutationVariables>;
export const UpdateNotificationChannelDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"UpdateNotificationChannel"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"NotificationChannelInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"updateNotificationChannel"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"NotificationChannelFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeliveryFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Delivery"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"channelId"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"ok"}},{"kind":"Field","name":{"kind":"Name","value":"reason"}},{"kind":"Field","name":{"kind":"Name","value":"code"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"NotificationChannelFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"NotificationChannel"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"onByDefault"}},{"kind":"Field","name":{"kind":"Name","value":"settings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebhookSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"origin"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headerNames"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"NtfySettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"server"}},{"kind":"Field","name":{"kind":"Name","value":"priority"}},{"kind":"Field","name":{"kind":"Name","value":"tokenStored"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"EmailSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"security"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}},{"kind":"Field","name":{"kind":"Name","value":"passwordStored"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"AppriseSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"server"}},{"kind":"Field","name":{"kind":"Name","value":"mode"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"template"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"body"}}]}},{"kind":"Field","name":{"kind":"Name","value":"lastDelivery"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeliveryFields"}}]}}]}}]} as unknown as DocumentNode<UpdateNotificationChannelMutation, UpdateNotificationChannelMutationVariables>;
export const DeleteNotificationChannelDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DeleteNotificationChannel"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deleteNotificationChannel"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}]}]}}]} as unknown as DocumentNode<DeleteNotificationChannelMutation, DeleteNotificationChannelMutationVariables>;
export const SendTestNotificationDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SendTestNotification"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sendTestNotification"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeliveryFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeliveryFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Delivery"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"channelId"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"ok"}},{"kind":"Field","name":{"kind":"Name","value":"reason"}},{"kind":"Field","name":{"kind":"Name","value":"code"}}]}}]} as unknown as DocumentNode<SendTestNotificationMutation, SendTestNotificationMutationVariables>;
export const SetNotificationSettingsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SetNotificationSettings"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"NotificationSettingsInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"setNotificationSettings"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"NotificationSettingsFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"NotificationSettingsFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"NotificationSettings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"certificateDays"}},{"kind":"Field","name":{"kind":"Name","value":"template"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"body"}}]}},{"kind":"Field","name":{"kind":"Name","value":"placeholders"}}]}}]} as unknown as DocumentNode<SetNotificationSettingsMutation, SetNotificationSettingsMutationVariables>;
export const RetentionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Retention"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"retention"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"subject"}},{"kind":"Field","name":{"kind":"Name","value":"days"}},{"kind":"Field","name":{"kind":"Name","value":"minDays"}},{"kind":"Field","name":{"kind":"Name","value":"maxDays"}},{"kind":"Field","name":{"kind":"Name","value":"defaultDays"}}]}}]}}]} as unknown as DocumentNode<RetentionQuery, RetentionQueryVariables>;
export const SetRetentionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SetRetention"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"subject"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"RetentionSubject"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"days"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"setRetention"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"subject"},"value":{"kind":"Variable","name":{"kind":"Name","value":"subject"}}},{"kind":"Argument","name":{"kind":"Name","value":"days"},"value":{"kind":"Variable","name":{"kind":"Name","value":"days"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"subject"}},{"kind":"Field","name":{"kind":"Name","value":"days"}},{"kind":"Field","name":{"kind":"Name","value":"minDays"}},{"kind":"Field","name":{"kind":"Name","value":"maxDays"}},{"kind":"Field","name":{"kind":"Name","value":"defaultDays"}}]}}]}}]} as unknown as DocumentNode<SetRetentionMutation, SetRetentionMutationVariables>;
export const StartSessionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"StartSession"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"StartSessionInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"startSession"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sessionId"}},{"kind":"Field","name":{"kind":"Name","value":"token"}},{"kind":"Field","name":{"kind":"Name","value":"tunnelPath"}},{"kind":"Field","name":{"kind":"Name","value":"expiresAt"}},{"kind":"Field","name":{"kind":"Name","value":"wakeSent"}}]}}]}}]} as unknown as DocumentNode<StartSessionMutation, StartSessionMutationVariables>;
export const EndSessionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"EndSession"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"endSession"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceName"}},{"kind":"Field","name":{"kind":"Name","value":"protocol"}},{"kind":"Field","name":{"kind":"Name","value":"startedAt"}},{"kind":"Field","name":{"kind":"Name","value":"endedAt"}},{"kind":"Field","name":{"kind":"Name","value":"endReason"}}]}}]}}]} as unknown as DocumentNode<EndSessionMutation, EndSessionMutationVariables>;
export const TestConnectionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"TestConnection"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ConnectionTestInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"testConnection"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"reachable"}},{"kind":"Field","name":{"kind":"Name","value":"authenticated"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}},{"kind":"Field","name":{"kind":"Name","value":"hostKeyChanged"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}}]}}]} as unknown as DocumentNode<TestConnectionMutation, TestConnectionMutationVariables>;
export const AcceptHostKeyDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"AcceptHostKey"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"fingerprintSha256"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"acceptHostKey"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"fingerprintSha256"},"value":{"kind":"Variable","name":{"kind":"Name","value":"fingerprintSha256"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}}]} as unknown as DocumentNode<AcceptHostKeyMutation, AcceptHostKeyMutationVariables>;
export const TestRemoteConnectionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"TestRemoteConnection"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ConnectionTestInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"testConnection"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"reachable"}},{"kind":"Field","name":{"kind":"Name","value":"authenticated"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}},{"kind":"Field","name":{"kind":"Name","value":"hostKeyChanged"}},{"kind":"Field","name":{"kind":"Name","value":"message"}},{"kind":"Field","name":{"kind":"Name","value":"serverCertificate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"subject"}},{"kind":"Field","name":{"kind":"Name","value":"notAfter"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}},{"kind":"Field","name":{"kind":"Name","value":"certificateChanged"}},{"kind":"Field","name":{"kind":"Name","value":"certificateMatched"}}]}}]}}]} as unknown as DocumentNode<TestRemoteConnectionMutation, TestRemoteConnectionMutationVariables>;
export const AcceptServerCertificateDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"AcceptServerCertificate"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"fingerprintSha256"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"acceptServerCertificate"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"fingerprintSha256"},"value":{"kind":"Variable","name":{"kind":"Name","value":"fingerprintSha256"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"subject"}},{"kind":"Field","name":{"kind":"Name","value":"notAfter"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}}]} as unknown as DocumentNode<AcceptServerCertificateMutation, AcceptServerCertificateMutationVariables>;
export const DeviceTransferDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"DeviceTransfer"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"device"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"transfer"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"copy"}},{"kind":"Field","name":{"kind":"Name","value":"paste"}}]}}]}}]}}]} as unknown as DocumentNode<DeviceTransferQuery, DeviceTransferQueryVariables>;
export const SetRdpCredentialDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SetRdpCredential"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"credential"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"PasswordCredentialInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"setRdpCredential"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"credential"},"value":{"kind":"Variable","name":{"kind":"Name","value":"credential"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}}]}}]} as unknown as DocumentNode<SetRdpCredentialMutation, SetRdpCredentialMutationVariables>;
export const SetVncCredentialDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SetVncCredential"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"credential"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"PasswordCredentialInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"setVncCredential"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"credential"},"value":{"kind":"Variable","name":{"kind":"Name","value":"credential"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}}]}}]} as unknown as DocumentNode<SetVncCredentialMutation, SetVncCredentialMutationVariables>;
export const DeviceSessionsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"DeviceSessions"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"first"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sessions"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"Variable","name":{"kind":"Name","value":"first"}}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"protocol"}},{"kind":"Field","name":{"kind":"Name","value":"operatorName"}},{"kind":"Field","name":{"kind":"Name","value":"startedAt"}},{"kind":"Field","name":{"kind":"Name","value":"endedAt"}},{"kind":"Field","name":{"kind":"Name","value":"endReason"}}]}}]}}]}}]}}]} as unknown as DocumentNode<DeviceSessionsQuery, DeviceSessionsQueryVariables>;
export const OpenFilesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"OpenFiles"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"sessionId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"openFiles"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"sessionId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"sessionId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sessionId"}},{"kind":"Field","name":{"kind":"Name","value":"token"}},{"kind":"Field","name":{"kind":"Name","value":"tunnelPath"}},{"kind":"Field","name":{"kind":"Name","value":"expiresAt"}}]}}]}}]} as unknown as DocumentNode<OpenFilesMutation, OpenFilesMutationVariables>;
export const LiveSessionsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"LiveSessions"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"first"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sessions"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"live"},"value":{"kind":"BooleanValue","value":true}},{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"Variable","name":{"kind":"Name","value":"first"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceName"}},{"kind":"Field","name":{"kind":"Name","value":"protocol"}},{"kind":"Field","name":{"kind":"Name","value":"operatorName"}},{"kind":"Field","name":{"kind":"Name","value":"startedAt"}},{"kind":"Field","name":{"kind":"Name","value":"clientAddr"}}]}}]}}]}}]}}]} as unknown as DocumentNode<LiveSessionsQuery, LiveSessionsQueryVariables>;
export const LiveSessionChangedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"LiveSessionChanged"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sessionChanged"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"session"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceName"}},{"kind":"Field","name":{"kind":"Name","value":"protocol"}},{"kind":"Field","name":{"kind":"Name","value":"operatorName"}},{"kind":"Field","name":{"kind":"Name","value":"startedAt"}},{"kind":"Field","name":{"kind":"Name","value":"clientAddr"}},{"kind":"Field","name":{"kind":"Name","value":"endedAt"}},{"kind":"Field","name":{"kind":"Name","value":"endReason"}}]}}]}}]}}]} as unknown as DocumentNode<LiveSessionChangedSubscription, LiveSessionChangedSubscriptionVariables>;
export const WakeDeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"WakeDevice"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"wakeDevice"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sentAt"}},{"kind":"Field","name":{"kind":"Name","value":"relayName"}}]}}]}}]} as unknown as DocumentNode<WakeDeviceMutation, WakeDeviceMutationVariables>;
export const VpnDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Vpn"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"vpn"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnPictureFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"VpnNodeFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"VpnNode"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"address"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"role"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"routes"}},{"kind":"Field","name":{"kind":"Name","value":"waitingRoutes"}},{"kind":"Field","name":{"kind":"Name","value":"online"}},{"kind":"Field","name":{"kind":"Name","value":"lastSeen"}},{"kind":"Field","name":{"kind":"Name","value":"keyExpiry"}},{"kind":"Field","name":{"kind":"Name","value":"keyState"}},{"kind":"Field","name":{"kind":"Name","value":"path"}},{"kind":"Field","name":{"kind":"Name","value":"relayRegion"}},{"kind":"Field","name":{"kind":"Name","value":"inAnima"}},{"kind":"Field","name":{"kind":"Name","value":"device"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"VpnPictureFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"VpnPicture"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"readAt"}},{"kind":"Field","name":{"kind":"Name","value":"fresh"}},{"kind":"Field","name":{"kind":"Name","value":"hub"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnNodeFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"sites"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"subnet"}}]}},{"kind":"Field","name":{"kind":"Name","value":"router"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnNodeFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"state"}},{"kind":"Field","name":{"kind":"Name","value":"path"}},{"kind":"Field","name":{"kind":"Name","value":"relayRegion"}},{"kind":"Field","name":{"kind":"Name","value":"hasPingCheck"}},{"kind":"Field","name":{"kind":"Name","value":"waitingRouter"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnNodeFields"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"nodes"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnNodeFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"broker"}}]}}]} as unknown as DocumentNode<VpnQuery, VpnQueryVariables>;
export const VpnChangedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"VpnChanged"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"vpnChanged"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnPictureFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"VpnNodeFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"VpnNode"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"address"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"role"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"routes"}},{"kind":"Field","name":{"kind":"Name","value":"waitingRoutes"}},{"kind":"Field","name":{"kind":"Name","value":"online"}},{"kind":"Field","name":{"kind":"Name","value":"lastSeen"}},{"kind":"Field","name":{"kind":"Name","value":"keyExpiry"}},{"kind":"Field","name":{"kind":"Name","value":"keyState"}},{"kind":"Field","name":{"kind":"Name","value":"path"}},{"kind":"Field","name":{"kind":"Name","value":"relayRegion"}},{"kind":"Field","name":{"kind":"Name","value":"inAnima"}},{"kind":"Field","name":{"kind":"Name","value":"device"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"VpnPictureFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"VpnPicture"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"readAt"}},{"kind":"Field","name":{"kind":"Name","value":"fresh"}},{"kind":"Field","name":{"kind":"Name","value":"hub"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnNodeFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"sites"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"subnet"}}]}},{"kind":"Field","name":{"kind":"Name","value":"router"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnNodeFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"state"}},{"kind":"Field","name":{"kind":"Name","value":"path"}},{"kind":"Field","name":{"kind":"Name","value":"relayRegion"}},{"kind":"Field","name":{"kind":"Name","value":"hasPingCheck"}},{"kind":"Field","name":{"kind":"Name","value":"waitingRouter"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnNodeFields"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"nodes"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"VpnNodeFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"broker"}}]}}]} as unknown as DocumentNode<VpnChangedSubscription, VpnChangedSubscriptionVariables>;
export const CreateVpnInviteDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateVpnInvite"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createVpnInvite"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"key"}},{"kind":"Field","name":{"kind":"Name","value":"expiresAt"}},{"kind":"Field","name":{"kind":"Name","value":"steps"}}]}}]}}]} as unknown as DocumentNode<CreateVpnInviteMutation, CreateVpnInviteMutationVariables>;
export const CreateRouterKeyDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateRouterKey"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"siteId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createRouterKey"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"siteId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"siteId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"key"}},{"kind":"Field","name":{"kind":"Name","value":"expiresAt"}},{"kind":"Field","name":{"kind":"Name","value":"steps"}}]}}]}}]} as unknown as DocumentNode<CreateRouterKeyMutation, CreateRouterKeyMutationVariables>;
export const RegisterVpnNodeDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"RegisterVpnNode"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"authId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"registerVpnNode"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"authId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"authId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"address"}},{"kind":"Field","name":{"kind":"Name","value":"keyExpiry"}}]}}]}}]} as unknown as DocumentNode<RegisterVpnNodeMutation, RegisterVpnNodeMutationVariables>;
export const RenewVpnKeyDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"RenewVpnKey"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"name"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"renewVpnKey"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"name"},"value":{"kind":"Variable","name":{"kind":"Name","value":"name"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"address"}},{"kind":"Field","name":{"kind":"Name","value":"keyExpiry"}}]}}]}}]} as unknown as DocumentNode<RenewVpnKeyMutation, RenewVpnKeyMutationVariables>;
export const ApproveSiteRouteDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ApproveSiteRoute"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"siteId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"approveSiteRoute"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"siteId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"siteId"}}}]}]}}]} as unknown as DocumentNode<ApproveSiteRouteMutation, ApproveSiteRouteMutationVariables>;
export const CreateWebInterfaceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateWebInterface"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createWebInterface"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}}]} as unknown as DocumentNode<CreateWebInterfaceMutation, CreateWebInterfaceMutationVariables>;
export const UpdateWebInterfaceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"UpdateWebInterface"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"updateWebInterface"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}}]} as unknown as DocumentNode<UpdateWebInterfaceMutation, UpdateWebInterfaceMutationVariables>;
export const DeleteWebInterfaceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DeleteWebInterface"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deleteWebInterface"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}]}]}}]} as unknown as DocumentNode<DeleteWebInterfaceMutation, DeleteWebInterfaceMutationVariables>;
export const AcceptWebInterfaceSuggestionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"AcceptWebInterfaceSuggestion"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"checkId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"acceptWebInterfaceSuggestion"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"checkId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"checkId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}}]} as unknown as DocumentNode<AcceptWebInterfaceSuggestionMutation, AcceptWebInterfaceSuggestionMutationVariables>;
export const DismissWebInterfaceSuggestionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DismissWebInterfaceSuggestion"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"checkId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dismissWebInterfaceSuggestion"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"checkId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"checkId"}}}]}]}}]} as unknown as DocumentNode<DismissWebInterfaceSuggestionMutation, DismissWebInterfaceSuggestionMutationVariables>;
export const AcceptWebCertificateDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"AcceptWebCertificate"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"fingerprintSha256"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"acceptWebCertificate"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"fingerprintSha256"},"value":{"kind":"Variable","name":{"kind":"Name","value":"fingerprintSha256"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}}]}}]} as unknown as DocumentNode<AcceptWebCertificateMutation, AcceptWebCertificateMutationVariables>;