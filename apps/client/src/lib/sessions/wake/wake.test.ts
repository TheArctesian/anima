import { describe, expect, it } from 'vitest';
import type { HubCode, HubResult } from '$lib/hub/hub';
import type { WakeSent } from './api';
import { wakeText } from './wake';

function failed(code: HubCode | 'unreachable' | 'malformed', message = 'relay 10.0.0.9 refused'): HubResult<WakeSent> {
  return { ok: false, error: code === 'VALIDATION' ? { code, message, fields: [] } : { code, message } };
}

describe('wakeText (RA-031)', () => {
  it('RA-031: success names the relay', () => {
    const sent: HubResult<WakeSent> = { ok: true, value: { sentAt: '2026-11-06T10:00:00Z', relayName: 'Pi at home' } };
    expect(wakeText(sent, false)).toBe('Wake sent via Pi at home.');
  });

  it.each(['NOT_FOUND', 'VALIDATION', 'CONFLICT', 'UNAVAILABLE', 'INTERNAL', 'unreachable', 'malformed', 'FORBIDDEN'] as const)(
    'RA-031: %s gives fixed text that never holds the hub text',
    (code) => {
      const text = wakeText(failed(code), false);
      expect(text.length).toBeGreaterThan(0);
      expect(text).not.toContain('10.0.0.9');
    },
  );

  it('RA-031: each wake code has its own cause in the text', () => {
    expect(wakeText(failed('NOT_FOUND'), false)).toContain('no longer exists');
    expect(wakeText(failed('CONFLICT'), false)).toContain('MAC address');
    expect(wakeText(failed('CONFLICT'), false)).toContain('wake relay');
    expect(wakeText(failed('UNAVAILABLE'), false)).toContain('unreachable');
    expect(wakeText(failed('unreachable'), false)).toBe('The hub did not answer. Check the VPN client.');
    expect(wakeText(failed('unreachable'), true)).toBe("The hub did not answer. Check this computer's network.");
  });
});
