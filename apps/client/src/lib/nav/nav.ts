import { dataAge, type HubStatus } from '$lib/hub/hub';
import type { IconName } from '$lib/icons/icons';

/** One destination of the navigation bar. */
export interface NavDestination {
  readonly href: '/devices' | '/agents/usage' | '/projects' | '/builds' | '/logs' | '/settings';
  readonly label: string;
  readonly icon: IconName;
}

export const DEVICES: NavDestination = { href: '/devices', label: 'Devices', icon: 'carbon:bare-metal-server' };
/** The use of the AI agent programs (AIS-020); always there, because the page explains itself when no daemon reports. */
export const AGENT_USAGE: NavDestination = { href: '/agents/usage', label: 'Agent usage', icon: 'carbon:chart-line' };
/** Shown only while the Projects extension is on (EXT-002). */
export const PROJECTS: NavDestination = { href: '/projects', label: 'Projects', icon: 'carbon:folder' };
export const BUILDS: NavDestination = { href: '/builds', label: 'Builds', icon: 'carbon:tool-kit' };
/** A page in the phone layout only; a computer has the logs panel on each page (MOB-013). */
export const LOGS: NavDestination = { href: '/logs', label: 'Logs', icon: 'carbon:catalog' };
export const SETTINGS: NavDestination = { href: '/settings', label: 'Settings', icon: 'carbon:settings' };

/**
 * The destinations between Devices and Settings that are not always there, in the bar's order: Projects only while its
 * extension is on (EXT-002), Builds only in the desktop app, because a browser has no builder (BLD-080).
 */
export function optionalDestinations(projectsOn: boolean, builds: boolean): readonly NavDestination[] {
  return [...(projectsOn ? [PROJECTS] : []), ...(builds ? [BUILDS] : [])];
}

/** Whether `pathname` is `item`'s page or one below it (NAV-002). */
export function isActive(item: NavDestination, pathname: string): boolean {
  return pathname === item.href || pathname.startsWith(`${item.href}/`);
}

/** The hub status label above Settings: anything but `connected` reads "Disconnected" (NAV-003). */
export function statusLabel(status: HubStatus): string {
  return status.connection === 'connected' ? 'Connected' : 'Disconnected';
}

/** What the operator reads when the hub cannot be reached (NET-004). */
export interface UnreachableBanner {
  readonly title: string;
  readonly origin: string;
  readonly advice: string;
  readonly age: string;
}

/** The banner for `status`, or `null` when the hub is not unreachable. `now` dates the data age. */
export function unreachableBanner(status: HubStatus, origin: string, now: Date): UnreachableBanner | null {
  if (status.connection !== 'unreachable') return null;
  // A copy on the screen may be older than the last contact, and is all there is after a start with no hub (FND-007).
  const since = status.copyFrom ?? status.lastContactAt;
  const age = since === null ? 'No data yet' : `Data from ${dataAge(since, now)}`;
  return { title: 'Not on the network', origin, advice: 'Check the VPN client', age };
}

/** How often the data age in the banner is recomputed; `dataAge` changes at most once a minute. */
export const DATA_AGE_REFRESH_MS = 30_000;
