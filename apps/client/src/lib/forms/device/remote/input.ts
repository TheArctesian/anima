import type {
  PasswordCredentialInput,
  PendingDeviceInput,
  RdpSettingsInput,
  TransferPolicyInput,
  VncSettingsInput,
} from '../../hub-pending';
import { isValidPort } from '../values';
import {
  DEFAULT_RDP_PORT,
  DEFAULT_VNC_PORT,
  defaultRemoteValues,
  normalizedRemote,
  type RemoteSecrets,
  type RemoteValues,
} from './values';

type Rdp = RemoteValues['rdp'];
type Vnc = RemoteValues['vnc'];
type Transfer = RemoteValues['transfer'];

/** The port to send. With the protocol off, an unusable box falls back to the last valid port of `initial`, else the default. */
function portToSend(enabled: boolean, typed: string, initial: string | undefined, fallback: string): number {
  if (enabled || isValidPort(typed)) return Number(typed);
  return Number(initial !== undefined && isValidPort(initial) ? initial : fallback);
}

/** The RDP settings as the hub input (normalized values); shared with the connection test. */
export function rdpInput(rdp: Rdp, initialPort?: string): RdpSettingsInput {
  return {
    enabled: rdp.enabled,
    port: portToSend(rdp.enabled, rdp.port, initialPort, DEFAULT_RDP_PORT),
    username: rdp.username,
    domain: rdp.domain === '' ? null : rdp.domain,
    security: rdp.security,
    ignoreCertificate: rdp.ignoreCertificate,
    resizeMethod: rdp.resizeMethod,
    audioOut: rdp.audioOut,
    microphone: rdp.microphone,
    sharedDrive: rdp.sharedDrive,
  };
}

/** The VNC settings as the hub input (normalized values); shared with the connection test. */
export function vncInput(vnc: Vnc, initialPort?: string): VncSettingsInput {
  return {
    enabled: vnc.enabled,
    port: portToSend(vnc.enabled, vnc.port, initialPort, DEFAULT_VNC_PORT),
    username: vnc.username === '' ? null : vnc.username,
  };
}

function transferInput(transfer: Transfer): TransferPolicyInput {
  return { upload: transfer.upload, download: transfer.download, copy: transfer.copy, paste: transfer.paste };
}

function sameJson(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** D-2, create: only the K1 additions that differ from the K1 default, so an untouched form is the Phase 1 input. */
export function createAdditions(values: RemoteValues): PendingDeviceInput {
  const now = normalizedRemote(values);
  const everyFlagOn = sameJson(now.transfer, defaultRemoteValues().transfer);
  return {
    ...(now.rdp.enabled ? { rdp: rdpInput(now.rdp) } : {}),
    ...(now.vnc.enabled ? { vnc: vncInput(now.vnc) } : {}),
    ...(now.macAddress === '' ? {} : { macAddress: now.macAddress }),
    ...(now.metricsSource === 'NONE' ? {} : { metricsSource: now.metricsSource }),
    ...(everyFlagOn ? {} : { transfer: transferInput(now.transfer) }),
  };
}

/** D-2, update: only the K1 additions that differ from the saved ones; RDP and VNC go whole. An emptied address sends "". */
export function updateAdditions(initial: RemoteValues, values: RemoteValues): PendingDeviceInput {
  const before = normalizedRemote(initial);
  const now = normalizedRemote(values);
  const rdp = rdpInput(now.rdp, before.rdp.port);
  const vnc = vncInput(now.vnc, before.vnc.port);
  return {
    ...(sameJson(rdp, rdpInput(before.rdp)) ? {} : { rdp }),
    ...(sameJson(vnc, vncInput(before.vnc)) ? {} : { vnc }),
    ...(now.macAddress === before.macAddress ? {} : { macAddress: now.macAddress }),
    ...(now.metricsSource === before.metricsSource ? {} : { metricsSource: now.metricsSource }),
    ...(sameJson(now.transfer, before.transfer) ? {} : { transfer: transferInput(now.transfer) }),
  };
}

interface WithRdp {
  readonly rdp: { readonly enabled: boolean };
  readonly remoteSecrets: RemoteSecrets;
}

interface WithVnc {
  readonly vnc: { readonly enabled: boolean };
  readonly remoteSecrets: RemoteSecrets;
}

/** The RDP password to store, or null when none was typed or RDP is off, so the stored one is kept (DEV-011). */
export function rdpCredentialInput(values: WithRdp): PasswordCredentialInput | null {
  const { rdpPassword } = values.remoteSecrets;
  return values.rdp.enabled && rdpPassword !== '' ? { password: rdpPassword } : null;
}

/** The VNC password to store, or null when none was typed or VNC is off, so the stored one is kept (DEV-011). */
export function vncCredentialInput(values: WithVnc): PasswordCredentialInput | null {
  const { vncPassword } = values.remoteSecrets;
  return values.vnc.enabled && vncPassword !== '' ? { password: vncPassword } : null;
}
