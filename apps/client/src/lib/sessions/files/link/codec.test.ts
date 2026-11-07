import { describe, expect, it } from 'vitest';
import { ENTRIES_MAX, decodeHubMessage, encodeRequest, isSendablePath, type HubMessage } from './codec';

const ENTRY = { name: 'notes.txt', kind: 'file', size: 12, modifiedAt: '2026-11-06T10:00:00Z' };
const ENTRIES = { type: 'entries', path: '/home/op', entries: [ENTRY, { name: 'src', kind: 'dir', size: null, modifiedAt: null }] };

const decode = (value: unknown) => decodeHubMessage(JSON.stringify(value));

describe('client messages (K4 §9)', () => {
  it('K4 §9: list, download and upload are the hub’s exact shapes', () => {
    expect(JSON.parse(encodeRequest({ type: 'list', path: '/etc' }) ?? '')).toEqual({ type: 'list', path: '/etc' });
    expect(JSON.parse(encodeRequest({ type: 'download', path: '/a b' }) ?? '')).toEqual({ type: 'download', path: '/a b' });
    expect(JSON.parse(encodeRequest({ type: 'upload', path: '/up', size: 3 }) ?? '')).toEqual({ type: 'upload', path: '/up', size: 3 });
  });

  it('K4 §9: an empty path, a NUL or a lone surrogate is not sent', () => {
    for (const path of ['', '/a\0b', '/bad\ud800']) {
      expect(encodeRequest({ type: 'list', path })).toBeUndefined();
      expect(isSendablePath(path)).toBe(false);
    }
  });

  it('K4 §9: a path over 4096 UTF-8 bytes is not sent, counted in bytes, not characters', () => {
    expect(isSendablePath(`/${'é'.repeat(2047)}`)).toBe(true);
    expect(isSendablePath(`/${'é'.repeat(2048)}`)).toBe(false);
  });

  it('K4 §9: a message over 4 KiB is not sent, even when its path fits', () => {
    const near = `/${'a'.repeat(4060)}`;
    expect(encodeRequest({ type: 'list', path: near })).toBeDefined();
    const escaped = `/${'"'.repeat(2100)}`;
    expect(isSendablePath(escaped)).toBe(true);
    expect(encodeRequest({ type: 'list', path: escaped })).toBeUndefined();
  });

  it('K4 §9: an upload size that is not a non-negative safe integer is not sent', () => {
    for (const size of [-1, 1.5, Number.NaN, 2 ** 53]) expect(encodeRequest({ type: 'upload', path: '/up', size })).toBeUndefined();
    expect(encodeRequest({ type: 'upload', path: '/up', size: 0 })).toBeDefined();
  });
});

describe('hub messages, a closed set (K4 §9)', () => {
  it('K4 §9: entries, with size and modifiedAt null where the device does not say', () => {
    expect(decode(ENTRIES)).toEqual({ kind: 'entries', listing: { path: '/home/op', entries: ENTRIES.entries } });
  });

  it('K4 §9: download, done, refused and ping', () => {
    expect(decode({ type: 'download', size: 0 })).toEqual({ kind: 'download', size: 0 });
    expect(decode({ type: 'done' })).toEqual({ kind: 'done' });
    for (const reason of ['DOWNLOAD_OFF', 'UPLOAD_OFF', 'NOT_FOUND', 'DENIED', 'EXISTS']) {
      expect(decode({ type: 'refused', reason })).toEqual({ kind: 'refused', reason });
    }
    expect(decodeHubMessage(' { "type" : "ping" } ')).toEqual({ kind: 'ping' });
  });

  it('K4 §9: a time with fractional seconds or an offset is read; one that is not RFC 3339 is not', () => {
    expect(decode({ ...ENTRIES, entries: [{ ...ENTRY, modifiedAt: '2026-11-06T10:00:00.123456789+01:00' }] })).toBeDefined();
    expect(decode({ ...ENTRIES, entries: [{ ...ENTRY, modifiedAt: 'yesterday' }] })).toBeUndefined();
  });

  it('K4 §9: up to 10 000 entries are read; one more is not', () => {
    const many = (count: number) => ({ ...ENTRIES, entries: Array.from({ length: count }, () => ENTRY) });
    expect(decode(many(ENTRIES_MAX))).toMatchObject({ kind: 'entries' });
    expect(decode(many(ENTRIES_MAX + 1))).toBeUndefined();
  });

  it.each([
    ['another type', { type: 'end', reason: 'OPERATOR' }],
    ['an extra key', { type: 'done', more: 1 }],
    ['a missing key', { type: 'download' }],
    ['a negative size', { type: 'download', size: -1 }],
    ['a fractional size', { type: 'download', size: 1.5 }],
    ['a size past exact integers', { type: 'download', size: 2 ** 53 }],
    ['a size as text', { type: 'download', size: '10' }],
    ['an unknown reason', { type: 'refused', reason: 'BUSY' }],
    ['an entry size as text', { ...ENTRIES, entries: [{ ...ENTRY, size: '12' }] }],
    ['a negative entry size', { ...ENTRIES, entries: [{ ...ENTRY, size: -1 }] }],
    ['a fractional entry size', { ...ENTRIES, entries: [{ ...ENTRY, size: 1.5 }] }],
    ['an entry size past exact integers', { ...ENTRIES, entries: [{ ...ENTRY, size: 2 ** 53 }] }],
    ['an unknown kind', { ...ENTRIES, entries: [{ ...ENTRY, kind: 'socket' }] }],
    ['an entry with an extra key', { ...ENTRIES, entries: [{ ...ENTRY, mode: 420 }] }],
    ['an entry without modifiedAt', { ...ENTRIES, entries: [{ name: 'a', kind: 'file', size: 1 }] }],
    ['a name that is not text', { ...ENTRIES, entries: [{ ...ENTRY, name: 5 }] }],
    ['entries that are not a list', { ...ENTRIES, entries: {} }],
    ['an array', [{ type: 'ping' }]],
    ['a string', 'ping'],
    ['null', null],
  ])('K4 §9: %s is not a hub message', (_case, value) => {
    expect(decode(value)).toBeUndefined();
  });

  it('K4 §9: text that is not JSON, and a __proto__ key, are not hub messages', () => {
    expect(decodeHubMessage('{"type":"done"')).toBeUndefined();
    expect(decodeHubMessage('{"type":"done","__proto__":{}}')).toBeUndefined();
  });
});

/** A seeded generator, so a failing case can be replayed. */
function seeded(seed: number) {
  let state = seed;
  return (below: number) => {
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t ^= t + Math.imul(t ^ (t >>> 7), 61 | t);
    return (((t ^ (t >>> 14)) >>> 0) % below) as number;
  };
}

const SEEDS = [JSON.stringify(ENTRIES), '{"type":"download","size":65536}', '{"type":"refused","reason":"EXISTS"}', '{"type":"done"}', '{"type":"ping"}'];
const NOISE = '{}[]",:0123456789-.eE nulltruefalse\\u\0é';

/** `text` with a few characters replaced, deleted or inserted. */
function mutate(text: string, next: (below: number) => number): string {
  let out = text;
  for (let edits = 1 + next(4); edits > 0; edits -= 1) {
    const at = next(out.length + 1);
    const noise = NOISE.charAt(next(NOISE.length));
    const edit = next(3);
    out = edit === 0 ? out.slice(0, at) + noise + out.slice(at + 1) : edit === 1 ? out.slice(0, at) + out.slice(at + 1) : out.slice(0, at) + noise + out.slice(at);
  }
  return out;
}

/** Whether `message` holds only values the closed set allows, whatever text it came from. */
function wellFormed(message: HubMessage): boolean {
  switch (message.kind) {
    case 'download':
      return Number.isSafeInteger(message.size) && message.size >= 0;
    case 'entries':
      return message.listing.entries.every((entry) => ['file', 'dir', 'link'].includes(entry.kind) && (entry.size === null || Number.isSafeInteger(entry.size)));
    default:
      return true;
  }
}

describe('decoder fuzz (parse, never trust)', () => {
  it('K4 §9: mutated hub messages never throw, and whatever decodes holds only allowed values', () => {
    const next = seeded(0x5eed);
    for (let round = 0; round < 20_000; round += 1) {
      const text = mutate(SEEDS[next(SEEDS.length)] ?? '', next);
      const message = decodeHubMessage(text);
      if (message !== undefined) expect(wellFormed(message), text).toBe(true);
    }
  });
});
