import { describe, expect, it } from 'vitest';
import { CELLS_MAX, FRAME_MAX_BYTES, clampSize, decodeHubMessage, encodePing, encodeResize, inputFrames } from './codec';

describe('control messages (K4 §8.4)', () => {
  it('K4 §8.4: resize is a JSON object with type, cols and rows only', () => {
    expect(JSON.parse(encodeResize({ cols: 80, rows: 24 }))).toEqual({ type: 'resize', cols: 80, rows: 24 });
    expect(encodeResize({ cols: 80, rows: 24 })).toBe('{"type":"resize","cols":80,"rows":24}');
  });

  it.each([
    [{ cols: 0, rows: 0 }, { cols: 1, rows: 1 }],
    [{ cols: -5, rows: 24 }, { cols: 1, rows: 24 }],
    [{ cols: 5000, rows: 1001 }, { cols: CELLS_MAX, rows: CELLS_MAX }],
    [{ cols: 80.6, rows: 23.2 }, { cols: 81, rows: 23 }],
    [{ cols: Number.NaN, rows: Number.POSITIVE_INFINITY }, { cols: 1, rows: 1 }],
  ])('K4 §8.4: the size %j is sent as the integers %j in 1..1000', (size, expected) => {
    expect(clampSize(size)).toEqual(expected);
    expect(JSON.parse(encodeResize(size))).toEqual({ type: 'resize', ...expected });
  });

  it('K4 §8.4: ping is {"type":"ping"}', () => {
    expect(encodePing()).toBe('{"type":"ping"}');
  });

  it.each([
    ['{"type":"ping"}', { kind: 'ping' }],
    ['{"type":"end","reason":"AUTH_FAILED"}', { kind: 'end', reason: 'AUTH_FAILED' }],
    ['{"type":"end","reason":"HOST_KEY_MISMATCH"}', { kind: 'end', reason: 'HOST_KEY_MISMATCH' }],
  ])('K4 §8.4: the hub message %s is understood', (text, expected) => {
    expect(decodeHubMessage(text)).toEqual(expected);
  });

  it.each([
    '{"type":"mystery"}',
    '{"type":"end"}',
    '{"type":"end","reason":"NOT_A_REASON"}',
    '{"type":"end","reason":7}',
    '{"type":"end","reason":"OPERATOR","extra":1}',
    '{"type":"ping","extra":true}',
    '{"type":"resize","cols":1,"rows":1}',
    '[]',
    'null',
    '"ping"',
    'ping',
    '',
  ])('K4 §8.4: %j is outside the closed set, so it gives nothing and the caller ignores it', (text) => {
    expect(decodeHubMessage(text)).toBeUndefined();
  });
});

describe('hub text over the size limit (K4 §8.5)', () => {
  it('K4 §8.5: a text over 256 characters is dropped without parsing, even if it starts like a ping', () => {
    const padded = `{"type":"ping"${' '.repeat(300)}}`;
    expect(decodeHubMessage(padded)).toBeUndefined();
    expect(decodeHubMessage('{"type":"ping"}')).toEqual({ kind: 'ping' });
  });
});

describe('input frames (K4 §8.5)', () => {
  it('K4 §8.5: a small input is one frame', () => {
    expect(inputFrames(Uint8Array.from([1, 2, 3]))).toEqual([Uint8Array.from([1, 2, 3])]);
  });

  it('K4 §8.5: a large input is cut into frames of at most 32 KiB, in order, losing no byte', () => {
    const input = Uint8Array.from({ length: FRAME_MAX_BYTES * 2 + 5 }, (_, at) => at % 251);
    const frames = inputFrames(input);
    expect(frames.map((frame) => frame.length)).toEqual([FRAME_MAX_BYTES, FRAME_MAX_BYTES, 5]);
    expect(Uint8Array.from(frames.flatMap((frame) => [...frame]))).toEqual(input);
    expect(FRAME_MAX_BYTES).toBeLessThan(64 * 1024);
  });

  it('K4 §8.5: an empty input sends no frame', () => {
    expect(inputFrames(new Uint8Array())).toEqual([]);
  });
});
