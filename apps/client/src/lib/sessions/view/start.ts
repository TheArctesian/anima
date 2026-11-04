import type { SessionDisplay, SessionsApi } from '../api';
import { startTunnel, type TunnelStart } from '../tunnel/start';
import type { SocketConstructor, TunnelState } from '../tunnel/tunnel';

/** What a session view hears from its tunnel. */
export interface TunnelEvents {
  readonly onchange: (state: TunnelState) => void;
  readonly onend: (state: TunnelState) => void;
}

/**
 * The seam a `SessionView` starts through: one call asks the hub for a new single-use ticket for the view's device
 * and opens its tunnel with `display`, reporting to `events`. "Reconnect" is another call (D-5). The view never
 * sees the ticket or its token (SEC-002).
 */
export type SessionStart = (display: SessionDisplay, events: TunnelEvents) => Promise<TunnelStart>;

/**
 * The `SessionStart` for `deviceId` on the hub at `origin` (`hub.address.origin`), over T1's `startTunnel`:
 * `sessionStart(sessionsApi(hub), deviceId, hub.address.origin)`.
 */
export function sessionStart(
  api: Pick<SessionsApi, 'startSession' | 'endSession'>,
  deviceId: string,
  origin: string,
  WebSocket?: SocketConstructor,
): SessionStart {
  return (display, events) => startTunnel(api, { deviceId, display }, { origin, WebSocket, ...events });
}
