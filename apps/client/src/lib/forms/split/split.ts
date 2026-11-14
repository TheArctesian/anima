import type { IconName } from '$lib/icons/icons';

/** One choice in a split button's menu. */
export interface SplitChoice {
  readonly id: string;
  readonly label: string;
  readonly icon?: IconName;
  /** A key the operator can press for it, shown at the end of the row (for example "⌘K"). */
  readonly shortcut?: string;
}

/** The menu item to focus after `key`, from `current` in `count` items; null when the key does not move focus. */
export function nextIndex(key: string, current: number, count: number): number | null {
  if (count === 0) return null;
  if (key === 'ArrowDown') return (current + 1) % count;
  if (key === 'ArrowUp') return (current - 1 + count) % count;
  if (key === 'Home') return 0;
  if (key === 'End') return count - 1;
  return null;
}
