import type { AccessWarning, CardDevice } from '$lib/devices/api';
import { offersSsh } from '$lib/devices/card/card';
import type { SessionProtocol } from '$lib/sessions/tabs';
import type { WebInterface } from '$lib/web-interfaces/api';

/** The protocol buttons a card or table row shows, in order (RA-030, D-4): SSH with an accepted host key, RDP and VNC when on. */
export function connectProtocols(device: Pick<CardDevice, 'ssh' | 'rdp' | 'vnc'>): SessionProtocol[] {
  return [
    ...(offersSsh(device) ? (['SSH'] as const) : []),
    ...(device.rdp.enabled ? (['RDP'] as const) : []),
    ...(device.vnc.enabled ? (['VNC'] as const) : []),
  ];
}

/** The device's web interfaces; none until the hub serves them (WEB-003). */
export function webInterfacesOf(device: { readonly webInterfaces?: readonly WebInterface[] }): readonly WebInterface[] {
  return device.webInterfaces ?? [];
}

/** RA-031: a device can be woken from the list only when the operator gave it a MAC address. */
export function canWake(device: Pick<CardDevice, 'macAddress'>): boolean {
  return device.macAddress != null && device.macAddress !== '';
}

/** The accessible name of a protocol button (RA-030). */
export function connectLabel(protocol: SessionProtocol, deviceName: string): string {
  return `${protocol} to ${deviceName}`;
}

const WARNING_TEXT: Record<AccessWarning, string> = {
  IGNORED_CERTIFICATE: 'The certificate is ignored, so Anima cannot tell this device from an impostor.',
  RDP_SECURITY: 'Standard RDP security is weak: the sign-in is not protected the way the other modes protect it.',
  UNENCRYPTED_VNC: 'The VNC connection is not encrypted. Use it only over the VPN.',
};

/** RA-020: each warning in words, in the order the hub gave them. */
export function warningLines(warnings: readonly AccessWarning[]): string[] {
  return warnings.map((warning) => WARNING_TEXT[warning]);
}

export interface Point {
  readonly left: number;
  readonly top: number;
}

interface Box {
  readonly left: number;
  readonly top: number;
  readonly bottom: number;
}

/** The least distance from the window edge, and the gap between the button and the panel. */
export interface Spacing {
  readonly margin: number;
  readonly gap: number;
}

interface Size {
  readonly width: number;
  readonly height: number;
}

/**
 * Where a fixed panel goes so it lies under its button, or above it when the window ends first, and stays `margin`
 * inside the window on both axes (WEB-003). Fixed, because a card clips what lies outside it.
 */
export function panelPosition(anchor: Box, panel: Size, viewport: Size, spacing: Spacing): Point {
  const { margin, gap } = spacing;
  const below = anchor.bottom + gap;
  const fitsBelow = below + panel.height + margin <= viewport.height;
  const top = fitsBelow ? below : Math.max(margin, anchor.top - gap - panel.height);
  const toRight = Math.min(0, viewport.width - margin - (anchor.left + panel.width));
  const left = anchor.left + Math.max(toRight, margin - anchor.left);
  return { left, top };
}
