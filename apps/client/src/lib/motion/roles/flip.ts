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

/**
 * Moves each measured element from its old place to its new one after a DOM change (FLIP).
 * A second call mid-flight starts from where the element is on screen (MOT-002); under
 * reduced motion the elements are in place at once.
 */
export function flip(first: Places, options: FlipOptions = {}): Playback {
  const { spring = 'move', scale = false, preference = motionPreference() } = options;
  const moving = [...first].filter(([element]) => element.isConnected);
  for (const [element] of moving) {
    element.style.translate = '';
    if (scale) element.style.scale = '';
  }
  const last = moving.map(([element, before]) => ({ element, before, after: element.getBoundingClientRect() }));
  const motion = springMotion(spring, preference.reduced);
  return together(
    last.map(({ element, before, after }) => {
      const x = before.left + before.width / 2 - (after.left + after.width / 2);
      const y = before.top + before.height / 2 - (after.top + after.height / 2);
      const from = scale ? { x: ratio(before.width, after.width), y: ratio(before.height, after.height) } : undefined;
      animate(element, { translate: { x, y }, scale: from }, { type: 'instant' });
      return animate(element, { translate: { x: 0, y: 0 }, scale: scale ? 1 : undefined }, motion);
    }),
  );
}
