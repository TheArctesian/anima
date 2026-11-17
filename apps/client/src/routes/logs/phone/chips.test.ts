import { describe, expect, it } from 'vitest';
import { DEFAULT_LOG_FILTER } from '$lib/events/log';
import { chipFilter, chipOf, CHIPS } from './chips';

describe('the log chips of the phone layout', () => {
  it('shows each chip as on after it is chosen', () => {
    for (const chip of CHIPS) expect(chipOf(chipFilter(chip.id, DEFAULT_LOG_FILTER))).toBe(chip.id);
  });

  it('keeps the time window and the device when a chip is chosen', () => {
    const filter = { ...DEFAULT_LOG_FILTER, since: '7d' as const, deviceId: 'd1', severities: ['ERROR' as const] };
    expect(chipFilter('audit', filter)).toEqual({ since: '7d', deviceId: 'd1', severities: [], sources: ['ACCESS'] });
  });

  it('shows All for a filter that no single chip sets', () => {
    expect(chipOf({ ...DEFAULT_LOG_FILTER, severities: ['ERROR', 'WARNING'] })).toBe('all');
    expect(chipOf({ ...DEFAULT_LOG_FILTER, sources: ['CHECK'] })).toBe('all');
  });
});
