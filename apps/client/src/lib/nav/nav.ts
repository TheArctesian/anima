import { dataAge, type HubStatus } from '$lib/hub/hub';
import type { IconName } from '$lib/icons/icons';

/** One destination of the navigation bar. */
export interface NavDestination {
  readonly href: '/devices' | '/builds' | '/settings';
  readonly label: string;
  readonly icon: IconName;
}

export const DEVICES: NavDestination = { href: '/devices', label: 'Devices', icon: 'nf-md-server' };
export const BUILDS: NavDestination = { href: '/builds', label: 'Builds', icon: 'nf-md-wrench' };
export const SETTINGS: NavDestination = { href: '/settings', label: 'Settings', icon: 'nf-md-cog' };

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
  const age = status.lastContactAt === null ? 'No data received yet' : `Data from ${dataAge(status.lastContactAt, now)}`;
  return { title: 'Not on the network', origin, advice: 'Check the VPN client', age };
}

/** How often the data age in the banner is recomputed; `dataAge` changes at most once a minute. */
export const DATA_AGE_REFRESH_MS = 30_000;
