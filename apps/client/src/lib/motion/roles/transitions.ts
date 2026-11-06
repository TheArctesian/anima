import type { TransitionConfig } from 'svelte/transition';
import { easeCurve, springCurve } from '../animate';
import { modeTokens, springMotion, staggerDelayMs, type SpringName } from '../options';
import { motionPreference, type MotionPreference } from '../preference';
import { roleScale } from './scales';

/*
 * Each role is one bidirectional transition: use it with `transition:`, not a pair of `in:`
 * and `out:`, so that a reversal mid-flight starts from where the element is (MOT-002).
 * Svelte asks the returned function for the configuration of each direction once the DOM
 * has been updated.
 */

/** Parameters every role transition takes; the window's preference is the default (MOT-041). */
export interface RoleParams {
  readonly preference?: MotionPreference;
}

/** A menu or popover grows from the control that opened it (MOT-005). */
export interface PopoverParams extends RoleParams {
  readonly from?: Element;
}

/** A row's place among the rows that come in together (`stagger-list`). */
export interface ListRowParams extends RoleParams {
  readonly index?: number;
}

export type RoleTransition = (options?: { direction?: 'in' | 'out' }) => TransitionConfig;

const opacity = (t: number) => `opacity: ${String(t)};`;

/** A fade with a scale from `atZero` when gone to 1 in place; with no scale, a fade only. */
function scaledFade(atZero: number | undefined, origin: string | undefined) {
  if (atZero === undefined) return opacity;
  const at = origin === undefined ? '' : ` transform-origin: ${origin};`;
  return (t: number) => `${opacity(t)} scale: ${String(atZero + (1 - atZero) * t)};${at}`;
}

/**
 * The scale an element has when gone, fixed by the first direction of one transition. A
 * reversal mid-flight keeps it, so it goes back on the path it came on with no step in scale
 * (MOT-002, MOT-005); Svelte keeps one transition's options until it ends.
 */
function firstScale(): (own: number | undefined) => number | undefined {
  let first: { readonly atZero: number | undefined } | undefined;
  return (own) => (first ??= { atZero: own }).atZero;
}

/** `transform-origin` for `surface` at the centre of `control` (MOT-005). */
function originAt(control: Element, surface: Element): string {
  const from = control.getBoundingClientRect();
  const box = surface.getBoundingClientRect();
  const x = from.left + from.width / 2 - box.left;
  const y = from.top + from.height / 2 - box.top;
  return `${String(x)}px ${String(y)}px`;
}

function enter(spring: SpringName, atZero: number | undefined, reduced: boolean, origin?: string): TransitionConfig {
  const motion = springMotion(spring, reduced);
  if (motion.type !== 'spring') {
    return { duration: modeTokens(reduced).duration.base, easing: easeCurve('enter'), css: opacity };
  }
  const { easing, durationMs } = springCurve(motion);
  return { duration: durationMs, easing, css: scaledFade(atZero, origin) };
}

function exit(atZero: number | undefined, reduced: boolean, origin?: string): TransitionConfig {
  const css = reduced ? opacity : scaledFade(atZero, origin);
  return { duration: modeTokens(reduced).duration.fast, easing: easeCurve('exit'), css };
}

/**
 * A cross-fade over `duration-base` for page change, status, readings and loaded data (5.7.3,
 * MOT-008); the same under reduced motion. The outgoing and the incoming layer run on one
 * curve, `ease-enter`, so their opacities sum to 1 at every frame and the two layers never
 * show at full strength together. `ease-exit` is for an exit that nothing replaces.
 */
export function crossFade(_node: Element, { preference = motionPreference() }: RoleParams = {}): RoleTransition {
  return () => ({
    duration: modeTokens(preference.reduced).duration.base,
    easing: easeCurve('enter'),
    css: opacity,
  });
}

/**
 * A fade for a layer that nothing replaces, such as a modal's scrim or a banner (5.7.3 Modal
 * and Toast): in over `duration-base` with `ease-enter`, out over `duration-fast` with
 * `ease-exit`. The same under reduced motion.
 */
export function fade(_node: Element, { preference = motionPreference() }: RoleParams = {}): RoleTransition {
  return ({ direction } = {}) => {
    const { duration } = modeTokens(preference.reduced);
    return direction === 'out'
      ? { duration: duration.fast, easing: easeCurve('exit'), css: opacity }
      : { duration: duration.base, easing: easeCurve('enter'), css: opacity };
  };
}

/**
 * Menu and popover (5.7.3): enter scales 0.96 to 1 with a fade from the control on
 * `spring-snappy`; exit fades and scales to 0.98 over `duration-fast`. Reduced: fade only.
 */
export function popover(node: Element, { preference = motionPreference(), from }: PopoverParams = {}): RoleTransition {
  const atZero = firstScale();
  return ({ direction } = {}) => {
    const origin = from === undefined ? undefined : originAt(from, node);
    return direction === 'out'
      ? exit(atZero(roleScale.popoverExit), preference.reduced, origin)
      : enter('snappy', atZero(roleScale.popoverEnter), preference.reduced, origin);
  };
}

/**
 * Modal surface (5.7.3): enter scales 0.98 to 1 with a fade on `spring-default`; exit is the
 * same path in reverse over `duration-fast`. Reduced: fade only. The role's blur is left out
 * because MOT-006 allows only transform and opacity to animate.
 */
export function modal(_node: Element, { preference = motionPreference() }: RoleParams = {}): RoleTransition {
  const atZero = firstScale();
  return ({ direction } = {}) =>
    direction === 'out'
      ? exit(atZero(roleScale.modal), preference.reduced)
      : enter('default', atZero(roleScale.modal), preference.reduced);
}

/**
 * Reorder (5.7.3): a new row fades and scales from 0.98 on `spring-move` after its stagger
 * delay; a leaving row fades over `duration-fast` in its place. Reduced: fade only, no stagger.
 */
export function listRow(
  _node: Element,
  { preference = motionPreference(), index = 0 }: ListRowParams = {},
): RoleTransition {
  const atZero = firstScale();
  return ({ direction } = {}) => {
    if (direction === 'out') return exit(atZero(undefined), preference.reduced);
    const delay = staggerDelayMs(index, preference.reduced);
    return { ...enter('move', atZero(roleScale.listEnter), preference.reduced), delay };
  };
}
