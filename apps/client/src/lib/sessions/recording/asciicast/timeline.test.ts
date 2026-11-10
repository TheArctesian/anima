import { expect, it } from 'vitest';
import { parseCast, type Cast } from './parse';
import { clockText, durationOf, eventsBy } from './timeline';

const text = [
  '{"version":2,"width":80,"height":24,"timestamp":0}',
  '[0.5,"o","a"]',
  '[1,"r","100x30"]',
  '[1,"o","b"]',
  '[3,"r","60x20"]',
  '[4,"o","c"]',
].join('\n');
const parsed = parseCast(text, true);
if (!parsed.ok) throw new Error('fixture refused');
const cast: Cast = parsed.cast;

it('RA-041: the duration is the time of the last event, and 0 for no events', () => {
  expect(durationOf(cast)).toBe(4);
  expect(durationOf({ header: cast.header, events: [] })).toBe(0);
});

it('D-12: eventsBy counts events at exactly t and none before the first', () => {
  expect(eventsBy(cast, 0)).toBe(0);
  expect(eventsBy(cast, 1)).toBe(3);
});

it('D-12: clock text reads m:ss, h:mm:ss from an hour, and 0:00 for a bad time', () => {
  expect(clockText(0)).toBe('0:00');
  expect(clockText(65.9)).toBe('1:05');
  expect(clockText(3725)).toBe('1:02:05');
  expect(clockText(-3)).toBe('0:00');
  expect(clockText(Number.NaN)).toBe('0:00');
});
