import type { OperatingSystem } from '$lib/hub/generated/graphql';
import type { IconName } from '$lib/icons/icons';
import { distribution as named } from './distribution';

/** Each K1 operating system's logo and its name in words, so the logo never stands alone (UI-003, UI-015). */
const SYSTEMS: Record<OperatingSystem, { readonly icon: IconName; readonly label: string }> = {
  LINUX: { icon: 'nf-md-linux', label: 'Linux' },
  MACOS: { icon: 'nf-md-apple', label: 'macOS' },
  WINDOWS: { icon: 'nf-md-microsoft_windows', label: 'Windows' },
  OTHER: { icon: 'nf-md-monitor', label: 'Other OS' },
};

/** A Linux device's distribution when the hub has read one that has a logo (DEV-016). */
function distro(os: OperatingSystem, distribution: string | null | undefined) {
  return os === 'LINUX' ? named(distribution) : null;
}

/**
 * The logo of a device's operating system: its distribution's logo when the hub read one (DEV-016), else the system's;
 * any other system shows a plain monitor.
 */
export function osIcon(os: OperatingSystem, distribution?: string | null): IconName {
  return distro(os, distribution)?.icon ?? SYSTEMS[os].icon;
}

/** The operating system's name as the operator reads it, by its distribution when the hub read one. */
export function osLabel(os: OperatingSystem, distribution?: string | null): string {
  return distro(os, distribution)?.label ?? SYSTEMS[os].label;
}
