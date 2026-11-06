import type { SessionDisplay, SessionsApi } from '../api';
import { startTerminal, type TunnelStart } from '../tunnel/start';
import type { SocketConstructor, TunnelState } from '../tunnel/state';
import type { CellSize } from '../tunnel/terminal/codec';

/** What a session view hears from its terminal tunnel, and the size it gives it. */
export interface TunnelEvents {
  /** The fitted terminal size in cells, read when the socket opens (K4 §8.4). */
  readonly size: () => CellSize;
  /** Terminal output from the hub. */
  readonly onbytes: (bytes: Uint8Array) => void;
  readonly onchange: (state: TunnelState) => void;
  readonly onend: (state: TunnelState) => void;
}

/**
 * The seam a `SessionView` starts through: one call asks the hub for a new single-use ticket for the view's device
 * and opens its terminal tunnel, reporting to `events`. "Reconnect" is another call (D-5). The view never sees the
 * ticket or its token (SEC-002). `display` is the pane's size, which the hub's `startSession` still takes.
 */
export type SessionStart = (display: SessionDisplay, events: TunnelEvents) => Promise<TunnelStart>;

/**
 * The `SessionStart` for `deviceId` on the hub at `origin` (`hub.address.origin`), over `startTerminal`:
 * `sessionStart(sessionsApi(hub), deviceId, hub.address.origin)`.
 */
export function sessionStart(
  api: Pick<SessionsApi, 'startSession' | 'endSession'>,
  deviceId: string,
  origin: string,
  WebSocket?: SocketConstructor,
): SessionStart {
  return (display, events) => startTerminal(api, { deviceId, display }, { origin, WebSocket, ...events });
}
