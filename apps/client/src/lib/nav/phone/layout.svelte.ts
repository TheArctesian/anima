import { MediaQuery } from 'svelte/reactivity';
import { breakpoint } from '$lib/motion/tokens';

/** Below the compact breakpoint; `maxPx` is exclusive (5.5.3). `bar.css` and the root layout use the same width. */
const QUERY = `max-width: ${String(breakpoint.compact.maxPx - 0.02)}px`;

let phone: MediaQuery | undefined;

/**
 * Whether the window has the phone layout now (MOB-010): the navigation bar is at the bottom of the screen and the logs
 * are a page, not a panel. It follows the window as it changes size. A window without `matchMedia` is never a phone.
 */
export function phoneLayout(): boolean {
  if (typeof window.matchMedia !== 'function') return false;
  phone ??= new MediaQuery(QUERY, false);
  return phone.current;
}
