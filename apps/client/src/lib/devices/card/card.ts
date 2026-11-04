import type { CardDevice } from '$lib/devices/api';

export type DeviceStatus = CardDevice['status'];

const LABELS: Record<DeviceStatus, string> = {
  UP: 'Up',
  DOWN: 'Down',
  DEGRADED: 'Degraded',
  MAINTENANCE: 'Maintenance',
  UNKNOWN: 'Unknown',
};

/** The status as text, so it never relies on colour alone. */
export function statusLabel(status: DeviceStatus): string {
  return LABELS[status];
}

/**
 * Whether the card offers SSH: SSH is on and the operator has accepted a host key. Without a key the hub refuses to
 * start a session (CONFLICT), so the device page's connection test comes first (WP-14 H-1).
 */
export function offersSsh(device: Pick<CardDevice, 'ssh'>): boolean {
  return device.ssh.enabled && device.ssh.hostKey != null;
}

const IPV4 = /^\d{1,3}(?:\.\d{1,3}){3}$/;

/** A host that is an IP address (v4, or v6 with its colons) rather than a name. */
function isAddress(host: string): boolean {
  return IPV4.test(host) || host.includes(':');
}

/** The host to show, and the IP address next to it when the host is a name and the IP is known (D-7). */
export function hostLine(device: Pick<CardDevice, 'host' | 'ip'>): { host: string; ip: string | null } {
  const showIp = device.ip !== null && device.ip !== '' && !isAddress(device.host);
  return { host: device.host, ip: showIp ? device.ip : null };
}
