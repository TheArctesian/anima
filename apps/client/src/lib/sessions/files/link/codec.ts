/** The WebSocket subprotocol of the files tunnel (K4 §9). */
export const FILES_SUBPROTOCOL = 'anima-files';

/** The longest path the hub takes, in bytes of UTF-8 (K4 §9). */
export const PATH_MAX_BYTES = 4096;
/** The longest text message the hub takes, in bytes of UTF-8 (K4 §9). */
export const TEXT_MAX_BYTES = 4096;
/** The largest binary frame the client sends (K4 §9). */
export const FRAME_MAX_BYTES = 65_536;
/** The most entries one `entries` message holds (K4 §9). */
export const ENTRIES_MAX = 10_000;

/** Why the hub refused a request (K4 §9 `refused.reason`). */
export type Refusal = 'DOWNLOAD_OFF' | 'UPLOAD_OFF' | 'NOT_FOUND' | 'DENIED' | 'EXISTS';
const REFUSALS: readonly string[] = ['DOWNLOAD_OFF', 'UPLOAD_OFF', 'NOT_FOUND', 'DENIED', 'EXISTS'] satisfies Refusal[];

/** What a directory entry is; the hub lists no other kinds (K4 §9). */
export type EntryKind = 'file' | 'dir' | 'link';
const KINDS: readonly string[] = ['file', 'dir', 'link'] satisfies EntryKind[];

/** One directory entry as the device reported it. `size` and `modifiedAt` are `null` when the device does not say. */
export interface FileEntry {
  /** The device's name for the entry, unchanged: show it as text, never as markup or a path of its own. */
  readonly name: string;
  readonly kind: EntryKind;
  readonly size: number | null;
  /** RFC 3339 time. */
  readonly modifiedAt: string | null;
}

/** A directory's entries; `path` is the directory as the device resolved it, which may differ from the one asked. */
export interface Listing {
  readonly path: string;
  readonly entries: readonly FileEntry[];
}

/** A request the client sends (K4 §9); `cancel` and `ping` have their own fixed messages. */
export type FilesRequest =
  | { readonly type: 'list'; readonly path: string }
  | { readonly type: 'download'; readonly path: string }
  | { readonly type: 'upload'; readonly path: string; readonly size: number };

/** A text message from the hub's closed set (K4 §9). */
export type HubMessage =
  | { readonly kind: 'entries'; readonly listing: Listing }
  | { readonly kind: 'download'; readonly size: number }
  | { readonly kind: 'done' }
  | { readonly kind: 'refused'; readonly reason: Refusal }
  | { readonly kind: 'ping' };

/** The client's `ping` (K4 §9). */
export const PING_MESSAGE = '{"type":"ping"}';
/** The client's `cancel` (K4 §9). */
export const CANCEL_MESSAGE = '{"type":"cancel"}';

/** An RFC 3339 time as the hub writes one (chrono). */
const RFC3339 = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/;
/** A lone UTF-16 surrogate: not UTF-8, so the hub could not read the message. */
const LONE_SURROGATE = /\p{Cs}/u;

const utf8 = new TextEncoder();

function utf8Bytes(text: string): number {
  return utf8.encode(text).byteLength;
}

/** Whether the hub takes `path` (K4 §9): not empty, no NUL, valid UTF-8, at most `PATH_MAX_BYTES`. */
export function isSendablePath(path: string): boolean {
  return path !== '' && !path.includes('\0') && !LONE_SURROGATE.test(path) && utf8Bytes(path) <= PATH_MAX_BYTES;
}

/** Whether `size` is a byte count the hub takes and this client can count exactly. */
export function isByteCount(size: unknown): size is number {
  return typeof size === 'number' && Number.isSafeInteger(size) && size >= 0;
}

/**
 * The text message for `request`, or `undefined` when the hub would refuse it (a path outside K4 §9, a size that is
 * not a byte count, a message over `TEXT_MAX_BYTES`): such a request is never sent.
 */
export function encodeRequest(request: FilesRequest): string | undefined {
  if (!isSendablePath(request.path)) return undefined;
  if (request.type === 'upload' && !isByteCount(request.size)) return undefined;
  const text = JSON.stringify(request);
  return utf8Bytes(text) <= TEXT_MAX_BYTES ? text : undefined;
}

type Fields = Readonly<Record<string, unknown>>;

/** `value` as an object with exactly `keys`, each once; anything else is `undefined`. */
function shaped(value: unknown, keys: readonly string[]): Fields | undefined {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return undefined;
  const own = Object.keys(value);
  return own.length === keys.length && keys.every((key) => own.includes(key)) ? (value as Fields) : undefined;
}

function entry(value: unknown): FileEntry | undefined {
  const fields = shaped(value, ['name', 'kind', 'size', 'modifiedAt']);
  if (fields === undefined) return undefined;
  const { name, kind, size, modifiedAt } = fields;
  if (typeof name !== 'string' || typeof kind !== 'string' || !KINDS.includes(kind)) return undefined;
  if (size !== null && !isByteCount(size)) return undefined;
  if (modifiedAt !== null && (typeof modifiedAt !== 'string' || !RFC3339.test(modifiedAt))) return undefined;
  return { name, kind: kind as EntryKind, size, modifiedAt };
}

function listing(fields: Fields): HubMessage | undefined {
  const { path, entries } = fields;
  if (typeof path !== 'string' || !Array.isArray(entries) || entries.length > ENTRIES_MAX) return undefined;
  const read: FileEntry[] = [];
  for (const value of entries) {
    const decoded = entry(value);
    if (decoded === undefined) return undefined;
    read.push(decoded);
  }
  return { kind: 'entries', listing: { path, entries: read } };
}

/** The message `fields` holds, by its `type`, when it has exactly that message's keys and valid values. */
function message(fields: Fields): HubMessage | undefined {
  const keys = Object.keys(fields).length;
  switch (fields.type) {
    case 'ping':
      return keys === 1 ? { kind: 'ping' } : undefined;
    case 'done':
      return keys === 1 ? { kind: 'done' } : undefined;
    case 'download':
      return shaped(fields, ['type', 'size']) !== undefined && isByteCount(fields.size) ? { kind: 'download', size: fields.size } : undefined;
    case 'refused':
      return shaped(fields, ['type', 'reason']) !== undefined && typeof fields.reason === 'string' && REFUSALS.includes(fields.reason)
        ? { kind: 'refused', reason: fields.reason as Refusal }
        : undefined;
    case 'entries':
      return shaped(fields, ['type', 'path', 'entries']) === undefined ? undefined : listing(fields);
    default:
      return undefined;
  }
}

/**
 * Read a text message from the hub. The set is closed (K4 §9): another `type`, a missing or extra key, another JSON
 * value, a size that is not a byte count, an unknown kind or reason, a time that is not RFC 3339 or more than
 * `ENTRIES_MAX` entries give `undefined`, and the link closes (fail closed): unlike the terminal, a files message
 * the client misreads could save or send the wrong bytes.
 */
export function decodeHubMessage(text: string): HubMessage | undefined {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return undefined;
  }
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return undefined;
  return message(value as Fields);
}
