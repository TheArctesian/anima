import type { HubError } from '$lib/hub/hub';
import type { SessionDisplay, SessionTicket, SessionsApi } from '../api';
import { openTunnel, type SessionTunnel, type TunnelOptions } from './guacamole/tunnel';
import type { TunnelRefusal } from './state';
import { openTerminalLink, type LinkOptions, type TerminalLink } from './terminal/link';

/** The device and display a session is for; a reconnect passes the same device with the current display. */
export interface SessionTarget {
  readonly deviceId: string;
  readonly display: SessionDisplay;
}

/** A started session: its ID and its open tunnel. The ticket is not part of it (SEC-002). */
export interface Started<Tunnel> {
  readonly sessionId: string;
  readonly tunnel: Tunnel;
}

export type StartResult<Tunnel> =
  | { readonly ok: true; readonly value: Started<Tunnel> }
  | { readonly ok: false; readonly error: HubError | TunnelRefusal };

/** A started Guacamole session. */
export type StartedTunnel = Started<SessionTunnel>;

/** A started terminal session (SSH, K4 §8). */
export type TunnelStart = StartResult<TerminalLink>;

/** The Guacamole tunnel options other than the ticket (hub origin, hooks), which `startTunnel` gets from the hub. */
export type TunnelSetup = Omit<TunnelOptions, 'ticket'>;

/** The terminal link's options other than the ticket. */
export type TerminalSetup = Omit<LinkOptions, 'ticket'>;

type Opening<Tunnel> =
  | { readonly ok: true; readonly value: Tunnel }
  | { readonly ok: false; readonly error: TunnelRefusal };

/**
 * Start a session on `target` and open its tunnel with the new ticket through `open` (D-3). "Reconnect" is this
 * call again for the same device (D-5): every call asks `startSession` for a new single-use ticket, so a ticket is
 * never reused. The ticket goes straight to `open` and is dropped; the caller never holds the token.
 */
async function startWith<Tunnel>(
  api: Pick<SessionsApi, 'startSession' | 'endSession'>,
  target: SessionTarget,
  open: (ticket: SessionTicket) => Opening<Tunnel>,
): Promise<StartResult<Tunnel>> {
  const started = await api.startSession(target.deviceId, target.display);
  if (!started.ok) return started;
  const { sessionId } = started.value;
  const opening = open(started.value);
  if (opening.ok) return { ok: true, value: { sessionId, tunnel: opening.value } };
  // The session row exists but will never get a tunnel, so end it. The refusal is what the operator needs to
  // see; a failed `endSession` here is not reported over it.
  await api.endSession(sessionId);
  return opening;
}

/** Start a session and open its Guacamole tunnel (RDP and VNC; no SSH session uses it, R-062). */
export function startTunnel(
  api: Pick<SessionsApi, 'startSession' | 'endSession'>,
  target: SessionTarget,
  setup: TunnelSetup,
): Promise<StartResult<SessionTunnel>> {
  return startWith(api, target, (ticket) => openTunnel({ ...setup, ticket }));
}

/** Start an SSH session and open its terminal tunnel (K4 §8). */
export function startTerminal(
  api: Pick<SessionsApi, 'startSession' | 'endSession'>,
  target: SessionTarget,
  setup: TerminalSetup,
): Promise<TunnelStart> {
  return startWith(api, target, (ticket) => openTerminalLink({ ...setup, ticket }));
}
