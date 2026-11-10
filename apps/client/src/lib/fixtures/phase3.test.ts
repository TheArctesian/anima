import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import {
  APPRISE_MODES,
  CHANNEL_KINDS,
  DELIVERY_REASONS,
  NOTIFICATION_KINDS,
  RECORDING_FORMATS,
  RETENTION_SUBJECTS,
  SMTP_SECURITY,
  TOKEN_SCOPES,
  WEBHOOK_METHODS,
  apiKeys,
  asciicastStream,
  channels,
  checkDefaults,
  deliveries,
  domainExpiryCheck,
  guacamoleStream,
  notificationSettings,
  recordings,
  retention,
} from './phase3';

describe('K6 phase3 client fixtures', () => {
  it('NOT-003: exactly one channel is on by default, and each kind appears once', () => {
    expect(channels.filter((c) => c.onByDefault)).toHaveLength(1);
    expect(channels.map((c) => c.kind).sort()).toEqual(['APPRISE', 'EMAIL', 'NTFY', 'WEBHOOK']);
  });

  it('K6, CR-14: each channel has an ok and a failed delivery; a failure carries a reason, a success none', () => {
    for (const channel of channels) {
      const oks = new Set(deliveries.filter((d) => d.channelId === channel.id).map((d) => d.ok));
      expect([...oks].sort()).toEqual([false, true]);
    }
    expect(deliveries.filter((d) => !d.ok).every((d) => d.reason !== null)).toBe(true);
    expect(deliveries.filter((d) => d.ok).every((d) => d.reason === null && d.code === null)).toBe(true);
  });

  it('INT-003: one READ and one WRITE key, one expired, none with a secret', () => {
    expect(apiKeys.map((k) => k.scope).sort()).toEqual(['READ', 'WRITE']);
    expect(apiKeys.filter((k) => Date.parse(k.expiresAt) < Date.parse('2026-11-07T00:00:00Z'))).toHaveLength(1);
    expect(apiKeys.every((k) => k.prefix.length === 12 && !('secret' in k))).toBe(true);
  });

  it('RA-041: recordings are complete, incomplete and stopped by the size limit', () => {
    expect(recordings.map((r) => r.complete)).toEqual([true, false, true]);
    expect(recordings.map((r) => r.stoppedReason)).toEqual([null, null, 'size_limit']);
  });

  it('RA-041: the Guacamole stream draws a rectangle in valid instructions', () => {
    const instructions = guacamoleStream.text.split(';').filter((i) => i !== '');
    expect(instructions.map((i) => i.slice(i.indexOf('.') + 1).split(',')[0])).toContain('rect');
    for (const instruction of instructions) {
      for (const element of instruction.split(',')) {
        const dot = element.indexOf('.');
        expect(element.length - dot - 1).toBe(Number(element.slice(0, dot)));
      }
    }
  });

  it('RA-041: the asciicast stream starts with a v2 header and prints hello', () => {
    const lines = asciicastStream.text.trimEnd().split('\n');
    expect(JSON.parse(lines[0] ?? '')).toMatchObject({ version: 2 });
    expect(lines.slice(1).some((l) => l.includes('hello\\r\\n'))).toBe(true);
  });

  it('D-25, D-26, D-22: retention, check defaults and certificate days are the defaults', () => {
    expect(retention.every((p) => p.days === p.defaultDays && p.minDays <= p.days && p.days <= p.maxDays)).toBe(true);
    expect(checkDefaults).toEqual({ intervalSeconds: 60, retries: 3, retryIntervalSeconds: 20, degradedThresholdMs: 1000 });
    expect(notificationSettings.certificateDays).toEqual([21, 14, 7]);
  });

  it('CHK-009: the domain expiry check has 12 days left under its 30 warning days', () => {
    expect(domainExpiryCheck.domainDaysLeft).toBeLessThan(domainExpiryCheck.config.domainExpiry.warnDays);
  });
});

describe('K6 phase3 client fixtures: ties to K1', () => {
  const sdl = readFileSync(new URL('../../../../../contracts/schema.graphql', import.meta.url), 'utf8');

  function block(keyword: 'enum' | 'type', name: string): string {
    const found = new RegExp(`^${keyword} ${name}\\b[^{]*\\{([^}]*)\\}`, 'm').exec(sdl);
    if (found === null) throw new Error(`${keyword} ${name} is not in schema.graphql`);
    return found[1] ?? '';
  }
  const code = (text: string): string[] =>
    text.split('\n').map((l) => l.replace(/#.*$/, '').trim()).filter((l) => l !== '' && !l.startsWith('"'));
  const enumValues = (name: string): string[] => code(block('enum', name)).map((l) => l.split(/\s/)[0] ?? '');
  const fields = (name: string): { name: string; required: boolean }[] =>
    code(block('type', name)).map((l) => ({ name: /^(\w+)/.exec(l)?.[1] ?? '', required: /:\s*\S+!(\s|$)/.test(l) }));

  it('every enum list equals its K1 enum', () => {
    const lists: [readonly string[], string][] = [
      [CHANNEL_KINDS, 'ChannelKind'],
      [NOTIFICATION_KINDS, 'NotificationKind'],
      [TOKEN_SCOPES, 'TokenScope'],
      [RECORDING_FORMATS, 'RecordingFormat'],
      [RETENTION_SUBJECTS, 'RetentionSubject'],
      [WEBHOOK_METHODS, 'WebhookMethod'],
      [SMTP_SECURITY, 'SmtpSecurity'],
      [APPRISE_MODES, 'AppriseMode'],
      [DELIVERY_REASONS, 'DeliveryReason'],
    ];
    for (const [list, name] of lists) expect([...list].sort(), name).toEqual(enumValues(name).sort());
  });

  it('every slice has the required K1 fields of its type and no key K1 lacks', () => {
    const slices: [object | undefined, string, string[]][] = [
      [channels[0], 'NotificationChannel', []],
      [deliveries[0], 'Delivery', []],
      [apiKeys[0], 'ApiKey', []],
      [recordings[0], 'Recording', ['stoppedReason']],
      [retention[0], 'RetentionPeriod', []],
      [checkDefaults, 'CheckDefaults', []],
      [notificationSettings, 'NotificationSettings', []],
      [channels.find((c) => c.kind === 'WEBHOOK')?.settings, 'WebhookSettings', []],
      [channels.find((c) => c.kind === 'NTFY')?.settings, 'NtfySettings', []],
      [channels.find((c) => c.kind === 'EMAIL')?.settings, 'EmailSettings', []],
      [channels.find((c) => c.kind === 'APPRISE')?.settings, 'AppriseSettings', []],
    ];
    for (const [sample, type, extra] of slices) {
      const keys = Object.keys(sample ?? {});
      expect(keys.length, type).toBeGreaterThan(0);
      const known = fields(type);
      expect(known.filter((f) => f.required && !keys.includes(f.name)).map((f) => f.name), type).toEqual([]);
      expect(keys.filter((k) => !extra.includes(k) && !known.some((f) => f.name === k)), type).toEqual([]);
    }
  });
});
