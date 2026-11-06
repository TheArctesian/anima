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

/** The tab's scale when gone (t = 0) and the point it scales about. */
interface Path {
  readonly atZero: number;
  readonly origin?: string;
}

const fade = (t: number) => `opacity: ${String(t)};`;

/** A closing tab shrinks to nothing about its own centre, in its place in the strip (5.7.3). */
const closing: Path = { atZero: 0 };

/** The tab starts at the control's width, so it reads as the control growing into the tab. */
function startScale(origin: Origin, tab: DOMRectReadOnly): number {
  return tab.width > 0 ? Math.min(Math.max(origin.width / tab.width, 0), 1) : 0;
}

/**
 * `transform-origin` in `tab` for which the tab, scaled to `scale`, is centred on the control
 * that opened it: the fixed point of that scale. The tab then grows from where the control is
 * along a straight line to its place in the strip (MOT-005). At scale 1 nothing moves, so the
 * control's centre serves.
 */
function originIn(origin: Origin, tab: DOMRectReadOnly, scale: number): string {
  const controlX = origin.left + origin.width / 2;
  const controlY = origin.top + origin.height / 2;
  const fixed = (control: number, centre: number) => (scale < 1 ? (control - scale * centre) / (1 - scale) : control);
  const x = fixed(controlX, tab.left + tab.width / 2) - tab.left;
  const y = fixed(controlY, tab.top + tab.height / 2) - tab.top;
  return `${String(x)}px ${String(y)}px`;
}

function opening(origin: Origin, tab: DOMRectReadOnly): Path {
  const atZero = startScale(origin, tab);
  return { atZero, origin: originIn(origin, tab, atZero) };
}

/** The style at progress `t` along `path`; with no path, a fade only. */
function draw(path: Path | null) {
  if (path === null) return fade;
  const at = path.origin === undefined ? '' : ` transform-origin: ${path.origin};`;
  return (t: number) => `${fade(t)} scale: ${String(path.atZero + (1 - path.atZero) * t)};${at}`;
}

/**
 * Session tab open (5.7.3): the tab grows from the control that opened it on `spring-snappy`
 * (MOT-005) and closes to its place in the strip over `duration-fast`. Reduced motion: fade.
 * One bidirectional transition whose path is fixed by its first direction, so a close mid-open
 * goes back to the control from where the tab is, with no step (MOT-002).
 */
export function grow(node: Element, { origin, preference = motionPreference() }: GrowParams): RoleTransition {
  const { reduced } = preference;
  let path: Path | null | undefined;
  return ({ direction } = {}) => {
    if (direction === 'out') {
      path ??= reduced ? null : closing;
      return { duration: modeTokens(reduced).duration.fast, easing: easeCurve('exit'), css: draw(path) };
    }
    const motion = springMotion('snappy', reduced);
    if (motion.type !== 'spring' || origin === null) {
      path ??= null;
      return { duration: modeTokens(reduced).duration.base, easing: easeCurve('enter'), css: draw(path) };
    }
    path ??= opening(origin, node.getBoundingClientRect());
    const { easing, durationMs } = springCurve(motion);
    return { duration: durationMs, easing, css: draw(path) };
  };
}
