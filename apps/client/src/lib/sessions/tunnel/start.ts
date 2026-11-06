import type { HubError } from '$lib/hub/hub';
import type { Protocol, RemoteProtocol, SessionDisplay, SessionTicket, SessionsApi } from '../api';
import type { TunnelRefusal } from './state';
import { openTerminalLink, type LinkOptions, type TerminalLink } from './terminal/link';

/** The device and display a session is for; a reconnect passes the same device with the current display. */
export interface SessionTarget {
  readonly deviceId: string;
  readonly display: SessionDisplay;
}

/** A target with the protocol `startSession` is asked for. */
export interface ProtocolTarget extends SessionTarget {
  readonly protocol: Protocol;
}

/** A target for a Guacamole session (RDP or VNC, K4 §1). */
export interface RemoteTarget extends SessionTarget {
  readonly protocol: RemoteProtocol;
}

/** A started session: its ID and its open tunnel. The ticket is not part of it (SEC-002). */
export interface Started<Tunnel> {
  readonly sessionId: string;
  readonly tunnel: Tunnel;
}

export type StartResult<Tunnel> =
  | { readonly ok: true; readonly value: Started<Tunnel> }
  | { readonly ok: false; readonly error: HubError | TunnelRefusal };

/** A started terminal session (SSH, K4 §8). */
export type TunnelStart = StartResult<TerminalLink>;

/** The terminal link's options other than the ticket. */
export type TerminalSetup = Omit<LinkOptions, 'ticket'>;

export type Opening<Tunnel> =
  | { readonly ok: true; readonly value: Tunnel }
  | { readonly ok: false; readonly error: TunnelRefusal };

/**
 * Start a session on `target` and open its tunnel with the new ticket through `open` (D-3). "Reconnect" is this
 * call again for the same device (D-5): every call asks `startSession` for a new single-use ticket, so a ticket is
 * never reused. The ticket goes straight to `open` and is dropped; the caller never holds the token.
 */
export async function startWith<Tunnel>(
  api: Pick<SessionsApi, 'startSession' | 'endSession'>,
  target: ProtocolTarget,
  open: (ticket: SessionTicket) => Opening<Tunnel>,
): Promise<StartResult<Tunnel>> {
  const started = await api.startSession(target.deviceId, target.protocol, target.display);
  if (!started.ok) return started;
  const { sessionId } = started.value;
  const opening = open(started.value);
  if (opening.ok) return { ok: true, value: { sessionId, tunnel: opening.value } };
  // The session row exists but will never get a tunnel, so end it. The refusal is what the operator needs to
  // see; a failed `endSession` here is not reported over it.
  await api.endSession(sessionId);
  return opening;
}

/** Start an SSH session and open its terminal tunnel (K4 §8). */
export function startTerminal(
  api: Pick<SessionsApi, 'startSession' | 'endSession'>,
  target: SessionTarget,
  setup: TerminalSetup,
): Promise<TunnelStart> {
  return startWith(api, { ...target, protocol: 'SSH' }, (ticket) => openTerminalLink({ ...setup, ticket }));
}
