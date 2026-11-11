import { describe, expect, it } from 'vitest';
import { TEXT_MAX_CHARS, decodeHubMessage, encodeCancel, encodePing } from './codec';

const decode = (message: unknown) => decodeHubMessage(JSON.stringify(message));

describe('playback messages (K4 §12)', () => {
  it('K4 §12: the header gives the format, the size and whether the recording is complete', () => {
    expect(decode({ type: 'recording', format: 'GUACAMOLE', sizeBytes: 1024, complete: true })).toEqual({
      kind: 'recording',
      header: { format: 'GUACAMOLE', sizeBytes: 1024, complete: true },
    });
    expect(decode({ type: 'recording', format: 'ASCIICAST', sizeBytes: 0, complete: false })).toMatchObject({
      header: { format: 'ASCIICAST', complete: false },
    });
  });

  it.each([
    ['an unknown format', { type: 'recording', format: 'MP4', sizeBytes: 1, complete: true }],
    ['a lower-case format', { type: 'recording', format: 'guacamole', sizeBytes: 1, complete: true }],
    ['a missing field', { type: 'recording', format: 'GUACAMOLE', complete: true }],
    ['an extra field', { type: 'recording', format: 'GUACAMOLE', sizeBytes: 1, complete: true, path: '/x' }],
    ['a negative size', { type: 'recording', format: 'GUACAMOLE', sizeBytes: -1, complete: true }],
    ['a fractional size', { type: 'recording', format: 'GUACAMOLE', sizeBytes: 1.5, complete: true }],
    ['a size as a string', { type: 'recording', format: 'GUACAMOLE', sizeBytes: '1', complete: true }],
    ['complete as a string', { type: 'recording', format: 'GUACAMOLE', sizeBytes: 1, complete: 'true' }],
    ['done with a field', { type: 'done', at: 1 }],
    ['failed with an unknown reason', { type: 'failed', reason: 'GONE' }],
    ['failed with an extra field', { type: 'failed', reason: 'DAMAGED', part: 3 }],
    ['a bare DAMAGED reason', { reason: 'DAMAGED' }],
  ])('K4 §12: a known message with bad fields is a protocol fault: %s', (_name, message) => {
    expect(decode(message)).toEqual({ kind: 'malformed' });
  });

  it('K4 §12: done, failed DAMAGED and failed MISSING; the bare {"reason":"MISSING"} of the contract text is MISSING too', () => {
    expect(decode({ type: 'done' })).toEqual({ kind: 'done' });
    expect(decode({ type: 'failed', reason: 'DAMAGED' })).toEqual({ kind: 'failed', reason: 'DAMAGED' });
    expect(decode({ type: 'failed', reason: 'MISSING' })).toEqual({ kind: 'failed', reason: 'MISSING' });
    expect(decode({ reason: 'MISSING' })).toEqual({ kind: 'failed', reason: 'MISSING' });
  });

  it('K4 §8.4: a ping is read; an unknown type, a non-object, bad JSON and an oversize text are ignored', () => {
    expect(decode({ type: 'ping' })).toEqual({ kind: 'ping' });
    expect(decode({ type: 'progress', at: 3 })).toBeUndefined();
    expect(decode(['recording'])).toBeUndefined();
    expect(decode('done')).toBeUndefined();
    expect(decode(null)).toBeUndefined();
    expect(decodeHubMessage('{"type":')).toBeUndefined();
    expect(decodeHubMessage(`{"type":"done"}${' '.repeat(TEXT_MAX_CHARS)}`)).toBeUndefined();
  });

  it('K4 §12: the client sends ping and cancel', () => {
    expect(JSON.parse(encodePing())).toEqual({ type: 'ping' });
    expect(JSON.parse(encodeCancel())).toEqual({ type: 'cancel' });
  });
});
