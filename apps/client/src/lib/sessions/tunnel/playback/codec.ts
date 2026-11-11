/** The WebSocket subprotocol of the playback tunnel (K4 §12). */
export const RECORDING_SUBPROTOCOL = 'anima-recording';

/** The only tunnel path a playback ticket names (K4 §1). */
export const RECORDING_PATH = '/recording';

/** The hub sends the plaintext in binary frames of at most 64 KiB (K4 §12); a larger one is a protocol fault. */
export const FRAME_MAX_BYTES = 65_536;

/** No message of the closed set is longer than this; a longer text is not parsed at all (K4 §8.5). */
export const TEXT_MAX_CHARS = 256;

/** The recording formats the hub names in the header (K1 `RecordingFormat`, K4 §11). */
export const RECORDING_FORMATS = ['GUACAMOLE', 'ASCIICAST'] as const;
export type RecordingFormat = (typeof RECORDING_FORMATS)[number];

/** Why the hub stopped sending before `done` (K4 §12): a part failed to decrypt, or the file is gone. */
export const RECORDING_FAILURES = ['DAMAGED', 'MISSING'] as const;
export type RecordingFailure = (typeof RECORDING_FAILURES)[number];

/** The first message of a playback (K4 §12). `complete` is false for a recording that has no last part. */
export interface RecordingHeader {
  readonly format: RecordingFormat;
  readonly sizeBytes: number;
  readonly complete: boolean;
}

/**
 * A text message from the hub (K4 §12). `malformed` is a message of a known type whose fields are not those of the
 * contract; it is a protocol fault, unlike a message of an unknown type, which a newer hub may send and the client
 * ignores (as the terminal does, K4 §8.4).
 */
export type HubMessage =
  | { readonly kind: 'recording'; readonly header: RecordingHeader }
  | { readonly kind: 'ping' }
  | { readonly kind: 'done' }
  | { readonly kind: 'failed'; readonly reason: RecordingFailure }
  | { readonly kind: 'malformed' };

const MALFORMED: HubMessage = { kind: 'malformed' };

function oneOf<T extends string>(allowed: readonly T[], value: unknown): T | undefined {
  return allowed.find((candidate) => candidate === value);
}

/** Whether `fields` has exactly the keys `keys`, each once. */
function hasExactly(fields: Record<string, unknown>, keys: readonly string[]): boolean {
  const own = Object.keys(fields);
  return own.length === keys.length && keys.every((key) => Object.hasOwn(fields, key));
}

function header(fields: Record<string, unknown>): HubMessage {
  const format = oneOf(RECORDING_FORMATS, fields.format);
  const { sizeBytes, complete } = fields;
  if (!hasExactly(fields, ['type', 'format', 'sizeBytes', 'complete']) || format === undefined) return MALFORMED;
  if (typeof sizeBytes !== 'number' || !Number.isSafeInteger(sizeBytes) || sizeBytes < 0) return MALFORMED;
  if (typeof complete !== 'boolean') return MALFORMED;
  return { kind: 'recording', header: { format, sizeBytes, complete } };
}

/**
 * The end of a playback before `done`. K4 §12 writes the missing-file message as `{"reason":"MISSING"}`; it is read
 * as `{"type":"failed","reason":"MISSING"}` too, so either form ends the playback the same way.
 */
function failure(fields: Record<string, unknown>): HubMessage {
  const reason = oneOf(RECORDING_FAILURES, fields.reason);
  if (reason === undefined) return MALFORMED;
  if (fields.type === 'failed' && hasExactly(fields, ['type', 'reason'])) return { kind: 'failed', reason };
  if (reason === 'MISSING' && hasExactly(fields, ['reason'])) return { kind: 'failed', reason };
  return MALFORMED;
}

function parsed(text: string): Record<string, unknown> | undefined {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return undefined;
  }
  return typeof value === 'object' && value !== null && !Array.isArray(value) ? (value as Record<string, unknown>) : undefined;
}

/**
 * Read a text message from the hub (K4 §12): `recording` (the header), `ping`, `done` and `failed`. A text that is
 * not a JSON object, too long, or of an unknown type gives `undefined` (ignored).
 */
export function decodeHubMessage(text: string): HubMessage | undefined {
  if (text.length > TEXT_MAX_CHARS) return undefined;
  const fields = parsed(text);
  if (fields === undefined) return undefined;
  switch (fields.type) {
    case 'recording':
      return header(fields);
    case 'ping':
      return hasExactly(fields, ['type']) ? { kind: 'ping' } : undefined;
    case 'done':
      return hasExactly(fields, ['type']) ? { kind: 'done' } : MALFORMED;
    case 'failed':
      return failure(fields);
    case undefined:
      return Object.hasOwn(fields, 'reason') ? failure(fields) : undefined;
    default:
      return undefined;
  }
}

/** The client's `ping` message (K4 §8.4, §12). */
export function encodePing(): string {
  return JSON.stringify({ type: 'ping' });
}

/** The client's `cancel` message: the hub stops reading the recording (K4 §12). */
export function encodeCancel(): string {
  return JSON.stringify({ type: 'cancel' });
}
