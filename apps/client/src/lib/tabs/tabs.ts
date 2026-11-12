import type { IconName } from '$lib/icons/icons';

/** One underlined tab: the word is always shown (UI-003); the count and the icon are optional. */
export interface TabItem {
  readonly id: string;
  readonly label: string;
  readonly count?: number;
  readonly icon?: IconName;
}

/** The id of a tab's element and of its panel, for `aria-controls` and `aria-labelledby`. */
export function tabIds(prefix: string, id: string): { tab: string; panel: string } {
  return { tab: `${prefix}-tab-${id}`, panel: `${prefix}-panel-${id}` };
}

/** An interval along the strip: where it starts and how long it is. */
interface Span {
  readonly start: number;
  readonly size: number;
}

/** The scroll position that shows `tab` inside the visible `view`, `margin` clear of the edge fades, moving as little as it can. */
export function scrolledToShow(view: Span, tab: Span, margin: number): number {
  if (tab.start - margin < view.start) return Math.max(0, tab.start - margin);
  const tabEnd = tab.start + tab.size + margin;
  return tabEnd > view.start + view.size ? tabEnd - view.size : view.start;
}
