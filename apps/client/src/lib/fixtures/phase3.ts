// The K6 Phase 3 fixtures (`contracts/fixtures/phase3.json`) as typed slices, for tests and mock data in
// the Phase 3 client packages. No route imports this module: the JSON lives outside the client root, which
// Vite's fs.allow may refuse in a build. Use it from `*.test.ts` and stories only.
//
// The K1 types of Phase 3 are @pending, so codegen does not emit them yet. The shapes below follow K1 and
// are built field by field: a K1 field that is renamed in the JSON, or removed from a shape, fails `bun run
// check`. Enum values are checked at load time against the lists here, which also take the generated types
// where codegen has them (Protocol, CheckType).
import type { CheckType, Protocol } from '$lib/hub/generated/graphql';
import raw from '../../../../../contracts/fixtures/phase3.json';

function oneOf<T extends string>(allowed: readonly T[], value: string): T {
  const found = allowed.find((candidate) => candidate === value);
  if (found === undefined) throw new Error(`Fixture value "${value}" is not one of ${allowed.join(', ')}`);
  return found;
}

const CHANNEL_KINDS = ['WEBHOOK', 'NTFY', 'EMAIL', 'APPRISE'] as const;
const NOTIFICATION_KINDS = ['DOWN', 'UP', 'STILL_DOWN', 'CERTIFICATE_EXPIRY', 'DOMAIN_EXPIRY', 'TEST'] as const;
const TOKEN_SCOPES = ['READ', 'WRITE'] as const;
const SHARE_ACCESS = ['VIEW', 'CONTROL'] as const;
const RECORDING_FORMATS = ['GUACAMOLE', 'ASCIICAST'] as const;
const RETENTION_SUBJECTS = ['HEARTBEATS', 'METRICS', 'EVENTS', 'RECORDINGS', 'DELIVERIES'] as const;
const PROTOCOLS = ['SSH', 'RDP', 'VNC'] as const satisfies readonly Protocol[];

export interface NotificationTemplate {
  title: string;
  body: string;
}

export type ChannelSettings =
  | { origin: string; method: 'POST' | 'PUT'; headerNames: string[] }
  | { server: string; priority: number; tokenStored: boolean }
  | {
      host: string;
      port: number;
      security: 'TLS' | 'STARTTLS';
      username: string | null;
      from: string;
      to: string[];
      passwordStored: boolean;
    }
  | { server: string; mode: 'STATELESS_URLS' | 'CONFIG_KEY'; tags: string[] };

export interface NotificationChannel {
  id: string;
  name: string;
  kind: (typeof CHANNEL_KINDS)[number];
  onByDefault: boolean;
  settings: ChannelSettings;
  template: NotificationTemplate | null;
  createdAt: string;
  updatedAt: string;
}

export interface Delivery {
  id: string;
  channelId: string | null;
  channelName: string;
  kind: (typeof NOTIFICATION_KINDS)[number];
  at: string;
  ok: boolean;
  message: string | null;
}

export interface ApiKey {
  id: string;
  name: string;
  scope: (typeof TOKEN_SCOPES)[number];
  prefix: string;
  expiresAt: string;
  createdAt: string;
  lastUsedAt: string | null;
}

export interface SessionShare {
  id: string;
  sessionId: string;
  access: (typeof SHARE_ACCESS)[number];
  maxViewers: number;
  viewers: number;
  createdAt: string;
  expiresAt: string;
}

export interface Recording {
  id: string;
  sessionId: string;
  deviceId: string | null;
  deviceName: string;
  protocol: Protocol;
  format: (typeof RECORDING_FORMATS)[number];
  startedAt: string;
  endedAt: string | null;
  sizeBytes: number;
  complete: boolean;
  expiresAt: string;
  /** The K2 column `sessions.recordings.stopped_reason`; not a K1 field. */
  stoppedReason: 'size_limit' | 'disk_full' | 'write_failed' | null;
}

export interface RetentionPeriod {
  subject: (typeof RETENTION_SUBJECTS)[number];
  days: number;
  minDays: number;
  maxDays: number;
  defaultDays: number;
}

export interface CheckDefaults {
  intervalSeconds: number;
  retries: number;
  retryIntervalSeconds: number;
  degradedThresholdMs: number;
}

export interface NotificationSettings {
  certificateDays: number[];
  template: NotificationTemplate;
  placeholders: string[];
}

/** A plaintext recording for a player test: Guacamole instructions or an asciicast v2 file. */
export interface RecordingStream {
  recordingId: string;
  format: (typeof RECORDING_FORMATS)[number];
  text: string;
}

export interface DomainExpiryCheck {
  id: string;
  deviceId: string;
  name: string;
  type: CheckType;
  domainDaysLeft: number;
  config: { domainExpiry: { domain: string; warnDays: number; timeoutMs: number } };
}

export const channels: readonly NotificationChannel[] = raw.channels.map((c) => ({
  ...c,
  kind: oneOf(CHANNEL_KINDS, c.kind),
  settings: c.settings as ChannelSettings,
}));

export const deliveries: readonly Delivery[] = raw.deliveries.map((d) => ({
  ...d,
  kind: oneOf(NOTIFICATION_KINDS, d.kind),
}));

export const apiKeys: readonly ApiKey[] = raw.apiKeys.map((k) => ({ ...k, scope: oneOf(TOKEN_SCOPES, k.scope) }));

export const shares: readonly SessionShare[] = raw.shares.map((s) => ({ ...s, access: oneOf(SHARE_ACCESS, s.access) }));

export const recordings: readonly Recording[] = raw.recordings.map((r) => ({
  ...r,
  protocol: oneOf(PROTOCOLS, r.protocol),
  format: oneOf(RECORDING_FORMATS, r.format),
  stoppedReason: r.stoppedReason as Recording['stoppedReason'],
}));

export const retention: readonly RetentionPeriod[] = raw.retention.map((p) => ({
  ...p,
  subject: oneOf(RETENTION_SUBJECTS, p.subject),
}));

export const checkDefaults: CheckDefaults = raw.checkDefaults;

export const notificationSettings: NotificationSettings = raw.notificationSettings;

export const domainExpiryCheck: DomainExpiryCheck = { ...raw.checks[0], type: 'DOMAIN_EXPIRY' } as DomainExpiryCheck;

export const guacamoleStream: RecordingStream = {
  ...raw.streams.guacamole,
  format: oneOf(RECORDING_FORMATS, raw.streams.guacamole.format),
};

export const asciicastStream: RecordingStream = {
  ...raw.streams.asciicast,
  format: oneOf(RECORDING_FORMATS, raw.streams.asciicast.format),
};
