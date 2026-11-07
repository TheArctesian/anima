import { describe, expect, it } from 'vitest';
import { seriesPlot } from '$lib/charts/plot';
import { valueScale } from '$lib/charts/scales';
import { rangeGap, rangeWindow } from './ranges';

describe('rangeWindow (MET-003)', () => {
  const now = new Date('2026-11-05T12:00:00Z');
  it('spans 1 hour, 24 hours and 7 days ending at now', () => {
    const start = (range: 'HOUR' | 'DAY' | 'WEEK') => new Date(rangeWindow(range, now).start).toISOString();
    expect(start('HOUR')).toBe('2026-11-05T11:00:00.000Z');
    expect(start('DAY')).toBe('2026-11-04T12:00:00.000Z');
    expect(start('WEEK')).toBe('2026-10-29T12:00:00.000Z');
    expect(rangeWindow('WEEK', now).end).toBe(now.getTime());
  });
});

describe('rangeGap (MET-003)', () => {
  const MINUTE = 60_000;
  const at = (minutes: number) => ({ at: minutes * MINUTE, value: 10 });
  const span = { start: 0, end: 60 * MINUTE };

  it('MET-003: two missing one-minute buckets break the HOUR line; present buckets do not', () => {
    const scale = valueScale('percent', 100);
    expect(seriesPlot([at(1), at(2), at(3), at(4)], span, scale, rangeGap('HOUR')).lines).toHaveLength(1);
    expect(seriesPlot([at(1), at(2), at(5), at(6)], span, scale, rangeGap('HOUR')).lines).toHaveLength(2);
  });

  it('MET-003: the gap grows with the bucket of the range', () => {
    expect([rangeGap('HOUR'), rangeGap('DAY'), rangeGap('WEEK')]).toEqual([150_000, 1_500_000, 9_000_000]);
  });
});
