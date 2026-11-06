// The K1 additions marked `@pending` that the forms read and write, written by hand from
// docs/programme/phase-2-contracts.md until they are in contracts/schema.graphql (CON-003).
// Each name is the one codegen gives it, so that a swap to `$lib/hub/generated/graphql` changes only imports.

// ---- devices
export type RdpSecurity = 'ANY' | 'NLA' | 'NLA_EXT' | 'TLS' | 'VMCONNECT' | 'RDP';
export type RdpResizeMethod = 'DISPLAY_UPDATE' | 'RECONNECT';
export type MetricsSource = 'NONE' | 'SSH';
/** RA-020. */
export type AccessWarning = 'IGNORED_CERTIFICATE' | 'RDP_SECURITY' | 'UNENCRYPTED_VNC';
export type Protocol = 'SSH' | 'RDP' | 'VNC';

export interface ServerCertificate {
  readonly fingerprintSha256: string;
  readonly subject?: string | null;
  readonly notAfter?: string | null;
  readonly firstSeenAt: string;
}

export interface RdpSettings {
  readonly enabled: boolean;
  readonly port: number;
  readonly username: string;
  readonly domain?: string | null;
  readonly security: RdpSecurity;
  readonly ignoreCertificate: boolean;
  readonly resizeMethod: RdpResizeMethod;
  readonly audioOut: boolean;
  readonly microphone: boolean;
  readonly sharedDrive: boolean;
  readonly credentialStored: boolean;
  readonly serverCertificate?: ServerCertificate | null;
}

export interface VncSettings {
  readonly enabled: boolean;
  readonly port: number;
  /** macOS Screen Sharing needs one. */
  readonly username?: string | null;
  readonly credentialStored: boolean;
}

/** RA-032, RA-033. All default true. */
export interface TransferPolicy {
  readonly upload: boolean;
  readonly download: boolean;
  readonly copy: boolean;
  readonly paste: boolean;
}

/** The K1 `Device` fields the device form reads. */
export interface PendingDeviceFields {
  readonly rdp: RdpSettings;
  readonly vnc: VncSettings;
  /** Lower-case aa:bb:cc:dd:ee:ff (RA-031). */
  readonly macAddress?: string | null;
  readonly metricsSource: MetricsSource;
  readonly transfer: TransferPolicy;
  readonly accessWarnings: readonly AccessWarning[];
}

export interface RdpSettingsInput {
  enabled: boolean;
  port?: number | null;
  username: string;
  domain?: string | null;
  security?: RdpSecurity | null;
  ignoreCertificate?: boolean | null;
  resizeMethod?: RdpResizeMethod | null;
  audioOut?: boolean | null;
  microphone?: boolean | null;
  sharedDrive?: boolean | null;
}

export interface VncSettingsInput {
  enabled: boolean;
  port?: number | null;
  username?: string | null;
}

export interface TransferPolicyInput {
  upload?: boolean | null;
  download?: boolean | null;
  copy?: boolean | null;
  paste?: boolean | null;
}

export interface PasswordCredentialInput {
  password: string;
}

/** The K1 additions to `CreateDeviceInput` and `UpdateDeviceInput`. On update an empty `macAddress` clears it. */
export interface PendingDeviceInput {
  rdp?: RdpSettingsInput | null;
  vnc?: VncSettingsInput | null;
  macAddress?: string | null;
  metricsSource?: MetricsSource | null;
  transfer?: TransferPolicyInput | null;
}

/** The K1 additions to `ConnectionTestInput`. */
export interface PendingConnectionTestInput {
  protocol?: Protocol | null;
  rdp?: RdpSettingsInput | null;
  vnc?: VncSettingsInput | null;
  password?: PasswordCredentialInput | null;
}

/** The K1 additions to `ConnectionTestResult`. */
export interface PendingConnectionTestResult {
  readonly serverCertificate?: ServerCertificate | null;
  readonly certificateChanged: boolean;
}

// ---- web interfaces
export type OpenIn = 'ANIMA_TAB' | 'SYSTEM_BROWSER';

export interface WebInterface {
  readonly id: string;
  readonly deviceId: string;
  readonly label: string;
  readonly url: string;
  readonly openIn: OpenIn;
  /** Pinned on accept (WEB-008). */
  readonly pinnedFingerprintSha256?: string | null;
  readonly pinnedAt?: string | null;
}

/** WEB-002. */
export interface WebInterfaceSuggestion {
  readonly checkId: string;
  readonly url: string;
  readonly label: string;
}

export interface WebInterfaceInput {
  /** Defaults to the URL's host. */
  label?: string | null;
  url: string;
  openIn?: OpenIn | null;
}

// ---- checks (identifiers fixed forever, CHK-009)
export type PendingCheckType = 'DOCKER' | 'SNMP' | 'TAILSCALE_PING' | 'SYSTEMD';
export type SnmpVersion = 'V2C' | 'V3';

export interface DockerConfig {
  readonly __typename: 'DockerConfig';
  readonly container: string;
  readonly timeoutMs: number;
}

export interface SystemdConfig {
  readonly __typename: 'SystemdConfig';
  readonly unit: string;
  readonly timeoutMs: number;
}

/** Community and v3 passwords are never returned (SEC-005). */
export interface SnmpConfig {
  readonly __typename: 'SnmpConfig';
  readonly port: number;
  readonly version: SnmpVersion;
  readonly oid: string;
  /** Up when equal; null: Up when the OID answers. */
  readonly expected?: string | null;
  readonly username?: string | null;
  readonly timeoutMs: number;
}

/** The target defaults to the device host. */
export interface TailscalePingConfig {
  readonly __typename: 'TailscalePingConfig';
  readonly target?: string | null;
  readonly timeoutMs: number;
}

export type PendingCheckConfig = DockerConfig | SystemdConfig | SnmpConfig | TailscalePingConfig;

export interface DockerConfigInput {
  container: string;
  timeoutMs?: number | null;
}

export interface SystemdConfigInput {
  unit: string;
  timeoutMs?: number | null;
}

export interface SnmpConfigInput {
  port?: number | null;
  version?: SnmpVersion | null;
  oid: string;
  expected?: string | null;
  community?: string | null;
  username?: string | null;
  authPassword?: string | null;
  privacyPassword?: string | null;
  timeoutMs?: number | null;
}

export interface TailscalePingConfigInput {
  target?: string | null;
  timeoutMs?: number | null;
}

/** The K1 additions to `CheckConfigInput`. */
export interface PendingCheckConfigInput {
  docker?: DockerConfigInput | null;
  snmp?: SnmpConfigInput | null;
  tailscalePing?: TailscalePingConfigInput | null;
  systemd?: SystemdConfigInput | null;
}
