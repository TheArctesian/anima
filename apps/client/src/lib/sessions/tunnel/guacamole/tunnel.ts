// The Guacamole tunnel is kept for RDP and VNC, which still use guacd; SSH uses the terminal tunnel, K4 Section 8. The set kept:
// this folder (`tunnel.ts`, `socket.ts`, `input.ts`, `start.ts`), `../vendor/`, `../fake/` and, in the view,
// `view/guacamole/` and the size helpers of `view/display.ts`. The SSH path imports none of it.
import Guacamole from '../vendor/guacamole.js';
import { withObservedSocket } from './socket';
import { refusal, type SocketConstructor, type TunnelPhase, type TunnelRefusal, type TunnelState, type TunnelTicket } from '../state';
import { tunnelUrl } from '../url';

export type { SocketConstructor, TunnelRefusal, TunnelTicket } from '../state';

/** The 1.5.5 client states as phases: `waiting` lasts until the gateway's first frame (`sync`). */
export type GuacamolePhase = TunnelPhase | 'waiting';

/** What the Guacamole tunnel knows; it holds no token, no URL and no gateway text (SEC-002, LOG-007). */
export interface GuacamoleState extends Omit<TunnelState, 'phase'> {
  readonly phase: GuacamolePhase;
  /** The status code of the last gateway `error` instruction; its text is never kept (R-035). */
  readonly gatewayError: number | undefined;
  /**
   * The parameter names of the last gateway `required` instruction (GW-015), a new array for each one; kept in the
   * state so that one that arrives before the start has answered is not lost. The caller checks the names.
   */
  readonly required?: readonly string[] | undefined;
}

export interface TunnelOptions {
  /** The hub origin, `https://host[:port]` (`hub.address.origin`). */
  readonly origin: string;
  readonly ticket: TunnelTicket;
  /** The hub sent a Wake-on-LAN packet, so it may take up to `WAKE_OPENING_LIMIT_MS` to open the socket (K4 §1). */
  readonly waking: boolean;
  /** Hears every change of the state after `openTunnel` has connected; the first call gives the state then. */
  readonly onchange?: (state: GuacamoleState) => void;
  /** Hears the final state once, after the WebSocket closed and the client stopped. */
  readonly onend?: (state: GuacamoleState) => void;
  /** The WebSocket constructor; defaults to the global one (CQ-021). */
  readonly WebSocket?: SocketConstructor;
}

/**
 * One open tunnel. The view draws `client.getDisplay()` and sends input through `client`. The tunnel owns
 * `client.onstatechange` and `client.onerror`; set any other hook (such as `onclipboard`) on the client directly.
 */
export interface SessionTunnel {
  readonly client: Guacamole.Client;
  state(): GuacamoleState;
  /** Disconnect from the gateway and close the socket; safe to call more than once. */
  close(): void;
}

export type TunnelOpening =
  | { readonly ok: true; readonly value: SessionTunnel }
  | { readonly ok: false; readonly error: TunnelRefusal };

/** The library's own limit on silence, in ms: it also bounds the opening, from `connect` to the first instruction. */
const RECEIVE_LIMIT_MS = 15_000;
/** The hub's opening limit for a start that sent a Wake-on-LAN packet (K4 §1). */
export const WAKE_OPENING_LIMIT_MS = 120_000;

/** The 1.5.5 client states (private in the library), as phases; 0 IDLE follows a failed `connect`. */
const PHASES: readonly GuacamolePhase[] = ['disconnected', 'connecting', 'waiting', 'connected', 'disconnected', 'disconnected'];

interface Events {
  readonly onchange?: (state: GuacamoleState) => void;
  readonly onend?: (state: GuacamoleState) => void;
}

/** A client on a WebSocket tunnel to `url`, with its state tracked; nothing in this scope sees the token. */
function trackedTunnel(url: string, events: Events, waking: boolean) {
  const tunnel = new Guacamole.WebSocketTunnel(url);
  // The library closes a tunnel that hears nothing for `receiveTimeout`, from `connect` on; a waking device may
  // take the hub longer than that to open the socket, so the wider limit lasts only until it is open.
  tunnel.receiveTimeout = waking ? WAKE_OPENING_LIMIT_MS : RECEIVE_LIMIT_MS;
  const client = new Guacamole.Client(tunnel);
  const initial: GuacamoleState = { phase: 'connecting', opened: false, gatewayError: undefined, closeCode: undefined };
  let state = initial;
  // `onchange` is held back while `connect` runs (the global WebSocket is swapped then, see `socket.ts`).
  let connected = false;
  const update = (change: Partial<GuacamoleState>) => {
    if (Object.entries(change).every(([key, value]) => state[key as keyof GuacamoleState] === value)) return;
    state = { ...state, ...change };
    if (connected) events.onchange?.(state);
  };
  /** `connect` has returned: report the state it reached, once, if it changed. */
  const release = () => {
    connected = true;
    if (state !== initial) events.onchange?.(state);
  };
  client.onstatechange = (clientState) => {
    update({ phase: PHASES[clientState] ?? 'disconnected' });
  };
  client.onerror = (status) => {
    update({ gatewayError: status.code });
  };
  client.onrequired = (parameters) => {
    update({ required: [...parameters] });
  };
  tunnel.onstatechange = (tunnelState) => {
    if (tunnelState === Guacamole.Tunnel.State.OPEN) {
      tunnel.receiveTimeout = RECEIVE_LIMIT_MS;
      update({ opened: true });
    }
    // The 1.5.5 client does not watch its tunnel; stop it so that its keep-alive timer ends too.
    if (tunnelState === Guacamole.Tunnel.State.CLOSED) client.disconnect();
  };
  const socketClosed = (code: number) => {
    // A socket closed after a failed `connect` belongs to a tunnel that was refused, not ended.
    if (!connected) return;
    update({ closeCode: code });
    // The listener runs before the library handles the close; report the end after it has. It is added to the
    // tunnel's one socket only, and a WebSocket fires `close` once, so the end is reported once.
    queueMicrotask(() => {
      events.onend?.(state);
    });
  };
  const handle: SessionTunnel = {
    client,
    state: () => state,
    close: () => {
      client.disconnect();
    },
  };
  // After a `connect` that threw there is no socket, and 1.5.5's `disconnect` would call `close` on null:
  // marking the tunnel closed first makes it only clear its timers (and stops the client through the hook above).
  const abandon = () => {
    tunnel.setState(Guacamole.Tunnel.State.CLOSED);
  };
  return { handle, socketClosed, abandon, release };
}

/**
 * Open the session tunnel for `ticket` at the hub `origin` (K4 §2, D-3): a `Guacamole.WebSocketTunnel` to the
 * hub address with https→wss (http→ws) plus `ticket.tunnelPath`, connected with `token=<token>` and nothing
 * else (GW-013). The library adds the connect data as the query string, with subprotocol `guacamole`.
 */
export function openTunnel(options: TunnelOptions): TunnelOpening {
  const url = tunnelUrl(options.origin, options.ticket.tunnelPath);
  if (url === undefined) return refusal('bad-address');
  const { handle, socketClosed, abandon, release } = trackedTunnel(
    url,
    { onchange: options.onchange, onend: options.onend },
    options.waking,
  );
  try {
    withObservedSocket(options.WebSocket ?? globalThis.WebSocket, socketClosed, () => {
      handle.client.connect(`token=${encodeURIComponent(options.ticket.token)}`);
    });
  } catch {
    // The WebSocket constructor's error names the URL, which holds the token; only the fact is kept (LOG-007).
    abandon();
    return refusal('not-opened');
  }
  release();
  return { ok: true, value: handle };
}
