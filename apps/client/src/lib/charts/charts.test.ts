import { describe, expect, it } from 'vitest';
import { chartLabel } from './label';
import { highestOf, latestOf, seriesPlot, thresholdY, type ChartPoint } from './plot';
import { PLOT, slideOffset, timeScale, valueScale, valueY } from './scales';

const HOUR = 3_600_000;
const span = { start: 0, end: HOUR };
const percent = valueScale('percent', 0);

/** The x and y of each point in an SVG `points` attribute. */
function coordinates(points: string | undefined): number[][] {
  return (points ?? '').split(' ').map((pair) => pair.split(',').map(Number));
}

describe('scales (D-1)', () => {
  it('D-1: the window start is the left edge and its end the right edge', () => {
    const x = timeScale(span);
    expect([x(0), x(HOUR / 2), x(HOUR)]).toEqual([0, PLOT.width / 2, PLOT.width]);
  });

  it('D-1: an empty or reversed window puts every point at the right edge instead of dividing by zero', () => {
    expect(timeScale({ start: 5, end: 5 })(3)).toBe(PLOT.width);
    expect(timeScale({ start: 9, end: 1 })(3)).toBe(PLOT.width);
  });

  it('D-4: a percent axis is 0 to 100 whatever the values', () => {
    expect(valueScale('percent', 3)).toEqual({ top: 100, step: 25 });
    expect(valueScale('percent', 250)).toEqual({ top: 100, step: 25 });
  });

  it.each([
    [20, 20, 5],
    [21, 40, 10],
    [100, 100, 25],
    [101, 200, 50],
    [0.3, 0.4, 0.1],
    [7, 8, 2],
    [9, 10, 2.5],
    [1234, 2000, 500],
  ])('D-1: an open axis with %d at its highest tops out at %d in steps of %d', (highest, top, step) => {
    expect(valueScale('open', highest)).toEqual({ top, step });
  });

  it('D-1: an open axis with nothing above 0, or a value that is not a number, still has a scale', () => {
    for (const highest of [0, -5, Number.NaN, Number.POSITIVE_INFINITY]) {
      expect(valueScale('open', highest)).toEqual({ top: 1, step: 0.25 });
    }
  });

  it('D-1: 0 is at the bottom of the plot and the top value at the top', () => {
    expect([valueY(percent, 0), valueY(percent, 50), valueY(percent, 100)]).toEqual([PLOT.height, PLOT.height / 2, 0]);
  });
});

describe('seriesPlot (D-1)', () => {
  const point = (minute: number, value: number | null, band?: [number, number]): ChartPoint => ({
    at: minute * 60_000,
    value,
    ...(band ? { min: band[0], max: band[1] } : {}),
  });

  it('D-1: one line through the points, oldest at the left', () => {
    const { lines } = seriesPlot([point(30, 50), point(60, 100), point(0, 0)], span, percent);
    expect(lines.map(coordinates)).toEqual([[[0, PLOT.height], [300, 60], [600, 0]]]);
  });

  it('D-1: a missing value, or one that is not a number, breaks the line', () => {
    const points = [point(0, 10), point(1, null), point(2, 10), point(3, Number.NaN), point(4, 10), point(5, 10)];
    expect(seriesPlot(points, span, percent).lines.map((line) => coordinates(line).length)).toEqual([1, 1, 2]);
  });

  it('D-1: a gap wider than the interval breaks the line; one as wide does not', () => {
    const points = [point(0, 10), point(1, 10), point(5, 10), point(6, 10)];
    expect(seriesPlot(points, span, percent, 60_000).lines).toHaveLength(2);
    expect(seriesPlot(points, span, percent, 4 * 60_000).lines).toHaveLength(1);
  });

  it('D-1: no points draw nothing', () => {
    expect(seriesPlot([], span, percent)).toEqual({ lines: [], bands: [] });
  });

  it('D-1: points outside the window are kept so the plot can clip them', () => {
    const [line] = seriesPlot([point(-30, 10), point(30, 10)], span, percent).lines;
    expect(coordinates(line).map(([x]) => x)).toEqual([-300, 300]);
  });

  it('D-1: the band runs along the max, then back along the min', () => {
    const { bands } = seriesPlot([point(0, 50, [25, 75]), point(60, 50, [0, 100])], span, percent);
    expect(bands.map(coordinates)).toEqual([[[0, 30], [600, 0], [600, 120], [0, 90]]]);
  });

  it('D-1: the band breaks where a point has no min or max, and a lone banded point draws no band', () => {
    const points = [point(0, 5, [1, 9]), point(1, 5, [1, 9]), point(2, 5), point(3, 5, [1, 9])];
    const { bands, lines } = seriesPlot(points, span, percent);
    expect(bands.map((band) => coordinates(band).length)).toEqual([4]);
    expect(lines).toHaveLength(1);
  });

  it('D-1: a min above its max still draws the band the right way up', () => {
    const { bands } = seriesPlot([point(0, 50, [75, 25]), point(60, 50, [75, 25])], span, percent);
    expect(coordinates(bands[0]).map(([, y]) => y)).toEqual([30, 30, 90, 90]);
  });

  it('D-1: the highest of values, mins and maxes sets an open axis; the latest is the newest value', () => {
    const points = [point(2, 4), point(0, 3, [1, 12]), point(1, null)];
    expect(highestOf(points)).toBe(12);
    expect(latestOf(points)).toBe(4);
    expect(latestOf([point(0, null)])).toBeNull();
    expect(highestOf([])).toBe(0);
  });
});

describe('thresholdY (MET-004)', () => {
  it('MET-004: the 85 % line is 85 % of the way up a percent chart', () => {
    expect(thresholdY(percent, 85)).toBeCloseTo(PLOT.height * 0.15);
  });

  it('MET-004: no threshold, or one outside the axis, draws no line', () => {
    expect(thresholdY(percent, undefined)).toBeNull();
    expect(thresholdY(percent, 120)).toBeNull();
    expect(thresholdY(percent, -1)).toBeNull();
    expect(thresholdY(percent, Number.NaN)).toBeNull();
  });
});

describe('slideOffset (5.7.3 Chart update)', () => {
  it('5.7.3: a window that moves forward slides by the distance the time axis moved', () => {
    expect(slideOffset(span, { start: HOUR / 4, end: HOUR * 1.25 })).toBe(PLOT.width / 4);
  });

  it('5.7.3: a range change, a move back, no move or a jump past the whole window does not slide', () => {
    expect(slideOffset(span, { start: HOUR, end: HOUR * 25 })).toBe(0);
    expect(slideOffset(span, { start: -10, end: HOUR - 10 })).toBe(0);
    expect(slideOffset(span, span)).toBe(0);
    expect(slideOffset(span, { start: HOUR, end: HOUR * 2 })).toBe(0);
    expect(slideOffset({ start: 0, end: 0 }, { start: 1, end: 1 })).toBe(0);
  });
});

describe('chartLabel (D-1)', () => {
  const ms = (value: number) => `${String(value)} ms`;
  const one = [{ label: 'Response time', points: [{ at: 1, value: 20 }, { at: 2, value: 30 }] }];

  it('D-1: one series says the title, the latest value and the top of the chart', () => {
    expect(chartLabel({ title: 'CPU, last hour', series: one, top: 40, format: ms })).toBe('CPU, last hour, latest 30 ms, top of the chart 40 ms');
  });

  it('D-1: more series name each latest value, so colour is never the only cue', () => {
    const two = [one[0] ?? { label: '', points: [] }, { label: 'Out', points: [{ at: 1, value: null }] }];
    expect(chartLabel({ title: 'Network', series: two, top: 40, format: ms })).toBe('Network, Response time latest 30 ms, Out latest no value, top of the chart 40 ms');
  });

  it('D-1: no value at all says so', () => {
    expect(chartLabel({ title: 'CPU', series: [{ label: 'CPU', points: [] }], top: 100, format: ms })).toBe('CPU: no values yet');
    expect(chartLabel({ title: 'CPU', series: [], top: 100, format: ms, empty: 'Nothing' })).toBe('Nothing');
  });
});
