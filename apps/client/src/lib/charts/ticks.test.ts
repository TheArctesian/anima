import { describe, expect, it } from 'vitest';
import { PLOT, valueScale } from './scales';
import { timeTicks, valueTicks } from './ticks';

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
/** Thursday 5 November 2026, 10:07:30 UTC. */
const now = Date.UTC(2026, 10, 5, 10, 7, 30);
const texts = (ticks: readonly { text: string }[]) => ticks.map((tick) => tick.text);

describe('timeTicks (MET-003 ranges)', () => {
  it('MET-003: 1 hour is labelled every 10 minutes, on the minute', () => {
    expect(texts(timeTicks({ start: now - HOUR, end: now }, 0))).toEqual(['09:10', '09:20', '09:30', '09:40', '09:50', '10:00']);
  });

  it('MET-003: 24 hours is labelled every 4 hours', () => {
    expect(texts(timeTicks({ start: now - DAY, end: now }, 0))).toEqual(['12:00', '16:00', '20:00', '00:00', '04:00', '08:00']);
  });

  it('MET-003: 7 days is labelled at each midnight with the weekday', () => {
    expect(texts(timeTicks({ start: now - 7 * DAY, end: now }, 0))).toEqual(['Fri 30', 'Sat 31', 'Sun 1', 'Mon 2', 'Tue 3', 'Wed 4', 'Thu 5']);
  });

  it('MET-003: ticks fall on local time, given as the zone offset', () => {
    const ticks = timeTicks({ start: now - DAY, end: now }, 2 * HOUR);
    expect(texts(ticks)[0]).toBe('16:00');
    expect(ticks[0]?.at).toBe(Date.UTC(2026, 10, 4, 14));
    const halfHour = timeTicks({ start: now - HOUR, end: now }, -3.5 * HOUR);
    expect(texts(halfHour)).toEqual(['05:40', '05:50', '06:00', '06:10', '06:20', '06:30']);
  });

  it('D-1: a tick sits where the time scale puts its time, and the edges get none', () => {
    const start = Date.UTC(2026, 10, 5, 9);
    const ticks = timeTicks({ start, end: start + HOUR }, 0);
    expect(ticks.map((tick) => tick.x)).toEqual([100, 200, 300, 400, 500]);
    expect(ticks.every((tick) => tick.x > 0 && tick.x < PLOT.width)).toBe(true);
  });

  it('D-1: an empty window, a window wider than any step, or a bad zone has no ticks', () => {
    expect(timeTicks({ start: now, end: now }, 0)).toEqual([]);
    expect(timeTicks({ start: now - 400 * DAY, end: now }, 0)).toEqual([]);
    expect(timeTicks({ start: now - HOUR, end: now }, Number.NaN)).toEqual([]);
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
