import type { DeviceLinkStatus } from '$lib/desktop/link/shell';
import type { HubError } from '$lib/hub/hub';
import type { LinkDevice } from './api';

/** How a device reaches the hub, in words (LNK-006). */
export function pathText(device: Pick<LinkDevice, 'path' | 'relay'>): string {
  switch (device.path) {
    case 'DIRECT':
      return 'Direct';
    case 'RELAYED':
      return device.relay === null || device.relay === '' ? 'Relayed' : `Relayed through ${device.relay}`;
    case 'NONE':
      return 'Not yet connected';
  }
}

/** A device key's expiry for the operator; "No expiry" when the link gives none. */
export function expiryText(expiresAt: string | null, locale?: string): string {
  if (expiresAt === null) return 'No expiry';
  const at = new Date(expiresAt);
  return Number.isNaN(at.getTime()) ? '—' : at.toLocaleString(locale, { dateStyle: 'medium', timeStyle: 'short' });
}

/** Whole seconds until `expiresAt`, never below 0; 0 for a time the client cannot read (LNK-003). */
export function secondsLeft(expiresAt: string, now: Date): number {
  const end = Date.parse(expiresAt);
  if (Number.isNaN(end)) return 0;
  return Math.max(0, Math.ceil((end - now.getTime()) / 1000));
}

/** A countdown as minutes and seconds, for example "14:59". */
export function countdownText(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds));
  return `${String(Math.floor(whole / 60))}:${String(whole % 60).padStart(2, '0')}`;
}

/** The device list's state: hidden on a hub that is not in the link mode (VPS mode, UNAVAILABLE) (FRN-009). */
export type DevicesShown = 'loading' | 'hidden' | 'failed' | 'listed';

export function devicesShown(result: { readonly ok: true } | { readonly ok: false; readonly error: Pick<HubError, 'code'> } | null): DevicesShown {
  if (result === null) return 'loading';
  if (result.ok) return 'listed';
  return result.error.code === 'UNAVAILABLE' ? 'hidden' : 'failed';
}

/** The fixed text for a refused invite (LOG-007: the hub's own text is not shown for a secret-bearing call). */
export function inviteFailure(error: Pick<HubError, 'code'>): string {
  switch (error.code) {
    case 'VALIDATION':
      return 'Use 1 to 64 visible characters for the name.';
    case 'FORBIDDEN':
    case 'UNAUTHENTICATED':
      return 'Only a signed-in operator can invite a device. Sign in again.';
    case 'unreachable':
      return 'The hub did not answer. Try again.';
    default:
      return 'The hub could not make an invite. Try again.';
  }
}

/** The "Link to the hub" row's sentence on a joined device (LNK-006, F-2). */
export function deviceLinkText(status: DeviceLinkStatus): string {
  switch (status.state) {
    case 'up': {
      const path = status.path === 'direct' ? 'direct' : status.path === 'relayed' ? 'relayed' : 'not yet connected';
      const latency = status.latencyMs === undefined ? '' : `, ${String(Math.round(status.latencyMs))} ms`;
      return `Up: ${path}${latency}.`;
    }
    case 'starting':
      return 'Starting.';
    case 'stopped':
      return 'Stopped.';
    case 'down':
      return status.reason === 'port-taken'
        ? `Down: another program holds port ${String(status.port ?? '')}.`
        : status.reason === 'not-carried'
          ? 'Down: this build does not carry the link program.'
          : 'Down: the link ended. It starts again in 10 seconds.';
  }
}
