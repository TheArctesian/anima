import { describe, expect, it } from 'vitest';
import { PLOT, valueScale } from './scales';
import { timeTicks, valueTicks } from './ticks';

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
/** Thursday 5 November 2026, 10:07:30 UTC. */
const now = Date.UTC(2026, 10, 5, 10, 7, 30);
const utc = () => 0;
const texts = (ticks: readonly { text: string }[]) => ticks.map((tick) => tick.text);

describe('timeTicks (MET-003 ranges)', () => {
  it('MET-003: 1 hour is labelled every 10 minutes, on the minute', () => {
    expect(texts(timeTicks({ start: now - HOUR, end: now }, utc))).toEqual(['09:10', '09:20', '09:30', '09:40', '09:50', '10:00']);
  });

  it('MET-003: 24 hours is labelled every 4 hours', () => {
    expect(texts(timeTicks({ start: now - DAY, end: now }, utc))).toEqual(['12:00', '16:00', '20:00', '00:00', '04:00', '08:00']);
  });

  it('MET-003: 7 days is labelled at each midnight with the weekday', () => {
    expect(texts(timeTicks({ start: now - 7 * DAY, end: now }, utc))).toEqual(['Fri 30', 'Sat 31', 'Sun 1', 'Mon 2', 'Tue 3', 'Wed 4', 'Thu 5']);
  });

  it('MET-003: ticks fall on local time, given as the zone offset', () => {
    const ticks = timeTicks({ start: now - DAY, end: now }, () => 2 * HOUR);
    expect(texts(ticks)[0]).toBe('16:00');
    expect(ticks[0]?.at).toBe(Date.UTC(2026, 10, 4, 14));
    const halfHour = timeTicks({ start: now - HOUR, end: now }, () => -3.5 * HOUR);
    expect(texts(halfHour)).toEqual(['05:40', '05:50', '06:00', '06:10', '06:20', '06:30']);
  });

  it('MET-003: a 7-day span across a daylight-saving change keeps its labels at local midnight', () => {
    // Central European time: UTC+2 until 01:00 UTC on Sunday 25 October 2026, then UTC+1.
    const change = Date.UTC(2026, 9, 25, 1);
    const offsetAt = (at: number) => (at < change ? 2 : 1) * HOUR;
    const ticks = timeTicks({ start: Date.UTC(2026, 9, 21, 12), end: Date.UTC(2026, 9, 28, 12) }, offsetAt);
    expect(texts(ticks)).toEqual(['Thu 22', 'Fri 23', 'Sat 24', 'Sun 25', 'Mon 26', 'Tue 27', 'Wed 28']);
    expect(ticks.map((tick) => tick.at)).toEqual([
      Date.UTC(2026, 9, 21, 22),
      Date.UTC(2026, 9, 22, 22),
      Date.UTC(2026, 9, 23, 22),
      Date.UTC(2026, 9, 24, 22),
      Date.UTC(2026, 9, 25, 23),
      Date.UTC(2026, 9, 26, 23),
      Date.UTC(2026, 9, 27, 23),
    ]);
  });

  it('MET-003: a local hour skipped when the clocks go forward gets no label of its own', () => {
    // Central European time: UTC+1 until 01:00 UTC on Sunday 29 March 2026, then UTC+2; 02:00 local never happens.
    const change = Date.UTC(2026, 2, 29, 1);
    const offsetAt = (at: number) => (at < change ? 1 : 2) * HOUR;
    const ticks = timeTicks({ start: change - 2 * HOUR, end: change + 2 * HOUR }, offsetAt);
    expect(texts(ticks)).toEqual(['01:00', '03:00', '04:00']);
    expect(ticks.map((tick) => tick.at)).toEqual([change - HOUR, change, change + HOUR]);
  });

  it('D-1: a tick sits where the time scale puts its time, and the edges get none', () => {
    const start = Date.UTC(2026, 10, 5, 9);
    const ticks = timeTicks({ start, end: start + HOUR }, utc);
    expect(ticks.map((tick) => tick.x)).toEqual([100, 200, 300, 400, 500]);
    expect(ticks.every((tick) => tick.x > 0 && tick.x < PLOT.width)).toBe(true);
  });

  it('D-1: an empty window, a window wider than any step, or a bad zone has no ticks', () => {
    expect(timeTicks({ start: now, end: now }, utc)).toEqual([]);
    expect(timeTicks({ start: now - 400 * DAY, end: now }, utc)).toEqual([]);
    expect(timeTicks({ start: now - HOUR, end: now }, () => Number.NaN)).toEqual([]);
  });
});

describe('valueTicks (D-1)', () => {
  it('D-1: a percent axis has grid lines at 0, 25, 50, 75 and 100, bottom first', () => {
    const ticks = valueTicks(valueScale('percent', 0));
    expect(ticks.map((tick) => tick.value)).toEqual([0, 25, 50, 75, 100]);
    expect(ticks.map((tick) => tick.y)).toEqual([120, 90, 60, 30, 0]);
  });

  it('D-1: an open axis has the same five lines at round values', () => {
    expect(valueTicks(valueScale('open', 0.3)).map((tick) => tick.value)).toEqual([0, 0.1, 0.2, 0.3, 0.4]);
  });
});
