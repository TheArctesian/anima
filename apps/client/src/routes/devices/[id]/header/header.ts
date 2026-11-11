import type { DeviceDetail } from '$lib/devices/api';
import { offersScreenSharing } from '$lib/devices/screen-sharing/setup';
import { osIcon, osLabel } from '$lib/devices/os/os';
import type { OperatingSystem } from '$lib/hub/generated/graphql';
import type { IconName } from '$lib/icons/icons';
import type { SessionProtocol } from '$lib/sessions/tabs';
import type { SettingsSection } from '../tabs/tabs';

/** What the header line reads from a device (DEV-020). */
export type HeaderDevice = Pick<
  DeviceDetail,
  'id' | 'name' | 'status' | 'host' | 'ip' | 'kind' | 'os' | 'distribution' | 'tags' | 'accessWarnings' | 'webInterfaces'
> & {
  readonly ssh: Pick<DeviceDetail['ssh'], 'enabled' | 'hostKey'>;
  readonly rdp: Pick<DeviceDetail['rdp'], 'enabled'>;
  readonly vnc: Pick<DeviceDetail['vnc'], 'enabled' | 'username'>;
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
  /** The details row: each item is a chip with its icon, its full text and the same text as its tooltip (DEV-021). */
  readonly items: readonly HeaderText[];
  /** The kind's icon, on the kind chip (UI-003: the kind is also said in words there). */
  readonly kindIcon: IconName;
  /** The operating system's logo, before the name and on the OS chip (UI-003: the OS is said in words on the chip). */
  readonly osIcon: IconName;
  readonly ssh: SshAction;
  /** RDP connects when on and opens Settings at RDP when off (DEV-022). */
  readonly rdp: 'connect' | 'set-up';
  /** VNC shows only when on (D-4). */
  readonly vnc: boolean;
  /** "Set up screen sharing" takes the VNC slot for a macOS or Linux device with VNC off (R-079). */
  readonly screenSharing: boolean;
}

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
    { part: 'os', text: osLabel(device.os, device.distribution) },
    ...device.tags.map((tag) => ({ part: 'tag' as const, text: tag })),
  ];
  return {
    items,
    kindIcon: kindIcon(device.kind, device.os),
    osIcon: osIcon(device.os, device.distribution),
    ssh: sshAction(device.ssh),
    rdp: device.rdp.enabled ? 'connect' : 'set-up',
    vnc: device.vnc.enabled,
    screenSharing: offersScreenSharing(device),
  };
}

/** Words in a kind, first match wins: the operator types the kind freely, so it is matched by the words it holds. */
const KIND_ICONS: readonly (readonly [RegExp, IconName])[] = [
  [/laptop|notebook|macbook/, 'nf-md-laptop'],
  [/phone|mobile|tablet|ipad/, 'nf-md-cellphone'],
  [/\bnas\b|storage|file ?server/, 'nf-md-nas'],
  [/router|switch|access ?point|firewall|gateway|\bap\b/, 'nf-md-router_wireless'],
  [/monitor|display|screen|\btv\b|kiosk/, 'nf-md-monitor'],
  [/workstation|desktop|\bpc\b|tower/, 'nf-md-desktop_tower'],
  [/server|\bvm\b|virtual/, 'nf-md-server'],
];

/** The icon for a device's kind; a kind with no known word is a desktop for macOS and Windows and a server otherwise. */
export function kindIcon(kind: string, os: OperatingSystem): IconName {
  const words = kind.toLowerCase();
  const found = KIND_ICONS.find(([pattern]) => pattern.test(words));
  if (found !== undefined) return found[1];
  return os === 'MACOS' || os === 'WINDOWS' ? 'nf-md-desktop_tower' : 'nf-md-server';
}

/** An IPv4 or IPv6 address, as opposed to a host name. */
const ADDRESS = /^(\d{1,3}(\.\d{1,3}){3}|[\da-f:]*:[\da-f:.]*)$/i;

/** The icon of one details chip: an address, a host name, the kind, the operating system's logo or a tag. */
export function itemIcon(item: HeaderText, kind: IconName, os: IconName): IconName {
  switch (item.part) {
    case 'host':
      return ADDRESS.test(item.text) ? 'nf-md-ip_network' : 'nf-md-dns';
    case 'ip':
      return 'nf-md-ip_network';
    case 'kind':
      return kind;
    case 'os':
      return os;
    case 'tag':
      return 'nf-md-tag';
  }
}
