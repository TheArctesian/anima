import type { MetricRange } from '$lib/metrics/api';

/** The device detail tabs, in order (Section 6.7). */
export const TABS = [
  { name: 'overview', label: 'Overview' },
  { name: 'checks', label: 'Checks' },
  { name: 'sessions', label: 'Sessions' },
  { name: 'logs', label: 'Logs' },
  { name: 'settings', label: 'Settings' },
] as const;

export type TabName = (typeof TABS)[number]['name'];

/** The Settings place a header button opens (DEV-022): a disclosure section, or `test`, the SSH connection test (D-8). */
export type SettingsSection = 'general' | 'ssh' | 'rdp' | 'test';

/** The metric ranges of the Overview: the hub's enum, the URL word and the label of the choice (MET-003, D-5). */
export const RANGES = [
  { range: 'HOUR', param: 'hour', label: '1 hour' },
  { range: 'DAY', param: 'day', label: '24 hours' },
  { range: 'WEEK', param: 'week', label: '7 days' },
] as const satisfies readonly { range: MetricRange; param: string; label: string }[];

export interface TabPlace {
  readonly tab: TabName;
  /** Only on the Settings tab. */
  readonly section: SettingsSection | null;
  /** Only on the Overview tab; every other place reads as the 1-hour default. */
  readonly range: MetricRange;
}

const SECTIONS: readonly SettingsSection[] = ['general', 'ssh', 'rdp', 'test'];

function isTab(text: string | null): text is TabName {
  return TABS.some((tab) => tab.name === text);
}

function isSection(text: string | null): text is SettingsSection {
  return SECTIONS.some((section) => section === text);
}

/** D-5: the range word of the URL; an unknown word is the 1-hour default. */
function readRange(param: string | null): MetricRange {
  return RANGES.find((each) => each.param === param)?.range ?? 'HOUR';
}

/** D-9, D-5: the tab, section and range from the URL; an unknown tab is Overview, an unknown section is none, an unknown range is 1 hour. */
export function readPlace(params: URLSearchParams): TabPlace {
  const tab = params.get('tab');
  const section = params.get('section');
  if (!isTab(tab)) return { tab: 'overview', section: null, range: readRange(params.get('range')) };
  return {
    tab,
    section: tab === 'settings' && isSection(section) ? section : null,
    range: tab === 'overview' ? readRange(params.get('range')) : 'HOUR',
  };
}

/** D-9, D-5: the query string for a place, empty for Overview at 1 hour, so Back returns to it and a reload keeps it. */
export function placeSearch(place: TabPlace): string {
  if (place.tab === 'overview') {
    const word = RANGES.find((each) => each.range === place.range)?.param;
    return place.range === 'HOUR' || word === undefined ? '' : `?range=${word}`;
  }
  const params = new URLSearchParams({ tab: place.tab });
  if (place.tab === 'settings' && place.section !== null) params.set('section', place.section);
  return `?${params.toString()}`;
}

/** The id of a tab's button and of its panel, for `aria-controls` and `aria-labelledby`. */
export function tabIds(name: TabName): { tab: string; panel: string } {
  return { tab: `device-tab-${name}`, panel: `device-panel-${name}` };
}

/** Which ends of a scrolling strip have tabs past them: `start` when scrolled away from the beginning, `end` when more lies ahead. */
export function hiddenEdges(scrollStart: number, clientSize: number, scrollSize: number): { start: boolean; end: boolean } {
  return { start: scrollStart > 0, end: scrollStart + clientSize < scrollSize - 1 };
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
