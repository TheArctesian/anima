import type { MotionPreference } from '$lib/motion/preference';
import { flip, measure } from '$lib/motion/roles/flip';
import type { Slot } from './reorder';

/** A dragged tab drawn above the strip, and the neighbour whose margin holds its place. */
export interface Lift {
  readonly element: HTMLElement;
  readonly holder: HTMLElement | undefined;
  readonly margin: 'marginInlineStart' | 'marginInlineEnd';
}

const px = (value: number) => `${String(value)}px`;

/**
 * Takes the tab at `from` out of the strip's flow into a fixed layer at exactly its place on
 * screen, so the strip, which scrolls sideways and so clips, cannot hide it or its shadow. A
 * neighbour's margin keeps the place, so no other tab moves. `baseY` is the tab's translate.
 */
export function lift(tabs: readonly HTMLElement[], slots: readonly Slot[], from: number, baseY: number): Lift | undefined {
  const element = tabs[from];
  const slot = slots[from];
  if (element === undefined || slot === undefined) return undefined;
  const box = element.getBoundingClientRect();
  const next = tabs[from + 1];
  const nextSlot = slots[from + 1];
  const previous = tabs[from - 1];
  const previousSlot = slots[from - 1];
  let held: Lift = { element, holder: undefined, margin: 'marginInlineStart' };
  if (next !== undefined && nextSlot !== undefined) {
    next.style.marginInlineStart = px(nextSlot.left - slot.left);
    held = { element, holder: next, margin: 'marginInlineStart' };
  } else if (previous !== undefined && previousSlot !== undefined) {
    previous.style.marginInlineEnd = px(slot.left + slot.width - (previousSlot.left + previousSlot.width));
    held = { element, holder: previous, margin: 'marginInlineEnd' };
  }
  Object.assign(element.style, {
    position: 'fixed',
    left: px(slot.left),
    top: px(box.top - baseY),
    width: px(box.width),
    height: px(box.height),
  });
  return held;
}

/** Puts the tab back in the strip's flow; the caller then moves it to its place (FLIP). */
export function lower({ element, holder, margin }: Lift): void {
  Object.assign(element.style, { position: '', left: '', top: '', width: '', height: '' });
  if (holder !== undefined) holder.style[margin] = '';
}

/** The tab has left the strip: its place closes and the others move into it with `spring-move`. */
export function closeGap({ holder, margin }: Lift, others: readonly HTMLElement[], preference: MotionPreference): void {
  const places = measure(others);
  if (holder !== undefined) holder.style[margin] = '';
  flip(places, { spring: 'move', preference });
}
