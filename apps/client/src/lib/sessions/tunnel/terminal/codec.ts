import { isEndReason, type SessionEndReason } from '../../ending';

/** The WebSocket subprotocol of the terminal tunnel (K4 §8.1). */
export const TERMINAL_SUBPROTOCOL = 'anima-terminal';

/** The terminal size in cells the hub accepts (K4 §8.4). */
export const CELLS_MIN = 1;
export const CELLS_MAX = 1000;

/** The largest binary message the client sends is 64 KiB (K4 §8.5); input is cut well under it. */
export const FRAME_MAX_BYTES = 32_768;

/** No message of the closed set is longer than this; a longer text is not parsed at all. */
export const TEXT_MAX_CHARS = 256;

/** A terminal size in character cells. */
export interface CellSize {
  readonly cols: number;
  readonly rows: number;
}

/** A message from the hub's closed set (K4 §8.4). */
export type HubMessage =
  | { readonly kind: 'ping' }
  | { readonly kind: 'end'; readonly reason: SessionEndReason };

function cells(value: number): number {
  return Number.isFinite(value) ? Math.min(CELLS_MAX, Math.max(CELLS_MIN, Math.round(value))) : CELLS_MIN;
}

/** `size` with each side rounded and clamped to 1..1000 (K4 §8.4). */
export function clampSize(size: CellSize): CellSize {
  return { cols: cells(size.cols), rows: cells(size.rows) };
}

/** The `resize` message for `size`, clamped as `clampSize` does. */
export function encodeResize(size: CellSize): string {
  return JSON.stringify({ type: 'resize', ...clampSize(size) });
}

/** The client's `ping` message (K4 §8.4). */
export function encodePing(): string {
  return JSON.stringify({ type: 'ping' });
}

/**
 * Read a text message from the hub. The set is closed: `{"type":"ping"}` and `{"type":"end","reason":R}` with a
 * reason this client knows. Anything else (another type, extra fields, bad JSON, an unknown reason) gives
 * `undefined`, and the caller ignores it: a newer hub may add messages, and an old client must not end a session
 * for one. The close code that follows an `end` still tells how the session ended.
 */
export function decodeHubMessage(text: string): HubMessage | undefined {
  if (text.length > TEXT_MAX_CHARS) return undefined;
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return undefined;
  }
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return undefined;
  const fields = value as Record<string, unknown>;
  const keys = Object.keys(fields);
  if (fields.type === 'ping' && keys.length === 1) return { kind: 'ping' };
  if (fields.type === 'end' && keys.length === 2 && isEndReason(fields.reason)) return { kind: 'end', reason: fields.reason };
  return undefined;
}

/** `bytes` cut into pieces of at most `FRAME_MAX_BYTES`, in order, so a large paste is never one oversize frame. */
export function inputFrames(bytes: Uint8Array<ArrayBuffer>): Uint8Array<ArrayBuffer>[] {
  const frames: Uint8Array<ArrayBuffer>[] = [];
  for (let at = 0; at < bytes.length; at += FRAME_MAX_BYTES) frames.push(bytes.subarray(at, at + FRAME_MAX_BYTES));
  return frames;
}
