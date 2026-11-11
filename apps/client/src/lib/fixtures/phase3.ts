// The K6 Phase 3 fixtures (`contracts/fixtures/phase3.json`) as typed slices. This is test and story
// support: no route imports it, because the JSON lives outside the client root and Vite's fs.allow has not
// been shown to accept it in a build. When the Phase 3 client packages land, split the slices into their
// domain folders (notifications, api-keys, sessions/recording, retention).
//
// The K1 types of Phase 3 are @pending, so codegen does not emit them yet. The shapes below follow K1 and are
// built field by field, so a field renamed in the JSON fails `bun run check`. `phase3.test.ts` checks every
// list below against its K1 enum and every slice's keys against the K1 fields of its type. Enum values are
// narrowed at load time with `oneOf`.
import type { CheckType, Protocol } from '$lib/hub/generated/graphql';
import raw from '../../../../../contracts/fixtures/phase3.json';

function oneOf<T extends string>(allowed: readonly T[], value: string): T {
  const found = allowed.find((candidate) => candidate === value);
  if (found === undefined) throw new Error(`Fixture value "${value}" is not one of ${allowed.join(', ')}`);
  return found;
}

export const CHANNEL_KINDS = ['WEBHOOK', 'NTFY', 'EMAIL', 'APPRISE'] as const;
export const NOTIFICATION_KINDS = ['DOWN', 'UP', 'STILL_DOWN', 'CERTIFICATE_EXPIRY', 'DOMAIN_EXPIRY', 'TEST'] as const;
export const TOKEN_SCOPES = ['READ', 'WRITE'] as const;
export const RECORDING_FORMATS = ['GUACAMOLE', 'ASCIICAST'] as const;
export const RETENTION_SUBJECTS = ['HEARTBEATS', 'METRICS', 'EVENTS', 'RECORDINGS', 'DELIVERIES'] as const;
export const WEBHOOK_METHODS = ['POST', 'PUT'] as const;
export const SMTP_SECURITY = ['TLS', 'STARTTLS'] as const;
export const APPRISE_MODES = ['STATELESS_URLS', 'CONFIG_KEY'] as const;
export const DELIVERY_REASONS = [
  'TARGET_NOT_PERMITTED',
  'BAD_ADDRESS',
  'SECRET_UNREADABLE',
  'SETUP_FAILED',
  'CONNECTION_FAILED',
  'TLS_FAILED',
  'TIMED_OUT',
  'HTTP_STATUS',
  'NO_STARTTLS',
  'SMTP_REFUSED',
  'NOT_UNDERSTOOD',
] as const;
const STOP_REASONS = ['size_limit', 'disk_full', 'write_failed'] as const;
const PROTOCOLS = ['SSH', 'RDP', 'VNC'] as const satisfies readonly Protocol[];

export interface NotificationTemplate {
  title: string;
  body: string;
}

export type ChannelSettings =
  | { origin: string; method: (typeof WEBHOOK_METHODS)[number]; headerNames: string[] }
  | { server: string; priority: number; tokenStored: boolean }
  | {
      host: string;
      port: number;
      security: (typeof SMTP_SECURITY)[number];
      username: string | null;
      from: string;
      to: string[];
      passwordStored: boolean;
    }
  | { server: string; mode: (typeof APPRISE_MODES)[number]; tags: string[] };

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
  reason: (typeof DELIVERY_REASONS)[number] | null;
  code: number | null;
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
  stoppedReason: (typeof STOP_REASONS)[number] | null;
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

function need<T>(value: T | undefined): T {
  if (value === undefined) throw new Error('A channel fixture lacks a field of its kind');
  return value;
}

type RawSettings = (typeof raw.channels)[number]['settings'];

function channelSettings(s: RawSettings): ChannelSettings {
  if (s.origin !== undefined) return { origin: s.origin, method: oneOf(WEBHOOK_METHODS, need(s.method)), headerNames: need(s.headerNames) };
  if (s.tokenStored !== undefined) return { server: need(s.server), priority: need(s.priority), tokenStored: s.tokenStored };
  if (s.passwordStored !== undefined) {
    return {
      host: need(s.host),
      port: need(s.port),
      security: oneOf(SMTP_SECURITY, need(s.security)),
      username: s.username ?? null,
      from: need(s.from),
      to: need(s.to),
      passwordStored: s.passwordStored,
    };
  }
  return { server: need(s.server), mode: oneOf(APPRISE_MODES, need(s.mode)), tags: need(s.tags) };
}

export const channels: readonly NotificationChannel[] = raw.channels.map((c) => ({
  ...c,
  kind: oneOf(CHANNEL_KINDS, c.kind),
  settings: channelSettings(c.settings),
}));

export const deliveries: readonly Delivery[] = raw.deliveries.map((d) => ({
  ...d,
  kind: oneOf(NOTIFICATION_KINDS, d.kind),
  reason: d.reason === null ? null : oneOf(DELIVERY_REASONS, d.reason),
}));

export const apiKeys: readonly ApiKey[] = raw.apiKeys.map((k) => ({ ...k, scope: oneOf(TOKEN_SCOPES, k.scope) }));

export const recordings: readonly Recording[] = raw.recordings.map((r) => ({
  ...r,
  protocol: oneOf(PROTOCOLS, r.protocol),
  format: oneOf(RECORDING_FORMATS, r.format),
  stoppedReason: r.stoppedReason === null ? null : oneOf(STOP_REASONS, r.stoppedReason),
}));

export const retention: readonly RetentionPeriod[] = raw.retention.map((p) => ({
  ...p,
  subject: oneOf(RETENTION_SUBJECTS, p.subject),
}));

export const checkDefaults: CheckDefaults = raw.checkDefaults;

export const notificationSettings: NotificationSettings = raw.notificationSettings;

const [rawDomainCheck] = raw.checks;
if (rawDomainCheck === undefined) throw new Error('phase3.json has no domain expiry check');

export const domainExpiryCheck: DomainExpiryCheck = {
  id: rawDomainCheck.id,
  deviceId: rawDomainCheck.deviceId,
  name: rawDomainCheck.name,
  type: oneOf(['DOMAIN_EXPIRY'] as const satisfies readonly CheckType[], rawDomainCheck.type),
  domainDaysLeft: rawDomainCheck.domainDaysLeft,
  config: { domainExpiry: { ...rawDomainCheck.config.domainExpiry } },
};

export const guacamoleStream: RecordingStream = {
  ...raw.streams.guacamole,
  format: oneOf(RECORDING_FORMATS, raw.streams.guacamole.format),
};

export const asciicastStream: RecordingStream = {
  ...raw.streams.asciicast,
  format: oneOf(RECORDING_FORMATS, raw.streams.asciicast.format),
};
