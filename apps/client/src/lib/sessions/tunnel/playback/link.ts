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

/**
 * The most plaintext one playback takes by default: the hub's default `ANIMA_RECORDING_MAX_BYTES` (D-14, K7). A hub
 * that sends more ends the link as `too-large`; what came before still plays.
 */
export const RECORDING_MAX_BYTES = 2_147_483_648;

/**
 * How a playback ended. `done`: every part arrived (a complete recording that ends with `receivedBytes` below
 * `header.sizeBytes` also ends `done`; K4 §12 does not define it, so the owner compares the two). `damaged`: a part failed to decrypt, after the parts that
 * verified; `missing`: the file is gone (K4 §12). `refused`: the socket closed before it opened (K4 §8.1).
 * `interrupted`: it closed before `done` (see `closeCode`). `lost`: nothing came for `SILENCE_LIMIT_MS`. `fault`: the
 * hub broke the protocol (no header first, `done` before the header, a bad header, an oversize frame, more bytes than
 * a complete recording's `sizeBytes`). `too-large`: the next frame would pass `maxBytes`; it is not delivered, and
 * the bytes before it may be played. `cancelled`: the owner closed it.
 */
export type RecordingOutcome =
  | 'done'
  | 'damaged'
  | 'missing'
  | 'refused'
  | 'interrupted'
  | 'lost'
  | 'fault'
  | 'too-large'
  | 'cancelled';

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
  /** The most plaintext the link delivers: `RECORDING_MAX_BYTES` unless a lower number is given (RA-041, D-14). */
  readonly maxBytes?: number;
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

/** The outcomes the hub itself sends; after them it reads nothing more, so the link sends no `cancel`. */
const HUB_ENDINGS: ReadonlySet<RecordingOutcome> = new Set(['done', 'damaged', 'missing']);

const OPEN = 1;
const CLOSING = 2;

/**
 * Open the playback tunnel for `ticket` at the hub `origin` (K4 §12, RA-041): a WebSocket to the hub address with
 * https→wss (http→ws to loopback) plus `/recording`, `?token=<token>` and subprotocol `anima-recording`. The token
 * lives only in that one expression: the link keeps the events, not the ticket.
 */
export function openRecordingLink(options: RecordingLinkOptions): RecordingLinkOpening {
  const { origin, ticket, WebSocket: Socket = globalThis.WebSocket, maxBytes = RECORDING_MAX_BYTES, ...events } = options;
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
  const limit = Number.isFinite(maxBytes) ? Math.min(maxBytes, RECORDING_MAX_BYTES) : RECORDING_MAX_BYTES;
  return { ok: true, value: linkOver(socket, events, limit) };
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

/**
 * A text message against the state so far: the header comes once, first; `done` and `failed` end the playback. Only
 * `failed` may come before the header (a missing file); `done` before it is a fault, so it never passes for a whole
 * recording (K4 §12).
 */
function textStep(message: HubMessage | undefined, state: RecordingLinkState): Step {
  switch (message?.kind) {
    case 'recording':
      return state.header === undefined ? { change: { header: message.header } } : { outcome: 'fault' };
    case 'done':
      return { outcome: state.header === undefined ? 'fault' : 'done' };
    case 'failed':
      return { outcome: message.reason === 'DAMAGED' ? 'damaged' : 'missing' };
    case 'malformed':
      return { outcome: 'fault' };
    default:
      return undefined;
  }
}

/**
 * A binary frame against the state so far: plaintext after the header, at most 64 KiB, never past a complete
 * recording's `sizeBytes` (a fault) or `maxBytes` (too large). An incomplete recording's `sizeBytes` bounds nothing.
 */
function bytesStep(data: unknown, state: RecordingLinkState, maxBytes: number): Step {
  const { header } = state;
  if (!(data instanceof ArrayBuffer) || header === undefined || data.byteLength > FRAME_MAX_BYTES) return { outcome: 'fault' };
  const receivedBytes = state.receivedBytes + data.byteLength;
  if (header.complete && receivedBytes > header.sizeBytes) return { outcome: 'fault' };
  if (receivedBytes > maxBytes) return { outcome: 'too-large' };
  return { change: { receivedBytes } };
}

/** The link on `socket`: state, liveness and the message handling. */
function linkOver(socket: WebSocket, events: RecordingLinkEvents, maxBytes: number): RecordingLink {
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
    const { data } = event as { readonly data: unknown };
    const step = typeof data === 'string' ? textStep(decodeHubMessage(data), state) : bytesStep(data, state, maxBytes);
    if (step === undefined) return;
    if ('outcome' in step) {
      // After `done` or `failed` the hub has nothing left to read; anything else that ends here stops it reading.
      finish({ outcome: step.outcome }, !HUB_ENDINGS.has(step.outcome));
      return;
    }
    if (data instanceof ArrayBuffer) events.onbytes(new Uint8Array(data));
    // The owner may have closed the link from `onbytes`.
    if (!ended()) publish(step.change);
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
