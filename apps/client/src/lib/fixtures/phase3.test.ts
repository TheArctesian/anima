import { describe, expect, it } from 'vitest';
import {
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
  shares,
} from './phase3';

describe('K6 phase3 client fixtures', () => {
  it('NOT-003: exactly one channel is on by default, and each kind appears once', () => {
    expect(channels.filter((c) => c.onByDefault)).toHaveLength(1);
    expect(channels.map((c) => c.kind).sort()).toEqual(['APPRISE', 'EMAIL', 'NTFY', 'WEBHOOK']);
  });

  it('NOT-004: each channel has one ok and one failed delivery, and a failure carries a fixed message', () => {
    for (const channel of channels) {
      const own = deliveries.filter((d) => d.channelId === channel.id);
      expect(own.map((d) => d.ok).sort()).toEqual([false, true]);
    }
    expect(deliveries.filter((d) => !d.ok).every((d) => d.message?.endsWith('.'))).toBe(true);
  });

  it('INT-003: one READ and one WRITE key, one expired, none with a secret', () => {
    expect(apiKeys.map((k) => k.scope).sort()).toEqual(['READ', 'WRITE']);
    expect(apiKeys.filter((k) => Date.parse(k.expiresAt) < Date.parse('2026-11-07T00:00:00Z'))).toHaveLength(1);
    expect(apiKeys.every((k) => k.prefix.length === 12 && !('secret' in k))).toBe(true);
  });

  it('RA-035: the one share is VIEW with one viewer', () => {
    expect(shares).toMatchObject([{ access: 'VIEW', viewers: 1 }]);
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
