import type { HubError } from '$lib/hub/hub';
import type { SessionDisplay, SessionsApi } from '../api';
import { openTunnel, type SessionTunnel, type TunnelOptions, type TunnelRefusal } from './tunnel';

/** The device and display a session is for; a reconnect passes the same device with the current display. */
export interface SessionTarget {
  readonly deviceId: string;
  readonly display: SessionDisplay;
}

/** A started session: its ID and its open tunnel. The ticket is not part of it (SEC-002). */
export interface StartedTunnel {
  readonly sessionId: string;
  readonly tunnel: SessionTunnel;
}

export type TunnelStart =
  | { readonly ok: true; readonly value: StartedTunnel }
  | { readonly ok: false; readonly error: HubError | TunnelRefusal };

/** The tunnel options other than the ticket (hub origin, hooks), which `startTunnel` gets from the hub. */
export type TunnelSetup = Omit<TunnelOptions, 'ticket'>;

/**
 * Start a session on `target` and open its tunnel with the new ticket (D-3). "Reconnect" is this call again for
 * the same device (D-5): every call asks `startSession` for a new single-use ticket, so a ticket is never reused.
 * The ticket goes straight to `openTunnel` and is dropped; the caller never holds the token.
 */
export async function startTunnel(
  api: Pick<SessionsApi, 'startSession' | 'endSession'>,
  target: SessionTarget,
  setup: TunnelSetup,
): Promise<TunnelStart> {
  const started = await api.startSession(target.deviceId, target.display);
  if (!started.ok) return started;
  const { sessionId } = started.value;
  const opening = openTunnel({ ...setup, ticket: started.value });
  if (opening.ok) return { ok: true, value: { sessionId, tunnel: opening.value } };
  // The session row exists but will never get a tunnel, so end it. The refusal is what the operator needs to
  // see; a failed `endSession` here is not reported over it.
  await api.endSession(sessionId);
  return opening;
}
