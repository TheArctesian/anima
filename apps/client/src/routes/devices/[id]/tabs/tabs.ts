import type { IconName } from '$lib/icons/icons';
import type { MetricRange } from '$lib/metrics/api';
import { RANGES } from '$lib/metrics/ranges';
import { tabIds as sharedIds, type TabSpec } from '$lib/tabs/tabs';

/** The device detail tabs, in order (Section 6.7), each with the icon shown before its label. */
export const TABS = [
  { name: 'overview', label: 'Overview', icon: 'nf-md-speedometer' },
  { name: 'checks', label: 'Checks', icon: 'nf-md-clipboard_check_outline' },
  { name: 'sessions', label: 'Sessions', icon: 'nf-md-history' },
  { name: 'logs', label: 'Logs', icon: 'nf-md-text_box_outline' },
  { name: 'settings', label: 'Settings', icon: 'nf-md-cog_outline' },
] as const satisfies readonly { readonly name: string; readonly label: string; readonly icon: IconName }[];

export type TabName = (typeof TABS)[number]['name'];

/** The Settings place a header button opens (DEV-022): a disclosure section, or `test`, the SSH connection test (D-8). */
export type SettingsSection = 'general' | 'ssh' | 'rdp' | 'test';

export interface TabPlace {
  readonly tab: TabName;
  /** Only on the Settings tab. */
  readonly section: SettingsSection | null;
  /** Only on the Overview tab; every other place reads as the 1-hour default. */
  readonly range: MetricRange;
}

const SECTIONS: readonly SettingsSection[] = ['general', 'ssh', 'rdp', 'test'];

/** Whether a tab id from the strip is one of the device tabs. */
export function isTab(text: string | null): text is TabName {
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
  return sharedIds('device', name);
}

/** The tabs as the strip draws them. */
export const TAB_ITEMS: readonly TabSpec[] = TABS.map(({ name, label, icon }) => ({ id: name, label, icon }));
