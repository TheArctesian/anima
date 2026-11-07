/** The plot's drawing box, in viewBox units; the SVG stretches it to its slot, so lines keep a fixed stroke. */
export const PLOT = { width: 600, height: 120 } as const;

/** The value axis is cut into this many equal intervals, so every chart and its skeleton share one grid. */
export const VALUE_INTERVALS = 4;

/** The span of the x axis, in the points' `at` unit (milliseconds for a time chart). */
export interface TimeWindow {
  readonly start: number;
  readonly end: number;
}

/** `percent` is a fixed 0–100 axis; `open` runs from 0 to a round number above the highest value. */
export type ValueAxis = 'percent' | 'open';

export interface ValueScale {
  /** The value at the top of the plot. */
  readonly top: number;
  /** The distance between two grid lines, in values. */
  readonly step: number;
}

const NICE = [1, 2, 2.5, 5, 10];
const PERCENT: ValueScale = { top: 100, step: 25 };
/** The scale of an open axis with nothing above 0 to show. */
const EMPTY: ValueScale = { top: 1, step: 0.25 };

/** Drops the floating-point noise of `2.5 * 0.1`. */
function clean(value: number): number {
  return Number(value.toPrecision(12));
}

/** The span of `window`, or 0 when it is empty, reversed or not a number. */
export function spanOf(window: TimeWindow): number {
  const span = window.end - window.start;
  return Number.isFinite(span) && span > 0 ? span : 0;
}

/** x in viewBox units for `at`; `start` is the left edge and `end` the right edge. An empty window puts all at the right. */
export function timeScale(window: TimeWindow): (at: number) => number {
  const span = spanOf(window);
  return (at) => (span === 0 ? PLOT.width : ((at - window.start) / span) * PLOT.width);
}

/**
 * The value scale for `axis`. An open axis takes the smallest step of 1, 2, 2.5 or 5 times a power of ten that puts
 * `highest` at or below the top, so the grid lines fall on round values.
 */
export function valueScale(axis: ValueAxis, highest: number): ValueScale {
  if (axis === 'percent') return PERCENT;
  if (!Number.isFinite(highest) || highest <= 0) return EMPTY;
  const wanted = highest / VALUE_INTERVALS;
  const power = 10 ** Math.floor(Math.log10(wanted));
  const step = clean((NICE.find((nice) => clean(nice * power) >= wanted) ?? 10) * power);
  return { top: clean(step * VALUE_INTERVALS), step };
}

/** y in viewBox units for `value`: 0 at the bottom, the scale's top at the top. Values above the top fall outside the box. */
export function valueY(scale: ValueScale, value: number): number {
  return PLOT.height - (value / scale.top) * PLOT.height;
}

/**
 * Chart update (5.7.3): how far right, in viewBox units, the drawn line starts when the window moves from `was` to
 * `now`. Only a forward move of the same span slides, and only by less than the whole span; a range change, a move
 * back or a jump past the old window is 0, and the line is drawn in place.
 */
export function slideOffset(was: TimeWindow, now: TimeWindow): number {
  const span = spanOf(now);
  const moved = now.start - was.start;
  if (span === 0 || spanOf(was) !== span || !(moved > 0 && moved < span)) return 0;
  return (moved / span) * PLOT.width;
}
