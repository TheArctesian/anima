import { nearestSnap, rubberband } from '$lib/motion/gesture';

/** The bar's two widths, in px, measured from the layout at the grab. */
export interface BarWidths {
  readonly icons: number;
  readonly full: number;
}

/**
 * The width the bar shows for a dragged edge at `position` px: the pointer's place between the two widths, with
 * rubber-band resistance below the icon width and above the full width (5.7.4, MOT-033).
 */
export function draggedWidth(position: number, widths: BarWidths): number {
  if (position < widths.icons) return widths.icons + rubberband(position - widths.icons, widths.icons);
  if (position > widths.full) return widths.full + rubberband(position - widths.full, widths.full);
  return position;
}

/** Whether the bar comes to rest collapsed: the width nearest where the release would carry it (MOT-032, MOT-034). */
export function restsCollapsed(position: number, velocityPxPerS: number, widths: BarWidths): boolean {
  return nearestSnap(position, velocityPxPerS, [widths.icons, widths.full]) === widths.icons;
}

const WIDTH = '--nav-inline-size';

/** Holds the bar at `width` px while the edge is dragged; the page follows, as the hand does (5.7.4). */
export function holdWidth(root: HTMLElement, width: number): void {
  root.style.setProperty(WIDTH, `${String(width)}px`);
}

/** Gives the bar back to the width its state sets. */
export function releaseWidth(root: HTMLElement): void {
  root.style.removeProperty(WIDTH);
}
