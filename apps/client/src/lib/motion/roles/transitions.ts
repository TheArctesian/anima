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

/** A list item's place among the items that come in together (`stagger-list`). */
export interface ListItemParams extends RoleParams {
  readonly index?: number;
}

export type RoleTransition = (options?: { direction?: 'in' | 'out' }) => TransitionConfig;

const fade = (t: number) => `opacity: ${String(t)};`;

function scaledFade(fromScale: number, origin: string | undefined) {
  const at = origin === undefined ? '' : ` transform-origin: ${origin};`;
  return (t: number) => `${fade(t)} scale: ${String(fromScale + (1 - fromScale) * t)};${at}`;
}

/** `transform-origin` for `surface` at the centre of `control` (MOT-005). */
function originAt(control: Element, surface: Element): string {
  const from = control.getBoundingClientRect();
  const box = surface.getBoundingClientRect();
  const x = from.left + from.width / 2 - box.left;
  const y = from.top + from.height / 2 - box.top;
  return `${String(x)}px ${String(y)}px`;
}

function enter(spring: SpringName, fromScale: number, reduced: boolean, origin?: string): TransitionConfig {
  const motion = springMotion(spring, reduced);
  if (motion.type !== 'spring') {
    return { duration: modeTokens(reduced).duration.base, easing: easeCurve('enter'), css: fade };
  }
  const { easing, durationMs } = springCurve(motion);
  return { duration: durationMs, easing, css: scaledFade(fromScale, origin) };
}

function exit(toScale: number | undefined, reduced: boolean, origin?: string): TransitionConfig {
  const css = reduced || toScale === undefined ? fade : scaledFade(toScale, origin);
  return { duration: modeTokens(reduced).duration.fast, easing: easeCurve('exit'), css };
}

/**
 * A cross-fade over `duration-base` for page change, readings and the modal scrim (5.7.3,
 * MOT-008); the same under reduced motion.
 */
export function crossFade(_node: Element, { preference = motionPreference() }: RoleParams = {}): RoleTransition {
  return ({ direction } = {}) => ({
    duration: modeTokens(preference.reduced).duration.base,
    easing: easeCurve(direction === 'out' ? 'exit' : 'enter'),
    css: fade,
  });
}

/**
 * Menu and popover (5.7.3): enter scales 0.96 to 1 with a fade from the control on
 * `spring-snappy`; exit fades and scales to 0.98 over `duration-fast`. Reduced: fade only.
 */
export function popover(node: Element, { preference = motionPreference(), from }: PopoverParams = {}): RoleTransition {
  return ({ direction } = {}) => {
    const origin = from === undefined ? undefined : originAt(from, node);
    return direction === 'out'
      ? exit(roleScale.popoverExit, preference.reduced, origin)
      : enter('snappy', roleScale.popoverEnter, preference.reduced, origin);
  };
}

/**
 * Modal surface (5.7.3): enter scales 0.98 to 1 with a fade on `spring-default`; exit is the
 * same path in reverse over `duration-fast`. Reduced: fade only. The role's blur is left out
 * because MOT-006 allows only transform and opacity to animate.
 */
export function modal(_node: Element, { preference = motionPreference() }: RoleParams = {}): RoleTransition {
  return ({ direction } = {}) =>
    direction === 'out'
      ? exit(roleScale.modal, preference.reduced)
      : enter('default', roleScale.modal, preference.reduced);
}

/**
 * Reorder (5.7.3): a new item fades and scales from 0.98 on `spring-move` after its stagger
 * delay; a leaving item fades over `duration-fast` in its place. Reduced: fade only, no stagger.
 */
export function listItem(
  _node: Element,
  { preference = motionPreference(), index = 0 }: ListItemParams = {},
): RoleTransition {
  return ({ direction } = {}) => {
    if (direction === 'out') return exit(undefined, preference.reduced);
    const delay = staggerDelayMs(index, preference.reduced);
    return { ...enter('move', roleScale.listEnter, preference.reduced), delay };
  };
}
