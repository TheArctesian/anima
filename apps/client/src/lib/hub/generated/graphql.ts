/* eslint-disable */
/** Internal type. DO NOT USE DIRECTLY. */
type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
/** Internal type. DO NOT USE DIRECTLY. */
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
import type { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';
export type BasicAuthInput = {
  password: string;
  username: string;
};

export type CheckConfigInput =
  {   dns: DnsConfigInput; group?: never; http?: never; icmp?: never; push?: never; tcp?: never; tls?: never; }
  |  { dns?: never;   group: GroupConfigInput; http?: never; icmp?: never; push?: never; tcp?: never; tls?: never; }
  |  { dns?: never; group?: never;   http: HttpConfigInput; icmp?: never; push?: never; tcp?: never; tls?: never; }
  |  { dns?: never; group?: never; http?: never;   icmp: IcmpConfigInput; push?: never; tcp?: never; tls?: never; }
  |  { dns?: never; group?: never; http?: never; icmp?: never;   push: boolean; tcp?: never; tls?: never; }
  |  { dns?: never; group?: never; http?: never; icmp?: never; push?: never;   tcp: TcpConfigInput; tls?: never; }
  |  { dns?: never; group?: never; http?: never; icmp?: never; push?: never; tcp?: never;   tls: TlsConfigInput; };

/** Identifiers are fixed forever (CHK-009). */
export type CheckType =
  | 'DNS'
  | 'GROUP'
  | 'HTTP'
  | 'ICMP'
  | 'PUSH'
  | 'TCP'
  | 'TLS';

export type ConnectionTestInput = {
  credential?: SshCredentialInput | null | undefined;
  /** Test a saved device, or test unsaved values from the form (DEV-013). */
  deviceId?: string | null | undefined;
  host?: string | null | undefined;
  ssh?: SshSettingsInput | null | undefined;
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
  name: string;
  os?: OperatingSystem | null | undefined;
  siteId: string;
  ssh?: SshSettingsInput | null | undefined;
  tags?: Array<string> | null | undefined;
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
  | 'SESSION';

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

export type OperatingSystem =
  | 'LINUX'
  | 'MACOS'
  | 'OTHER'
  | 'WINDOWS';

export type PrivateKeyInput = {
  key: string;
  passphrase?: string | null | undefined;
};

export type Protocol =
  | 'SSH';

export type SessionEndReason =
  | 'AUTH_FAILED'
  | 'GATEWAY_ERROR'
  | 'HOST_KEY_MISMATCH'
  | 'HUB_SHUTDOWN'
  | 'OPERATOR'
  /** The client sent an instruction that does not decode (GW-011, R-034). */
  | 'PROTOCOL_ERROR'
  | 'REMOTE_CLOSED'
  | 'TIMEOUT';

export type Severity =
  | 'ERROR'
  | 'INFO'
  | 'WARNING';

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

export type TcpConfigInput = {
  port: number;
  timeoutMs?: number | null | undefined;
};

export type TlsConfigInput = {
  port?: number | null | undefined;
  serverName?: string | null | undefined;
  timeoutMs?: number | null | undefined;
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
  maintenance?: boolean | null | undefined;
  name?: string | null | undefined;
  os?: OperatingSystem | null | undefined;
  siteId?: string | null | undefined;
  ssh?: SshSettingsInput | null | undefined;
  tags?: Array<string> | null | undefined;
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
    | { __typename: 'GroupConfig', children: Array<{ id: string, name: string }> }
    | { __typename: 'HttpConfig', url: string, method: HttpMethod, body: string | null, maxRedirects: number, keyword: string | null, jsonQuery: string | null, jsonExpected: string | null, authKind: HttpAuthKind, verifyTls: boolean, timeoutMs: number, headers: Array<{ name: string, value: string | null }>, acceptedStatus: Array<{ from: number, to: number }> }
    | { __typename: 'IcmpConfig', timeoutMs: number, ipVersion: IpVersion }
    | { __typename: 'PushConfig', pushPath: string }
    | { __typename: 'TcpConfig', port: number, timeoutMs: number }
    | { __typename: 'TlsConfig', port: number, serverName: string | null, timeoutMs: number }
   };

export type DeviceChecksQueryVariables = Exact<{
  deviceId: string;
}>;


export type DeviceChecksQuery = { device: { id: string, checks: Array<{ id: string, type: CheckType, name: string, intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number, invert: boolean, paused: boolean, status: Status, certificateDaysLeft: number | null, parent: { id: string } | null, lastHeartbeat: { at: string, status: Status, responseTimeMs: number | null, message: string | null } | null, config:
        | { __typename: 'DnsConfig', name: string, recordType: DnsRecordType, resolver: string | null, expected: string | null }
        | { __typename: 'GroupConfig', children: Array<{ id: string, name: string }> }
        | { __typename: 'HttpConfig', url: string, method: HttpMethod, body: string | null, maxRedirects: number, keyword: string | null, jsonQuery: string | null, jsonExpected: string | null, authKind: HttpAuthKind, verifyTls: boolean, timeoutMs: number, headers: Array<{ name: string, value: string | null }>, acceptedStatus: Array<{ from: number, to: number }> }
        | { __typename: 'IcmpConfig', timeoutMs: number, ipVersion: IpVersion }
        | { __typename: 'PushConfig', pushPath: string }
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
      | { __typename: 'GroupConfig', children: Array<{ id: string, name: string }> }
      | { __typename: 'HttpConfig', url: string, method: HttpMethod, body: string | null, maxRedirects: number, keyword: string | null, jsonQuery: string | null, jsonExpected: string | null, authKind: HttpAuthKind, verifyTls: boolean, timeoutMs: number, headers: Array<{ name: string, value: string | null }>, acceptedStatus: Array<{ from: number, to: number }> }
      | { __typename: 'IcmpConfig', timeoutMs: number, ipVersion: IpVersion }
      | { __typename: 'PushConfig', pushPath: string }
      | { __typename: 'TcpConfig', port: number, timeoutMs: number }
      | { __typename: 'TlsConfig', port: number, serverName: string | null, timeoutMs: number }
     } };

export type UpdateCheckMutationVariables = Exact<{
  id: string;
  input: UpdateCheckInput;
}>;


export type UpdateCheckMutation = { updateCheck: { id: string, type: CheckType, name: string, intervalSeconds: number, retries: number, retryIntervalSeconds: number, degradedThresholdMs: number, invert: boolean, paused: boolean, status: Status, certificateDaysLeft: number | null, parent: { id: string } | null, lastHeartbeat: { at: string, status: Status, responseTimeMs: number | null, message: string | null } | null, config:
      | { __typename: 'DnsConfig', name: string, recordType: DnsRecordType, resolver: string | null, expected: string | null }
      | { __typename: 'GroupConfig', children: Array<{ id: string, name: string }> }
      | { __typename: 'HttpConfig', url: string, method: HttpMethod, body: string | null, maxRedirects: number, keyword: string | null, jsonQuery: string | null, jsonExpected: string | null, authKind: HttpAuthKind, verifyTls: boolean, timeoutMs: number, headers: Array<{ name: string, value: string | null }>, acceptedStatus: Array<{ from: number, to: number }> }
      | { __typename: 'IcmpConfig', timeoutMs: number, ipVersion: IpVersion }
      | { __typename: 'PushConfig', pushPath: string }
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
      | { __typename: 'GroupConfig', children: Array<{ id: string, name: string }> }
      | { __typename: 'HttpConfig', url: string, method: HttpMethod, body: string | null, maxRedirects: number, keyword: string | null, jsonQuery: string | null, jsonExpected: string | null, authKind: HttpAuthKind, verifyTls: boolean, timeoutMs: number, headers: Array<{ name: string, value: string | null }>, acceptedStatus: Array<{ from: number, to: number }> }
      | { __typename: 'IcmpConfig', timeoutMs: number, ipVersion: IpVersion }
      | { __typename: 'PushConfig', pushPath: string }
      | { __typename: 'TcpConfig', port: number, timeoutMs: number }
      | { __typename: 'TlsConfig', port: number, serverName: string | null, timeoutMs: number }
     } };

export type DevicesQueryVariables = Exact<{
  filter?: DeviceFilter | null | undefined;
  sort?: DeviceSort | null | undefined;
  first?: number | null | undefined;
  after?: string | null | undefined;
}>;


export type DevicesQuery = { devices: { totalCount: number, pageInfo: { hasNextPage: boolean, endCursor: string | null }, edges: Array<{ node: { id: string, name: string, host: string, ip: string | null, tags: Array<string>, maintenance: boolean, status: Status, uptime: number | null, ssh: { enabled: boolean, hostKey: { fingerprintSha256: string } | null }, checks: Array<{ id: string, heartbeats: { edges: Array<{ node: { at: string, status: Status, responseTimeMs: number | null } }> } }> } }> } };

export type DeviceHostsQueryVariables = Exact<{
  after?: string | null | undefined;
}>;


export type DeviceHostsQuery = { devices: { pageInfo: { hasNextPage: boolean, endCursor: string | null }, edges: Array<{ node: { host: string } }> } };

export type TagsQueryVariables = Exact<{ [key: string]: never; }>;


export type TagsQuery = { tags: Array<string> };

export type DeviceStatusChangedSubscriptionVariables = Exact<{ [key: string]: never; }>;


export type DeviceStatusChangedSubscription = { deviceStatusChanged: { deviceId: string, status: Status, at: string } };

export type DeviceDetailFieldsFragment = { id: string, name: string, host: string, ip: string | null, kind: string, os: OperatingSystem, tags: Array<string>, maintenance: boolean, status: Status, day: number | null, week: number | null, month: number | null, site: { id: string, name: string }, ssh: { enabled: boolean, port: number, username: string, auth: SshAuth, credentialStored: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null } };

export type DeviceQueryVariables = Exact<{
  id: string;
}>;


export type DeviceQuery = { device: { id: string, name: string, host: string, ip: string | null, kind: string, os: OperatingSystem, tags: Array<string>, maintenance: boolean, status: Status, day: number | null, week: number | null, month: number | null, site: { id: string, name: string }, ssh: { enabled: boolean, port: number, username: string, auth: SshAuth, credentialStored: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null } } | null };

export type SitesQueryVariables = Exact<{ [key: string]: never; }>;


export type SitesQuery = { sites: Array<{ id: string, name: string }> };

export type CreateDeviceMutationVariables = Exact<{
  input: CreateDeviceInput;
}>;


export type CreateDeviceMutation = { createDevice: { id: string, name: string, host: string, ip: string | null, kind: string, os: OperatingSystem, tags: Array<string>, maintenance: boolean, status: Status, day: number | null, week: number | null, month: number | null, site: { id: string, name: string }, ssh: { enabled: boolean, port: number, username: string, auth: SshAuth, credentialStored: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null } } };

export type UpdateDeviceMutationVariables = Exact<{
  id: string;
  input: UpdateDeviceInput;
}>;


export type UpdateDeviceMutation = { updateDevice: { id: string, name: string, host: string, ip: string | null, kind: string, os: OperatingSystem, tags: Array<string>, maintenance: boolean, status: Status, day: number | null, week: number | null, month: number | null, site: { id: string, name: string }, ssh: { enabled: boolean, port: number, username: string, auth: SshAuth, credentialStored: boolean, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null } } };

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

export type StartSessionMutationVariables = Exact<{
  input: StartSessionInput;
}>;


export type StartSessionMutation = { startSession: { sessionId: string, token: string, tunnelPath: string, expiresAt: string } };

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

export type DeviceSessionsQueryVariables = Exact<{
  deviceId: string;
  first?: number | null | undefined;
  after?: string | null | undefined;
}>;


export type DeviceSessionsQuery = { sessions: { pageInfo: { hasNextPage: boolean, endCursor: string | null }, edges: Array<{ node: { id: string, protocol: Protocol, operatorName: string, startedAt: string, endedAt: string | null, endReason: SessionEndReason | null } }> } };

export const CheckFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"CheckFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Check"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}}]}},{"kind":"Field","name":{"kind":"Name","value":"type"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"intervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"retries"}},{"kind":"Field","name":{"kind":"Name","value":"retryIntervalSeconds"}},{"kind":"Field","name":{"kind":"Name","value":"degradedThresholdMs"}},{"kind":"Field","name":{"kind":"Name","value":"invert"}},{"kind":"Field","name":{"kind":"Name","value":"paused"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"certificateDaysLeft"}},{"kind":"Field","name":{"kind":"Name","value":"lastHeartbeat"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}},{"kind":"Field","name":{"kind":"Name","value":"config"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"__typename"}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"IcmpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}},{"kind":"Field","name":{"kind":"Name","value":"ipVersion"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TcpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"TlsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"serverName"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"HttpConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"url"}},{"kind":"Field","name":{"kind":"Name","value":"method"}},{"kind":"Field","name":{"kind":"Name","value":"headers"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"value"}}]}},{"kind":"Field","name":{"kind":"Name","value":"body"}},{"kind":"Field","name":{"kind":"Name","value":"maxRedirects"}},{"kind":"Field","name":{"kind":"Name","value":"acceptedStatus"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"to"}}]}},{"kind":"Field","name":{"kind":"Name","value":"keyword"}},{"kind":"Field","name":{"kind":"Name","value":"jsonQuery"}},{"kind":"Field","name":{"kind":"Name","value":"jsonExpected"}},{"kind":"Field","name":{"kind":"Name","value":"authKind"}},{"kind":"Field","name":{"kind":"Name","value":"verifyTls"}},{"kind":"Field","name":{"kind":"Name","value":"timeoutMs"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"DnsConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"recordType"}},{"kind":"Field","name":{"kind":"Name","value":"resolver"}},{"kind":"Field","name":{"kind":"Name","value":"expected"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"PushConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pushPath"}}]}},{"kind":"InlineFragment","typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"GroupConfig"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"children"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}}]}}]}}]} as unknown as DocumentNode<CheckFieldsFragment, unknown>;
export const DeviceDetailFieldsFragmentDoc = {"kind":"Document","definitions":[{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeviceDetailFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Device"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"auth"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","alias":{"kind":"Name","value":"day"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","alias":{"kind":"Name","value":"week"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"WEEK"}}]},{"kind":"Field","alias":{"kind":"Name","value":"month"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"MONTH"}}]}]}}]} as unknown as DocumentNode<DeviceDetailFieldsFragment, unknown>;
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
export const DevicesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Devices"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"filter"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"DeviceFilter"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"sort"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"DeviceSort"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"first"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"devices"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"filter"},"value":{"kind":"Variable","name":{"kind":"Name","value":"filter"}}},{"kind":"Argument","name":{"kind":"Name","value":"sort"},"value":{"kind":"Variable","name":{"kind":"Name","value":"sort"}}},{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"Variable","name":{"kind":"Name","value":"first"}}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"totalCount"}},{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"checks"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"heartbeats"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"IntValue","value":"50"}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"responseTimeMs"}}]}}]}}]}}]}}]}}]}}]}}]}}]} as unknown as DocumentNode<DevicesQuery, DevicesQueryVariables>;
export const DeviceHostsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"DeviceHosts"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"devices"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"IntValue","value":"200"}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"host"}}]}}]}}]}}]}}]} as unknown as DocumentNode<DeviceHostsQuery, DeviceHostsQueryVariables>;
export const TagsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Tags"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"tags"}}]}}]} as unknown as DocumentNode<TagsQuery, TagsQueryVariables>;
export const DeviceStatusChangedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"DeviceStatusChanged"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deviceStatusChanged"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"NullValue"}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"at"}}]}}]}}]} as unknown as DocumentNode<DeviceStatusChangedSubscription, DeviceStatusChangedSubscriptionVariables>;
export const DeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Device"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"device"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeviceDetailFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeviceDetailFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Device"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"auth"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","alias":{"kind":"Name","value":"day"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","alias":{"kind":"Name","value":"week"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"WEEK"}}]},{"kind":"Field","alias":{"kind":"Name","value":"month"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"MONTH"}}]}]}}]} as unknown as DocumentNode<DeviceQuery, DeviceQueryVariables>;
export const SitesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Sites"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sites"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}}]} as unknown as DocumentNode<SitesQuery, SitesQueryVariables>;
export const CreateDeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateDevice"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"CreateDeviceInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"createDevice"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeviceDetailFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeviceDetailFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Device"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"auth"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","alias":{"kind":"Name","value":"day"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","alias":{"kind":"Name","value":"week"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"WEEK"}}]},{"kind":"Field","alias":{"kind":"Name","value":"month"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"MONTH"}}]}]}}]} as unknown as DocumentNode<CreateDeviceMutation, CreateDeviceMutationVariables>;
export const UpdateDeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"UpdateDevice"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"UpdateDeviceInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"updateDevice"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"FragmentSpread","name":{"kind":"Name","value":"DeviceDetailFields"}}]}}]}},{"kind":"FragmentDefinition","name":{"kind":"Name","value":"DeviceDetailFields"},"typeCondition":{"kind":"NamedType","name":{"kind":"Name","value":"Device"}},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"host"}},{"kind":"Field","name":{"kind":"Name","value":"ip"}},{"kind":"Field","name":{"kind":"Name","value":"kind"}},{"kind":"Field","name":{"kind":"Name","value":"os"}},{"kind":"Field","name":{"kind":"Name","value":"tags"}},{"kind":"Field","name":{"kind":"Name","value":"maintenance"}},{"kind":"Field","name":{"kind":"Name","value":"status"}},{"kind":"Field","name":{"kind":"Name","value":"site"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}},{"kind":"Field","name":{"kind":"Name","value":"ssh"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"enabled"}},{"kind":"Field","name":{"kind":"Name","value":"port"}},{"kind":"Field","name":{"kind":"Name","value":"username"}},{"kind":"Field","name":{"kind":"Name","value":"auth"}},{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}},{"kind":"Field","alias":{"kind":"Name","value":"day"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"DAY"}}]},{"kind":"Field","alias":{"kind":"Name","value":"week"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"WEEK"}}]},{"kind":"Field","alias":{"kind":"Name","value":"month"},"name":{"kind":"Name","value":"uptime"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"window"},"value":{"kind":"EnumValue","value":"MONTH"}}]}]}}]} as unknown as DocumentNode<UpdateDeviceMutation, UpdateDeviceMutationVariables>;
export const DeleteDeviceDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DeleteDevice"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"deleteDevice"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}]}]}}]} as unknown as DocumentNode<DeleteDeviceMutation, DeleteDeviceMutationVariables>;
export const SetSshCredentialDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"SetSshCredential"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"credential"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"SshCredentialInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"setSshCredential"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"credential"},"value":{"kind":"Variable","name":{"kind":"Name","value":"credential"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}}]}}]} as unknown as DocumentNode<SetSshCredentialMutation, SetSshCredentialMutationVariables>;
export const ClearSshCredentialDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ClearSshCredential"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"clearSshCredential"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"credentialStored"}}]}}]}}]} as unknown as DocumentNode<ClearSshCredentialMutation, ClearSshCredentialMutationVariables>;
export const EventsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"Events"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"filter"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"EventFilter"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"first"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"events"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"filter"},"value":{"kind":"Variable","name":{"kind":"Name","value":"filter"}}},{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"Variable","name":{"kind":"Name","value":"first"}}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"severity"}},{"kind":"Field","name":{"kind":"Name","value":"source"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceName"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}}]}}]}}]}}]} as unknown as DocumentNode<EventsQuery, EventsQueryVariables>;
export const EventAddedDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"EventAdded"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"minSeverity"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Severity"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"eventAdded"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"minSeverity"},"value":{"kind":"Variable","name":{"kind":"Name","value":"minSeverity"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"at"}},{"kind":"Field","name":{"kind":"Name","value":"severity"}},{"kind":"Field","name":{"kind":"Name","value":"source"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceName"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}}]}}]} as unknown as DocumentNode<EventAddedSubscription, EventAddedSubscriptionVariables>;
export const StartSessionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"StartSession"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"StartSessionInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"startSession"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sessionId"}},{"kind":"Field","name":{"kind":"Name","value":"token"}},{"kind":"Field","name":{"kind":"Name","value":"tunnelPath"}},{"kind":"Field","name":{"kind":"Name","value":"expiresAt"}}]}}]}}]} as unknown as DocumentNode<StartSessionMutation, StartSessionMutationVariables>;
export const EndSessionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"EndSession"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"endSession"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceName"}},{"kind":"Field","name":{"kind":"Name","value":"protocol"}},{"kind":"Field","name":{"kind":"Name","value":"startedAt"}},{"kind":"Field","name":{"kind":"Name","value":"endedAt"}},{"kind":"Field","name":{"kind":"Name","value":"endReason"}}]}}]}}]} as unknown as DocumentNode<EndSessionMutation, EndSessionMutationVariables>;
export const TestConnectionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"TestConnection"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ConnectionTestInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"testConnection"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"reachable"}},{"kind":"Field","name":{"kind":"Name","value":"authenticated"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}},{"kind":"Field","name":{"kind":"Name","value":"hostKeyChanged"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}}]}}]} as unknown as DocumentNode<TestConnectionMutation, TestConnectionMutationVariables>;
export const AcceptHostKeyDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"AcceptHostKey"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"fingerprintSha256"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"acceptHostKey"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"fingerprintSha256"},"value":{"kind":"Variable","name":{"kind":"Name","value":"fingerprintSha256"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}}]} as unknown as DocumentNode<AcceptHostKeyMutation, AcceptHostKeyMutationVariables>;
export const DeviceSessionsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"DeviceSessions"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"first"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"after"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sessions"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"first"},"value":{"kind":"Variable","name":{"kind":"Name","value":"first"}}},{"kind":"Argument","name":{"kind":"Name","value":"after"},"value":{"kind":"Variable","name":{"kind":"Name","value":"after"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"pageInfo"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"hasNextPage"}},{"kind":"Field","name":{"kind":"Name","value":"endCursor"}}]}},{"kind":"Field","name":{"kind":"Name","value":"edges"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"node"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"protocol"}},{"kind":"Field","name":{"kind":"Name","value":"operatorName"}},{"kind":"Field","name":{"kind":"Name","value":"startedAt"}},{"kind":"Field","name":{"kind":"Name","value":"endedAt"}},{"kind":"Field","name":{"kind":"Name","value":"endReason"}}]}}]}}]}}]}}]} as unknown as DocumentNode<DeviceSessionsQuery, DeviceSessionsQueryVariables>;