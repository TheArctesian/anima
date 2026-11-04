import type { TransitionConfig } from 'svelte/transition';
import { easeCurve, springCurve } from '../animate';
import { modeTokens, springMotion, type SpringName } from '../options';
import { motionPreference, type MotionPreference } from '../preference';
import { roleScale } from './scales';

/** Parameters every role transition takes; the window's preference is the default (MOT-041). */
export interface RoleParams {
  readonly preference?: MotionPreference;
}

/** A menu or popover grows from the control that opened it (MOT-005). */
export interface PopoverParams extends RoleParams {
  readonly from?: Element;
}

interface Direction {
  readonly direction?: 'in' | 'out' | 'both';
}

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

function exit(toScale: number, reduced: boolean, origin?: string): TransitionConfig {
  const css = reduced ? fade : scaledFade(toScale, origin);
  return { duration: modeTokens(reduced).duration.fast, easing: easeCurve('exit'), css };
}

/**
 * A cross-fade over `duration-base` for page change, readings and the modal scrim (5.7.3,
 * MOT-008); the same under reduced motion.
 */
export function crossFade(
  _node: Element,
  { preference = motionPreference() }: RoleParams = {},
  options: Direction = {},
): TransitionConfig {
  return {
    duration: modeTokens(preference.reduced).duration.base,
    easing: easeCurve(options.direction === 'out' ? 'exit' : 'enter'),
    css: fade,
  };
}

/** Menu and popover enter: scale 0.96 to 1 and fade from the control with `spring-snappy`; reduced: fade. */
export function popoverIn(
  node: Element,
  { preference = motionPreference(), from }: PopoverParams = {},
): TransitionConfig {
  const origin = from === undefined ? undefined : originAt(from, node);
  return enter('snappy', roleScale.popoverEnter, preference.reduced, origin);
}

/** Menu and popover exit: fade and scale to 0.98 over `duration-fast`; reduced: fade. */
export function popoverOut(
  node: Element,
  { preference = motionPreference(), from }: PopoverParams = {},
): TransitionConfig {
  const origin = from === undefined ? undefined : originAt(from, node);
  return exit(roleScale.popoverExit, preference.reduced, origin);
}

/** Modal surface enter: scale 0.98 to 1 and fade with `spring-default`; reduced: fade. */
export function modalIn(_node: Element, { preference = motionPreference() }: RoleParams = {}): TransitionConfig {
  return enter('default', roleScale.modal, preference.reduced);
}

/** Modal surface exit: the enter path in reverse over `duration-fast`; reduced: fade. */
export function modalOut(_node: Element, { preference = motionPreference() }: RoleParams = {}): TransitionConfig {
  return exit(roleScale.modal, preference.reduced);
}
