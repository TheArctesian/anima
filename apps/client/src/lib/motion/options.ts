import { duration, ease, gesture, reducedMotion, spring, stagger, threshold } from './tokens';

export type SpringName = keyof typeof spring;
export type DurationName = keyof typeof duration;
export type EaseName = keyof typeof ease;

/** A spring for motion the operator can touch or interrupt (MOT-003); `velocity` is in units per second (MOT-031). */
export interface SpringMotion {
  readonly type: 'spring';
  readonly stiffness: number;
  readonly damping: number;
  readonly mass: number;
  readonly velocity?: number;
}

/** A duration and a curve, only for opacity and colour (MOT-003); `durationMs` is in milliseconds. */
export interface TweenMotion {
  readonly type: 'tween';
  readonly durationMs: number;
  readonly ease: (typeof ease)[EaseName];
}

/** A change in one frame, with no movement (MOT-040). */
export interface InstantMotion {
  readonly type: 'instant';
}

export type Motion = SpringMotion | TweenMotion | InstantMotion;

/** The token build assumes mass 1 when it converts damping ratio and response (5.7.2). */
const springMass = 1;

const instant: InstantMotion = { type: 'instant' };

/** The token tables that apply in one mode; thresholds and gesture constants are the same in both (MOT-021). */
export function modeTokens(reduced: boolean) {
  return {
    duration: reduced ? reducedMotion.duration : duration,
    stagger: reduced ? reducedMotion.stagger : stagger,
    threshold,
    gesture,
  };
}

/** The spring for a K5 name; under reduced motion springs are instant (D-5, MOT-041). */
export function springMotion(name: SpringName, reduced: boolean, velocity?: number): SpringMotion | InstantMotion {
  if (reduced) return instant;
  const { stiffness, damping } = spring[name];
  const base = { type: 'spring', stiffness, damping, mass: springMass } as const;
  return velocity === undefined ? base : { ...base, velocity };
}

/** A tween for opacity or colour with the mode's duration; a zero duration is instant. */
export function tweenMotion(name: DurationName, curve: EaseName, reduced: boolean): TweenMotion | InstantMotion {
  const durationMs = modeTokens(reduced).duration[name];
  return durationMs === 0 ? instant : { type: 'tween', durationMs, ease: ease[curve] };
}

/** Delay before list item `index` comes in (`stagger-list`); items past the cap share the last delay. */
export function staggerDelayMs(index: number, reduced: boolean): number {
  const { intervalMs, maxItems } = modeTokens(reduced).stagger.list;
  return Math.min(Math.max(index, 0), maxItems - 1) * intervalMs;
}
