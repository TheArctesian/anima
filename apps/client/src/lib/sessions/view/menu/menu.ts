/**
 * The menu item a key moves focus to (WAI-ARIA menu pattern): ArrowDown and ArrowUp step and wrap, Home and End
 * jump to the ends. Undefined for any other key, or when there are no items.
 */
export function menuFocus<T>(key: string, items: readonly T[], active: unknown): T | undefined {
  if (items.length === 0) return undefined;
  const at = items.indexOf(active as T);
  switch (key) {
    case 'ArrowDown':
      return items[(at + 1) % items.length];
    case 'ArrowUp':
      return items[at <= 0 ? items.length - 1 : at - 1];
    case 'Home':
      return items[0];
    case 'End':
      return items.at(-1);
    default:
      return undefined;
  }
}

/** Where a menu opens from its control: the side with room, and the height it may take there (CSS px). */
export interface MenuPlacement {
  readonly side: 'below' | 'above';
  readonly maxHeight: number;
}

/** The room a menu leaves between itself and the window edge, and between itself and its control (CSS px). */
export const MENU_MARGIN_PX = 8;

/**
 * Open below the control when the menu fits there or below has at least as much room, otherwise above. The menu is
 * no taller than the room on its side, so a short window scrolls it instead of clipping it (UI-N2).
 */
export function menuPlacement(control: { top: number; bottom: number }, viewportHeight: number, menuHeight: number): MenuPlacement {
  const below = viewportHeight - control.bottom - 2 * MENU_MARGIN_PX;
  const above = control.top - 2 * MENU_MARGIN_PX;
  const side = menuHeight <= below || below >= above ? 'below' : 'above';
  return { side, maxHeight: Math.max(0, side === 'below' ? below : above) };
}

/** Which edge of its control a menu lines up with, and the width it may take there (CSS px). */
export interface MenuAlignment {
  readonly align: 'start' | 'end';
  readonly maxWidth: number;
}

/**
 * Line the menu up with the control's start edge when it fits there or that side has at least as much room,
 * otherwise with its end edge. The menu is no wider than the room on its side, so a narrow window wraps it instead
 * of clipping it (UI-009).
 */
export function menuAlignment(control: { left: number; right: number }, viewportWidth: number, menuWidth: number): MenuAlignment {
  const start = viewportWidth - control.left - MENU_MARGIN_PX;
  const end = control.right - MENU_MARGIN_PX;
  const align = menuWidth <= start || start >= end ? 'start' : 'end';
  return { align, maxWidth: Math.max(0, align === 'start' ? start : end) };
}
