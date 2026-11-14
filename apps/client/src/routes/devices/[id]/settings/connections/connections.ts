import type { DeviceDetail } from '$lib/devices/api';
import type { DeviceFormValues } from '$lib/devices/form/values';

export type Protocol = 'ssh' | 'rdp' | 'vnc';

/**
 * D17: whether a protocol has been set up, so its row shows a switch rather than "Set up": it is on, or it has a user
 * name or a stored password to come back to.
 */
export function isSetUp(device: DeviceDetail, protocol: Protocol): boolean {
  if (protocol === 'ssh') return device.ssh.enabled || device.ssh.username !== '' || device.ssh.credentialStored;
  const settings = device[protocol];
  return settings.enabled || (settings.username ?? '') !== '' || settings.credentialStored;
}

/** The parts of a protocol row's summary, as shown: "Port 22", the user name, and for SSH how it signs in (D17). */
export function summaryParts(values: DeviceFormValues, protocol: Protocol): string[] {
  const settings = values[protocol];
  const parts = [`Port ${settings.port.trim()}`, settings.username.trim()];
  if (protocol === 'ssh') parts.push(values.ssh.auth === 'PASSWORD' ? 'password' : 'private key');
  return parts.filter((part) => part !== '');
}
