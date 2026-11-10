import { expect, it } from 'vitest';
import { asciicastStream } from '$lib/fixtures/phase3';
import { createCastReader, parseCast } from './parse';

const HEADER = '{"version":2,"width":80,"height":24,"timestamp":1793000000}';
const lines = (...rest: string[]) => [HEADER, ...rest].join('\n') + '\n';

it('RA-041: the K6 asciicast fixture parses to its header and four output events', () => {
  const result = parseCast(asciicastStream.text, true);
  if (!result.ok) throw new Error('refused');
  expect(result.cast.header).toEqual({ width: 80, height: 24, timestamp: 1793000000 });
  expect(result.cast.events.map((event) => event.at)).toEqual([0.2, 0.9, 0.91, 1.0]);
  expect(result.cast.events[1]).toEqual({ at: 0.9, kind: 'output', text: 'echo hello\r\n' });
});

it('D-12: a resize event reads CxR, and an input or marker event is left out', () => {
  const result = parseCast(lines('[1,"r","100x30"]', '[1.5,"i","x"]', '[2,"m","mark"]'), true);
  if (!result.ok) throw new Error('refused');
  expect(result.cast.events).toEqual([{ at: 1, kind: 'resize', cols: 100, rows: 30 }]);
});

it.each([
  ['not JSON', 'nope'],
  ['version 1', '{"version":1,"width":80,"height":24}'],
  ['a zero width', '{"version":2,"width":0,"height":24}'],
  ['a missing height', '{"version":2,"width":80}'],
])('RA-041: a header that is %s is refused as a header fault', (_name, header) => {
  expect(parseCast(`${header}\n[0,"o","a"]\n`, true)).toEqual({ ok: false, fault: { kind: 'header', line: 1 } });
});

it('RA-041: an empty recording is refused as a header fault', () => {
  expect(parseCast('', false)).toEqual({ ok: false, fault: { kind: 'header', line: 1 } });
});

it.each([
  ['not JSON', '[1,"o"'],
  ['a negative time', '[-1,"o","a"]'],
  ['a bad resize', '[1,"r","wide"]'],
  ['a time that goes back', '[0.5,"o","a"]'],
  ['not an array', '{"t":1}'],
])('RA-041: a middle line with %s is refused with its line number, also for an incomplete recording', (_name, bad) => {
  const text = lines('[1,"o","a"]', bad, '[2,"o","b"]');
  const expected = { ok: false, fault: { kind: 'event', line: 3 } };
  expect(parseCast(text, true)).toEqual(expected);
  expect(parseCast(text, false)).toEqual(expected);
});

it('RA-041: an incomplete recording drops a cut last line and plays the whole lines', () => {
  const result = parseCast(lines('[1,"o","a"]') + '[2,"o","b', false);
  if (!result.ok) throw new Error('refused');
  expect(result.droppedLast).toBe(true);
  expect(result.cast.events).toHaveLength(1);
});

it('RA-041: a complete recording with a cut last line is refused', () => {
  expect(parseCast(lines('[1,"o","a"]') + '[2,"o","b', true)).toEqual({ ok: false, fault: { kind: 'event', line: 3 } });
});

it('RA-041: a complete recording whose last line has no newline still plays that line', () => {
  const result = parseCast(lines('[1,"o","a"]') + '[2,"o","b"]', true);
  if (!result.ok) throw new Error('refused');
  expect(result.cast.events).toHaveLength(2);
  expect(result.droppedLast).toBe(false);
});

it('D-12: the incremental reader joins a UTF-8 character split across chunks and gives the same events as one pass', () => {
  const bytes = new TextEncoder().encode(lines('[1,"o","héllo ☃ 😀"]', '[2,"o","b"]'));
  const reader = createCastReader();
  for (const byte of bytes) expect(reader.push(Uint8Array.of(byte))).toBeUndefined();
  const whole = parseCast(bytes, true);
  const done = reader.finish(true);
  if (!done.ok || !whole.ok) throw new Error('refused');
  expect(done.cast.events).toEqual(whole.cast.events);
  expect(done.cast.events[0]).toEqual({ at: 1, kind: 'output', text: 'héllo ☃ 😀' });
});

it('D-12: the incremental reader shows the recording so far, and a refusal is final', () => {
  const reader = createCastReader();
  expect(reader.cast()).toBeUndefined();
  reader.push(new TextEncoder().encode(HEADER + '\n[1,"o","a"]\n[2,"o"'));
  expect(reader.cast()?.events).toHaveLength(1);
  const fault = reader.push(new TextEncoder().encode(',\n'));
  expect(fault).toEqual({ kind: 'event', line: 3 });
  expect(reader.push(new TextEncoder().encode('[3,"o","c"]\n'))).toEqual(fault);
  expect(reader.finish(false)).toEqual({ ok: false, fault });
});

it('D-12: finishing twice gives the same answer and does not read the cut line the second time', () => {
  const reader = createCastReader();
  reader.push(new TextEncoder().encode(HEADER + '\n[1,"o","a"]\n[2,"o"'));
  const first = reader.finish(false);
  expect(first).toMatchObject({ ok: true, droppedLast: true });
  expect(reader.finish(true)).toBe(first);
});
