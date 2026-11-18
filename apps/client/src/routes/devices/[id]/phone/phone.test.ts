import { describe, expect, it } from 'vitest';
import { checkValue, responseText } from './phone';

const at = '2026-11-17T08:00:00Z';

describe('the figures of the device page in the phone layout', () => {
  it('gives the newest response time, and a dash when no beat has one', () => {
    const beats = [
      { checkId: 'a', at, status: 'UP' as const, responseTimeMs: 12 },
      { checkId: 'b', at, status: 'DOWN' as const, responseTimeMs: null },
    ];
    expect(responseText(beats)).toBe('12 ms');
    expect(responseText([])).toBe('—');
  });

  it('ends a check row with the days left, else the response time, else the status', () => {
    const base = { status: 'UP' as const, certificateDaysLeft: null, domainDaysLeft: null, lastHeartbeat: null };
    expect(checkValue({ ...base, certificateDaysLeft: 61 })).toBe('61 d');
    expect(checkValue({ ...base, lastHeartbeat: { at, status: 'UP', responseTimeMs: 3, message: null } })).toBe('3 ms');
    expect(checkValue({ ...base, status: 'DOWN' })).toBe('Down');
  });
});
