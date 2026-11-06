import type { DeviceDetail } from '$lib/devices/api';
import type { OperatingSystem } from '$lib/hub/generated/graphql';

/** What the header line reads from a device (DEV-020). */
export type HeaderDevice = Pick<DeviceDetail, 'id' | 'name' | 'status' | 'host' | 'ip' | 'kind' | 'os' | 'tags'> & {
  readonly ssh: Pick<DeviceDetail['ssh'], 'enabled' | 'hostKey'>;
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

export interface HeaderLine {
  readonly items: readonly HeaderText[];
  /** The whole group as one text, for its tooltip (DEV-021). */
  readonly title: string;
  readonly ssh: SshAction;
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
  return { items, title: items.map((item) => item.text).join(' · '), ssh: sshAction(device.ssh) };
}
