/** One recorded step: output text, or the terminal changing size (asciicast v2 `o` and `r` events, K4 §11). */
export type CastEvent =
  | { readonly at: number; readonly kind: 'output'; readonly text: string }
  | { readonly at: number; readonly kind: 'resize'; readonly cols: number; readonly rows: number };

export interface CastHeader {
  readonly width: number;
  readonly height: number;
  /** Unix seconds when the recording began. */
  readonly timestamp: number;
}

export interface Cast {
  readonly header: CastHeader;
  readonly events: readonly CastEvent[];
}

/** Why a recording is refused: its first line, or a line in the middle (`line` counts from 1). */
export type CastFault =
  | { readonly kind: 'header'; readonly line: 1 }
  | { readonly kind: 'event'; readonly line: number };

export type CastResult =
  | { readonly ok: true; readonly cast: Cast; readonly droppedLast: boolean }
  | { readonly ok: false; readonly fault: CastFault };

const RESIZE = /^(\d+)x(\d+)$/;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function positiveInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value > 0;
}

function headerOf(line: string): CastHeader | undefined {
  const value = parseJson(line);
  if (!isRecord(value) || value.version !== 2) return undefined;
  const { width, height, timestamp } = value;
  if (!positiveInteger(width) || !positiveInteger(height)) return undefined;
  return { width, height, timestamp: typeof timestamp === 'number' ? timestamp : 0 };
}

function parseJson(line: string): unknown {
  try {
    return JSON.parse(line);
  } catch {
    return undefined;
  }
}

/** An event line, `null` for a kind this player does not show (input, markers), `undefined` for a bad line. */
function eventOf(line: string): CastEvent | null | undefined {
  const value = parseJson(line);
  if (!Array.isArray(value) || value.length < 3) return undefined;
  const [at, kind, data]: unknown[] = value;
  if (typeof at !== 'number' || !Number.isFinite(at) || at < 0 || typeof kind !== 'string' || typeof data !== 'string') return undefined;
  if (kind === 'o') return { at, kind: 'output', text: data };
  if (kind !== 'r') return null;
  const size = RESIZE.exec(data);
  if (size === null) return undefined;
  const [cols, rows] = [Number(size[1]), Number(size[2])];
  return positiveInteger(cols) && positiveInteger(rows) ? { at, kind: 'resize', cols, rows } : undefined;
}

/** Reads a recording as it arrives, bytes at a time; a character split between two chunks is kept whole. */
export interface CastReader {
  /** Take the next bytes; a refusal is final and the same fault is returned for every later call. */
  push(bytes: Uint8Array): CastFault | undefined;
  /** The recording so far: undefined before the header line is complete. */
  cast(): Cast | undefined;
  /**
   * The recording is over. A last line that stops before its newline is a cut line: dropped when `complete` is false
   * (the hub stopped before the end), a refusal when it is true.
   */
  finish(complete: boolean): CastResult;
}

export function createCastReader(): CastReader {
  const decoder = new TextDecoder('utf-8', { fatal: false });
  const events: CastEvent[] = [];
  let header: CastHeader | undefined;
  let pending = '';
  let lineNumber = 0;
  let fault: CastFault | undefined;
  let finished: CastResult | undefined;

  const take = (line: string): void => {
    lineNumber += 1;
    if (header === undefined) {
      header = headerOf(line);
      if (header === undefined) fault = { kind: 'header', line: 1 };
      return;
    }
    const event = eventOf(line);
    if (event === undefined) fault = { kind: 'event', line: lineNumber };
    else if (event !== null && event.at < (events.at(-1)?.at ?? 0)) fault = { kind: 'event', line: lineNumber };
    else if (event !== null) events.push(event);
  };

  const feed = (text: string): void => {
    pending += text;
    let end = pending.indexOf('\n');
    while (end >= 0 && fault === undefined) {
      take(pending.slice(0, end));
      pending = pending.slice(end + 1);
      end = pending.indexOf('\n');
    }
  };

  const cast = (): Cast | undefined => (header === undefined ? undefined : { header, events: [...events] });

  const conclude = (complete: boolean): CastResult => {
    if (fault === undefined) feed(decoder.decode());
    const cut = pending.length > 0;
    if (fault === undefined && cut && complete) take(pending);
    const found = cast();
    if (fault !== undefined) return { ok: false, fault };
    if (found === undefined) return { ok: false, fault: { kind: 'header', line: 1 } };
    return { ok: true, cast: found, droppedLast: cut && !complete };
  };

  return {
    push(bytes) {
      if (fault === undefined) feed(decoder.decode(bytes, { stream: true }));
      return fault;
    },
    cast,
    // The recording ends once: a second call gives the same answer and does not read the cut line again.
    finish(complete) {
      finished ??= conclude(complete);
      return finished;
    },
  };
}

/** Parse a whole recording: text, or the plaintext bytes. */
export function parseCast(source: string | Uint8Array, complete: boolean): CastResult {
  const reader = createCastReader();
  reader.push(typeof source === 'string' ? new TextEncoder().encode(source) : source);
  return reader.finish(complete);
}
