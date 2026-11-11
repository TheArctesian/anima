import { breakpoint } from '$lib/motion/tokens';

/** The attribute on the root element while the navigation bar shows icons only; the stylesheet reads it. */
const ATTRIBUTE = 'data-nav';

/** What `watchIconOnly` needs from the window; a window without `matchMedia` is never compact. */
interface MediaWindow {
  readonly matchMedia?: Window['matchMedia'];
}

/** Why the bar shows icons only: the window is below the compact breakpoint, or the operator collapsed it. */
export interface IconOnlyState {
  readonly iconOnly: boolean;
  readonly compact: boolean;
}

/** The running watch: `collapse` records the operator's choice, `stop` stops following the window. */
export interface IconOnlyWatch {
  collapse(collapsed: boolean): void;
  stop(): void;
}

/** Below the compact breakpoint: `maxPx` is exclusive (5.5.3), so a fractional width under it still counts. */
const COMPACT_QUERY = `(max-width: ${String(breakpoint.compact.maxPx - 0.02)}px)`;

/**
 * Marks `root` with `data-nav="icons"` while the window is below `bp-compact` or the bar is collapsed, and follows the
 * window as it resizes. It is set before the first render, so crossing the breakpoint during load moves nothing
 * (MOT-007). `onChange` hears the state at once and on every change.
 */
export function watchIconOnly(
  root: HTMLElement,
  win: MediaWindow,
  collapsed: boolean,
  onChange?: (state: IconOnlyState) => void,
): IconOnlyWatch {
  const media = win.matchMedia?.(COMPACT_QUERY);
  let compact = media?.matches ?? false;
  let folded = collapsed;
  const apply = (): void => {
    const iconOnly = compact || folded;
    if (iconOnly) root.setAttribute(ATTRIBUTE, 'icons');
    else root.removeAttribute(ATTRIBUTE);
    onChange?.({ iconOnly, compact });
  };
  apply();
  const follow = (event: MediaQueryListEvent): void => {
    compact = event.matches;
    apply();
  };
  media?.addEventListener('change', follow);
  return {
    collapse(next) {
      folded = next;
      apply();
    },
    stop() {
      media?.removeEventListener('change', follow);
    },
  };
}
