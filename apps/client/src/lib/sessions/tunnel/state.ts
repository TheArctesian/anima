import type { SessionEndReason } from '../ending';

/** The WebSocket constructor a tunnel uses; tests pass a fake one (CQ-021). */
export type SocketConstructor = typeof WebSocket;

/** The ticket parts a tunnel needs (K4 §1). The token is used once, to connect, and never kept. */
export interface TunnelTicket {
  readonly tunnelPath: string;
  readonly token: string;
}

/** Where the session stands: `connecting` while the socket is made, `connected` once it is open, `disconnected` after it ended. */
export type TunnelPhase = 'connecting' | 'connected' | 'disconnected';

/** What a tunnel knows; it holds no token, no URL and no gateway text (SEC-002, LOG-007). */
export interface TunnelState {
  readonly phase: TunnelPhase;
  /** Whether the WebSocket upgrade succeeded. */
  readonly opened: boolean;
  /** The WebSocket close code, once the socket has closed (K4 §5, §8.6). */
  readonly closeCode: number | undefined;
  /** The reason of the hub's `end` message (K4 §8.4). */
  readonly endReason?: SessionEndReason;
}

/** Why no tunnel was opened. The message is fixed text: it never holds the token, the URL or a library error. */
export interface TunnelRefusal {
  readonly problem: 'bad-address' | 'not-opened';
  readonly message: string;
}

const REFUSALS: Readonly<Record<TunnelRefusal['problem'], string>> = {
  'bad-address': 'The hub gave a tunnel address that Anima does not accept.',
  'not-opened': 'The session tunnel could not be opened.',
};

/** A refused opening, with the fixed text of `problem`. */
export function refusal(problem: TunnelRefusal['problem']): { readonly ok: false; readonly error: TunnelRefusal } {
  return { ok: false, error: { problem, message: REFUSALS[problem] } };
}
