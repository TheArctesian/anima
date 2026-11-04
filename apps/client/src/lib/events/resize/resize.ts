import { nearestSnap, rubberband } from '$lib/motion/gesture';

/** The panel's three resting heights in px: collapsed, 33 % and 66 % of the page (5.7.4). */
export type Snaps = readonly [collapsed: number, third: number, twoThirds: number];

/** The index of a resting height in `Snaps`. */
export type Stop = 0 | 1 | 2;

export const COLLAPSED: Stop = 0;
export const THIRD: Stop = 1;
export const TWO_THIRDS: Stop = 2;

/** The page shares of the 33 % and 66 % stops; `geometry.css` has the same shares in vh (`--logs-third`, `--logs-two-thirds`), so change them together. */
const THIRD_SHARE = 0.33;
const TWO_THIRDS_SHARE = 0.66;

/** The resting heights for a page `pageHeight` tall whose collapsed panel is `collapsed` tall. */
export function snapsOf(collapsed: number, pageHeight: number): Snaps {
  return [collapsed, Math.max(pageHeight * THIRD_SHARE, collapsed), Math.max(pageHeight * TWO_THIRDS_SHARE, collapsed)];
}

/** The height shown while dragging to `raw`: 1:1 between the limits, rubber-band resistance beyond them (MOT-033). */
export function draggedHeight(raw: number, snaps: Snaps): number {
  const [low, , high] = snaps;
  const size = high - low;
  if (size <= 0) return low;
  if (raw < low) return low + rubberband(raw - low, size);
  if (raw > high) return high + rubberband(raw - high, size);
  return raw;
}

/**
 * Where a released panel comes to rest (MOT-032, MOT-034). `velocity` is the speed of the panel's top edge going up, in px/s;
 * the nearest snap to the projected height wins, so a flick toward the page edge decides open or close.
 */
export function releasedStop(height: number, velocity: number, snaps: Snaps): Stop {
  const resting = nearestSnap(height, velocity, snaps);
  return snaps.indexOf(resting) as Stop;
}

/** The stop a key leads to, or null when the key does nothing: Up opens one step, Down closes one, Home collapses, End opens fully. */
export function keyStop(key: string, from: Stop): Stop | null {
  switch (key) {
    case 'ArrowUp':
      return Math.min(from + 1, TWO_THIRDS) as Stop;
    case 'ArrowDown':
      return Math.max(from - 1, COLLAPSED) as Stop;
    case 'Home':
      return COLLAPSED;
    case 'End':
      return TWO_THIRDS;
    default:
      return null;
  }
}
