import type { TransitionConfig } from 'svelte/transition';
import { easeCurve, springCurve } from '$lib/motion/animate';
import { modeTokens, springMotion } from '$lib/motion/options';
import { motionPreference, type MotionPreference } from '$lib/motion/preference';
import type { RoleTransition } from '$lib/motion/roles/transitions';
import type { Origin } from '../tabs';

export interface GrowParams {
  /** The control that opened the tab; `null` when it is not known. */
  readonly origin: Origin | null;
  readonly preference?: MotionPreference;
}

const fade = (t: number) => `opacity: ${String(t)};`;

/** `transform-origin` for `tab` at the centre of the control that opened it (MOT-005). */
export function originIn(origin: Origin, tab: DOMRectReadOnly): string {
  const x = origin.left + origin.width / 2 - tab.left;
  const y = origin.top + origin.height / 2 - tab.top;
  return `${String(x)}px ${String(y)}px`;
}

/** The tab starts at the control's width, so it reads as the control growing into the tab. */
function startScale(origin: Origin, tab: DOMRectReadOnly): number {
  return tab.width > 0 ? Math.min(Math.max(origin.width / tab.width, 0), 1) : 0;
}

function enter(node: Element, origin: Origin | null, reduced: boolean): TransitionConfig {
  const motion = springMotion('snappy', reduced);
  if (motion.type !== 'spring' || origin === null) {
    return { duration: modeTokens(reduced).duration.base, easing: easeCurve('enter'), css: fade };
  }
  const box = node.getBoundingClientRect();
  const from = startScale(origin, box);
  const at = originIn(origin, box);
  const { easing, durationMs } = springCurve(motion);
  return {
    duration: durationMs,
    easing,
    css: (t) => `${fade(t)} scale: ${String(from + (1 - from) * t)}; transform-origin: ${at};`,
  };
}

/** The tab closes to its own place in the strip; under reduced motion it only fades. */
function exit(reduced: boolean): TransitionConfig {
  const css = reduced ? fade : (t: number) => `${fade(t)} scale: ${String(t)};`;
  return { duration: modeTokens(reduced).duration.fast, easing: easeCurve('exit'), css };
}

/**
 * Session tab open (5.7.3): the tab grows from the control that opened it on `spring-snappy`
 * (MOT-005) and closes to its place in the strip over `duration-fast`. Reduced motion: fade.
 * One bidirectional transition, so a close mid-open starts from where the tab is (MOT-002).
 */
export function grow(node: Element, { origin, preference = motionPreference() }: GrowParams): RoleTransition {
  return ({ direction } = {}) => (direction === 'out' ? exit(preference.reduced) : enter(node, origin, preference.reduced));
}
