import { animate, together, type Animatable, type Playback } from '../animate';
import { springMotion, type SpringName } from '../options';
import { motionPreference, type MotionPreference } from '../preference';

export type Places = ReadonlyMap<Animatable, DOMRectReadOnly>;

export interface FlipOptions {
  /** The spring to the new place; reorder and shared items use `move` (5.7.3). */
  readonly spring?: SpringName;
  /** Also animate size by scale, for disclosure; reorder moves without scale. */
  readonly scale?: boolean;
  readonly preference?: MotionPreference;
}

/** Where each element is on screen now, including any motion still in flight. Call before the DOM changes. */
export function measure(elements: Iterable<Animatable>): Map<Animatable, DOMRectReadOnly> {
  return new Map([...elements].map((element) => [element, element.getBoundingClientRect()]));
}

function ratio(before: number, after: number): number {
  return after === 0 ? 1 : before / after;
}

interface Offset {
  readonly x: number;
  readonly y: number;
}

/**
 * How far an element moved on screen: by its centre when it also scales, else by its top-left corner,
 * which is what a translate restores when its size changed too.
 */
function offset(before: DOMRectReadOnly, after: DOMRectReadOnly, scale: boolean): Offset {
  if (!scale) return { x: before.left - after.left, y: before.top - after.top };
  return {
    x: before.left + before.width / 2 - (after.left + after.width / 2),
    y: before.top + before.height / 2 - (after.top + after.height / 2),
  };
}

/** The offset of the nearest ancestor of `element` that moves too; its translate carries `element` along. */
function carried(element: Animatable, offsets: ReadonlyMap<Animatable, Offset>): Offset {
  for (let node = element.parentElement; node; node = node.parentElement) {
    const found = offsets.get(node);
    if (found) return found;
  }
  return { x: 0, y: 0 };
}

/**
 * Moves each measured element from its old place to its new one after a DOM change (FLIP).
 * An element inside another measured one moves by the difference only, as the outer one's
 * translate carries it. A second call mid-flight starts from where the element is on screen
 * (MOT-002); under reduced motion the elements are in place at once.
 */
export function flip(first: Places, options: FlipOptions = {}): Playback {
  const { spring = 'move', scale = false, preference = motionPreference() } = options;
  const moving = [...first].filter(([element]) => element.isConnected);
  for (const [element] of moving) {
    element.style.translate = '';
    if (scale) element.style.scale = '';
  }
  const last = moving.map(([element, before]) => ({ element, before, after: element.getBoundingClientRect() }));
  const offsets = new Map(last.map(({ element, before, after }) => [element, offset(before, after, scale)]));
  const motion = springMotion(spring, preference.reduced);
  return together(
    last.map(({ element, before, after }) => {
      const own = offsets.get(element) ?? { x: 0, y: 0 };
      const outer = carried(element, offsets);
      const x = own.x - outer.x;
      const y = own.y - outer.y;
      const from = scale ? { x: ratio(before.width, after.width), y: ratio(before.height, after.height) } : undefined;
      animate(element, { translate: { x, y }, scale: from }, { type: 'instant' });
      return animate(element, { translate: { x: 0, y: 0 }, scale: scale ? 1 : undefined }, motion);
    }),
  );
}
