import { hiddenEdges } from './edges';

/** The ends of a horizontal strip that have content past them. */
export interface StripEdges {
  readonly start: boolean;
  readonly end: boolean;
}

export const NO_HIDDEN_EDGES: StripEdges = { start: false, end: false };

/** Part of the visible width that one arrow press scrolls, so the last tab seen stays in view as a landmark (NAV-005). */
const STEP_SHARE = 0.75;

/** How far an arrow scrolls a strip of `clientWidth`, signed by its direction. */
export function stripStep(clientWidth: number, direction: 'start' | 'end'): number {
  const step = Math.round(clientWidth * STEP_SHARE);
  return direction === 'start' ? -step : step;
}

/**
 * An attachment for a horizontally scrolling strip of tabs: it marks `data-strip-fade` always, and `data-more-before`
 * and `data-more-after` while tabs lie past that end, so `strip-fade.css` fades the end; `onEdges` hears each change.
 * It follows scrolling, the strip's size and the size of each tab, also of a tab that joins later.
 */
export function stripEdges(onEdges: (edges: StripEdges, strip: HTMLElement) => void): (strip: HTMLElement) => () => void {
  return (strip) => {
    let last = '';
    const mark = (): void => {
      const edges = hiddenEdges(strip.scrollLeft, strip.clientWidth, strip.scrollWidth);
      strip.toggleAttribute('data-more-before', edges.start);
      strip.toggleAttribute('data-more-after', edges.end);
      const key = `${String(edges.start)}:${String(edges.end)}:${String(strip.clientWidth)}:${String(strip.scrollWidth)}`;
      if (key === last) return;
      last = key;
      onEdges(edges, strip);
    };
    strip.setAttribute('data-strip-fade', '');
    mark();
    strip.addEventListener('scroll', mark, { passive: true });
    const resized = typeof ResizeObserver === 'function' ? new ResizeObserver(mark) : undefined;
    const watch = (): void => {
      resized?.observe(strip);
      for (const child of strip.children) resized?.observe(child);
    };
    watch();
    const joined = typeof MutationObserver === 'function' ? new MutationObserver(() => {
      watch();
      mark();
    }) : undefined;
    joined?.observe(strip, { childList: true });
    return () => {
      strip.removeEventListener('scroll', mark);
      resized?.disconnect();
      joined?.disconnect();
    };
  };
}
