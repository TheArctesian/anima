import { breakpoint } from '$lib/motion/tokens';

/** The attribute on the root element while the navigation bar shows icons only; the stylesheet reads it. */
const ATTRIBUTE = 'data-nav';

/** What `watchIconOnly` needs from the window; a window without `matchMedia` keeps the full bar. */
interface MediaWindow {
  readonly matchMedia?: Window['matchMedia'];
}

/** Below the compact breakpoint: `maxPx` is exclusive (5.5.3), so a fractional width under it still counts. */
const COMPACT_QUERY = `(max-width: ${String(breakpoint.compact.maxPx - 0.02)}px)`;

/**
 * Marks `root` with `data-nav="icons"` while the window is below `bp-compact`, and follows the window as it resizes.
 * It is set before the first render, so crossing the breakpoint during load moves nothing (MOT-007).
 * Returns the function that stops watching.
 */
export function watchIconOnly(root: HTMLElement, win: MediaWindow): () => void {
  const media = win.matchMedia?.(COMPACT_QUERY);
  if (media === undefined) return () => undefined;
  const apply = (iconOnly: boolean): void => {
    if (iconOnly) root.setAttribute(ATTRIBUTE, 'icons');
    else root.removeAttribute(ATTRIBUTE);
  };
  apply(media.matches);
  const onChange = (event: MediaQueryListEvent): void => {
    apply(event.matches);
  };
  media.addEventListener('change', onChange);
  return () => {
    media.removeEventListener('change', onChange);
  };
}
