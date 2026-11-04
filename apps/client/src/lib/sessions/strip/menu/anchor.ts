import type { Attachment } from 'svelte/attachments';

/** Where a menu sits under its control, in viewport pixels (`position: fixed`). */
export interface Placement {
  readonly left: number;
  readonly top: number;
}

/**
 * The menu opens below `control`, aligned to its left edge, but moved left as far as needed to
 * stay `space` inside the viewport's right edge, and never closer than `space` to the left one
 * (UI-009).
 */
export function placeUnder(control: DOMRectReadOnly, menuWidth: number, viewportWidth: number, space: number): Placement {
  const furthest = viewportWidth - menuWidth - space;
  return { left: Math.max(space, Math.min(control.left, furthest)), top: control.bottom };
}

/** The `--space-2` token in pixels, the gap the menu keeps from the viewport's edges. */
function edgeSpace(node: Element): number {
  const value = Number.parseFloat(getComputedStyle(node).getPropertyValue('--space-2'));
  return Number.isNaN(value) ? 0 : value;
}

/**
 * Places the menu under its control once it is in the page and can be measured. It runs before
 * the popover role measures the menu, so the role's `transform-origin` is the control's centre
 * relative to where the menu ends up (MOT-005).
 */
export function under(control: () => Element | undefined): Attachment<HTMLElement> {
  return (node) => {
    const anchor = control();
    if (anchor === undefined) return;
    const width = node.getBoundingClientRect().width;
    const { left, top } = placeUnder(anchor.getBoundingClientRect(), width, window.innerWidth, edgeSpace(node));
    node.style.left = `${String(left)}px`;
    node.style.top = `${String(top)}px`;
  };
}

/**
 * Moves the menu to the end of `document.body` while it is attached, so no scroll container
 * clips it or scrolls to show it (MOT-007, UI-009). The tab strip scrolls sideways.
 */
export const portal: Attachment<HTMLElement> = (node) => {
  document.body.append(node);
  return () => {
    node.remove();
  };
};

/** Keeps a press on an icon button from moving focus into it: the button is `aria-hidden` (UI-015). */
export function keepFocus(event: Event): void {
  event.preventDefault();
}
