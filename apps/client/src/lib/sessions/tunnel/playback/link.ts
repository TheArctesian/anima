import { refusal, type SocketConstructor, type TunnelRefusal, type TunnelTicket } from '../state';
import { PING_AFTER_MS, SILENCE_LIMIT_MS } from '../terminal/link';
import { tunnelUrl } from '../url';
import {
  FRAME_MAX_BYTES,
  RECORDING_PATH,
  RECORDING_SUBPROTOCOL,
  decodeHubMessage,
  encodeCancel,
  encodePing,
  type HubMessage,
  type RecordingHeader,
} from './codec';

export { PING_AFTER_MS, SILENCE_LIMIT_MS };

/**
 * How a playback ended. `done`: every part arrived. `damaged`: a part failed to decrypt, after the parts that
 * verified; `missing`: the file is gone (K4 §12). `refused`: the socket closed before it opened (K4 §8.1).
 * `interrupted`: it closed before `done` (see `closeCode`). `lost`: nothing came for `SILENCE_LIMIT_MS`. `fault`: the
 * hub broke the protocol (no header first, a bad header, an oversize frame). `cancelled`: the owner closed it.
 */
export type RecordingOutcome = 'done' | 'damaged' | 'missing' | 'refused' | 'interrupted' | 'lost' | 'fault' | 'cancelled';

/** What a playback link knows; it holds no token, no URL and no recording content (SEC-002, LOG-007). */
export interface RecordingLinkState {
  /** `connecting` until the upgrade, `receiving` while it is open, `ended` once it has an outcome. */
  readonly phase: 'connecting' | 'receiving' | 'ended';
  /** The header, once the hub sent it. */
  readonly header: RecordingHeader | undefined;
  /** Plaintext bytes received so far. */
  readonly receivedBytes: number;
  /** The WebSocket close code, when the socket closed before the playback had another outcome. */
  readonly closeCode: number | undefined;
  readonly outcome: RecordingOutcome | undefined;
}

/** What the link tells its owner. */
export interface RecordingLinkEvents {
  /** Each binary frame of plaintext, in order, after the header. */
  readonly onbytes: (bytes: Uint8Array) => void;
  /** Hears every change of the state after `openRecordingLink` returned. */
  readonly onchange?: (state: RecordingLinkState) => void;
  /** Hears the final state once. */
  readonly onend?: (state: RecordingLinkState) => void;
}

export interface RecordingLinkOptions extends RecordingLinkEvents {
  /** The hub origin, `https://host[:port]` (`hub.address.origin`). */
  readonly origin: string;
  /** The ticket from `openRecording`; its `tunnelPath` must be `/recording` (K4 §1). */
  readonly ticket: TunnelTicket;
  /** The WebSocket constructor; defaults to the global one (CQ-021). */
  readonly WebSocket?: SocketConstructor;
}

/** One open playback tunnel (K4 §12). */
export interface RecordingLink {
  state(): RecordingLinkState;
  /** Send `cancel`, close the socket (1000) and end as `cancelled`; safe to call more than once. */
  close(): void;
}

export type RecordingLinkOpening =
  | { readonly ok: true; readonly value: RecordingLink }
  | { readonly ok: false; readonly error: TunnelRefusal };

const OPEN = 1;
const CLOSING = 2;

/**
 * Open the playback tunnel for `ticket` at the hub `origin` (K4 §12, RA-041): a WebSocket to the hub address with
 * https→wss (http→ws to loopback) plus `/recording`, `?token=<token>` and subprotocol `anima-recording`. The token
 * lives only in that one expression: the link keeps the events, not the ticket.
 */
export function openRecordingLink(options: RecordingLinkOptions): RecordingLinkOpening {
  const { origin, ticket, WebSocket: Socket = globalThis.WebSocket, ...events } = options;
  const url = ticket.tunnelPath === RECORDING_PATH ? tunnelUrl(origin, ticket.tunnelPath) : undefined;
  if (url === undefined) return refusal('bad-address');
  let socket: WebSocket;
  try {
    socket = new Socket(`${url}?token=${encodeURIComponent(ticket.token)}`, RECORDING_SUBPROTOCOL);
  } catch {
    // The constructor's error names the URL, which holds the token; only the fact is kept (LOG-007).
    return refusal('not-opened');
  }
  socket.binaryType = 'arraybuffer';
  return { ok: true, value: linkOver(socket, events) };
}

/** The two liveness timers: a ping after our own quiet spell (K4 §8.4), a lost link after the hub's (K4 §8.5). */
function liveness(ping: () => void, lost: () => void) {
  let pingTimer: ReturnType<typeof setTimeout> | undefined;
  let silenceTimer: ReturnType<typeof setTimeout> | undefined;
  return {
    sent: () => {
      clearTimeout(pingTimer);
      pingTimer = setTimeout(ping, PING_AFTER_MS);
    },
    heard: () => {
      clearTimeout(silenceTimer);
      silenceTimer = setTimeout(lost, SILENCE_LIMIT_MS);
    },
    stop: () => {
      clearTimeout(pingTimer);
      clearTimeout(silenceTimer);
    },
  };
}

/** What one message from the hub does: a state change, an outcome that ends the link, or nothing. */
type Step = { readonly change: Partial<RecordingLinkState> } | { readonly outcome: RecordingOutcome } | undefined;

/** A text message against the state so far: the header comes once, first; `done` and `failed` end the playback. */
function textStep(message: HubMessage | undefined, state: RecordingLinkState): Step {
  switch (message?.kind) {
    case 'recording':
      return state.header === undefined ? { change: { header: message.header } } : { outcome: 'fault' };
    case 'done':
      return { outcome: 'done' };
    case 'failed':
      return { outcome: message.reason === 'DAMAGED' ? 'damaged' : 'missing' };
    case 'malformed':
      return { outcome: 'fault' };
    default:
      return undefined;
  }
}

/** A frame from the hub: binary is plaintext (after the header, at most 64 KiB), text is a control message. */
function frameStep(data: unknown, state: RecordingLinkState, onbytes: RecordingLinkEvents['onbytes']): Step {
  if (typeof data === 'string') return textStep(decodeHubMessage(data), state);
  if (!(data instanceof ArrayBuffer) || state.header === undefined || data.byteLength > FRAME_MAX_BYTES) return { outcome: 'fault' };
  onbytes(new Uint8Array(data));
  return { change: { receivedBytes: state.receivedBytes + data.byteLength } };
}

/** The link on `socket`: state, liveness and the message handling. */
function linkOver(socket: WebSocket, events: RecordingLinkEvents): RecordingLink {
  let state: RecordingLinkState = { phase: 'connecting', header: undefined, receivedBytes: 0, closeCode: undefined, outcome: undefined };
  /** Read through a call, so a check after a callback is not narrowed by one before it. */
  const ended = () => state.phase === 'ended';
  const publish = (change: Partial<RecordingLinkState>) => {
    state = { ...state, ...change };
    events.onchange?.(state);
  };
  const transmit = (frame: string) => {
    if (state.phase !== 'receiving' || socket.readyState !== OPEN) return;
    socket.send(frame);
    beats.sent();
  };
  /** End with `outcome`, once; `cancel` tells the hub to stop reading first. */
  const finish = (change: Partial<RecordingLinkState> & { readonly outcome: RecordingOutcome }, cancel: boolean) => {
    if (ended()) return;
    if (cancel) transmit(encodeCancel());
    beats.stop();
    publish({ ...change, phase: 'ended' });
    if (socket.readyState < CLOSING) socket.close(1000);
    events.onend?.(state);
  };
  const beats = liveness(
    () => {
      transmit(encodePing());
    },
    () => {
      finish({ outcome: 'lost' }, true);
    },
  );

  socket.onopen = () => {
    beats.heard();
    beats.sent();
    publish({ phase: 'receiving' });
  };
  socket.onmessage = (event: MessageEvent) => {
    if (state.phase !== 'receiving') return;
    beats.heard();
    const step = frameStep(event.data, state, events.onbytes);
    // The owner may have closed the link from `onbytes`.
    if (step === undefined || ended()) return;
    if ('change' in step) publish(step.change);
    // After `done` or `failed` the hub has nothing left to read; anything else that ends here stops it reading.
    else finish({ outcome: step.outcome }, step.outcome === 'fault');
  };
  socket.onclose = (event: CloseEvent) => {
    finish({ outcome: state.phase === 'connecting' ? 'refused' : 'interrupted', closeCode: event.code }, false);
  };

  return {
    state: () => state,
    close: () => {
      finish({ outcome: 'cancelled' }, true);
    },
  };
}
