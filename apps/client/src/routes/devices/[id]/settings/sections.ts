import type { IndexEntry } from '$lib/page/contents/PageIndex.svelte';

/** The groups of the device's Settings tab, in order, as their heads and the "On this page" index name them (D17). */
export const SECTIONS = {
  device: { id: 'device-settings-device', title: 'Device', icon: 'carbon:data-base' },
  monitoring: { id: 'device-settings-monitoring', title: 'Monitoring', icon: 'carbon:activity' },
  connections: { id: 'device-settings-connections', title: 'Connections', icon: 'carbon:login' },
  sessions: { id: 'device-settings-sessions', title: 'Sessions and files', icon: 'carbon:recently-viewed' },
  web: { id: 'device-settings-web', title: 'Web interfaces', icon: 'carbon:earth' },
} as const satisfies Record<string, IndexEntry>;

export const INDEX: readonly IndexEntry[] = Object.values(SECTIONS);
