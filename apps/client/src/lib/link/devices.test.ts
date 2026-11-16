import { describe, expect, it } from 'vitest';
import { countdownText, deviceLinkText, devicesShown, expiryText, inviteFailure, pathText, secondsLeft } from './devices';

describe('a link device in words (FRN-009, LNK-006)', () => {
  it('LNK-006: the path says direct, relayed with its relay, or not yet', () => {
    expect(pathText({ path: 'DIRECT', relay: null })).toBe('Direct');
    expect(pathText({ path: 'RELAYED', relay: 'fra' })).toBe('Relayed through fra');
    expect(pathText({ path: 'RELAYED', relay: null })).toBe('Relayed');
    expect(pathText({ path: 'NONE', relay: null })).toBe('Not yet connected');
  });

  it('FRN-009: a key with no expiry says so; an unreadable one is a dash', () => {
    expect(expiryText(null)).toBe('No expiry');
    expect(expiryText('not a date')).toBe('—');
    expect(expiryText('2026-12-01T10:00:00Z', 'en-GB')).toContain('2026');
  });
});

describe('the invite countdown (LNK-003)', () => {
  const NOW = new Date('2026-11-15T10:00:00Z');

  it('LNK-003: an invite made now has 15 minutes, counted down in whole seconds', () => {
    expect(secondsLeft('2026-11-15T10:15:00Z', NOW)).toBe(900);
    expect(countdownText(900)).toBe('15:00');
    expect(secondsLeft('2026-11-15T10:14:59.200Z', NOW)).toBe(900);
    expect(countdownText(59)).toBe('0:59');
  });

  it('LNK-003: an expired or unreadable invite has no time left', () => {
    expect(secondsLeft('2026-11-15T09:59:00Z', NOW)).toBe(0);
    expect(secondsLeft('soon', NOW)).toBe(0);
    expect(countdownText(-5)).toBe('0:00');
  });
});

describe('the device list', () => {
  it('FRN-009: hidden on a hub with no link (UNAVAILABLE), an error for any other failure', () => {
    expect(devicesShown(null)).toBe('loading');
    expect(devicesShown({ ok: true })).toBe('listed');
    expect(devicesShown({ ok: false, error: { code: 'UNAVAILABLE' } })).toBe('hidden');
    expect(devicesShown({ ok: false, error: { code: 'unreachable' } })).toBe('failed');
  });

  it('LOG-007: an invite refusal is fixed text for each kind', () => {
    const texts = (['VALIDATION', 'FORBIDDEN', 'unreachable', 'INTERNAL'] as const).map((code) => inviteFailure({ code }));
    expect(new Set(texts).size).toBe(4);
  });
});

describe('the link of a joined device (LNK-006, F-2)', () => {
  it('LNK-006: up shows the path and the latency', () => {
    expect(deviceLinkText({ joined: true, state: 'up', path: 'relayed', latencyMs: 41.6 })).toBe('Up: relayed, 42 ms.');
    expect(deviceLinkText({ joined: true, state: 'up', path: 'direct' })).toBe('Up: direct.');
  });

  it('F-2: a taken port is told, not changed in silence', () => {
    expect(deviceLinkText({ joined: true, state: 'down', reason: 'port-taken', port: 41_000 })).toBe('Down: another program holds port 41000.');
    expect(deviceLinkText({ joined: true, state: 'down', reason: 'ended' })).toContain('starts again');
  });
});
