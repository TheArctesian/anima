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

export type BasicAuthInput = {
  password: string;
  username: string;
};

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
  warnDays?: number | null | undefined;
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
  | 'SESSION';

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

export type LoginInput = {
  password: string;
  totpCode?: string | null | undefined;
  username: string;
};

export type MetricKind =
  | 'CPU_PERCENT'
  | 'DISK_BYTES'
  | 'DISK_PERCENT'
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
  | 'NONE'
  | 'SSH';

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

export type WebInterfaceInput = {
  /** Defaults to the URL's host. */
  label?: string | null | undefined;
  openIn?: OpenIn | null | undefined;
  url: string;
};

export type LoginMutationVariables = Exact<{
  input: LoginInput;
}>;


export type LoginMutation = { login: { token: string } };

export type LogoutMutationVariables = Exact<{ [key: string]: never; }>;


export type LogoutMutation = { logout: boolean };

export type MeQueryVariables = Exact<{ [key: string]: never; }>;


export type MeQuery = { me: { username: string } };

export type HeartbeatAddedSubscriptionVariables = Exact<{ [key: string]: never; }>;


export type HeartbeatAddedSubscription = { heartbeatAdded: { checkId: string, deviceId: string, heartbeat: { at: string, status: Status, responseTimeMs: number | null } } };

export type CheckFieldsFragment = { id: string, type: CheckType, name: string, intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number, invert: boolean, paused: boolean, status: Status, certificateDaysLeft: number | null, parent: { id: string } | null, lastHeartbeat: { at: string, status: Status, responseTimeMs: number | null, message: string | null } | null, config:
    | { __typename: 'DnsConfig', name: string, recordType: DnsRecordType, resolver: string | null, expected: string | null }
    | { __typename: 'DockerConfig' }
    | { __typename: 'DomainExpiryConfig' }
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

export type DeviceChecksQueryVariables = Exact<{
  deviceId: string;
}>;


export type DeviceChecksQuery = { device: { id: string, checks: Array<{ id: string, type: CheckType, name: string, intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number, invert: boolean, paused: boolean, status: Status, certificateDaysLeft: number | null, parent: { id: string } | null, lastHeartbeat: { at: string, status: Status, responseTimeMs: number | null, message: string | null } | null, config:
        | { __typename: 'DnsConfig', name: string, recordType: DnsRecordType, resolver: string | null, expected: string | null }
        | { __typename: 'DockerConfig' }
        | { __typename: 'DomainExpiryConfig' }
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


export type CreateCheckMutation = { createCheck: { id: string, type: CheckType, name: string, intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number, invert: boolean, paused: boolean, status: Status, certificateDaysLeft: number | null, parent: { id: string } | null, lastHeartbeat: { at: string, status: Status, responseTimeMs: number | null, message: string | null } | null, config:
      | { __typename: 'DnsConfig', name: string, recordType: DnsRecordType, resolver: string | null, expected: string | null }
      | { __typename: 'DockerConfig' }
      | { __typename: 'DomainExpiryConfig' }
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


export type UpdateCheckMutation = { updateCheck: { id: string, type: CheckType, name: string, intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number, invert: boolean, paused: boolean, status: Status, certificateDaysLeft: number | null, parent: { id: string } | null, lastHeartbeat: { at: string, status: Status, responseTimeMs: number | null, message: string | null } | null, config:
      | { __typename: 'DnsConfig', name: string, recordType: DnsRecordType, resolver: string | null, expected: string | null }
      | { __typename: 'DockerConfig' }
      | { __typename: 'DomainExpiryConfig' }
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


export type SetCheckPausedMutation = { setCheckPaused: { id: string, type: CheckType, name: string, intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number, invert: boolean, paused: boolean, status: Status, certificateDaysLeft: number | null, parent: { id: string } | null, lastHeartbeat: { at: string, status: Status, responseTimeMs: number | null, message: string | null } | null, config:
      | { __typename: 'DnsConfig', name: string, recordType: DnsRecordType, resolver: string | null, expected: string | null }
      | { __typename: 'DockerConfig' }
      | { __typename: 'DomainExpiryConfig' }
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

export type DevicesQueryVariables = Exact<{
  filter?: DeviceFilter | null | undefined;
  sort?: DeviceSort | null | undefined;
  first?: number | null | undefined;
  after?: string | null | undefined;
}>;


export type DevicesQuery = { devices: { totalCount: number, pageInfo: { hasNextPage: boolean, endCursor: string | null }, edges: Array<{ node: { id: string, name: string, host: string, ip: string | null, tags: Array<string>, maintenance: boolean, status: Status, uptime: number | null, os: OperatingSystem, macAddress: string | null, metricsSource: MetricsSource, accessWarnings: Array<AccessWarning>, readings: { at: string, cpuPercent: number | null, memoryPercent: number | null, diskPercent: number | null, vramPercent: number | null, vramGpu: { index: number, name: string, vendor: GpuVendor, hasVram: boolean } | null } | null, webInterfaces: Array<{ id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null }>, rdp: { enabled: boolean }, vnc: { enabled: boolean, username: string | null }, ssh: { enabled: boolean, hostKey: { fingerprintSha256: string } | null }, checks: Array<{ id: string, heartbeats: { edges: Array<{ node: { at: string, status: Status, responseTimeMs: number | null } }> } }> } }> } };

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

export type DeviceDetailFieldsFragment = { id: string, name: string, host: string, ip: string | null, kind: string, os: OperatingSystem, tags: Array<string>, maintenance: boolean, status: Status, macAddress: string | null, metricsSource: MetricsSource, accessWarnings: Array<AccessWarning>, day: number | null, week: number | null, month: number | null, site: { id: string, name: string }, ssh: { enabled: boolean, port: number, username: string, auth: SshAuth, credentialStored: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null }, rdp: { enabled: boolean, port: number, username: string, domain: string | null, security: RdpSecurity, ignoreCertificate: boolean, resizeMethod: RdpResizeMethod, audioOut: boolean, microphone: boolean, sharedDrive: boolean, credentialStored: boolean, serverCertificate: { fingerprintSha256: string, subject: string | null, notAfter: string | null, firstSeenAt: string } | null }, vnc: { enabled: boolean, port: number, username: string | null, credentialStored: boolean }, transfer: { upload: boolean, download: boolean, copy: boolean, paste: boolean }, webInterfaces: Array<{ id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null }>, webInterfaceSuggestions: Array<{ checkId: string, url: string, label: string }> };

export type CardReadingsFragment = { at: string, cpuPercent: number | null, memoryPercent: number | null, diskPercent: number | null, vramPercent: number | null, vramGpu: { index: number, name: string, vendor: GpuVendor, hasVram: boolean } | null };

export type DeviceQueryVariables = Exact<{
  id: string;
}>;


export type DeviceQuery = { device: { id: string, name: string, host: string, ip: string | null, kind: string, os: OperatingSystem, tags: Array<string>, maintenance: boolean, status: Status, macAddress: string | null, metricsSource: MetricsSource, accessWarnings: Array<AccessWarning>, day: number | null, week: number | null, month: number | null, site: { id: string, name: string }, ssh: { enabled: boolean, port: number, username: string, auth: SshAuth, credentialStored: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null }, rdp: { enabled: boolean, port: number, username: string, domain: string | null, security: RdpSecurity, ignoreCertificate: boolean, resizeMethod: RdpResizeMethod, audioOut: boolean, microphone: boolean, sharedDrive: boolean, credentialStored: boolean, serverCertificate: { fingerprintSha256: string, subject: string | null, notAfter: string | null, firstSeenAt: string } | null }, vnc: { enabled: boolean, port: number, username: string | null, credentialStored: boolean }, transfer: { upload: boolean, download: boolean, copy: boolean, paste: boolean }, webInterfaces: Array<{ id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null }>, webInterfaceSuggestions: Array<{ checkId: string, url: string, label: string }> } | null };

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


export type CreateDeviceMutation = { createDevice: { id: string, name: string, host: string, ip: string | null, kind: string, os: OperatingSystem, tags: Array<string>, maintenance: boolean, status: Status, macAddress: string | null, metricsSource: MetricsSource, accessWarnings: Array<AccessWarning>, day: number | null, week: number | null, month: number | null, site: { id: string, name: string }, ssh: { enabled: boolean, port: number, username: string, auth: SshAuth, credentialStored: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null }, rdp: { enabled: boolean, port: number, username: string, domain: string | null, security: RdpSecurity, ignoreCertificate: boolean, resizeMethod: RdpResizeMethod, audioOut: boolean, microphone: boolean, sharedDrive: boolean, credentialStored: boolean, serverCertificate: { fingerprintSha256: string, subject: string | null, notAfter: string | null, firstSeenAt: string } | null }, vnc: { enabled: boolean, port: number, username: string | null, credentialStored: boolean }, transfer: { upload: boolean, download: boolean, copy: boolean, paste: boolean }, webInterfaces: Array<{ id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null }>, webInterfaceSuggestions: Array<{ checkId: string, url: string, label: string }> } };

export type UpdateDeviceMutationVariables = Exact<{
  id: string;
  input: UpdateDeviceInput;
}>;


export type UpdateDeviceMutation = { updateDevice: { id: string, name: string, host: string, ip: string | null, kind: string, os: OperatingSystem, tags: Array<string>, maintenance: boolean, status: Status, macAddress: string | null, metricsSource: MetricsSource, accessWarnings: Array<AccessWarning>, day: number | null, week: number | null, month: number | null, site: { id: string, name: string }, ssh: { enabled: boolean, port: number, username: string, auth: SshAuth, credentialStored: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null }, rdp: { enabled: boolean, port: number, username: string, domain: string | null, security: RdpSecurity, ignoreCertificate: boolean, resizeMethod: RdpResizeMethod, audioOut: boolean, microphone: boolean, sharedDrive: boolean, credentialStored: boolean, serverCertificate: { fingerprintSha256: string, subject: string | null, notAfter: string | null, firstSeenAt: string } | null }, vnc: { enabled: boolean, port: number, username: string | null, credentialStored: boolean }, transfer: { upload: boolean, download: boolean, copy: boolean, paste: boolean }, webInterfaces: Array<{ id: string, deviceId: string, label: string, url: string, openIn: OpenIn, pinnedFingerprintSha256: string | null, pinnedAt: string | null }>, webInterfaceSuggestions: Array<{ checkId: string, url: string, label: string }> } };

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

export const CheckFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CheckFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Check"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"type"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}},{"kind":"Field","name":{"kind":"Name","value":"invert"}},{"kind":"Field","name":{"kind":"Name","value":"paused"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"certificateDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"lastHeartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}},{"kind":"Field","name":{"kind":"Name","value":"config"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"IcmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}},{"kind":"Field","name":{"kind":"Name","value":"ipVersion"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TcpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TlsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"serverName"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"HttpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headers"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"body"}},{"kind":"Field","name":{"kind":"Name","value":"maxRedirects"}},{"kind":"Field","name":{"kind":"Name","value":"acceptedStatus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}}]}},{"kind":"Field","name":{"kind":"Name","value":"keyword"}},{"kind":"Field","name":{"kind":"Name","value":"jsonQuery"}},{"kind":"Field","name":{"kind":"Name","value":"jsonExpected"}},{"kind":"Field","name":{"kind":"Name","value":"authKind"}},{"kind":"Field","name":{"kind":"Name","value":"verifyTls"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DnsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"recordType"}},{"kind":"Field","name":{"kind":"Name","value":"resolver"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"PushConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pushPath"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"GroupConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"children"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}}]}}]}}]} as unknown as DocumentNode<CheckFieldsFragment, unknown>;
export const DockerCheckConfigFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DockerCheckConfig"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DockerConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"Field","name":{"kind":"Name","value":"container"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]} as unknown as DocumentNode<DockerCheckConfigFragment, unknown>;
export const SystemdCheckConfigFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"SystemdCheckConfig"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"SystemdConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"Field","name":{"kind":"Name","value":"unit"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]} as unknown as DocumentNode<SystemdCheckConfigFragment, unknown>;
export const SnmpCheckConfigFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"SnmpCheckConfig"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"SnmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"version"}},{"kind":"Field","name":{"kind":"Name","value":"oid"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]} as unknown as DocumentNode<SnmpCheckConfigFragment, unknown>;
export const TailscalePingCheckConfigFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"TailscalePingCheckConfig"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TailscalePingConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"Field","name":{"kind":"Name","value":"target"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}}]} as unknown as DocumentNode<TailscalePingCheckConfigFragment, unknown>;
export const WebInterfaceFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}}]} as unknown as DocumentNode<WebInterfaceFieldsFragment, unknown>;
export const WebInterfaceSuggestionFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceSuggestion"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkId"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"label"}}]}}]} as unknown as DocumentNode<WebInterfaceSuggestionFieldsFragment, unknown>;
export const DeviceDetailFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeviceDetailFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Device"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"auth"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"macAddress"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"accessWarnings"}},{"kind":"Field","name":{"kind":"Name","value":"rdp"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"security"}},{"kind":"Field","name":{"kind":"Name","value":"ignoreCertificate"}},{"kind":"Field","name":{"kind":"Name","value":"resizeMethod"}},{"kind":"Field","name":{"kind":"Name","value":"audioOut"}},{"kind":"Field","name":{"kind":"Name","value":"microphone"}},{"kind":"Field","name":{"kind":"Name","value":"sharedDrive"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"serverCertificate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"subject"}},{"kind":"Field","name":{"kind":"Name","value":"notAfter"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"vnc"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}},{"kind":"Field","name":{"kind":"Name","value":"transfer"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"upload"}},{"kind":"Field","name":{"kind":"Name","value":"download"}},{"kind":"Field","name":{"kind":"Name","value":"copy"}},{"kind":"Field","name":{"kind":"Name","value":"paste"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaceSuggestions"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"}}]}},{"kind":"Field","alias":{"kind":"Name","value":"day"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","alias":{"kind":"Name","value":"week"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"WEEK"}}]},{"kind":"Field","alias":{"kind":"Name","value":"month"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"MONTH"}}]}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceSuggestion"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkId"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"label"}}]}}]} as unknown as DocumentNode<DeviceDetailFieldsFragment, unknown>;
export const CardReadingsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CardReadings"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Readings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"cpuPercent"}},{"kind":"Field","name":{"kind":"Name","value":"memoryPercent"}},{"kind":"Field","name":{"kind":"Name","value":"diskPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramGpu"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"index"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"vendor"}},{"kind":"Field","name":{"kind":"Name","value":"hasVram"}}]}}]}}]} as unknown as DocumentNode<CardReadingsFragment, unknown>;
export const LoginDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"Login"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"LoginInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"login"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"token"}}]}}]}}]} as unknown as DocumentNode<LoginMutation, LoginMutationVariables>;
export const LogoutDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"Logout"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"logout"}}]}}]} as unknown as DocumentNode<LogoutMutation, LogoutMutationVariables>;
export const MeDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Me"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"me"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"username"}}]}}]}}]} as unknown as DocumentNode<MeQuery, MeQueryVariables>;
export const HeartbeatAddedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"HeartbeatAdded"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"heartbeatAdded"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"NullValue"}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"heartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}}]}}]}}]}}]} as unknown as DocumentNode<HeartbeatAddedSubscription, HeartbeatAddedSubscriptionVariables>;
export const DeviceChecksDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"DeviceChecks"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"device"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"checks"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CheckFields"}}]}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CheckFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Check"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"type"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}},{"kind":"Field","name":{"kind":"Name","value":"invert"}},{"kind":"Field","name":{"kind":"Name","value":"paused"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"certificateDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"lastHeartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}},{"kind":"Field","name":{"kind":"Name","value":"config"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"IcmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}},{"kind":"Field","name":{"kind":"Name","value":"ipVersion"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TcpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TlsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"serverName"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"HttpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headers"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"body"}},{"kind":"Field","name":{"kind":"Name","value":"maxRedirects"}},{"kind":"Field","name":{"kind":"Name","value":"acceptedStatus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}}]}},{"kind":"Field","name":{"kind":"Name","value":"keyword"}},{"kind":"Field","name":{"kind":"Name","value":"jsonQuery"}},{"kind":"Field","name":{"kind":"Name","value":"jsonExpected"}},{"kind":"Field","name":{"kind":"Name","value":"authKind"}},{"kind":"Field","name":{"kind":"Name","value":"verifyTls"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DnsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"recordType"}},{"kind":"Field","name":{"kind":"Name","value":"resolver"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"PushConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pushPath"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"GroupConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"children"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}}]}}]}}]} as unknown as DocumentNode<DeviceChecksQuery, DeviceChecksQueryVariables>;
export const CheckHeartbeatsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"CheckHeartbeats"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"first"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"check"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"heartbeats"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"Variable","name":{"kind":"Name","value":"first"}}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}}]}}]}}]}}]}}]} as unknown as DocumentNode<CheckHeartbeatsQuery, CheckHeartbeatsQueryVariables>;
export const CreateCheckDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateCheck"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"CreateCheckInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createCheck"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CheckFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CheckFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Check"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"type"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}},{"kind":"Field","name":{"kind":"Name","value":"invert"}},{"kind":"Field","name":{"kind":"Name","value":"paused"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"certificateDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"lastHeartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}},{"kind":"Field","name":{"kind":"Name","value":"config"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"IcmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}},{"kind":"Field","name":{"kind":"Name","value":"ipVersion"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TcpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TlsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"serverName"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"HttpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headers"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"body"}},{"kind":"Field","name":{"kind":"Name","value":"maxRedirects"}},{"kind":"Field","name":{"kind":"Name","value":"acceptedStatus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}}]}},{"kind":"Field","name":{"kind":"Name","value":"keyword"}},{"kind":"Field","name":{"kind":"Name","value":"jsonQuery"}},{"kind":"Field","name":{"kind":"Name","value":"jsonExpected"}},{"kind":"Field","name":{"kind":"Name","value":"authKind"}},{"kind":"Field","name":{"kind":"Name","value":"verifyTls"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DnsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"recordType"}},{"kind":"Field","name":{"kind":"Name","value":"resolver"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"PushConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pushPath"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"GroupConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"children"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}}]}}]}}]} as unknown as DocumentNode<CreateCheckMutation, CreateCheckMutationVariables>;
export const UpdateCheckDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"UpdateCheck"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"UpdateCheckInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"updateCheck"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CheckFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CheckFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Check"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"type"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}},{"kind":"Field","name":{"kind":"Name","value":"invert"}},{"kind":"Field","name":{"kind":"Name","value":"paused"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"certificateDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"lastHeartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}},{"kind":"Field","name":{"kind":"Name","value":"config"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"IcmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}},{"kind":"Field","name":{"kind":"Name","value":"ipVersion"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TcpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TlsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"serverName"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"HttpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headers"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"body"}},{"kind":"Field","name":{"kind":"Name","value":"maxRedirects"}},{"kind":"Field","name":{"kind":"Name","value":"acceptedStatus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}}]}},{"kind":"Field","name":{"kind":"Name","value":"keyword"}},{"kind":"Field","name":{"kind":"Name","value":"jsonQuery"}},{"kind":"Field","name":{"kind":"Name","value":"jsonExpected"}},{"kind":"Field","name":{"kind":"Name","value":"authKind"}},{"kind":"Field","name":{"kind":"Name","value":"verifyTls"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DnsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"recordType"}},{"kind":"Field","name":{"kind":"Name","value":"resolver"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"PushConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pushPath"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"GroupConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"children"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}}]}}]}}]} as unknown as DocumentNode<UpdateCheckMutation, UpdateCheckMutationVariables>;
export const DeleteCheckDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DeleteCheck"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deleteCheck"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}]}]}}]} as unknown as DocumentNode<DeleteCheckMutation, DeleteCheckMutationVariables>;
export const SetCheckPausedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SetCheckPaused"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"paused"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Boolean"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"setCheckPaused"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"paused"},"value":{"kind":"Variable","name":{"kind":"Name","value":"paused"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CheckFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CheckFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Check"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"type"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}},{"kind":"Field","name":{"kind":"Name","value":"invert"}},{"kind":"Field","name":{"kind":"Name","value":"paused"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"certificateDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"lastHeartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}},{"kind":"Field","name":{"kind":"Name","value":"config"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"IcmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}},{"kind":"Field","name":{"kind":"Name","value":"ipVersion"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TcpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TlsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"serverName"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"HttpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headers"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"body"}},{"kind":"Field","name":{"kind":"Name","value":"maxRedirects"}},{"kind":"Field","name":{"kind":"Name","value":"acceptedStatus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}}]}},{"kind":"Field","name":{"kind":"Name","value":"keyword"}},{"kind":"Field","name":{"kind":"Name","value":"jsonQuery"}},{"kind":"Field","name":{"kind":"Name","value":"jsonExpected"}},{"kind":"Field","name":{"kind":"Name","value":"authKind"}},{"kind":"Field","name":{"kind":"Name","value":"verifyTls"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DnsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"recordType"}},{"kind":"Field","name":{"kind":"Name","value":"resolver"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"PushConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pushPath"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"GroupConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"children"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}}]}}]}}]} as unknown as DocumentNode<SetCheckPausedMutation, SetCheckPausedMutationVariables>;
export const DevicesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Devices"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"filter"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"DeviceFilter"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"sort"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"DeviceSort"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"first"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"devices"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"filter"},"value":{"kind":"Variable","name":{"kind":"Name","value":"filter"}}},{"kind":"Argument","name":{"kind":"Name","value":"sort"},"value":{"kind":"Variable","name":{"kind":"Name","value":"sort"}}},{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"Variable","name":{"kind":"Name","value":"first"}}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"totalCount"}},{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"macAddress"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"readings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CardReadings"}}]}},{"kind":"Field","name":{"kind":"Name","value":"accessWarnings"}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"rdp"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}}]}},{"kind":"Field","name":{"kind":"Name","value":"vnc"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"username"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"checks"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"heartbeats"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"IntValue","value":"50"}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}}]}}]}}]}}]}}]}}]}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CardReadings"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Readings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"cpuPercent"}},{"kind":"Field","name":{"kind":"Name","value":"memoryPercent"}},{"kind":"Field","name":{"kind":"Name","value":"diskPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramGpu"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"index"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"vendor"}},{"kind":"Field","name":{"kind":"Name","value":"hasVram"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}}]} as unknown as DocumentNode<DevicesQuery, DevicesQueryVariables>;
export const DeviceHostsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"DeviceHosts"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"devices"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"IntValue","value":"200"}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"host"}}]}}]}}]}}]}}]} as unknown as DocumentNode<DeviceHostsQuery, DeviceHostsQueryVariables>;
export const TagsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Tags"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"tags"}}]}}]} as unknown as DocumentNode<TagsQuery, TagsQueryVariables>;
export const DeviceStatusChangedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"DeviceStatusChanged"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deviceStatusChanged"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"NullValue"}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"at"}}]}}]}}]} as unknown as DocumentNode<DeviceStatusChangedSubscription, DeviceStatusChangedSubscriptionVariables>;
export const CardReadingsAddedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"CardReadingsAdded"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"metricsAdded"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"NullValue"}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"readings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"CardReadings"}}]}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CardReadings"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Readings"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"cpuPercent"}},{"kind":"Field","name":{"kind":"Name","value":"memoryPercent"}},{"kind":"Field","name":{"kind":"Name","value":"diskPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramGpu"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"index"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"vendor"}},{"kind":"Field","name":{"kind":"Name","value":"hasVram"}}]}}]}}]} as unknown as DocumentNode<CardReadingsAddedSubscription, CardReadingsAddedSubscriptionVariables>;
export const DeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Device"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"device"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeviceDetailFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceSuggestion"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkId"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"label"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeviceDetailFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Device"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"auth"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"macAddress"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"accessWarnings"}},{"kind":"Field","name":{"kind":"Name","value":"rdp"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"security"}},{"kind":"Field","name":{"kind":"Name","value":"ignoreCertificate"}},{"kind":"Field","name":{"kind":"Name","value":"resizeMethod"}},{"kind":"Field","name":{"kind":"Name","value":"audioOut"}},{"kind":"Field","name":{"kind":"Name","value":"microphone"}},{"kind":"Field","name":{"kind":"Name","value":"sharedDrive"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"serverCertificate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"subject"}},{"kind":"Field","name":{"kind":"Name","value":"notAfter"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"vnc"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}},{"kind":"Field","name":{"kind":"Name","value":"transfer"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"upload"}},{"kind":"Field","name":{"kind":"Name","value":"download"}},{"kind":"Field","name":{"kind":"Name","value":"copy"}},{"kind":"Field","name":{"kind":"Name","value":"paste"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaceSuggestions"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"}}]}},{"kind":"Field","alias":{"kind":"Name","value":"day"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","alias":{"kind":"Name","value":"week"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"WEEK"}}]},{"kind":"Field","alias":{"kind":"Name","value":"month"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"MONTH"}}]}]}}]} as unknown as DocumentNode<DeviceQuery, DeviceQueryVariables>;
export const SitesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Sites"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sites"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"subnet"}}]}}]}}]} as unknown as DocumentNode<SitesQuery, SitesQueryVariables>;
export const CreateSiteDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateSite"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"name"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"subnet"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Cidr"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createSite"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"name"},"value":{"kind":"Variable","name":{"kind":"Name","value":"name"}}},{"kind":"Argument","name":{"kind":"Name","value":"subnet"},"value":{"kind":"Variable","name":{"kind":"Name","value":"subnet"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"subnet"}}]}}]}}]} as unknown as DocumentNode<CreateSiteMutation, CreateSiteMutationVariables>;
export const CreateDeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateDevice"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"CreateDeviceInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createDevice"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeviceDetailFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceSuggestion"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkId"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"label"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeviceDetailFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Device"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"auth"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"macAddress"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"accessWarnings"}},{"kind":"Field","name":{"kind":"Name","value":"rdp"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"security"}},{"kind":"Field","name":{"kind":"Name","value":"ignoreCertificate"}},{"kind":"Field","name":{"kind":"Name","value":"resizeMethod"}},{"kind":"Field","name":{"kind":"Name","value":"audioOut"}},{"kind":"Field","name":{"kind":"Name","value":"microphone"}},{"kind":"Field","name":{"kind":"Name","value":"sharedDrive"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"serverCertificate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"subject"}},{"kind":"Field","name":{"kind":"Name","value":"notAfter"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"vnc"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}},{"kind":"Field","name":{"kind":"Name","value":"transfer"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"upload"}},{"kind":"Field","name":{"kind":"Name","value":"download"}},{"kind":"Field","name":{"kind":"Name","value":"copy"}},{"kind":"Field","name":{"kind":"Name","value":"paste"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaceSuggestions"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"}}]}},{"kind":"Field","alias":{"kind":"Name","value":"day"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","alias":{"kind":"Name","value":"week"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"WEEK"}}]},{"kind":"Field","alias":{"kind":"Name","value":"month"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"MONTH"}}]}]}}]} as unknown as DocumentNode<CreateDeviceMutation, CreateDeviceMutationVariables>;
export const UpdateDeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"UpdateDevice"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"UpdateDeviceInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"updateDevice"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeviceDetailFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceSuggestion"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"checkId"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"label"}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeviceDetailFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Device"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"auth"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"macAddress"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"accessWarnings"}},{"kind":"Field","name":{"kind":"Name","value":"rdp"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"domain"}},{"kind":"Field","name":{"kind":"Name","value":"security"}},{"kind":"Field","name":{"kind":"Name","value":"ignoreCertificate"}},{"kind":"Field","name":{"kind":"Name","value":"resizeMethod"}},{"kind":"Field","name":{"kind":"Name","value":"audioOut"}},{"kind":"Field","name":{"kind":"Name","value":"microphone"}},{"kind":"Field","name":{"kind":"Name","value":"sharedDrive"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"serverCertificate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"subject"}},{"kind":"Field","name":{"kind":"Name","value":"notAfter"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"vnc"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}},{"kind":"Field","name":{"kind":"Name","value":"transfer"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"upload"}},{"kind":"Field","name":{"kind":"Name","value":"download"}},{"kind":"Field","name":{"kind":"Name","value":"copy"}},{"kind":"Field","name":{"kind":"Name","value":"paste"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}},{"kind":"Field","name":{"kind":"Name","value":"webInterfaceSuggestions"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceSuggestionFields"}}]}},{"kind":"Field","alias":{"kind":"Name","value":"day"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","alias":{"kind":"Name","value":"week"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"WEEK"}}]},{"kind":"Field","alias":{"kind":"Name","value":"month"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"MONTH"}}]}]}}]} as unknown as DocumentNode<UpdateDeviceMutation, UpdateDeviceMutationVariables>;
export const DeleteDeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DeleteDevice"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deleteDevice"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}]}]}}]} as unknown as DocumentNode<DeleteDeviceMutation, DeleteDeviceMutationVariables>;
export const SetSshCredentialDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SetSshCredential"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"credential"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"SshCredentialInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"setSshCredential"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"credential"},"value":{"kind":"Variable","name":{"kind":"Name","value":"credential"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}}]}}]} as unknown as DocumentNode<SetSshCredentialMutation, SetSshCredentialMutationVariables>;
export const ClearSshCredentialDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ClearSshCredential"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"clearSshCredential"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}}]}}]} as unknown as DocumentNode<ClearSshCredentialMutation, ClearSshCredentialMutationVariables>;
export const ClearRdpCredentialDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ClearRdpCredential"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"clearRdpCredential"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}}]}}]} as unknown as DocumentNode<ClearRdpCredentialMutation, ClearRdpCredentialMutationVariables>;
export const ClearVncCredentialDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ClearVncCredential"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"clearVncCredential"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}}]}}]} as unknown as DocumentNode<ClearVncCredentialMutation, ClearVncCredentialMutationVariables>;
export const EventsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Events"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"filter"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"EventFilter"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"first"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"events"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"filter"},"value":{"kind":"Variable","name":{"kind":"Name","value":"filter"}}},{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"Variable","name":{"kind":"Name","value":"first"}}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"severity"}},{"kind":"Field","name":{"kind":"Name","value":"source"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceName"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}}]}}]}}]}}]} as unknown as DocumentNode<EventsQuery, EventsQueryVariables>;
export const EventAddedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"EventAdded"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"minSeverity"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Severity"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"eventAdded"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"minSeverity"},"value":{"kind":"Variable","name":{"kind":"Name","value":"minSeverity"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"severity"}},{"kind":"Field","name":{"kind":"Name","value":"source"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceName"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}}]}}]} as unknown as DocumentNode<EventAddedSubscription, EventAddedSubscriptionVariables>;
export const DeviceMetricsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"DeviceMetrics"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"range"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"MetricRange"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"device"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"metricsSource"}},{"kind":"Field","name":{"kind":"Name","value":"gpus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"index"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"vendor"}},{"kind":"Field","name":{"kind":"Name","value":"hasVram"}}]}},{"kind":"Field","name":{"kind":"Name","value":"readings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"cpuPercent"}},{"kind":"Field","name":{"kind":"Name","value":"memoryPercent"}},{"kind":"Field","name":{"kind":"Name","value":"diskPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramGpu"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"index"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"vendor"}},{"kind":"Field","name":{"kind":"Name","value":"hasVram"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"metrics"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"range"},"value":{"kind":"Variable","name":{"kind":"Name","value":"range"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"gpuIndex"}},{"kind":"Field","name":{"kind":"Name","value":"points"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"value"}},{"kind":"Field","name":{"kind":"Name","value":"min"}},{"kind":"Field","name":{"kind":"Name","value":"max"}}]}}]}}]}}]}}]} as unknown as DocumentNode<DeviceMetricsQuery, DeviceMetricsQueryVariables>;
export const MetricsAddedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"MetricsAdded"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"metricsAdded"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"samples"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"readings"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"cpuPercent"}},{"kind":"Field","name":{"kind":"Name","value":"memoryPercent"}},{"kind":"Field","name":{"kind":"Name","value":"diskPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramPercent"}},{"kind":"Field","name":{"kind":"Name","value":"vramGpu"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"index"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"vendor"}},{"kind":"Field","name":{"kind":"Name","value":"hasVram"}}]}}]}}]}}]}}]} as unknown as DocumentNode<MetricsAddedSubscription, MetricsAddedSubscriptionVariables>;
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
export const CreateWebInterfaceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateWebInterface"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createWebInterface"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}}]} as unknown as DocumentNode<CreateWebInterfaceMutation, CreateWebInterfaceMutationVariables>;
export const UpdateWebInterfaceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"UpdateWebInterface"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterfaceInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"updateWebInterface"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}}]} as unknown as DocumentNode<UpdateWebInterfaceMutation, UpdateWebInterfaceMutationVariables>;
export const DeleteWebInterfaceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DeleteWebInterface"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deleteWebInterface"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}]}]}}]} as unknown as DocumentNode<DeleteWebInterfaceMutation, DeleteWebInterfaceMutationVariables>;
export const AcceptWebInterfaceSuggestionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"AcceptWebInterfaceSuggestion"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"checkId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"acceptWebInterfaceSuggestion"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"checkId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"checkId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"WebInterfaceFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"WebInterfaceFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"WebInterface"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"label"}},{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"openIn"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}}]} as unknown as DocumentNode<AcceptWebInterfaceSuggestionMutation, AcceptWebInterfaceSuggestionMutationVariables>;
export const DismissWebInterfaceSuggestionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DismissWebInterfaceSuggestion"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"checkId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dismissWebInterfaceSuggestion"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"checkId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"checkId"}}}]}]}}]} as unknown as DocumentNode<DismissWebInterfaceSuggestionMutation, DismissWebInterfaceSuggestionMutationVariables>;
export const AcceptWebCertificateDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"AcceptWebCertificate"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"fingerprintSha256"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"acceptWebCertificate"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"fingerprintSha256"},"value":{"kind":"Variable","name":{"kind":"Name","value":"fingerprintSha256"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedFingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"pinnedAt"}}]}}]}}]} as unknown as DocumentNode<AcceptWebCertificateMutation, AcceptWebCertificateMutationVariables>;