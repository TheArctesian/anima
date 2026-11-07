import type { SessionsApi } from '../../api';
import { startWith, type RemoteTarget, type Started, type StartResult } from '../start';
import { openTunnel, type SessionTunnel, type TunnelOptions } from './tunnel';

/** A started Guacamole session. */
export type StartedTunnel = Started<SessionTunnel>;

/** The Guacamole tunnel options other than the ticket (hub origin, hooks), which `startTunnel` gets from the hub. */
export type TunnelSetup = Omit<TunnelOptions, 'ticket' | 'waking'>;

/** Start a session and open its Guacamole tunnel (RDP and VNC; no SSH session uses it, R-062). */
export function startTunnel(
  api: Pick<SessionsApi, 'startSession' | 'endSession'>,
  target: RemoteTarget,
  setup: TunnelSetup,
): Promise<StartResult<SessionTunnel>> {
  return startWith(api, target, (ticket) => openTunnel({ ...setup, ticket, waking: ticket.wakeSent }));
}
