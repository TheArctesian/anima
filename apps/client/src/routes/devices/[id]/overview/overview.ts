import { uptimeText } from '$lib/checks/heartbeats';
import type { DeviceDetail } from '$lib/devices/api';
import { statusLabel, type DeviceStatus } from '$lib/devices/card/card';

/** What the Overview tab reads from a device (D-10). */
export type OverviewDevice = Pick<DeviceDetail, 'maintenance' | 'day' | 'week' | 'month'> & {
  readonly ssh: Pick<DeviceDetail['ssh'], 'enabled' | 'hostKey'>;
};

export interface Reading {
  readonly label: string;
  readonly text: string;
}

export interface StatusCount {
  readonly status: DeviceStatus;
  readonly label: string;
  readonly count: number;
}

/** Worst first, the order of the device list's status sort (R-044). */
const ORDER: readonly DeviceStatus[] = ['DOWN', 'DEGRADED', 'UNKNOWN', 'MAINTENANCE', 'UP'];

/** D-10: uptime over 24 hours, 7 days and 30 days; no value is "—". */
export function uptimeReadings(device: OverviewDevice): readonly Reading[] {
  return [
    { label: '24 hours', text: uptimeText(device.day ?? null) },
    { label: '7 days', text: uptimeText(device.week ?? null) },
    { label: '30 days', text: uptimeText(device.month ?? null) },
  ];
}

/** D-10: how many checks have each status, worst first; statuses no check has are left out. */
export function checkCounts(checks: readonly { readonly status: DeviceStatus }[]): readonly StatusCount[] {
  return ORDER.map((status) => ({ status, label: statusLabel(status), count: checks.filter((check) => check.status === status).length })).filter(
    (entry) => entry.count > 0,
  );
}

/** D-10: the accepted SSH host key, or why there is none. */
export function hostKeyText(ssh: OverviewDevice['ssh']): string {
  if (ssh.hostKey != null) return `${ssh.hostKey.algorithm} ${ssh.hostKey.fingerprintSha256}`;
  return ssh.enabled ? 'No host key accepted yet' : 'SSH is off';
}

/**
 * What the Metrics card says of a device with no metrics. A hub that Anima.ing hosts reads none over SSH (HMD-049 is
 * not built), so its sentence names the daemon only (R-222).
 */
export function metricsOffText(hosted: boolean): string {
  return hosted ? 'Metrics are off. They appear when the daemon is installed on this device.' : 'Metrics are off. Install the daemon or turn on SSH metrics in Settings.';
}
