import type { ChartPoint } from '$lib/charts/plot';
import type { TimeWindow } from '$lib/charts/scales';

/** The most heartbeats the chart draws: one history page (D-10). */
export const RESPONSE_POINTS = 100;

/** A beat as the chart reads it. */
export interface ChartBeat {
  readonly at: string;
  readonly responseTimeMs?: number | null;
}

/**
 * The response-time chart's x axis counts beats, not time: a check beats once per interval, so one step per beat
 * keeps the newest at the right edge and the spacing even. `newest` is the step of the newest beat; it grows by one
 * for each new beat (`stepsAfter`), so the window moves forward one step and the line slides (Chart update role).
 */
export interface ResponseSteps {
  readonly points: readonly ChartPoint[];
  readonly window: TimeWindow;
}

/** The newest `RESPONSE_POINTS` of `beats` (newest first) as chart points, the newest at step `newest`. */
export function responseSteps(beats: readonly ChartBeat[], newest: number): ResponseSteps {
  const points = beats.slice(0, RESPONSE_POINTS).map((beat, index) => ({ at: newest - index, value: beat.responseTimeMs ?? null }));
  return { points, window: { start: newest - (RESPONSE_POINTS - 1), end: newest } };
}

/**
 * The step of the newest beat once `now` follows `was`: one step more for each beat in `now` newer than the newest of
 * `was`. A first load or older beats arriving add nothing, so they draw in place.
 */
export function stepsAfter(was: readonly ChartBeat[], now: readonly ChartBeat[], steps: number): number {
  const before = was[0];
  if (before === undefined) return steps;
  const newestBefore = Date.parse(before.at);
  const newer = now.filter((beat) => Date.parse(beat.at) > newestBefore).length;
  return steps + newer;
}
