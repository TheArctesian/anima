import Guacamole from './vendor/guacamole.js';
import { withObservedSocket, type SocketConstructor } from './socket';
import { tunnelUrl } from './url';

export type { SocketConstructor } from './socket';

/**
 * Where the session stands, from the 1.5.0 client states: `connecting` while the socket is made, `waiting` until
 * the gateway's first frame (`sync`), `connected` after it, `disconnected` once the client or the tunnel stopped.
 */
export type TunnelPhase = 'connecting' | 'waiting' | 'connected' | 'disconnected';

/** What the tunnel knows; it holds no token, no URL and no gateway text (SEC-002, LOG-007). */
export interface TunnelState {
  readonly phase: TunnelPhase;
  /** Whether the hub's first instruction arrived, so the WebSocket upgrade succeeded (K4 §4). */
  readonly opened: boolean;
  /** The status code of the last gateway `error` instruction; its text is never kept (R-035). */
  readonly gatewayError: number | undefined;
  /** The WebSocket close code, once the socket has closed (K4 §5). */
  readonly closeCode: number | undefined;
}

/** The ticket parts the tunnel needs (K4 §1). The token is used once, for `connect`, and never kept. */
export interface TunnelTicket {
  readonly tunnelPath: string;
  readonly token: string;
}

export interface TunnelOptions {
  /** The hub origin, `https://host[:port]` (`hub.address.origin`). */
  readonly origin: string;
  readonly ticket: TunnelTicket;
  /** Hears every change of the state. */
  readonly onchange?: (state: TunnelState) => void;
  /** Hears the final state once, after the WebSocket closed and the client stopped. */
  readonly onend?: (state: TunnelState) => void;
  /** The WebSocket constructor; defaults to the global one (CQ-021). */
  readonly WebSocket?: SocketConstructor;
}

/**
 * One open tunnel. The view draws `client.getDisplay()` and sends input through `client`. The tunnel owns
 * `client.onstatechange` and `client.onerror`; set any other hook (such as `onclipboard`) on the client directly.
 */
export interface SessionTunnel {
  readonly client: Guacamole.Client;
  state(): TunnelState;
  /** Disconnect from the gateway and close the socket; safe to call more than once. */
  close(): void;
}

/** Why no tunnel was opened. The message is fixed text: it never holds the token, the URL or a library error. */
export interface TunnelRefusal {
  readonly problem: 'bad-address' | 'not-opened';
  readonly message: string;
}

export type TunnelOpening =
  | { readonly ok: true; readonly value: SessionTunnel }
  | { readonly ok: false; readonly error: TunnelRefusal };

const REFUSALS: Readonly<Record<TunnelRefusal['problem'], string>> = {
  'bad-address': 'The hub gave a tunnel address that Anima does not accept.',
  'not-opened': 'The session tunnel could not be opened.',
};

/** The 1.5.0 client states (private in the library), as phases; 0 IDLE follows a failed `connect`. */
const PHASES: readonly TunnelPhase[] = ['disconnected', 'connecting', 'waiting', 'connected', 'disconnected', 'disconnected'];

interface Events {
  readonly onchange?: (state: TunnelState) => void;
  readonly onend?: (state: TunnelState) => void;
}

/** A client on a WebSocket tunnel to `url`, with its state tracked; nothing in this scope sees the token. */
function trackedTunnel(url: string, events: Events) {
  const tunnel = new Guacamole.WebSocketTunnel(url);
  const client = new Guacamole.Client(tunnel);
  let state: TunnelState = { phase: 'connecting', opened: false, gatewayError: undefined, closeCode: undefined };
  let ended = false;
  const update = (change: Partial<TunnelState>) => {
    if (Object.entries(change).every(([key, value]) => state[key as keyof TunnelState] === value)) return;
    state = { ...state, ...change };
    events.onchange?.(state);
  };
  client.onstatechange = (clientState) => {
    update({ phase: PHASES[clientState] ?? 'disconnected' });
  };
  client.onerror = (status) => {
    update({ gatewayError: status.code });
  };
  tunnel.onstatechange = (tunnelState) => {
    if (tunnelState === Guacamole.Tunnel.State.OPEN) update({ opened: true });
    // The 1.5.0 client does not watch its tunnel; stop it so that its keep-alive timer ends too.
    if (tunnelState === Guacamole.Tunnel.State.CLOSED) client.disconnect();
  };
  const socketClosed = (code: number) => {
    update({ closeCode: code });
    // The listener runs before the library handles the close; report the end after it has.
    queueMicrotask(() => {
      if (ended) return;
      ended = true;
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
  // After a `connect` that threw there is no socket, and 1.5.0's `disconnect` would call `close` on null:
  // marking the tunnel closed first makes it only clear its timers (and stops the client through the hook above).
  const abandon = () => {
    tunnel.setState(Guacamole.Tunnel.State.CLOSED);
  };
  return { handle, socketClosed, abandon };
}

/**
 * Open the session tunnel for `ticket` at the hub `origin` (K4 §2, D-3): a `Guacamole.WebSocketTunnel` to the
 * hub address with https→wss (http→ws) plus `ticket.tunnelPath`, connected with `token=<token>` and nothing
 * else (GW-013). The library adds the connect data as the query string, with subprotocol `guacamole`.
 */
export function openTunnel(options: TunnelOptions): TunnelOpening {
  const url = tunnelUrl(options.origin, options.ticket.tunnelPath);
  if (url === undefined) return refused('bad-address');
  const { handle, socketClosed, abandon } = trackedTunnel(url, { onchange: options.onchange, onend: options.onend });
  try {
    withObservedSocket(options.WebSocket ?? globalThis.WebSocket, socketClosed, () => {
      handle.client.connect(`token=${encodeURIComponent(options.ticket.token)}`);
    });
  } catch {
    // The WebSocket constructor's error names the URL, which holds the token; only the fact is kept (LOG-007).
    abandon();
    return refused('not-opened');
  }
  return { ok: true, value: handle };
}

function refused(problem: TunnelRefusal['problem']): TunnelOpening {
  return { ok: false, error: { problem, message: REFUSALS[problem] } };
}
