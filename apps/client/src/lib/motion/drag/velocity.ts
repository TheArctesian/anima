/** One pointer reading: when (event timestamp, ms) and where (px along the axis). */
export interface Sample {
  readonly at: number;
  readonly value: number;
}

/** Only the last stretch of the drag tells how fast the pointer left; older readings are dropped. */
export const VELOCITY_WINDOW_MS = 100;

const msPerSecond = 1000;

/**
 * The pointer's speed in px/s at `until`, from the samples in the last `VELOCITY_WINDOW_MS`. A pointer that
 * stopped before it was released has speed 0. Time comes from the samples, never from a clock.
 */
export function velocityOf(samples: readonly Sample[], until: number): number {
  const recent = samples.filter((sample) => until - sample.at <= VELOCITY_WINDOW_MS);
  const first = recent[0];
  const last = recent.at(-1);
  if (first === undefined || last === undefined || last.at === first.at) return 0;
  return ((last.value - first.value) / (last.at - first.at)) * msPerSecond;
}
