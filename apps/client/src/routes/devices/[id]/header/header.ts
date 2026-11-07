import type { DeviceDetail } from '$lib/devices/api';
import type { OperatingSystem } from '$lib/hub/generated/graphql';
import type { SessionProtocol } from '$lib/sessions/tabs';
import type { SettingsSection } from '../tabs/tabs';

/** What the header line reads from a device (DEV-020). */
export type HeaderDevice = Pick<
  DeviceDetail,
  'id' | 'name' | 'status' | 'host' | 'ip' | 'kind' | 'os' | 'tags' | 'accessWarnings' | 'webInterfaces'
> & {
  readonly ssh: Pick<DeviceDetail['ssh'], 'enabled' | 'hostKey'>;
  readonly rdp: Pick<DeviceDetail['rdp'], 'enabled'>;
  readonly vnc: Pick<DeviceDetail['vnc'], 'enabled'>;
};

/** One text item of the host-to-tags group, in line order. */
export interface HeaderText {
  readonly part: 'host' | 'ip' | 'kind' | 'os' | 'tag';
  readonly text: string;
}

/**
 * What the SSH button does (D-8, DEV-022, WP-14 H-1): start a session; open the connection test, because the hub
 * refuses a session without an accepted host key; or open Settings at SSH, because SSH is off.
 */
export type SshAction = 'connect' | 'test' | 'set-up';

/** What a protocol button does: start a session, open the connection test, or open Settings at its section (DEV-022). */
export type ProtocolMode = 'connect' | 'test' | 'set-up';

/** The accessible name of a protocol button, and the Settings section it opens when it does not connect (DEV-022). */
export function protocolButton(
  protocol: SessionProtocol,
  mode: ProtocolMode,
  deviceName: string,
): { readonly label: string; readonly section: SettingsSection | null } {
  if (mode === 'connect') return { label: `${protocol} to ${deviceName}`, section: null };
  if (mode === 'test') return { label: `Test the ${protocol} connection`, section: 'test' };
  return { label: `Set up ${protocol}`, section: protocol === 'RDP' ? 'rdp' : 'ssh' };
}

export interface HeaderLine {
  readonly items: readonly HeaderText[];
  /** The whole group as one text, for its tooltip (DEV-021). */
  readonly title: string;
  readonly ssh: SshAction;
  /** RDP connects when on and opens Settings at RDP when off (DEV-022). */
  readonly rdp: 'connect' | 'set-up';
  /** VNC shows only when on (D-4). */
  readonly vnc: boolean;
}

const OS_LABELS: Record<OperatingSystem, string> = {
  LINUX: 'Linux',
  MACOS: 'macOS',
  WINDOWS: 'Windows',
  OTHER: 'Other OS',
};

function sshAction(ssh: HeaderDevice['ssh']): SshAction {
  if (!ssh.enabled) return 'set-up';
  return ssh.hostKey == null ? 'test' : 'connect';
}

/** The header line after the name and status (DEV-020, D-8): host, IP when it differs, kind, OS, then each tag. */
export function headerLine(device: HeaderDevice): HeaderLine {
  const ip = device.ip ?? '';
  const items: HeaderText[] = [
    { part: 'host', text: device.host },
    ...(ip !== '' && ip !== device.host ? [{ part: 'ip' as const, text: ip }] : []),
    { part: 'kind', text: device.kind },
    { part: 'os', text: OS_LABELS[device.os] },
    ...device.tags.map((tag) => ({ part: 'tag' as const, text: tag })),
  ];
  return {
    items,
    title: items.map((item) => item.text).join(' · '),
    ssh: sshAction(device.ssh),
    rdp: device.rdp.enabled ? 'connect' : 'set-up',
    vnc: device.vnc.enabled,
  };
}
