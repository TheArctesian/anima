import type { HubError } from '$lib/hub/hub';
import type { RemoteProtocol, SessionDisplay, SessionsApi } from '../api';
import { startTunnel } from '../tunnel/guacamole/start';
import type { GuacamoleState, SessionTunnel } from '../tunnel/guacamole/tunnel';
import { startTerminal, type Started, type StartResult } from '../tunnel/start';
import type { SocketConstructor, TunnelRefusal, TunnelState } from '../tunnel/state';
import type { CellSize } from '../tunnel/terminal/codec';
import type { TerminalLink } from '../tunnel/terminal/link';
import { readPolicy, type SessionPolicy } from '../transfer';

/** What a session view hears from its terminal tunnel, and the size it gives it. */
export interface TunnelEvents {
  /** The fitted terminal size in cells, read when the socket opens (K4 §8.4). */
  readonly size: () => CellSize;
  /** Terminal output from the hub. */
  readonly onbytes: (bytes: Uint8Array) => void;
  readonly onchange: (state: TunnelState) => void;
  readonly onend: (state: TunnelState) => void;
}

/** What a remote view hears from its Guacamole tunnel. */
export interface RemoteEvents {
  readonly onchange: (state: GuacamoleState) => void;
  readonly onend: (state: GuacamoleState) => void;
}

/** A started session with the policy read at its start (RA-033); the ticket is not part of it (SEC-002). */
export interface PolicedSession<Tunnel> extends Started<Tunnel> {
  readonly policy: SessionPolicy;
}

/** The outcome of one view start. */
export type ViewStartResult<Tunnel> =
  | { readonly ok: true; readonly value: PolicedSession<Tunnel> }
  | { readonly ok: false; readonly error: HubError | TunnelRefusal; readonly wakeSent?: boolean };

/** A started SSH session (K4 §8). */
export type TerminalStartResult = ViewStartResult<TerminalLink>;

/** A started RDP or VNC session (K4 §1–§5). */
export type RemoteStartResult = ViewStartResult<SessionTunnel>;

/**
 * The seam a `SessionView` starts through: one call asks the hub for a new single-use ticket for the view's device
 * and opens its terminal tunnel, reporting to `events`. "Reconnect" is another call (D-5). The view never sees the
 * ticket or its token (SEC-002). `display` is the pane's size, which the hub's `startSession` still takes.
 */
export type SessionStart = (display: SessionDisplay, events: TunnelEvents) => Promise<TerminalStartResult>;

/** The same seam for a remote view: a new ticket and a Guacamole tunnel per call (K4 §1). */
export type RemoteStart = (display: SessionDisplay, events: RemoteEvents) => Promise<RemoteStartResult>;

/** Every start a device's session tab may use, one per tunnel. */
export interface DeviceStarts {
  /** SSH, over the terminal tunnel. */
  readonly terminal: SessionStart;
  /** RDP or VNC, over the Guacamole tunnel. */
  readonly remote: (protocol: RemoteProtocol) => RemoteStart;
}

/** `start` with the policy read beside it; both run at once, so the policy costs no extra round trip. */
async function policed<Tunnel>(policy: Promise<SessionPolicy>, start: Promise<StartResult<Tunnel>>): Promise<ViewStartResult<Tunnel>> {
  const [read, result] = await Promise.all([policy, start]);
  return result.ok ? { ok: true, value: { ...result.value, policy: read } } : result;
}

/**
 * The starts for `deviceId` on the hub at `origin` (`hub.address.origin`):
 * `sessionStart(sessionsApi(hub), deviceId, hub.address.origin)`. Each start asks `startSession` for the tab's
 * protocol and reads the device's transfer policy at the same time (RA-033).
 */
export function sessionStart(
  api: Pick<SessionsApi, 'startSession' | 'endSession' | 'transferPolicy'>,
  deviceId: string,
  origin: string,
  WebSocket?: SocketConstructor,
): DeviceStarts {
  return {
    terminal: (display, events) =>
      policed(readPolicy(api, deviceId), startTerminal(api, { deviceId, display }, { origin, WebSocket, ...events })),
    remote: (protocol) => (display, events) =>
      policed(readPolicy(api, deviceId), startTunnel(api, { deviceId, protocol, display }, { origin, WebSocket, ...events })),
  };
}
