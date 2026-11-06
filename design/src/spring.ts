// Spring conversion and sampling (spec 5.7.2, D-4). Pure maths.

export interface SpringShape {
  readonly dampingRatio: number;
  readonly responseS: number;
}

export interface SpringStiffness extends SpringShape {
  readonly stiffness: number;
  readonly damping: number;
}

/** Samples in the CSS `linear()` curve of a spring. */
export const SAMPLE_COUNT = 40;
/** A spring has settled when its distance from the target stays below this. */
const SETTLE_BAND = 0.001;
const SEARCH_LIMIT_MS = 10_000;

/** stiffness = (2π ÷ response)², damping = 4π × ratio ÷ response, mass = 1. */
export function toLibraryForm(spring: SpringShape): SpringStiffness {
  const omega = (2 * Math.PI) / spring.responseS;
  return {
    ...spring,
    stiffness: omega * omega,
    damping: (4 * Math.PI * spring.dampingRatio) / spring.responseS,
  };
}

/** Position of a unit-mass spring released at 0 towards 1, after `seconds`. */
export function stepResponse(spring: SpringShape, seconds: number): number {
  const omega = (2 * Math.PI) / spring.responseS;
  const zeta = spring.dampingRatio;
  const decay = Math.exp(-zeta * omega * seconds);
  if (zeta >= 1) return 1 - decay * (1 + omega * seconds);
  const omegaDamped = omega * Math.sqrt(1 - zeta * zeta);
  const phase = omegaDamped * seconds;
  return 1 - decay * (Math.cos(phase) + ((zeta * omega) / omegaDamped) * Math.sin(phase));
}

/** First whole millisecond after which the spring stays within the settle band. */
export function settleMs(spring: SpringShape): number {
  let lastOutside = 0;
  for (let ms = 0; ms <= SEARCH_LIMIT_MS; ms += 1) {
    if (Math.abs(1 - stepResponse(spring, ms / 1000)) >= SETTLE_BAND) lastOutside = ms;
  }
  return lastOutside + 1;
}

/** Evenly spaced samples from 0 to the settle time; the last sample is exactly 1. */
export function sampleCurve(spring: SpringShape, count = SAMPLE_COUNT): number[] {
  const duration = settleMs(spring) / 1000;
  return Array.from({ length: count }, (_, index) => {
    if (index === 0) return 0;
    if (index === count - 1) return 1;
    return Number(stepResponse(spring, (duration * index) / (count - 1)).toFixed(4));
  });
}
