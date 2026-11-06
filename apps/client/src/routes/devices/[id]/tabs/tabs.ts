/** The device detail tabs, in order (Section 6.7). */
export const TABS = [
  { name: 'overview', label: 'Overview' },
  { name: 'checks', label: 'Checks' },
  { name: 'sessions', label: 'Sessions' },
  { name: 'logs', label: 'Logs' },
  { name: 'settings', label: 'Settings' },
] as const;

export type TabName = (typeof TABS)[number]['name'];

/** The Settings section a header button opens (DEV-022): the device form's disclosure sections. */
export type SettingsSection = 'general' | 'ssh' | 'rdp';

export interface TabPlace {
  readonly tab: TabName;
  /** Only on the Settings tab. */
  readonly section: SettingsSection | null;
}

const SECTIONS: readonly SettingsSection[] = ['general', 'ssh', 'rdp'];

function isTab(text: string | null): text is TabName {
  return TABS.some((tab) => tab.name === text);
}

function isSection(text: string | null): text is SettingsSection {
  return SECTIONS.some((section) => section === text);
}

/** D-9: the tab and section from the URL; an unknown tab is Overview and an unknown section is none. */
export function readPlace(params: URLSearchParams): TabPlace {
  const tab = params.get('tab');
  const section = params.get('section');
  if (!isTab(tab)) return { tab: 'overview', section: null };
  return { tab, section: tab === 'settings' && isSection(section) ? section : null };
}

/** D-9: the query string for a place, empty for Overview, so Back returns to it and a reload keeps it. */
export function placeSearch(place: TabPlace): string {
  if (place.tab === 'overview') return '';
  const params = new URLSearchParams({ tab: place.tab });
  if (place.tab === 'settings' && place.section !== null) params.set('section', place.section);
  return `?${params.toString()}`;
}

/** The id of a tab's button and of its panel, for `aria-controls` and `aria-labelledby`. */
export function tabIds(name: TabName): { tab: string; panel: string } {
  return { tab: `device-tab-${name}`, panel: `device-panel-${name}` };
}
