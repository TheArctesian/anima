import type { Attachment } from 'svelte/attachments';
import { animate } from '$lib/motion/animate';
import { rubberband } from '$lib/motion/gesture';
import { springMotion } from '$lib/motion/options';
import { motionPreference, type MotionPreference } from '$lib/motion/preference';
import { flip, measure } from '$lib/motion/roles/flip';
import { threshold } from '$lib/motion/tokens';
import { VELOCITY_WINDOW_MS, isOutside, shifts, slotIndex, velocityOf, type Bounds, type Sample, type Slot, type Velocity } from './reorder';

/** What a dragged tab needs from its strip. */
export interface DragOptions {
  /** The tabs in strip order, the dragged one among them. */
  readonly tabs: () => readonly HTMLElement[];
  /** The strip's bounds; a release beyond them is a drag out. */
  readonly bounds: () => Bounds;
  /** Whether a drag out may pop the session out (RA-002, D-13). */
  readonly canDragOut: () => boolean;
  /** Moves the tab at `from` to `to` and updates the DOM before it returns. */
  readonly drop: (from: number, to: number) => void;
  /** Pops the session out; resolves `false` when it stayed, so the tab goes back to its place. */
  readonly dragOut: () => Promise<boolean>;
  readonly preference?: MotionPreference;
  /** The clock for the release velocity, in milliseconds. */
  readonly now?: () => number;
}

interface Grab {
  readonly pointerId: number;
  readonly downX: number;
  readonly downY: number;
  readonly samples: Sample[];
  drag?: Drag;
}

interface Drag {
  readonly tabs: readonly HTMLElement[];
  readonly slots: readonly Slot[];
  readonly from: number;
  /** The translate on screen at the grab, so the grab offset holds mid-motion (MOT-030). */
  readonly baseX: number;
  readonly baseY: number;
  readonly height: number;
  over: number;
  centre: number;
}

const noDrag = '[data-no-drag]';

function translateOf(element: Element): [number, number] {
  const [x = 0, y = 0] = getComputedStyle(element)
    .translate.split(/\s+/)
    .map((part) => Number.parseFloat(part))
    .map((value) => (Number.isNaN(value) ? 0 : value));
  return [x, y];
}

function begin(element: HTMLElement, grab: Grab, tabs: readonly HTMLElement[]): Drag {
  const [baseX, baseY] = translateOf(element);
  const slots = tabs.map((tab) => {
    const box = tab.getBoundingClientRect();
    return { left: box.left - (tab === element ? baseX : translateOf(tab)[0]), width: box.width };
  });
  const from = tabs.indexOf(element);
  element.setPointerCapture(grab.pointerId);
  element.dataset.dragging = '';
  const height = element.getBoundingClientRect().height;
  return { tabs, slots, from, baseX, baseY, height, over: from, centre: 0 };
}

function moveAside(drag: Drag, over: number, reduced: boolean): void {
  const offsets = shifts(drag.slots, drag.from, over);
  drag.tabs.forEach((tab, index) => {
    if (index !== drag.from) animate(tab, { translate: { x: offsets[index] ?? 0 } }, springMotion('move', reduced));
  });
}

/** The tab lands at `to`: its spring starts with the pointer's velocity on each axis (MOT-031, MOT-035). */
function land(element: HTMLElement, drag: Drag, to: number, settings: Release): void {
  const { options, preference } = settings;
  const reduced = preference.reduced;
  const places = measure(drag.tabs);
  const before = places.get(element);
  places.delete(element);
  options.drop(drag.from, to);
  element.style.translate = '';
  const after = element.getBoundingClientRect();
  if (before !== undefined) {
    const x = before.left - after.left;
    const y = before.top - after.top;
    animate(element, { translate: { x, y } }, { type: 'instant' });
  }
  const velocity = settings.velocity;
  animate(element, { translate: { x: 0 } }, springMotion('momentum', reduced, velocity.x));
  animate(element, { translate: { y: 0 } }, springMotion('momentum', reduced, velocity.y));
  flip(places, { spring: 'move', preference });
}

interface Settings {
  readonly options: DragOptions;
  readonly preference: MotionPreference;
}

interface Release extends Settings {
  readonly velocity: Velocity;
}

function release(element: HTMLElement, grab: Grab, drag: Drag, settings: Release): void {
  const { options, preference } = settings;
  const last = grab.samples.at(-1);
  if (last !== undefined && options.canDragOut() && isOutside(last.x, last.y, options.bounds(), threshold.drag)) {
    moveAside(drag, drag.from, preference.reduced);
    void options.dragOut().then((gone) => {
      if (!gone) land(element, drag, drag.from, settings);
    });
    return;
  }
  land(element, drag, slotIndex(drag.slots, drag.centre, settings.velocity.x), settings);
}

function follow(element: HTMLElement, grab: Grab, drag: Drag, { options, preference }: Settings): void {
  const last = grab.samples.at(-1);
  if (last === undefined) return;
  const x = drag.baseX + last.x - grab.downX;
  const dy = last.y - grab.downY;
  const y = drag.baseY + (options.canDragOut() ? dy : rubberband(dy, drag.height));
  animate(element, { translate: { x, y } }, { type: 'instant' });
  const slot = drag.slots[drag.from];
  drag.centre = (slot?.left ?? 0) + x + (slot?.width ?? 0) / 2;
  const over = slotIndex(drag.slots, drag.centre, 0);
  if (over === drag.over) return;
  drag.over = over;
  moveAside(drag, over, preference.reduced);
}

/** Adds `latest` and drops the samples too old to count for the velocity, so the list stays short. */
function keepRecent(samples: Sample[], latest: Sample): void {
  samples.push(latest);
  const recent = samples.findIndex((sample) => latest.t - sample.t <= VELOCITY_WINDOW_MS);
  samples.splice(0, recent);
}

function travelled(grab: Grab, event: PointerEvent): number {
  return Math.hypot(event.clientX - grab.downX, event.clientY - grab.downY);
}

/**
 * Reorder by drag (Section 5.7.4): after `threshold-drag` the tab follows the pointer from the
 * grab point with pointer capture (MOT-030), the others move aside with `spring-move`, and on
 * release the tab goes to the place nearest its projected position (MOT-032). Under reduced
 * motion every change of place is instant (MOT-040).
 */
export function dragTab(options: DragOptions): Attachment<HTMLElement> {
  const { preference = motionPreference(), now = () => performance.now() } = options;
  const settings: Settings = { options, preference };
  return (element) => {
    let grab: Grab | undefined;
    let swallowClick = false;
    const sample = (event: PointerEvent) => {
      if (grab) keepRecent(grab.samples, { x: event.clientX, y: event.clientY, t: now() });
    };
    const down = (event: PointerEvent) => {
      if (event.button !== 0 || (event.target instanceof Element && event.target.closest(noDrag))) return;
      swallowClick = false;
      grab = { pointerId: event.pointerId, downX: event.clientX, downY: event.clientY, samples: [] };
      sample(event);
    };
    const move = (event: PointerEvent) => {
      if (grab?.pointerId !== event.pointerId) return;
      sample(event);
      if (!grab.drag && travelled(grab, event) < threshold.drag) return;
      grab.drag ??= begin(element, grab, options.tabs());
      follow(element, grab, grab.drag, settings);
    };
    const up = (event: PointerEvent) => {
      if (grab?.pointerId !== event.pointerId) return;
      const { drag } = grab;
      sample(event);
      if (drag) {
        const velocity = event.type === 'pointercancel' ? { x: 0, y: 0 } : velocityOf(grab.samples);
        delete element.dataset.dragging;
        swallowClick = true;
        if (event.type === 'pointercancel') land(element, drag, drag.from, { ...settings, velocity });
        else release(element, grab, drag, { ...settings, velocity });
      }
      grab = undefined;
    };
    const click = (event: MouseEvent) => {
      if (!swallowClick) return;
      swallowClick = false;
      event.stopPropagation();
      event.preventDefault();
    };
    const listeners = [
      ['pointerdown', down],
      ['pointermove', move],
      ['pointerup', up],
      ['pointercancel', up],
    ] as const;
    for (const [name, listener] of listeners) element.addEventListener(name, listener);
    element.addEventListener('click', click, true);
    return () => {
      for (const [name, listener] of listeners) element.removeEventListener(name, listener);
      element.removeEventListener('click', click, true);
    };
  };
}
