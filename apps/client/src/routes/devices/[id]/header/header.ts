import type { DeviceCheck } from '$lib/checks/api';
import type { CardDevice, DeviceDetail } from '$lib/devices/api';
import { reasonOf } from '$lib/devices/list/look';
import { offersScreenSharing } from '$lib/devices/screen-sharing/setup';
import { osIcon } from '$lib/devices/os/os';
import type { IconName } from '$lib/icons/icons';
import type { SessionProtocol } from '$lib/sessions/tabs';
import type { SettingsSection } from '../tabs/tabs';

/** What the detail bar reads from a device (DEV-020, D16). */
export type HeaderDevice = Pick<
  DeviceDetail,
  'id' | 'name' | 'status' | 'host' | 'ip' | 'os' | 'distribution' | 'accessWarnings' | 'webInterfaces'
> & {
  readonly ssh: Pick<DeviceDetail['ssh'], 'enabled' | 'hostKey'>;
  readonly rdp: Pick<DeviceDetail['rdp'], 'enabled'>;
  readonly vnc: Pick<DeviceDetail['vnc'], 'enabled' | 'username'>;
};

/**
 * What the SSH action does (D-8, DEV-022, WP-14 H-1): start a session; open the connection test, because the hub
 * refuses a session without an accepted host key; or open Settings at SSH, because SSH is off.
 */
export type SshAction = 'connect' | 'test' | 'set-up';

/** What a protocol action does: start a session, open the connection test, or open Settings at its section (DEV-022). */
export type ProtocolMode = 'connect' | 'test' | 'set-up';

/** The accessible name of a protocol action, and the Settings section it opens when it does not connect (DEV-022). */
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
  /** The IP address in the bar's meta, after the host; `null` when it is empty or the same as the host (D-8). */
  readonly ip: string | null;
  /** The operating system's logo, the bar's mark before the name. */
  readonly osIcon: IconName;
  readonly ssh: SshAction;
  /** RDP connects when on and opens Settings at RDP when off (DEV-022). */
  readonly rdp: 'connect' | 'set-up';
  /** VNC is offered only when on (D-4). */
  readonly vnc: boolean;
  /** "Set up screen sharing" stands in for VNC on a macOS or Linux device with VNC off (R-079). */
  readonly screenSharing: boolean;
}

function sshAction(ssh: HeaderDevice['ssh']): SshAction {
  if (!ssh.enabled) return 'set-up';
  return ssh.hostKey == null ? 'test' : 'connect';
}

/** What the detail bar shows of a device besides its name and status (DEV-020, D-8, D16). */
export function headerLine(device: HeaderDevice): HeaderLine {
  const ip = device.ip ?? '';
  return {
    ip: ip !== '' && ip !== device.host ? ip : null,
    osIcon: osIcon(device.os, device.distribution),
    ssh: sshAction(device.ssh),
    rdp: device.rdp.enabled ? 'connect' : 'set-up',
    vnc: device.vnc.enabled,
    screenSharing: offersScreenSharing(device),
  };
}

/** One protocol action of the connect split button. */
export interface ProtocolAction {
  readonly protocol: SessionProtocol;
  readonly mode: ProtocolMode;
  /** The accessible name: "SSH to nas", "Test the SSH connection", "Set up RDP". */
  readonly label: string;
  /** The Settings section it opens; `null` when it starts a session. */
  readonly section: SettingsSection | null;
}

/**
 * The connect split button (D16, DEV-022): the first protocol that connects, in the order SSH, RDP, VNC, is the main
 * action, or SSH's test or set-up when none does. The menu holds the rest, the ones that connect first.
 */
export function connectActions(line: HeaderLine, deviceName: string): { readonly main: ProtocolAction; readonly others: readonly ProtocolAction[] } {
  const action = (protocol: SessionProtocol, mode: ProtocolMode): ProtocolAction => ({ protocol, mode, ...protocolButton(protocol, mode, deviceName) });
  const ssh = action('SSH', line.ssh);
  const actions = [ssh, action('RDP', line.rdp), ...(line.vnc ? [action('VNC', 'connect')] : [])];
  const main = actions.find((each) => each.mode === 'connect') ?? ssh;
  const rest = actions.filter((each) => each !== main);
  return { main, others: [...rest.filter((each) => each.mode === 'connect'), ...rest.filter((each) => each.mode !== 'connect')] };
}

/** The words of a protocol action on its button or menu row: "SSH", "Test SSH", "Set up RDP". */
export function actionWords(action: Pick<ProtocolAction, 'protocol' | 'mode'>): string {
  if (action.mode === 'connect') return action.protocol;
  return action.mode === 'test' ? `Test ${action.protocol}` : `Set up ${action.protocol}`;
}

/** A detail-page check in the list's shape, its last heartbeat as the one beat the reason reads. */
function lookCheck(check: DeviceCheck): CardDevice['checks'][number] {
  const beat = check.lastHeartbeat;
  return { id: check.id, name: check.name, status: check.status, paused: check.paused, heartbeats: { edges: beat == null ? [] : [{ node: beat }] } };
}

/**
 * Why the device is not up, as one sentence under the bar (D16): the list's reason (D18) from the device's checks.
 * `null` when it is up, and while the checks the reason needs have not arrived.
 */
export function statusReason(device: Pick<HeaderDevice, 'id' | 'status'>, checks: readonly DeviceCheck[] | null): string | null {
  if (device.status === 'UP' || (checks === null && device.status !== 'MAINTENANCE')) return null;
  const reason = reasonOf({ id: device.id, status: device.status, checks: (checks ?? []).map(lookCheck) }, undefined, 0);
  if (reason === null) return null;
  const text = reason.text.charAt(0).toUpperCase() + reason.text.slice(1);
  return /[.!?]$/.test(text) ? text : `${text}.`;
}
