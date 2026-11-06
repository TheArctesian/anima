/** The chart's drawing box, in viewBox units; the SVG scales it to its slot. */
export const CHART_BOX = { width: 600, height: 120 } as const;

/** The most heartbeats the chart draws: one history page (D-10). */
export const CHART_POINTS = 100;

/** The distance between two neighbouring points, in viewBox units. */
export const CHART_STEP = CHART_BOX.width / (CHART_POINTS - 1);

/** A beat as the chart reads it. */
export interface ChartBeat {
  readonly at: string;
  readonly responseTimeMs?: number | null;
}

export interface ChartLines {
  /** One `points` attribute per run of beats with a response time; a beat without one breaks the line. */
  readonly segments: readonly string[];
  /** The response time at the top of the box, in milliseconds. */
  readonly maxMs: number;
}

function round(value: number): string {
  return String(Math.round(value * 100) / 100);
}

/**
 * The response-time polyline (D-10, Chart update role): the newest of `beats` (newest first) at the right edge,
 * one `CHART_STEP` per beat to the left, up to `CHART_POINTS`. The height scales to the slowest beat drawn.
 */
export function chartLines(beats: readonly ChartBeat[]): ChartLines {
  const drawn = beats.slice(0, CHART_POINTS).toReversed();
  const times = drawn.map((beat) => beat.responseTimeMs ?? null);
  const maxMs = Math.max(1, ...times.filter((ms): ms is number => ms !== null));
  const segments: string[] = [];
  let run: string[] = [];
  times.forEach((ms, index) => {
    if (ms === null) {
      if (run.length > 0) segments.push(run.join(' '));
      run = [];
      return;
    }
    const x = CHART_BOX.width - (drawn.length - 1 - index) * CHART_STEP;
    const y = CHART_BOX.height - (ms / maxMs) * CHART_BOX.height;
    run.push(`${round(x)},${round(y)}`);
  });
  if (run.length > 0) segments.push(run.join(' '));
  return { segments, maxMs };
}

/** Whether the newest beat is a new one: the line then moves one step left (Chart update role). */
export function gainedNewest(was: readonly ChartBeat[], now: readonly ChartBeat[]): boolean {
  const before = was[0];
  const latest = now[0];
  return before !== undefined && latest !== undefined && Date.parse(latest.at) > Date.parse(before.at);
}
