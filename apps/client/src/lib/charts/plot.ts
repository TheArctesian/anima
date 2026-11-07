import { timeScale, valueY, type TimeWindow, type ValueScale } from './scales';

/** One reading on a chart: `value` is missing when there was no reading; `min` and `max` bound an aggregate. */
export interface ChartPoint {
  readonly at: number;
  readonly value: number | null;
  readonly min?: number | null;
  readonly max?: number | null;
}

/** A K5 series colour, `--chart-series-1` to `--chart-series-6`. */
export type SeriesColour = 1 | 2 | 3 | 4 | 5 | 6;

/** One line on a chart, named in the legend and the text alternative. */
export interface ChartSeries {
  readonly label: string;
  readonly colour: SeriesColour;
  readonly points: readonly ChartPoint[];
}

/** What to draw for one series, in viewBox units. */
interface SeriesPlot {
  /** One SVG `points` attribute per unbroken run of readings. */
  readonly lines: readonly string[];
  /** One SVG `points` attribute per run of readings with a min and a max: the max edge, then the min edge back. */
  readonly bands: readonly string[];
}

interface Placed {
  readonly at: number;
  readonly x: number;
}

const usable = (value: number | null | undefined): value is number => typeof value === 'number' && Number.isFinite(value);

function round(value: number): string {
  return String(Math.round(value * 100) / 100);
}

function pair(x: number, y: number): string {
  return `${round(x)},${round(y)}`;
}

/**
 * Splits `points` (in time order) into runs where `keep` holds. A point that fails `keep` ends the run, and so does a
 * time gap wider than `gap`.
 */
function runs<T extends Placed>(points: readonly T[], keep: (point: T) => boolean, gap: number): T[][] {
  const found: T[][] = [];
  let run: T[] = [];
  for (const point of points) {
    const last = run.at(-1);
    if (!keep(point) || (last !== undefined && point.at - last.at > gap)) {
      if (run.length > 0) found.push(run);
      run = [];
    }
    if (keep(point)) run.push(point);
  }
  if (run.length > 0) found.push(run);
  return found;
}

/**
 * The polylines and min–max bands of one series over `span`. Points are put in time order first; a point without a
 * usable value breaks the line, and so does a gap wider than `gap` (in the `at` unit; no limit by default). Points
 * outside the window are kept: the plot clips them, and the Chart update slide shows them coming in.
 */
export function seriesPlot(
  points: readonly ChartPoint[],
  span: TimeWindow,
  scale: ValueScale,
  gap: number = Number.POSITIVE_INFINITY,
): SeriesPlot {
  const x = timeScale(span);
  const placed = points
    .filter((point) => Number.isFinite(point.at))
    .toSorted((a, b) => a.at - b.at)
    .map((point) => ({ ...point, x: x(point.at) }));
  const lines = runs(placed, (point) => usable(point.value), gap).map((run) =>
    run.map((point) => pair(point.x, valueY(scale, point.value ?? 0))).join(' '),
  );
  const banded = runs(placed, (point) => usable(point.min) && usable(point.max), gap).filter((run) => run.length > 1);
  const bands = banded.map((run) => {
    const upper = run.map((point) => pair(point.x, valueY(scale, Math.max(point.min ?? 0, point.max ?? 0))));
    const lower = run.toReversed().map((point) => pair(point.x, valueY(scale, Math.min(point.min ?? 0, point.max ?? 0))));
    return [...upper, ...lower].join(' ');
  });
  return { lines, bands };
}

/** The highest value, min or max among `points`, for an open axis; 0 when there is none. */
export function highestOf(points: readonly ChartPoint[]): number {
  const values = points.flatMap((point) => [point.value, point.max, point.min]).filter(usable);
  return values.length === 0 ? 0 : Math.max(...values);
}

/** The newest usable value among `points`, or null when there is none. */
export function latestOf(points: readonly ChartPoint[]): number | null {
  let latest: ChartPoint | undefined;
  for (const point of points) {
    if (usable(point.value) && Number.isFinite(point.at) && (latest === undefined || point.at >= latest.at)) latest = point;
  }
  return latest?.value ?? null;
}

/** The y of the threshold line (the 85 % line, MET-004), or null when it is not inside the plot. */
export function thresholdY(scale: ValueScale, threshold: number | undefined): number | null {
  if (threshold === undefined || !Number.isFinite(threshold) || threshold < 0 || threshold > scale.top) return null;
  return valueY(scale, threshold);
}
