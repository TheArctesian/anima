/** How far the button may drift, in pixels, before the open menu counts as left behind (sub-pixel layout settling is not a move). */
export const ANCHOR_TOLERANCE_PX = 2;

interface Anchor {
  readonly top: number;
  readonly left: number;
}

/** Where the button was when the menu opened and how wide the window was; the panel is fixed there (Section 5.7.3). */
export interface MenuAnchor extends Anchor {
  readonly viewportWidth: number;
}

export function anchorOf(button: Pick<DOMRect, 'top' | 'left'>, viewportWidth: number): MenuAnchor {
  return { top: button.top, left: button.left, viewportWidth };
}

/**
 * Whether a scroll left the panel away from its button. Browsers also fire a document `scroll` after a click with no scroll
 * input, and mobile address bars fire `resize`; neither moves the button, so neither closes the menu (WEB-011, MOT-002).
 */
export function menuLeftBehind(opened: MenuAnchor, button: Pick<DOMRect, 'top' | 'left'>, viewportWidth: number, event: 'scroll' | 'resize'): boolean {
  if (event === 'resize') return viewportWidth !== opened.viewportWidth;
  return Math.abs(button.top - opened.top) > ANCHOR_TOLERANCE_PX || Math.abs(button.left - opened.left) > ANCHOR_TOLERANCE_PX;
}
