import type { TabId } from '../tabs';

/** The id of the panel that shows tab `id`'s session, for `aria-controls` and `aria-labelledby`. */
export function panelId(id: TabId): string {
  return `session-panel-${id}`;
}

/** The id of tab `id`'s tab element. */
export function tabElementId(id: TabId): string {
  return `session-tab-${id}`;
}

/**
 * Where focus goes in the tab strip for `key` (APG tabs pattern, manual activation): the arrows
 * move to the neighbour and wrap, Home and End go to the ends. `null` for any other key.
 */
export function nextFocus(key: string, current: number, count: number): number | null {
  if (count === 0) return null;
  switch (key) {
    case 'ArrowRight':
      return (current + 1) % count;
    case 'ArrowLeft':
      return (current - 1 + count) % count;
    case 'Home':
      return 0;
    case 'End':
      return count - 1;
    default:
      return null;
  }
}
